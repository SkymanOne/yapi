//! `yapi import pi`: copies pi's state into yapi's directories. yapi reads pi's
//! formats unchanged, so files are copied as they are, except pi's
//! `lastChangelogVersion` and sign-ins; files yapi already has are kept.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use yapi_types::config::ConfigFile;

/// The setting in which pi and yapi each record their own version.
const CHANGELOG_VERSION: &str = "lastChangelogVersion";

/// The agent directory entries yapi reads, in the order they are copied.
pub const AGENT_ENTRIES: &[&str] = &[
    "settings.json",
    "auth.json",
    "models.json",
    "keybindings.json",
    "mcp.json",
    "mcp-auth.json",
    "trust.json",
    "SYSTEM.md",
    "APPEND_SYSTEM.md",
    "AGENTS.override.md",
    "AGENTS.md",
    "AGENTS.MD",
    "CLAUDE.md",
    "CLAUDE.MD",
    "sessions",
    "prompts",
    "skills",
    "themes",
    "extensions",
    "npm",
    "git",
];

/// The project directory (`.pi`) entries yapi reads, in the order they are
/// copied.
pub const PROJECT_ENTRIES: &[&str] = &[
    "settings.json",
    "mcp.json",
    "SYSTEM.md",
    "APPEND_SYSTEM.md",
    "prompts",
    "skills",
    "themes",
    "extensions",
    "npm",
    "git",
];

/// What happened to one entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    /// The entry's name.
    pub entry: String,
    /// Files copied.
    pub copied: usize,
    /// Files yapi already had, left as they are.
    pub kept: usize,
    /// Sign-ins left out of a copied `auth.json` (providers) or
    /// `mcp-auth.json` (MCP servers). Servers may rotate refresh tokens, so
    /// copies in pi and yapi would sign each other out.
    pub signed_out: Vec<String>,
}

/// pi's agent directory: `PI_CODING_AGENT_DIR`, or `~/.pi/agent`.
pub fn pi_agent_dir() -> PathBuf {
    match std::env::var("PI_CODING_AGENT_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(crate::tools::path::expand(&dir)),
        _ => crate::tools::path::home_dir().join(".pi").join("agent"),
    }
}

/// An I/O error naming the path it concerns.
fn at(path: &Path, error: std::io::Error) -> std::io::Error {
    std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// Copies `from` to `to`, following symbolic links. Links that lead nowhere,
/// loop, or lead back into a directory being copied are skipped.
fn copy_tree(
    from: &Path,
    to: &Path,
    imported: &mut Imported,
    ancestors: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let metadata = match std::fs::metadata(from) {
        Ok(metadata) => metadata,
        Err(_) if from.symlink_metadata().is_ok_and(|link| link.is_symlink()) => return Ok(()),
        Err(error) => return Err(at(from, error)),
    };
    if metadata.is_dir() {
        let real = std::fs::canonicalize(from).map_err(|error| at(from, error))?;
        if ancestors.contains(&real) {
            return Ok(());
        }
        std::fs::create_dir_all(to).map_err(|error| at(to, error))?;
        let mut entries: Vec<_> = std::fs::read_dir(from)
            .map_err(|error| at(from, error))?
            .flatten()
            .collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        ancestors.push(real);
        for entry in entries {
            copy_tree(
                &entry.path(),
                &to.join(entry.file_name()),
                imported,
                ancestors,
            )?;
        }
        ancestors.pop();
        return Ok(());
    }
    if to.exists() {
        imported.kept += 1;
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|error| at(parent, error))?;
    }
    // `copy` keeps the permissions, so credentials stay private.
    std::fs::copy(from, to).map_err(|error| at(from, error))?;
    imported.copied += 1;
    Ok(())
}

/// The settings or credentials document at `path`, if it is a JSON object.
fn settings(path: &Path) -> Option<Map<String, Value>> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Removes the sign-ins from the copied credentials `file` at `path`: OAuth
/// credentials of `auth.json`, servers holding tokens in `mcp-auth.json`.
/// Returns their providers or server names; a file without any is untouched.
fn drop_sign_ins(path: &Path, file: ConfigFile) -> std::io::Result<Vec<String>> {
    let Some(mut document) = settings(path) else {
        return Ok(Vec::new());
    };
    let signed_in = |state: &Value| match file {
        ConfigFile::Auth => state["type"] == "oauth",
        _ => state.get("tokens").is_some(),
    };
    let names: Vec<String> = document
        .iter()
        .filter(|(_, state)| signed_in(state))
        // MCP keys are `mcp__<server>|<url>`, or the URL alone in older files.
        .map(|(key, _)| {
            match key
                .strip_prefix("mcp__")
                .and_then(|rest| rest.split_once('|'))
            {
                Some((server, _)) => server.to_owned(),
                None => key.clone(),
            }
        })
        .collect();
    if !names.is_empty() {
        document.retain(|_, state| !signed_in(state));
        let text = file.render(&document).map_err(std::io::Error::other)?;
        yapi_ai::auth::lock::write_atomic(path, text.as_bytes(), 0o600)
            .map_err(|error| at(path, error))?;
    }
    Ok(names)
}

/// Copies `entries` of directory `from` into `to`, keeping files `to`
/// already has. Entries `from` lacks are skipped. A `settings.json` that
/// holds only the version yapi recorded on its first start is replaced, and
/// pi's `lastChangelogVersion`, a pi version, is not copied. Nor are the
/// sign-ins of a copied `auth.json` or `mcp-auth.json`; see
/// [`Imported::signed_out`].
pub fn import(from: &Path, to: &Path, entries: &[&str]) -> std::io::Result<Vec<Imported>> {
    let mut out = Vec::new();
    for entry in entries {
        let source = from.join(entry);
        if source.symlink_metadata().is_err() {
            continue;
        }
        let mut imported = Imported {
            entry: (*entry).to_owned(),
            ..Imported::default()
        };
        let target = to.join(entry);
        let is_settings = *entry == ConfigFile::Settings.file_name();
        if is_settings
            && settings(&target).is_some_and(|document| {
                document.len() == 1 && document.contains_key(CHANGELOG_VERSION)
            })
        {
            std::fs::remove_file(&target).map_err(|error| at(&target, error))?;
        }
        copy_tree(&source, &target, &mut imported, &mut Vec::new())?;
        if is_settings
            && imported.copied == 1
            && let Some(mut document) = settings(&target)
            && document.shift_remove(CHANGELOG_VERSION).is_some()
        {
            let text = ConfigFile::Settings
                .render(&document)
                .map_err(std::io::Error::other)?;
            yapi_ai::auth::lock::write_atomic(&target, text.as_bytes(), 0o666)
                .map_err(|error| at(&target, error))?;
        }
        let credentials = [ConfigFile::Auth, ConfigFile::McpAuth]
            .into_iter()
            .find(|file| *entry == file.file_name());
        if let Some(file) = credentials
            && imported.copied == 1
        {
            imported.signed_out = drop_sign_ins(&target, file)?;
        }
        out.push(imported);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, name: &str, text: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn copies_state_keeps_existing() {
        let dir = std::env::temp_dir().join(format!("yapi-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (pi, yapi) = (dir.join("pi"), dir.join("yapi"));
        write(&pi, "settings.json", "{\"theme\":\"light\"}\n");
        write(&pi, "auth.json", "{}\n");
        write(&pi, "mcp-auth.json", "{}\n");
        write(&pi, "sessions/--a--/one.jsonl", "{}\n");
        write(&pi, "sessions/--a--/two.jsonl", "{}\n");
        write(&pi, "bin/fd", "binary");
        write(&pi, "pi-debug.log", "log");
        write(&yapi, "auth.json", "{\"mine\":true}\n");

        let imported = import(&pi, &yapi, AGENT_ENTRIES).unwrap();

        let summary: Vec<(&str, usize, usize)> = imported
            .iter()
            .map(|entry| (entry.entry.as_str(), entry.copied, entry.kept))
            .collect();
        assert_eq!(
            summary,
            [
                ("settings.json", 1, 0),
                ("auth.json", 0, 1),
                ("mcp-auth.json", 1, 0),
                ("sessions", 2, 0)
            ]
        );
        assert_eq!(
            std::fs::read_to_string(yapi.join("auth.json")).unwrap(),
            "{\"mine\":true}\n"
        );
        assert!(yapi.join("sessions/--a--/two.jsonl").is_file());
        assert!(!yapi.join("bin").exists());
        assert!(!yapi.join("pi-debug.log").exists());
    }

    #[test]
    fn skips_sign_ins() {
        let dir = std::env::temp_dir().join(format!("yapi-import-auth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (pi, yapi) = (dir.join("pi"), dir.join("yapi"));
        let oauth =
            r#""anthropic": {"type": "oauth", "refresh": "r", "access": "a", "expires": 1}"#;
        let api_key = "\"openai\": {\n    \"type\": \"api_key\",\n    \"key\": \"sk-1\"\n  }";
        write(
            &pi,
            "auth.json",
            &format!("{{\n  {oauth},\n  {api_key}\n}}"),
        );
        let url = "https://mcp.example.com/mcp";
        let states = serde_json::json!({
            format!("mcp__work|{url}"): {"serverUrl": url, "tokens": {"access_token": "t"}},
            url: {"tokens": {}},
            format!("mcp__new|{url}"): {"codeVerifier": "v"},
        });
        write(&pi, "mcp-auth.json", &states.to_string());

        let imported = import(&pi, &yapi, AGENT_ENTRIES).unwrap();

        let signed_out: Vec<(&str, Vec<&str>)> = imported
            .iter()
            .map(|entry| {
                let names = entry.signed_out.iter().map(String::as_str).collect();
                (entry.entry.as_str(), names)
            })
            .collect();
        assert_eq!(
            signed_out,
            [
                ("auth.json", vec!["anthropic"]),
                ("mcp-auth.json", vec!["work", url])
            ]
        );
        assert_eq!(
            std::fs::read_to_string(yapi.join("auth.json")).unwrap(),
            format!("{{\n  {api_key}\n}}")
        );
        let states = std::fs::read_to_string(yapi.join("mcp-auth.json")).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&states).unwrap(),
            serde_json::json!({format!("mcp__new|{url}"): {"codeVerifier": "v"}})
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_replace_first_start() {
        let dir = std::env::temp_dir().join(format!("yapi-import-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (pi, yapi) = (dir.join("pi"), dir.join("yapi"));
        let pi_settings = "{\n  \"theme\": \"light\",\n  \"lastChangelogVersion\": \"1.0.0\",\n  \"defaultProvider\": \"anthropic\"\n}";
        write(&pi, "settings.json", pi_settings);
        // What yapi's first start writes.
        write(
            &yapi,
            "settings.json",
            "{\n  \"lastChangelogVersion\": \"0.1.0\"\n}",
        );

        let imported = import(&pi, &yapi, &["settings.json"]).unwrap();
        assert_eq!((imported[0].copied, imported[0].kept), (1, 0));
        assert_eq!(
            std::fs::read_to_string(yapi.join("settings.json")).unwrap(),
            "{\n  \"theme\": \"light\",\n  \"defaultProvider\": \"anthropic\"\n}"
        );

        // Settings with anything else are kept as before.
        let kept = dir.join("kept");
        write(
            &kept,
            "settings.json",
            "{\"lastChangelogVersion\":\"0.1.0\",\"theme\":\"dark\"}",
        );
        let imported = import(&pi, &kept, &["settings.json"]).unwrap();
        assert_eq!((imported[0].copied, imported[0].kept), (0, 1));
        assert_eq!(
            std::fs::read_to_string(kept.join("settings.json")).unwrap(),
            "{\"lastChangelogVersion\":\"0.1.0\",\"theme\":\"dark\"}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn context_files_and_symlinks() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join(format!("yapi-import-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (pi, yapi) = (dir.join("pi"), dir.join("yapi"));
        write(&pi, "AGENTS.md", "global rules\n");
        write(&pi, "skills/real/SKILL.md", "skill\n");
        symlink(pi.join("nowhere"), pi.join("skills/dangling")).unwrap();
        symlink(pi.join("skills/loop-b"), pi.join("skills/loop-a")).unwrap();
        symlink(pi.join("skills/loop-a"), pi.join("skills/loop-b")).unwrap();
        symlink(pi.join("skills"), pi.join("skills/real/up")).unwrap();
        symlink(pi.join("skills/real"), pi.join("skills/alias")).unwrap();

        let imported = import(&pi, &yapi, AGENT_ENTRIES).unwrap();

        // On a case-insensitive file system `AGENTS.MD` names the same file
        // and is kept.
        let summary: Vec<(&str, usize)> = imported
            .iter()
            .filter(|entry| entry.copied > 0)
            .map(|entry| (entry.entry.as_str(), entry.copied))
            .collect();
        assert_eq!(summary, [("AGENTS.md", 1), ("skills", 2)]);
        assert!(yapi.join("skills/alias/SKILL.md").is_file());
        assert!(!yapi.join("skills/dangling").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
