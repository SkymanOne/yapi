//! Settings: global `settings.json` merged with the trusted project's.
//!
//! Port of `packages/coding-agent/src/core/settings-manager.ts` in pi `v1.0.0`.
//! Each scope is an order-preserving document; edits re-read its file under the
//! file's lock, change it and write it back the way pi does. Readers use the
//! typed view of the merged document.

use std::path::{Path, PathBuf};

use serde::Deserialize as _;
use serde_json::{Map, Value};
use yapi_ai::auth::lock::{FileLock, write_atomic};
use yapi_ai::http::is_truthy;
use yapi_types::config::ConfigFile;
use yapi_types::settings::{CacheWarming, Settings};

use crate::config::PROJECT_DIR;

/// Which file a setting lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// `<agent>/settings.json`.
    Global,
    /// `<project>/.yapi/settings.json`.
    Project,
}

/// Settings errors.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The file is not valid settings JSON.
    #[error("Failed to parse {path}: {message}")]
    Parse {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        message: String,
    },
}

/// Both scopes and their merged view.
#[derive(Clone, Debug, Default)]
pub struct SettingsManager {
    project_dir: PathBuf,
    global_path: PathBuf,
    project_path: PathBuf,
    global: Map<String, Value>,
    project: Map<String, Value>,
    project_trusted: bool,
    /// Both scopes merged as written, as pi's `settings`.
    document: Map<String, Value>,
    merged: Settings,
    /// Files that failed to load, by scope; they read as empty and are never
    /// written, so a broken file is not overwritten.
    broken: [bool; 2],
    errors: Vec<String>,
}

fn read_document(path: &Path) -> Result<Map<String, Value>, SettingsError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(source) => {
            return Err(SettingsError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(mut map)) => {
            migrate(&mut map);
            Ok(map)
        }
        Ok(_) => Err(SettingsError::Parse {
            path: path.to_path_buf(),
            message: "expected an object".into(),
        }),
        Err(err) => Err(SettingsError::Parse {
            path: path.to_path_buf(),
            message: err.to_string(),
        }),
    }
}

/// pi's `migrateSettings`: settings in older forms take their current
/// ones in the document, which is written back that way the next time its
/// scope is saved. Keys move as JavaScript moves them: a new key goes last
/// and a reassigned one keeps its place.
fn migrate(document: &mut Map<String, Value>) {
    if !document.contains_key("steeringMode")
        && let Some(mode) = document.shift_remove("queueMode")
    {
        document.insert("steeringMode".into(), mode);
    }
    if let Some(&Value::Bool(websockets)) = document.get("websockets")
        && !document.contains_key("transport")
    {
        let transport = if websockets { "websocket" } else { "sse" };
        document.insert("transport".into(), Value::from(transport));
        document.shift_remove("websockets");
    }
    if let Some(Value::Object(skills)) = document.get("skills").cloned() {
        if let Some(enable) = skills.get("enableSkillCommands")
            && !document.contains_key("enableSkillCommands")
        {
            document.insert("enableSkillCommands".into(), enable.clone());
        }
        match skills.get("customDirectories") {
            Some(Value::Array(directories)) if !directories.is_empty() => {
                document.insert("skills".into(), Value::Array(directories.clone()));
            }
            _ => {
                document.shift_remove("skills");
            }
        }
    }
    if let Some(Value::Object(retry)) = document.get_mut("retry") {
        let provider = retry.get("provider").and_then(Value::as_object).cloned();
        let delay_unset = provider
            .as_ref()
            .is_none_or(|provider| provider.get("maxRetryDelayMs").is_none_or(Value::is_null));
        if let Some(delay) = retry.get("maxDelayMs").filter(|delay| delay.is_number())
            && delay_unset
        {
            let mut provider = provider.unwrap_or_default();
            provider.insert("maxRetryDelayMs".into(), delay.clone());
            retry.insert("provider".into(), Value::Object(provider));
        }
        retry.shift_remove("maxDelayMs");
    }
}

/// JavaScript's `String(value)` for a JSON value.
pub fn js_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                item => js_string(item),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
        other => yapi_types::json::stringify(other),
    }
}

/// pi's deep merge: objects merge recursively, anything else replaces.
fn deep_merge(base: &Map<String, Value>, overrides: &Map<String, Value>) -> Map<String, Value> {
    let mut result = base.clone();
    for (key, value) in overrides {
        let merged = match (base.get(key), value) {
            (Some(Value::Object(base)), Value::Object(over)) => {
                Value::Object(deep_merge(base, over))
            }
            _ => value.clone(),
        };
        result.insert(key.clone(), merged);
    }
    result
}

fn is_tool_modifier(entry: &str) -> bool {
    entry.starts_with('+') || entry.starts_with('-')
}

/// `defaultTools`: a list of plain names, or any other value, replaces the
/// inherited one; a list of only `+name` and `-name` entries is appended to it.
fn merge_default_tools(base: Option<&Value>, overrides: Option<&Value>) -> Option<Value> {
    let Some(overrides) = overrides else {
        return base.cloned();
    };
    match (base.and_then(Value::as_array), overrides.as_array()) {
        (Some(base), Some(list))
            if list
                .iter()
                .all(|entry| entry.as_str().is_some_and(is_tool_modifier)) =>
        {
            Some(Value::Array(base.iter().chain(list).cloned().collect()))
        }
        _ => Some(overrides.clone()),
    }
}

/// pi's `resolveDefaultTools`: the plain names, or the default tools when a
/// list has only modifiers, then each `+name` adds and `-name` removes a tool
/// in list order.
fn resolve_default_tools(entries: &[&str]) -> Vec<String> {
    let mut tools: Vec<String> = entries
        .iter()
        .filter(|entry| !is_tool_modifier(entry))
        .map(|&entry| entry.to_owned())
        .collect();
    if tools.is_empty() && !entries.is_empty() {
        tools = crate::tools::DEFAULT_TOOLS.map(str::to_owned).to_vec();
    }
    for entry in entries.iter().filter(|entry| is_tool_modifier(entry)) {
        let name = &entry[1..];
        let index = tools.iter().position(|tool| tool == name);
        match index {
            None if entry.starts_with('+') && !name.is_empty() => tools.push(name.to_owned()),
            Some(index) if entry.starts_with('-') => {
                tools.remove(index);
            }
            _ => {}
        }
    }
    tools
}

/// Whether [`Settings`] reads `value` at `path`.
fn reads_at(path: &[String], value: &Value) -> bool {
    let nested = path.iter().rev().fold(value.clone(), |inner, key| {
        Value::Object(Map::from_iter([(key.clone(), inner)]))
    });
    Settings::deserialize(nested).is_ok()
}

/// The fields of `object`, found at `path` in the settings, that [`Settings`]
/// reads: a field of the wrong type is left out alone, also inside an object,
/// as pi reads each field on its own.
fn valid_fields(path: &mut Vec<String>, object: Map<String, Value>) -> Map<String, Value> {
    let mut valid = Map::new();
    for (key, value) in object {
        path.push(key.clone());
        let kept = match value {
            value if reads_at(path, &value) => Some(value),
            Value::Object(inner) => {
                Some(Value::Object(valid_fields(path, inner))).filter(|inner| reads_at(path, inner))
            }
            _ => None,
        };
        path.pop();
        if let Some(value) = kept {
            valid.insert(key, value);
        }
    }
    valid
}

impl SettingsManager {
    /// Loads both scopes, the project's from `project_dir`, and that only
    /// when `project_trusted`.
    pub fn load(agent_dir: &Path, project_dir: &Path, project_trusted: bool) -> SettingsManager {
        let global_path = agent_dir.join(ConfigFile::Settings.file_name());
        let project_path = project_dir
            .join(PROJECT_DIR)
            .join(ConfigFile::Settings.file_name());
        let mut errors = Vec::new();
        let mut broken = [false; 2];
        // pi reports a file that fails to load and carries on without it.
        let mut read = |path: &Path, scope: usize| match read_document(path) {
            Ok(document) => document,
            Err(error) => {
                let message = match error {
                    SettingsError::Parse { message, .. } => message,
                    SettingsError::Io { source, .. } => source.to_string(),
                };
                errors.push(format!(
                    "Invalid settings file {}: {message}",
                    path.display()
                ));
                broken[scope] = true;
                Map::new()
            }
        };
        let global = read(&global_path, 0);
        let project = if project_trusted {
            read(&project_path, 1)
        } else {
            Map::new()
        };
        let mut manager = SettingsManager {
            project_dir: project_dir.to_path_buf(),
            global_path,
            project_path,
            global,
            project,
            project_trusted,
            document: Map::new(),
            merged: Settings::default(),
            broken,
            errors,
        };
        manager.remerge();
        manager
    }

    /// Settings with no files behind them.
    pub fn in_memory() -> SettingsManager {
        SettingsManager::default()
    }

    /// Why settings files failed to load, as pi's warnings word them.
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    fn remerge(&mut self) {
        let mut merged = deep_merge(&self.global, &self.project);
        if let Some(tools) = merge_default_tools(
            self.global.get("defaultTools"),
            self.project.get("defaultTools"),
        ) {
            merged.insert("defaultTools".into(), tools);
        }
        self.document = merged.clone();
        // pi reads these switches by truthiness, so a value of another type
        // keeps the rest of its object; `null` reads as unset.
        for field in ["compaction", "retry"] {
            if let Some(Value::Object(object)) = merged.get_mut(field)
                && let Some(enabled) = object.get("enabled").filter(|value| !value.is_boolean())
            {
                match enabled {
                    Value::Null => object.shift_remove("enabled"),
                    value => object.insert("enabled".into(), is_truthy(value).into()),
                };
            }
        }
        // Unknown or invalid fields are ignored, as in pi.
        self.merged = Settings::deserialize(Value::Object(merged.clone())).unwrap_or_else(|_| {
            let valid = Value::Object(valid_fields(&mut Vec::new(), merged));
            Settings::deserialize(valid).unwrap_or_default()
        });
    }

    /// The merged settings.
    pub fn settings(&self) -> &Settings {
        &self.merged
    }

    /// pi's `getDefaultTools`: the tools the merged `defaultTools` enables, or
    /// `None` when no scope sets it. A value that is not a list enables none,
    /// and entries that are not strings are skipped.
    pub fn default_tools(&self) -> Option<Vec<String>> {
        let merged = merge_default_tools(
            self.global.get("defaultTools"),
            self.project.get("defaultTools"),
        )?;
        let entries: Vec<&str> = match &merged {
            Value::Array(entries) => entries.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        Some(resolve_default_tools(&entries))
    }

    /// Both scopes merged as written, values of any type included: pi's
    /// `settings`.
    pub fn merged_document(&self) -> &Map<String, Value> {
        &self.document
    }

    /// `defaultThinkingLevel`, if set: a value that names no level becomes
    /// `off`, the first level pi's clamp offers.
    pub fn default_thinking_level(&self) -> Option<yapi_types::message::ThinkingLevel> {
        let value = self
            .project
            .get("defaultThinkingLevel")
            .or_else(|| self.global.get("defaultThinkingLevel"))?;
        Some(
            value
                .as_str()
                .and_then(yapi_types::message::ThinkingLevel::parse)
                .unwrap_or(yapi_types::message::ThinkingLevel::Off),
        )
    }

    /// pi's `getHttpIdleTimeoutMs`: `httpIdleTimeoutMs` in milliseconds, as a
    /// number, a numeric string or `"disabled"` (0); five minutes when unset.
    pub fn http_idle_timeout_ms(&self) -> Result<u64, String> {
        Ok(self.timeout_ms("httpIdleTimeoutMs")?.unwrap_or(300_000))
    }

    /// pi's `getWebSocketConnectTimeoutMs`: `websocketConnectTimeoutMs`, read
    /// as `httpIdleTimeoutMs` is; `None` when unset.
    pub fn websocket_connect_timeout_ms(&self) -> Result<Option<u64>, String> {
        self.timeout_ms("websocketConnectTimeoutMs")
    }

    /// `retry.provider.timeoutMs` when it is a number, unchecked, as pi
    /// passes it on to providers.
    pub fn provider_timeout_ms(&self) -> Option<f64> {
        let read = |document: &Map<String, Value>| {
            document
                .get("retry")?
                .get("provider")?
                .get("timeoutMs")?
                .as_f64()
        };
        read(&self.project).or_else(|| read(&self.global))
    }

    /// pi's `parseTimeoutSetting` of setting `key`: milliseconds as a
    /// number, a numeric string or `"disabled"` (0); `None` when unset, and
    /// pi's error for any other value.
    fn timeout_ms(&self, key: &str) -> Result<Option<u64>, String> {
        let Some(value) = self.project.get(key).or_else(|| self.global.get(key)) else {
            return Ok(None);
        };
        let number = match value {
            Value::String(text) if text.trim().eq_ignore_ascii_case("disabled") => Some(0.0),
            Value::String(text) if text.trim().is_empty() => None,
            Value::String(text) => text.trim().parse::<f64>().ok(),
            Value::Number(number) => number.as_f64(),
            _ => None,
        };
        number
            .filter(|ms| ms.is_finite() && *ms >= 0.0)
            .map(|ms| Some(ms.floor() as u64))
            .ok_or_else(|| format!("Invalid {key} setting: {}", js_string(value)))
    }

    /// pi's `getCompactionTokenSetting` checks: the error for a
    /// `compaction` token setting, or `model`'s override of one, that is not
    /// a non-negative safe integer.
    pub fn compaction_error(&self, model: Option<&yapi_types::model::Model>) -> Option<String> {
        const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
        let merged = deep_merge(&self.global, &self.project);
        let compaction = merged.get("compaction");
        let key = model.map(|model| format!("{}/{}", model.provider, model.id));
        let entry = key
            .as_ref()
            .and_then(|key| compaction?.get("modelOverrides")?.get(key));
        let invalid = |value: &&Value| {
            !value
                .as_f64()
                .is_some_and(|n| n.fract() == 0.0 && (0.0..=MAX_SAFE_INTEGER).contains(&n))
        };
        let expected = "Expected a non-negative safe integer.";
        for field in ["reserveTokens", "keepRecentTokens"] {
            if let Some(value) = compaction.and_then(|c| c.get(field)).filter(invalid) {
                let value = js_string(value);
                return Some(format!(
                    "Invalid compaction.{field} setting: {value}. {expected}"
                ));
            }
            let (Some(key), Some(entry)) = (&key, entry) else {
                continue;
            };
            let name = format!("compaction.modelOverrides[\"{key}\"]");
            if !entry.is_object() {
                let entry = js_string(entry);
                return Some(format!(
                    "Invalid {name} setting: {entry}. Expected an object."
                ));
            }
            if let Some(value) = entry.get(field).filter(invalid) {
                let value = js_string(value);
                return Some(format!(
                    "Invalid {name}.{field} setting: {value}. {expected}"
                ));
            }
        }
        None
    }

    /// pi's `getCacheWarmingMode`: `cacheWarming` from the global settings
    /// only, because each refresh costs money; `streaming` when unset or
    /// invalid.
    pub fn cache_warming_mode(&self) -> CacheWarming {
        self.global
            .get("cacheWarming")
            .and_then(|mode| CacheWarming::deserialize(mode).ok())
            .unwrap_or(CacheWarming::Streaming)
    }

    /// The directory whose `.yapi` folder holds the project's files: the
    /// project's settings, resources and system prompt files.
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// Whether the project scope is loaded.
    pub fn project_trusted(&self) -> bool {
        self.project_trusted
    }

    /// The settings document of `scope` as written; empty for an untrusted
    /// project.
    pub fn document(&self, scope: Scope) -> &Map<String, Value> {
        match scope {
            Scope::Global => &self.global,
            Scope::Project => &self.project,
        }
    }

    /// Sets a top-level key in a scope and writes that file. `None` removes it.
    pub fn set(
        &mut self,
        scope: Scope,
        key: &str,
        value: Option<Value>,
    ) -> Result<(), SettingsError> {
        self.update(scope, |document| match &value {
            Some(value) => {
                document.insert(key.to_owned(), value.clone());
            }
            None => {
                document.shift_remove(key);
            }
        })
    }

    /// Sets `key` inside the object setting `field` of `scope`, keeping its
    /// other keys, and writes that scope's file. `None` removes the key and
    /// keeps the object.
    pub fn set_nested(
        &mut self,
        scope: Scope,
        field: &str,
        key: &str,
        value: Option<Value>,
    ) -> Result<(), SettingsError> {
        self.update(scope, |document| {
            let mut object = match document.get(field) {
                Some(Value::Object(object)) => object.clone(),
                _ => Map::new(),
            };
            match &value {
                Some(value) => object.insert(key.to_owned(), value.clone()),
                None => object.shift_remove(key),
            };
            document.insert(field.to_owned(), Value::Object(object));
        })
    }

    /// Applies `change` to `scope` and writes its file as pi does: under the
    /// file's lock, to the file as it is now, so keys other processes changed
    /// since loading are kept. The scope then holds the file's document.
    fn update(
        &mut self,
        scope: Scope,
        change: impl Fn(&mut Map<String, Value>),
    ) -> Result<(), SettingsError> {
        let (document, path, broken) = match scope {
            Scope::Global => (&mut self.global, &self.global_path, self.broken[0]),
            Scope::Project => (&mut self.project, &self.project_path, self.broken[1]),
        };
        let mut written = None;
        if !path.as_os_str().is_empty() && !broken {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|source| SettingsError::Io {
                    path: dir.to_path_buf(),
                    source,
                })?;
            }
            let failed = |source: std::io::Error| SettingsError::Io {
                path: path.clone(),
                source,
            };
            let _lock = FileLock::acquire_blocking(path)
                .map_err(|err| failed(std::io::Error::other(err)))?;
            let mut current = read_document(path)?;
            change(&mut current);
            let text = ConfigFile::Settings
                .render(&Value::Object(current.clone()))
                .map_err(|err| SettingsError::Parse {
                    path: path.clone(),
                    message: err.to_string(),
                })?;
            write_atomic(path, text.as_bytes(), 0o666).map_err(failed)?;
            written = Some(current);
        }
        match written {
            // An untrusted project's other settings stay unloaded.
            Some(current) if scope == Scope::Global || self.project_trusted => *document = current,
            _ => change(document),
        }
        self.remerge();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_like_pi() {
        let global = json!({"compaction": {"enabled": true, "reserveTokens": 100}, "defaultTools": ["read", "bash"], "theme": "dark"});
        let project =
            json!({"compaction": {"reserveTokens": 5}, "defaultTools": ["+grep", "-bash"]});
        let mut merged = deep_merge(global.as_object().unwrap(), project.as_object().unwrap());
        merged.insert(
            "defaultTools".into(),
            merge_default_tools(global.get("defaultTools"), project.get("defaultTools")).unwrap(),
        );
        assert_eq!(
            Value::Object(merged),
            json!({"compaction": {"enabled": true, "reserveTokens": 5}, "defaultTools": ["read", "bash", "+grep", "-bash"], "theme": "dark"})
        );
        assert_eq!(
            merge_default_tools(Some(&json!(["read"])), Some(&json!(["ls"]))),
            Some(json!(["ls"]))
        );
        // pi's getDefaultTools resolves the merged list.
        let resolved = |global: &Value, project: &Value| {
            let mut manager = SettingsManager::in_memory();
            manager.global = Map::from_iter([("defaultTools".to_owned(), global.clone())]);
            manager.project = Map::from_iter([("defaultTools".to_owned(), project.clone())]);
            manager.default_tools().unwrap()
        };
        let (global, project) = (&global["defaultTools"], &project["defaultTools"]);
        assert_eq!(resolved(global, project), ["read", "grep"]);
        let plain = json!(["read", "+ls"]);
        assert_eq!(resolved(&plain, &json!(["+find"])), ["read", "ls", "find"]);
        let modifiers = json!(["-bash"]);
        assert_eq!(
            resolved(&modifiers, &json!(["+grep"])),
            ["read", "edit", "write", "grep"]
        );
        // An empty list, or a value that is not a list, enables no tool.
        assert!(resolved(&json!([]), &json!([])).is_empty());
        assert!(resolved(&plain, &json!("read")).is_empty());
        assert_eq!(SettingsManager::in_memory().default_tools(), None);
    }

    /// pi's legacy setting forms, migrated as pi writes them back.
    #[test]
    fn migrates_like_pi() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/pi/settings/migrations.json");
        let cases: Vec<Value> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let dir =
            std::env::temp_dir().join(format!("yapi-settings-migrate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        for case in cases {
            std::fs::write(&file, case["input"].as_str().unwrap()).unwrap();
            let mut manager = SettingsManager::load(&dir, &dir.join("cwd"), false);
            manager
                .set(Scope::Global, "lastChangelogVersion", Some(json!("1.0.0")))
                .unwrap();
            assert_eq!(
                std::fs::read_to_string(&file).unwrap(),
                case["output"].as_str().unwrap(),
                "{}",
                case["name"]
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn invalid_timeouts_fail() {
        let mut manager = SettingsManager::in_memory();
        manager.global =
            json!({"httpIdleTimeoutMs": " 1500.7 ", "websocketConnectTimeoutMs": "soon"})
                .as_object()
                .unwrap()
                .clone();
        assert_eq!(manager.http_idle_timeout_ms(), Ok(1500));
        assert_eq!(
            manager.websocket_connect_timeout_ms(),
            Err("Invalid websocketConnectTimeoutMs setting: soon".to_owned())
        );
        manager.global = json!({"httpIdleTimeoutMs": -1, "websocketConnectTimeoutMs": "disabled"})
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(
            manager.http_idle_timeout_ms(),
            Err("Invalid httpIdleTimeoutMs setting: -1".to_owned())
        );
        assert_eq!(manager.websocket_connect_timeout_ms(), Ok(Some(0)));
        manager.global = Map::new();
        assert_eq!(manager.http_idle_timeout_ms(), Ok(300_000));
        assert_eq!(manager.websocket_connect_timeout_ms(), Ok(None));
        assert_eq!(manager.compaction_error(None), None);
        // pi's compaction getters take whole numbers only.
        manager.global = json!({"compaction": {"reserveTokens": 1.5e5, "keepRecentTokens": 1.5}})
            .as_object()
            .unwrap()
            .clone();
        let error = "Invalid compaction.keepRecentTokens setting: 1.5. Expected a non-negative safe integer.";
        assert_eq!(manager.compaction_error(None).as_deref(), Some(error));

        // As pi's next-turn hook throws it, the run ends between turns with a
        // failed response; as its check before a prompt throws it, the next
        // prompt fails before its request.
        use yapi_ai::faux::{Faux, Response};
        use yapi_types::message::{Message, StopReason};
        let faux = Faux::new([
            Response::tool_call("t", "missing", json!({})),
            Response::text("unused"),
        ]);
        let model: yapi_types::model::Model = serde_json::from_value(json!({
            "id": "m", "name": "M", "api": "faux", "provider": "p", "baseUrl": "",
            "reasoning": false, "input": ["text"],
            "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
            "contextWindow": 1000, "maxTokens": 100,
        }))
        .unwrap();
        let mut registry = yapi_ai::registry::ModelRegistry::builtin();
        registry.register_provider("p", vec![model.clone()]);
        registry.set_runtime_key("p", "key".into());
        let mut apis = yapi_ai::api::Apis::default();
        apis.register(std::sync::Arc::new(faux.clone()));
        let session =
            crate::agent_session::AgentSession::new(crate::agent_session::SessionConfig {
                cwd: "/work".into(),
                agent_dir: "/agent".into(),
                settings: manager,
                registry,
                apis,
                session: crate::session::SessionManager::in_memory(Path::new("/work")),
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
        assert_eq!(session.prompt("go", Vec::new()).await, Err(error.into()));
        let Some(Message::Assistant(failed)) = session.messages().pop() else {
            panic!("no failed response");
        };
        assert_eq!(
            (failed.stop_reason, failed.error_message.as_deref()),
            (StopReason::Error, Some(error))
        );
        assert_eq!(session.prompt("again", Vec::new()).await, Err(error.into()));
        assert_eq!(faux.requests().len(), 1);
    }

    #[test]
    fn invalid_fields_keep_valid() {
        let mut manager = SettingsManager::in_memory();
        manager.global = json!({
            "theme": "dark",
            "retry": "often",
            "steeringMode": "all",
            "markdown": {"codeBlockIndent": "    ", "mermaid": "bogus"},
            "compaction": {"enabled": "", "reserveTokens": "16384", "keepRecentTokens": 2e4},
            "branchSummary": {"skipPrompt": true, "reserveTokens": 1.5e5},
            "modelThinkingLevels": {"a/b": "high", "a/c": "extreme"},
            "terminal": {"imageWidthCells": 40.7},
            "outputPad": 0.5,
        })
        .as_object()
        .unwrap()
        .clone();
        manager.remerge();
        let settings = manager.settings();
        assert_eq!(settings.theme.as_deref(), Some("dark"));
        assert!(settings.retry.is_none());
        // A wrongly typed nested field drops alone, and pi reads a switch of
        // another type by truthiness.
        let compaction = settings.compaction.as_ref().unwrap();
        let tokens = (compaction.reserve_tokens, compaction.keep_recent_tokens);
        assert_eq!(
            (compaction.enabled, tokens),
            (Some(false), (None, Some(20000)))
        );
        // A whole float is its integer; pi rounds a fraction down where it
        // counts cells, and reads any output padding but 0 as 1.
        let summary = settings.branch_summary.as_ref().unwrap();
        assert_eq!(
            (summary.skip_prompt, summary.reserve_tokens),
            (Some(true), Some(150_000))
        );
        let terminal = settings.terminal.as_ref().unwrap();
        assert_eq!(
            (terminal.image_width_cells, settings.output_pad),
            (Some(40), None)
        );
        assert_eq!(settings.model_thinking_levels.as_ref().unwrap().len(), 1);
        assert_eq!(
            settings.steering_mode,
            Some(yapi_types::settings::QueueMode::All)
        );
        // pi reads a mermaid mode it does not know as streaming.
        assert_eq!(
            settings.markdown,
            Some(yapi_types::settings::MarkdownSettings {
                code_block_indent: Some("    ".into()),
                mermaid: Some(yapi_types::settings::MermaidMode::Streaming),
            })
        );
    }

    #[test]
    fn cache_warming_global_only() {
        let mut manager = SettingsManager::in_memory();
        assert_eq!(manager.cache_warming_mode(), CacheWarming::Streaming);
        manager.project = json!({"cacheWarming": "off"}).as_object().unwrap().clone();
        manager.remerge();
        assert_eq!(manager.cache_warming_mode(), CacheWarming::Streaming);
        manager.global = json!({"cacheWarming": "idle"}).as_object().unwrap().clone();
        assert_eq!(manager.cache_warming_mode(), CacheWarming::Idle);
        manager.global = json!({"cacheWarming": "often"})
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(manager.cache_warming_mode(), CacheWarming::Streaming);
    }

    /// Two processes that loaded the same file keep each other's changes.
    #[test]
    fn concurrent_writes_keep_keys() {
        let dir = std::env::temp_dir().join(format!("yapi-settings-two-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut first = SettingsManager::load(&dir, &dir.join("cwd"), false);
        let mut second = SettingsManager::load(&dir, &dir.join("cwd"), false);
        second
            .set(Scope::Global, "packages", Some(json!(["npm:foo"])))
            .unwrap();
        first
            .set(Scope::Global, "defaultModel", Some(json!("m")))
            .unwrap();
        first
            .set_nested(Scope::Global, "retry", "enabled", Some(json!(false)))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("settings.json")).unwrap(),
            "{\n  \"packages\": [\n    \"npm:foo\"\n  ],\n  \"defaultModel\": \"m\",\n  \"retry\": {\n    \"enabled\": false\n  }\n}"
        );
        assert_eq!(first.document(Scope::Global).len(), 3);
        assert!(!dir.join("settings.json.lock").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
