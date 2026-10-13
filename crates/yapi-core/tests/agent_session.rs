//! Agent session behavior driven by the faux provider.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use yapi_ai::api::Apis;
use yapi_ai::faux::{Faux, Response};
use yapi_ai::registry::ModelRegistry;
use yapi_ai::stream::{EventStream, Provider, Request, StreamEvent};
use yapi_core::agent_session::{
    AgentSession, CacheWarmingPhase, CacheWarmingState, Resources, SessionConfig, TreeNavigation,
};
use yapi_core::session::SessionManager;
use yapi_core::settings::SettingsManager;
use yapi_types::event::AgentEvent;
use yapi_types::message::{
    Content, ContentBlock, ImageContent, Message, StopReason, ThinkingLevel,
};
use yapi_types::model::Model;
use yapi_types::session::FileEntry;
use yapi_types::settings::CacheWarming;

fn faux_model() -> Model {
    serde_json::from_value(serde_json::json!({
        "id": "faux-1", "name": "Faux", "api": "faux", "provider": "faux", "baseUrl": "",
        "reasoning": false, "input": ["text"],
        "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
        "contextWindow": 100000, "maxTokens": 8000,
    }))
    .unwrap()
}

fn session(faux: &Faux) -> AgentSession {
    session_with(Arc::new(faux.clone()))
}

fn session_with(provider: Arc<dyn Provider>) -> AgentSession {
    session_for(faux_model(), provider)
}

fn session_for(model: Model, provider: Arc<dyn Provider>) -> AgentSession {
    let mut registry = ModelRegistry::builtin();
    registry.register_provider("faux", vec![model.clone()]);
    registry.set_runtime_key("faux", "key".into());
    let mut apis = Apis::default();
    apis.register(provider);
    AgentSession::new(SessionConfig {
        cwd: Path::new("/work").to_path_buf(),
        agent_dir: Path::new("/agent").to_path_buf(),
        settings: SettingsManager::in_memory(),
        registry,
        apis,
        session: SessionManager::in_memory(Path::new("/work")),
        model: Some(model),
        thinking_level: ThinkingLevel::Off,
        tools: Vec::new(),
        extensions: Vec::new(),
        include_extension_tools: false,
        allowed_tools: None,
        excluded_tools: Vec::new(),
        resources: Resources::default(),
        docs: yapi_core::docs::Locations::default(),
    })
}

/// The sign-in help names the docs the session was given, as its prompt does.
#[tokio::test]
async fn sign_in_help_names_the_sessions_docs() {
    let session = AgentSession::new(SessionConfig {
        cwd: Path::new("/work").to_path_buf(),
        agent_dir: Path::new("/agent").to_path_buf(),
        settings: SettingsManager::in_memory(),
        registry: ModelRegistry::builtin(),
        apis: Apis::default(),
        session: SessionManager::in_memory(Path::new("/work")),
        model: None,
        thinking_level: ThinkingLevel::Off,
        tools: Vec::new(),
        extensions: Vec::new(),
        include_extension_tools: false,
        allowed_tools: None,
        excluded_tools: Vec::new(),
        resources: Resources::default(),
        docs: yapi_core::docs::Locations {
            pi_docs: "/agent/docs/pi/docs".into(),
            ..yapi_core::docs::Locations::default()
        },
    });
    let err = session.prompt("hi", Vec::new()).await.unwrap_err();
    assert!(
        err.ends_with("See:\n  /agent/docs/pi/docs/providers.md\n  /agent/docs/pi/docs/models.md"),
        "{err}"
    );
}

fn entry_id(session: &AgentSession, text: &str) -> String {
    session.with_session(|file| {
        file.entries()
            .find_map(|entry| match entry {
                FileEntry::Message(message) => match &message.message {
                    Message::Assistant(assistant)
                        if yapi_types::message::blocks_text(&assistant.content, "") == text =>
                    {
                        Some(message.meta.id.clone())
                    }
                    _ => None,
                },
                _ => None,
            })
            .unwrap()
    })
}

#[tokio::test]
async fn navigating_with_a_summary_records_the_abandoned_branch() {
    let faux = Faux::new([
        Response::text("one"),
        Response::text("two"),
        Response::text("## Goal\nExplore."),
    ]);
    let session = session(&faux);
    session.prompt("first", Vec::new()).await.unwrap();
    session.prompt("second", Vec::new()).await.unwrap();
    let target = entry_id(&session, "one");

    let outcome = session
        .navigate_tree(
            &target,
            TreeNavigation {
                summarize: true,
                label: Some("checkpoint".into()),
                ..TreeNavigation::default()
            },
        )
        .await
        .unwrap();

    let Some(FileEntry::BranchSummary(summary)) = &outcome.summary_entry else {
        panic!("expected a branch summary, got {outcome:?}");
    };
    assert!(summary.summary.starts_with(
        "The user explored a different conversation branch before returning here.\nSummary of that exploration:\n\n## Goal\nExplore."
    ));
    assert_eq!(summary.meta.parent_id.as_deref(), Some(target.as_str()));
    let prompt = faux.requests().last().unwrap().clone();
    let text = match &prompt[1] {
        Message::User(user) => match &user.content {
            yapi_types::message::Content::Blocks(blocks) => {
                yapi_types::message::blocks_text(blocks, "")
            }
            yapi_types::message::Content::Text(text) => text.clone(),
        },
        other => panic!("expected the summary prompt, got {other:?}"),
    };
    assert!(
        text.starts_with("<conversation>\n[User]: second\n\n[Assistant]: two\n</conversation>")
    );
    let roles: Vec<&str> = session
        .messages()
        .iter()
        .map(|message| match message {
            Message::System(_) => "system",
            Message::User(_) => "user",
            Message::Assistant(_) => "assistant",
            Message::BranchSummary(_) => "branchSummary",
            _ => "other",
        })
        .collect();
    assert_eq!(roles, ["system", "user", "assistant", "branchSummary"]);
    let id = summary.meta.id.clone();
    assert_eq!(
        session.with_session(|file| file.label(&id).map(str::to_owned)),
        Some("checkpoint".into())
    );
}

#[tokio::test]
async fn navigating_to_a_user_message_returns_its_text() {
    let faux = Faux::new([Response::text("one")]);
    let session = session(&faux);
    session.prompt("first", Vec::new()).await.unwrap();
    let user = session.with_session(|file| {
        file.entries()
            .find_map(|entry| match entry {
                FileEntry::Message(message) if matches!(message.message, Message::User(_)) => {
                    Some(message.meta.id.clone())
                }
                _ => None,
            })
            .unwrap()
    });
    let outcome = session
        .navigate_tree(&user, TreeNavigation::default())
        .await
        .unwrap();
    assert_eq!(outcome.editor_text.as_deref(), Some("first"));
    assert!(outcome.summary_entry.is_none());
    assert!(
        !session
            .messages()
            .iter()
            .any(|message| matches!(message, Message::User(_)))
    );
}

/// The faux provider, cancelling the requests it gets once `cancel` is set,
/// as an abort while they stream does.
struct Cancelling {
    faux: Faux,
    cancel: Arc<AtomicBool>,
}

impl Provider for Cancelling {
    fn api(&self) -> &str {
        "faux"
    }

    fn stream(&self, request: Request) -> EventStream {
        if self.cancel.load(Ordering::SeqCst) {
            request.options.cancel.cancel();
        }
        self.faux.stream(request)
    }
}

#[tokio::test]
async fn a_manual_compaction_cancelled_mid_summary_records_nothing() {
    let faux = Faux::new([
        Response::text("one"),
        Response::text("two"),
        Response {
            stop_reason: Some(StopReason::Aborted),
            ..Response::text("## Goal\nPartial")
        },
    ]);
    let cancel = Arc::new(AtomicBool::new(false));
    let session = session_with(Arc::new(Cancelling {
        faux,
        cancel: Arc::clone(&cancel),
    }));
    session
        .set_nested_global_setting("compaction", "keepRecentTokens", 2.into())
        .unwrap();
    session.prompt("first", Vec::new()).await.unwrap();
    session.prompt("second", Vec::new()).await.unwrap();

    cancel.store(true, Ordering::SeqCst);
    let error = session.compact(None).await.unwrap_err();

    // As pi, which checks the abort before it appends the compaction.
    assert_eq!(error, "Compaction cancelled");
    assert!(session.with_session(|file| {
        !file
            .entries()
            .any(|entry| matches!(entry, FileEntry::Compaction(_)))
    }));
}

/// A tool that returns a BMP screenshot, as tools of extensions may.
struct Screenshot(yapi_types::message::ToolDeclaration, String);

impl yapi_agent::Tool for Screenshot {
    fn declaration(&self) -> &yapi_types::message::ToolDeclaration {
        &self.0
    }

    fn execute(
        &self,
        _call_id: String,
        _args: serde_json::Value,
        _cancel: tokio_util::sync::CancellationToken,
        _updates: yapi_agent::UpdateSink,
    ) -> futures_util::future::BoxFuture<'_, Result<yapi_types::event::ToolResult, String>> {
        Box::pin(async move {
            Ok(yapi_types::event::ToolResult {
                content: vec![ContentBlock::Image(bmp(&self.1))],
                ..Default::default()
            })
        })
    }
}

fn bmp(data: &str) -> ImageContent {
    ImageContent {
        data: data.to_owned(),
        mime_type: "image/bmp".into(),
    }
}

#[tokio::test]
async fn prompt_and_tool_result_images_are_converted_as_pi() {
    use base64::Engine as _;
    let mut file = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(2, 2)
        .write_to(&mut file, image::ImageOutputFormat::Bmp)
        .unwrap();
    let data = base64::engine::general_purpose::STANDARD.encode(file.into_inner());
    let faux = Faux::new([
        Response::tool_call("call-1", "screenshot", serde_json::json!({})),
        Response::text("done"),
    ]);
    let session = session(&faux);
    let declaration = yapi_types::message::ToolDeclaration {
        name: "screenshot".into(),
        description: "Take a screenshot".into(),
        parameters: serde_json::json!({"type": "object"}),
        constrained_sampling: None,
    };
    session
        .tools()
        .register(yapi_core::tools::RegisteredTool::direct(
            Arc::new(Screenshot(declaration, data.clone())),
            None,
            Vec::new(),
        ));
    session.prompt("Look", vec![bmp(&data)]).await.unwrap();

    let note = "[Image converted from image/bmp to image/png.]";
    let messages = session.messages();
    let user = messages
        .iter()
        .find_map(|message| match message {
            Message::User(user) => Some(user),
            _ => None,
        })
        .unwrap();
    let Content::Blocks(blocks) = &user.content else {
        panic!("blocks");
    };
    assert_eq!(blocks[0], ContentBlock::text(format!("Look\n\n{note}")));
    let ContentBlock::Image(png) = &blocks[1] else {
        panic!("image");
    };
    assert_eq!(png.mime_type, "image/png");
    let result = messages
        .iter()
        .find_map(|message| match message {
            Message::ToolResult(result) => Some(result),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        result.content,
        [ContentBlock::Image(png.clone()), ContentBlock::text(note)]
    );
}

/// A model with a five-minute prompt cache whose writes cost far more than
/// reads, so warming pays off even between runs.
fn cached_model() -> Model {
    serde_json::from_value(serde_json::json!({
        "id": "cached", "name": "Cached", "api": "faux", "provider": "faux", "baseUrl": "",
        "reasoning": false, "input": ["text"],
        "cost": {"input": 3, "output": 15, "cacheRead": 1, "cacheWrite": 100},
        "promptCache": {"short": 300},
        "contextWindow": 100000, "maxTokens": 8000,
    }))
    .unwrap()
}

/// Answers each request with text and a 20,000-token prompt written to the
/// cache, after the next of `delays` for requests without an output cap.
/// Records each request's output cap.
#[derive(Clone, Default)]
struct Cached {
    caps: Arc<std::sync::Mutex<Vec<Option<u64>>>>,
    delays: Arc<std::sync::Mutex<std::collections::VecDeque<Duration>>>,
}

impl Cached {
    fn caps(&self) -> Vec<Option<u64>> {
        self.caps.lock().unwrap().clone()
    }
}

impl Provider for Cached {
    fn api(&self) -> &str {
        "faux"
    }

    fn stream(&self, request: Request) -> EventStream {
        self.caps.lock().unwrap().push(request.options.max_tokens);
        let delay = match request.options.max_tokens {
            Some(_) => Duration::ZERO,
            None => self.delays.lock().unwrap().pop_front().unwrap_or_default(),
        };
        let (sender, stream) = EventStream::channel();
        let mut output = yapi_ai::stream::new_output(&request.model, 0);
        output.content = vec![ContentBlock::text("ok")];
        output.usage.cache_write = 20_000;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            sender.send(StreamEvent::Done(output));
        });
        stream
    }
}

fn warm_entries(session: &AgentSession) -> Vec<yapi_types::session::UsageEntry> {
    session.with_session(|file| {
        file.entries()
            .filter_map(|entry| match entry {
                FileEntry::Usage(usage) => Some(usage.clone()),
                _ => None,
            })
            .collect()
    })
}

#[tokio::test(start_paused = true)]
async fn idle_warming_refreshes_the_cache_until_its_limit() {
    let provider = Cached::default();
    let session = session_for(cached_model(), Arc::new(provider.clone()));
    session.set_cache_warming_mode(CacheWarming::Idle).unwrap();
    let appended = Arc::new(std::sync::Mutex::new(0));
    let counter = Arc::clone(&appended);
    session.subscribe(Box::new(move |event| {
        if let AgentEvent::EntryAppended {
            entry: FileEntry::Usage(_),
        } = event
        {
            *counter.lock().unwrap() += 1;
        }
    }));
    session.prompt("hi", Vec::new()).await.unwrap();

    let status = session.cache_warming_status();
    assert_eq!(status.state, CacheWarmingState::Scheduled);
    let decision = status.decision.unwrap();
    assert_eq!(decision.phase, CacheWarmingPhase::Idle);
    assert_eq!(decision.continuation_probability, 0.15);
    assert!(decision.warm);

    // Refreshes come 270 s apart, at 90% of the five-minute lifetime.
    tokio::time::sleep(Duration::from_secs(271)).await;
    assert_eq!(provider.caps(), [None, Some(1)]);
    let entries = warm_entries(&session);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (entries[0].kind.as_str(), entries[0].provider.as_str()),
        ("cache_warm", "faux")
    );
    assert_eq!(entries[0].model, "cached");
    assert_eq!(entries[0].usage.cache_write, 20_000);
    assert_eq!(entries[0].note, None);
    assert_eq!(*appended.lock().unwrap(), 1);
    // The context is unchanged, so the next refresh is scheduled.
    assert_eq!(
        session.cache_warming_status().state,
        CacheWarmingState::Scheduled
    );

    // The sixth refresh, at 27 minutes, is the last within the 30-minute
    // idle limit.
    tokio::time::sleep(Duration::from_secs(30 * 60)).await;
    assert_eq!(provider.caps().len(), 7);
    assert_eq!(warm_entries(&session).len(), 6);
    let status = session.cache_warming_status();
    assert_eq!(status.state, CacheWarmingState::Inactive);
    assert_eq!(status.reason, Some("30-minute idle safety limit reached"));
}

#[tokio::test(start_paused = true)]
async fn streaming_warming_refreshes_while_the_run_lasts() {
    let provider = Cached::default();
    provider
        .delays
        .lock()
        .unwrap()
        .extend([Duration::ZERO, Duration::from_secs(400)]);
    let session = session_for(cached_model(), Arc::new(provider.clone()));
    session.prompt("first", Vec::new()).await.unwrap();
    assert_eq!(
        session.cache_warming_status().reason,
        Some("agent run settled")
    );

    // The second response takes 400 s; the request's cache entry is
    // refreshed at 270 s, priced from the first response's prompt.
    let run = tokio::spawn({
        let session = session.clone();
        async move { session.prompt("second", Vec::new()).await }
    });
    tokio::time::sleep(Duration::from_secs(271)).await;
    assert_eq!(provider.caps(), [None, None, Some(1)]);
    let status = session.cache_warming_status();
    assert_eq!(status.state, CacheWarmingState::Scheduled);
    assert_eq!(status.decision.unwrap().phase, CacheWarmingPhase::Streaming);

    run.await.unwrap().unwrap();
    assert_eq!(
        session.cache_warming_status().reason,
        Some("agent run settled")
    );
    tokio::time::sleep(Duration::from_secs(600)).await;
    assert_eq!(provider.caps().len(), 3);
}

#[tokio::test(start_paused = true)]
async fn warming_stops_when_the_context_or_mode_changes() {
    let provider = Cached::default();
    let session = session_for(cached_model(), Arc::new(provider.clone()));
    session.set_cache_warming_mode(CacheWarming::Idle).unwrap();
    session.prompt("hi", Vec::new()).await.unwrap();
    session
        .set_model(Model {
            id: "other".into(),
            ..cached_model()
        })
        .unwrap();
    assert_eq!(
        session.cache_warming_status().reason,
        Some("conversation context changed")
    );
    tokio::time::sleep(Duration::from_secs(300)).await;
    assert_eq!(provider.caps(), [None]);

    session.set_model(cached_model()).unwrap();
    session.prompt("again", Vec::new()).await.unwrap();
    assert_eq!(
        session.cache_warming_status().state,
        CacheWarmingState::Scheduled
    );
    session.set_cache_warming_mode(CacheWarming::Off).unwrap();
    assert_eq!(
        session.cache_warming_status().reason,
        Some("cache warming disabled")
    );
    tokio::time::sleep(Duration::from_secs(300)).await;
    assert_eq!(provider.caps(), [None, None]);
}

#[tokio::test(start_paused = true)]
async fn warming_needs_a_cache_lifetime_and_economics() {
    let mut model = cached_model();
    model.prompt_cache = None;
    let session = session_for(model, Arc::new(Cached::default()));
    session.prompt("hi", Vec::new()).await.unwrap();
    session.set_cache_warming_mode(CacheWarming::Idle).unwrap();
    assert_eq!(
        session.cache_warming_status().reason,
        Some("cache lifetime unavailable")
    );

    // Free cache reads and writes: nothing to save.
    let mut model = cached_model();
    model.cost = Default::default();
    let session = session_for(model, Arc::new(Cached::default()));
    session.set_cache_warming_mode(CacheWarming::Idle).unwrap();
    session.prompt("hi", Vec::new()).await.unwrap();
    assert_eq!(
        session.cache_warming_status().reason,
        Some("cache economics unavailable")
    );
}
