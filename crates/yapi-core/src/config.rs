//! Where yapi keeps its state. Same layout as pi, under yapi's own directories.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::tools::path::{expand, home_dir};

/// Application name in messages and paths.
pub const APP_NAME: &str = "yapi";
/// Project configuration directory name.
pub const PROJECT_DIR: &str = ".yapi";
/// Environment variable overriding the agent directory.
pub const AGENT_DIR_ENV: &str = "YAPI_CODING_AGENT_DIR";
/// Environment variable overriding the session directory; `--session-dir` wins.
pub const SESSION_DIR_ENV: &str = "YAPI_CODING_AGENT_SESSION_DIR";

/// Set once offline mode is on for the process.
static OFFLINE: AtomicBool = AtomicBool::new(false);

/// The variables pi sets on its own process in offline mode.
const OFFLINE_ENV: [&str; 2] = ["PI_OFFLINE", "PI_SKIP_VERSION_CHECK"];

/// Turns on offline mode for the process, as pi does for `--offline` or a
/// true `PI_OFFLINE`: from then on `PI_OFFLINE` and `PI_SKIP_VERSION_CHECK`
/// read as `1` here and in every child's environment.
pub fn set_offline() {
    OFFLINE.store(true, Ordering::Relaxed);
}

/// Environment variable `name`, with the values offline mode sets.
pub fn env_var(name: &str) -> Option<String> {
    if OFFLINE.load(Ordering::Relaxed) && OFFLINE_ENV.contains(&name) {
        return Some("1".to_owned());
    }
    std::env::var(name).ok()
}

/// Whether environment variable `name` is set and not empty, as pi tests
/// `process.env.PI_OFFLINE` and `PI_SKIP_VERSION_CHECK`.
pub fn env_set(name: &str) -> bool {
    env_var(name).is_some_and(|value| !value.is_empty())
}

/// Variables every child process gets, as pi sets them on its own process:
/// the agent markers `PI_CODING_AGENT=true` and `AI_AGENT=yapi`, the offline
/// variables in offline mode, and the proxy from the `httpProxy` setting
/// where the environment has none.
pub fn child_env() -> Vec<(&'static str, String)> {
    let mut env = vec![
        ("PI_CODING_AGENT", "true".to_owned()),
        ("AI_AGENT", APP_NAME.to_owned()),
    ];
    if OFFLINE.load(Ordering::Relaxed) {
        env.extend(OFFLINE_ENV.map(|name| (name, "1".to_owned())));
    }
    env.extend(yapi_ai::http::proxy_env());
    env
}

/// The agent directory: `YAPI_CODING_AGENT_DIR`, else `~/.yapi/agent`.
pub fn agent_dir() -> PathBuf {
    match std::env::var(AGENT_DIR_ENV) {
        Ok(dir) if !dir.is_empty() => PathBuf::from(expand(&dir)),
        _ => home_dir().join(PROJECT_DIR).join("agent"),
    }
}

/// Where sessions for `cwd` live by default:
/// `<agent>/sessions/--<cwd with separators as dashes>--`, both resolved
/// against the process directory.
pub fn default_session_dir(agent_dir: &Path, cwd: &Path) -> PathBuf {
    let base = std::env::current_dir().unwrap_or_default();
    let resolve = |path: &Path| crate::tools::path::resolve_lexically(&base, path);
    let cwd = resolve(cwd);
    let text = cwd.to_string_lossy();
    let trimmed = text.trim_start_matches(['/', '\\']);
    let safe: String = trimmed
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') {
                '-'
            } else {
                c
            }
        })
        .collect();
    resolve(agent_dir)
        .join("sessions")
        .join(format!("--{safe}--"))
}

/// Whether environment variable `name` is set to `1`, `true` or `yes`, as pi
/// reads `PI_OFFLINE`.
pub fn env_flag(name: &str) -> bool {
    env_var(name).is_some_and(|value| {
        value == "1" || matches!(value.to_lowercase().as_str(), "true" | "yes")
    })
}

/// The documentation the model reads: yapi's pages, with pi's README, docs
/// and examples under `pi/`. The installer or yapi's first run puts it here.
pub fn docs_dir(agent_dir: &Path) -> PathBuf {
    agent_dir.join("docs")
}

/// Executables the agent installs for its tools, prepended to `PATH` for commands.
pub fn bin_dir(agent_dir: &Path) -> PathBuf {
    agent_dir.join("bin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_dirs_match_pi() {
        assert_eq!(
            default_session_dir(Path::new("/h/.yapi/agent"), Path::new("/home/user/project")),
            PathBuf::from("/h/.yapi/agent/sessions/--home-user-project--")
        );
    }
}
