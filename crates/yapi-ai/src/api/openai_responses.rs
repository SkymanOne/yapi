//! The OpenAI Responses API and its two variants: `openai-responses`,
//! `azure-openai-responses` and `openai-codex-responses` (ChatGPT's Codex
//! backend).
//!
//! Port of `packages/ai/src/api/openai-responses.ts`,
//! `azure-openai-responses.ts`, `openai-codex-responses.ts` and
//! `openai-responses-shared.ts` in pi `v1.0.0`. Not yet ported: service tier
//! selection. Codex's WebSocket transport is in `codex_websocket.rs`.

use std::collections::HashMap;

use indexmap::IndexMap;
use serde_json::{Map, Value, json};
use yapi_types::message::{
    AssistantMessage, Content, ContentBlock, Message, StopReason, TextContent, ThinkingContent,
    ThinkingLevel, ToolCall, ToolDeclaration, ToolResultMessage, blocks_text,
};
use yapi_types::model::{Model, OpenAiResponsesCompat};

use super::codex_websocket::{self, Continuation, Failure};
use crate::cost::calculate_cost;
use crate::hash::short_hash;
use crate::http::{self, Headers, SseReader, is_truthy};
use crate::json_parse::parse_streaming_json;
use crate::schema::{self, GrammarInput};
use crate::stream::{
    CacheRetention, EventSender, Request, StreamEvent, StreamOptions, Transport, check_complete,
    new_output, now_ms, send_error,
};
use crate::thinking::{effort, requested_max_tokens};
use crate::transcript::{resolve_transcript, resolve_transcript_tools, transform_messages};

/// Which Responses endpoint a request goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flavor {
    /// `openai-responses`.
    OpenAi,
    /// `azure-openai-responses`: Responses on an Azure resource.
    Azure,
    /// `openai-codex-responses`: ChatGPT's Codex backend, signed in with a
    /// ChatGPT account.
    Codex,
}

impl Flavor {
    /// Providers whose `call|item` tool-call ids are kept as Responses item ids.
    fn tool_call_providers(self) -> &'static [&'static str] {
        match self {
            Flavor::Azure => &[
                "openai",
                "openai-codex",
                "opencode",
                "azure-openai-responses",
            ],
            Flavor::OpenAi | Flavor::Codex => &["openai", "openai-codex", "opencode"],
        }
    }

    /// The error when the stream ends without a stop reason.
    fn pending_message(self) -> &'static str {
        match self {
            Flavor::OpenAi => "OpenAI Responses stream ended without a stop reason",
            Flavor::Azure => "Azure OpenAI Responses stream ended without a stop reason",
            Flavor::Codex => "Codex stream ended without a stop reason",
        }
    }
}
/// The Responses API rejects `max_output_tokens` below this.
const MIN_OUTPUT_TOKENS: u64 = 16;
const CHATGPT_USAGE_URL: &str = "https://chatgpt.com/settings/usage";

/// Compat flags with defaults applied.
#[derive(Clone, Debug)]
struct Compat {
    flavor: Flavor,
    supports_developer_role: bool,
    supports_mid_convo_system_messages: bool,
    session_affinity_format: String,
    supports_long_cache_retention: bool,
    supports_strict_mode: bool,
    supports_grammar_tools: bool,
    supports_additional_tools: bool,
    supports_tool_search: bool,
    supports_explicit_prompt_cache_mode: bool,
    supports_max_output_tokens: bool,
}

impl Compat {
    fn new(model: &Model, flavor: Flavor) -> Compat {
        let raw: OpenAiResponsesCompat = model.compat();
        let detected = if model.provider == "openrouter" || model.base_url.contains("openrouter.ai")
        {
            "openrouter"
        } else {
            "openai"
        };
        Compat {
            flavor,
            supports_developer_role: raw.supports_developer_role.unwrap_or(true),
            supports_mid_convo_system_messages: raw
                .supports_mid_convo_system_messages
                .unwrap_or(false),
            session_affinity_format: raw
                .session_affinity_format
                .unwrap_or_else(|| detected.to_owned()),
            supports_long_cache_retention: raw.supports_long_cache_retention.unwrap_or(true),
            supports_strict_mode: raw.supports_strict_mode.unwrap_or(flavor != Flavor::OpenAi),
            supports_grammar_tools: raw.supports_open_ai_grammar_tools.unwrap_or(false),
            supports_additional_tools: raw.supports_additional_tools.unwrap_or(false),
            supports_tool_search: raw.supports_tool_search.unwrap_or(false),
            supports_explicit_prompt_cache_mode: raw
                .supports_explicit_prompt_cache_mode
                .unwrap_or(false),
            supports_max_output_tokens: raw.supports_max_output_tokens.unwrap_or(true),
        }
    }
}

/// A non-`sk-` credential sent straight to OpenAI is a Sign in with ChatGPT token,
/// which rejects some request fields.
fn is_chatgpt_sign_in(model: &Model, api_key: &str) -> bool {
    model.provider == "openai"
        && model.base_url == "https://api.openai.com/v1"
        && !api_key.starts_with("sk-")
}

/// pi's text signature: `{"v":1,"id":…,"phase":…}`.
fn encode_text_signature(id: Option<&str>, phase: Option<&str>) -> String {
    let mut payload = Map::new();
    payload.insert("v".into(), json!(1));
    if let Some(id) = id {
        payload.insert("id".into(), json!(id));
    }
    if let Some(phase) = phase.filter(|phase| !phase.is_empty()) {
        payload.insert("phase".into(), json!(phase));
    }
    yapi_types::json::stringify(&payload)
}

/// The message id and phase from a text signature: v1 JSON, or a legacy plain id.
fn parse_text_signature(signature: Option<&str>) -> Option<(String, Option<String>)> {
    let signature = signature.filter(|signature| !signature.is_empty())?;
    if signature.starts_with('{')
        && let Ok(parsed) = serde_json::from_str::<Value>(signature)
        && parsed["v"] == 1
        && let Some(id) = parsed["id"].as_str()
    {
        let phase = parsed["phase"]
            .as_str()
            .filter(|phase| matches!(*phase, "commentary" | "final_answer"))
            .map(str::to_owned);
        return Some((id.to_owned(), phase));
    }
    Some((signature.to_owned(), None))
}

/// An id part as the Responses API accepts it: sanitized, at most 64 units, no
/// trailing underscores.
fn normalize_id_part(part: &str) -> String {
    super::normalize_tool_call_id(part)
        .trim_end_matches('_')
        .to_owned()
}

fn foreign_item_id(item_id: &str) -> String {
    format!("fc_{}", short_hash(item_id))
        .chars()
        .take(64)
        .collect()
}

/// Rewrites a tool-call id from another model into `call|item` with an `fc_` item.
fn normalize_tool_call_id(
    id: &str,
    model: &Model,
    source: &AssistantMessage,
    flavor: Flavor,
) -> String {
    if !flavor
        .tool_call_providers()
        .contains(&model.provider.as_str())
        || !id.contains('|')
    {
        return normalize_id_part(id);
    }
    let mut parts = id.split('|');
    let call = normalize_id_part(parts.next().unwrap_or_default());
    let item = parts.next().unwrap_or_default();
    let foreign = source.provider != model.provider || source.api != model.api;
    let mut item = if foreign {
        foreign_item_id(item)
    } else {
        normalize_id_part(item)
    };
    if !item.starts_with("fc_") {
        item = normalize_id_part(&format!("fc_{item}"));
    }
    format!("{call}|{item}")
}

fn convert_tools(
    tools: &[ToolDeclaration],
    compat: &Compat,
    tool_search_result: bool,
) -> Result<Vec<Value>, String> {
    tools
        .iter()
        .map(|tool| {
            if let Some(grammar) = schema::grammar_sampling(tool, compat.supports_grammar_tools)? {
                let mut custom = Map::new();
                custom.insert("type".into(), json!("custom"));
                custom.insert("name".into(), json!(tool.name));
                custom.insert("description".into(), json!(tool.description));
                custom.insert(
                    "format".into(),
                    json!({"type": "grammar", "syntax": grammar.syntax, "definition": grammar.definition}),
                );
                if tool_search_result {
                    custom.insert("defer_loading".into(), json!(true));
                }
                return Ok(Value::Object(custom));
            }
            let strict = schema::strict_sampling(tool, compat.supports_strict_mode, None)?;
            let mut function = Map::new();
            function.insert("type".into(), json!("function"));
            function.insert("name".into(), json!(tool.name));
            function.insert("description".into(), json!(tool.description));
            function.insert("parameters".into(), schema::tool_parameters(tool, strict));
            if tool_search_result {
                function.insert("defer_loading".into(), json!(true));
            }
            if compat.supports_strict_mode {
                // Codex leaves strictness to the server unless a tool asks for it.
                let value = if !strict && compat.flavor == Flavor::Codex {
                    Value::Null
                } else {
                    json!(strict)
                };
                function.insert("strict".into(), value);
            }
            Ok(Value::Object(function))
        })
        .collect()
}

fn image_input(mime_type: &str, data: &str) -> Value {
    json!({
        "type": "input_image",
        "detail": "auto",
        "image_url": format!("data:{mime_type};base64,{data}"),
    })
}

fn tool_result_output(model: &Model, result: &ToolResultMessage) -> Value {
    let text = blocks_text(&result.content, "\n");
    let images: Vec<Value> = result
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Image(image) => Some(image_input(&image.mime_type, &image.data)),
            _ => None,
        })
        .collect();
    if images.is_empty() || !model.accepts_images() {
        return Value::String(if !text.is_empty() {
            text
        } else if !images.is_empty() {
            "(see attached image)".into()
        } else {
            "(no tool output)".into()
        });
    }
    let mut output = Vec::new();
    if !text.is_empty() {
        output.push(json!({"type": "input_text", "text": text}));
    }
    output.extend(images);
    Value::Array(output)
}

fn convert_assistant(
    assistant: &AssistantMessage,
    model: &Model,
    index: usize,
    grammar: &HashMap<String, String>,
) -> Result<Vec<Value>, String> {
    let same_api = assistant.provider == model.provider && assistant.api == model.api;
    let same_model = same_api && assistant.model == model.id;
    let different_model = same_api && assistant.model != model.id;
    let mut output = Vec::new();
    let mut text_index = 0;
    for block in &assistant.content {
        match block {
            ContentBlock::Thinking(thinking) => {
                if let Some(signature) = thinking
                    .thinking_signature
                    .as_deref()
                    .filter(|signature| !signature.is_empty())
                {
                    let item: Value = serde_json::from_str(signature)
                        .map_err(|err| format!("Invalid reasoning signature: {err}"))?;
                    output.push(item);
                }
            }
            ContentBlock::Text(text) => {
                let parsed = parse_text_signature(text.text_signature.as_deref());
                let fallback = if text_index == 0 {
                    format!("msg_pi_{index}")
                } else {
                    format!("msg_pi_{index}_{text_index}")
                };
                text_index += 1;
                let id = match parsed.as_ref().map(|(id, _)| id.as_str()) {
                    None | Some("") => fallback,
                    Some(id) if yapi_types::js::len(id) > 64 => format!("msg_{}", short_hash(id)),
                    Some(id) => id.to_owned(),
                };
                let mut item = Map::new();
                item.insert("type".into(), json!("message"));
                item.insert("role".into(), json!("assistant"));
                item.insert(
                    "content".into(),
                    json!([{"type": "output_text", "text": text.text, "annotations": []}]),
                );
                item.insert("status".into(), json!("completed"));
                item.insert("id".into(), json!(id));
                if let Some(phase) = parsed.and_then(|(_, phase)| phase) {
                    item.insert("phase".into(), json!(phase));
                }
                output.push(Value::Object(item));
            }
            ContentBlock::ToolCall(call) => {
                let property = grammar.get(&call.name);
                let mut parts = call.id.split('|');
                let call_id = parts.next().unwrap_or_default();
                // An item id must match the replayed type: a call switches between
                // the two when grammar support differs between models.
                let prefix = if property.is_some() { "ctc_" } else { "fc_" };
                let item_id = parts
                    .next()
                    .filter(|item| !different_model && item.starts_with(prefix));
                let mut item = Map::new();
                item.insert(
                    "type".into(),
                    json!(if property.is_some() {
                        "custom_tool_call"
                    } else {
                        "function_call"
                    }),
                );
                if let Some(item_id) = item_id {
                    item.insert("id".into(), json!(item_id));
                }
                item.insert("call_id".into(), json!(call_id));
                item.insert("name".into(), json!(call.name));
                match property {
                    Some(property) => item.insert(
                        "input".into(),
                        json!(schema::grammar_input(
                            &call.name,
                            &call.arguments,
                            property
                        )?),
                    ),
                    None => item.insert(
                        "arguments".into(),
                        json!(yapi_types::json::stringify(&call.arguments)),
                    ),
                };
                if same_model && let Some(namespace) = &call.namespace {
                    item.insert("namespace".into(), json!(namespace));
                }
                output.push(Value::Object(item));
            }
            ContentBlock::Image(_) => {}
        }
    }
    Ok(output)
}

/// Converts the transcript to Responses input items.
fn convert_messages(
    model: &Model,
    messages: &[Message],
    compat: &Compat,
    grammar: &HashMap<String, String>,
) -> Result<Vec<Value>, String> {
    let messages = resolve_transcript(messages, compat.supports_mid_convo_system_messages);
    let normalize = |id: &str, source: &AssistantMessage| {
        normalize_tool_call_id(id, model, source, compat.flavor)
    };
    let transformed = transform_messages(&messages, model, Some(&normalize), now_ms());
    let (_, anchors) = resolve_transcript_tools(
        &messages,
        compat.supports_additional_tools || compat.supports_tool_search,
    );
    let instruction_role = if model.reasoning && compat.supports_developer_role {
        "developer"
    } else {
        "system"
    };
    let mut input: Vec<Value> = Vec::new();
    let mut index = 0;
    for (position, message) in transformed.iter().enumerate() {
        let leading = position == 0 && matches!(message, Message::System(_));
        match message {
            Message::System(system) => {
                let added = system.tools_added.as_deref().unwrap_or_default();
                if !leading && anchors && !added.is_empty() {
                    if compat.supports_additional_tools {
                        input.push(json!({
                            "type": "additional_tools",
                            "role": "developer",
                            "tools": convert_tools(added, compat, false)?,
                        }));
                    } else if compat.supports_tool_search {
                        let names: Vec<&str> =
                            added.iter().map(|tool| tool.name.as_str()).collect();
                        let call_id = format!(
                            "pi_tool_load_{}",
                            short_hash(&format!("system:{index}:{}", names.join(",")))
                        );
                        input.push(json!({
                            "type": "tool_search_call",
                            "call_id": call_id,
                            "execution": "client",
                            "status": "completed",
                            "arguments": {"query": names.join(" "), "limit": names.len()},
                        }));
                        input.push(json!({
                            "type": "tool_search_output",
                            "call_id": call_id,
                            "execution": "client",
                            "status": "completed",
                            "tools": convert_tools(added, compat, true)?,
                        }));
                    }
                }
                // Codex sends the initial prompt as `instructions`.
                let text = if leading && compat.flavor == Flavor::Codex {
                    String::new()
                } else if leading {
                    system.text()
                } else {
                    system.render_update()
                };
                if !text.is_empty() {
                    input.push(json!({"role": instruction_role, "content": text}));
                }
            }
            Message::User(user) => match &user.content {
                Content::Text(text) => {
                    input.push(json!({
                        "role": "user",
                        "content": [{"type": "input_text", "text": text}],
                    }));
                }
                Content::Blocks(blocks) => {
                    let content: Vec<Value> = blocks
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text(text) => {
                                Some(json!({"type": "input_text", "text": text.text}))
                            }
                            ContentBlock::Image(image) => {
                                Some(image_input(&image.mime_type, &image.data))
                            }
                            _ => None,
                        })
                        .collect();
                    if content.is_empty() {
                        continue;
                    }
                    input.push(json!({"role": "user", "content": content}));
                }
            },
            Message::Assistant(assistant) => {
                let output = convert_assistant(assistant, model, index, grammar)?;
                if output.is_empty() {
                    continue;
                }
                input.extend(output);
            }
            Message::ToolResult(result) => {
                let call_id = result.tool_call_id.split('|').next().unwrap_or_default();
                let kind = if grammar.contains_key(&result.tool_name) {
                    "custom_tool_call_output"
                } else {
                    "function_call_output"
                };
                input.push(json!({
                    "type": kind,
                    "call_id": call_id,
                    "output": tool_result_output(model, result),
                }));
            }
            _ => {}
        }
        if !leading {
            index += 1;
        }
    }
    Ok(input)
}

fn build_params(
    model: &Model,
    messages: &[Message],
    options: &StreamOptions,
    compat: &Compat,
    grammar: &HashMap<String, String>,
    max_tokens: u64,
    effort: Option<ThinkingLevel>,
) -> Result<Value, String> {
    let (request_tools, _) = resolve_transcript_tools(
        messages,
        compat.supports_additional_tools || compat.supports_tool_search,
    );
    let input = convert_messages(model, messages, compat, grammar)?;
    let retention = options.resolved_cache_retention();
    let omit = options
        .api_key
        .as_deref()
        .is_some_and(|key| is_chatgpt_sign_in(model, key));
    let mut params = Map::new();
    params.insert("model".into(), json!(model.id));
    params.insert("input".into(), Value::Array(input));
    params.insert("stream".into(), json!(true));
    if retention != CacheRetention::None
        && let Some(session) = &options.session_id
    {
        params.insert(
            "prompt_cache_key".into(),
            json!(session.chars().take(64).collect::<String>()),
        );
    }
    if !omit {
        if retention == CacheRetention::Long
            && compat.supports_long_cache_retention
            && !compat.supports_explicit_prompt_cache_mode
        {
            params.insert("prompt_cache_retention".into(), json!("24h"));
        }
        if compat.supports_explicit_prompt_cache_mode {
            match retention {
                CacheRetention::None => {
                    params.insert("prompt_cache_options".into(), json!({"mode": "explicit"}));
                }
                CacheRetention::Long if compat.supports_long_cache_retention => {
                    params.insert("prompt_cache_options".into(), json!({"ttl": "30m"}));
                }
                _ => {}
            }
        }
    }
    params.insert("store".into(), json!(false));
    if max_tokens > 0 && compat.supports_max_output_tokens && !omit {
        params.insert(
            "max_output_tokens".into(),
            json!(max_tokens.max(MIN_OUTPUT_TOKENS)),
        );
    }
    if let Some(temperature) = options.temperature.filter(|_| !omit) {
        params.insert("temperature".into(), json!(temperature));
    }
    if !request_tools.is_empty() {
        params.insert(
            "tools".into(),
            Value::Array(convert_tools(&request_tools, compat, false)?),
        );
    }
    if model.reasoning {
        if let Some(level) = effort {
            let effort = match model.thinking_level_value(level) {
                Some(Some(value)) => value.to_owned(),
                _ => level.as_str().to_owned(),
            };
            params.insert(
                "reasoning".into(),
                json!({"effort": effort, "summary": "auto"}),
            );
            params.insert("include".into(), json!(["reasoning.encrypted_content"]));
        } else if model.provider != "github-copilot" {
            match model.thinking_level_value(ThinkingLevel::Off) {
                Some(None) => {}
                Some(Some(value)) => {
                    params.insert("reasoning".into(), json!({"effort": value}));
                }
                None => {
                    params.insert("reasoning".into(), json!({"effort": "none"}));
                }
            }
        }
        if model.provider == "xai" {
            params.insert("include".into(), json!(["reasoning.encrypted_content"]));
        }
    }
    for (key, value) in model.sampling_params.iter().flatten() {
        params.insert(key.clone(), value.clone());
    }
    Ok(Value::Object(params))
}

fn service_tier_multiplier(model: &Model, tier: Option<&str>, flavor: Flavor) -> f64 {
    match (flavor, tier) {
        (Flavor::Azure, _) => 1.0,
        (_, Some("flex")) => 0.5,
        (Flavor::OpenAi, Some("fast")) | (_, Some("priority")) if model.id == "gpt-5.5" => 2.5,
        (Flavor::OpenAi, Some("fast")) | (_, Some("priority")) => 2.0,
        _ => 1.0,
    }
}

/// pi's `clampOpenAIPromptCacheKey`: at most 64 characters.
fn prompt_cache_key(session: &str) -> String {
    session.chars().take(64).collect()
}

/// Request fields for `azure-openai-responses`.
fn build_azure_params(
    model: &Model,
    messages: &[Message],
    options: &StreamOptions,
    compat: &Compat,
    grammar: &HashMap<String, String>,
    max_tokens: u64,
    effort: Option<ThinkingLevel>,
) -> Result<Value, String> {
    let (request_tools, _) = resolve_transcript_tools(
        messages,
        compat.supports_additional_tools || compat.supports_tool_search,
    );
    let input = convert_messages(model, messages, compat, grammar)?;
    let mut params = Map::new();
    params.insert("model".into(), json!(azure_deployment(model)));
    params.insert("input".into(), Value::Array(input));
    params.insert("stream".into(), json!(true));
    if let Some(session) = &options.session_id {
        params.insert("prompt_cache_key".into(), json!(prompt_cache_key(session)));
    }
    params.insert("store".into(), json!(false));
    if max_tokens > 0 {
        params.insert(
            "max_output_tokens".into(),
            json!(max_tokens.max(MIN_OUTPUT_TOKENS)),
        );
    }
    if let Some(temperature) = options.temperature {
        params.insert("temperature".into(), json!(temperature));
    }
    if !request_tools.is_empty() {
        params.insert(
            "tools".into(),
            Value::Array(convert_tools(&request_tools, compat, false)?),
        );
    }
    if model.reasoning {
        if let Some(level) = effort {
            let effort = match model.thinking_level_value(level) {
                Some(Some(value)) => value.to_owned(),
                _ => level.as_str().to_owned(),
            };
            params.insert(
                "reasoning".into(),
                json!({"effort": effort, "summary": "auto"}),
            );
            params.insert("include".into(), json!(["reasoning.encrypted_content"]));
        } else {
            match model.thinking_level_value(ThinkingLevel::Off) {
                Some(None) => {}
                Some(Some(value)) => {
                    params.insert("reasoning".into(), json!({"effort": value}));
                }
                None => {
                    params.insert("reasoning".into(), json!({"effort": "none"}));
                }
            }
        }
    }
    for (key, value) in model.sampling_params.iter().flatten() {
        params.insert(key.clone(), value.clone());
    }
    Ok(Value::Object(params))
}

/// The request body for `openai-codex-responses`: the initial system prompt
/// goes in `instructions`, and Codex's fixed fields follow it.
fn build_codex_body(
    model: &Model,
    messages: &[Message],
    options: &StreamOptions,
    compat: &Compat,
    grammar: &HashMap<String, String>,
    effort: Option<ThinkingLevel>,
) -> Result<Value, String> {
    let (request_tools, _) = resolve_transcript_tools(
        messages,
        compat.supports_additional_tools || compat.supports_tool_search,
    );
    let input = convert_messages(model, messages, compat, grammar)?;
    let instructions = match messages.first() {
        Some(Message::System(system)) => system.text(),
        _ => String::new(),
    };
    let mut body = Map::new();
    body.insert("model".into(), json!(model.id));
    body.insert("store".into(), json!(false));
    body.insert("stream".into(), json!(true));
    body.insert(
        "instructions".into(),
        json!(if instructions.is_empty() {
            "You are a helpful assistant."
        } else {
            instructions.as_str()
        }),
    );
    body.insert("input".into(), Value::Array(input));
    body.insert("text".into(), json!({"verbosity": "low"}));
    body.insert("include".into(), json!(["reasoning.encrypted_content"]));
    if options.resolved_cache_retention() != CacheRetention::None
        && let Some(session) = &options.session_id
    {
        body.insert("prompt_cache_key".into(), json!(prompt_cache_key(session)));
    }
    body.insert("tool_choice".into(), json!("auto"));
    body.insert("parallel_tool_calls".into(), json!(true));
    if let Some(temperature) = options.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if !request_tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(convert_tools(&request_tools, compat, false)?),
        );
    }
    if let Some(level) = effort {
        let effort = match model.thinking_level_value(level) {
            Some(Some(value)) => Some(value.to_owned()),
            Some(None) => None,
            None => Some(level.as_str().to_owned()),
        };
        if let Some(effort) = effort {
            body.insert(
                "reasoning".into(),
                json!({"effort": effort, "summary": "auto"}),
            );
        }
    } else if model.reasoning {
        match model.thinking_level_value(ThinkingLevel::Off) {
            Some(None) => {}
            Some(Some(value)) => {
                body.insert("reasoning".into(), json!({"effort": value}));
            }
            None => {
                body.insert("reasoning".into(), json!({"effort": "none"}));
            }
        }
    }
    Ok(Value::Object(body))
}

/// pi's `resolveAzureConfig`: the base URL from `AZURE_OPENAI_BASE_URL`, else
/// `AZURE_OPENAI_RESOURCE_NAME`, else the model, with `/openai/v1` added to
/// bare Azure hosts; and the API version.
fn azure_endpoint(model: &Model) -> Result<String, String> {
    let env = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let version = env("AZURE_OPENAI_API_VERSION").unwrap_or_else(|| "v1".into());
    let base = env("AZURE_OPENAI_BASE_URL")
        .or_else(|| env("AZURE_OPENAI_RESOURCE_NAME").map(|name| format!("https://{name}.openai.azure.com/openai/v1")))
        .or_else(|| Some(model.base_url.clone()).filter(|url| !url.is_empty()))
        .ok_or_else(|| "Azure OpenAI base URL is required. Set AZURE_OPENAI_BASE_URL or AZURE_OPENAI_RESOURCE_NAME, or pass azureBaseUrl, azureResourceName, or model.baseUrl.".to_owned())?;
    let trimmed = base.trim().trim_end_matches('/');
    let mut url =
        url::Url::parse(trimmed).map_err(|_| format!("Invalid Azure OpenAI base URL: {base}"))?;
    let host = url.host_str().unwrap_or_default().to_owned();
    let azure = [
        ".openai.azure.com",
        ".cognitiveservices.azure.com",
        ".ai.azure.com",
    ]
    .iter()
    .any(|suffix| host.ends_with(suffix));
    let path = url.path().trim_end_matches('/').to_owned();
    if azure && matches!(path.as_str(), "" | "/openai" | "/openai/v1/responses") {
        url.set_path("/openai/v1");
        url.set_query(None);
    }
    let base = url.to_string();
    let base = base.trim_end_matches('/');
    let separator = if base.contains('?') { '&' } else { '?' };
    Ok(format!("{base}/responses{separator}api-version={version}"))
}

/// The deployment for a model: `AZURE_OPENAI_DEPLOYMENT_NAME_MAP`
/// (`model=deployment,...`), else the model id.
fn azure_deployment(model: &Model) -> String {
    std::env::var("AZURE_OPENAI_DEPLOYMENT_NAME_MAP")
        .ok()
        .and_then(|map| {
            map.split(',').find_map(|entry| {
                let (id, deployment) = entry.trim().split_once('=')?;
                (id.trim() == model.id && !deployment.trim().is_empty())
                    .then(|| deployment.trim().to_owned())
            })
        })
        .unwrap_or_else(|| model.id.clone())
}

/// `<base>/codex/responses`, as pi's `resolveCodexUrl`.
fn codex_url(base_url: &str) -> String {
    let base = if base_url.trim().is_empty() {
        "https://chatgpt.com/backend-api"
    } else {
        base_url
    };
    let base = base.trim_end_matches('/');
    if base.ends_with("/codex/responses") {
        base.to_owned()
    } else if base.ends_with("/codex") {
        format!("{base}/responses")
    } else {
        format!("{base}/codex/responses")
    }
}

/// pi's `parseErrorResponse` for Codex: a usage-limit notice, else the
/// error's message, else the body.
fn codex_error_message(status: u16, body: &str) -> String {
    let mut message = if body.is_empty() {
        "Request failed".to_owned()
    } else {
        body.to_owned()
    };
    let mut friendly = None;
    if let Ok(parsed) = serde_json::from_str::<Value>(body)
        && let Some(error) = parsed.get("error").filter(|error| is_truthy(error))
    {
        let code = [&error["code"], &error["type"]]
            .into_iter()
            .find_map(|value| value.as_str().filter(|text| !text.is_empty()))
            .unwrap_or_default()
            .to_lowercase();
        let limited = [
            "usage_limit_reached",
            "usage_not_included",
            "rate_limit_exceeded",
        ]
        .iter()
        .any(|pattern| code.contains(pattern));
        if limited || status == 429 {
            let plan = error["plan_type"]
                .as_str()
                .map_or_else(String::new, |plan| {
                    format!(" ({} plan)", plan.to_lowercase())
                });
            let when = error["resets_at"]
                .as_f64()
                .filter(|at| *at != 0.0)
                .map_or_else(String::new, |at| {
                    let minutes = ((at * 1000.0 - now_ms() as f64) / 60000.0).round().max(0.0);
                    format!(" Try again in ~{minutes} min.")
                });
            friendly = Some(
                format!("You have hit your ChatGPT usage limit{plan}.{when}")
                    .trim()
                    .to_owned(),
            );
        }
        if let Some(text) = error["message"].as_str().filter(|text| !text.is_empty()) {
            message = text.to_owned();
        } else if let Some(friendly) = &friendly {
            message = friendly.clone();
        }
    }
    friendly.unwrap_or(message)
}

pub(super) async fn run(request: Request, sender: EventSender, flavor: Flavor) {
    let Request {
        model,
        messages,
        options,
    } = request;
    let compat = Compat::new(&model, flavor);
    let output = new_output(&model, now_ms());
    let normalized = resolve_transcript(&messages, compat.supports_mid_convo_system_messages);
    // Calls are stored as ordinary tool calls. Each request decides from the
    // declared tools which ones travel as custom tool calls.
    let grammar = match schema::grammar_properties(&normalized, compat.supports_grammar_tools) {
        Ok(grammar) => grammar,
        Err(message) => return send_error(&sender, output, &options.cancel, message),
    };
    let mut state = State::new(flavor, output, grammar);
    let prepared = match prepare(
        &model,
        &messages,
        &normalized,
        &options,
        &compat,
        &state.grammar,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(message) => return send_error(&sender, state.output, &options.cancel, message),
    };
    if let Some(account) = &prepared.account {
        match codex_websocket(&prepared, account, &mut state, &model, &sender, &options).await {
            Ok(true) => return sender.send(StreamEvent::Done(state.output)),
            Ok(false) => {}
            Err(message) => return send_error(&sender, state.output, &options.cancel, message),
        }
    }
    let response = match send(&prepared, &model, &options, flavor).await {
        Ok(response) => response,
        Err(message) => return send_error(&sender, state.output, &options.cancel, message),
    };
    options.hooks.response(&response).await;
    state.start(&sender);
    let result = state
        .consume(SseReader::new(response), &model, &sender, &options)
        .await
        .and_then(|()| check_complete(&state.output, &options.cancel, flavor.pending_message()));
    match result {
        Ok(()) => sender.send(StreamEvent::Done(state.output)),
        Err(message) => send_error(&sender, state.output, &options.cancel, message),
    }
}

/// A request ready to send, after the payload hook.
struct Prepared {
    url: String,
    headers: Headers,
    body: Value,
    /// The ChatGPT account of a Codex request.
    account: Option<String>,
}

/// Builds the request.
async fn prepare(
    model: &Model,
    messages: &[Message],
    normalized: &[Message],
    options: &StreamOptions,
    compat: &Compat,
    grammar: &HashMap<String, String>,
) -> Result<Prepared, String> {
    let flavor = compat.flavor;
    let api_key = match options.api_key.clone().filter(|key| !key.is_empty()) {
        Some(key) => key,
        None if flavor == Flavor::OpenAi
            && (options.has_header("authorization")
                || options.has_header("cf-aig-authorization")) =>
        {
            "unused".to_owned()
        }
        None => return Err(format!("No API key for provider: {}", model.provider)),
    };
    let max_tokens = requested_max_tokens(model, messages, options);
    let effort = effort(model, options);

    let mut headers = Headers::default();
    let mut account = None;
    let session = options
        .session_id
        .as_deref()
        .filter(|_| options.resolved_cache_retention() != CacheRetention::None);
    let (params, url) = match flavor {
        Flavor::OpenAi => {
            let params = build_params(
                model, normalized, options, compat, grammar, max_tokens, effort,
            )?;
            headers = super::openai_headers(&api_key, model.headers.as_ref());
            if model.provider == "github-copilot" {
                for (key, value) in super::copilot_headers(messages) {
                    headers.set(key, Some(value));
                }
            }
            if let Some(session) = session {
                if compat.session_affinity_format == "openrouter" {
                    headers.set("x-session-id", Some(session));
                } else {
                    if compat.session_affinity_format == "openai" {
                        headers.set("session_id", Some(session));
                    }
                    headers.set("x-client-request-id", Some(session));
                }
            }
            headers.extend(&options.headers);
            let url = format!("{}/responses", model.base_url.trim_end_matches('/'));
            (params, url)
        }
        Flavor::Azure => {
            let url = azure_endpoint(model)?;
            let params = build_azure_params(
                model, normalized, options, compat, grammar, max_tokens, effort,
            )?;
            headers.set("api-key", Some(api_key.as_str()));
            headers.set("content-type", Some("application/json"));
            headers.set("accept", Some("application/json"));
            headers.extend_model(model.headers.as_ref());
            headers.extend(&options.headers);
            (params, url)
        }
        Flavor::Codex => {
            let id = crate::auth::codex::account_id(&api_key)
                .ok_or("Failed to extract accountId from token")?;
            let body = build_codex_body(model, normalized, options, compat, grammar, effort)?;
            headers.extend_model(model.headers.as_ref());
            headers.extend(&options.headers);
            headers.set("authorization", Some(format!("Bearer {api_key}")));
            headers.set("chatgpt-account-id", Some(id.as_str()));
            account = Some(id);
            // The value the Codex backend expects from this client id.
            headers.set("originator", Some("pi"));
            headers.set("openai-beta", Some("responses=experimental"));
            headers.set("accept", Some("text/event-stream"));
            headers.set("content-type", Some("application/json"));
            if let Some(session) = session {
                let session = prompt_cache_key(session);
                headers.set("session-id", Some(session.as_str()));
                headers.set("x-client-request-id", Some(session));
            }
            (body, codex_url(&model.base_url))
        }
    };
    let body = options.hooks.payload(params).await;
    if flavor == Flavor::Codex
        && let Some(timeout) = options.timeout_ms.filter(|ms| !ms.is_finite() || *ms < 0.0)
    {
        return Err(format!(
            "Invalid timeoutMs: {}",
            yapi_types::json::stringify(&json!(timeout))
        ));
    }
    Ok(Prepared {
        url,
        headers,
        body,
        account,
    })
}

/// Sends `prepared` over HTTP; the response once its status is a success.
async fn send(
    prepared: &Prepared,
    model: &Model,
    options: &StreamOptions,
    flavor: Flavor,
) -> Result<reqwest::Response, String> {
    let mut headers = prepared.headers.clone();
    let mut body = yapi_types::json::stringify(&prepared.body).into_bytes();
    if flavor == Flavor::Codex {
        // ruzstd implements only its fastest level, near zstd's level 1, so the
        // bytes differ from pi's level 3. The Codex backend decodes either.
        body = ruzstd::encoding::compress_to_vec(
            body.as_slice(),
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        headers.set("content-encoding", Some("zstd"));
    }
    let build = || headers.apply(http::client().post(&prepared.url).body(body.clone()));
    http::send(build, options)
        .await
        .map_err(|failure| match (flavor, failure) {
            (Flavor::Codex, http::Failure::Status { status, body }) => {
                codex_error_message(status, &body)
            }
            (Flavor::Codex, http::Failure::Aborted) => http::ABORTED_DURING_STREAM.to_owned(),
            (Flavor::Azure, failure) => failure_message(failure, "Azure OpenAI API error"),
            (_, failure) => {
                let prefix = if model.provider == "openai" {
                    "OpenAI API error".to_owned()
                } else {
                    format!("{} API error", model.provider)
                };
                failure_message(failure, &prefix)
            }
        })
}

/// pi's WebSocket attempt for Codex, retried once when the backend's
/// connection limit is reached before any event and once when it lost the
/// continued response. `Ok(true)` once the response streamed, `Ok(false)` to
/// fall back to SSE, or the error that ends the request.
async fn codex_websocket(
    prepared: &Prepared,
    account: &str,
    state: &mut State,
    model: &Model,
    sender: &EventSender,
    options: &StreamOptions,
) -> Result<bool, String> {
    let transport = options.transport.unwrap_or(Transport::Auto);
    let session = options
        .session_id
        .as_deref()
        .filter(|_| options.resolved_cache_retention() != CacheRetention::None);
    if transport == Transport::Sse || codex_websocket::fallback_active(session) {
        return Ok(false);
    }
    let request_id = session.map_or_else(yapi_types::time::uuid_v7, prompt_cache_key);
    let mut headers = prepared.headers.clone();
    headers.set("accept", None::<String>);
    headers.set("content-type", None::<String>);
    headers.set("openai-beta", Some("responses_websockets=2026-02-06"));
    headers.set("x-client-request-id", Some(request_id.as_str()));
    headers.set("session-id", Some(request_id));
    let socket = CodexSocket {
        url: &prepared.url,
        headers,
        key: session.map(|session| (session.to_owned(), account.to_owned())),
        // Continuing needs the transport set explicitly, as pi's agent sets it.
        continues: matches!(
            options.transport,
            Some(Transport::Auto | Transport::WebsocketCached)
        ),
    };
    let (mut retried_limit, mut retried_missing) = (false, false);
    loop {
        let result = codex_websocket_once(&socket, &prepared.body, state, model, sender, options)
            .await
            .and_then(|()| {
                check_complete(
                    &state.output,
                    &options.cancel,
                    Flavor::Codex.pending_message(),
                )
                .map_err(Failure::error)
            });
        let Err(failure) = result else {
            return Ok(true);
        };
        let cancelled = options.cancel.is_cancelled();
        if !cancelled
            && !retried_missing
            && failure.api_code() == Some("previous_response_not_found")
        {
            retried_missing = true;
            continue;
        }
        let limit_before_start =
            !state.events && failure.api_code() == Some("websocket_connection_limit_reached");
        if !cancelled && limit_before_start && !retried_limit {
            retried_limit = true;
            continue;
        }
        if cancelled || (!failure.transport && !limit_before_start) {
            return Err(failure.message);
        }
        let bytes = yapi_types::json::stringify(&prepared.body).len();
        let diagnostic = failure.diagnostic(transport, state.events, bytes);
        state
            .output
            .diagnostics
            .get_or_insert_with(Vec::new)
            .push(diagnostic);
        codex_websocket::record_failure(session);
        return if state.events {
            Err(failure.message)
        } else {
            Ok(false)
        };
    }
}

/// Where and how a Codex request opens its WebSocket.
struct CodexSocket<'a> {
    url: &'a str,
    headers: Headers,
    /// The session and account whose cached socket the request uses.
    key: Option<(String, String)>,
    /// Whether a request on a reused socket sends only what follows the
    /// response it last carried.
    continues: bool,
}

/// One request over a WebSocket: pi's `processWebSocketStream`.
async fn codex_websocket_once(
    socket: &CodexSocket<'_>,
    body: &Value,
    state: &mut State,
    model: &Model,
    sender: &EventSender,
    options: &StreamOptions,
) -> Result<(), Failure> {
    state.restart();
    let mut lease = codex_websocket::acquire(
        socket.url,
        &socket.headers,
        socket.key.clone(),
        &options.cancel,
        options.websocket_connect_timeout_ms,
    )
    .await?;
    let continues = socket.continues && lease.cached();
    let request = if continues {
        codex_websocket::continued(body, &mut lease.continuation)
    } else {
        body.clone()
    };
    let mut result = state
        .consume_websocket(&mut lease, &request, model, sender, options)
        .await;
    if result.is_ok()
        && continues
        && !options.cancel.is_cancelled()
        && let Some(response_id) = state.output.response_id.clone()
    {
        match response_items(model, &state.output, &state.grammar) {
            Ok(items) => {
                lease.continuation = Some(Continuation {
                    body: body.clone(),
                    response_id,
                    items,
                });
            }
            Err(message) => result = Err(Failure::error(message)),
        }
    }
    if result.is_err() {
        lease.continuation = None;
    }
    lease.release(result.is_ok() && !options.cancel.is_cancelled());
    result
}

/// The input items that repeat `output` in a later request with the same
/// grammar tools, without the tool results the conversion adds for its calls.
fn response_items(
    model: &Model,
    output: &AssistantMessage,
    grammar: &HashMap<String, String>,
) -> Result<Vec<Value>, String> {
    let message = Message::Assistant(Box::new(output.clone()));
    let compat = Compat::new(model, Flavor::Codex);
    let items = convert_messages(model, &[message], &compat, grammar)?;
    Ok(items
        .into_iter()
        .filter(|item| {
            item["type"] != "function_call_output" && item["type"] != "custom_tool_call_output"
        })
        .collect())
}

/// The code of a Codex `error` or `response.failed` event.
fn codex_error_code(event: &Value) -> Option<&str> {
    [
        &event["code"],
        &event["error"]["code"],
        &event["response"]["error"]["code"],
    ]
    .into_iter()
    .find_map(Value::as_str)
}

fn failure_message(failure: http::Failure, prefix: &str) -> String {
    let message = match failure {
        http::Failure::Status { status, body } => {
            http::openai_status_message(status, &body, Some(prefix))
        }
        other => other.plain_message().unwrap_or_default(),
    };
    if message.contains("subscription_sharing_usage_limit_exceeded") {
        format!("{message}\nCheck your ChatGPT usage: {CHATGPT_USAGE_URL}")
    } else {
        message
    }
}

/// What an output index streams into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Thinking(usize),
    Text(usize),
    ToolCall(usize),
}

struct State {
    flavor: Flavor,
    output: AssistantMessage,
    /// The input argument of each tool sent with a grammar, by name.
    grammar: HashMap<String, String>,
    slots: IndexMap<u64, Slot>,
    /// Argument text of function calls still streaming, by content position.
    partial_args: IndexMap<usize, String>,
    /// Input of custom tool calls still streaming, by content position.
    custom_inputs: IndexMap<usize, GrammarInput>,
    reasoning_by_id: IndexMap<String, usize>,
    terminal: bool,
    /// Whether the start event was sent.
    started: bool,
    /// Whether this attempt has had an event other than an error.
    events: bool,
}

/// A JS template-literal rendering of a JSON value.
fn template(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "null".into(),
        other => other.to_string(),
    }
}

fn string_or<'a>(value: &'a Value, fallback: &'a str) -> &'a str {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .unwrap_or(fallback)
}

impl State {
    fn new(flavor: Flavor, output: AssistantMessage, grammar: HashMap<String, String>) -> State {
        State {
            flavor,
            output,
            grammar,
            slots: IndexMap::new(),
            partial_args: IndexMap::new(),
            custom_inputs: IndexMap::new(),
            reasoning_by_id: IndexMap::new(),
            terminal: false,
            started: false,
            events: false,
        }
    }

    /// Forgets a failed attempt's stream before another, keeping the message.
    fn restart(&mut self) {
        self.slots.clear();
        self.partial_args.clear();
        self.custom_inputs.clear();
        self.reasoning_by_id.clear();
        self.terminal = false;
        self.events = false;
    }

    /// Sends the start event, once.
    fn start(&mut self, sender: &EventSender) {
        if !self.started {
            self.started = true;
            sender.send(StreamEvent::Start(self.output.clone()));
        }
    }

    fn apply_phase(&mut self, item: &Value) {
        if item["type"] == "message" && item["phase"] == "final_answer" {
            self.output.stop_reason = StopReason::Stop;
        }
    }

    fn create_slot(&mut self, index: u64, item: &Value, sender: &EventSender) -> Option<Slot> {
        let position = self.output.content.len();
        let (slot, block) = match item["type"].as_str()? {
            "reasoning" => (Slot::Thinking(position), ContentBlock::thinking("", None)),
            "message" => {
                self.apply_phase(item);
                (Slot::Text(position), ContentBlock::text(""))
            }
            "function_call" => {
                self.partial_args
                    .insert(position, string_or(&item["arguments"], "").to_owned());
                let call = ToolCall {
                    id: call_id(item),
                    name: item["name"].as_str().unwrap_or_default().to_owned(),
                    arguments: Map::new(),
                    thought_signature: None,
                    namespace: item["namespace"].as_str().map(str::to_owned),
                };
                (Slot::ToolCall(position), ContentBlock::ToolCall(call))
            }
            "custom_tool_call" => {
                let name = item["name"].as_str().unwrap_or_default();
                let custom = GrammarInput::for_call(&self.grammar, name);
                let mut arguments = Map::new();
                arguments.insert(
                    custom.property.clone(),
                    json!(string_or(&item["input"], "")),
                );
                self.custom_inputs.insert(position, custom);
                let call = ToolCall {
                    id: call_id(item),
                    name: name.to_owned(),
                    arguments,
                    thought_signature: None,
                    namespace: item["namespace"].as_str().map(str::to_owned),
                };
                (Slot::ToolCall(position), ContentBlock::ToolCall(call))
            }
            _ => return None,
        };
        self.slots.insert(index, slot);
        sender.start(&mut self.output, block);
        Some(slot)
    }

    fn thinking(&mut self, position: usize) -> Option<&mut ThinkingContent> {
        match self.output.content.get_mut(position) {
            Some(ContentBlock::Thinking(thinking)) => Some(thinking),
            _ => None,
        }
    }

    fn text(&mut self, position: usize) -> Option<&mut TextContent> {
        match self.output.content.get_mut(position) {
            Some(ContentBlock::Text(text)) => Some(text),
            _ => None,
        }
    }

    fn tool_call(&mut self, position: usize) -> Option<&mut ToolCall> {
        match self.output.content.get_mut(position) {
            Some(ContentBlock::ToolCall(call)) => Some(call),
            _ => None,
        }
    }

    fn thinking_delta(&mut self, index: u64, delta: &str, sender: &EventSender) {
        if let Some(Slot::Thinking(position)) = self.slots.get(&index).copied() {
            sender.delta(&mut self.output, position, delta);
        }
    }

    fn text_delta(&mut self, index: u64, delta: &str, sender: &EventSender) {
        if let Some(Slot::Text(position)) = self.slots.get(&index).copied() {
            sender.delta(&mut self.output, position, delta);
        }
    }

    fn streaming_call(&self, index: u64) -> Option<usize> {
        match self.slots.get(&index) {
            Some(Slot::ToolCall(position)) if self.partial_args.contains_key(position) => {
                Some(*position)
            }
            _ => None,
        }
    }

    fn set_partial_args(&mut self, position: usize, partial: String) {
        let arguments = parse_streaming_json(&partial);
        self.partial_args.insert(position, partial);
        if let Some(call) = self.tool_call(position) {
            call.arguments = arguments;
        }
    }

    fn custom_call(&self, index: u64) -> Option<usize> {
        match self.slots.get(&index) {
            Some(Slot::ToolCall(position)) if self.custom_inputs.contains_key(position) => {
                Some(*position)
            }
            _ => None,
        }
    }

    /// The input a custom tool call carries so far.
    fn custom_input(&self, position: usize) -> String {
        match (
            self.custom_inputs.get(&position),
            self.output.content.get(position),
        ) {
            (Some(custom), Some(ContentBlock::ToolCall(call))) => {
                custom.input(&call.arguments).to_owned()
            }
            _ => String::new(),
        }
    }

    /// Sets a custom tool call's input and streams what it adds to the
    /// arguments JSON.
    fn set_custom_input(
        &mut self,
        position: usize,
        input: String,
        close: bool,
        sender: &EventSender,
    ) -> Result<(), String> {
        let (Some(custom), Some(ContentBlock::ToolCall(call))) = (
            self.custom_inputs.get_mut(&position),
            self.output.content.get_mut(position),
        ) else {
            return Ok(());
        };
        if let Some(delta) = custom.set(&mut call.arguments, input, close)? {
            sender.delta(&mut self.output, position, &delta);
        }
        Ok(())
    }

    fn item_done(&mut self, index: u64, item: &Value, sender: &EventSender) -> Result<(), String> {
        self.apply_phase(item);
        let slot = match self.slots.get(&index).copied() {
            Some(slot) => Some(slot),
            None => self.create_slot(index, item, sender),
        };
        match (item["type"].as_str(), slot) {
            (Some("reasoning"), Some(Slot::Thinking(position))) => {
                let join = |key: &str| {
                    item[key]
                        .as_array()
                        .map(|parts| {
                            parts
                                .iter()
                                .map(|part| join_part(&part["text"]))
                                .collect::<Vec<_>>()
                                .join("\n\n")
                        })
                        .unwrap_or_default()
                };
                let (summary, content) = (join("summary"), join("content"));
                let signature = yapi_types::json::stringify(item);
                let Some(block) = self.thinking(position) else {
                    return Ok(());
                };
                if !summary.is_empty() {
                    block.thinking = summary;
                } else if !content.is_empty() {
                    block.thinking = content;
                }
                block.thinking_signature = Some(signature);
                if let Some(id) = item["id"].as_str() {
                    self.reasoning_by_id.insert(id.to_owned(), position);
                }
                sender.end(&self.output, position);
                self.slots.shift_remove(&index);
            }
            (Some("message"), Some(Slot::Text(position))) => {
                let text: String = item["content"]
                    .as_array()
                    .map(|parts| {
                        parts
                            .iter()
                            .map(|part| {
                                if part["type"] == "output_text" {
                                    join_part(&part["text"])
                                } else {
                                    join_part(&part["refusal"])
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let signature = encode_text_signature(
                    item["id"].as_str(),
                    item["phase"].as_str().filter(|phase| !phase.is_empty()),
                );
                let Some(block) = self.text(position) else {
                    return Ok(());
                };
                block.text = text;
                block.text_signature = Some(signature);
                sender.end(&self.output, position);
                self.slots.shift_remove(&index);
            }
            (Some("function_call"), Some(Slot::ToolCall(position)))
                if self.partial_args.contains_key(&position) =>
            {
                let partial = self
                    .partial_args
                    .shift_remove(&position)
                    .unwrap_or_default();
                let source = item["arguments"]
                    .as_str()
                    .filter(|arguments| !arguments.is_empty())
                    .map(str::to_owned)
                    .unwrap_or(if partial.is_empty() {
                        "{}".to_owned()
                    } else {
                        partial
                    });
                let arguments = parse_streaming_json(&source);
                let Some(call) = self.tool_call(position) else {
                    return Ok(());
                };
                call.arguments = arguments;
                if let Some(namespace) = item["namespace"].as_str() {
                    call.namespace = Some(namespace.to_owned());
                }
                sender.end(&self.output, position);
                self.slots.shift_remove(&index);
            }
            (Some("custom_tool_call"), Some(Slot::ToolCall(position)))
                if self.custom_inputs.contains_key(&position) =>
            {
                let input = item["input"]
                    .as_str()
                    .map_or_else(|| self.custom_input(position), str::to_owned);
                self.set_custom_input(position, input, true, sender)?;
                self.custom_inputs.shift_remove(&position);
                if let Some(namespace) = item["namespace"].as_str()
                    && let Some(call) = self.tool_call(position)
                {
                    call.namespace = Some(namespace.to_owned());
                }
                sender.end(&self.output, position);
                self.slots.shift_remove(&index);
            }
            _ => {}
        }
        Ok(())
    }

    /// Azure can leave `encrypted_content` out of `output_item.done` and send it
    /// only in the final response; copy it into the stored reasoning items.
    fn backfill_reasoning(&mut self, output: &Value) {
        for item in output.as_array().into_iter().flatten() {
            let Some(encrypted) = item["encrypted_content"]
                .as_str()
                .filter(|text| !text.is_empty())
            else {
                continue;
            };
            if item["type"] != "reasoning" {
                continue;
            }
            let Some(position) = item["id"]
                .as_str()
                .and_then(|id| self.reasoning_by_id.get(id).copied())
            else {
                continue;
            };
            let Some(block) = self.thinking(position) else {
                continue;
            };
            let Some(mut stored) = block
                .thinking_signature
                .as_deref()
                .and_then(|signature| serde_json::from_str::<Map<String, Value>>(signature).ok())
            else {
                continue;
            };
            if stored
                .get("encrypted_content")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.is_empty())
            {
                continue;
            }
            stored.insert("encrypted_content".into(), json!(encrypted));
            block.thinking_signature = Some(yapi_types::json::stringify(&stored));
        }
    }

    fn finalize(&mut self, response: &Value, model: &Model) -> Result<(), String> {
        self.terminal = true;
        self.backfill_reasoning(&response["output"]);
        if let Some(id) = response["id"].as_str().filter(|id| !id.is_empty()) {
            self.output.response_id = Some(id.to_owned());
        }
        let usage = &response["usage"];
        if usage.is_object() {
            let count = |value: &Value| value.as_u64().unwrap_or(0);
            let cached = count(&usage["input_tokens_details"]["cached_tokens"]);
            let written = count(&usage["input_tokens_details"]["cache_write_tokens"]);
            let usage = yapi_types::message::Usage {
                input: count(&usage["input_tokens"]).saturating_sub(cached + written),
                output: count(&usage["output_tokens"]),
                cache_read: cached,
                cache_write: written,
                reasoning: Some(count(&usage["output_tokens_details"]["reasoning_tokens"])),
                total_tokens: Some(count(&usage["total_tokens"])),
                ..Default::default()
            };
            self.output.usage = usage;
        }
        calculate_cost(model, &mut self.output.usage);
        let multiplier =
            service_tier_multiplier(model, response["service_tier"].as_str(), self.flavor);
        if multiplier != 1.0 {
            let cost = &mut self.output.usage.cost;
            cost.input *= multiplier;
            cost.output *= multiplier;
            cost.cache_read *= multiplier;
            cost.cache_write *= multiplier;
            cost.total = cost.input + cost.output + cost.cache_read + cost.cache_write;
        }
        let status = response["status"].as_str();
        let reason = response["incomplete_details"]["reason"].as_str();
        self.output.raw_stop_reason = match (status, reason) {
            (Some(status), Some(reason)) => Some(format!("{status}.{reason}")),
            (None, Some(reason)) => Some(format!("undefined.{reason}")),
            (status, None) => status.map(str::to_owned),
        };
        let (stop, message) = match status {
            None | Some("completed" | "in_progress" | "queued") => (StopReason::Stop, None),
            Some("incomplete") => match reason {
                Some("max_output_tokens") => (StopReason::Length, None),
                Some(reason) => (
                    StopReason::Error,
                    Some(format!("Response incomplete: {reason}")),
                ),
                None => (
                    StopReason::Error,
                    Some("Response incomplete without a provider reason".to_owned()),
                ),
            },
            Some("failed" | "cancelled") => (StopReason::Error, None),
            Some(other) => return Err(format!("Unhandled stop reason: {other}")),
        };
        self.output.stop_reason = stop;
        self.output.error_message = message;
        if stop == StopReason::Stop
            && self
                .output
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::ToolCall(_)))
        {
            self.output.stop_reason = StopReason::ToolUse;
        }
        Ok(())
    }

    /// pi's `mapCodexEvents`: Codex errors and its terminal events. Returns
    /// `true` when the event was handled.
    fn handle_codex(&mut self, event: &Value, model: &Model) -> Result<bool, String> {
        match event["type"].as_str().unwrap_or_default() {
            "error" => {
                let nested = &event["error"];
                let pick = |name: &str| {
                    event[name]
                        .as_str()
                        .or_else(|| nested[name].as_str())
                        .filter(|text| !text.is_empty())
                        .map(str::to_owned)
                };
                let detail = pick("message")
                    .or_else(|| pick("code"))
                    .unwrap_or_else(|| yapi_types::json::stringify(event));
                Err(format!("Codex error: {detail}"))
            }
            "response.failed" => Err(event["response"]["error"]["message"]
                .as_str()
                .filter(|text| !text.is_empty())
                .unwrap_or("Codex response failed")
                .to_owned()),
            "response.done" | "response.completed" | "response.incomplete" => {
                let mut response = event["response"].clone();
                if let Some(end_turn) = response["end_turn"].as_bool() {
                    self.output.end_turn = Some(end_turn);
                }
                let known = [
                    "completed",
                    "incomplete",
                    "failed",
                    "cancelled",
                    "queued",
                    "in_progress",
                ];
                if let Some(object) = response.as_object_mut()
                    && !object
                        .get("status")
                        .and_then(Value::as_str)
                        .is_some_and(|status| known.contains(&status))
                {
                    object.remove("status");
                }
                self.finalize(&response, model)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle(&mut self, event: &Value, model: &Model, sender: &EventSender) -> Result<(), String> {
        if self.flavor == Flavor::Codex && self.handle_codex(event, model)? {
            return Ok(());
        }
        let index = event["output_index"].as_u64().unwrap_or(0);
        let delta = event["delta"].as_str().unwrap_or_default();
        match event["type"].as_str().unwrap_or_default() {
            "response.created" => {
                self.output.response_id = event["response"]["id"].as_str().map(str::to_owned);
            }
            "response.output_item.added" => {
                self.create_slot(index, &event["item"], sender);
            }
            "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                self.thinking_delta(index, delta, sender);
            }
            "response.reasoning_summary_part.done" => self.thinking_delta(index, "\n\n", sender),
            "response.output_text.delta" | "response.refusal.delta" => {
                self.text_delta(index, delta, sender);
            }
            "response.function_call_arguments.delta" => {
                if let Some(position) = self.streaming_call(index) {
                    let partial = format!("{}{delta}", self.partial_args[&position]);
                    self.set_partial_args(position, partial);
                    sender.delta(&mut self.output, position, delta);
                }
            }
            "response.function_call_arguments.done" => {
                if let Some(position) = self.streaming_call(index) {
                    let previous = self.partial_args[&position].clone();
                    let arguments = event["arguments"].as_str().unwrap_or_default().to_owned();
                    self.set_partial_args(position, arguments.clone());
                    if let Some(rest) = arguments
                        .strip_prefix(previous.as_str())
                        .filter(|rest| !rest.is_empty())
                    {
                        sender.delta(&mut self.output, position, rest);
                    }
                }
            }
            "response.custom_tool_call_input.delta" => {
                if let Some(position) = self.custom_call(index) {
                    let input = format!("{}{delta}", self.custom_input(position));
                    self.set_custom_input(position, input, false, sender)?;
                }
            }
            "response.custom_tool_call_input.done" => {
                if let Some(position) = self.custom_call(index) {
                    let input = event["input"].as_str().unwrap_or_default().to_owned();
                    self.set_custom_input(position, input, true, sender)?;
                }
            }
            "response.output_item.done" => self.item_done(index, &event["item"], sender)?,
            "response.completed" | "response.incomplete" => {
                self.finalize(&event["response"], model)?;
            }
            "error" => {
                return Err(format!(
                    "Error Code {}: {}",
                    template_or_undefined(event.get("code")),
                    template_or_undefined(event.get("message"))
                ));
            }
            "response.failed" => {
                self.terminal = true;
                let response = &event["response"];
                self.output.raw_stop_reason = response["status"].as_str().map(str::to_owned);
                let error = &response["error"];
                let reason = &response["incomplete_details"]["reason"];
                return Err(if is_truthy(error) {
                    format!(
                        "{}: {}",
                        truthy_or(&error["code"], "unknown"),
                        truthy_or(&error["message"], "no message")
                    )
                } else if is_truthy(reason) {
                    format!("incomplete: {}", template(reason))
                } else {
                    "Unknown error (no error details in response)".to_owned()
                });
            }
            _ => {}
        }
        Ok(())
    }

    async fn consume(
        &mut self,
        mut reader: SseReader,
        model: &Model,
        sender: &EventSender,
        options: &StreamOptions,
    ) -> Result<(), String> {
        loop {
            // The SDK ends the iteration quietly when the request is aborted.
            let chunk = match http::next_openai_chunk(&mut reader, &options.cancel).await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(_) if options.cancel.is_cancelled() => break,
                Err(message) => return Err(message),
            };
            options.hooks.stream_event(&chunk).await;
            self.handle(&chunk, model, sender)?;
            // Codex ends its stream at the first terminal response.
            if self.flavor == Flavor::Codex && self.terminal {
                break;
            }
        }
        self.finish()
    }

    /// pi's WebSocket event loop: sends `request`, then handles events up to
    /// the terminal one. Error events fail as the backend's errors.
    async fn consume_websocket(
        &mut self,
        lease: &mut codex_websocket::Lease,
        request: &Value,
        model: &Model,
        sender: &EventSender,
        options: &StreamOptions,
    ) -> Result<(), Failure> {
        lease.send(codex_websocket::frame(request)).await?;
        while !self.terminal {
            let text = lease.next(&options.cancel).await?;
            let event: Value = serde_json::from_str(&text)
                .map_err(|err| Failure::protocol(format!("Invalid Codex WebSocket JSON: {err}")))?;
            options.hooks.stream_event(&event).await;
            let Some(kind) = event["type"].as_str() else {
                continue;
            };
            let error = matches!(kind, "error" | "response.failed");
            if !error {
                self.events = true;
                self.start(sender);
            }
            self.handle(&event, model, sender).map_err(|message| {
                if error {
                    Failure::api(message, codex_error_code(&event))
                } else {
                    Failure::error(message)
                }
            })?;
        }
        self.finish().map_err(Failure::error)
    }

    /// pi's checks once a stream ends: a terminal event arrived and every
    /// tool call finished.
    fn finish(&self) -> Result<(), String> {
        if !self.terminal {
            return Err("OpenAI Responses stream ended before a terminal response event".into());
        }
        if self.output.stop_reason == StopReason::ToolUse
            && let Some(call) =
                self.output
                    .content
                    .iter()
                    .enumerate()
                    .find_map(|(position, block)| match block {
                        ContentBlock::ToolCall(call)
                            if self.partial_args.contains_key(&position)
                                || self.custom_inputs.contains_key(&position) =>
                        {
                            Some(call)
                        }
                        _ => None,
                    })
        {
            return Err(format!(
                "OpenAI Responses stream completed with an unfinished tool call: {} ({})",
                call.name, call.id
            ));
        }
        Ok(())
    }
}

/// An element as `Array.prototype.join` renders it: null and undefined are empty.
fn join_part(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        other => template(other),
    }
}

/// A streamed call's `call|item` id.
fn call_id(item: &Value) -> String {
    format!(
        "{}|{}",
        template_or_undefined(item.get("call_id")),
        template_or_undefined(item.get("id"))
    )
}

fn template_or_undefined(value: Option<&Value>) -> String {
    value.map_or_else(|| "undefined".to_owned(), template)
}

fn truthy_or(value: &Value, fallback: &str) -> String {
    if is_truthy(value) {
        template(value)
    } else {
        fallback.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_round_trip() {
        let signature = encode_text_signature(Some("msg_1"), Some("final_answer"));
        assert_eq!(signature, r#"{"v":1,"id":"msg_1","phase":"final_answer"}"#);
        assert_eq!(
            parse_text_signature(Some(&signature)),
            Some(("msg_1".into(), Some("final_answer".into())))
        );
        assert_eq!(
            parse_text_signature(Some("legacy")),
            Some(("legacy".into(), None))
        );
        assert_eq!(parse_text_signature(Some("")), None);
    }

    #[test]
    fn normalizes_ids_for_responses() {
        assert_eq!(normalize_id_part("call.1__"), "call_1");
        assert_eq!(normalize_id_part("a😀b"), "a__b");
        assert_eq!(
            foreign_item_id("item"),
            format!("fc_{}", short_hash("item"))
        );
    }

    #[test]
    fn replays_grammar_calls_by_the_current_tools() {
        let model: Model = serde_json::from_value(json!({
            "id": "gpt-5.1", "name": "G", "api": "openai-responses", "provider": "openai",
            "baseUrl": "u", "reasoning": false, "input": ["text"],
            "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            "contextWindow": 1000, "maxTokens": 100
        }))
        .unwrap();
        let call = |code: Value| {
            ContentBlock::tool_call(
                "call_1|ctc_1",
                "codemode",
                Map::from_iter([("code".to_owned(), code)]),
            )
        };
        let mut assistant = new_output(&model, 0);
        assistant.content.push(call(json!("return 1;")));
        let grammar = HashMap::from([("codemode".to_owned(), "code".to_owned())]);
        assert_eq!(
            convert_assistant(&assistant, &model, 0, &grammar).unwrap(),
            [
                json!({"type": "custom_tool_call", "id": "ctc_1", "call_id": "call_1",
                "name": "codemode", "input": "return 1;"})
            ]
        );
        // Without grammar support the call travels as a function call, which
        // cannot keep the custom item id.
        assert_eq!(
            convert_assistant(&assistant, &model, 0, &HashMap::new()).unwrap(),
            [
                json!({"type": "function_call", "call_id": "call_1", "name": "codemode",
                "arguments": "{\"code\":\"return 1;\"}"})
            ]
        );
        // OpenAI pairs item ids with another model's reasoning, so they go.
        let mut other = assistant.clone();
        other.model = "gpt-5".into();
        assert_eq!(
            convert_assistant(&other, &model, 0, &grammar).unwrap(),
            [
                json!({"type": "custom_tool_call", "call_id": "call_1", "name": "codemode",
                "input": "return 1;"})
            ]
        );
        assistant.content[0] = call(json!(1));
        assert_eq!(
            convert_assistant(&assistant, &model, 0, &grammar).unwrap_err(),
            "Grammar tool call \"codemode\" requires argument \"code\" to be a string."
        );
    }

    #[test]
    fn a_removed_grammar_tool_replays_as_a_function_without_mid_conversation_system_messages() {
        let model: Model = serde_json::from_value(json!({
            "id": "gpt-5.1", "name": "G", "api": "openai-responses", "provider": "openai",
            "baseUrl": "u", "reasoning": false, "input": ["text"],
            "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            "contextWindow": 1000, "maxTokens": 100,
            "compat": {"supportsOpenAIGrammarTools": true}
        }))
        .unwrap();
        let grammar_tool = |name: &str, parameters: Value| {
            json!({"name": name, "description": "", "parameters": parameters,
                "constrainedSampling": {"type": "grammar", "variants": {"openai_regex": "a+"}}})
        };
        let codemode = grammar_tool(
            "codemode",
            json!({"type": "object", "required": ["code"], "properties": {"code": {"type": "string"}}}),
        );
        // No usable grammar: declaring it now would fail the request.
        let broken = grammar_tool("broken", json!({"type": "object"}));
        let mut assistant = new_output(&model, 0);
        assistant.content.push(ContentBlock::tool_call(
            "call_1|ctc_1",
            "codemode",
            Map::from_iter([("code".to_owned(), json!("return 1;"))]),
        ));
        let mut messages: Vec<Message> = serde_json::from_value(json!([
            {"role": "system", "content": "Prompt", "timestamp": 0, "toolsAdded": [codemode, broken]},
            {"role": "user", "content": "Go", "timestamp": 0},
        ]))
        .unwrap();
        messages.push(Message::Assistant(Box::new(assistant)));
        messages.extend(
            serde_json::from_value::<Vec<Message>>(json!([
                {"role": "toolResult", "toolCallId": "call_1|ctc_1", "toolName": "codemode",
                    "content": [{"type": "text", "text": "1"}], "isError": false, "timestamp": 0},
                {"role": "system", "content": "", "timestamp": 0,
                    "toolsRemoved": [{"name": "codemode"}, {"name": "broken"}]},
            ]))
            .unwrap(),
        );
        let compat = Compat::new(&model, Flavor::OpenAi);
        assert!(schema::grammar_properties(&messages, true).is_err());
        let normalized = resolve_transcript(&messages, compat.supports_mid_convo_system_messages);
        let grammar = schema::grammar_properties(&normalized, true).unwrap();
        assert!(grammar.is_empty());
        let input = convert_messages(&model, &normalized, &compat, &grammar).unwrap();
        assert_eq!(
            input[2..],
            [
                json!({"type": "function_call", "call_id": "call_1", "name": "codemode",
                    "arguments": "{\"code\":\"return 1;\"}"}),
                json!({"type": "function_call_output", "call_id": "call_1", "output": "1"}),
            ]
        );
    }
}
