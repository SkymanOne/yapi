//! Providers that pi extensions register: their own streams, sign-ins and
//! model refreshes, as the session uses them.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{cli_source, custom_entries, engine, eventually, options, scratch};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use yapi_ai::api::Apis;
use yapi_ai::api::classify::ClassifyOptions;
use yapi_ai::api::images::ImagesOptions;
use yapi_ai::auth::{AuthError, AuthPrompt, AuthRequest, Interaction, LoginOptions};
use yapi_ai::model_catalog::RefreshOptions;
use yapi_ai::registry::{LoginKind, ModelRegistry};
use yapi_ai::stream::{EventStream, Request, StreamEvent, StreamOptions};
use yapi_core::agent_session::{AgentSession, Resources, SessionConfig};
use yapi_core::extensions::{Mode, NoUi};
use yapi_core::session::SessionManager;
use yapi_core::settings::SettingsManager;
use yapi_ext::ExtensionHost;
use yapi_types::auth::Credential;
use yapi_types::classify::ClassifierContext;
use yapi_types::event::AssistantMessageEvent;
use yapi_types::message::{ContentBlock, Message, StopReason, ThinkingLevel};
use yapi_types::model::Model;

const EXTENSION: &str = r#"
import { createAssistantMessageEventStream, createProvider, getCurrentSystemPrompt, registerApiProvider } from "@earendil-works/pi-ai";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

const zero = () => ({ input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } });

function streamSimple(model, context, options) {
	if (model.id === "throws") throw new Error("No stream for you");
	const events = createAssistantMessageEventStream();
	const output = { role: "assistant", content: [], api: model.api, provider: model.provider, model: model.id, usage: zero(), stopReason: "pending", timestamp: Date.now() };
	(async () => {
		events.push({ type: "start", partial: output });
		const text = model.id === "options"
			? [options.apiKey, options.reasoning, options.sessionId, getCurrentSystemPrompt(context.messages), context.messages.length].join("|")
			: "Hello";
		output.content.push({ type: "text", text: "" });
		events.push({ type: "text_start", contentIndex: 0, partial: output });
		output.content[0].text = text.slice(0, 3);
		output.usage.output = 1;
		events.push({ type: "text_delta", contentIndex: 0, delta: text.slice(0, 3), partial: output });
		if (model.id === "slow") {
			await new Promise((resolve) => options.signal.addEventListener("abort", resolve, { once: true }));
			output.stopReason = "aborted";
			output.errorMessage = "Request was aborted";
			events.push({ type: "error", reason: "aborted", error: output });
			return;
		}
		await new Promise((resolve) => setTimeout(resolve, 1));
		output.content[0].text = text;
		events.push({ type: "text_delta", contentIndex: 0, delta: text.slice(3), partial: output });
		events.push({ type: "text_end", contentIndex: 0, content: text, partial: output });
		output.usage = { ...zero(), input: 3, output: 2, totalTokens: 5 };
		output.stopReason = "stop";
		events.push({ type: "done", reason: "stop", message: output });
	})();
	return events;
}

export default function (pi: ExtensionAPI) {
	pi.registerProvider("echo", {
		baseUrl: "http://127.0.0.1:9",
		apiKey: "echo-key",
		api: "echo-api",
		models: ["hello", "options", "slow", "throws"].map((id) => ({
			id,
			name: id,
			reasoning: true,
			input: ["text"],
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
			contextWindow: 1000,
			maxTokens: 100,
		})),
		streamSimple,
	});
	registerApiProvider({ api: "relay-api", stream: streamSimple, streamSimple });
	pi.registerProvider("corp", {
		baseUrl: "http://127.0.0.1:9",
		api: "echo-api",
		models: [{ id: "corp-1", name: "Corp 1", reasoning: false, input: ["text"], cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 1000, maxTokens: 100 }],
		oauth: {
			name: "Corp SSO",
			async login(callbacks) {
				callbacks.onAuth({ url: "https://corp.example/login", instructions: "Sign in there" });
				callbacks.onProgress?.("Waiting for the code");
				const code = await callbacks.onPrompt({ message: "Code:" });
				const team = await callbacks.onSelect({ message: "Team", options: [{ id: "a", label: "Team A" }, { id: "b", label: "Team B" }] });
				return { refresh: "refresh-1", access: `access-${code}`, expires: 1, team };
			},
			async refreshToken(credentials) {
				return { ...credentials, refresh: "refresh-2", access: `${credentials.access}-refreshed`, expires: Date.now() + 3600000 };
			},
			getApiKey: (credentials) => `key:${credentials.access}:${credentials.team}`,
			modifyModels: (models, credentials) => models.map((model) => ({ ...model, name: `${model.name} (${credentials.team})` })),
		},
		streamSimple,
	});
	const model = (id, name) => ({ id, name, reasoning: false, input: ["text"], cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 1000, maxTokens: 100 });
	pi.registerProvider("dyn", {
		baseUrl: "http://127.0.0.1:9",
		apiKey: "dyn-key",
		api: "echo-api",
		models: [model("dyn-static", "Static")],
		async refreshModels(context) {
			if (!context.allowNetwork) return context.stored?.models;
			const models = [model("dyn-live", `Live ${context.credential.type} ${context.credential.key}`)];
			await context.publish({ persist: { models, checkedAt: 1 } });
			return models;
		},
		streamSimple,
	});
	const cost = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
	const generated = (model, text) => ({ api: model.api, provider: model.provider, model: model.id, output: [{ type: "text", text }], stopReason: "stop", timestamp: 1 });
	const classified = (model, probability) => ({ api: model.api, provider: model.provider, model: model.id, answers: { ok: { type: "bool", probability } }, stopReason: "stop", timestamp: 1 });
	pi.registerProvider("media", {
		baseUrl: "http://127.0.0.1:9",
		apiKey: "media-key",
		models: [
			{ type: "image", id: "draw", name: "Draw", api: "media-images", input: ["text"], output: ["image"], cost },
			{ type: "classifier", id: "judge", name: "Judge", api: "media-classify", input: ["text"], cost, contextWindow: 1000 },
		],
		images: { "media-images": { generateImages: async (model, context, options) => generated(model, `${options.apiKey}:${context.input[0].text}`) } },
		classifiers: { "media-classify": { classify: async (model, context, options) => classified(model, options.temperature) } },
	});
	// A complete pi-ai provider: its own auth, models, streams, classifier and refresh.
	pi.registerProvider(
		createProvider({
			id: "native",
			name: "Native",
			auth: {
				apiKey: {
					name: "Native key",
					check: async () => ({ type: "api_key", source: "native check" }),
					resolve: async ({ credential }) => ({ auth: { apiKey: credential?.key ?? "ambient-key", headers: { "x-native": "1" } }, source: "native" }),
				},
				oauth: {
					name: "Native SSO",
					async login(interaction) {
						interaction.notify({ type: "progress", message: "Signing in" });
						const token = await interaction.prompt({ type: "secret", message: "Token:" });
						return { type: "oauth", access: token, refresh: "r", expires: 1 };
					},
					refresh: async (credential) => ({ ...credential, access: `${credential.access}+`, expires: Date.now() + 3600000 }),
					toAuth: async (credential) => ({ apiKey: `sso:${credential.access}`, headers: { "x-account": credential.type }, baseUrl: "http://account.example" }),
				},
			},
			models: [
				{ ...model("n1", "N1"), api: "native-api", provider: "native", baseUrl: "http://127.0.0.1:9" },
				{ type: "classifier", id: "nc", name: "NC", api: "native-classify", provider: "native", baseUrl: "http://127.0.0.1:9", input: ["text"], cost, contextWindow: 1000 },
			],
			api: { stream: streamSimple, streamSimple },
			classifiers: { "native-classify": { classify: async (model) => classified(model, 0.5) } },
			fetchModels: async (context) => [{ ...model("n2", `Fetched ${context.credential.key}`), api: "native-api", provider: "native", baseUrl: "http://127.0.0.1:9" }],
		}),
	);
}
"#;

async fn load(dir: &Path) -> Arc<ExtensionHost> {
    let path = dir.join("providers.ts");
    std::fs::write(&path, EXTENSION).unwrap();
    let host = ExtensionHost::load(&engine(), options(dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    host
}

/// The registry and wire APIs of the extensions' providers, as startup
/// builds them, with the agent directory in `dir`.
fn registry_and_apis(host: &Arc<ExtensionHost>, dir: &Path) -> (ModelRegistry, Apis) {
    let agent_dir = dir.join("agent");
    std::fs::create_dir_all(&agent_dir).unwrap();
    let mut registry = ModelRegistry::load(&agent_dir);
    let mut apis = Apis::default();
    for provider in host.providers() {
        provider.install(&mut registry, &mut apis).unwrap();
    }
    for api in host.apis() {
        apis.register(api);
    }
    (registry, apis)
}

/// A session on the extensions' providers in `dir`, without a model.
fn session_config(host: &Arc<ExtensionHost>, dir: &Path) -> SessionConfig {
    let (registry, apis) = registry_and_apis(host, dir);
    SessionConfig {
        cwd: dir.to_path_buf(),
        agent_dir: dir.join("agent"),
        settings: SettingsManager::in_memory(),
        registry,
        apis,
        session: SessionManager::in_memory(dir),
        model: None,
        thinking_level: ThinkingLevel::Off,
        tools: Vec::new(),
        extensions: host.for_session(),
        include_extension_tools: true,
        allowed_tools: None,
        excluded_tools: Vec::new(),
        docs: yapi_core::docs::Locations::default(),
        resources: Resources::default(),
    }
}

fn request(model: Model, cancel: CancellationToken) -> Request {
    Request {
        model,
        messages: Vec::new(),
        options: StreamOptions {
            api_key: Some("echo-key".into()),
            cancel,
            ..StreamOptions::default()
        },
    }
}

async fn next(stream: &mut EventStream) -> StreamEvent {
    tokio::time::timeout(Duration::from_secs(10), stream.next())
        .await
        .expect("an event in time")
        .expect("an event before the end")
}

fn text(event: &StreamEvent) -> String {
    match event {
        StreamEvent::Done(message) | StreamEvent::Error(message) => message
            .content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect(),
        other => panic!("not a final event: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn streams_events_as_the_extension_emits_them() {
    let dir = scratch("provider-stream");
    let host = load(&dir).await;
    let (registry, apis) = registry_and_apis(&host, &dir);
    let model = registry.find("echo", "hello").unwrap().clone();
    assert_eq!(model.api, "echo-api");

    let mut stream = apis.stream(request(model, CancellationToken::new()));
    let StreamEvent::Start(start) = next(&mut stream).await else {
        panic!("no start event");
    };
    assert_eq!(
        (start.provider.as_str(), start.model.as_str()),
        ("echo", "hello")
    );
    // Updates carry the usage of the live message as they are forwarded.
    let mut updates = Vec::new();
    let done = loop {
        match next(&mut stream).await {
            StreamEvent::Update { event, .. } => updates.push(event),
            other => break other,
        }
    };
    let delta = |delta: &str| AssistantMessageEvent::TextDelta {
        content_index: 0,
        delta: delta.into(),
    };
    assert_eq!(
        updates,
        [
            AssistantMessageEvent::TextStart { content_index: 0 },
            delta("Hel"),
            delta("lo"),
            AssistantMessageEvent::TextEnd {
                content_index: 0,
                content: "Hello".into()
            },
        ]
    );
    let StreamEvent::Done(message) = &done else {
        panic!("not done: {done:?}");
    };
    assert_eq!(text(&done), "Hello");
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.usage.total_tokens, Some(5));
    assert!(stream.next().await.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn aborts_and_failures_end_the_stream_with_an_error() {
    let dir = scratch("provider-abort");
    let host = load(&dir).await;
    let (registry, apis) = registry_and_apis(&host, &dir);

    // The first events arrive while the extension still waits.
    let cancel = CancellationToken::new();
    let slow = registry.find("echo", "slow").unwrap().clone();
    let mut stream = apis.stream(request(slow, cancel.clone()));
    assert!(matches!(next(&mut stream).await, StreamEvent::Start(_)));
    assert!(matches!(
        next(&mut stream).await,
        StreamEvent::Update { .. }
    ));
    assert!(matches!(
        next(&mut stream).await,
        StreamEvent::Update { .. }
    ));
    cancel.cancel();
    let StreamEvent::Error(message) = next(&mut stream).await else {
        panic!("no error event");
    };
    assert_eq!(message.stop_reason, StopReason::Aborted);
    assert_eq!(
        message.error_message.as_deref(),
        Some("Request was aborted")
    );

    // A stream that throws ends with pi's setup error.
    let throws = registry.find("echo", "throws").unwrap().clone();
    let message = apis
        .stream(request(throws, CancellationToken::new()))
        .result()
        .await
        .unwrap();
    assert_eq!(message.stop_reason, StopReason::Error);
    assert_eq!(message.error_message.as_deref(), Some("No stream for you"));
}

#[tokio::test(flavor = "multi_thread")]
async fn registered_apis_stream_models_of_any_provider() {
    let dir = scratch("provider-api");
    let host = load(&dir).await;
    let (registry, apis) = registry_and_apis(&host, &dir);
    let mut model = registry.find("echo", "hello").unwrap().clone();
    model.provider = "elsewhere".into();
    model.api = "relay-api".into();
    let message = apis
        .stream(request(model.clone(), CancellationToken::new()))
        .result()
        .await
        .unwrap();
    assert_eq!(message.error_message, None);
    assert_eq!(message.api, "relay-api");

    // Another API stays unknown.
    model.api = "other-api".into();
    let message = apis
        .stream(request(model, CancellationToken::new()))
        .result()
        .await
        .unwrap();
    assert_eq!(
        message.error_message.as_deref(),
        Some("No API provider registered for api: other-api")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_session_runs_on_an_extension_provider() {
    let dir = scratch("provider-session");
    let host = load(&dir).await;
    let config = session_config(&host, &dir);
    let model = config.registry.find("echo", "options").unwrap().clone();
    assert!(config.registry.has_auth("echo"));
    let session = AgentSession::new(SessionConfig {
        model: Some(model),
        thinking_level: ThinkingLevel::High,
        resources: Resources {
            custom_prompt: Some("Be brief.".into()),
            ..Resources::default()
        },
        ..config
    });
    session.prompt("hi", Vec::new()).await.unwrap();
    let Some(Message::Assistant(reply)) = session.messages().last().cloned() else {
        panic!("no reply: {:?}", session.messages());
    };
    assert_eq!(reply.error_message, None);
    let session_id = session.with_session(|file| file.id().to_owned());
    // The key, thinking level, session id, prompt and transcript reached the stream.
    let ContentBlock::Text(text) = &reply.content[0] else {
        panic!("no text: {reply:?}");
    };
    let parts: Vec<&str> = text.text.split('|').collect();
    assert_eq!(parts[..3], ["echo-key", "high", session_id.as_str()]);
    assert!(parts[3].starts_with("Be brief."), "{}", parts[3]);
    assert_eq!(parts[4], "2");
    assert_eq!(json!(reply.thinking_level), json!("high"));
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_providers_sign_in_store_and_refresh() {
    let dir = scratch("provider-oauth");
    let host = load(&dir).await;
    let (registry, _) = registry_and_apis(&host, &dir);
    assert_eq!(registry.provider_name("corp"), "Corp SSO");
    let flow = registry.oauth_flow("corp").unwrap();
    assert_eq!(flow.name(), "Corp SSO");
    assert!(!flow.is_subscription());
    assert!(!registry.has_auth("corp"));

    // The sign-in reaches the user through the interaction.
    let (interaction, mut requests) = Interaction::new(CancellationToken::new());
    let ui = tokio::spawn(async move {
        let mut seen = Vec::new();
        while let Some(request) = requests.recv().await {
            match request {
                AuthRequest::Notify(event) => seen.push(format!("{event:?}")),
                AuthRequest::Prompt { prompt, reply, .. } => {
                    let answer = match &prompt {
                        AuthPrompt::Select { options, .. } => options[1].id.clone(),
                        _ => "123".to_owned(),
                    };
                    seen.push(format!("{prompt:?}"));
                    let _ = reply.send(answer);
                }
            }
        }
        seen
    });
    let credential = registry
        .login(
            "corp",
            LoginKind::OAuth,
            &interaction,
            &LoginOptions::default(),
        )
        .await
        .unwrap();
    drop(interaction);
    let seen = ui.await.unwrap();
    assert_eq!(seen.len(), 4, "{seen:?}");
    assert!(seen[0].contains("https://corp.example/login") && seen[0].contains("Sign in there"));
    assert!(seen[1].contains("Waiting for the code"));
    assert!(seen[2].starts_with("Text") && seen[2].contains("Code:"));
    assert!(seen[3].starts_with("Select") && seen[3].contains("Team B"));
    let Credential::OAuth(stored) = &credential else {
        panic!("not OAuth: {credential:?}");
    };
    assert_eq!(stored.access, "access-123");

    // auth.json holds it as pi stores OAuth credentials.
    let file: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("agent").join("auth.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        file["corp"],
        json!({"type": "oauth", "refresh": "refresh-1", "access": "access-123", "expires": 1, "team": "b"})
    );
    assert!(registry.has_auth("corp"));

    // The expired token refreshes before a request, and the extension
    // derives the key.
    let model = registry.find("corp", "corp-1").unwrap().clone();
    let auth = registry.auth(&model).await;
    assert_eq!(auth.error, None);
    assert_eq!(auth.api_key.as_deref(), Some("key:access-123-refreshed:b"));
    let Some(Credential::OAuth(refreshed)) = registry.store().get("corp") else {
        panic!("no stored credential");
    };
    assert_eq!(refreshed.refresh, "refresh-2");

    // Cancelling the sign-in while it waits for the user stops it.
    let cancel = CancellationToken::new();
    let (interaction, mut requests) = Interaction::new(cancel.clone());
    tokio::spawn(async move {
        while let Some(request) = requests.recv().await {
            if let AuthRequest::Prompt { reply, .. } = request {
                cancel.cancel();
                // Keep the question open: only the cancellation ends it.
                tokio::time::sleep(Duration::from_secs(30)).await;
                drop(reply);
            }
        }
    });
    let options = LoginOptions::default();
    let result = registry
        .login("corp", LoginKind::OAuth, &interaction, &options)
        .await;
    assert_eq!(result.unwrap_err(), AuthError::Cancelled);
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_models_replaces_and_persists_the_model_list() {
    let dir = scratch("provider-refresh");
    let host = load(&dir).await;
    let ids = |session: &AgentSession| -> Vec<(String, String)> {
        session
            .registry()
            .models()
            .iter()
            .filter(|model| model.provider == "dyn")
            .map(|model| (model.id.clone(), model.name.clone()))
            .collect()
    };
    let only_dyn = |allow_network: bool| RefreshOptions {
        providers: Some(vec!["dyn".into()]),
        allow_network,
        ..RefreshOptions::default()
    };

    // Online, the extension's list replaces the registered one and persists.
    let first = AgentSession::new(session_config(&host, &dir));
    assert_eq!(ids(&first), [("dyn-static".into(), "Static".into())]);
    let result = first.refresh_model_catalogs(only_dyn(true)).await;
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(
        ids(&first),
        [("dyn-live".into(), "Live api_key dyn-key".into())]
    );
    let store: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("agent").join("models-store.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(store["dyn"]["models"][0]["id"], "dyn-live");
    first.shutdown().await;

    // Offline, a later session restores the stored list.
    let second = AgentSession::new(session_config(&host, &dir));
    second.refresh_model_catalogs(only_dyn(false)).await;
    assert_eq!(
        ids(&second),
        [("dyn-live".into(), "Live api_key dyn-key".into())]
    );
}

/// One `bool` question.
fn bool_question() -> ClassifierContext {
    serde_json::from_value(json!({"state": {}, "questions": {
        "ok": {"type": "bool", "instructions": "OK?", "criteria": {"true": "yes", "false": "no"}},
    }}))
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn image_and_classifier_models_run_in_the_extension() {
    let dir = scratch("provider-media");
    let host = load(&dir).await;
    let (registry, _) = registry_and_apis(&host, &dir);
    assert!(
        !registry
            .models()
            .iter()
            .any(|model| model.provider == "media")
    );

    // The temperature and the configured key reach the implementations.
    let judge = registry
        .classifiers()
        .iter()
        .find(|model| model.provider == "media")
        .unwrap()
        .clone();
    let options = ClassifyOptions {
        temperature: Some(0.25),
        ..ClassifyOptions::default()
    };
    let result = registry.classify(&judge, &bool_question(), options).await;
    assert_eq!(result.error_message, None);
    assert_eq!(
        json!(result.answers),
        json!({"ok": {"type": "bool", "probability": 0.25}})
    );
    let draw = registry
        .image_models()
        .iter()
        .find(|model| model.provider == "media")
        .unwrap()
        .clone();
    let context = serde_json::from_value(json!({"input": [{"type": "text", "text": "a cat"}]}));
    let images = registry
        .generate_images(&draw, &context.unwrap(), ImagesOptions::default())
        .await;
    assert_eq!(
        json!(images.output),
        json!([{"type": "text", "text": "media-key:a cat"}])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pi_ai_provider_brings_its_auth_models_streams_and_classifier() {
    let dir = scratch("provider-native");
    let host = load(&dir).await;
    let (registry, apis) = registry_and_apis(&host, &dir);
    assert_eq!(registry.provider_name("native"), "Native");
    let model = registry.find("native", "n1").unwrap().clone();
    assert_eq!(model.api, "native-api");

    // Its check found a key as the extension loaded; its resolve gives it.
    assert!(registry.has_auth("native"));
    assert_eq!(
        registry.login_status("native").as_deref(),
        Some("native check")
    );
    let auth = registry.auth(&model).await;
    assert_eq!(
        (auth.api_key.as_deref(), auth.source.as_deref()),
        (Some("ambient-key"), Some("native"))
    );
    assert_eq!(auth.headers.get("x-native"), Some(&Some("1".to_owned())));

    let message = apis
        .stream(request(model.clone(), CancellationToken::new()))
        .result()
        .await
        .unwrap();
    assert_eq!(message.error_message, None);
    assert!(matches!(&message.content[0], ContentBlock::Text(text) if text.text == "Hello"));
    let classifier = registry
        .classifiers()
        .iter()
        .find(|model| model.provider == "native")
        .unwrap()
        .clone();
    let result = registry
        .classify(&classifier, &bool_question(), ClassifyOptions::default())
        .await;
    assert_eq!(
        json!(result.answers),
        json!({"ok": {"type": "bool", "probability": 0.5}})
    );

    // Its OAuth sign-in prompts through the interaction; the expired token
    // refreshes and gives the account's credentials.
    assert_eq!(registry.oauth_flow("native").unwrap().name(), "Native SSO");
    let (interaction, mut requests) = Interaction::new(CancellationToken::new());
    tokio::spawn(async move {
        while let Some(request) = requests.recv().await {
            if let AuthRequest::Prompt { reply, .. } = request {
                let _ = reply.send("t".to_owned());
            }
        }
    });
    registry
        .login(
            "native",
            LoginKind::OAuth,
            &interaction,
            &LoginOptions::default(),
        )
        .await
        .unwrap();
    let auth = registry.auth(&model).await;
    assert_eq!(auth.error, None);
    assert_eq!(auth.api_key.as_deref(), Some("sso:t+"));
    assert_eq!(auth.base_url.as_deref(), Some("http://account.example"));
    assert_eq!(
        auth.headers.get("x-account"),
        Some(&Some("oauth".to_owned()))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pi_ai_provider_refreshes_its_own_models() {
    let dir = scratch("provider-native-refresh");
    let host = load(&dir).await;
    let session = AgentSession::new(session_config(&host, &dir));
    let options = RefreshOptions {
        providers: Some(vec!["native".into()]),
        allow_network: true,
        ..RefreshOptions::default()
    };
    let result = session.refresh_model_catalogs(options).await;
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let names: Vec<String> = session
        .registry()
        .models()
        .iter()
        .filter(|model| model.provider == "native")
        .map(|model| model.name.clone())
        .collect();
    // The refresh gets the key its resolve gives.
    assert_eq!(names, ["N1", "Fetched ambient-key"]);
    assert!(
        session
            .registry()
            .classifiers()
            .iter()
            .any(|model| model.provider == "native")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn modify_models_projects_the_models_for_the_signed_in_account() {
    let dir = scratch("provider-modify");
    let host = load(&dir).await;
    let session = AgentSession::new(session_config(&host, &dir));
    let names = |session: &AgentSession| -> Vec<String> {
        session
            .registry()
            .models()
            .iter()
            .filter(|model| model.provider == "corp")
            .map(|model| model.name.clone())
            .collect()
    };
    let only_corp = RefreshOptions {
        providers: Some(vec!["corp".into()]),
        ..RefreshOptions::default()
    };
    let cancel = CancellationToken::new();
    session.refresh_model_catalogs(only_corp.clone()).await;
    assert_eq!(names(&session), ["Corp 1"]);

    // Each refresh projects the configured models, not the projected ones.
    let credential = json!({"type": "oauth", "access": "a", "refresh": "r", "expires": 4_102_444_800_000_u64, "team": "b"});
    let store = Arc::clone(session.registry().store());
    store
        .set("corp", serde_json::from_value(credential).unwrap(), &cancel)
        .await
        .unwrap();
    for _ in 0..2 {
        let result = session.refresh_model_catalogs(only_corp.clone()).await;
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(names(&session), ["Corp 1 (b)"]);
    }

    // Signed out, the models are restored.
    store.delete("corp", &cancel).await.unwrap();
    session.refresh_model_catalogs(only_corp).await;
    assert_eq!(names(&session), ["Corp 1"]);
}

const AUTH_EXTENSION: &str = r#"
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	const provider = (apiKey) => ({
		baseUrl: "http://127.0.0.1:9",
		apiKey,
		api: "openai-completions",
		headers: { "X-Team": "yapi" },
		models: [{ id: "m", name: "M", reasoning: false, input: ["text"], cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 1000, maxTokens: 100, headers: { "X-Model": "m" } }],
	});
	pi.registerProvider("fixture", provider("fixture-key"));
	pi.registerProvider("broken", provider("!exit 1"));
	pi.registerCommand("auth", {
		description: "Reads provider credentials",
		handler: async (_args, ctx) => {
			const read = (id) => ctx.modelRegistry.getProviderAuth(id).then((auth) => (auth === undefined ? "undefined" : auth), (error) => error.message);
			const request = (provider) => ctx.modelRegistry.getApiKeyAndHeaders({ provider, id: "m" });
			pi.appendEntry("auth", {
				fixture: await read("fixture"),
				broken: await read("broken"),
				missing: await read("missing"),
				stored: await read("stored"),
				request: await request("fixture"),
				brokenRequest: await request("broken"),
				storedRequest: await request("stored"),
			});
		},
	});
}
"#;

/// `getProviderAuth` resolves pi's `AuthResult` for a provider, without its
/// models' headers, and nothing for an unknown provider or one without
/// credentials. `getApiKeyAndHeaders` resolves what a request to a model
/// sends. Both need the network grant.
#[tokio::test(flavor = "multi_thread")]
async fn provider_auth_needs_the_network_grant() {
    let mut entries = Vec::new();
    for network in [true, false] {
        let dir = scratch(&format!("provider-auth-{network}"));
        let path = dir.join("auth.ts");
        std::fs::write(&path, AUTH_EXTENSION).unwrap();
        // pi knows no provider "stored", so its credential stays unused.
        std::fs::create_dir_all(dir.join("agent")).unwrap();
        std::fs::write(
            dir.join("agent/auth.json"),
            r#"{"stored":{"type":"api_key","key":"stored-key"}}"#,
        )
        .unwrap();
        let mut options = options(&dir);
        options.grants.network = network;
        let host = ExtensionHost::load(&engine(), options, &[cli_source(&path)])
            .await
            .unwrap();
        assert!(host.errors().is_empty(), "{:?}", host.errors());
        let session = AgentSession::new(session_config(&host, &dir));
        session
            .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
            .await;
        session.prompt("/auth", Vec::new()).await.unwrap();
        eventually("the auth entry", || {
            !custom_entries(&session, "auth").is_empty()
        })
        .await;
        entries.extend(custom_entries(&session, "auth"));
    }
    let denied = "Network access is not permitted for this extension";
    let broken = "API key auth failed for provider broken: Failed to resolve API key for provider \"broken\" from shell command: exit 1";
    assert_eq!(
        entries,
        [
            json!({
                "fixture": {"auth": {"apiKey": "fixture-key", "headers": {"X-Team": "yapi"}}, "source": "configured API key"},
                "broken": broken,
                "missing": "undefined",
                "stored": "undefined",
                "request": {"ok": true, "apiKey": "fixture-key", "headers": {"X-Model": "m", "X-Team": "yapi"}},
                "brokenRequest": {"ok": false, "error": broken},
                "storedRequest": {"ok": true},
            }),
            json!({
                "fixture": denied,
                "broken": denied,
                "missing": denied,
                "stored": denied,
                "request": {"ok": false, "error": denied},
                "brokenRequest": {"ok": false, "error": denied},
                "storedRequest": {"ok": false, "error": denied},
            }),
        ]
    );
}
