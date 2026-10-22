//! Requests and operations the runtime itself answers: modules, the process
//! environment, randomness, hashing, processes, HTTP, yapi's wire APIs and
//! JSON helpers. Grants gate the ones that reach outside the instance;
//! everything else goes to the [`Bridge`](crate::Bridge).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use aws_lc_rs::digest;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use futures_util::FutureExt as _;
use futures_util::future::BoxFuture;
use md5::Digest as _;
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use yapi_ai::api::Apis;
use yapi_ai::stream::{EventStream, ProviderResponse, Request, RequestHooks};
use yapi_core::tools::{RegisteredTool, ToolEnv};
use yapi_types::message::Message;
use yapi_types::model::Model;
use yapi_types::sync::lock;

use crate::instance::Options;
use crate::loader::Loader;
use crate::{Bridge, ops};

/// What an instance's requests and operations reach.
pub(crate) struct Host {
    pub(crate) loader: Loader,
    pub(crate) bridge: Arc<dyn Bridge>,
    pub(crate) options: Options,
    pub(crate) ai_streams: AiStreams,
    pub(crate) streams: ops::Streams,
}

impl Drop for Host {
    /// Processes, responses and wire API streams end with the instance
    /// that started them.
    fn drop(&mut self) {
        self.streams.clear();
        self.ai_streams.clear();
    }
}

/// A running request of yapi's wire APIs or image generation: what `ai.next`
/// reads of it, out of the map while a call reads it, its cancellation, and
/// the extension's callbacks it calls.
struct Running {
    reading: Option<Reading>,
    cancel: CancellationToken,
    callbacks: Arc<Callbacks>,
}

/// What `ai.next` reads of a request: the events of a stream and the calls
/// of the extension's callbacks; an image request ends with a call that
/// carries its result.
struct Reading {
    events: Option<EventStream>,
    calls: mpsc::UnboundedReceiver<Value>,
}

impl Reading {
    /// The next item as `ai.next` sends it, waiting for one; `None` once the
    /// stream ended. Events that arrived come first, as a callback call
    /// follows them in the request.
    async fn wait(&mut self) -> Option<Value> {
        match &mut self.events {
            Some(events) => tokio::select! {
                biased;
                event = events.next() => event.map(|event| crate::streams::event_json(&event)),
                Some(call) = self.calls.recv() => Some(call),
            },
            None => self.calls.recv().await,
        }
    }

    /// The next item that has arrived, as [`Reading::wait`] gives it.
    fn ready(&mut self) -> Option<Option<Value>> {
        if let Some(events) = &mut self.events
            && let Some(event) = events.next().now_or_never()
        {
            return Some(event.map(|event| crate::streams::event_json(&event)));
        }
        self.calls.try_recv().ok().map(Some)
    }
}

/// pi-ai's `onPayload`, `onResponse` and `onProviderStreamEvent` an extension
/// passed to one request. The guest reads each call through `ai.next`, runs
/// the callback and answers with `ai.callback`; the request waits for the
/// answer, as pi's wire APIs await the callback.
struct Callbacks {
    calls: mpsc::UnboundedSender<Value>,
    answers: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    next: AtomicU64,
    cancel: CancellationToken,
}

impl Callbacks {
    fn new(cancel: CancellationToken) -> (Arc<Callbacks>, mpsc::UnboundedReceiver<Value>) {
        let (calls, receiver) = mpsc::unbounded_channel();
        let callbacks = Callbacks {
            calls,
            answers: Mutex::default(),
            next: AtomicU64::new(0),
            cancel,
        };
        (Arc::new(callbacks), receiver)
    }

    /// Calls callback `name` with `value`: the guest's answer, or `None`
    /// once the request is cancelled.
    fn call(&self, name: &str, value: Value) -> BoxFuture<'static, Option<Value>> {
        let call = self.next.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = oneshot::channel();
        lock(&self.answers).insert(call, answer);
        let _ = self
            .calls
            .send(json!({"type": "callback", "callback": name, "call": call, "value": value}));
        let cancel = self.cancel.clone();
        Box::pin(async move {
            tokio::select! {
                answer = answered => answer.ok(),
                () = cancel.cancelled() => None,
            }
        })
    }

    /// `hooks` with the callbacks `wanted`, `{ payload, response,
    /// streamEvent }`, names in place of its own.
    fn hooks(self: &Arc<Self>, wanted: &Value, mut hooks: RequestHooks) -> RequestHooks {
        if wanted["payload"] == true {
            let callbacks = self.clone();
            hooks.payload = Some(Arc::new(move |body: Value| {
                let answer = callbacks.call("payload", body.clone());
                Box::pin(async move {
                    // pi keeps the body when `onPayload` returns `undefined`.
                    match answer.await {
                        Some(Value::Object(mut answer)) => answer.remove("payload").unwrap_or(body),
                        _ => body,
                    }
                })
            }));
        }
        if wanted["response"] == true {
            let callbacks = self.clone();
            hooks.response = Some(Arc::new(move |response: ProviderResponse| {
                let value = json!({"status": response.status, "headers": response.headers});
                let answer = callbacks.call("response", value);
                Box::pin(async move {
                    answer.await;
                })
            }));
        }
        if wanted["streamEvent"] == true {
            let callbacks = self.clone();
            hooks.stream_event = Some(Arc::new(move |data: Value| {
                let answer = callbacks.call("streamEvent", data);
                Box::pin(async move {
                    answer.await;
                })
            }));
        }
        hooks
    }
}

/// Requests of yapi's wire APIs and image generation that extensions make
/// through pi-ai, by id.
#[derive(Clone, Default)]
pub(crate) struct AiStreams {
    running: Arc<Mutex<HashMap<u64, Running>>>,
    next_id: Arc<AtomicU64>,
}

impl AiStreams {
    /// Starts streaming `{api, model, context, options, callbacks}` with
    /// yapi's implementation of `api`, or with the model's provider and API
    /// as pi-ai's `streamSimple` does without one, and returns the stream's
    /// id. A request without a key takes the provider's key variable when
    /// `env_keys` allows it. `hooks` observe the request, except where the
    /// extension's own `callbacks` do.
    fn start(&self, payload: &Value, env_keys: bool, hooks: RequestHooks) -> Result<Value, String> {
        let model: Model = serde_json::from_value(payload["model"].clone())
            .map_err(|err| format!("Invalid model: {err}"))?;
        let messages: Vec<Message> = serde_json::from_value(payload["context"]["messages"].clone())
            .map_err(|err| format!("Invalid context: {err}"))?;
        let cancel = CancellationToken::new();
        let (callbacks, calls) = Callbacks::new(cancel.clone());
        let mut options = crate::streams::options_from_json(&payload["options"], cancel.clone());
        options.hooks = callbacks.hooks(&payload["callbacks"], hooks);
        if env_keys
            && options
                .api_key
                .as_deref()
                .is_none_or(|key| key.trim().is_empty())
        {
            let key = yapi_ai::credentials::env_api_key(&model.provider, options.env.as_ref())
                .map(|(_, key)| key)
                .filter(|key| key != yapi_ai::credentials::AMBIENT_CREDENTIALS);
            options.api_key = key.or(options.api_key);
        }
        let request = Request {
            model,
            messages,
            options,
        };
        let events = match payload["api"].as_str() {
            Some(api) => {
                let provider = yapi_ai::api::builtin(api)
                    .ok_or_else(|| format!("No API provider registered for api: {api}"))?;
                provider.stream(request)
            }
            None => Apis::default().stream(request),
        };
        Ok(self.insert(Some(events), calls, cancel, callbacks))
    }

    /// Starts generating images for `{ model, context, options, callbacks }`
    /// as pi-ai's `generateImages` of `api/openrouter-images` does, and
    /// returns the request's id. `ai.next` reads its result.
    fn start_images(&self, payload: Value) -> Result<Value, String> {
        fn parse<T: serde::de::DeserializeOwned>(payload: &Value, key: &str) -> Option<T> {
            serde_json::from_value(payload["options"][key].clone()).ok()
        }
        let model = serde_json::from_value(payload["model"].clone())
            .map_err(|err| format!("Invalid model: {err}"))?;
        let context = serde_json::from_value(payload["context"].clone())
            .map_err(|err| format!("Invalid context: {err}"))?;
        let cancel = CancellationToken::new();
        let (callbacks, calls) = Callbacks::new(cancel.clone());
        let options = yapi_ai::api::images::ImagesOptions {
            api_key: parse(&payload, "apiKey"),
            headers: parse(&payload, "headers").unwrap_or_default(),
            max_retries: parse(&payload, "maxRetries"),
            max_retry_delay_ms: parse(&payload, "maxRetryDelayMs"),
            timeout_ms: parse(&payload, "timeoutMs"),
            cancel: cancel.clone(),
            hooks: callbacks.hooks(&payload["callbacks"], RequestHooks::default()),
        };
        let results = callbacks.calls.clone();
        tokio::spawn(async move {
            let result = yapi_ai::api::images::generate_images(&model, &context, &options).await;
            let result = serde_json::to_value(result).unwrap_or_default();
            let _ = results.send(json!({"type": "result", "result": result}));
        });
        Ok(self.insert(None, calls, cancel, callbacks))
    }

    /// Keeps a request, which `cancel` aborts, for `ai.next`; its id.
    fn insert(
        &self,
        events: Option<EventStream>,
        calls: mpsc::UnboundedReceiver<Value>,
        cancel: CancellationToken,
        callbacks: Arc<Callbacks>,
    ) -> Value {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let reading = Some(Reading { events, calls });
        lock(&self.running).insert(
            id,
            Running {
                reading,
                cancel,
                callbacks,
            },
        );
        json!(id)
    }

    /// What request `id` sent that has arrived, waiting for at least one
    /// item: events, callback calls and an image request's result; none
    /// once the request has ended.
    async fn next(&self, id: u64) -> Value {
        let Some(mut reading) = lock(&self.running)
            .get_mut(&id)
            .and_then(|running| running.reading.take())
        else {
            return json!([]);
        };
        let mut batch = Vec::new();
        let mut item = Some(reading.wait().await);
        let mut ended = false;
        while let Some(wire) = item {
            let Some(wire) = wire else {
                ended = true;
                break;
            };
            ended = matches!(wire["type"].as_str(), Some("done" | "error" | "result"));
            batch.push(wire);
            item = if ended { None } else { reading.ready() };
        }
        let mut running = lock(&self.running);
        if ended {
            running.remove(&id);
        } else if let Some(running) = running.get_mut(&id) {
            running.reading = Some(reading);
        }
        Value::Array(batch)
    }

    /// The guest's `answer` to callback call `{ call }` of request `id`.
    fn answer(&self, id: u64, answer: &Value) {
        let callbacks = lock(&self.running)
            .get(&id)
            .map(|running| running.callbacks.clone());
        let call = answer["call"].as_u64().unwrap_or_default();
        if let Some(sender) = callbacks.and_then(|callbacks| lock(&callbacks.answers).remove(&call))
        {
            let _ = sender.send(answer.clone());
        }
    }

    /// Cancels request `id`.
    fn abort(&self, id: u64) {
        if let Some(running) = lock(&self.running).get(&id) {
            running.cancel.cancel();
        }
    }

    /// Cancels every request and forgets them.
    pub(crate) fn clear(&self) {
        for (_, running) in lock(&self.running).drain() {
            running.cancel.cancel();
        }
    }
}

fn denied(what: &str) -> String {
    format!("{what} is not permitted for this extension")
}

/// The offset of the zone `payload` names, in seconds east of UTC: the local
/// zone for a number, the zone of `{ zone, time }` otherwise, and `null` for a
/// name no zone has. `kind` picks an instant or a wall-clock time.
fn zone_offset(kind: &str, payload: &Value) -> Value {
    let (zone, time) = match payload {
        Value::Object(_) => (
            payload["zone"].as_str().and_then(yapi_core::zone::named),
            &payload["time"],
        ),
        _ => (Some(yapi_core::zone::local()), payload),
    };
    let time = time.as_i64().unwrap_or_default();
    zone.map_or(Value::Null, |zone| {
        json!(if kind == "timezone.offset" {
            zone.offset(time)
        } else {
            zone.local_offset(time)
        })
    })
}

fn text<'a>(payload: &'a Value, key: &str) -> &'a str {
    payload[key].as_str().unwrap_or_default()
}

impl Host {
    /// Whether the extension reads the process environment, which provider
    /// key variables come from.
    fn reads_process_env(&self) -> bool {
        self.options.grants.environment && self.options.environment.is_none()
    }

    /// Answers request `kind` at once.
    pub(crate) fn request(&self, kind: &str, payload: &Value) -> Result<Value, String> {
        match kind {
            // Modules are files, so the loader reads them only with the grant.
            "module.resolve" | "module.load" | "module.source"
                if !self.options.grants.filesystem =>
            {
                Err(denied("Loading modules"))
            }
            "module.resolve" => self
                .loader
                .resolve(
                    text(payload, "specifier"),
                    text(payload, "referrer"),
                    payload["kind"] == "require",
                )
                .map(Value::String),
            "module.load" => self.loader.load(text(payload, "path")),
            "module.source" => self.loader.source(text(payload, "path")),
            "log" => {
                self.bridge
                    .log(text(payload, "level"), text(payload, "message"));
                Ok(Value::Null)
            }
            "cwd" => Ok(Value::String(
                self.options.cwd.to_string_lossy().into_owned(),
            )),
            "home" => Ok(Value::String(
                self.options.home_dir.to_string_lossy().into_owned(),
            )),
            "agentDir" => Ok(Value::String(
                self.options.agent_dir.to_string_lossy().into_owned(),
            )),
            "tmpdir" => Ok(Value::String(
                self.options.temp_dir.to_string_lossy().into_owned(),
            )),
            // pi-ai's built-in catalog of `{ provider, type }`, which needs no session.
            "models.list" => {
                let provider = text(payload, "provider");
                match text(payload, "type") {
                    "image" => {
                        serde_json::to_value(yapi_ai::catalog::builtin_image_models(provider))
                    }
                    "classifier" => {
                        serde_json::to_value(yapi_ai::catalog::builtin_classifiers(provider))
                    }
                    _ => serde_json::to_value(yapi_ai::catalog::builtin_models(provider)),
                }
                .map_err(|err| err.to_string())
            }
            "models.envApiKey" if self.reads_process_env() => Ok(
                yapi_ai::credentials::env_api_key(text(payload, "provider"), None)
                    .map_or(Value::Null, |(_, key)| Value::String(key)),
            ),
            "models.envApiKey" => Ok(Value::Null),
            "platform" => Ok(Value::String(platform().into())),
            // The local zone, which Node exposes without a grant, as pi's
            // codemode does: its name, and its offset in seconds east of UTC
            // at a Unix time or for a wall-clock time.
            "timezone" => Ok(json!(yapi_core::zone::local().name())),
            "timezone.offset" | "timezone.localOffset" => Ok(zone_offset(kind, payload)),
            "timezone.name" => Ok(payload
                .as_str()
                .and_then(yapi_core::zone::named)
                .map_or(Value::Null, |zone| json!(zone.name()))),
            "env" => Ok(Value::Object(
                if let (true, Some(environment)) =
                    (self.options.grants.environment, &self.options.environment)
                {
                    environment
                        .iter()
                        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                        .collect()
                } else if self.options.grants.environment {
                    std::env::vars()
                        .chain(
                            yapi_core::config::child_env()
                                .into_iter()
                                .map(|(key, value)| (key.to_owned(), value)),
                        )
                        .map(|(key, value)| (key, Value::String(value)))
                        .collect()
                } else {
                    Map::new()
                },
            )),
            "random" => {
                let count = payload["bytes"].as_u64().unwrap_or(0).min(65536) as usize;
                let mut bytes = vec![0; count];
                getrandom::fill(&mut bytes).map_err(|err| err.to_string())?;
                Ok(Value::String(STANDARD.encode(bytes)))
            }
            "hash" => hash(text(payload, "algorithm"), text(payload, "data")),
            "exec.sync" if self.options.grants.process => ops::exec_sync(payload),
            "process.spawn" if self.options.grants.process => self.streams.spawn(payload),
            "exec.sync" | "process.spawn" => Err(denied("Running processes")),
            "process.end" => {
                self.streams.end(payload);
                Ok(Value::Null)
            }
            "process.kill" => Ok(self.streams.kill(payload)),
            "process.release" | "fetch.release" => {
                self.streams.release(payload);
                Ok(Value::Null)
            }
            "execPath" => Ok(json!(
                std::env::current_exe()
                    .ok()
                    .map(|path| path.to_string_lossy().into_owned())
            )),
            "json.repair" => Ok(Value::String(yapi_ai::json_parse::repair_json(text(
                payload, "text",
            )))),
            "json.parseWithRepair" => {
                yapi_ai::json_parse::parse_json_with_repair(text(payload, "text"))
                    .map_err(|err| err.to_string())
            }
            "json.partial" => Ok(Value::Object(yapi_ai::json_parse::parse_streaming_json(
                text(payload, "text"),
            ))),
            "codemode.definition" => Ok(crate::codemode::definition()),
            // pi-ai's Codex WebSocket cache, which is process-wide as in pi.
            "codex.closeSessions" => {
                yapi_ai::api::close_codex_websockets(payload["sessionId"].as_str());
                Ok(Value::Null)
            }
            "codex.stats" => Ok(payload["sessionId"]
                .as_str()
                .and_then(yapi_ai::api::codex_websocket_stats)
                .and_then(|stats| serde_json::to_value(stats).ok())
                .unwrap_or_default()),
            "codex.resetStats" => {
                yapi_ai::api::reset_codex_websocket_stats(payload["sessionId"].as_str());
                Ok(Value::Null)
            }
            // pi checks a registration as it happens, while extensions load
            // too, so the name must not be another runtime's.
            "mcp.validate" => {
                let name = text(payload, "name");
                let config =
                    self.validate_mcp_server(name, &payload["config"], text(payload, "extension"))?;
                self.bridge.check_mcp_server(name)?;
                Ok(config)
            }
            // The guest reports the servers it registered, checked or not.
            "mcp.servers" => {
                let mut servers = payload.clone();
                for server in servers["servers"].as_array_mut().into_iter().flatten() {
                    server["config"] = self.validate_mcp_server(
                        text(server, "name"),
                        &server["config"],
                        text(server, "extensionPath"),
                    )?;
                }
                self.bridge.request(kind, &servers)
            }
            "builtin.tool" => {
                // Declaring a tool needs no grant; running it does.
                let tool = self.builtin_tool(payload, false)?;
                Ok(json!({
                    "name": tool.name(),
                    "label": tool.tool.label(),
                    "description": tool.tool.declaration().description,
                    "parameters": tool.tool.declaration().parameters,
                    "promptSnippet": tool.snippet,
                    "promptGuidelines": tool.guidelines,
                }))
            }
            "ai.abort" => {
                self.ai_streams
                    .abort(payload["id"].as_u64().unwrap_or_default());
                Ok(Value::Null)
            }
            "ai.callback" => {
                self.ai_streams
                    .answer(payload["id"].as_u64().unwrap_or_default(), payload);
                Ok(Value::Null)
            }
            "language.fromPath" => Ok(json!(yapi_core::language::from_path(text(payload, "path")))),
            "frontmatter.parse" => {
                let (frontmatter, body) =
                    yapi_core::resources::parse_frontmatter(text(payload, "content"))?;
                Ok(json!({ "frontmatter": frontmatter, "body": body }))
            }
            _ => self.bridge.request(kind, payload),
        }
    }

    /// Starts operation `kind`.
    pub(crate) fn start(
        &self,
        kind: &str,
        payload: Value,
    ) -> BoxFuture<'static, Result<Value, String>> {
        match kind {
            "timer" => Box::pin(ops::timer(payload)),
            "process.write" => self.streams.write(&payload),
            "process.next" | "fetch.next" => {
                let streams = self.streams.clone();
                let id = ops::process_id(&payload);
                Box::pin(async move { Ok(streams.next(id).await) })
            }
            "fetch" if self.options.grants.network => Box::pin(ops::fetch(payload)),
            "fetch.stream" if self.options.grants.network => self.streams.fetch(payload),
            "fetch" | "fetch.stream" => Box::pin(async { Err(denied("Network access")) }),
            "dns.lookup" if self.options.grants.network => Box::pin(ops::dns_lookup(payload)),
            "dns.lookup" => Box::pin(async { Err(denied("Network access")) }),
            "ai.stream" if self.options.grants.network => {
                let streams = self.ai_streams.clone();
                let env_keys = self.reads_process_env();
                // A stream that passes on the hooks of the extension stream it
                // serves reports to that stream's session.
                let hooks = payload["hooks"]
                    .as_u64()
                    .map(|id| self.bridge.stream_hooks(id))
                    .unwrap_or_default();
                // The stream starts on the runtime.
                Box::pin(async move { streams.start(&payload, env_keys, hooks) })
            }
            "ai.stream" => Box::pin(async { Err(denied("Network access")) }),
            "ai.images" if self.options.grants.network => {
                let streams = self.ai_streams.clone();
                Box::pin(async move { streams.start_images(payload) })
            }
            "ai.images" => Box::pin(async { Err(denied("Network access")) }),
            // The session's requests, which `ai.next` reads as it reads `ai.stream`'s.
            "models.stream" if self.options.grants.network => {
                let streams = self.ai_streams.clone();
                let cancel = CancellationToken::new();
                let (callbacks, calls) = Callbacks::new(cancel.clone());
                let hooks = callbacks.hooks(&payload["callbacks"], RequestHooks::default());
                let started = self.bridge.stream_model(payload, cancel.clone(), hooks);
                Box::pin(async move {
                    let events = started.await?;
                    Ok(streams.insert(Some(events), calls, cancel, callbacks))
                })
            }
            "models.stream" => Box::pin(async { Err(denied("Network access")) }),
            // Credentials are only good for requests, so they need the network grant.
            "models.auth" | "models.requestAuth" if !self.options.grants.network => {
                Box::pin(async { Err(denied("Network access")) })
            }
            "ai.next" => {
                let streams = self.ai_streams.clone();
                let id = payload["id"].as_u64().unwrap_or_default();
                Box::pin(async move { Ok(streams.next(id).await) })
            }
            "builtin.execute" => {
                let tool = match self.builtin_tool(&payload, true) {
                    Ok(tool) => tool.tool,
                    Err(message) => return Box::pin(async move { Err(message) }),
                };
                let call_id = text(&payload, "toolCallId").to_owned();
                let params = payload["params"].clone();
                // The extension tool that runs it lends its cancellation and updates.
                let (updates, cancel) = self
                    .bridge
                    .caller(&call_id)
                    .unwrap_or_else(|| (Arc::new(|_| {}), CancellationToken::new()));
                Box::pin(async move {
                    let result = tool.execute(call_id, params, cancel, updates).await?;
                    serde_json::to_value(result).map_err(|err| err.to_string())
                })
            }
            _ => self.bridge.start(kind, payload),
        }
    }

    /// pi's checks of `registerMcpServer(name, config)` by `extension`, and
    /// the grant the server's transport needs: process for stdio, network
    /// for HTTP. Answers the config with pi's legacy exposure names
    /// replaced.
    fn validate_mcp_server(
        &self,
        name: &str,
        config: &Value,
        extension: &str,
    ) -> Result<Value, String> {
        use yapi_core::mcp::config::{ServerTransport, validate_server};
        let checked = validate_server(name, config).map_err(|problem| {
            format!("Invalid MCP server registered by extension \"{extension}\": {problem}")
        })?;
        let (granted, grant) = match checked.transport {
            ServerTransport::Stdio { .. } => (self.options.grants.process, "process"),
            ServerTransport::Http { .. } => (self.options.grants.network, "network"),
        };
        if !granted {
            return Err(format!(
                "MCP server \"{name}\" needs the {grant} grant, which this extension does not have"
            ));
        }
        Ok(yapi_core::mcp::config::resolve_exposure_aliases(config))
    }

    /// Built-in tool `{name}` for `{cwd}`; with `run`, only when the grants
    /// allow running it.
    fn builtin_tool(&self, payload: &Value, run: bool) -> Result<RegisteredTool, String> {
        let name = text(payload, "name");
        let granted = if name == "bash" {
            self.options.grants.process
        } else {
            self.options.grants.filesystem
        };
        if run && !granted {
            return Err(denied(&format!("The {name} tool")));
        }
        let cwd = payload["cwd"]
            .as_str()
            .map_or_else(|| self.options.cwd.clone(), PathBuf::from);
        let env = ToolEnv {
            cwd,
            runtime: Arc::default(),
            bin_dir: yapi_core::config::bin_dir(&self.options.agent_dir),
        };
        yapi_core::tools::builtin(name, &env)
            .ok_or_else(|| format!("Unknown built-in tool: {name}"))
    }
}

fn platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

fn hash(algorithm: &str, data: &str) -> Result<Value, String> {
    let data = STANDARD.decode(data).map_err(|err| err.to_string())?;
    let sha = |algorithm| digest::digest(algorithm, &data).as_ref().to_vec();
    let digest = match algorithm.to_ascii_lowercase().as_str() {
        "sha256" => sha(&digest::SHA256),
        "sha512" => sha(&digest::SHA512),
        "sha384" => sha(&digest::SHA384),
        "sha224" => sha(&digest::SHA224),
        "sha1" => sha(&digest::SHA1_FOR_LEGACY_USE_ONLY),
        "md5" => md5::Md5::digest(&data).to_vec(),
        other => return Err(format!("Digest method not supported: {other}")),
    };
    Ok(Value::String(STANDARD.encode(digest)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_cancels_streams() {
        let streams = AiStreams::default();
        let cancel = CancellationToken::new();
        let (callbacks, calls) = Callbacks::new(cancel.clone());
        streams.insert(None, calls, cancel.clone(), callbacks);
        streams.clear();
        assert!(cancel.is_cancelled());
        assert!(lock(&streams.running).is_empty());
    }

    #[test]
    fn zone_offsets_by_name() {
        let summer = 1_784_116_800;
        let tokyo = json!({ "zone": "asia/tokyo", "time": summer });
        assert_eq!(zone_offset("timezone.offset", &tokyo), json!(9 * 3600));
        assert_eq!(zone_offset("timezone.localOffset", &tokyo), json!(9 * 3600));
        let unknown = json!({ "zone": "Foo/Bar", "time": summer });
        assert_eq!(zone_offset("timezone.offset", &unknown), Value::Null);
        assert!(zone_offset("timezone.offset", &json!(summer)).is_i64());
    }
}
