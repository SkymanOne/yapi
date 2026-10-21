//! Agent session behavior driven by the faux provider.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use yapi_ai::api::Apis;
use yapi_ai::faux::{Faux, Response};
use yapi_ai::registry::ModelRegistry;
use yapi_ai::stream::{EventStream, Provider, Request};
use yapi_core::agent_session::{AgentSession, Resources, SessionConfig, TreeNavigation};
use yapi_core::session::SessionManager;
use yapi_core::settings::SettingsManager;
use yapi_types::message::{
    Content, ContentBlock, ImageContent, Message, StopReason, ThinkingLevel,
};
use yapi_types::model::Model;
use yapi_types::session::FileEntry;

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
    session_resources(provider, Resources::default())
}

fn session_resources(provider: Arc<dyn Provider>, resources: Resources) -> AgentSession {
    let model = faux_model();
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
        resources,
        docs: yapi_core::docs::Locations::default(),
    })
}

/// The sign-in help names the docs the session was given, as its prompt does.
#[tokio::test]
async fn sign_in_help_docs() {
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
async fn summary_records_abandoned_branch() {
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
async fn navigating_returns_message_text() {
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
async fn cancelled_compaction_records_nothing() {
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
async fn converts_prompt_tool_images() {
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

/// The first failed session write reaches the user once, however many follow.
#[tokio::test]
async fn write_error_reported_once() {
    struct Notes(std::sync::Mutex<Vec<String>>);
    impl yapi_core::extensions::ExtensionUi for Notes {
        fn has_ui(&self) -> bool {
            true
        }
        fn notify(&self, message: &str, _kind: yapi_core::extensions::NotifyKind) {
            self.0.lock().unwrap().push(message.to_owned());
        }
    }
    let dir = std::env::temp_dir().join(format!("yapi-write-error-{}", std::process::id()));
    let session = session(&Faux::default());
    let notes = Arc::new(Notes(std::sync::Mutex::default()));
    session
        .bind_extensions(notes.clone(), yapi_core::extensions::Mode::Tui, None, None)
        .await;
    let user = || {
        Message::User(yapi_types::message::UserMessage {
            content: Content::Text("hi".into()),
            timestamp: 0,
        })
    };
    let file = session.with_session(|file| {
        *file = SessionManager::create(Path::new("/work"), &dir, None).unwrap();
        file.append_message(user()).unwrap();
        file.file().unwrap().to_path_buf()
    });
    // A directory in the file's place fails every write.
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    for _ in 0..2 {
        let _ = session.with_session(|file| file.append_message(user()));
    }
    let _ = std::fs::remove_dir_all(&dir);
    let notes = notes.0.lock().unwrap();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0].starts_with("Failed to save session: "),
        "{notes:?}"
    );
}

/// pi's `getLastAssistantText` reads only the last assistant message.
#[tokio::test]
async fn last_text_after_error() {
    let faux = Faux::new([Response::text("one"), Response::error("boom")]);
    let session = session(&faux);
    session.prompt("first", Vec::new()).await.unwrap();
    assert_eq!(session.last_assistant_text().as_deref(), Some("one"));
    session.prompt("second", Vec::new()).await.unwrap();
    assert_eq!(session.last_assistant_text(), None);
}

/// Records the extension errors reported to it.
#[derive(Default)]
struct Errors(std::sync::Mutex<Vec<String>>);

impl yapi_core::extensions::ExtensionUi for Errors {
    fn has_ui(&self) -> bool {
        false
    }
    fn notify(&self, _message: &str, _kind: yapi_core::extensions::NotifyKind) {}
    fn extension_error(&self, path: &str, event: &str, error: &str, _stack: Option<&str>) {
        self.0
            .lock()
            .unwrap()
            .push(format!("{event} {path}: {error}"));
    }
}

/// A `/skill:` name ends at a space only; an unreadable skill file is
/// reported and leaves the prompt as typed, as in pi.
#[tokio::test]
async fn skill_split_space() {
    let dir = std::env::temp_dir().join(format!("yapi-skill-split-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("SKILL.md");
    std::fs::write(&file, "---\nname: review\ndescription: d\n---\nBody\n").unwrap();
    let skill = |name: &str, file_path: &Path| yapi_types::system_prompt::Skill {
        name: name.into(),
        file_path: file_path.to_path_buf(),
        base_dir: dir.clone(),
        ..Default::default()
    };
    let faux = Faux::new([
        Response::text("a"),
        Response::text("b"),
        Response::text("c"),
    ]);
    let resources = Resources {
        skills: vec![skill("review", &file), skill("gone", &dir.join("gone.md"))],
        ..Resources::default()
    };
    let session = session_resources(Arc::new(faux.clone()), resources);
    let errors = Arc::new(Errors::default());
    session
        .bind_extensions(
            errors.clone(),
            yapi_core::extensions::Mode::Print,
            None,
            None,
        )
        .await;
    for prompt in [
        "/skill:review\tfocus",
        "/skill:review  focus ",
        "/skill:gone x",
    ] {
        session.prompt(prompt, Vec::new()).await.unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
    let sent: Vec<String> = faux
        .requests()
        .iter()
        .map(|messages| match messages.last().unwrap() {
            Message::User(user) => user.content.text(""),
            other => panic!("{other:?}"),
        })
        .collect();
    let block = format!(
        "<skill name=\"review\" location=\"{}\">\nReferences are relative to {}.\n\nBody\n</skill>",
        file.display(),
        dir.display()
    );
    assert_eq!(
        sent,
        [
            "/skill:review\tfocus".to_owned(),
            format!("{block}\n\nfocus"),
            "/skill:gone x".to_owned(),
        ]
    );
    let gone = dir.join("gone.md");
    assert_eq!(
        *errors.0.lock().unwrap(),
        [format!(
            "skill_expansion {}: ENOENT: no such file or directory, open '{}'",
            gone.display(),
            gone.display()
        )]
    );
}

/// The faux provider, queueing two steering messages and switching the
/// steering mode to `all` while its first request streams.
struct SteerOnce {
    faux: Faux,
    session: Arc<std::sync::OnceLock<AgentSession>>,
    done: AtomicBool,
}

impl Provider for SteerOnce {
    fn api(&self) -> &str {
        "faux"
    }

    fn stream(&self, request: Request) -> EventStream {
        if !self.done.swap(true, Ordering::SeqCst) {
            let session = self.session.get().unwrap();
            session.steer("s1", Vec::new());
            session.steer("s2", Vec::new());
            session.set_steering_mode(Some("all".into())).unwrap();
        }
        self.faux.stream(request)
    }
}

/// A steering mode change applies to the running agent, as in pi.
#[tokio::test]
async fn steering_mode_live() {
    let faux = Faux::new([
        Response::tool_call("call-1", "missing", serde_json::json!({})),
        Response::text("done"),
    ]);
    let cell = Arc::new(std::sync::OnceLock::new());
    let session = session_with(Arc::new(SteerOnce {
        faux: faux.clone(),
        session: cell.clone(),
        done: AtomicBool::new(false),
    }));
    let _ = cell.set(session.clone());
    session.prompt("go", Vec::new()).await.unwrap();
    let users: Vec<String> = faux.requests()[1]
        .iter()
        .filter_map(|message| match message {
            Message::User(user) => Some(user.content.text("")),
            _ => None,
        })
        .collect();
    assert_eq!(users, ["go", "s1", "s2"]);
}
