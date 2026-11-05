//! Settings: global `settings.json` merged with the trusted project's.
//!
//! Port of `packages/coding-agent/src/core/settings-manager.ts` in pi `v1.0.0`.
//! Each scope is an order-preserving document. Edits re-read its file under the
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
    /// `<agent>/settings.json`, or `settings.toml` there.
    Global,
    /// `<project>/.yapi/settings.json`, or `settings.toml` there.
    Project,
}

/// The TOML form of a folder's settings file, which yapi reads and writes
/// in place of `settings.json`.
pub const TOML_FILE: &str = "settings.toml";

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
    /// The file is not valid settings JSON or TOML.
    #[error("Failed to parse {path}: {message}")]
    Parse {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        message: String,
    },
    /// A folder holds both forms of its settings file.
    #[error("Both settings.json and settings.toml are in {}. Keep one of them.", dir.display())]
    Conflict {
        /// The folder.
        dir: PathBuf,
    },
}

/// The settings file of folder `dir`: `settings.toml` when it holds one,
/// else `settings.json`, which a first write creates. A folder that holds
/// both is a [`SettingsError::Conflict`].
pub fn settings_file(dir: &Path) -> Result<PathBuf, SettingsError> {
    let json = dir.join(ConfigFile::Settings.file_name());
    let toml = dir.join(TOML_FILE);
    match (json.exists(), toml.exists()) {
        (true, true) => Err(SettingsError::Conflict {
            dir: dir.to_path_buf(),
        }),
        (_, true) => Ok(toml),
        _ => Ok(json),
    }
}

/// The settings document in the `settings.json` or `settings.toml` at
/// `path`, read as yapi reads it. A missing file is an empty document.
pub fn read_settings(path: &Path) -> Result<Map<String, Value>, SettingsError> {
    read_document(path)
}

fn is_toml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "toml")
}

/// The TOML document at `path` as written, with its text, or an empty one
/// when there is none.
fn read_toml_document(path: &Path) -> Result<(toml_edit::DocumentMut, String), SettingsError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok((toml_edit::DocumentMut::new(), String::new()));
        }
        Err(source) => {
            return Err(SettingsError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let document = text
        .strip_prefix('\u{feff}')
        .unwrap_or(&text)
        .parse()
        .map_err(|err: toml_edit::TomlError| SettingsError::Parse {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
    Ok((document, text))
}

/// `written`, which toml_edit wrote for a document read from `text`, with
/// the byte order mark and line endings of `text`, as toml_edit ends its
/// lines with `\n`. Blank lines that start it and not `text`, as after
/// removing the first table, go.
fn as_read(text: &str, written: String) -> String {
    let (bom, text) = match text.strip_prefix('\u{feff}') {
        Some(text) => ("\u{feff}", text),
        None => ("", text),
    };
    let written = match text.starts_with(['\r', '\n']) {
        true => written,
        false => written.trim_start_matches(['\r', '\n']).to_owned(),
    };
    let written = match text.contains("\r\n") {
        true => written.replace("\r\n", "\n").replace('\n', "\r\n"),
        false => written,
    };
    format!("{bom}{written}")
}

/// The settings that TOML `document` from `path` holds, as `settings.json`
/// would hold them, migrated as pi migrates JSON. A date or time is no
/// setting, so it fails.
fn toml_settings(
    path: &Path,
    document: &toml_edit::DocumentMut,
) -> Result<Map<String, Value>, SettingsError> {
    let mut map = toml_table(document.as_table()).map_err(|message| SettingsError::Parse {
        path: path.to_path_buf(),
        message,
    })?;
    migrate(&mut map);
    Ok(map)
}

fn toml_table(table: &dyn toml_edit::TableLike) -> Result<Map<String, Value>, String> {
    let mut map = Map::new();
    for (key, item) in table.iter() {
        if let Some(value) = toml_item(item)? {
            map.insert(key.to_owned(), value);
        }
    }
    Ok(map)
}

fn toml_item(item: &toml_edit::Item) -> Result<Option<Value>, String> {
    Ok(match item {
        toml_edit::Item::None => None,
        toml_edit::Item::Value(value) => Some(toml_value(value)?),
        toml_edit::Item::Table(table) => Some(Value::Object(toml_table(table)?)),
        toml_edit::Item::ArrayOfTables(tables) => Some(Value::Array(
            tables
                .iter()
                .map(|table| toml_table(table).map(Value::Object))
                .collect::<Result<_, _>>()?,
        )),
    })
}

fn toml_value(value: &toml_edit::Value) -> Result<Value, String> {
    use toml_edit::Value as Toml;
    Ok(match value {
        Toml::String(text) => Value::String(text.value().clone()),
        Toml::Integer(number) => Value::from(*number.value()),
        Toml::Float(number) => serde_json::Number::from_f64(*number.value())
            .map(Value::Number)
            .ok_or_else(|| format!("{} is not a number JSON can hold", number.value()))?,
        Toml::Boolean(flag) => Value::Bool(*flag.value()),
        Toml::Datetime(date) => {
            return Err(format!(
                "{} is a date or time, which no setting takes",
                date.value()
            ));
        }
        Toml::Array(items) => Value::Array(items.iter().map(toml_value).collect::<Result<_, _>>()?),
        Toml::InlineTable(table) => Value::Object(toml_table(table)?),
    })
}

/// `value` as a TOML value: objects as inline tables, `null` keys left out.
fn json_to_toml(value: &Value) -> Result<toml_edit::Value, String> {
    Ok(match value {
        Value::Null => return Err("TOML has no null in an array".to_owned()),
        Value::Bool(flag) => (*flag).into(),
        Value::Number(number) => match (number.as_i64(), number.as_f64()) {
            (Some(integer), _) => integer.into(),
            (None, Some(float)) if !number.is_u64() => float.into(),
            _ => return Err(format!("{number} is beyond the integers TOML holds")),
        },
        Value::String(text) => text.as_str().into(),
        Value::Array(items) => {
            let mut array = toml_edit::Array::new();
            for item in items {
                array.push(json_to_toml(item)?);
            }
            array.into()
        }
        Value::Object(map) => {
            let mut table = toml_edit::InlineTable::new();
            for (key, value) in map.iter().filter(|(_, value)| !value.is_null()) {
                table.insert(key, json_to_toml(value)?);
            }
            table.into()
        }
    })
}

/// Brings `table` in line with `new`, key by key, so unchanged keys keep their
/// comments and layout: keys `new` lacks or sets to `null` go, changed ones
/// change in place, and new ones are added. `root` makes new objects
/// standard tables rather than inline ones. `lines` says the table's keys
/// start their lines, as in a standard table.
fn reconcile(
    table: &mut dyn toml_edit::TableLike,
    new: &Map<String, Value>,
    root: bool,
    lines: bool,
) -> Result<(), String> {
    let gone: Vec<String> = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| new.get(key).is_none_or(Value::is_null))
        .collect();
    for key in gone {
        table.remove(&key);
    }
    for (key, value) in new.iter().filter(|(_, value)| !value.is_null()) {
        // shortcut: the text before a value on its line is known only where
        // keys start lines, not in dotted keys or inline tables; upgrade if
        // their arrays need it.
        let line = table.key(key).filter(|_| lines).map(|key| {
            let decor = key.leaf_decor();
            let prefix = raw(decor.prefix());
            let indent = &prefix[prefix.rfind('\n').map_or(0, |at| at + 1)..];
            format!("{indent}{}{}=", key.display_repr(), raw(decor.suffix()))
        });
        let in_place = match table.get_mut(key) {
            Some(item) => reconcile_item(item, value, line.as_deref())?,
            None => false,
        };
        // New, or a table that becomes a value: a key of its own.
        if !in_place {
            let old = table.remove(key);
            let mut item = match (root, json_to_toml(value)?) {
                (true, toml_edit::Value::InlineTable(inline)) => {
                    toml_edit::Item::Table(inline.into_table())
                }
                (_, value) => toml_edit::Item::Value(value),
            };
            // An array of tables that becomes an inline array keeps the
            // comments above its headers.
            let mut above = None;
            if let (
                Some(toml_edit::Item::ArrayOfTables(tables)),
                toml_edit::Item::Value(toml_edit::Value::Array(array)),
                Value::Array(items),
            ) = (&old, &mut item, value)
            {
                above = tables
                    .get(0)
                    .map(|first| raw(first.decor().prefix()).to_owned());
                *array = tables_to_array(tables, items)?;
            }
            table.insert(key, item);
            if let (Some(above), Some(mut key)) = (above, table.key_mut(key)) {
                key.leaf_decor_mut().set_prefix(above);
            }
        }
    }
    Ok(())
}

/// The text of a decor part, which a parsed or built document holds.
fn raw(text: Option<&toml_edit::RawString>) -> &str {
    text.and_then(toml_edit::RawString::as_str)
        .unwrap_or_default()
}

/// `text` split after its first line break, into the end of a line and the
/// lines after it. Without a line break, all of it is the rest.
fn split_line(text: &str) -> (&str, &str) {
    text.find('\n')
        .map_or(("", text), |at| text.split_at(at + 1))
}

/// For each entry of `new`, the index of the entry of `old` whose comments
/// it keeps: an equal one no other entry keeps, wherever it was, else the
/// one at its own index, when no entry keeps that.
fn pick(old: &[Option<Value>], new: &[Value]) -> Vec<Option<usize>> {
    let mut kept = vec![false; old.len()];
    let mut picks: Vec<Option<usize>> = new
        .iter()
        .map(|entry| {
            let index = (0..old.len())
                .find(|index| !kept[*index] && old[*index].as_ref() == Some(entry))?;
            kept[index] = true;
            Some(index)
        })
        .collect();
    for index in 0..new.len().min(old.len()) {
        if picks[index].is_none() && !kept[index] {
            picks[index] = Some(index);
        }
    }
    picks
}

/// Brings inline `array`, which holds values, in line with `items`. A value
/// keeps the comment lines above it and the rest of its own line, as its
/// decor holds them, and every value the indent of its place. A new value
/// takes the layout of the last one, on a line of its own in a multi-line
/// array. `line` is the text before `[` on its line, when known.
fn reconcile_array(
    array: &mut toml_edit::Array,
    items: &[Value],
    line: Option<&str>,
) -> Result<(), String> {
    let old: Vec<toml_edit::Value> = array.iter().cloned().collect();
    let last = old.len().saturating_sub(1);
    let mut suffixes: Vec<String> = old
        .iter()
        .map(|value| raw(value.decor().suffix()).to_owned())
        .collect();
    // Without a trailing comma, the last value's suffix ends its line.
    let mut after = raw(Some(array.trailing())).to_owned();
    if !array.trailing_comma() {
        after = std::mem::take(&mut suffixes[last]) + &after;
    }
    // What follows `[`, the lines above each value, and the end of each
    // value's line.
    let prefixes: Vec<&str> = old
        .iter()
        .map(|value| raw(value.decor().prefix()))
        .collect();
    let (head, first) = split_line(prefixes[0]);
    let mut leads = vec![first];
    let mut tails = Vec::new();
    for prefix in &prefixes[1..] {
        let (tail, lead) = split_line(prefix);
        tails.push(tail);
        leads.push(lead);
    }
    let (tail, close) = split_line(&after);
    tails.push(tail);
    let comments = |lead: &str| lead.rfind('\n').map_or("", |at| &lead[..=at]).to_owned();
    // In a multi-line array, a new value and the one after it start their
    // own lines, as does a value after the old last one. A new last value
    // ends its line as the last one did.
    let multi_line = prefixes.iter().any(|prefix| prefix.contains('\n')) || after.contains('\n');
    let new_tail = if multi_line { "\n" } else { "" };
    // The first value's column, as blanks that keep the line's tabs.
    let column: Option<String> = line.map(|line| {
        let text = format!("{line}[{}", prefixes[0]);
        text.chars()
            .map(|c| if c == '\t' { c } else { ' ' })
            .collect()
    });
    let indent = |index: usize| match index > 0 && last == 0 && !prefixes[0].contains('\n') {
        // One value on the line of `[` has no separator to copy: on lines
        // of their own, the values after it line up with it.
        true => match (multi_line, &column) {
            (true, Some(column)) => column.clone(),
            _ => " ".to_owned(),
        },
        false => {
            let lead = leads[index.min(last)];
            let at = lead.rfind('\n').map_or(0, |at| at + 1);
            lead[at..].to_owned()
        }
    };
    let end = if tails[last].contains('\n') { "\n" } else { "" };
    let values: Vec<Option<Value>> = old.iter().map(|value| toml_value(value).ok()).collect();
    let picks = pick(&values, items);
    let new_last = matches!(picks.last(), Some(None));
    array.clear();
    let mut before = head.to_owned();
    let mut previous = None;
    for (index, (item, picked)) in items.iter().zip(picks).enumerate() {
        let mut value = match picked {
            Some(at) if values[at].as_ref() == Some(item) => old[at].clone(),
            _ => json_to_toml(item)?,
        };
        let starts_line = picked.is_none() || previous == Some(last);
        if index > 0 && starts_line && !before.ends_with('\n') {
            before += new_tail;
        }
        previous = picked;
        let lead = picked.map_or(String::new(), |at| comments(leads[at]));
        let decor = value.decor_mut();
        decor.set_prefix(before + &lead + &indent(index));
        decor.set_suffix(picked.map_or("", |at| suffixes[at].as_str()));
        array.push_formatted(value);
        before = picked.map_or(new_tail, |at| tails[at]).to_owned();
    }
    let trailing = match (items.is_empty(), new_last) {
        (true, _) => String::new(),
        (false, true) => end.to_owned() + close,
        (false, false) => before + close,
    };
    array.set_trailing(trailing);
    Ok(())
}

/// `items`, the entries of array of tables `tables`, as a multi-line inline
/// array. Each value keeps, as lines above it, the comments of the table it
/// was: those on its header line, between its keys, at the ends of their
/// lines and inside their arrays, and those above its header, except the
/// first table's, which go above the key.
fn tables_to_array(
    tables: &toml_edit::ArrayOfTables,
    items: &[Value],
) -> Result<toml_edit::Array, String> {
    let values: Vec<Option<Value>> = tables
        .iter()
        .map(|table| toml_table(table).ok().map(Value::Object))
        .collect();
    let mut array = toml_edit::Array::new();
    for (item, picked) in items.iter().zip(pick(&values, items)) {
        let mut texts = Vec::new();
        if let Some(at) = picked
            && let Some(table) = tables.get(at)
        {
            if at > 0 {
                texts.push(raw(table.decor().prefix()));
            }
            texts.push(raw(table.decor().suffix()));
            for (keys, value) in table.get_values() {
                let key = keys.last().and_then(|key| key.leaf_decor().prefix());
                texts.push(raw(key));
                nested_comments(value, &mut texts);
                texts.push(raw(value.decor().suffix()));
            }
        }
        let comments: String = texts
            .iter()
            .flat_map(|text| text.lines())
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| format!("\n  {line}"))
            .collect();
        let mut value = json_to_toml(item)?;
        value.decor_mut().set_prefix(format!("{comments}\n  "));
        value.decor_mut().set_suffix("");
        array.push_formatted(value);
    }
    if !array.is_empty() {
        array.set_trailing_comma(true);
        array.set_trailing("\n");
    }
    Ok(array)
}

/// Adds to `texts` the decor inside `value`, where an array holds the
/// comments between its values. An inline table holds none.
fn nested_comments<'a>(value: &'a toml_edit::Value, texts: &mut Vec<&'a str>) {
    if let toml_edit::Value::Array(array) = value {
        for item in array.iter() {
            texts.push(raw(item.decor().prefix()));
            nested_comments(item, texts);
            texts.push(raw(item.decor().suffix()));
        }
        texts.push(raw(Some(array.trailing())));
    }
}

/// Brings `item` in line with `value` in place, or returns false when it
/// must give way to a new key, as a table does to a plain value. `line` is
/// the text before the value on its line, when known.
fn reconcile_item(
    item: &mut toml_edit::Item,
    value: &Value,
    line: Option<&str>,
) -> Result<bool, String> {
    if toml_item(item).ok().flatten().as_ref() == Some(value) {
        return Ok(true);
    }
    match (&mut *item, value) {
        (toml_edit::Item::Table(table), Value::Object(map)) => {
            let lines = !table.is_dotted();
            reconcile(table, map, false, lines)?;
            return Ok(true);
        }
        (toml_edit::Item::Value(toml_edit::Value::InlineTable(table)), Value::Object(map)) => {
            reconcile(table, map, false, false)?;
            return Ok(true);
        }
        // An array of tables stays one while every entry is a table.
        // Empty, it would print as nothing and lose its key. Each entry
        // keeps the table of an equal one, with its comments.
        (toml_edit::Item::ArrayOfTables(tables), Value::Array(items))
            if !items.is_empty() && items.iter().all(Value::is_object) =>
        {
            let old: Vec<toml_edit::Table> = tables.iter().cloned().collect();
            let values: Vec<Option<Value>> = old
                .iter()
                .map(|table| toml_table(table).ok().map(Value::Object))
                .collect();
            let picks = pick(&values, items);
            // The file prints tables by position, so moved ones swap theirs.
            let mut positions: Vec<Option<isize>> = picks
                .iter()
                .flatten()
                .map(|at| old[*at].position())
                .collect();
            positions.sort();
            let mut positions = positions.into_iter();
            let mut next = toml_edit::ArrayOfTables::new();
            for (entry, picked) in items.iter().zip(picks) {
                let Value::Object(map) = entry else { continue };
                let mut table = picked.map_or_else(toml_edit::Table::new, |at| old[at].clone());
                if picked.is_some() {
                    table.set_position(positions.next().flatten());
                }
                reconcile(&mut table, map, false, true)?;
                next.push(table);
            }
            *tables = next;
            return Ok(true);
        }
        (toml_edit::Item::Value(toml_edit::Value::Array(array)), Value::Array(items))
            if !array.is_empty() =>
        {
            let line = line.map(|line| format!("{line}{}", raw(array.decor().prefix())));
            reconcile_array(array, items, line.as_deref())?;
            return Ok(true);
        }
        (toml_edit::Item::Value(old), _) => {
            // A replaced value keeps the comments around it.
            let mut next = json_to_toml(value)?;
            *next.decor_mut() = old.decor().clone();
            *old = next;
            return Ok(true);
        }
        _ => {}
    }
    Ok(false)
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
    /// Files that failed to load, by scope. They read as empty and are never
    /// written, so a broken file is not overwritten.
    broken: [bool; 2],
    errors: Vec<String>,
    /// A folder read that holds both settings files.
    conflict: Option<String>,
}

fn read_document(path: &Path) -> Result<Map<String, Value>, SettingsError> {
    if is_toml(path) {
        return toml_settings(path, &read_toml_document(path)?.0);
    }
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
/// inherited one. A list of only `+name` and `-name` entries is appended to it.
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

/// The global `httpProxy` setting in `agent_dir`, when its settings file
/// reads and sets one.
pub fn http_proxy(agent_dir: &Path) -> Option<String> {
    let path = settings_file(agent_dir).ok()?;
    read_document(&path)
        .ok()?
        .get("httpProxy")?
        .as_str()
        .map(str::to_owned)
}

/// pi's `applyHttpProxySettings`: the global `httpProxy` setting stands in
/// for unset proxy variables, for yapi and for chest alike.
pub fn apply_http_proxy() {
    let proxy = http_proxy(&crate::config::agent_dir());
    yapi_ai::http::set_settings_proxy(proxy.as_deref());
}

impl SettingsManager {
    /// Loads both scopes, the project's from `project_dir`, and that only
    /// when `project_trusted`.
    /// A folder that holds both `settings.json` and `settings.toml` loads as
    /// empty, is never written and sets [`SettingsManager::conflict`].
    pub fn load(agent_dir: &Path, project_dir: &Path, project_trusted: bool) -> SettingsManager {
        let project_folder = project_dir.join(PROJECT_DIR);
        let mut conflict = None;
        let mut broken = [false; 2];
        // An untrusted project's folder is never read, so it never conflicts.
        let mut locate = |dir: &Path, scope: usize, read: bool| {
            let json = dir.join(ConfigFile::Settings.file_name());
            match read.then(|| settings_file(dir)) {
                Some(Ok(path)) => path,
                Some(Err(err)) => {
                    conflict.get_or_insert(err.to_string());
                    broken[scope] = true;
                    json
                }
                None => json,
            }
        };
        let global_path = locate(agent_dir, 0, true);
        let project_path = locate(&project_folder, 1, project_trusted);
        let mut errors = Vec::new();
        // pi reports a file that fails to load and continues without it.
        let mut read = |path: &Path, scope: usize| match read_document(path) {
            _ if broken[scope] => Map::new(),
            Ok(document) => document,
            Err(error) => {
                let message = match error {
                    SettingsError::Parse { message, .. } => message,
                    SettingsError::Io { source, .. } => source.to_string(),
                    SettingsError::Conflict { .. } => error.to_string(),
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
            conflict,
        };
        manager.remerge();
        manager
    }

    /// The file that holds `scope`'s settings: `settings.json` or
    /// `settings.toml`.
    pub fn file(&self, scope: Scope) -> &Path {
        match scope {
            Scope::Global => &self.global_path,
            Scope::Project => &self.project_path,
        }
    }

    /// Why the settings must not be used: a folder they read holds both
    /// `settings.json` and `settings.toml`. Commands stop with it.
    pub fn conflict(&self) -> Option<&str> {
        self.conflict.as_deref()
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
        // keeps the rest of its object. `null` reads as unset.
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
    /// number, a numeric string or `"disabled"` (0), five minutes when unset.
    pub fn http_idle_timeout_ms(&self) -> Result<u64, String> {
        Ok(self.timeout_ms("httpIdleTimeoutMs")?.unwrap_or(300_000))
    }

    /// pi's `getWebSocketConnectTimeoutMs`: `websocketConnectTimeoutMs`, read
    /// as `httpIdleTimeoutMs` is, or `None` when unset.
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
    /// number, a numeric string or `"disabled"` (0). `None` when unset, and
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
    /// only, because each refresh costs money, defaulting to `streaming` when
    /// unset or invalid.
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

    /// The settings document of `scope` as written, empty for an untrusted
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
            // A TOML file is read once, as a document that keeps its
            // comments and layout.
            let mut toml = is_toml(path)
                .then(|| read_toml_document(path))
                .transpose()?;
            let mut current = match &toml {
                Some((document, _)) => toml_settings(path, document)?,
                None => read_document(path)?,
            };
            change(&mut current);
            let parse_error = |message: String| SettingsError::Parse {
                path: path.clone(),
                message,
            };
            let text = match &mut toml {
                Some((document, text)) => {
                    // Without keys, a file's comments are its trailing
                    // text, which new keys would come before.
                    let leading = document
                        .is_empty()
                        .then(|| raw(Some(document.trailing())).to_owned());
                    reconcile(document.as_table_mut(), &current, true, true)
                        .map_err(parse_error)?;
                    if let Some(mut leading) = leading.filter(|_| !document.is_empty()) {
                        if !leading.is_empty() && !leading.ends_with('\n') {
                            leading.push('\n');
                        }
                        document.decor_mut().set_prefix(leading);
                        document.set_trailing("");
                    }
                    as_read(text, document.to_string())
                }
                None => ConfigFile::Settings
                    .render(&Value::Object(current.clone()))
                    .map_err(|err| parse_error(err.to_string()))?,
            };
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
        // chest and yapi read the proxy from the same file.
        std::fs::write(&file, r#"{"httpProxy": "http://proxy:8080"}"#).unwrap();
        assert_eq!(http_proxy(&dir).as_deref(), Some("http://proxy:8080"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(http_proxy(&dir), None);
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
        // failed response. As its check before a prompt throws it, the next
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
        // A whole float is its integer. pi rounds a fraction down where it
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

    /// `settings.toml` holds what `settings.json` holds, and writes change
    /// only what changed, keeping comments and layout.
    #[test]
    fn toml_settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("yapi-settings-toml-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let toml = dir.join("settings.toml");
        let text = "# The model.\ndefaultModel = \"m\" # inline\nqueueMode = \"all\"\nenabledModels = [\"a\"]\n\n[compaction]\n# Kept.\nenabled = true\nreserveTokens = 100\n\n[modelThinkingLevels]\n\"p/a\" = \"high\"\n\"p/b\" = \"low\"\n\n[[packages]]\nsource = \"git:github.com/acme/skills@v2\"\nskills = [\"!legacy/**\"]\n\n[projectOnly]\nthemes = [\"*\"]\n";
        std::fs::write(&toml, text).unwrap();
        let json = json!({
            "defaultModel": "m",
            "enabledModels": ["a"],
            "compaction": {"enabled": true, "reserveTokens": 100},
            "modelThinkingLevels": {"p/a": "high", "p/b": "low"},
            "packages": [{"source": "git:github.com/acme/skills@v2", "skills": ["!legacy/**"]}],
            "projectOnly": {"themes": ["*"]},
            "steeringMode": "all",
        });
        let mut manager = SettingsManager::load(&dir, &dir.join("cwd"), false);
        assert_eq!(manager.file(Scope::Global), toml);
        assert_eq!(Value::Object(manager.document(Scope::Global).clone()), json);

        // A nested change, a removal, a nested removal and the migration of
        // `queueMode` leave the rest of the file as it was.
        manager
            .set_nested(
                Scope::Global,
                "compaction",
                "reserveTokens",
                Some(json!(200)),
            )
            .unwrap();
        manager.set(Scope::Global, "enabledModels", None).unwrap();
        manager
            .set_nested(Scope::Global, "modelThinkingLevels", "p/b", None)
            .unwrap();
        // An object entry changes in place, keeping the array of tables.
        let changed =
            json!([{"source": "git:github.com/acme/skills@v3", "skills": ["!legacy/**"]}]);
        manager
            .set(Scope::Global, "packages", Some(changed))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&toml).unwrap(),
            "# The model.\ndefaultModel = \"m\" # inline\nsteeringMode = \"all\"\n\n[compaction]\n# Kept.\nenabled = true\nreserveTokens = 200\n\n[modelThinkingLevels]\n\"p/a\" = \"high\"\n\n[[packages]]\nsource = \"git:github.com/acme/skills@v3\"\nskills = [\"!legacy/**\"]\n\n[projectOnly]\nthemes = [\"*\"]\n"
        );
        // A plain source makes `packages` a multi-line inline array, and
        // `null` removes a key, while one in an array and an integer TOML
        // can't hold fail.
        let mixed = json!(["npm:x", {"source": "git:github.com/acme/skills@v3"}]);
        manager
            .set(Scope::Global, "packages", Some(mixed.clone()))
            .unwrap();
        manager
            .set(Scope::Global, "projectOnly", Some(Value::Null))
            .unwrap();
        let reread = SettingsManager::load(&dir, &dir.join("cwd"), false);
        assert_eq!(reread.document(Scope::Global)["packages"], mixed);
        assert!(!reread.document(Scope::Global).contains_key("projectOnly"));
        assert_eq!(
            std::fs::read_to_string(&toml).unwrap(),
            "# The model.\ndefaultModel = \"m\" # inline\nsteeringMode = \"all\"\n\npackages = [\n  \"npm:x\",\n  { source = \"git:github.com/acme/skills@v3\" },\n]\n\n[compaction]\n# Kept.\nenabled = true\nreserveTokens = 200\n\n[modelThinkingLevels]\n\"p/a\" = \"high\"\n"
        );
        let null = manager.set(Scope::Global, "list", Some(json!([null])));
        assert!(null.is_err(), "{null:?}");
        let big = manager.set(Scope::Global, "big", Some(json!(u64::MAX)));
        assert!(big.is_err(), "{big:?}");

        // Each kept entry of `packages` keeps its comments and line breaks,
        // whether a table or a value, and a new one the layout of the last.
        let themes = r#"{ source = "npm:@acme/pi-themes", skills = [] }"#;
        let tables = "theme = \"g\"\n\n# Themes from npm, skills off.\n[[packages]] # shared\nsource = \"npm:@acme/pi-themes\" # pinned by the team\n# No skills.\nskills = []\n\n# My local tools.\n[[packages]]\nsource = \"../pkgs/local\"\nprompts = []\n";
        let inline = format!("packages = [\n  \"../pkgs/local\", # local tools\n  {themes},\n]\n");
        let commented =
            "packages = [\n  # First.\n  \"a\", # one\n  \"b\", # two\n  \"c\" # three\n]\n";
        for (text, packages, expected) in [
            // Removing a table takes its own comments along.
            (
                tables,
                json!([{"source": "../pkgs/local", "prompts": []}]),
                "theme = \"g\"\n\n# My local tools.\n[[packages]]\nsource = \"../pkgs/local\"\nprompts = []\n".to_owned(),
            ),
            // Tables that swap keep their comments, in the new order.
            (
                tables,
                json!([{"source": "../pkgs/local", "prompts": []}, {"source": "npm:@acme/pi-themes", "skills": []}]),
                "theme = \"g\"\n\n# My local tools.\n[[packages]]\nsource = \"../pkgs/local\"\nprompts = []\n\n# Themes from npm, skills off.\n[[packages]] # shared\nsource = \"npm:@acme/pi-themes\" # pinned by the team\n# No skills.\nskills = []\n".to_owned(),
            ),
            (
                &inline,
                json!(["../pkgs/local", {"source": "npm:@acme/pi-themes", "skills": []}, "../pkgs/new"]),
                format!("packages = [\n  \"../pkgs/local\", # local tools\n  {themes},\n  \"../pkgs/new\",\n]\n"),
            ),
            (
                &inline,
                json!([{"source": "npm:@acme/pi-themes", "skills": []}]),
                format!("packages = [\n  {themes},\n]\n"),
            ),
            (
                commented,
                json!(["a", "c", "d"]),
                "packages = [\n  # First.\n  \"a\", # one\n  \"c\", # three\n  \"d\"\n]\n".to_owned(),
            ),
            (
                "packages = [\"a\", \"b\"] # kept\n",
                json!(["b", "c"]),
                "packages = [\"b\", \"c\"] # kept\n".to_owned(),
            ),
            // A plain source makes the tables a multi-line array, with the
            // first table's comments above the key and the others' on their
            // values, as are those inside each table.
            (
                tables,
                json!([{"source": "npm:@acme/pi-themes", "skills": []}, {"source": "../pkgs/local", "prompts": []}, "../pkgs/new"]),
                format!("theme = \"g\"\n\n# Themes from npm, skills off.\npackages = [\n  # shared\n  # pinned by the team\n  # No skills.\n  {themes},\n  # My local tools.\n  {{ source = \"../pkgs/local\", prompts = [] }},\n  \"../pkgs/new\",\n]\n"),
            ),
            // So are the comments inside a table's arrays.
            (
                "[[packages]]\nsource = \"../pkgs/local\"\nskills = [\n  # keep this\n  \"lpskill\", # and this\n]\n",
                json!([{"source": "../pkgs/local", "skills": ["lpskill"]}, "../pkgs/new"]),
                "packages = [\n  # keep this\n  # and this\n  { source = \"../pkgs/local\", skills = [\"lpskill\"] },\n  \"../pkgs/new\",\n]\n".to_owned(),
            ),
            // A removed first table leaves no blank line at the start.
            (
                "[[packages]]\nsource = \"npm:x\"\n\n# Kept.\n[[packages]]\nsource = \"npm:y\"\n",
                json!([{"source": "npm:y"}]),
                "# Kept.\n[[packages]]\nsource = \"npm:y\"\n".to_owned(),
            ),
            // One value on one line has no separator to copy.
            (
                "packages = [\"npm:a\"]\n",
                json!(["npm:a", "../pkg2"]),
                "packages = [\"npm:a\", \"../pkg2\"]\n".to_owned(),
            ),
            // In a multi-line array, a new value and the one after it start
            // their own lines, even where `]` follows the last value.
            (
                "packages = [\"a\",\n            \"b\"]\n",
                json!(["a", "b", "c"]),
                "packages = [\"a\",\n            \"b\",\n            \"c\"]\n".to_owned(),
            ),
            (
                "packages = [\n  \"a\"]\n",
                json!(["z", "a", "c"]),
                "packages = [\n  \"z\",\n  \"a\",\n  \"c\"]\n".to_owned(),
            ),
            // On lines of their own, values after one on the line of `[`
            // line up with it.
            (
                "packages = [\"a\",\n]\n",
                json!(["a", "c"]),
                "packages = [\"a\",\n            \"c\",\n]\n".to_owned(),
            ),
            (
                "packages = [\"a\" # one\n]\n",
                json!(["a", "c"]),
                "packages = [\"a\", # one\n            \"c\"\n]\n".to_owned(),
            ),
            // A value that moves keeps its comments, and one after the old
            // last value starts its own line.
            (
                "packages = [\"a\", # main\n            \"b\"]\n",
                json!(["b", "a"]),
                "packages = [\"b\",\n            \"a\" # main\n]\n".to_owned(),
            ),
            (
                "packages = [\"a\", # main\n            \"b\", # two\n            \"c\"]\n",
                json!(["b", "a", "c"]),
                "packages = [\"b\", # two\n            \"a\", # main\n            \"c\"]\n".to_owned(),
            ),
            // New keys follow the comments of a file without keys.
            (
                "# only a comment\n",
                json!(["npm:x"]),
                "# only a comment\npackages = [\"npm:x\"]\n".to_owned(),
            ),
            (
                "# no line break",
                json!(["npm:x"]),
                "# no line break\npackages = [\"npm:x\"]\n".to_owned(),
            ),
            // A file with a byte order mark keeps it.
            (
                "\u{feff}packages = [\"a\"]\n",
                json!(["a", "b"]),
                "\u{feff}packages = [\"a\", \"b\"]\n".to_owned(),
            ),
            // A file with CRLF line endings keeps them.
            (
                "# Mine.\r\npackages = [\r\n  \"a\", # one\r\n]\r\ntheme = \"g\"\r\n",
                json!(["a", "b"]),
                "# Mine.\r\npackages = [\r\n  \"a\", # one\r\n  \"b\",\r\n]\r\ntheme = \"g\"\r\n".to_owned(),
            ),
            // An array of tables emptied keeps its key, as `packages = []`,
            // and the comments above it.
            (
                "theme = \"g\"\n\n# The only package.\n[[packages]]\nsource = \"npm:x\"\n",
                json!([]),
                "theme = \"g\"\n\n# The only package.\npackages = []\n".to_owned(),
            ),
        ] {
            std::fs::write(&toml, text).unwrap();
            let mut manager = SettingsManager::load(&dir, &dir.join("cwd"), false);
            manager
                .set(Scope::Global, "packages", Some(packages.clone()))
                .unwrap();
            assert_eq!(std::fs::read_to_string(&toml).unwrap(), expected, "{text}");
            let reread = SettingsManager::load(&dir, &dir.join("cwd"), false);
            assert_eq!(reread.document(Scope::Global)["packages"], packages);
        }

        // A date is no setting, so the file is reported and never written.
        std::fs::write(&toml, "theme = 1979-05-27\n").unwrap();
        let mut dated = SettingsManager::load(&dir, &dir.join("cwd"), false);
        assert!(
            dated.errors()[0].contains("is a date or time"),
            "{:?}",
            dated.errors()
        );
        dated
            .set(Scope::Global, "theme", Some(json!("dark")))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&toml).unwrap(),
            "theme = 1979-05-27\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A folder with both settings files loads as empty, is never written
    /// and reports the conflict, unless it is an untrusted project's.
    #[test]
    fn both_files_conflict() {
        let dir = std::env::temp_dir().join(format!("yapi-settings-both-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let project = dir.join("cwd");
        for folder in [dir.clone(), project.join(PROJECT_DIR)] {
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join("settings.json"), "{\"theme\": \"dark\"}").unwrap();
            std::fs::write(folder.join("settings.toml"), "theme = \"light\"\n").unwrap();
        }
        let untrusted = SettingsManager::load(&dir, &project, false);
        let conflict = untrusted.conflict().unwrap();
        assert_eq!(
            conflict,
            format!(
                "Both settings.json and settings.toml are in {}. Keep one of them.",
                dir.display()
            )
        );
        std::fs::remove_file(dir.join("settings.toml")).unwrap();
        assert_eq!(
            SettingsManager::load(&dir, &project, false).conflict(),
            None
        );
        let mut trusted = SettingsManager::load(&dir, &project, true);
        assert!(trusted.conflict().unwrap().contains(".yapi"));
        assert!(trusted.document(Scope::Project).is_empty());
        trusted
            .set(Scope::Project, "theme", Some(json!("x")))
            .unwrap();
        let toml = project.join(PROJECT_DIR).join("settings.toml");
        assert_eq!(
            std::fs::read_to_string(toml).unwrap(),
            "theme = \"light\"\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
