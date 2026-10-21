//! Turning arguments into a session: settings, trust, session file, model, tools
//! and prompt resources. Mirrors `main.ts` and `sdk.ts` in pi `v1.0.0`.

use std::io::IsTerminal;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, bail};
use yapi_ai::api::Apis;
use yapi_ai::registry::ModelRegistry;
use yapi_core::agent_session::{AgentSession, Replacement, Resources, SessionConfig};
use yapi_core::config::{SESSION_DIR_ENV, agent_dir, default_session_dir};
use yapi_core::extensions::discovery::is_native;
use yapi_core::model_resolver::{
    DEFAULT_THINKING_LEVEL, initial_model, resolve_cli_model, resolve_model_scope,
};
use yapi_core::packages::resolve::ResourceType;
use yapi_core::resources::{Diagnostic, context_files, system_prompt_file};
use yapi_core::session::{self, SessionManager};
use yapi_core::settings::Scope;
use yapi_core::settings::SettingsManager;
use yapi_core::tools::path::resolve_to_cwd;
use yapi_core::trust::{TrustStore, resolve_trusted};
use yapi_ext::{Engine, ExtensionHost};
use yapi_types::message::{ImageContent, ThinkingLevel};
use yapi_types::rpc::SourceInfo;
use yapi_types::sync::lock;

use crate::args::{Args, FlagValue};

/// A ready session and what to send to it.
pub struct Startup {
    /// The session.
    pub session: AgentSession,
    /// pi's `modelFallbackMessage`, which the interactive mode shows.
    pub model_fallback: Option<String>,
    /// The first prompt: piped stdin, `@file` contents and the first message.
    pub initial_message: Option<String>,
    /// Images from `@file` arguments.
    pub initial_images: Vec<ImageContent>,
    /// Remaining messages, sent one after another.
    pub messages: Vec<String>,
}

/// Reads piped stdin when stdin is not a terminal; empty input is no input.
pub fn read_piped_stdin() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut text = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut text).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// pi's `resolvePromptInput`: a path to an existing file means its contents.
fn prompt_input(input: &str) -> String {
    let path = Path::new(input);
    if path.exists()
        && let Ok(text) = std::fs::read_to_string(path)
    {
        return text
            .strip_prefix('\u{feff}')
            .map(str::to_owned)
            .unwrap_or(text);
    }
    input.to_owned()
}

/// The texts appended to the system prompt: the `--append-system-prompt`
/// values, each a file or text, else the `discovered` `APPEND_SYSTEM.md`.
/// An empty value adds nothing.
fn append_prompts(flags: &[String], discovered: impl FnOnce() -> Option<String>) -> Vec<String> {
    if flags.is_empty() {
        return discovered().into_iter().collect();
    }
    flags
        .iter()
        .filter(|input| !input.is_empty())
        .map(|input| prompt_input(input))
        .collect()
}

fn file_arguments(files: &[String], cwd: &Path) -> anyhow::Result<(String, Vec<ImageContent>)> {
    let mut text = String::new();
    let mut images = Vec::new();
    for file in files {
        let path = yapi_core::tools::path::resolve_read_path(file, cwd);
        let metadata = std::fs::metadata(&path)
            .map_err(|_| anyhow::anyhow!("Error: File not found: {}", path.display()))?;
        if metadata.len() == 0 {
            continue;
        }
        let bytes = std::fs::read(&path)
            .with_context(|| format!("Error: Could not read file {}", path.display()))?;
        if let Some(mime_type) = yapi_core::tools::image_mime_type(&bytes) {
            // The session resizes prompt images once the request's model is known.
            let note = match yapi_core::images::process(&bytes, mime_type, false, None) {
                Ok(processed) => {
                    images.push(processed.image);
                    processed.hints.join("\n")
                }
                Err(message) => message.to_owned(),
            };
            text += &format!("<file name=\"{}\">{note}</file>\n", path.display());
        } else {
            let content = String::from_utf8_lossy(&bytes);
            let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
            text += &format!("<file name=\"{}\">\n{content}\n</file>\n", path.display());
        }
    }
    Ok((text, images))
}

/// Where `--session`, `--fork` and `--resume` arguments point.
enum Resolved {
    /// A file path or a session of this project.
    Local(PathBuf),
    /// A session of another project, with its working directory.
    Global(PathBuf, String),
    /// Nothing matched.
    NotFound,
}

/// pi's `resolveSessionPath`: a path, else an exact or prefix id in this
/// project, else an exact or prefix id in any project.
fn resolve_session(
    target: &str,
    cwd: &Path,
    dir: &Path,
    cwd_filter: Option<&Path>,
    custom_dir: Option<&Path>,
    agent_dir: &Path,
) -> Resolved {
    if target.contains('/') || target.contains('\\') || target.ends_with(".jsonl") {
        return Resolved::Local(resolve_to_cwd(target, cwd));
    }
    if let Some(path) = session::find_by_id(dir, target, cwd_filter) {
        return Resolved::Local(path);
    }
    if let Some(found) = session::list(dir, cwd_filter)
        .into_iter()
        .find(|summary| summary.id.starts_with(target))
    {
        return Resolved::Local(found.path);
    }
    let all = match custom_dir {
        Some(custom) => session::list(custom, None),
        None => session::list_all(&agent_dir.join("sessions")),
    };
    match all
        .iter()
        .find(|summary| summary.id == target)
        .or_else(|| all.iter().find(|summary| summary.id.starts_with(target)))
    {
        Some(found) => Resolved::Global(found.path.clone(), found.cwd.clone()),
        None => Resolved::NotFound,
    }
}

/// Asks a yes/no question on the terminal; anything but `y` is no. Asks on
/// stderr, as stdout carries print, JSON and RPC output.
fn confirm(question: &str) -> bool {
    use std::io::{BufRead, Write};
    let mut out = std::io::stderr();
    let _ = write!(out, "{question} [y/N] ");
    let _ = out.flush();
    let mut answer = String::new();
    let _ = std::io::stdin().lock().read_line(&mut answer);
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

/// pi's `createSessionManager`. `custom_dir` is the session directory from
/// `--session-dir`, the environment or settings.
fn open_session(
    args: &Args,
    cwd: &Path,
    agent_dir: &Path,
    custom_dir: Option<&Path>,
) -> anyhow::Result<SessionManager> {
    if args.no_session || args.help || args.list_models.is_some() {
        let mut session = SessionManager::in_memory(cwd);
        if let Some(id) = &args.session_id {
            session.new_session(Some(id.clone()), None);
        }
        return Ok(session);
    }
    let default_dir = default_session_dir(agent_dir, cwd);
    let dir = custom_dir.map_or_else(|| default_dir.clone(), Path::to_path_buf);
    // A shared custom directory holds other projects' sessions too.
    let filter = custom_dir.is_some() && dir != default_dir;
    let cwd_filter = filter.then_some(cwd);
    let fail = |err: yapi_core::session::SessionError| anyhow::anyhow!("Error: {err}");

    if let Some(source) = &args.fork {
        if let Some(id) = &args.session_id
            && session::find_by_id(&dir, id, cwd_filter).is_some()
        {
            bail!("Session already exists with id '{id}'");
        }
        return match resolve_session(source, cwd, &dir, cwd_filter, custom_dir, agent_dir) {
            Resolved::Local(path) | Resolved::Global(path, _) => {
                SessionManager::fork_from(&path, cwd, &dir, args.session_id.clone()).map_err(fail)
            }
            Resolved::NotFound => bail!("No session found matching '{source}'"),
        };
    }
    if let Some(target) = &args.session {
        return match resolve_session(target, cwd, &dir, cwd_filter, custom_dir, agent_dir) {
            Resolved::Local(path) => SessionManager::open(&path, custom_dir, None).map_err(fail),
            Resolved::Global(path, other) => {
                if !confirm(&format!(
                    "Session found in different project: {other}\nFork this session into current directory?"
                )) {
                    use std::io::Write;
                    let _ = writeln!(std::io::stderr(), "Aborted.");
                    std::process::exit(0);
                }
                SessionManager::fork_from(&path, cwd, &dir, None).map_err(fail)
            }
            Resolved::NotFound => bail!("No session found matching '{target}'"),
        };
    }
    if args.resume {
        bail!("Selecting a session with --resume needs a terminal; use --session or --continue");
    }
    if args.continue_ {
        return SessionManager::continue_recent(cwd, &dir, filter).map_err(fail);
    }
    if let Some(id) = &args.session_id {
        if let Some(path) = session::find_by_id(&dir, id, cwd_filter) {
            return SessionManager::open(&path, custom_dir, None).map_err(fail);
        }
        eprintln!(
            "Warning: No project session found with id '{id}'; creating a new session with that id."
        );
    }
    SessionManager::create(cwd, &dir, args.session_id.clone()).map_err(fail)
}

fn tool_names(args: &Args, settings: &SettingsManager) -> Vec<String> {
    if args.no_tools || args.no_builtin_tools {
        return Vec::new();
    }
    let mut names: Vec<String> = yapi_core::tools::DEFAULT_TOOLS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    if let Some(configured) = &settings.settings().default_tools {
        for entry in configured {
            if let Some(name) = entry.strip_prefix('+') {
                if !names.iter().any(|existing| existing == name) {
                    names.push(name.to_owned());
                }
            } else if let Some(name) = entry.strip_prefix('-') {
                names.retain(|existing| existing != name);
            } else {
                names = configured
                    .iter()
                    .filter(|entry| !entry.starts_with('+') && !entry.starts_with('-'))
                    .cloned()
                    .collect();
                break;
            }
        }
    }
    if let Some(tools) = &args.tools {
        names = tools.clone();
    }
    if let Some(excluded) = &args.exclude_tools {
        names.retain(|name| !excluded.contains(name));
    }
    names
}

/// The session directory from `--session-dir`, the environment or settings;
/// an empty value is unset.
fn custom_session_dir(args: &Args, settings: &SettingsManager, cwd: &Path) -> Option<PathBuf> {
    let set = |dir: &&str| !dir.is_empty();
    let env_dir = std::env::var(SESSION_DIR_ENV).ok();
    args.session_dir
        .as_deref()
        .filter(set)
        .or(env_dir.as_deref().filter(set))
        .or(settings.settings().session_dir.as_deref().filter(set))
        .map(|dir| resolve_to_cwd(dir, cwd))
}

/// The directory whose project files a session in `cwd` uses:
/// `--project-dir`, else `cwd`.
pub fn project_dir(args: &Args, cwd: &Path) -> PathBuf {
    args.project_dir
        .clone()
        .unwrap_or_else(|| cwd.to_path_buf())
}

/// Settings for the project in `project` and whether it is trusted: pi's
/// `createCommandSettingsManager`. Project settings load when `override_`, a
/// stored decision or `defaultProjectTrust` trusts the project, or, with
/// `ask` and a terminal, when the user trusts it at pi's prompt.
pub fn load_settings(
    project: &Path,
    agent_dir: &Path,
    override_: Option<bool>,
    ask: bool,
) -> anyhow::Result<(SettingsManager, bool)> {
    use std::io::IsTerminal;
    let global = SettingsManager::load(agent_dir, project, false);
    let view = global.settings();
    let store = TrustStore::new(agent_dir);
    let default = view.default_project_trust;
    let trusted = if ask
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && yapi_core::trust::needs_prompt(project, &store, override_, default)?
    {
        crate::interactive::picker::ask_project_trust(agent_dir, project, view.theme.as_deref())?
            .unwrap_or(false)
    } else {
        resolve_trusted(project, &store, override_, default)?
    };
    let settings = if trusted {
        SettingsManager::load(agent_dir, project, true)
    } else {
        global
    };
    Ok((settings, trusted))
}

/// The settings as the run starts, loaded once for the steps that build the
/// first session.
pub struct RunSettings {
    /// The process's working directory.
    pub cwd: PathBuf,
    /// The settings of its project.
    pub settings: SettingsManager,
    /// Whether the project is trusted.
    pub trusted: bool,
}

/// Loads the [`RunSettings`] without prompting.
fn run_settings(args: &Args) -> anyhow::Result<RunSettings> {
    let cwd = std::env::current_dir()
        .map_err(|err| anyhow::anyhow!("reading the working directory: {err}"))?;
    let (settings, trusted) = load_settings(
        &project_dir(args, &cwd),
        &agent_dir(),
        args.project_trust_override,
        false,
    )?;
    Ok(RunSettings {
        cwd,
        settings,
        trusted,
    })
}

/// Where `--resume` looks: the working directory, the custom session
/// directory if any, and the theme setting.
pub fn resume_context(args: &Args, run: &RunSettings) -> (Option<PathBuf>, Option<String>) {
    let custom = custom_session_dir(args, &run.settings, &run.cwd);
    (custom, run.settings.settings().theme.clone())
}

/// The user cancelled a startup prompt; the run ends without an error.
#[derive(Debug, thiserror::Error)]
#[error("")]
pub struct Cancelled;

/// Builds the session for a run. Errors are user-facing messages; a
/// [`Cancelled`] error ends the run quietly. `interactive` allows prompts.
pub fn start(
    args: &mut Args,
    stdin: Option<String>,
    extensions: &Extensions,
    interactive: bool,
    run: RunSettings,
) -> anyhow::Result<Startup> {
    let RunSettings {
        cwd,
        settings,
        trusted,
    } = run;
    let agent_dir = agent_dir();
    let custom_dir = custom_session_dir(args, &settings, &cwd);
    let mut session = open_session(args, &cwd, &agent_dir, custom_dir.as_deref())?;
    // pi's missing-cwd check: the interactive mode offers to continue in the
    // current directory; the other modes refuse the session.
    if let Some(file) = session.file().map(Path::to_path_buf)
        && !session.cwd().as_os_str().is_empty()
        && !session.cwd().exists()
    {
        let session_cwd = session.cwd().to_path_buf();
        if !interactive {
            bail!(
                "{}",
                crate::runtime::SwitchError::MissingCwd {
                    file,
                    session_cwd,
                    fallback: cwd,
                }
            );
        }
        let title = format!(
            "cwd from session file does not exist\n{}\n\ncontinue in current cwd\n{}",
            session_cwd.display(),
            cwd.display()
        );
        let theme = args.use_theme.clone().or(settings.settings().theme.clone());
        let choice = crate::interactive::picker::ask_choice(
            &agent_dir,
            theme.as_deref(),
            &title,
            &["Continue", "Cancel"],
        )?;
        if choice != Some(0) {
            return Err(Cancelled.into());
        }
        session = SessionManager::open(&file, custom_dir.as_deref(), Some(&cwd))
            .map_err(|err| anyhow::anyhow!("Error: {err}"))?;
    }
    // pi names the session before anything else is recorded in it.
    if let Some(name) = &args.name {
        let name = name.trim();
        if name.is_empty() {
            bail!("Error: --name requires a non-empty value");
        }
        session
            .append_session_info(name)
            .map_err(|err| anyhow::anyhow!("Error: {err}"))?;
    }
    // Settings for another working directory load with the session.
    let preloaded = (session.cwd() == cwd).then_some((settings, trusted));
    let (session, model_fallback) = build(args, session, true, extensions, preloaded)?;

    let (file_text, images) = file_arguments(&args.file_args, &cwd)?;
    let mut parts = Vec::new();
    if let Some(stdin) = stdin {
        parts.push(stdin);
    }
    if !file_text.is_empty() {
        parts.push(file_text);
    }
    if !args.messages.is_empty() {
        parts.push(args.messages.remove(0));
    }
    let initial_message = (!parts.is_empty()).then(|| parts.concat());
    Ok(Startup {
        session,
        model_fallback,
        initial_message,
        initial_images: images,
        messages: std::mem::take(&mut args.messages),
    })
}

/// pi's runtime factory for the sessions that replace the first: each is
/// built around its file in its working directory, with the settings on disk
/// then, as the first one was from the arguments. As pi creates a runtime,
/// the extensions' factories run first, so the session gets their
/// registrations, and the extensions that failed to load are its
/// `[Extension issues]`. Each session gets the extensions as the files on
/// disk give them then, through [`session_extensions`]: a reload or another
/// working directory imports every file again, as pi clears its module
/// cache then. Extensions read the terminal's size as they load when
/// `terminal`.
pub fn factory(
    args: &Args,
    extensions: Extensions,
    terminal: bool,
) -> crate::runtime::SessionFactory {
    let args = Arc::new(args.clone());
    let current = Arc::new(std::sync::Mutex::new(Arc::new(extensions)));
    Arc::new(move |session, reason| {
        let (args, current) = (args.clone(), current.clone());
        Box::pin(async move {
            let cwd = session.cwd().to_path_buf();
            let previous = Arc::clone(&lock(&current));
            let keep = (reason != Replacement::Reload && previous.cwd == cwd).then_some(&*previous);
            let size = terminal.then(yapi_tui::terminal::size);
            let (extensions, run) = session_extensions(&args, cwd, size, keep).await?;
            let extensions = Arc::new(extensions);
            *lock(&current) = Arc::clone(&extensions);
            let preloaded = Some((run.settings, run.trusted));
            let (session, _) = build(&args, session, false, &extensions, preloaded)?;
            for (path, error) in extensions.failures() {
                session.add_extension_load_error(path, error);
            }
            Ok(session)
        })
    })
}

/// Builds a session around `session` in its working directory, with the
/// files of that directory's project ([`project_dir`]): settings, model,
/// thinking level, tools and prompt resources follow the arguments.
/// `preloaded` are the settings of that project and whether it is trusted,
/// when already loaded, those `extensions` were found with, so their
/// resolved resources serve the session too. `warn` reports model problems
/// on stderr, which only the first session may do. Returns pi's
/// `modelFallbackMessage` too: why the session's model could not be
/// restored, or that no model is available.
fn build(
    args: &Args,
    session: SessionManager,
    warn: bool,
    extensions: &Extensions,
    preloaded: Option<(SettingsManager, bool)>,
) -> anyhow::Result<(AgentSession, Option<String>)> {
    let cwd = session.cwd().to_path_buf();
    let agent_dir = agent_dir();
    // The prompt, the sign-in help and codemode agree on where the docs are.
    let docs = yapi_core::docs::Locations::find(&agent_dir);
    let resolved_with_extensions = preloaded.is_some();
    let (settings, trusted) = match preloaded {
        Some(preloaded) => preloaded,
        None => load_settings(
            &project_dir(args, &cwd),
            &agent_dir,
            args.project_trust_override,
            false,
        )?,
    };
    let project = settings.project_dir().to_path_buf();
    // pi fails at startup when the setting is invalid.
    let idle = settings
        .http_idle_timeout_ms()
        .map_err(|message| anyhow::anyhow!("Error: {message}"))?;
    yapi_ai::http::set_idle_timeout_ms(idle);
    // A `models.json` error is shown by the interactive mode, as in pi.
    let (mut registry, apis) = models(args, &settings, extensions, warn);
    let builtin_settings = extension_settings(&settings);

    // Model: --model, then the session's last model, then defaults.
    let mut model = None;
    let mut thinking: Option<ThinkingLevel> = args.thinking;
    if let Some(pattern) = &args.model {
        let resolved =
            resolve_cli_model(args.provider.as_deref(), pattern, args.thinking, &registry);
        if warn && let Some(warning) = &resolved.warning {
            eprintln!("Warning: {warning}");
        }
        if let Some(error) = resolved.error {
            bail!("Error: {error}");
        }
        if thinking.is_none() {
            thinking = resolved.thinking_level;
        }
        model = resolved.model;
    } else if args.provider.is_some() {
        bail!(
            "Error: --provider requires --model (for example: --provider {} --model <pattern>)",
            args.provider.as_deref().unwrap_or_default()
        );
    }
    let context = session.build_context();
    let existing = !context.messages.is_empty();
    let settings_view = settings.settings();
    // The scope from `--models` or `enabledModels`; without `--model`, a new
    // session starts on the saved default if it is in scope, else on the
    // first scoped model.
    let patterns = args
        .models
        .clone()
        .or_else(|| settings_view.enabled_models.clone())
        .unwrap_or_default();
    let scoped = if patterns.is_empty() {
        Vec::new()
    } else {
        let available: Vec<_> = registry.available().into_iter().cloned().collect();
        let (scoped, warnings) = resolve_model_scope(&patterns, &available);
        if warn {
            for warning in warnings {
                eprintln!("Warning: {warning}");
            }
        }
        scoped
    };
    if model.is_none() && !scoped.is_empty() && !existing {
        let saved = settings_view
            .default_provider
            .as_deref()
            .zip(settings_view.default_model.as_deref());
        let pick = saved
            .and_then(|(provider, id)| {
                scoped
                    .iter()
                    .find(|entry| entry.model.provider == provider && entry.model.id == id)
            })
            .unwrap_or(&scoped[0]);
        model = Some(pick.model.clone());
        if thinking.is_none() {
            thinking = pick.thinking_level;
        }
    }
    // `--api-key` belongs to the model the command line chose.
    if let Some(key) = &args.api_key {
        let Some(model) = &model else {
            bail!(
                "Error: --api-key requires a model to be specified via --model, --provider/--model, or --models"
            );
        };
        registry.set_runtime_key(&model.provider, key.clone());
    }
    // pi's createAgentSession: the session's model if it still has
    // credentials, else the initial model.
    let mut fallback = None;
    if model.is_none()
        && existing
        && let Some((provider, id)) = &context.model
    {
        model = registry
            .find(provider, id)
            .filter(|found| registry.has_auth(&found.provider))
            .cloned();
        if model.is_none() {
            fallback = Some(format!("Could not restore model {provider}/{id}"));
        }
    }
    if model.is_none() {
        model = initial_model(
            &registry,
            settings_view.default_provider.as_deref(),
            settings_view.default_model.as_deref(),
        );
        fallback = match (&model, fallback) {
            (None, _) => Some(yapi_core::auth_guidance::no_models_available(&docs)),
            (Some(model), Some(message)) => {
                Some(format!("{message}. Using {}/{}", model.provider, model.id))
            }
            (Some(_), None) => None,
        };
    }
    // The session's level unless one was given, whatever chose the model.
    if thinking.is_none() && existing {
        let recorded = session.branch_path(None).iter().any(|entry| {
            matches!(
                entry,
                yapi_types::session::FileEntry::ThinkingLevelChange(_)
            )
        });
        thinking = if recorded {
            ThinkingLevel::parse(&context.thinking_level)
        } else {
            Some(
                settings
                    .default_thinking_level()
                    .unwrap_or(DEFAULT_THINKING_LEVEL),
            )
        };
    }
    let mut thinking_level = thinking
        .or_else(|| {
            let model = model.as_ref()?;
            settings_view
                .model_thinking_levels
                .as_ref()
                .and_then(|levels| levels.get(&model.reference()))
                .copied()
        })
        .or(settings.default_thinking_level())
        .unwrap_or(DEFAULT_THINKING_LEVEL);
    thinking_level = match &model {
        Some(model) => yapi_ai::thinking::clamp_level(model, thinking_level),
        None => ThinkingLevel::Off,
    };

    let tools = tool_names(args, &settings);
    let cli_paths = |paths: &[String]| -> Vec<PathBuf> {
        paths
            .iter()
            .map(|path| resolve_to_cwd(path, &cwd))
            .collect()
    };
    // pi's sources: settings entries are `local`, resolved against their
    // settings file's directory; command-line paths are `cli`, relative to
    // the working directory; package resources carry their package.
    let cli_sources = |paths: &[String]| -> Vec<SourceInfo> {
        cli_paths(paths)
            .iter()
            .map(|path| yapi_core::resources::cli_source(path))
            .collect()
    };
    // pi's order for each kind: `-e` packages' resources, then what settings,
    // packages and discovery enable (unless `--no-skills` and the like), then
    // command-line paths; a path counts once.
    let fresh;
    let resolved = if resolved_with_extensions {
        &extensions.resolved
    } else {
        fresh = yapi_core::packages::resolve_resources(&project, &agent_dir, &settings, &BUILTINS);
        &fresh
    };
    let merge = |cli: &[SourceInfo], kind: ResourceType, skip: bool, extra: Vec<SourceInfo>| {
        yapi_core::resources::merge_sources(
            cli.iter()
                .cloned()
                .chain(resolved.enabled(kind).filter(|_| !skip).cloned())
                .chain(extra),
        )
    };
    let skill_sources = merge(
        &extensions.skills,
        ResourceType::Skills,
        args.no_skills,
        cli_sources(&args.skills),
    );
    let template_sources = merge(
        &extensions.prompts,
        ResourceType::Prompts,
        args.no_prompt_templates,
        cli_sources(&args.prompt_templates),
    );
    let theme_paths: Vec<SourceInfo> = cli_paths(&args.themes)
        .into_iter()
        .map(|path| SourceInfo {
            path: path.to_string_lossy().into_owned(),
            source: "local".into(),
            scope: "temporary".into(),
            origin: "top-level".into(),
            base_dir: None,
        })
        .collect();
    let themes = merge(
        &extensions.themes,
        ResourceType::Themes,
        args.no_themes,
        theme_paths,
    );
    let appends = append_prompts(&args.append_system_prompt, || {
        system_prompt_file(&project, &agent_dir, trusted, "APPEND_SYSTEM.md")
    });
    let (skills, skill_diagnostics) = yapi_core::resources::skills_from(&skill_sources);
    let (templates, mut template_diagnostics) =
        yapi_core::resources::templates_from(&template_sources);
    // pi reports the command line's missing template paths; skill paths are
    // reported as they load.
    for path in cli_paths(&args.prompt_templates) {
        if !path.exists()
            && !template_diagnostics.iter().any(|diagnostic| {
                matches!(diagnostic, Diagnostic::Warning { path: known, .. } | Diagnostic::Error { path: known, .. } if *known == path)
            })
        {
            template_diagnostics.push(Diagnostic::Error {
                message: "Prompt template path does not exist".to_owned(),
                path,
            });
        }
    }
    let resources = Resources {
        context_files: if args.no_context_files {
            Vec::new()
        } else {
            context_files(&project, &agent_dir)
        },
        skills,
        skill_diagnostics,
        skill_sources,
        templates,
        template_diagnostics,
        template_sources,
        custom_prompt: args
            .system_prompt
            .as_deref()
            .map(prompt_input)
            .or_else(|| system_prompt_file(&project, &agent_dir, trusted, "SYSTEM.md")),
        append_prompt: (!appends.is_empty()).then(|| appends.join("\n\n")),
        themes,
    };

    let codemode_cache = agent_dir.join("cache").join("wasm");
    let codemode_docs = format!("{}/codemode.md", docs.pi_docs);
    let session = AgentSession::new(SessionConfig {
        cwd,
        agent_dir,
        settings,
        registry,
        apis,
        session,
        model,
        thinking_level,
        tools,
        // pi runs its built-in extensions after the loaded ones: llama.cpp,
        // codemode, then tool_search and MCP.
        extensions: extensions
            .hosts
            .iter()
            .flat_map(ExtensionHost::for_session)
            .chain(
                std::iter::once(Arc::new(yapi_core::llama::LlamaExtension)
                    as Arc<dyn yapi_core::extensions::Extension>)
                .chain(std::iter::once(
                    Arc::new(yapi_ext::codemode::CodemodeExtension::new(
                        Some(codemode_cache),
                        Some(codemode_docs),
                    )) as Arc<dyn yapi_core::extensions::Extension>,
                ))
                .chain(yapi_core::extensions::builtins())
                .filter(|extension| {
                    let source = extension.source();
                    let name = source.path.strip_prefix(BUILTIN_PREFIX).unwrap_or_default();
                    builtin_enabled(name, args, &builtin_settings)
                }),
            )
            .collect(),
        include_extension_tools: args.tools.is_none() && !args.no_tools,
        allowed_tools: args.tools.clone().or_else(|| args.no_tools.then(Vec::new)),
        excluded_tools: args.exclude_tools.clone().unwrap_or_default(),
        resources,
        docs,
    });
    session.set_scoped_models(scoped);
    for (path, warning) in &extensions.warnings {
        session.add_extension_warning(path.clone(), warning.clone());
    }
    Ok((session, fallback))
}

/// The models of a run: `models.json`, the built-in llama.cpp provider when
/// enabled, and the providers extensions register, after `models.json` as in
/// pi, with their sign-ins and the wire APIs they implement. `warn` reports
/// invalid providers on stderr.
pub fn models(
    args: &Args,
    settings: &SettingsManager,
    extensions: &Extensions,
    warn: bool,
) -> (ModelRegistry, Apis) {
    let mut registry = ModelRegistry::load(&agent_dir());
    // pi's built-in llama.cpp extension provides its provider.
    if builtin_enabled(yapi_core::llama::NAME, args, &extension_settings(settings)) {
        registry.enable_llama();
    }
    let mut apis = Apis::default();
    for host in &extensions.hosts {
        for provider in host.providers() {
            let name = provider.name.clone();
            if let Err(error) = provider.install(&mut registry, &mut apis)
                && warn
            {
                eprintln!("Warning: provider \"{name}\" from an extension is invalid: {error}");
            }
        }
        for api in host.apis() {
            apis.register(api);
        }
    }
    (registry, apis)
}

/// The path prefix naming a built-in extension, as in `-e builtin:mcp`.
const BUILTIN_PREFIX: &str = "builtin:";

/// yapi's built-in extensions, in pi's load order.
pub const BUILTINS: [&str; 4] = ["llama.cpp", "codemode", "tool-search", "mcp"];

/// Whether built-in extension `name` runs, as in pi: `-e builtin:<name>` loads
/// it; otherwise it runs unless `--no-extensions` or the `extensions` setting
/// excludes it. In the user's setting `-builtin:<name>` wins over
/// `+builtin:<name>`, which wins over a `!` pattern; in the project's, the last
/// matching entry decides and overrides the user's.
fn builtin_enabled(name: &str, args: &Args, settings: &[Vec<String>; 2]) -> bool {
    let path = format!("{BUILTIN_PREFIX}{name}");
    if args.extensions.contains(&path) {
        return true;
    }
    if args.no_extensions {
        return false;
    }
    // `+` and `-` name a path exactly; `!` takes a glob.
    let matches = |entry: &str| match entry.split_at(entry.chars().next().map_or(0, char::len_utf8))
    {
        ("+" | "-", target) => target == path,
        ("!", pattern) => yapi_core::glob::matches(pattern, &path),
        _ => false,
    };
    let [project, user] = settings;
    if let Some(entry) = project.iter().rev().find(|entry| matches(entry)) {
        return entry.starts_with('+');
    }
    let has = |sign: char| {
        user.iter()
            .any(|entry| entry.starts_with(sign) && matches(entry))
    };
    !has('-') && (has('+') || !has('!'))
}

/// The `extensions` setting of the project and the user, for
/// [`builtin_enabled`].
fn extension_settings(settings: &SettingsManager) -> [Vec<String>; 2] {
    [Scope::Project, Scope::Global].map(|scope| {
        yapi_core::packages::resolve::string_list(settings.document(scope), "extensions")
    })
}

/// pi's hint after an extension fails to load.
pub const EXTENSION_LOAD_FAILURE_HINT: &str = "Hint: Start without extensions using \"yapi -ne\".";

/// Why the run's extensions cannot start: pi's startup diagnostics.
pub struct ExtensionErrors {
    /// Error messages, without the `Error: ` prefix.
    pub messages: Vec<String>,
    /// Whether an extension failed to load, which earns pi's hint.
    pub load_failed: bool,
    /// What loaded despite the errors, when the extensions could load at all.
    /// pi's `--help` and `--list-models` use it.
    pub loaded: Option<Box<(Extensions, RunSettings)>>,
}

/// Loads the run's settings and its pi extensions: `-e` paths, then, unless
/// `--no-extensions`, those installed in a trusted project and in the agent
/// directory. Applies extension flags from the command line, which must name
/// registered flags. Extensions read `terminal_size` as they load, in a mode
/// that draws on the terminal.
pub async fn load_extensions(
    args: &Args,
    terminal_size: Option<(usize, usize)>,
) -> Result<(Extensions, RunSettings), ExtensionErrors> {
    let fail = |message: String| ExtensionErrors {
        messages: vec![message],
        load_failed: false,
        loaded: None,
    };
    let run = run_settings(args).map_err(|err| fail(err.to_string()))?;
    let warn = |_: &str, message: String| {
        eprintln!("Warning: {message}");
        ControlFlow::Continue(())
    };
    let (extensions, flag_errors) = start_extensions(args, &run, terminal_size, warn, None)
        .await
        .map_err(fail)?;
    let mut messages: Vec<String> = extensions
        .failures()
        .into_iter()
        .map(|(path, error)| format!("Failed to load extension \"{path}\": {error}"))
        .collect();
    let load_failed = !messages.is_empty();
    messages.extend(flag_errors);
    if !messages.is_empty() {
        return Err(ExtensionErrors {
            messages,
            load_failed,
            loaded: Some(Box::new((extensions, run))),
        });
    }
    Ok((extensions, run))
}

/// The extensions for a session in `cwd` as the settings of its project
/// ([`project_dir`]) and the files on disk give them now, with those
/// settings, as pi's runtime loads them for each session. Missing packages
/// are installed, and those that fail to are listed with the extensions that
/// failed to load. `previous`, the extensions of the session before in the
/// same directory, keeps the modules it imported, as pi's module cache does;
/// without it every file is imported. The command line's flags apply to the
/// extensions that still register them.
async fn session_extensions(
    args: &Args,
    cwd: PathBuf,
    terminal_size: Option<(usize, usize)>,
    previous: Option<&Extensions>,
) -> anyhow::Result<(Extensions, RunSettings)> {
    let (settings, trusted) = load_settings(
        &project_dir(args, &cwd),
        &agent_dir(),
        args.project_trust_override,
        false,
    )?;
    let run = RunSettings {
        cwd,
        settings,
        trusted,
    };
    let mut uninstalled = Vec::new();
    let install_failed = |source: &str, message| {
        uninstalled.push((source.to_owned(), message));
        ControlFlow::Continue(())
    };
    let (mut extensions, _) = start_extensions(args, &run, terminal_size, install_failed, previous)
        .await
        .map_err(anyhow::Error::msg)?;
    extensions.issues.splice(0..0, uninstalled);
    Ok((extensions, run))
}

/// Installs missing packages, reporting failures to `install_failed` with
/// their sources, and starts the extensions found in `run`'s directory with
/// the command line's flag values. The hosts of `previous` that still have
/// extensions run their factories again instead, keeping the modules they
/// imported. Returns the problems with the flags too; an error means no
/// extension could start.
async fn start_extensions(
    args: &Args,
    run: &RunSettings,
    terminal_size: Option<(usize, usize)>,
    install_failed: impl FnMut(&str, String) -> ControlFlow<()> + Send,
    previous: Option<&Extensions>,
) -> Result<(Extensions, Vec<String>), String> {
    let cwd = run.cwd.clone();
    // pi resolves `-e` paths against the directory yapi started in.
    let launch = std::env::current_dir().unwrap_or_else(|_| cwd.clone());
    let agent_dir = agent_dir();
    let mut packages = yapi_core::packages::PackageManager::new(
        run.settings.project_dir().to_path_buf(),
        agent_dir.clone(),
        run.settings.clone(),
        yapi_core::packages::npm::config(),
    );
    let offline = yapi_core::tools::external::offline();
    // Installs missing packages; what they provide comes from pi's resolver.
    // Unlike pi, a failure does not stop the others (docs/compat.md).
    if !offline {
        let _ = packages.install_missing(install_failed).await;
    }
    let mut issues = Vec::new();
    let mut missing = Vec::new();
    let mut requested = Vec::new();
    for path in &args.extensions {
        if let Some(name) = path.strip_prefix(BUILTIN_PREFIX) {
            if !BUILTINS.contains(&name) {
                missing.push((path.clone(), format!("Unknown built-in extension: {path}")));
            }
            continue;
        }
        if !yapi_core::packages::source::is_local(path) {
            // pi installs npm and git sources as temporary packages, and
            // skips a missing one when offline.
            match packages.install_temporary(path, offline).await {
                Ok(Some(root)) => requested.push((root, false)),
                Ok(None) => {}
                Err(err) => return Err(format!("Failed to install {path}: {err}")),
            }
            continue;
        }
        let resolved = resolve_to_cwd(path, &launch);
        if resolved.exists() {
            requested.push((resolved, true));
        } else {
            let resolved = resolved.display().to_string();
            let error = format!("Extension path does not exist: {resolved}");
            missing.push((resolved, error));
        }
    }
    let mut themes = Vec::new();
    let mut skills = Vec::new();
    let mut prompts = Vec::new();
    // pi resolves `-e` entries as temporary packages: a file is an extension,
    // and a directory brings its manifest's or conventional resources.
    let mut sources: Vec<SourceInfo> = Vec::new();
    for (root, local) in &requested {
        let found = yapi_core::packages::resolve::package_resources(root, None, *local);
        let cli = |info: &SourceInfo| yapi_core::resources::cli_source(Path::new(&info.path));
        sources.extend(found.enabled(ResourceType::Extensions).map(cli));
        skills.extend(found.enabled(ResourceType::Skills).map(cli));
        prompts.extend(found.enabled(ResourceType::Prompts).map(cli));
        themes.extend(found.enabled(ResourceType::Themes).map(cli));
    }
    // pi's precedence: the project's settings entries and discovered
    // extensions, then the user's, then packages. `--no-extensions` leaves
    // them out; packages still provide their skills, prompts and themes.
    let resolved = yapi_core::packages::resolve_resources(
        packages.settings().project_dir(),
        &agent_dir,
        packages.settings(),
        &BUILTINS,
    );
    if !args.no_extensions {
        sources.extend(
            resolved
                .enabled(ResourceType::Extensions)
                .filter(|info| !info.path.starts_with(BUILTIN_PREFIX))
                .cloned(),
        );
    }
    let mut seen = std::collections::HashSet::new();
    sources.retain(|source| seen.insert(source.path.clone()));
    // pi extensions share one JS runtime; each native extension has its own.
    // A host of `previous` is known by the path of its first extension.
    let previous_host = |native: bool, path: Option<&str>| {
        previous?.hosts.iter().find(|host| {
            host.sources().first().is_some_and(|first| {
                is_native(Path::new(&first.path)) == native
                    && path.is_none_or(|path| first.path == path)
            })
        })
    };
    let mut engine = previous.and_then(|previous| previous.engine.clone());
    let mut hosts = Vec::new();
    // The hosts started here, which need the flags.
    let mut started = Vec::new();
    if !sources.is_empty() {
        let cache = agent_dir.join("cache");
        let engine = match &mut engine {
            Some(engine) => engine.clone(),
            None => engine
                .insert(Engine::new(Some(&cache.join("wasm"))).map_err(|err| err.to_string())?)
                .clone(),
        };
        let mut options = yapi_ext::Options::new(cwd.clone());
        options.project_dir = run.settings.project_dir().to_path_buf();
        options.agent_dir = agent_dir.clone();
        options.cache_dir = Some(cache.join("js"));
        options.terminal_size = terminal_size;
        let (native, js): (Vec<SourceInfo>, Vec<SourceInfo>) = sources
            .into_iter()
            .partition(|source| is_native(Path::new(&source.path)));
        if !js.is_empty() {
            let host = match previous_host(false, None) {
                Some(host) => {
                    host.reload(&js).await;
                    Arc::clone(host)
                }
                None => {
                    let host = ExtensionHost::load(&engine, options.clone(), &js)
                        .await
                        .map_err(|err| err.to_string())?;
                    started.push(Arc::clone(&host));
                    host
                }
            };
            hosts.push(host);
        }
        for source in &native {
            if let Some(host) = previous_host(true, Some(&source.path)) {
                host.reload(std::slice::from_ref(source)).await;
                hosts.push(Arc::clone(host));
                continue;
            }
            match ExtensionHost::load_native(&engine, options.clone(), source).await {
                Ok(host) => {
                    started.push(Arc::clone(&host));
                    hosts.push(host);
                }
                Err(err) => issues.push((source.path.clone(), err.to_string())),
            }
        }
    }
    issues.append(&mut missing);
    issues.extend(conflicts(&hosts));

    // pi's applyExtensionFlagValues.
    let flags: Vec<yapi_ext::Flag> = hosts.iter().flat_map(|host| host.flags()).collect();
    let mut values = serde_json::Map::new();
    let mut flag_errors = Vec::new();
    let mut unknown = Vec::new();
    for (name, value) in &args.unknown_flags {
        let Some(flag) = flags.iter().find(|flag| flag.name == *name) else {
            unknown.push(format!("--{name}"));
            continue;
        };
        match (flag.takes_value, value) {
            (false, _) => {
                values.insert(name.clone(), serde_json::Value::Bool(true));
            }
            (true, FlagValue::Value(text)) => {
                values.insert(name.clone(), serde_json::Value::String(text.clone()));
            }
            (true, FlagValue::Present) => {
                flag_errors.push(format!("Extension flag \"--{name}\" requires a value"));
            }
        }
    }
    if !unknown.is_empty() {
        let plural = if unknown.len() == 1 { "" } else { "s" };
        flag_errors.push(format!("Unknown option{plural}: {}", unknown.join(", ")));
    }
    if !values.is_empty() {
        for host in &started {
            host.set_flags(values.clone())
                .await
                .map_err(|err| err.to_string())?;
        }
    }
    let extensions = Extensions {
        warnings: replaced_builtin(&hosts, args, &run.settings)
            .into_iter()
            .collect(),
        hosts,
        issues,
        skills,
        prompts,
        themes,
        resolved,
        cwd,
        engine,
    };
    Ok((extensions, flag_errors))
}

/// pi's extension conflicts: each tool or flag that an extension registers
/// after an earlier extension, in load order, registered the same name, by
/// the later extension's path.
fn conflicts(hosts: &[Arc<ExtensionHost>]) -> Vec<(String, String)> {
    let mut owners = std::collections::HashMap::<(&str, String), String>::new();
    let mut messages = Vec::new();
    for extension in hosts.iter().flat_map(|host| host.registrations()) {
        let path = extension["path"].as_str().unwrap_or_default();
        for (kind, label) in [("tools", "Tool \""), ("flags", "Flag \"--")] {
            let names = extension[kind]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| item["name"].as_str());
            for name in names {
                match owners.get(&(kind, name.to_owned())) {
                    Some(owner) if owner != path => messages.push((
                        path.to_owned(),
                        format!("{label}{name}\" conflicts with {owner}"),
                    )),
                    Some(_) => {}
                    None => {
                        owners.insert((kind, name.to_owned()), path.to_owned());
                    }
                }
            }
        }
    }
    messages
}

/// pi's `omitReplacedExtensions` for codemode, the built-in that yields to
/// an extension's tool of its name: the built-in's path and the warning, when
/// an extension registers `codemode` while the built-in is enabled.
fn replaced_builtin(
    hosts: &[Arc<ExtensionHost>],
    args: &Args,
    settings: &SettingsManager,
) -> Option<(String, String)> {
    let name = yapi_core::extensions::codemode::NAME;
    if !builtin_enabled(name, args, &extension_settings(settings)) {
        return None;
    }
    let owner = hosts
        .iter()
        .flat_map(|host| host.registrations())
        .find(|extension| {
            let mut tools = extension["tools"].as_array().into_iter().flatten();
            tools.any(|tool| tool["name"] == name)
        })?;
    let path = owner["path"].as_str().unwrap_or_default();
    Some((
        format!("{BUILTIN_PREFIX}{name}"),
        format!(
            "Extension {path} registers tool `{name}`, so built-in extension `{name}` was not loaded. To use `{name}`, run `yapi config` and make sure it is enabled under Built-in extensions, then disable or remove the existing extension. We recommend only having one or the other loaded at a time."
        ),
    ))
}

/// pi's startup diagnostic for an extension warning.
pub fn extension_warning(path: &str, warning: &str) -> String {
    format!("Extension package \"{path}\": {warning}")
}

/// The run's loaded extensions, and the skills, prompt templates and themes
/// its packages provide.
#[derive(Default)]
pub struct Extensions {
    /// Instances with loaded extensions.
    pub hosts: Vec<Arc<ExtensionHost>>,
    /// Warnings about extensions that were not loaded, by path.
    pub warnings: Vec<(String, String)>,
    /// Why the extensions without a host did not load and the conflicts
    /// between those that did, by path, with pi's messages.
    issues: Vec<(String, String)>,
    /// Package skills with their packages as sources.
    pub skills: Vec<SourceInfo>,
    /// Package prompt templates with their packages as sources.
    pub prompts: Vec<SourceInfo>,
    /// Package themes with their packages as sources.
    pub themes: Vec<SourceInfo>,
    /// What the settings of [`Extensions::cwd`] and the files on disk gave
    /// as the extensions were found.
    resolved: yapi_core::packages::resolve::ResolvedPaths,
    /// The working directory the extensions were found for.
    cwd: PathBuf,
    /// The engine of the hosts, which later hosts share.
    engine: Option<Engine>,
}

impl Extensions {
    /// The extensions that failed to load in the latest run of each host's
    /// factories, then the [`Extensions::issues`].
    fn failures(&self) -> Vec<(String, String)> {
        self.hosts
            .iter()
            .flat_map(|host| host.errors())
            .map(|error| (error.path.to_string_lossy().into_owned(), error.error))
            .chain(self.issues.iter().cloned())
            .collect()
    }
}

#[cfg(test)]
mod builtin_tests {
    use super::*;

    fn args(extensions: &[&str], no_extensions: bool) -> Args {
        Args {
            extensions: extensions.iter().map(|path| (*path).to_owned()).collect(),
            no_extensions,
            ..Args::default()
        }
    }

    fn entries(list: &[&str]) -> Vec<String> {
        list.iter().map(|entry| (*entry).to_owned()).collect()
    }

    #[test]
    fn builtins_follow_flags_settings() {
        let none = [Vec::new(), Vec::new()];
        assert!(builtin_enabled("mcp", &args(&[], false), &none));
        assert!(!builtin_enabled("mcp", &args(&[], true), &none));
        assert!(builtin_enabled("mcp", &args(&["builtin:mcp"], true), &none));
        assert!(!builtin_enabled(
            "codemode",
            &args(&["builtin:mcp"], true),
            &none
        ));

        let user = |list: &[&str]| [Vec::new(), entries(list)];
        assert!(!builtin_enabled(
            "mcp",
            &args(&[], false),
            &user(&["-builtin:mcp"])
        ));
        assert!(!builtin_enabled(
            "mcp",
            &args(&[], false),
            &user(&["!builtin:*"])
        ));
        assert!(builtin_enabled(
            "codemode",
            &args(&[], false),
            &user(&["-builtin:mcp"])
        ));
        assert!(builtin_enabled(
            "mcp",
            &args(&[], false),
            &user(&["!builtin:*", "+builtin:mcp"])
        ));
        assert!(!builtin_enabled(
            "mcp",
            &args(&[], false),
            &user(&["-builtin:mcp", "+builtin:mcp"])
        ));

        let both = [
            entries(&["-builtin:mcp", "+builtin:mcp"]),
            entries(&["-builtin:mcp"]),
        ];
        assert!(builtin_enabled("mcp", &args(&[], false), &both));
        let project_off = [entries(&["!builtin:m*"]), Vec::new()];
        assert!(!builtin_enabled("mcp", &args(&[], false), &project_off));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A settings manager whose global settings are `json`, in `dir`.
    fn settings(dir: &Path, json: &str) -> SettingsManager {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("settings.json"), json).unwrap();
        SettingsManager::load(dir, dir, false)
    }

    #[test]
    fn empty_session_dir() {
        let dir = std::env::temp_dir().join(format!("yapi-session-dir-{}", std::process::id()));
        let settings = settings(&dir, r#"{"sessionDir": ""}"#);
        let args = Args {
            session_dir: Some(String::new()),
            ..Args::default()
        };
        assert_eq!(custom_session_dir(&args, &settings, &dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_flags_replace_file() {
        let discovered = || Some("GLOBAL APPEND".to_owned());
        assert_eq!(append_prompts(&[], discovered), ["GLOBAL APPEND"]);
        let flags = ["literal".to_owned(), String::new()];
        assert_eq!(append_prompts(&flags, discovered), ["literal"]);
        assert!(append_prompts(&[String::new()], discovered).is_empty());
    }
}
