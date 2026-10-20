//! Bug reports for `/bug`: what a report holds, its redaction, the zip
//! archive, the summary the current model writes and the GitHub issue.
//!
//! A port of `core/bug-report.ts` in pi `v1.0.0`. pi uploads the report to
//! its own service; yapi always writes the zip and can post a public issue
//! with the GitHub CLI.

use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use flate2::Compression;
use flate2::write::DeflateEncoder;
use serde_json::{Value, json};
use yapi_ai::registry::ModelRegistry;
use yapi_types::message::{Message, StopReason, blocks_text};
use yapi_types::model::Model;
use yapi_types::session::FileEntry;

use crate::agent_session::AgentSession;
use crate::compaction::{Summarizer, check_summary, estimate_tokens, serialize_conversation};
use crate::messages::convert_to_llm;
use crate::session::SessionManager;
use crate::settings::Scope;
use crate::time::{now_iso, now_ms, uuid_v7};

/// The type of the session entry that records a report.
pub const ENTRY_TYPE: &str = "yapi.bug-report";
/// The repository reports go to.
pub const REPO: &str = "SkymanOne/yapi";
/// Where a report can be filed by hand.
pub const ISSUES_URL: &str = "https://github.com/SkymanOne/yapi/issues";

const SCHEMA_VERSION: u32 = 1;
const REDACTED: &str = "<redacted>";
/// Words that make a key name a credential, besides an API key.
const SENSITIVE_WORDS: [&str; 7] = [
    "secret",
    "token",
    "password",
    "passwd",
    "credential",
    "authorization",
    "cookie",
];
/// GitHub's limit on an issue body, in characters.
const BODY_LIMIT: usize = 65_536;
/// GitHub's limit on an issue title, in characters.
const TITLE_LIMIT: usize = 256;

/// pi's `isSensitiveKey`: whether `key`, split into words at `-`, `_` and
/// camel case, holds an API key, secret, token, password, credential,
/// authorization or cookie word.
fn is_sensitive_key(key: &str) -> bool {
    let mut split = String::new();
    let mut previous: Option<char> = None;
    for c in key.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            split.push('_');
        }
        split.push(c.to_ascii_lowercase());
        previous = Some(c);
    }
    let words: Vec<&str> = split.split(['-', '_']).collect();
    words.iter().enumerate().any(|(index, word)| {
        SENSITIVE_WORDS.contains(word)
            || *word == "apikey"
            || (*word == "api" && words.get(index + 1) == Some(&"key"))
    })
}

/// The length of the scheme and colon `text` starts with, if it does: a
/// letter, then letters, digits, `+`, `.` or `-`.
fn scheme_len(text: &str) -> Option<usize> {
    let end = text.find(':')?;
    let mut chars = text[..end].chars();
    let valid = chars.next()?.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'));
    valid.then_some(end + 1)
}

/// pi's `redactUrl`: `value` without the user info and the values of
/// secret-looking query parameters of the URL it is; unchanged when it is
/// no URL or has neither.
fn redact_url(value: &str) -> String {
    // A package source such as `git:https://...` wraps a URL in a scheme.
    if let Some(prefix) = scheme_len(value) {
        let rest = &value[prefix..];
        if scheme_len(rest).is_some_and(|end| rest[end..].starts_with("//")) {
            return format!("{}{}", &value[..prefix], redact_url(rest));
        }
    }
    let Ok(mut url) = url::Url::parse(value) else {
        return value.to_owned();
    };
    let mut changed = false;
    if !url.username().is_empty() || url.password().is_some() {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        changed = true;
    }
    if url.query_pairs().any(|(key, _)| is_sensitive_key(&key)) {
        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(key, value)| {
                let value = if is_sensitive_key(&key) {
                    REDACTED.to_owned()
                } else {
                    value.into_owned()
                };
                (key.into_owned(), value)
            })
            .collect();
        url.query_pairs_mut().clear().extend_pairs(pairs);
        changed = true;
    }
    if changed {
        url.into()
    } else {
        value.to_owned()
    }
}

/// pi's `redactJsonValue`, in place: the values of credential keys become
/// `<redacted>`, and every string goes through [`redact_url`].
fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if !child.is_null() && is_sensitive_key(key) {
                    *child = Value::String(REDACTED.to_owned());
                } else {
                    redact(child);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        Value::String(text) => *text = redact_url(text),
        _ => {}
    }
}

/// The names pi's `process.platform` and `process.arch` give the build's
/// operating system and processor.
fn node_platform() -> (&'static str, &'static str) {
    let platform = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        os => os,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        arch => arch,
    };
    (platform, arch)
}

/// Node's `os.release()` and `os.version()`.
#[cfg(unix)]
fn os_release() -> (Option<String>, Option<String>) {
    let name = rustix::system::uname();
    let text = |value: &std::ffi::CStr| Some(value.to_string_lossy().into_owned());
    (text(name.release()), text(name.version()))
}

#[cfg(not(unix))]
fn os_release() -> (Option<String>, Option<String>) {
    (None, None)
}

/// pi's `collectEnvironment` for yapi.
fn environment() -> Value {
    let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    let mut names: Vec<String> = std::env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter(|name| name.starts_with("PI_") || name.starts_with("YAPI_"))
        .collect();
    names.sort();
    let version = env!("CARGO_PKG_VERSION");
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let (platform, arch) = node_platform();
    let (os_release, os_version) = os_release();
    let ssh = ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
        .into_iter()
        .any(|name| env(name).is_some());
    json!({
        "version": version,
        "userAgent": yapi_ai::http::user_agent(),
        "runtime": format!("yapi/{version} ({build})"),
        "platform": platform,
        "arch": arch,
        "osRelease": os_release,
        "osVersion": os_version,
        "shell": env("SHELL").and_then(|shell| {
            shell.rsplit(['/', '\\']).next().filter(|name| !name.is_empty()).map(str::to_owned)
        }),
        "terminal": {
            "term": env("TERM"),
            "program": env("TERM_PROGRAM"),
            "programVersion": env("TERM_PROGRAM_VERSION"),
            "colorterm": env("COLORTERM"),
            "tmux": env("TMUX").is_some(),
            "ssh": ssh,
            "ci": env("CI").is_some(),
        },
        // Names help diagnose configuration; values never leave the machine.
        "piEnvironmentVariables": names,
    })
}

fn sorted_names<'a>(headers: impl IntoIterator<Item = &'a String>) -> Vec<&'a String> {
    let mut names: Vec<&String> = headers.into_iter().collect();
    names.sort();
    names
}

fn describe_model(model: &Model) -> Value {
    json!({
        "provider": model.provider,
        "id": model.id,
        "name": model.name,
        "api": model.api,
        "baseUrl": model.base_url,
        "reasoning": model.reasoning,
        "input": model.input,
        "contextWindow": model.context_window,
        "maxTokens": model.max_tokens,
        "samplingParams": model.sampling_params,
        "compat": model.compat,
        "thinkingLevelMap": model.thinking_level_map,
        "headerNames": sorted_names(model.headers.iter().flat_map(|headers| headers.keys())),
    })
}

fn describe_provider(registry: &ModelRegistry, id: &str) -> Value {
    let config = registry.provider_config(id);
    let mut auth_types = Vec::new();
    if yapi_ai::providers::info(id).is_none_or(|info| info.api_key.is_some()) {
        auth_types.push("api_key");
    }
    if registry.oauth_flow(id).is_some() {
        auth_types.push("oauth");
    }
    let auth_status = match registry.login_status(id) {
        Some(source) => json!({"configured": true, "source": source}),
        None => json!({"configured": false}),
    };
    json!({
        "id": id,
        "name": registry.provider_name(id),
        "baseUrl": config.and_then(|config| config.base_url.clone()),
        "headerNames": sorted_names(config.and_then(|config| config.headers.as_ref()).into_iter().flat_map(|headers| headers.keys())),
        "authTypes": auth_types,
        "authStatus": auth_status,
        "usingOAuth": registry.is_using_oauth(id),
        "registeredByExtension": registry.registered_by_extension(id),
    })
}

/// A settings document without pi's tracking ids.
fn settings(session: &AgentSession, scope: Scope) -> Value {
    let mut document = session.settings_manager().document(scope).clone();
    document.retain(|key, _| key != "trackingId" && key != "deviceId");
    Value::Object(document)
}

/// pi's `collectBugReportDiagnostics` over the current branch: the failed,
/// aborted and diagnostic assistant turns, without conversation text.
fn diagnostics(session: &SessionManager) -> Value {
    let branch = session.branch_path(None);
    let mut assistant = Vec::new();
    let mut count = 0;
    for entry in &branch {
        let FileEntry::Message(entry) = entry else {
            continue;
        };
        let Message::Assistant(message) = &entry.message else {
            continue;
        };
        count += 1;
        let diagnostics = message.diagnostics.clone().unwrap_or_default();
        let failed = matches!(message.stop_reason, StopReason::Error | StopReason::Aborted)
            || message
                .error_message
                .as_deref()
                .is_some_and(|m| !m.is_empty());
        if diagnostics.is_empty() && !failed {
            continue;
        }
        let mut turn = json!({
            "entryId": entry.meta.id,
            "timestamp": entry.meta.timestamp,
            "provider": message.provider,
            "model": message.model,
            "api": message.api,
            "stopReason": message.stop_reason,
        });
        if let Some(reason) = &message.raw_stop_reason {
            turn["rawStopReason"] = json!(reason);
        }
        if let Some(error) = &message.error_message {
            turn["errorMessage"] = json!(error);
        }
        turn["diagnostics"] = json!(diagnostics);
        assistant.push(turn);
    }
    json!({
        "schemaVersion": SCHEMA_VERSION,
        "sessionId": session.id(),
        "entryCount": branch.len(),
        "assistantMessageCount": count,
        "assistant": assistant,
    })
}

/// A report's files, before they are written.
pub struct Report {
    /// The report id, a UUID v7.
    pub id: String,
    /// `report.json`: the environment, model, provider, extensions and
    /// settings, redacted.
    metadata: Value,
    /// `diagnostics.json`: the failed turns of the branch, redacted.
    diagnostics: Value,
    /// `session.jsonl`: the branch, when the user includes the transcript.
    session: Option<String>,
    /// `summary.md`: the model's summary, when the user asked for one.
    summary: Option<String>,
}

impl Report {
    /// pi's `collectBugReportMetadata` and `collectBugReportDiagnostics`
    /// for `session`, with description `hint`, the transcript when
    /// `include_session`, and `summary`.
    pub fn collect(
        session: &AgentSession,
        hint: &str,
        include_session: bool,
        summary: Option<String>,
    ) -> Report {
        let id = uuid_v7();
        let (session_id, cwd, jsonl, mut diagnostics) = session.with_session(|file| {
            (
                file.id().to_owned(),
                file.cwd().display().to_string(),
                include_session.then(|| file.serialize_branch()),
                diagnostics(file),
            )
        });
        let mut session_info = json!({
            "id": session_id,
            "included": include_session,
            "summaryIncluded": summary.is_some(),
            "messageCount": session.messages().len(),
        });
        if include_session {
            session_info["cwd"] = json!(cwd);
        }
        let model = session.model();
        let registry = session.registry();
        let extensions: Vec<Value> = session
            .extensions()
            .iter()
            .map(|extension| {
                let source = extension.source();
                // pi hides its built-in extensions.
                let hidden = source.source == "builtin";
                json!({
                    "path": source.path,
                    "source": source.source,
                    "scope": source.scope,
                    "origin": source.origin,
                    "hidden": hidden,
                })
            })
            .collect();
        let errors: Vec<Value> = session
            .extension_load_errors()
            .into_iter()
            .map(|(path, error)| json!({"path": path, "error": error}))
            .collect();
        let hint = hint.trim();
        let mut metadata = json!({
            "schemaVersion": SCHEMA_VERSION,
            "id": id,
            "createdAt": now_iso(),
            "hint": (!hint.is_empty()).then_some(hint),
            "environment": environment(),
            "session": session_info,
            "model": model.as_ref().map(describe_model),
            "provider": model.as_ref().map(|model| describe_provider(&registry, &model.provider)),
            "thinkingLevel": session.thinking_level().as_str(),
            "extensions": extensions,
            "extensionErrors": errors,
            "settings": {
                "global": settings(session, Scope::Global),
                "project": settings(session, Scope::Project),
            },
        });
        redact(&mut metadata);
        redact(&mut diagnostics);
        Report {
            id,
            metadata,
            diagnostics,
            session: jsonl,
            summary,
        }
    }

    /// The zip's file name.
    pub fn file_name(&self) -> String {
        format!("yapi-bug-report-{}.zip", self.id)
    }

    /// The description the user gave, after redaction.
    fn hint(&self) -> Option<&str> {
        self.metadata["hint"].as_str()
    }

    /// The zip's files, by name, in pi's order.
    fn files(&self) -> Vec<(&'static str, String)> {
        let mut files = vec![
            ("report.json", pretty(&self.metadata)),
            ("diagnostics.json", pretty(&self.diagnostics)),
        ];
        if let Some(session) = &self.session {
            files.push(("session.jsonl", session.clone()));
        }
        if let Some(summary) = &self.summary {
            let newline = if summary.ends_with('\n') { "" } else { "\n" };
            files.push(("summary.md", format!("{summary}{newline}")));
        }
        files
    }

    /// Writes the zip to `path`.
    pub fn write_zip(&self, path: &Path) -> io::Result<()> {
        std::fs::write(path, zip(&self.files(), dos_time(now_ms()))?)
    }

    /// The issue title: the description's first line, or the report id.
    pub fn title(&self) -> String {
        match self.hint().and_then(|hint| hint.lines().next()) {
            Some(line) => line.chars().take(TITLE_LIMIT).collect(),
            None => format!("yapi bug report {}", self.id),
        }
    }

    /// The issue body: the description, the summary and `report.json`,
    /// within GitHub's limit. The transcript and the diagnostics, which
    /// can quote tool output, stay out.
    pub fn issue_body(&self) -> String {
        let mut head = self.hint().unwrap_or("No description given.").to_owned();
        if let Some(summary) = &self.summary {
            head.push_str(&format!("\n\n## Summary\n\n{summary}"));
        }
        let report = pretty(&self.metadata);
        let longest = report.split(|c| c != '`').map(str::len).max().unwrap_or(0);
        let fence = "`".repeat(longest.max(2) + 1);
        let open = format!("\n\n## Report\n\n{fence}json\n");
        let close = format!("{fence}\n");
        let head = truncate(&head, BODY_LIMIT / 2);
        let room = BODY_LIMIT - head.len() - open.len() - close.len();
        format!("{head}{open}{}{close}", truncate(&report, room))
    }

    /// The `yapi.bug-report` session entry's data for the zip at `path`
    /// and the issue at `issue_url`.
    pub fn entry(&self, path: &Path, issue_url: Option<&str>) -> Value {
        let mut entry = json!({
            "id": self.id,
            "createdAt": self.metadata["createdAt"],
            "hint": self.metadata["hint"],
            "sessionIncluded": self.session.is_some(),
            "summaryIncluded": self.summary.is_some(),
            "delivery": "zip",
            "path": path.display().to_string(),
        });
        if let Some(url) = issue_url {
            entry["issueUrl"] = json!(url);
        }
        entry
    }
}

/// `value` as `JSON.stringify(value, null, 2)` and a newline.
fn pretty(value: &Value) -> String {
    yapi_types::json::to_string_pretty(value, "  ").unwrap_or_default() + "\n"
}

/// `text` cut to at most `max` bytes, marked when cut.
fn truncate(text: &str, max: usize) -> String {
    const MARK: &str = "\n[truncated]\n";
    if text.len() <= max {
        return text.to_owned();
    }
    let end = text.floor_char_boundary(max.saturating_sub(MARK.len()));
    format!("{}{MARK}", &text[..end])
}

/// The local time at `ms` as MS-DOS time and date, as zip headers keep it.
fn dos_time(ms: u64) -> (u16, u16) {
    let seconds = (ms / 1000) as i64;
    let local = seconds + i64::from(crate::zone::local().offset(seconds));
    let iso = crate::time::iso(local.max(0) as u64 * 1000);
    let field = |at: usize, len: usize| {
        iso.get(at..at + len)
            .and_then(|text| text.parse::<u16>().ok())
            .unwrap_or(0)
    };
    let (year, month, day) = (field(0, 4), field(5, 2), field(8, 2));
    let (hour, minute, second) = (field(11, 2), field(14, 2), field(17, 2));
    (
        hour << 11 | minute << 5 | second >> 1,
        (year.max(1980) - 1980) << 9 | month << 5 | day,
    )
}

/// pi's `createZipArchive`: a classic zip of deflated `files` with UTF-8
/// names, stamped `(time, date)`.
// shortcut: no ZIP64, so sizes over 4 GiB wrap; add ZIP64 if a report ever gets that large.
fn zip(files: &[(&str, String)], (time, date): (u16, u16)) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in files {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data.as_bytes())?;
        let compressed = encoder.finish()?;
        let mut crc = flate2::Crc::new();
        crc.update(data.as_bytes());
        let offset = out.len() as u32;
        // From "version needed" to the extra field's length; both headers share it.
        let fields = |header: &mut Vec<u8>| {
            for value in [20, 0x0800, 8, time, date] {
                header.extend(value.to_le_bytes());
            }
            for value in [crc.sum(), compressed.len() as u32, data.len() as u32] {
                header.extend(value.to_le_bytes());
            }
            header.extend((name.len() as u16).to_le_bytes());
            header.extend(0u16.to_le_bytes());
        };
        out.extend(0x0403_4b50u32.to_le_bytes());
        fields(&mut out);
        out.extend(name.as_bytes());
        out.extend(&compressed);
        directory.extend(0x0201_4b50u32.to_le_bytes());
        directory.extend(20u16.to_le_bytes());
        fields(&mut directory);
        // Comment length, disk, internal and external attributes.
        directory.extend([0; 10]);
        directory.extend(offset.to_le_bytes());
        directory.extend(name.as_bytes());
    }
    let count = (files.len() as u16).to_le_bytes();
    let (directory_len, directory_at) = (directory.len() as u32, out.len() as u32);
    out.extend(directory);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend([0; 4]);
    out.extend(count);
    out.extend(count);
    out.extend(directory_len.to_le_bytes());
    out.extend(directory_at.to_le_bytes());
    out.extend([0; 2]);
    Ok(out)
}

const SUMMARY_SYSTEM_PROMPT: &str = "You are helping a user file a bug report about yapi, the coding agent they are talking to. You will be shown the conversation transcript. Write a report for the yapi developers describing what the user was doing and what went wrong.\n\nDo NOT continue the conversation. Do NOT respond to any questions in the conversation. ONLY output the report.";

const SUMMARY_INSTRUCTIONS: &str = "Write the bug report in Markdown with these sections:\n\n## What the user was doing\nOne short paragraph.\n\n## What went wrong\nConcrete description of the failure: wrong output, errors, hangs, tool failures, unexpected behavior. Quote error messages and tool output verbatim where they exist.\n\n## Steps to reproduce\nNumbered list, as specific as the transcript allows.\n\n## Relevant details\nTool calls involved, files touched, model behavior, anything else that helps a developer reproduce or locate the problem.\n\nDo not include file contents, secrets, or credentials from the transcript; refer to files by path only. Keep the report factual and concise.";

/// pi's `generateBugReportSummary`: a report `summarizer`'s model writes
/// without tools from the last `messages` that fit in 60% of its context
/// window, with the user's `hint`.
pub(crate) async fn summary(
    summarizer: &Summarizer<'_>,
    messages: &[Message],
    hint: Option<&str>,
) -> Result<String, String> {
    let model = summarizer.model;
    let window = if model.context_window > 0 {
        model.context_window
    } else {
        128_000
    };
    let budget = window * 6 / 10;
    let (mut start, mut tokens) = (messages.len(), 0);
    while start > 0 {
        let next = estimate_tokens(&messages[start - 1]);
        if start < messages.len() && tokens + next > budget {
            break;
        }
        tokens += next;
        start -= 1;
    }
    let shown = &messages[start..];
    let mut parts = Vec::new();
    if start > 0 {
        parts.push(format!(
            "Note: only the last {} of {} messages are shown.",
            shown.len(),
            messages.len()
        ));
    }
    let conversation = serialize_conversation(&convert_to_llm(shown.to_vec()));
    parts.push(format!("<conversation>\n{conversation}\n</conversation>"));
    if let Some(hint) = hint.map(str::trim).filter(|hint| !hint.is_empty()) {
        parts.push(format!("<user-report>\n{hint}\n</user-report>"));
    }
    parts.push(SUMMARY_INSTRUCTIONS.to_owned());
    let max_tokens = if model.max_tokens > 0 {
        model.max_tokens.min(4096)
    } else {
        4096
    };
    let response = summarizer
        .complete(SUMMARY_SYSTEM_PROMPT, &parts.join("\n\n"), max_tokens)
        .await;
    if response.stop_reason == StopReason::Aborted {
        return Err("Bug report summary was cancelled".to_owned());
    }
    check_summary(&response, "Bug report summary")?;
    let text = blocks_text(&response.content, "\n").trim().to_owned();
    if text.is_empty() {
        return Err("Bug report summary was empty".to_owned());
    }
    Ok(text)
}

/// Whether the GitHub CLI at `program` can file an issue: not `offline`,
/// installed and signed in. Blocks while `gh auth status` runs.
pub fn github_ready(program: &OsStr, offline: bool) -> bool {
    !offline
        && Command::new(program)
            .args(["auth", "status"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
}

/// Files a public issue on [`REPO`] with the GitHub CLI at `program`,
/// passing `body` through `body_file`, and returns the issue's URL. Blocks
/// while `gh` runs.
pub fn create_issue(
    program: &OsStr,
    title: &str,
    body: &str,
    body_file: &Path,
) -> Result<String, String> {
    std::fs::write(body_file, body).map_err(|error| error.to_string())?;
    let output = Command::new(program)
        .args([
            "issue",
            "create",
            "--repo",
            REPO,
            "--title",
            title,
            "--body-file",
        ])
        .arg(body_file)
        .stdin(Stdio::null())
        .output();
    let _ = std::fs::remove_file(body_file);
    let output = output.map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if stderr.is_empty() {
            format!("gh failed with {}", output.status)
        } else {
            stderr
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().last().unwrap_or_default().trim().to_owned())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn redacts_credential_keys_user_info_and_query_tokens() {
        let mut value = json!({
            "apiKey": "sk-1",
            "nested": {"x-api-key": "sk-2", "Authorization": "Bearer t", "maxTokens": 5},
            "cookies": "kept",
            "proxy": "https://user:pass@proxy.example/path?access_token=abc&q=1",
            "source": "git:https://me:tok@github.com/o/r",
            "plain": "not a url",
            "empty": null,
            "list": [{"refreshToken": "r"}],
        });
        redact(&mut value);
        assert_eq!(
            value,
            json!({
                "apiKey": "<redacted>",
                "nested": {"x-api-key": "<redacted>", "Authorization": "<redacted>", "maxTokens": 5},
                "cookies": "kept",
                "proxy": "https://proxy.example/path?access_token=%3Credacted%3E&q=1",
                "source": "git:https://github.com/o/r",
                "plain": "not a url",
                "empty": null,
                "list": [{"refreshToken": "<redacted>"}],
            })
        );
        assert_eq!(
            redact_url("https://example.com/a?b=c"),
            "https://example.com/a?b=c"
        );
    }

    fn report(session: Option<&str>, summary: Option<&str>) -> Report {
        Report {
            id: "0190-id".into(),
            metadata: json!({"id": "0190-id", "hint": "It broke\nafter /reload", "createdAt": "t"}),
            diagnostics: json!({"assistant": [{"errorMessage": "DIAGNOSTIC-MARKER"}]}),
            session: session.map(str::to_owned),
            summary: summary.map(str::to_owned),
        }
    }

    /// The names and texts of a zip's entries, checked against their CRCs.
    fn unzip(bytes: &[u8]) -> Vec<(String, String)> {
        let u16_at = |at: usize| usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]));
        let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let (mut entries, mut at) = (Vec::new(), 0);
        while u32_at(at) == 0x0403_4b50 {
            let (size, name_len) = (u32_at(at + 18) as usize, u16_at(at + 26));
            let name = String::from_utf8(bytes[at + 30..at + 30 + name_len].to_vec()).unwrap();
            let start = at + 30 + name_len;
            let mut text = String::new();
            flate2::read::DeflateDecoder::new(&bytes[start..start + size])
                .read_to_string(&mut text)
                .unwrap();
            let mut crc = flate2::Crc::new();
            crc.update(text.as_bytes());
            assert_eq!(crc.sum(), u32_at(at + 14));
            entries.push((name, text));
            at = start + size;
        }
        assert_eq!(u32_at(at), 0x0201_4b50);
        entries
    }

    #[test]
    fn zips_pi_file_names_and_leaves_out_a_declined_transcript() {
        let dir = std::env::temp_dir().join(format!("yapi-bug-zip-{}", crate::time::uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let declined = report(None, None);
        assert_eq!(declined.file_name(), "yapi-bug-report-0190-id.zip");
        let path = dir.join(declined.file_name());
        declined.write_zip(&path).unwrap();
        let entries = unzip(&std::fs::read(&path).unwrap());
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["report.json", "diagnostics.json"]);
        assert!(entries[0].1.starts_with("{\n  \"id\": \"0190-id\","));
        let full = report(Some("{\"type\":\"session\"}\n"), Some("## What went wrong"));
        full.write_zip(&path).unwrap();
        let entries = unzip(&std::fs::read(&path).unwrap());
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "report.json",
                "diagnostics.json",
                "session.jsonl",
                "summary.md"
            ]
        );
        assert_eq!(entries[3].1, "## What went wrong\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn issue_bodies_stay_within_the_limit() {
        let mut long = report(None, Some(&"s".repeat(BODY_LIMIT)));
        long.metadata["settings"] = json!("r".repeat(BODY_LIMIT));
        let body = long.issue_body();
        assert!(body.len() <= BODY_LIMIT, "{}", body.len());
        assert!(body.ends_with("[truncated]\n```\n"));
        assert_eq!(long.title(), "It broke");
        let mut untitled = report(None, None);
        untitled.metadata["hint"] = Value::Null;
        assert_eq!(untitled.title(), "yapi bug report 0190-id");
    }

    /// A session on model `m` of provider `p`, which `faux` answers.
    fn faux_session(faux: &yapi_ai::faux::Faux) -> AgentSession {
        let model: Model = serde_json::from_value(json!({
            "id": "m", "name": "M", "api": "faux", "provider": "p", "baseUrl": "",
            "reasoning": false, "input": ["text"],
            "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            "contextWindow": 1000, "maxTokens": 100,
        }))
        .unwrap();
        let mut registry = ModelRegistry::builtin();
        registry.register_provider("p", vec![model.clone()]);
        registry.set_runtime_key("p", "sk-SECRET".into());
        let mut apis = yapi_ai::api::Apis::default();
        apis.register(std::sync::Arc::new(faux.clone()));
        let session = AgentSession::new(crate::agent_session::SessionConfig {
            cwd: "/work".into(),
            agent_dir: "/agent".into(),
            settings: crate::settings::SettingsManager::in_memory(),
            registry,
            apis,
            session: SessionManager::in_memory(Path::new("/work")),
            model: Some(model),
            thinking_level: yapi_types::message::ThinkingLevel::Off,
            tools: Vec::new(),
            extensions: Vec::new(),
            include_extension_tools: false,
            allowed_tools: None,
            excluded_tools: Vec::new(),
            resources: crate::agent_session::Resources::default(),
            docs: crate::docs::Locations::default(),
        });
        let message = Message::User(yapi_types::message::UserMessage {
            content: yapi_types::message::Content::Text("the agent hangs".into()),
            timestamp: 0,
        });
        session
            .with_session(|file| file.append_message(message))
            .unwrap();
        session
    }

    #[test]
    fn reports_hold_the_cwd_and_transcript_only_when_included() {
        let session = faux_session(&yapi_ai::faux::Faux::default());
        let declined = Report::collect(&session, "  it hangs ", false, None);
        assert_eq!(declined.metadata["hint"], "it hangs");
        assert_eq!(declined.metadata["session"].get("cwd"), None);
        assert_eq!(declined.metadata["model"]["id"], "m");
        assert_eq!(
            declined.metadata["provider"]["authStatus"]["source"],
            "runtime"
        );
        assert!(declined.session.is_none());
        assert!(!pretty(&declined.metadata).contains("sk-SECRET"));
        let included = Report::collect(&session, "", true, None);
        assert_eq!(included.metadata["hint"], Value::Null);
        assert_eq!(included.metadata["session"]["cwd"], "/work");
        assert!(included.session.unwrap().contains("the agent hangs"));
        assert_eq!(included.diagnostics["assistantMessageCount"], 0);
    }

    #[tokio::test]
    async fn summaries_are_about_yapi_and_fail_on_tool_calls_or_no_text() {
        use tokio_util::sync::CancellationToken;
        use yapi_ai::faux::{Faux, Response};

        let faux = Faux::new([
            Response::text(" ## What went wrong\nIt hung. "),
            Response::tool_call("t", "read", json!({})),
            Response::text(" "),
        ]);
        let session = faux_session(&faux);
        let summary = session.bug_report_summary(Some(" after /reload "), CancellationToken::new());
        assert_eq!(summary.await.as_deref(), Ok("## What went wrong\nIt hung."));
        let request = format!("{:?}", faux.requests()[0]);
        assert!(request.contains("a bug report about yapi"), "{request}");
        assert!(request.contains("the agent hangs"));
        assert!(request.contains("<user-report>\\nafter /reload\\n</user-report>"));
        let tool = session
            .bug_report_summary(None, CancellationToken::new())
            .await;
        assert_eq!(
            tool,
            Err("Bug report summary attempted to call a tool".to_owned())
        );
        let empty = session
            .bug_report_summary(None, CancellationToken::new())
            .await;
        assert_eq!(empty, Err("Bug report summary was empty".to_owned()));
    }

    /// `gh issue create` gets its arguments as given, never through a
    /// shell, and a body file without the transcript or the diagnostics.
    #[cfg(unix)]
    #[test]
    fn issues_are_created_with_an_argument_vector_and_a_body_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("yapi-bug-gh-{}", crate::time::uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let gh = dir.join("gh");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{dir}/args'\nwhile [ $# -gt 0 ]; do [ \"$1\" = --body-file ] && cp \"$2\" '{dir}/body'; shift; done\necho https://github.com/SkymanOne/yapi/issues/7\n",
            dir = dir.display()
        );
        std::fs::write(&gh, script).unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
        let report = report(Some("TRANSCRIPT-MARKER"), Some("SUMMARY-MARKER"));
        let title = format!("$(touch {0}/pwned); `touch {0}/pwned` \"'", dir.display());
        let body_file = dir.join("issue.md");
        let url = create_issue(gh.as_os_str(), &title, &report.issue_body(), &body_file);
        assert_eq!(
            url.as_deref(),
            Ok("https://github.com/SkymanOne/yapi/issues/7")
        );
        let args = std::fs::read_to_string(dir.join("args")).unwrap();
        let expected = [
            "issue",
            "create",
            "--repo",
            "SkymanOne/yapi",
            "--title",
            title.as_str(),
            "--body-file",
        ];
        assert_eq!(args.lines().take(7).collect::<Vec<_>>(), expected);
        assert_eq!(args.lines().nth(7), Some(body_file.to_str().unwrap()));
        assert!(!dir.join("pwned").exists());
        assert!(!body_file.exists());
        let body = std::fs::read_to_string(dir.join("body")).unwrap();
        assert!(body.starts_with("It broke\nafter /reload\n\n## Summary\n\nSUMMARY-MARKER"));
        assert!(body.contains("\"createdAt\": \"t\""));
        assert!(!body.contains("TRANSCRIPT-MARKER") && !body.contains("DIAGNOSTIC-MARKER"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
