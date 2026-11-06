//! `yapi config`: turns the resources of packages, settings entries and the
//! agent and project directories on and off, globally or as project
//! overrides.
//!
//! Port of `components/config-selector.ts` in
//! `packages/coding-agent/src/modes/interactive` in pi `v1.0.0`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui_core::style::Modifier;
use ratatui_core::text::{Line, Span};
use serde_json::{Map, Value};
use yapi_core::packages::resolve::{
    BUILTIN_PREFIX, ProjectOnly, ResolvedPaths, ResourceType, string_list,
};
use yapi_core::packages::source::{is_local, local_path};
use yapi_core::packages::{entry_source, scope_dir, settings_packages};
use yapi_core::settings::{Scope, SettingsManager};
use yapi_core::tools::path::{home_dir, relative};
use yapi_tui::lines::{self, StyledLine, styled};
use yapi_tui::select_list::visible_range;
use yapi_tui::text::visible_width;
use yapi_tui::text_input::TextInput;
use yapi_types::rpc::SourceInfo;

use super::selectors::{Outcome, Ui, key_hint};

fn label(kind: ResourceType) -> &'static str {
    match kind {
        ResourceType::Extensions => "Extensions",
        ResourceType::Skills => "Skills",
        ResourceType::Prompts => "Prompts",
        ResourceType::Themes => "Themes",
    }
}

#[derive(Clone, Debug)]
struct Item {
    path: String,
    enabled: bool,
    info: SourceInfo,
    kind: ResourceType,
    name: String,
}

#[derive(Clone, Debug)]
struct Subgroup {
    kind: ResourceType,
    items: Vec<Item>,
}

#[derive(Clone, Debug)]
struct Group {
    label: String,
    scope: String,
    origin: String,
    source: String,
    subgroups: Vec<Subgroup>,
}

/// A row: a group header, a type header or an item, by index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Group(usize),
    Subgroup(usize, usize),
    Item(usize, usize, usize),
}

/// The state a project override gives a resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Override {
    Inherit,
    Load,
    Unload,
}

/// pi's `formatBaseDir`: `~` for the home directory, with a trailing `/`.
fn format_base_dir(dir: &str, home: &str) -> String {
    let shown = if dir == home {
        "~".to_owned()
    } else if !home.is_empty() && dir.starts_with(home) {
        format!("~{}", dir[home.len()..].replace('\\', "/"))
    } else {
        dir.replace('\\', "/")
    };
    if shown.ends_with('/') {
        shown
    } else {
        format!("{shown}/")
    }
}

/// Whether chest manages the resource `info` describes: its manifest turns
/// it on and off, and settings cannot.
fn managed(info: &SourceInfo) -> bool {
    info.source.starts_with("chest:")
}

/// pi's `getGroupLabel`, and chest's packages as chest manages them.
fn group_label(info: &SourceInfo, agent_dir: &Path, home: &str) -> String {
    if managed(info) {
        return format!("{} ({}, managed by chest)", info.source, info.scope);
    }
    if info.origin == "package" {
        return format!("{} ({})", info.source, info.scope);
    }
    if info.source == "builtin" {
        return if info.scope == "user" {
            "Built-in".to_owned()
        } else {
            "Built-in (project override)".to_owned()
        };
    }
    if info.source == "auto" {
        return match &info.base_dir {
            Some(base) if info.scope == "user" => format!("User ({})", format_base_dir(base, home)),
            Some(base) => format!("Project ({})", format_base_dir(base, home)),
            None if info.scope == "user" => format!(
                "User ({})",
                format_base_dir(&agent_dir.to_string_lossy(), home)
            ),
            None => format!("Project ({}/)", yapi_core::config::PROJECT_DIR),
        };
    }
    if info.scope == "user" {
        "User settings".to_owned()
    } else {
        "Project settings".to_owned()
    }
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn parent_name(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// pi's `buildGroups`: resources grouped by where they come from, packages
/// first and the user before the project, then by type and name.
fn build_groups(resolved: &ResolvedPaths, agent_dir: &Path, home: &str) -> Vec<Group> {
    let mut keys: Vec<String> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    for kind in ResourceType::ALL {
        for resource in resolved.of(kind) {
            let info = &resource.info;
            let key = format!(
                "{}:{}:{}:{}",
                info.origin,
                info.scope,
                info.source,
                info.base_dir.clone().unwrap_or_default()
            );
            let index = match keys.iter().position(|existing| *existing == key) {
                Some(index) => index,
                None => {
                    keys.push(key);
                    groups.push(Group {
                        label: group_label(info, agent_dir, home),
                        scope: info.scope.clone(),
                        origin: info.origin.clone(),
                        source: info.source.clone(),
                        subgroups: Vec::new(),
                    });
                    groups.len() - 1
                }
            };
            let group = &mut groups[index];
            let sub = match group.subgroups.iter().position(|sub| sub.kind == kind) {
                Some(sub) => sub,
                None => {
                    group.subgroups.push(Subgroup {
                        kind,
                        items: Vec::new(),
                    });
                    group.subgroups.len() - 1
                }
            };
            let path = &info.path;
            let name = if info.source == "builtin" {
                path.strip_prefix(BUILTIN_PREFIX).unwrap_or(path).to_owned()
            } else if kind == ResourceType::Extensions && parent_name(path) != "extensions" {
                format!("{}/{}", parent_name(path), file_name(path))
            } else if kind == ResourceType::Skills && file_name(path) == "SKILL.md" {
                parent_name(path)
            } else {
                file_name(path)
            };
            group.subgroups[sub].items.push(Item {
                path: path.clone(),
                enabled: resource.enabled,
                info: info.clone(),
                kind,
                name,
            });
        }
    }
    let collate = yapi_types::collate::locale_compare;
    groups.sort_by(|a, b| {
        (a.origin != "package")
            .cmp(&(b.origin != "package"))
            .then_with(|| (a.scope != "user").cmp(&(b.scope != "user")))
            .then_with(|| collate(&a.source, &b.source))
    });
    for group in &mut groups {
        group.subgroups.sort_by_key(|sub| sub.kind as usize);
        for sub in &mut group.subgroups {
            sub.items.sort_by(|a, b| collate(&a.name, &b.name));
        }
    }
    groups
}

fn canonical(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_owned())
}

fn item_key(item: &Item) -> String {
    format!("{}:{}", item.kind.key(), canonical(&item.path))
}

fn strip_sign(entry: &str) -> &str {
    if entry.starts_with(['!', '+', '-']) {
        &entry[1..]
    } else {
        entry
    }
}

/// The `yapi config` selector.
pub struct ConfigSelector {
    groups: [Vec<Group>; 2],
    inherited: HashMap<String, bool>,
    rows: Vec<Row>,
    filtered: Vec<Row>,
    selected: usize,
    input: TextInput,
    max_visible: usize,
    settings: SettingsManager,
    cwd: PathBuf,
    agent_dir: PathBuf,
    project: bool,
    project_available: bool,
}

impl ConfigSelector {
    /// A selector over the global and project resolutions, writing to
    /// `settings`, starting in project mode when `project` is set.
    #[allow(
        clippy::too_many_arguments,
        reason = "mirrors pi's ConfigSelectorComponent constructor"
    )]
    pub fn new(
        global: &ResolvedPaths,
        project_resolved: &ResolvedPaths,
        settings: SettingsManager,
        cwd: PathBuf,
        agent_dir: PathBuf,
        terminal_rows: usize,
        project: bool,
        project_available: bool,
    ) -> ConfigSelector {
        let home = super::home_dir()
            .map(|home| home.to_string_lossy().into_owned())
            .unwrap_or_default();
        let groups = [
            build_groups(global, &agent_dir, &home),
            build_groups(project_resolved, &agent_dir, &home),
        ];
        let mut inherited = HashMap::new();
        for group in &groups[0] {
            for sub in &group.subgroups {
                for item in &sub.items {
                    inherited.insert(item_key(item), item.enabled);
                }
            }
        }
        let mut selector = ConfigSelector {
            groups,
            inherited,
            rows: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            input: TextInput::default(),
            max_visible: terminal_rows.saturating_sub(8).max(5),
            settings,
            cwd,
            agent_dir,
            project,
            project_available,
        };
        selector.build_rows();
        selector.filtered = selector.rows.clone();
        selector
    }

    fn groups(&self) -> &Vec<Group> {
        &self.groups[usize::from(self.project)]
    }

    fn item(&self, row: Row) -> Option<&Item> {
        match row {
            Row::Item(group, sub, item) => Some(&self.groups()[group].subgroups[sub].items[item]),
            _ => None,
        }
    }

    fn build_rows(&mut self) {
        let mut rows = Vec::new();
        for (g, group) in self.groups().iter().enumerate() {
            rows.push(Row::Group(g));
            for (s, sub) in group.subgroups.iter().enumerate() {
                rows.push(Row::Subgroup(g, s));
                for i in 0..sub.items.len() {
                    rows.push(Row::Item(g, s, i));
                }
            }
        }
        self.rows = rows;
        self.selected = self
            .rows
            .iter()
            .position(|row| matches!(row, Row::Item(..)))
            .unwrap_or(0);
    }

    fn select_first_item(&mut self) {
        self.selected = self
            .filtered
            .iter()
            .position(|row| matches!(row, Row::Item(..)))
            .unwrap_or(0);
    }

    fn filter(&mut self) {
        let query = self.input.value().trim().to_lowercase();
        if query.is_empty() {
            self.filtered = self.rows.clone();
            self.select_first_item();
            return;
        }
        let matches = |item: &Item| {
            item.name.to_lowercase().contains(&query)
                || item.kind.key().contains(&query)
                || item.path.to_lowercase().contains(&query)
        };
        let groups = self.groups();
        self.filtered = self
            .rows
            .iter()
            .copied()
            .filter(|row| match *row {
                Row::Item(..) => self.item(*row).is_some_and(matches),
                Row::Subgroup(g, s) => groups[g].subgroups[s].items.iter().any(matches),
                Row::Group(g) => groups[g]
                    .subgroups
                    .iter()
                    .any(|sub| sub.items.iter().any(matches)),
            })
            .collect();
        self.select_first_item();
    }

    fn next_item(&self, from: usize, forward: bool) -> usize {
        let mut index = from;
        loop {
            index = match (forward, index) {
                (true, index) if index + 1 < self.filtered.len() => index + 1,
                (false, index) if index > 0 => index - 1,
                _ => return from,
            };
            if matches!(self.filtered[index], Row::Item(..)) {
                return index;
            }
        }
    }

    fn item_scope(item: &Item) -> Scope {
        if item.info.scope == "project" {
            Scope::Project
        } else {
            Scope::Global
        }
    }

    fn base(&self, scope: Scope) -> PathBuf {
        scope_dir(scope, &self.cwd, &self.agent_dir)
    }

    fn packages(&self, scope: Scope) -> Vec<Value> {
        settings_packages(&self.settings, scope)
    }

    fn write(&mut self, scope: Scope, key: &str, value: Option<Value>) {
        let _ = self.settings.set(scope, key, value);
    }

    /// Whether a project-only run keeps global `item`, by the `projectOnly`
    /// setting as it stands.
    fn kept(&self, item: &Item) -> bool {
        let keep = ProjectOnly::from_settings(self.settings.document(Scope::Global));
        Self::item_scope(item) == Scope::Global
            && item.info.source != "builtin"
            && keep.keeps(&item.info, item.kind, &self.agent_dir, &home_dir())
    }

    /// How `ctrl+o` names `item` in `projectOnly`: relative to the agent
    /// folder inside it, else from `~` inside the home folder, else whole. A
    /// skill folder goes by the folder, a single-file skill by its file.
    fn keep_path(&self, item: &Item) -> String {
        let path = Path::new(&item.info.path);
        let path = match (item.kind, path.file_name()) {
            (ResourceType::Skills, Some(name)) if name == "SKILL.md" => {
                path.parent().unwrap_or(path)
            }
            _ => path,
        };
        let home = home_dir();
        match (path.strip_prefix(&self.agent_dir), path.strip_prefix(&home)) {
            (Ok(inside), _) => inside.to_string_lossy().replace('\\', "/"),
            (_, Ok(inside)) => format!("~/{}", inside.to_string_lossy().replace('\\', "/")),
            _ => path.to_string_lossy().into_owned(),
        }
    }

    /// `ctrl+o` in global mode: keeps the selected global resource in
    /// project-only runs by its exact path, or stops keeping it. A built-in,
    /// or a resource that a pattern or `packages` keeps, stays as it is.
    fn toggle_keep(&mut self) {
        // A project's `projectOnly` would be ignored.
        if self.project {
            return;
        }
        let row = self.filtered.get(self.selected).copied();
        let Some(Row::Item(g, s, i)) = row else {
            return;
        };
        let item = self.groups()[g].subgroups[s].items[i].clone();
        if Self::item_scope(&item) != Scope::Global || item.info.source == "builtin" {
            return;
        }
        let path = self.keep_path(&item);
        let global = self.settings.document(Scope::Global);
        let mut setting = global
            .get("projectOnly")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let key = item.kind.key();
        let mut list = string_list(&setting, key);
        match list.iter().position(|entry| *entry == path) {
            Some(index) => {
                list.remove(index);
            }
            None if !self.kept(&item) => list.push(path),
            None => return,
        }
        // An emptied list goes, and so does the setting once nothing is left.
        match list.is_empty() {
            true => setting.remove(key),
            false => setting.insert(key.to_owned(), Value::from(list)),
        };
        let setting = (!setting.is_empty()).then_some(Value::Object(setting));
        self.write(Scope::Global, "projectOnly", setting);
    }

    /// A package entry as an object. A bare source becomes `{"source": ...}`.
    fn package_object(entry: &Value) -> Option<Map<String, Value>> {
        match entry {
            Value::String(_) => Some(Map::from_iter([("source".to_owned(), entry.clone())])),
            Value::Object(object) => Some(object.clone()),
            _ => None,
        }
    }

    /// Drops `pattern`'s entries from `list`, then adds it with `sign`.
    fn set_pattern(list: &mut Vec<String>, pattern: &str, sign: Option<&str>) {
        list.retain(|entry| strip_sign(entry) != pattern);
        list.extend(sign.map(|sign| format!("{sign}{pattern}")));
    }

    /// pi's `getPackageResourcePattern`.
    fn package_pattern(item: &Item) -> String {
        let base = item.info.base_dir.as_ref().map_or_else(
            || {
                Path::new(&item.path)
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default()
            },
            PathBuf::from,
        );
        relative(&base, Path::new(&item.path))
    }

    /// pi's `toggleResource` in global mode.
    fn toggle_global(&mut self, item: &Item) -> bool {
        let enabled = !item.enabled;
        let scope = Self::item_scope(item);
        let key = item.kind.key();
        let sign = Some(if enabled { "+" } else { "-" });
        if item.info.origin == "top-level" {
            // pi's `getResourcePattern`: the pattern in the item's own scope.
            let mut list = string_list(self.settings.document(scope), key);
            Self::set_pattern(&mut list, &self.pattern_for_scope(item, scope), sign);
            self.write(scope, key, Some(Value::from(list)));
            return enabled;
        }
        let mut packages = self.packages(scope);
        let Some(index) = packages
            .iter()
            .position(|entry| entry_source(entry) == Some(item.info.source.as_str()))
        else {
            return enabled;
        };
        let Some(mut entry) = Self::package_object(&packages[index]) else {
            return enabled;
        };
        let mut list = string_list(&entry, key);
        Self::set_pattern(&mut list, &Self::package_pattern(item), sign);
        entry.insert(key.to_owned(), Value::from(list));
        packages[index] = Value::Object(entry);
        self.write(scope, "packages", Some(Value::Array(packages)));
        enabled
    }

    fn inherited_enabled(&self, item: &Item) -> bool {
        self.inherited
            .get(&item_key(item))
            .copied()
            .unwrap_or(Self::item_scope(item) == Scope::Project || item.enabled)
    }

    fn is_inherited_global(&self, item: &Item) -> bool {
        Self::item_scope(item) == Scope::Global || self.inherited.contains_key(&item_key(item))
    }

    /// pi's `getResourcePatternForScope`.
    fn pattern_for_scope(&self, item: &Item, scope: Scope) -> String {
        let source_scope = Self::item_scope(item);
        if scope != source_scope || item.info.source == "builtin" {
            return item.path.clone();
        }
        let base = item
            .info
            .base_dir
            .as_ref()
            .map_or_else(|| self.base(source_scope), PathBuf::from);
        relative(&base, Path::new(&item.path))
    }

    /// pi's `getTopLevelOverridePatterns`.
    fn top_level_patterns(&self, item: &Item) -> Vec<String> {
        let mut patterns = vec![
            self.pattern_for_scope(item, Scope::Project),
            item.path.clone(),
            relative(&self.base(Scope::Project), Path::new(&item.path)),
        ];
        if let Some(base) = &item.info.base_dir {
            patterns.push(relative(Path::new(base), Path::new(&item.path)));
        }
        patterns
    }

    fn source_matches(
        &self,
        left: &str,
        left_scope: Scope,
        right: &str,
        right_scope: Scope,
    ) -> bool {
        if left == right {
            return true;
        }
        if !is_local(left) || !is_local(right) {
            return false;
        }
        local_path(left, &self.base(left_scope)) == local_path(right, &self.base(right_scope))
    }

    fn matching_package(&self, item: &Item) -> Option<Value> {
        self.packages(Scope::Project).into_iter().find(|entry| {
            entry_source(entry).is_some_and(|source| {
                self.source_matches(
                    &item.info.source,
                    Self::item_scope(item),
                    source,
                    Scope::Project,
                )
            })
        })
    }

    fn state_from_entries(
        entries: &[String],
        patterns: &[String],
        empty_unloads: bool,
    ) -> Override {
        if entries.is_empty() && empty_unloads {
            return Override::Unload;
        }
        let mut state = Override::Inherit;
        for entry in entries {
            if !patterns.iter().any(|pattern| pattern == strip_sign(entry)) {
                continue;
            }
            state = if entry.starts_with(['!', '-']) {
                Override::Unload
            } else {
                Override::Load
            };
        }
        state
    }

    /// pi's `getProjectOverrideState`.
    fn override_state(&self, item: &Item) -> Override {
        if !self.project {
            return Override::Inherit;
        }
        if item.info.origin == "top-level" {
            let entries = string_list(self.settings.document(Scope::Project), item.kind.key());
            return Self::state_from_entries(&entries, &self.top_level_patterns(item), false);
        }
        let Some(Value::Object(entry)) = self.matching_package(item) else {
            return Override::Inherit;
        };
        let Some(list) = entry.get(item.kind.key()).and_then(Value::as_array) else {
            return Override::Inherit;
        };
        let entries: Vec<String> = list
            .iter()
            .filter_map(|entry| entry.as_str().map(str::to_owned))
            .collect();
        Self::state_from_entries(
            &entries,
            &[Self::package_pattern(item)],
            entry.get("autoload") != Some(&Value::Bool(false)),
        )
    }

    /// pi's `getNextOverrideState`.
    fn next_state(&self, item: &Item) -> Override {
        let inherited = self.inherited_enabled(item);
        match (self.override_state(item), inherited) {
            (Override::Inherit, true) => Override::Unload,
            (Override::Inherit, false) => Override::Load,
            (Override::Unload, true) => Override::Load,
            (Override::Unload, false) => Override::Inherit,
            (Override::Load, true) => Override::Inherit,
            (Override::Load, false) => Override::Unload,
        }
    }

    /// pi's `setProjectResourceOverride`: whether anything changed.
    fn set_override(&mut self, item: &Item, state: Override) -> bool {
        if item.info.origin == "top-level" {
            let key = item.kind.key();
            let inherited = self.is_inherited_global(item);
            let pattern = if inherited {
                item.path.clone()
            } else {
                self.pattern_for_scope(item, Scope::Project)
            };
            let patterns = self.top_level_patterns(item);
            let mut list = string_list(self.settings.document(Scope::Project), key);
            list.retain(|entry| {
                let target = strip_sign(entry);
                if entry.starts_with(['!', '+', '-'])
                    && patterns.iter().any(|pattern| pattern == target)
                {
                    return false;
                }
                !(state == Override::Inherit && inherited && target == pattern)
            });
            if state != Override::Inherit {
                if inherited && item.info.source != "builtin" && !list.contains(&pattern) {
                    list.push(pattern.clone());
                }
                let sign = if state == Override::Load { "+" } else { "-" };
                list.push(format!("{sign}{pattern}"));
            }
            self.write(Scope::Project, key, Some(Value::from(list)));
            return true;
        }
        let mut packages = self.packages(Scope::Project);
        let scope = Self::item_scope(item);
        let index = packages.iter().position(|entry| {
            entry_source(entry).is_some_and(|source| {
                self.source_matches(&item.info.source, scope, source, Scope::Project)
            })
        });
        let index = match index {
            Some(index) => index,
            None if state == Override::Inherit => return false,
            None => {
                let source = &item.info.source;
                let source = if is_local(source) {
                    let path = local_path(source, &self.base(scope));
                    let rel = relative(&self.base(Scope::Project), &path);
                    if rel.is_empty() { ".".to_owned() } else { rel }
                } else {
                    source.clone()
                };
                let mut entry = Map::new();
                entry.insert("source".into(), Value::String(source));
                entry.insert("autoload".into(), Value::Bool(false));
                packages.push(Value::Object(entry));
                packages.len() - 1
            }
        };
        let Some(mut entry) = Self::package_object(&packages[index]) else {
            return false;
        };
        let key = item.kind.key();
        let sign = match state {
            Override::Inherit => None,
            Override::Load => Some("+"),
            Override::Unload => Some("-"),
        };
        let mut list = string_list(&entry, key);
        Self::set_pattern(&mut list, &Self::package_pattern(item), sign);
        if list.is_empty() {
            entry.remove(key);
        } else {
            entry.insert(key.to_owned(), Value::from(list));
        }
        let filtered = ResourceType::ALL
            .iter()
            .any(|kind| entry.contains_key(kind.key()));
        if filtered {
            packages[index] = Value::Object(entry);
        } else if entry.get("autoload") == Some(&Value::Bool(false)) {
            packages.remove(index);
        } else {
            packages[index] = entry["source"].clone();
        }
        self.write(Scope::Project, "packages", Some(Value::Array(packages)));
        true
    }

    fn toggle(&mut self) {
        let row = self.filtered.get(self.selected).copied();
        let Some(Row::Item(g, s, i)) = row else {
            return;
        };
        let item = self.groups()[g].subgroups[s].items[i].clone();
        // A settings entry for it would never apply.
        if managed(&item.info) || !self.project && Self::item_scope(&item) != Scope::Global {
            return;
        }
        let enabled = if self.project {
            let state = self.next_state(&item);
            if !self.set_override(&item, state) {
                return;
            }
            match state {
                Override::Inherit => self.inherited_enabled(&item),
                Override::Load => true,
                Override::Unload => false,
            }
        } else {
            self.toggle_global(&item)
        };
        let scope = usize::from(self.project);
        self.groups[scope][g].subgroups[s].items[i].enabled = enabled;
    }

    fn switch_mode(&mut self) {
        self.project = !self.project;
        self.build_rows();
        self.filter();
    }

    fn checkbox(&self, item: &Item, ui: &Ui<'_>) -> Span<'static> {
        let theme = ui.theme;
        if self.project {
            return match self.override_state(item) {
                Override::Load => Span::styled("[+]", theme.fg("success")),
                Override::Unload => Span::styled("[-]", theme.fg("warning")),
                Override::Inherit => {
                    Span::styled(if item.enabled { "[x]" } else { "[ ]" }, theme.fg("dim"))
                }
            };
        }
        if item.enabled {
            Span::styled("[x]", theme.fg("success"))
        } else {
            Span::styled("[ ]", theme.fg("dim"))
        }
    }

    fn suffix(&self, item: &Item, ui: &Ui<'_>) -> Option<Span<'static>> {
        if !self.project {
            return self
                .kept(item)
                .then(|| Span::styled("  project-only", ui.theme.fg("muted")));
        }
        match self.override_state(item) {
            Override::Load => Some(Span::styled("  project load", ui.theme.fg("muted"))),
            Override::Unload => Some(Span::styled("  project unload", ui.theme.fg("muted"))),
            Override::Inherit => self
                .is_inherited_global(item)
                .then(|| Span::styled("  inherited global", ui.theme.fg("dim"))),
        }
    }

    fn header(&self, width: usize, ui: &Ui<'_>) -> Vec<StyledLine> {
        let theme = ui.theme;
        let title = if self.project {
            "Project Local Resources"
        } else {
            "Global Resources"
        };
        let mut parts = Vec::new();
        if self.project_available {
            parts.push(ui.key_hint("tui.input.tab", "switch mode"));
        }
        let space = if self.project {
            "cycle inherit/+/-"
        } else {
            "toggle"
        };
        parts.push(key_hint(theme, "space", space));
        if !self.project {
            parts.push(key_hint(theme, "ctrl+o", "keep in project-only"));
        }
        parts.push(key_hint(theme, "esc", "close"));
        let hint = parts.join(&Span::styled(" · ", theme.fg("muted")));
        let hint_width: usize = hint.iter().map(|span| visible_width(&span.content)).sum();
        let spacing = width
            .saturating_sub(visible_width(title) + hint_width)
            .max(1);
        let mut first = vec![
            Span::styled(
                title,
                ratatui_core::style::Style::new().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(spacing)),
        ];
        first.extend(hint);
        // The settings file each scope writes, `settings.json` or `settings.toml`.
        let file = |scope| {
            let path = self.settings.file(scope);
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            name.unwrap_or_else(|| "settings.json".to_owned())
        };
        let scope_hint = if self.project {
            format!(
                "{}/{} · inherited global resources are dimmed",
                yapi_core::config::PROJECT_DIR,
                file(Scope::Project)
            )
        } else {
            format!(
                "~/{}/agent/{}",
                yapi_core::config::PROJECT_DIR,
                file(Scope::Global)
            )
        };
        vec![
            lines::truncate(&Line::from(first), width, ""),
            lines::truncate(&styled(scope_hint, theme.fg("muted")), width, ""),
        ]
    }

    fn list_rows(&mut self, width: usize, ui: &Ui<'_>) -> Vec<StyledLine> {
        let theme = ui.theme;
        let mut out = vec![self.input.render(width), Line::default()];
        if self.filtered.is_empty() {
            out.push(styled("  No resources found", theme.fg("muted")));
            return out;
        }
        let count = self.filtered.len();
        let (start, end) = visible_range(self.selected, count, self.max_visible);
        let groups = self.groups();
        for index in start..end {
            let row = self.filtered[index];
            let line = match row {
                Row::Group(g) => {
                    let group = &groups[g];
                    let inherited = self.project && group.scope == "user";
                    let text = format!(
                        "{}{}",
                        group.label,
                        if inherited {
                            " · inherited global"
                        } else {
                            ""
                        }
                    );
                    let color = theme.fg(if inherited { "dim" } else { "accent" });
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(text, color.add_modifier(Modifier::BOLD)),
                    ])
                }
                Row::Subgroup(g, s) => {
                    let group = &groups[g];
                    let color = theme.fg(if self.project && group.scope == "user" {
                        "dim"
                    } else {
                        "muted"
                    });
                    Line::from(vec![
                        Span::raw("    "),
                        Span::styled(label(group.subgroups[s].kind), color),
                    ])
                }
                Row::Item(..) => {
                    let Some(item) = self.item(row) else {
                        continue;
                    };
                    let selected = index == self.selected;
                    let dimmed = self.project
                        && self.is_inherited_global(item)
                        && self.override_state(item) == Override::Inherit;
                    let mut name_style = ratatui_core::style::Style::new();
                    if selected && !dimmed {
                        name_style = name_style.add_modifier(Modifier::BOLD);
                    }
                    if dimmed {
                        name_style = theme.fg("dim").patch(name_style);
                    }
                    let mut spans = vec![
                        Span::raw(if selected { ">     " } else { "      " }),
                        self.checkbox(item, ui),
                        Span::raw(" "),
                        Span::styled(item.name.clone(), name_style),
                    ];
                    spans.extend(self.suffix(item, ui));
                    lines::truncate(&Line::from(spans), width, "...")
                }
            };
            out.push(match row {
                Row::Item(..) => line,
                _ => lines::truncate(&line, width, ""),
            });
        }
        if start > 0 || end < count {
            let items = self
                .filtered
                .iter()
                .filter(|row| matches!(row, Row::Item(..)))
                .count();
            let current = self.filtered[..self.selected]
                .iter()
                .filter(|row| matches!(row, Row::Item(..)))
                .count()
                + 1;
            out.push(styled(format!("  ({current}/{items})"), theme.fg("dim")));
        }
        out
    }

    /// The rows at `width`.
    pub fn render(&mut self, width: usize, ui: &Ui<'_>) -> Vec<StyledLine> {
        let mut out = lines::spacer(1);
        out.push(ui.border(width));
        out.extend(lines::spacer(1));
        out.extend(self.header(width, ui));
        out.extend(lines::spacer(1));
        out.extend(self.list_rows(width, ui));
        out.extend(lines::spacer(1));
        out.push(ui.border(width));
        out
    }

    /// Handles a key.
    pub fn handle_input(&mut self, data: &str, ui: &Ui<'_>) -> Outcome {
        let kb = ui.keys;
        if kb.matches(data, "tui.select.up") {
            self.selected = self.next_item(self.selected, false);
        } else if kb.matches(data, "tui.select.down") {
            self.selected = self.next_item(self.selected, true);
        } else if kb.matches(data, "tui.select.pageUp") {
            let mut target = self.selected.saturating_sub(self.max_visible);
            while target < self.filtered.len() && !matches!(self.filtered[target], Row::Item(..)) {
                target += 1;
            }
            if target < self.filtered.len() {
                self.selected = target;
            }
        } else if kb.matches(data, "tui.select.pageDown") {
            let last = self.filtered.len().saturating_sub(1);
            let mut target = (self.selected + self.max_visible).min(last);
            while target > 0 && !matches!(self.filtered.get(target), Some(Row::Item(..))) {
                target -= 1;
            }
            if matches!(self.filtered.get(target), Some(Row::Item(..))) {
                self.selected = target;
            }
        } else if kb.matches(data, "tui.select.cancel") || kb.decoder().matches(data, "ctrl+c") {
            return Outcome::Cancel;
        } else if kb.matches(data, "tui.input.tab") {
            if self.project_available {
                self.switch_mode();
            }
        } else if data == " " || kb.matches(data, "tui.select.confirm") {
            self.toggle();
        } else if kb.decoder().matches(data, "ctrl+o") {
            self.toggle_keep();
        } else {
            self.input.handle_input(data, kb);
            self.filter();
        }
        Outcome::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// chest's resources show which manifest entry they come from and that
    /// chest manages them, so they are not toggled here.
    #[test]
    fn chest_items_read_only() {
        let info = |source: &str| SourceInfo {
            path: "/p/ext.ts".into(),
            source: source.into(),
            scope: "project".into(),
            origin: "package".into(),
            base_dir: None,
        };
        let label = |source| group_label(&info(source), Path::new("/agent"), "/home");
        assert_eq!(
            label("chest:todo"),
            "chest:todo (project, managed by chest)"
        );
        assert_eq!(label("npm:todo"), "npm:todo (project)");
        assert!(managed(&info("chest:todo")) && !managed(&info("npm:todo")));

        // Space leaves a global chest extension on.
        let dir = std::env::temp_dir().join(format!("yapi-config-chest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut global = ResolvedPaths::default();
        global.extensions = vec![yapi_core::packages::resolve::Resolved {
            info: SourceInfo {
                scope: "user".into(),
                ..info("chest:todo")
            },
            enabled: true,
        }];
        let settings = SettingsManager::load(&dir, &dir, false);
        let (cwd, agent) = (dir.clone(), dir.clone());
        let none = ResolvedPaths::default();
        let mut selector =
            ConfigSelector::new(&global, &none, settings, cwd, agent, 24, false, false);
        let item = |row: &Row| matches!(row, Row::Item(..));
        selector.selected = selector.filtered.iter().position(item).unwrap();
        selector.toggle();
        assert!(selector.groups[0][0].subgroups[0].items[0].enabled);
    }

    /// `ctrl+o` keeps a global resource in project-only runs by its path,
    /// a skill by its folder or its single file, in the global settings
    /// file whatever its format, and pressing it again stops keeping it.
    #[test]
    fn keep_key_writes_setting() {
        let dir = std::env::temp_dir().join(format!("yapi-config-keep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.toml"), "# Mine.\ntheme = \"dark\"\n").unwrap();
        let resource = |path: &str, source: &str| yapi_core::packages::resolve::Resolved {
            info: SourceInfo {
                path: dir.join(path).display().to_string(),
                source: source.into(),
                scope: "user".into(),
                origin: "top-level".into(),
                base_dir: None,
            },
            enabled: true,
        };
        let mut global = ResolvedPaths::default();
        global.skills = vec![
            resource("skills/pdf/SKILL.md", "auto"),
            resource("skills/notes.md", "auto"),
        ];
        global.extensions = vec![resource("builtin:mcp", "builtin")];
        let settings = SettingsManager::load(&dir, &dir, false);
        let none = ResolvedPaths::default();
        let mut selector = ConfigSelector::new(
            &global,
            &none,
            settings,
            dir.clone(),
            dir.clone(),
            24,
            false,
            false,
        );
        let rows: Vec<usize> = (0..selector.filtered.len())
            .filter(|row| matches!(selector.filtered[*row], Row::Item(..)))
            .collect();
        let item = |selector: &ConfigSelector, row: usize| {
            let Row::Item(g, s, i) = selector.filtered[row] else {
                unreachable!()
            };
            selector.groups()[g].subgroups[s].items[i].clone()
        };
        let kept = |selector: &ConfigSelector| -> Vec<String> {
            let setting = &selector.settings.document(Scope::Global)["projectOnly"];
            let mut kept = string_list(setting.as_object().unwrap(), "skills");
            kept.extend(string_list(setting.as_object().unwrap(), "extensions"));
            kept
        };
        let row = |selector: &ConfigSelector, end: &str| {
            let found = rows
                .iter()
                .find(|row| item(selector, **row).info.path.ends_with(end));
            *found.unwrap()
        };
        let (pdf, notes) = (row(&selector, "pdf/SKILL.md"), row(&selector, "notes.md"));
        for row in rows.iter().copied() {
            selector.selected = row;
            selector.toggle_keep();
        }
        let mut both = kept(&selector);
        both.sort();
        assert_eq!(both, ["skills/notes.md", "skills/pdf"]);
        let skill = item(&selector, pdf);
        assert!(selector.kept(&skill));
        let text = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
        assert!(text.starts_with("# Mine.\ntheme = \"dark\"\n"), "{text}");
        selector.selected = pdf;
        selector.toggle_keep();
        assert_eq!(kept(&selector), ["skills/notes.md"]);
        assert!(!selector.kept(&skill));
        // Project mode leaves the global setting alone.
        selector.project = true;
        selector.selected = notes;
        selector.toggle_keep();
        assert_eq!(kept(&selector), ["skills/notes.md"]);
        // The last item kept leaves no empty list, nor an empty setting.
        selector.project = false;
        selector.toggle_keep();
        let text = std::fs::read_to_string(dir.join("settings.toml")).unwrap();
        assert_eq!(text, "# Mine.\ntheme = \"dark\"\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
