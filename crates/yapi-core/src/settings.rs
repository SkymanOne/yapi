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

/// `defaultTools`: a list of plain names replaces the inherited one; a list of
/// only `+name` and `-name` entries is appended to it.
fn merge_default_tools(base: Option<&Value>, overrides: Option<&Value>) -> Option<Value> {
    let overrides = overrides?.as_array()?;
    let modifiers_only = overrides.iter().all(|entry| {
        entry
            .as_str()
            .is_some_and(|name| name.starts_with('+') || name.starts_with('-'))
    });
    if !modifiers_only {
        return Some(Value::Array(overrides.clone()));
    }
    let mut merged = base.and_then(Value::as_array).cloned().unwrap_or_default();
    merged.extend(overrides.iter().cloned());
    Some(Value::Array(merged))
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
        // Unknown or invalid fields are ignored, as in pi.
        self.merged = serde_json::from_value(Value::Object(merged.clone())).unwrap_or_else(|_| {
            let valid: Map<String, Value> = merged
                .into_iter()
                .filter(|(key, value)| {
                    let single = Map::from_iter([(key.clone(), value.clone())]);
                    serde_json::from_value::<Settings>(Value::Object(single)).is_ok()
                })
                .collect();
            serde_json::from_value(Value::Object(valid)).unwrap_or_default()
        });
    }

    /// The merged settings.
    pub fn settings(&self) -> &Settings {
        &self.merged
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
    /// other keys, and writes that scope's file.
    pub fn set_nested(
        &mut self,
        scope: Scope,
        field: &str,
        key: &str,
        value: Value,
    ) -> Result<(), SettingsError> {
        self.update(scope, |document| {
            let mut object = match document.get(field) {
                Some(Value::Object(object)) => object.clone(),
                _ => Map::new(),
            };
            object.insert(key.to_owned(), value.clone());
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

    #[test]
    fn invalid_timeouts_fail_as_in_pi() {
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
    }

    #[test]
    fn invalid_fields_leave_the_valid_ones() {
        let mut manager = SettingsManager::in_memory();
        manager.global = json!({
            "theme": "dark",
            "retry": "often",
            "steeringMode": "all",
            "markdown": {"codeBlockIndent": "    ", "mermaid": "bogus"},
        })
        .as_object()
        .unwrap()
        .clone();
        manager.remerge();
        let settings = manager.settings();
        assert_eq!(settings.theme.as_deref(), Some("dark"));
        assert!(settings.retry.is_none());
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
    fn cache_warming_comes_from_the_global_settings_only() {
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
            .set_nested(Scope::Global, "retry", "enabled", json!(false))
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
