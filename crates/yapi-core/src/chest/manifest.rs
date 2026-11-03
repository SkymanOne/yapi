//! `chest.toml`: what a project installs, in `[skills]`, `[agents]`,
//! `[extensions]` and `[patch]`, and what a package publishes, in
//! `[package]`.

use std::collections::HashMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use indexmap::IndexMap;
use serde::de::{DeserializeOwned, IntoDeserializer as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use toml_edit::{DocumentMut, Item};

use super::{ChestError, invalid};
use crate::extensions::{EventPolicy, Grants};
use crate::packages::source::{self, Source};
use crate::settings::Scope;
use crate::tools::path::{expand_home, home_dir, resolve_lexically};

/// A manifest section of named entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    /// `[skills]`: folders holding a `SKILL.md`.
    Skills,
    /// `[agents]`: subagent `.md` files.
    Agents,
    /// `[extensions]`: pi packages and native extensions.
    Extensions,
    /// `[patch]`: local folders that stand in for entries of the others.
    Patch,
}

impl Section {
    /// Every section, in manifest order.
    pub const ALL: [Section; 4] = [
        Section::Skills,
        Section::Agents,
        Section::Extensions,
        Section::Patch,
    ];

    /// Its key in the manifest.
    pub fn key(self) -> &'static str {
        match self {
            Section::Skills => "skills",
            Section::Agents => "agents",
            Section::Extensions => "extensions",
            Section::Patch => "patch",
        }
    }
}

/// `[folders]`: where installed items go, relative to the manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Folders {
    /// Skill folders. The first holds the canonical copies, the others link
    /// to them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    /// Subagent folders, placed as skills are.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// The extension store, with one folder per entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<String>,
}

/// An entry of a section. Its source is `git` or `npm`, each with an
/// optional subfolder `path`, or `path` alone, a path relative to the
/// manifest. The keys after `path` belong to `[extensions]` entries only.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// A git repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    /// The branch, tag or commit of `git`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    /// An npm package name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    /// The version range or dist-tag of `npm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The subfolder of the git or npm package, or the source itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The capabilities granted, with the manifest's defaults. Absent grants
    /// none of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
    /// Which `pi.events` channels cross to and from other grants. Absent
    /// is shared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<EventPolicy>,
    /// The package's skills to load, as a settings package filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    /// The package's prompt templates to load, as a settings package filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<String>>,
    /// The package's themes to load, as a settings package filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub themes: Option<Vec<String>>,
}

/// `[package]`: what a package says about itself. Its `capabilities` and
/// `events` are requests, which never grant anything by themselves. A
/// resource list holds paths and globs, with `!` excludes, relative to the
/// package.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    /// The package name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// A one-line description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Its authors, as `Name <email>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// An SPDX license expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Its source repository URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Its home page URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// Search keywords.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// The capabilities its extensions need.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<String>>,
    /// The `pi.events` channels its extensions use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<EventPolicy>,
    /// Its skills.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    /// Its subagents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<Vec<String>>,
    /// Its extension entry files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    /// Its prompt templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<String>>,
    /// Its themes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub themes: Option<Vec<String>>,
}

impl Package {
    /// Its resource list of `kind`, one of [`RESOURCE_KINDS`].
    fn list(&self, kind: &str) -> Option<&Vec<String>> {
        match kind {
            "extensions" => self.extensions.as_ref(),
            "skills" => self.skills.as_ref(),
            "prompts" => self.prompts.as_ref(),
            "themes" => self.themes.as_ref(),
            "agents" => self.agents.as_ref(),
            _ => None,
        }
    }
}

/// A valid `chest.toml`. Section maps keep manifest order, which is load
/// order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// Capabilities every extension entry of this manifest gets.
    pub capabilities_defaults: Vec<String>,
    /// `[folders]`.
    pub folders: Folders,
    /// `[skills]`.
    pub skills: IndexMap<String, Entry>,
    /// `[agents]`.
    pub agents: IndexMap<String, Entry>,
    /// `[extensions]`.
    pub extensions: IndexMap<String, Entry>,
    /// `[patch]`, by the name of the entry each replaces.
    pub patch: IndexMap<String, Entry>,
    /// `[package]`.
    pub package: Option<Package>,
}

impl Manifest {
    /// The manifest in `text`. Errors name the entry and key at fault, such
    /// as `skills.pdf`.
    ///
    /// Unknown keys are errors. Entry names match `[a-z0-9][a-z0-9._-]*` and
    /// are unique across `[skills]`, `[agents]` and `[extensions]`. An entry
    /// takes `git` or `npm`, each with an optional `path`, or `path` alone;
    /// `rev` only with `git` and `version` only with `npm`. A `[patch]` entry
    /// takes `path` alone. Capability lists must parse as [`Grants`]. In a
    /// project manifest, `[folders]` paths are relative, without `..` or a
    /// leading `~`, so they stay inside the project; the user's global
    /// manifest may name any folder.
    pub fn parse(text: &str, scope: Scope) -> Result<Manifest, ChestError> {
        let document: DocumentMut = text.parse().map_err(|err| invalid(format!("{err}")))?;
        Manifest::from_document(&document, scope)
    }

    fn from_document(document: &DocumentMut, scope: Scope) -> Result<Manifest, ChestError> {
        let mut manifest = Manifest::default();
        for (key, item) in document.iter() {
            match key {
                "capabilities-defaults" => manifest.capabilities_defaults = value(key, item)?,
                "folders" => manifest.folders = value(key, item)?,
                "package" => manifest.package = Some(value(key, item)?),
                _ => {
                    let section = Section::ALL
                        .into_iter()
                        .find(|section| section.key() == key)
                        .ok_or_else(|| invalid(format!("unknown key `{key}`")))?;
                    let table = item
                        .as_table_like()
                        .ok_or_else(|| invalid(format!("{key}: expected a table")))?;
                    for (name, item) in table.iter() {
                        let entry = value(&format!("{key}.{name}"), item)?;
                        manifest.section_mut(section).insert(name.to_owned(), entry);
                    }
                }
            }
        }
        manifest.validate(scope)?;
        Ok(manifest)
    }

    /// The entries of `section`.
    pub fn section(&self, section: Section) -> &IndexMap<String, Entry> {
        match section {
            Section::Skills => &self.skills,
            Section::Agents => &self.agents,
            Section::Extensions => &self.extensions,
            Section::Patch => &self.patch,
        }
    }

    /// The skill, agent and extension entries, in manifest order.
    pub fn entries(&self) -> impl Iterator<Item = (Section, &String, &Entry)> {
        let sections = [Section::Skills, Section::Agents, Section::Extensions];
        sections.into_iter().flat_map(move |section| {
            let entries = self.section(section).iter();
            entries.map(move |(name, entry)| (section, name, entry))
        })
    }

    fn section_mut(&mut self, section: Section) -> &mut IndexMap<String, Entry> {
        match section {
            Section::Skills => &mut self.skills,
            Section::Agents => &mut self.agents,
            Section::Extensions => &mut self.extensions,
            Section::Patch => &mut self.patch,
        }
    }

    /// The grants of extension `entry`: this manifest's
    /// `capabilities-defaults` with the entry's own `capabilities`. A list
    /// that does not parse, which [`Manifest::parse`] refuses, grants
    /// nothing.
    pub fn grants(&self, entry: &Entry) -> Grants {
        let grants = |values: &[String]| Grants::parse(values).unwrap_or_else(|_| Grants::none());
        let own = entry.capabilities.as_deref().unwrap_or_default();
        grants(&self.capabilities_defaults).union(grants(own))
    }

    fn validate(&self, scope: Scope) -> Result<(), ChestError> {
        Grants::parse(&self.capabilities_defaults)
            .map_err(|err| invalid(format!("capabilities-defaults: {err}")))?;
        let folders = &self.folders;
        let skills = folders.skills.iter().map(|path| ("skills", path));
        let agents = folders.agents.iter().map(|path| ("agents", path));
        let store = folders.extensions.iter().map(|path| ("extensions", path));
        let named: Vec<(&str, &String)> = skills.chain(agents).chain(store).collect();
        for (key, path) in &named {
            let expanded = expand_home(path);
            if !Path::new(&expanded)
                .components()
                .any(|part| matches!(part, Component::Normal(_)))
            {
                return Err(invalid(format!(
                    "folders.{key}: \"{path}\" must name a folder"
                )));
            }
            let parts = Path::new(path).components();
            let inside = !path.is_empty()
                && !path.starts_with('~')
                && parts
                    .into_iter()
                    .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
            if scope == Scope::Project && !inside {
                return Err(invalid(format!(
                    "folders.{key}: \"{path}\" must be a relative path inside the project, without `..` or `~`"
                )));
            }
        }
        // No folder may hold another, or the store, or the store them.
        let default_store = match scope {
            Scope::Project => ".yapi/chest",
            Scope::Global => "~/.chest/store",
        };
        let mut paths: Vec<&str> = named.iter().map(|(_, path)| path.as_str()).collect();
        if folders.extensions.is_none() {
            paths.push(default_store);
        }
        let base = match scope {
            Scope::Project => PathBuf::from("/"),
            Scope::Global => home_dir().join(".chest"),
        };
        let normal = |path: &str| resolve_lexically(&base, Path::new(&expand_home(path)));
        for (index, first) in paths.iter().enumerate() {
            for second in &paths[index + 1..] {
                let (a, b) = (normal(first), normal(second));
                if a.starts_with(&b) || b.starts_with(&a) {
                    return Err(invalid(format!(
                        "folders: \"{first}\" and \"{second}\" overlap"
                    )));
                }
            }
        }
        let mut seen: HashMap<&str, Section> = HashMap::new();
        for section in [Section::Skills, Section::Agents, Section::Extensions] {
            for (name, entry) in self.section(section) {
                let context = format!("{}.{name}", section.key());
                if !valid_name(name) {
                    return Err(invalid(format!(
                        "{context}: names use lowercase letters, digits, `.`, `_` and `-`, and start with a letter or digit"
                    )));
                }
                if let Some(other) = seen.insert(name, section) {
                    return Err(invalid(format!(
                        "{context}: the name is already used in [{}]",
                        other.key()
                    )));
                }
                entry.validate(section, &context, scope)?;
            }
        }
        for (name, entry) in &self.patch {
            let path_only = Entry {
                path: entry.path.clone(),
                ..Entry::default()
            };
            let Some(path) = entry.path.as_deref().filter(|_| *entry == path_only) else {
                return Err(invalid(format!("patch.{name}: a patch takes `path` only")));
            };
            check_source(path, scope).map_err(|err| invalid(format!("patch.{name}: {err}")))?;
        }
        if let Some(capabilities) = self.package.as_ref().and_then(|p| p.capabilities.as_ref()) {
            Grants::parse(capabilities)
                .map_err(|err| invalid(format!("package: `capabilities`: {err}")))?;
        }
        Ok(())
    }
}

impl Entry {
    fn validate(&self, section: Section, context: &str, scope: Scope) -> Result<(), ChestError> {
        let fail = |message: String| Err(invalid(format!("{context}: {message}")));
        match (&self.git, &self.npm, &self.path) {
            (Some(_), Some(_), _) => return fail("`git` and `npm` cannot be combined".into()),
            (None, None, None) => return fail("needs `git`, `npm` or `path`".into()),
            _ => {}
        }
        if self.rev.is_some() && self.git.is_none() {
            return fail("`rev` needs `git`".into());
        }
        if self.version.is_some() && self.npm.is_none() {
            return fail("`version` needs `npm`".into());
        }
        if self.rev.as_deref().is_some_and(|rev| rev.starts_with('-')) {
            return fail("`rev` must not start with `-`".into());
        }
        match (&self.path, self.git.is_some() || self.npm.is_some()) {
            // An extension's folder is its module root, so it must be the package.
            (Some(path), false)
                if section == Section::Extensions
                    && !Path::new(path)
                        .components()
                        .any(|part| matches!(part, Component::Normal(_))) =>
            {
                return fail(format!(
                    "`path`: \"{path}\" must name the extension package, not a folder that holds the manifest"
                ));
            }
            (Some(path), false) => check_source(path, scope).or_else(fail)?,
            (Some(path), true) => {
                let parts = Path::new(path).components();
                let inside = parts
                    .into_iter()
                    .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
                if path.is_empty() || !inside {
                    return fail(format!(
                        "`path`: \"{path}\" must be a folder inside the package"
                    ));
                }
            }
            (None, _) => {}
        }
        if let Some(git) = &self.git {
            match source::parse(&format!("git:{git}")) {
                Source::Git {
                    reference: Some(reference),
                    ..
                } => {
                    return fail(format!(
                        "`git`: \"{git}\" names the revision `{reference}`: put it in `rev`"
                    ));
                }
                Source::Git { .. } => {}
                _ => return fail(format!("`git`: not a git repository: {git}")),
            }
        }
        if section == Section::Extensions {
            if self.npm.is_some() && self.path.is_some() {
                return fail(
                    "`path` cannot be combined with `npm` for an extension, which loads its whole npm package"
                        .into(),
                );
            }
            if let Some(Err(err)) = self.capabilities.as_deref().map(Grants::parse) {
                return fail(format!("`capabilities`: {err}"));
            }
            return Ok(());
        }
        let extension_only = [
            ("capabilities", self.capabilities.is_some()),
            ("events", self.events.is_some()),
            ("skills", self.skills.is_some()),
            ("prompts", self.prompts.is_some()),
            ("themes", self.themes.is_some()),
        ];
        match extension_only.iter().find(|(_, set)| *set) {
            Some((key, _)) => fail(format!("`{key}` belongs to [extensions] entries")),
            None => Ok(()),
        }
    }
}

/// Fails unless a path source `path` is allowed in a manifest of `scope`:
/// in a project, relative to the manifest. It may climb out with `..`,
/// which the install notes.
fn check_source(path: &str, scope: Scope) -> Result<(), String> {
    let anchored = path.starts_with('~') || Path::new(path).has_root();
    if scope == Scope::Project && anchored {
        return Err(format!(
            "`path`: \"{path}\" must be relative to the manifest, not absolute or under `~`"
        ));
    }
    Ok(())
}

/// Whether `name` matches `[a-z0-9][a-z0-9._-]*`, the rule for entry and
/// item names, which keeps each a single path segment.
pub fn valid_name(name: &str) -> bool {
    let lower_or_digit = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let mut chars = name.chars();
    chars.next().is_some_and(lower_or_digit)
        && chars.all(|c| lower_or_digit(c) || matches!(c, '.' | '_' | '-'))
}

/// `item` read as `T`, every field of which is optional. Errors start with
/// `context` and, unless the message names it, the key at fault.
fn value<T: DeserializeOwned>(context: &str, item: &Item) -> Result<T, ChestError> {
    let read = |value: toml_edit::Value| T::deserialize(value.into_deserializer());
    let value = item
        .clone()
        .into_value()
        .map_err(|_| invalid(format!("{context}: expected a value")))?;
    read(value.clone()).map_err(|err| {
        let message = err.message();
        // With every field optional, the key at fault fails on its own too.
        let key = value.as_inline_table().and_then(|table| {
            let fails = |(key, value): &(&str, &toml_edit::Value)| {
                let mut alone = toml_edit::InlineTable::new();
                alone.insert(*key, (*value).clone());
                read(toml_edit::Value::InlineTable(alone)).is_err()
            };
            table.iter().find(fails).map(|(key, _)| key)
        });
        match key {
            Some(key) if !message.contains(&format!("`{key}`")) => {
                invalid(format!("{context}: `{key}`: {message}"))
            }
            _ => invalid(format!("{context}: {message}")),
        }
    })
}

/// A `chest.toml` being edited. Edits keep its comments and formatting, and
/// apply only when the result is still a valid manifest.
#[derive(Clone, Debug)]
pub struct ManifestDocument {
    document: DocumentMut,
    manifest: Manifest,
    scope: Scope,
}

impl ManifestDocument {
    /// The manifest in `text`, ready to edit, refused as [`Manifest::parse`]
    /// refuses it for `scope`.
    pub fn parse(text: &str, scope: Scope) -> Result<ManifestDocument, ChestError> {
        let document: DocumentMut = text.parse().map_err(|err| invalid(format!("{err}")))?;
        let manifest = Manifest::from_document(&document, scope)?;
        Ok(ManifestDocument {
            document,
            manifest,
            scope,
        })
    }

    /// The manifest as edited so far.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Adds entry `name` to `section`, or replaces it in its place and with
    /// its comments.
    pub fn set_entry(
        &mut self,
        section: Section,
        name: &str,
        entry: &Entry,
    ) -> Result<(), ChestError> {
        let context = format!("{}.{name}", section.key());
        let value = entry
            .serialize(toml_edit::ser::ValueSerializer::new())
            .map_err(|err| invalid(format!("{context}: {err}")))?;
        self.edit(|document| {
            if !document.contains_key(section.key()) {
                let mut table = toml_edit::Table::new();
                table.set_position(Some(next_position(document)));
                document.insert(section.key(), Item::Table(table));
            }
            let table = document[section.key()]
                .as_table_like_mut()
                .ok_or_else(|| invalid(format!("{}: expected a table", section.key())))?;
            match table.get_mut(name) {
                Some(item) => *item = replacement(item, value),
                None => {
                    table.insert(name, Item::Value(value));
                }
            }
            Ok(())
        })
    }

    /// Removes entry `name` from `section`. Returns whether it was there.
    pub fn remove_entry(&mut self, section: Section, name: &str) -> Result<bool, ChestError> {
        let mut removed = false;
        self.edit(|document| {
            if let Some(table) = document
                .get_mut(section.key())
                .and_then(Item::as_table_like_mut)
            {
                removed = table.remove(name).is_some();
            }
            Ok(())
        })?;
        Ok(removed)
    }

    /// Writes `package` as the `[package]` table, replacing any, as
    /// `chest init --package` scaffolds it.
    pub fn set_package(&mut self, package: &Package) -> Result<(), ChestError> {
        let written = toml_edit::ser::to_document(package)
            .map_err(|err| invalid(format!("package: {err}")))?;
        self.edit(|document| {
            let mut table = written.as_table().clone();
            match document.get("package") {
                Some(Item::Table(old)) => {
                    *table.decor_mut() = old.decor().clone();
                    table.set_position(old.position());
                }
                _ => table.set_position(Some(next_position(document))),
            }
            document.insert("package", Item::Table(table));
            Ok(())
        })
    }

    fn edit(
        &mut self,
        change: impl FnOnce(&mut DocumentMut) -> Result<(), ChestError>,
    ) -> Result<(), ChestError> {
        let mut document = self.document.clone();
        change(&mut document)?;
        self.manifest = Manifest::from_document(&document, self.scope)?;
        self.document = document;
        Ok(())
    }
}

/// The position that shows a new table after every table of `document`.
/// Without one, toml_edit shows it after the table before it in key order.
fn next_position(document: &DocumentMut) -> isize {
    fn last(table: &toml_edit::Table) -> isize {
        let tables = table.iter().filter_map(|(_, item)| item.as_table());
        tables
            .map(|table| table.position().unwrap_or_default().max(last(table)))
            .max()
            .unwrap_or_default()
    }
    last(document.as_table()) + 1
}

/// `value` in the form, place and comments of `old`, which it replaces.
fn replacement(old: &Item, value: toml_edit::Value) -> Item {
    match (old, value) {
        (Item::Table(old), toml_edit::Value::InlineTable(inline)) => {
            let mut table = inline.into_table();
            *table.decor_mut() = old.decor().clone();
            table.set_position(old.position());
            Item::Table(table)
        }
        (old, mut value) => {
            if let Some(old) = old.as_value() {
                *value.decor_mut() = old.decor().clone();
            }
            Item::Value(value)
        }
    }
}

/// The manifest as edited, comments and formatting included.
impl fmt::Display for ManifestDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.document.fmt(formatter)
    }
}

/// The resource kinds a package can list.
pub const RESOURCE_KINDS: [&str; 5] = ["extensions", "skills", "prompts", "themes", "agents"];

/// What a package says about itself, from its `chest.toml` `[package]`,
/// else its `package.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageMeta {
    /// `[package]` `name`, else package.json `name`.
    pub name: Option<String>,
    /// `[package]` `version`, else package.json `version`.
    pub version: Option<String>,
    /// `[package]` `description`, else package.json `description`.
    pub description: Option<String>,
    /// `[package]` `authors`, else package.json `author`.
    pub authors: Vec<String>,
    /// The capabilities requested: `[package]` `capabilities`, else
    /// package.json `yapi.capabilities`, else none. pi's key never requests
    /// any.
    pub capabilities: Vec<String>,
    /// The event channels it says it uses: `[package]` `events`, else
    /// package.json `yapi.events`.
    pub events: Option<EventPolicy>,
    /// The resource list of each kind of [`RESOURCE_KINDS`] that some source
    /// lists, taken from the first that lists it: `[package]`, then
    /// package.json `yapi`, then its `pi` key.
    pub resources: IndexMap<&'static str, Vec<String>>,
    /// Whether the lists decide the package's resources, as pi's manifest
    /// does: a kind none of them lists then provides nothing. True with a
    /// `pi` key, or with a `[package]` or `yapi` key that lists some kind.
    /// Otherwise each kind comes from its conventional folder.
    pub manifest: bool,
}

/// What the package at `root` says about itself, as [`PackageMeta`]
/// describes. `Err` when its `chest.toml` does not parse, or its
/// `[package]` holds what [`Package`] does not. A `chest.toml` that does not
/// read, and a `package.json` that does not read or parse, count as absent,
/// as pi treats a `package.json`.
pub fn package_meta(root: &Path) -> Result<PackageMeta, ChestError> {
    #[derive(Deserialize)]
    struct File {
        package: Option<Package>,
    }
    let path = root.join("chest.toml");
    let package = match std::fs::read_to_string(&path) {
        Ok(text) => {
            toml_edit::de::from_str::<File>(&text)
                .map_err(|err| invalid(format!("{}: {err}", path.display())))?
                .package
        }
        Err(_) => None,
    };
    let json: Value = std::fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|text| yapi_types::json::parse(&text).ok())
        .unwrap_or_default();
    let object = |key: &str| json.get(key).filter(|value| value.is_object());
    let (yapi, pi) = (object("yapi"), object("pi"));
    let strings = |value: Option<&Value>, key: &str| -> Option<Vec<String>> {
        let entries = value?.get(key)?.as_array()?;
        entries
            .iter()
            .map(|entry| entry.as_str().map(str::to_owned))
            .collect()
    };
    let own = package.as_ref();
    let mut resources = IndexMap::new();
    let mut listed = false;
    for kind in RESOURCE_KINDS {
        let first = own
            .and_then(|package| package.list(kind).cloned())
            .or_else(|| strings(yapi, kind));
        listed |= first.is_some();
        if let Some(list) = first.or_else(|| strings(pi, kind)) {
            resources.insert(kind, list);
        }
    }
    let text = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_owned);
    let field = |pick: fn(&Package) -> &Option<String>, key: &str| {
        own.and_then(|package| pick(package).clone())
            .or_else(|| text(key))
    };
    Ok(PackageMeta {
        name: field(|package| &package.name, "name"),
        version: field(|package| &package.version, "version"),
        description: field(|package| &package.description, "description"),
        authors: own
            .map(|package| package.authors.clone())
            .filter(|authors| !authors.is_empty())
            .unwrap_or_else(|| author(json.get("author").unwrap_or(&Value::Null))),
        capabilities: own
            .and_then(|package| package.capabilities.clone())
            .or_else(|| strings(yapi, "capabilities"))
            .unwrap_or_default(),
        events: own.and_then(|package| package.events.clone()).or_else(|| {
            let events = yapi?.get("events")?;
            EventPolicy::deserialize(events).ok()
        }),
        resources,
        manifest: pi.is_some() || listed,
    })
}

/// npm's `author`: `"Name <email> (url)"` as written, or `{ name, email }`.
fn author(value: &Value) -> Vec<String> {
    match value {
        Value::String(author) => vec![author.clone()],
        Value::Object(person) => {
            let name = person.get("name").and_then(Value::as_str);
            let email = person.get("email").and_then(Value::as_str);
            name.map(|name| match email {
                Some(email) => format!("{name} <{email}>"),
                None => name.to_owned(),
            })
            .into_iter()
            .collect()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan's consumer and publisher example.
    const EXAMPLE: &str = r#"capabilities-defaults = ["env"]

[folders]                                    # written by `chest init`
skills = [".agents/skills", ".claude/skills"] # the first folder holds the canonical copy
agents = [".yapi/agents", ".claude/agents"]
extensions = ".yapi/chest"                   # extension store, one folder per entry

[skills]
pdf    = { git = "https://github.com/anthropics/skills", rev = "v1.2.0", path = "skills/pdf" }
review = { npm = "@acme/agent-skills", version = "^2.1", path = "review" }
notes  = { path = "tools/skills/notes" }

[agents]
reviewer = { git = "github.com/acme/agents", rev = "main", path = "reviewer.md" }

[extensions]
subagent = { npm = "@acme/pi-subagent", version = "^1.4", capabilities = ["all"] }
todo  = { git = "https://github.com/acme/pi-todo", rev = "4f1c2e9", capabilities = ["fs"], events = { send = ["todo:*"], receive = [] },
          skills = ["!legacy/**"], prompts = [], themes = ["themes/dark.json"] }
shout = { path = "ext/shout.wasm" }

[patch]
todo = { path = "../pi-todo" }

[package]
name = "acme-review"
version = "1.3.0"
description = "Code review skills and a reviewer subagent"
authors = ["Ada <ada@acme.dev>"]
license = "MIT"
repository = "https://github.com/acme/review"
homepage = "https://acme.dev/review"
keywords = ["review", "git"]
capabilities = ["fs", "session"]   # requested, never granted by itself
events = { send = ["review:*"], receive = ["subagent:done"] }   # requested channel links
skills = ["skills/*"]              # globs. Absent: conventional folders
agents = ["agents/*.md"]
extensions = ["extensions/index.ts"]
"#;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn some(values: &[&str]) -> Option<Vec<String>> {
        Some(strings(values))
    }

    #[test]
    fn manifest_parses_example() {
        let manifest = Manifest::parse(EXAMPLE, Scope::Project).unwrap();
        assert_eq!(manifest.capabilities_defaults, ["env"]);
        assert_eq!(
            manifest.folders,
            Folders {
                skills: strings(&[".agents/skills", ".claude/skills"]),
                agents: strings(&[".yapi/agents", ".claude/agents"]),
                extensions: Some(".yapi/chest".into()),
            }
        );
        let names = |section: Section| -> Vec<&str> {
            manifest
                .section(section)
                .keys()
                .map(String::as_str)
                .collect()
        };
        assert_eq!(names(Section::Skills), ["pdf", "review", "notes"]);
        assert_eq!(names(Section::Agents), ["reviewer"]);
        assert_eq!(names(Section::Extensions), ["subagent", "todo", "shout"]);
        assert_eq!(
            manifest.skills["review"],
            Entry {
                npm: Some("@acme/agent-skills".into()),
                version: Some("^2.1".into()),
                path: Some("review".into()),
                ..Entry::default()
            }
        );
        assert_eq!(
            manifest.extensions["todo"],
            Entry {
                git: Some("https://github.com/acme/pi-todo".into()),
                rev: Some("4f1c2e9".into()),
                capabilities: some(&["fs"]),
                events: Some(EventPolicy::new(strings(&["todo:*"]), Vec::new()).unwrap()),
                skills: some(&["!legacy/**"]),
                prompts: some(&[]),
                themes: some(&["themes/dark.json"]),
                ..Entry::default()
            }
        );
        assert_eq!(manifest.patch["todo"].path.as_deref(), Some("../pi-todo"));
        let package = manifest.package.unwrap();
        assert_eq!(package.name.as_deref(), Some("acme-review"));
        assert_eq!(package.authors, ["Ada <ada@acme.dev>"]);
        assert_eq!(package.keywords, ["review", "git"]);
        assert_eq!(package.capabilities, some(&["fs", "session"]));
        assert_eq!(
            package.events,
            Some(EventPolicy::new(strings(&["review:*"]), strings(&["subagent:done"])).unwrap())
        );
        assert_eq!(package.agents, some(&["agents/*.md"]));

        let rejected = [
            (
                "[skills]\nPdf = { path = \"x\" }",
                "skills.Pdf: names use lowercase letters, digits, `.`, `_` and `-`, and start with a letter or digit",
            ),
            (
                "[skills]\n-x = { path = \"x\" }",
                "skills.-x: names use lowercase letters, digits, `.`, `_` and `-`, and start with a letter or digit",
            ),
            (
                "[skills]\nx = { path = \"a\" }\n[agents]\nx = { path = \"b\" }",
                "agents.x: the name is already used in [skills]",
            ),
            (
                "[skills]\nx = { git = \"https://github.com/a/b\", npm = \"c\" }",
                "skills.x: `git` and `npm` cannot be combined",
            ),
            ("[skills]\nx = {}", "skills.x: needs `git`, `npm` or `path`"),
            (
                "[skills]\nx = { path = \"a\", rev = \"v1\" }",
                "skills.x: `rev` needs `git`",
            ),
            (
                "[skills]\nx = { git = \"https://github.com/a/b\", version = \"1\" }",
                "skills.x: `version` needs `npm`",
            ),
            (
                "[skills]\nx = { git = \"a b\" }",
                "skills.x: `git`: not a git repository: a b",
            ),
            (
                "[skills]\nx = { path = 1 }",
                "skills.x: `path`: invalid type: integer `1`, expected a string",
            ),
            (
                "[skills]\nx = { path = \"a\", capabilities = [\"fs\"] }",
                "skills.x: `capabilities` belongs to [extensions] entries",
            ),
            (
                "[agents]\nx = { path = \"a\", events = \"isolated\" }",
                "agents.x: `events` belongs to [extensions] entries",
            ),
            (
                "[extensions]\nx = { npm = \"a\", path = \"b\" }",
                "extensions.x: `path` cannot be combined with `npm` for an extension, which loads its whole npm package",
            ),
            (
                "[extensions]\nx = { path = \"a\", capabilities = [\"process\"] }",
                "extensions.x: `capabilities`: \"process\" is not a capability: a process runs as you, outside the sandbox, so only \"all\" allows it",
            ),
            (
                "[extensions]\nx = { path = \"a\", capabilities = [\"none\", \"fs\"] }",
                "extensions.x: `capabilities`: \"none\" cannot be combined with other capabilities",
            ),
            (
                "[extensions]\nx = { path = \"a\", events = \"open\" }",
                "extensions.x: `events`: invalid value: string \"open\", expected \"shared\", \"isolated\" or { send, receive }",
            ),
            (
                "[extensions]\nx = { path = \"a\", events = { send = [\"a*b\"] } }",
                "extensions.x: `events`: invalid event channel pattern \"a*b\": use a channel name, a prefix ending in `*`, or `*`",
            ),
            (
                "[patch]\nx = { git = \"https://github.com/a/b\" }",
                "patch.x: a patch takes `path` only",
            ),
            (
                "capabilities-defaults = [\"exec\"]",
                "capabilities-defaults: unknown capability \"exec\"",
            ),
            (
                "[package]\ncapabilities = [\"none\", \"net\"]",
                "package: `capabilities`: \"none\" cannot be combined with other capabilities",
            ),
            ("colour = 1", "unknown key `colour`"),
            (
                "[folders]\nskills = [\".agents/skills\", \"../x\"]",
                "folders.skills: \"../x\" must be a relative path inside the project, without `..` or `~`",
            ),
            (
                "[folders]\nagents = [\"/abs\"]",
                "folders.agents: \"/abs\" must be a relative path inside the project, without `..` or `~`",
            ),
            (
                "[folders]\nextensions = \"~/x\"",
                "folders.extensions: \"~/x\" must be a relative path inside the project, without `..` or `~`",
            ),
            (
                "[skills]\nx = { path = \"~/x\" }",
                "skills.x: `path`: \"~/x\" must be relative to the manifest, not absolute or under `~`",
            ),
            (
                "[patch]\nx = { path = \"/abs\" }",
                "patch.x: `path`: \"/abs\" must be relative to the manifest, not absolute or under `~`",
            ),
            (
                "[skills]\nx = { git = \"https://github.com/a/b\", path = \"../x\" }",
                "skills.x: `path`: \"../x\" must be a folder inside the package",
            ),
            (
                "[extensions]\nx = { path = \".\" }",
                "extensions.x: `path`: \".\" must name the extension package, not a folder that holds the manifest",
            ),
            (
                "[extensions]\nx = { path = \"../..\" }",
                "extensions.x: `path`: \"../..\" must name the extension package, not a folder that holds the manifest",
            ),
            (
                "[skills]\nx = { git = \"https://github.com/a/b\", rev = \"--upload-pack=x\" }",
                "skills.x: `rev` must not start with `-`",
            ),
            ("skills = 1", "skills: expected a table"),
            (
                "[folders]\nskills = [\".\"]",
                "folders.skills: \".\" must name a folder",
            ),
            (
                "[folders]\nskills = [\".agents\", \".agents/skills\"]",
                "folders: \".agents\" and \".agents/skills\" overlap",
            ),
            (
                "[folders]\nagents = [\".yapi/chest/agents\"]",
                "folders: \".yapi/chest/agents\" and \".yapi/chest\" overlap",
            ),
            (
                "[skills]\nx = { git = \"github.com/a/b@v1\" }",
                "skills.x: `git`: \"github.com/a/b@v1\" names the revision `v1`: put it in `rev`",
            ),
        ];
        for (text, message) in rejected {
            let error = Manifest::parse(text, Scope::Project)
                .unwrap_err()
                .to_string();
            assert_eq!(error, message, "{text}");
        }
        // The user's own global manifest may name folders anywhere.
        let global = "[folders]\nskills = [\"~/x\", \"/abs\"]\n[skills]\nx = { path = \"~/x\" }";
        let global = Manifest::parse(global, Scope::Global).unwrap();
        assert_eq!(global.folders.skills, ["~/x", "/abs"]);
        // Unknown keys name the entry and the key.
        for (text, start) in [
            (
                "[skills]\nx = { path = \"a\", foo = 1 }",
                "skills.x: unknown field `foo`",
            ),
            ("[folders]\nskill = []", "folders: unknown field `skill`"),
            (
                "[package]\nauthor = \"Ada\"",
                "package: unknown field `author`",
            ),
            (
                "[extensions]\nx = { path = \"a\", events = { sent = [] } }",
                "extensions.x: `events`: unknown field `sent`",
            ),
        ] {
            let error = Manifest::parse(text, Scope::Project)
                .unwrap_err()
                .to_string();
            assert!(error.starts_with(start), "{error}");
        }
    }

    #[test]
    fn capabilities_union_defaults() {
        let manifest = Manifest::parse(
            "capabilities-defaults = [\"env\"]\n[extensions]\nfs = { path = \"a\", capabilities = [\"fs\"] }\nplain = { path = \"b\" }\nall = { path = \"c\", capabilities = [\"all\"] }\n",
            Scope::Project,
        )
        .unwrap();
        let grants = |name: &str| manifest.grants(&manifest.extensions[name]);
        let env = Grants {
            environment: true,
            ..Grants::none()
        };
        assert_eq!(
            grants("fs"),
            Grants {
                filesystem: true,
                ..env
            }
        );
        assert_eq!(grants("plain"), env);
        assert_eq!(grants("all"), Grants::default());
        let bare =
            Manifest::parse("[extensions]\nplain = { path = \"b\" }\n", Scope::Project).unwrap();
        assert_eq!(bare.grants(&bare.extensions["plain"]), Grants::none());
    }

    #[test]
    fn edit_preserves_format() {
        let text = r#"# Project packages.
capabilities-defaults = ["env"]

[skills]
# The PDF skill.
pdf    = { git = "https://github.com/anthropics/skills", rev = "v1.2.0", path = "skills/pdf" } # pinned
notes  = { path = "tools/skills/notes" }

# Long entries can be tables.
[extensions.todo]
git = "https://github.com/acme/pi-todo"
capabilities = ["fs"]

[extensions]
shout = { path = "ext/shout.wasm" }
"#;
        let mut document = ManifestDocument::parse(text, Scope::Project).unwrap();
        let pdf = Entry {
            rev: Some("v1.3.0".into()),
            ..document.manifest().skills["pdf"].clone()
        };
        document.set_entry(Section::Skills, "pdf", &pdf).unwrap();
        let todo = Entry {
            capabilities: Some(strings(&["fs", "net"])),
            ..document.manifest().extensions["todo"].clone()
        };
        document
            .set_entry(Section::Extensions, "todo", &todo)
            .unwrap();
        let reviewer = Entry {
            git: Some("github.com/acme/agents".into()),
            path: Some("reviewer.md".into()),
            ..Entry::default()
        };
        document
            .set_entry(Section::Agents, "reviewer", &reviewer)
            .unwrap();
        assert!(document.remove_entry(Section::Skills, "notes").unwrap());
        assert!(!document.remove_entry(Section::Skills, "gone").unwrap());
        // An edit that would not validate changes nothing.
        let before = document.to_string();
        assert!(
            document
                .set_entry(Section::Agents, "Bad", &reviewer)
                .is_err()
        );
        assert!(
            document
                .set_entry(Section::Agents, "pdf", &reviewer)
                .is_err()
        );
        assert_eq!(document.to_string(), before);
        document
            .set_package(&Package {
                name: Some("acme".into()),
                capabilities: Some(Vec::new()),
                ..Package::default()
            })
            .unwrap();
        assert_eq!(
            document.to_string(),
            r#"# Project packages.
capabilities-defaults = ["env"]

[skills]
# The PDF skill.
pdf    = { git = "https://github.com/anthropics/skills", rev = "v1.3.0", path = "skills/pdf" } # pinned

# Long entries can be tables.
[extensions.todo]
git = "https://github.com/acme/pi-todo"
capabilities = ["fs", "net"]

[extensions]
shout = { path = "ext/shout.wasm" }

[agents]
reviewer = { git = "github.com/acme/agents", path = "reviewer.md" }

[package]
name = "acme"
capabilities = []
"#
        );
        let manifest = document.manifest();
        assert_eq!(manifest.skills["pdf"].rev.as_deref(), Some("v1.3.0"));
        assert_eq!(manifest.agents["reviewer"], reviewer);
        assert_eq!(
            manifest.package.as_ref().and_then(|p| p.name.as_deref()),
            Some("acme")
        );
        assert_eq!(
            Manifest::parse(&document.to_string(), Scope::Project)
                .as_ref()
                .ok(),
            Some(manifest)
        );
    }

    #[test]
    fn metadata_precedence() {
        let dir = std::env::temp_dir().join(format!("yapi-chest-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();

        // pi gives resources, never capabilities.
        write(
            "package.json",
            r#"{"name": "pkg", "version": "1.0.0", "description": "From npm", "author": {"name": "Ada", "email": "ada@acme.dev"},
                "pi": {"extensions": ["./pi.ts"], "skills": ["pi-skills"], "capabilities": ["all"]}}"#,
        );
        let meta = package_meta(&dir).unwrap();
        assert_eq!(meta.name.as_deref(), Some("pkg"));
        assert_eq!(meta.authors, ["Ada <ada@acme.dev>"]);
        assert!(meta.capabilities.is_empty());
        assert_eq!(meta.resources["extensions"], ["./pi.ts"]);
        assert!(meta.manifest);

        // Per kind: [package], then `yapi`, then `pi`.
        write(
            "package.json",
            r#"{"name": "pkg", "version": "1.0.0", "description": "From npm", "author": "Ada",
                "yapi": {"capabilities": ["net"], "skills": ["yapi-skills"], "events": {"send": ["pkg:*"]}},
                "pi": {"extensions": ["./pi.ts"], "skills": ["pi-skills"], "prompts": ["pi-prompts"]}}"#,
        );
        write(
            "chest.toml",
            "[skills]\nignored = { path = \"x\" }\n[package]\ndescription = \"From chest\"\nprompts = [\"chest-prompts\"]\nagents = [\"agents/*.md\"]\n",
        );
        let meta = package_meta(&dir).unwrap();
        assert_eq!(meta.name.as_deref(), Some("pkg"));
        assert_eq!(meta.version.as_deref(), Some("1.0.0"));
        assert_eq!(meta.description.as_deref(), Some("From chest"));
        assert_eq!(meta.authors, ["Ada"]);
        assert_eq!(meta.capabilities, ["net"]);
        assert_eq!(
            meta.events,
            Some(EventPolicy::new(strings(&["pkg:*"]), Vec::new()).unwrap())
        );
        let lists: Vec<(&str, Vec<String>)> = meta.resources.clone().into_iter().collect();
        assert_eq!(
            lists,
            [
                ("extensions", strings(&["./pi.ts"])),
                ("skills", strings(&["yapi-skills"])),
                ("prompts", strings(&["chest-prompts"])),
                ("agents", strings(&["agents/*.md"])),
            ]
        );
        write(
            "chest.toml",
            "[package]\ncapabilities = []\nauthors = [\"Bo\"]\n",
        );
        let meta = package_meta(&dir).unwrap();
        assert!(meta.capabilities.is_empty());
        assert_eq!(meta.authors, ["Bo"]);

        // Lists only of capabilities decide nothing, so the folders apply.
        std::fs::remove_file(dir.join("chest.toml")).unwrap();
        write("package.json", r#"{"yapi": {"capabilities": ["fs"]}}"#);
        let meta = package_meta(&dir).unwrap();
        assert_eq!(meta.capabilities, ["fs"]);
        assert!(meta.resources.is_empty() && !meta.manifest);
        // A broken [package] is an error that names the file.
        write(
            "chest.toml",
            "[package]\nauthor = \"Ada\"\nskills = [\"s\"]\n",
        );
        let broken = package_meta(&dir).unwrap_err().to_string();
        let file = dir.join("chest.toml").display().to_string();
        assert!(broken.starts_with(&file), "{broken}");
        assert!(broken.contains("unknown field `author`"), "{broken}");
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(package_meta(&dir).unwrap(), PackageMeta::default());
    }
}
