//! chest, a project package manager for skills, subagents and extensions:
//! the `chest.toml` manifest, the `chest.lock`, content hashes, the
//! placement of installed items, and the engine that installs them. It has
//! no pi counterpart.

mod fetch;
pub mod hash;
pub mod lock;
pub mod manifest;
pub mod place;
mod record;
pub mod startup;
pub mod wasm;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use yapi_ai::auth::lock::{FileLock, LockError, write_atomic};

use crate::packages::npm::{self, Npm, NpmError, Pinned};
use crate::settings::Scope;
use crate::tools::path::{home_dir, resolve_lexically};
use hash::tree_hash;
use lock::{Lock, Locked, fresh};
use manifest::{Entry, Manifest, Section};
use record::{Held, Record, content_hash};

/// Why a chest operation failed.
#[derive(Debug, thiserror::Error)]
pub enum ChestError {
    /// A manifest, lock or package tree that chest refuses.
    #[error("{0}")]
    Invalid(String),
    /// A git command or another step of an install failed.
    #[error("{0}")]
    Failed(String),
    /// An npm operation failed.
    #[error(transparent)]
    Npm(#[from] NpmError),
    /// A file operation failed.
    #[error("{path}: {source}")]
    Io {
        /// Where.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
}

fn invalid(message: impl Into<String>) -> ChestError {
    ChestError::Invalid(message.into())
}

/// `path` with `/` between its parts. Only the platform's separator turns
/// into one: on Unix a `\` is part of a name.
fn slashed(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.replace(std::path::MAIN_SEPARATOR, "/")
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ChestError + '_ {
    move |source| ChestError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// What [`Project::sync`] does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `chest install`: keep what is locked, fresh and present, holding
    /// what the lock pins as chest's record says. Replay what is locked and
    /// fresh but missing or different, resolve the rest, then write the
    /// lock and the `.gitignore` block and remove what the manifest dropped.
    /// A copy chest placed that changed since stays as it is, and its entry
    /// is [`Outcome::Edited`].
    Resolve,
    /// `chest update`: [`Mode::Resolve`], resolving the named entries, or
    /// all with no names, again even when their lock is fresh.
    Update(Vec<String>),
    /// `chest install --frozen`: fail on a missing or stale lock and on an
    /// installed copy that no longer matches its hash, unless chest's
    /// record holds it, placed for another lock and unchanged since. Put
    /// back what is missing, and such copies, as the lock pins them. Never
    /// writes the lock.
    Frozen,
    /// yapi startup: replay what is locked, fresh and missing, unless
    /// `offline`, and skip the rest with one warning. Never writes the lock
    /// or `.gitignore`.
    Startup {
        /// Whether to leave missing entries alone rather than download them.
        offline: bool,
    },
}

/// What happened to one entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Resolved, or copied from a path source, and put in place.
    Installed,
    /// Put in place again exactly as locked.
    Replayed,
    /// Already in place as locked.
    Unchanged,
    /// Left as it is, wholly or in part: someone edited a copy chest placed.
    Edited,
    /// Left alone: stale, unlocked, offline, or not installed when frozen.
    Skipped,
    /// Removed, because the manifest no longer has it.
    Removed,
}

/// What a sync did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Each entry, as `section.name`, with what happened to it: manifest
    /// entries in order, then removed ones.
    pub entries: Vec<(String, Outcome)>,
    /// Problems the user should act on.
    pub warnings: Vec<String>,
    /// Facts worth showing, such as a path source outside the project.
    pub notices: Vec<String>,
}

/// A locked git or npm entry with the newest revision its source offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outdated {
    /// The entry, as `section.name`.
    pub id: String,
    /// The locked npm version or git commit.
    pub locked: String,
    /// The newest npm version in the entry's range, or the commit its `rev`
    /// names now.
    pub wanted: String,
    /// npm's `latest` version, which may lie outside the range.
    pub latest: Option<String>,
}

/// A package fetched to look at, kept until dropped.
pub struct Fetched {
    /// The package's folder: a checkout, an unpacked npm package, or a path
    /// source.
    pub root: PathBuf,
    _work: Option<fetch::Work>,
}

/// One manifest entry and what the old lock says of it.
struct Target<'a> {
    id: String,
    section: Section,
    name: &'a str,
    entry: &'a Entry,
    patch: Option<&'a Entry>,
    locked: Option<&'a Locked>,
}

/// An entry fetched, checked and ready to put in place.
struct Staged {
    id: String,
    section: Section,
    record: Option<Locked>,
    content: Content,
    outcome: Outcome,
    /// Items of the package left out, with why.
    warnings: Vec<String>,
}

enum Content {
    /// Skills or agents: each placed name with the folder or file it copies
    /// and its hash.
    Items(Vec<(String, PathBuf, String)>),
    /// An extension built in `staging`, beside its store folder `dir`, with
    /// its hash.
    Store {
        staging: PathBuf,
        dir: PathBuf,
        hash: String,
    },
    /// Nothing to put in place.
    Nothing,
}

/// Locks `file` as [`FileLock::acquire`] does, but waits for as long as
/// another holder keeps the lock fresh, where `acquire` gives up after the
/// stale period. A holder that stopped stops refreshing, so its lock goes
/// stale and is taken over. `waiting` hears once that the lock is held,
/// when it is not free within a second.
pub async fn hold_lock(
    file: &Path,
    waiting: &mut (dyn FnMut() + Send),
) -> Result<FileLock, ChestError> {
    let cancel = CancellationToken::new();
    let failed = |err: LockError| ChestError::Failed(format!("{}: {err}", file.display()));
    let first = tokio::time::timeout(Duration::from_secs(1), FileLock::acquire(file, &cancel));
    match first.await {
        Ok(Ok(lock)) => return Ok(lock),
        Ok(Err(LockError::Held)) | Err(_) => waiting(),
        Ok(Err(err)) => return Err(failed(err)),
    }
    loop {
        match FileLock::acquire(file, &cancel).await {
            Err(LockError::Held) => {}
            locked => return locked.map_err(failed),
        }
    }
}

/// Fails when `path` is a symbolic link. chest reads and writes a manifest
/// and its lock only as plain files, since a cloned project could point a
/// link anywhere.
pub fn refuse_link(path: &Path) -> Result<(), ChestError> {
    if path.symlink_metadata().is_ok_and(|meta| meta.is_symlink()) {
        return Err(invalid(format!(
            "{} is a symbolic link, which chest does not follow: replace it with the file itself",
            path.display()
        )));
    }
    Ok(())
}

/// Removes the extensions `staged` built and did not put in place.
fn discard(staged: &[Staged]) {
    for staged in staged {
        if let Content::Store { staging, .. } = &staged.content {
            let _ = std::fs::remove_dir_all(staging);
        }
    }
}

/// `path` with its deepest existing part resolved to its real path, links
/// followed and, where the file system ignores case, as the disk spells it.
fn real_path(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut current = path;
    loop {
        if let Ok(real) = current.canonicalize() {
            return rest.iter().rev().fold(real, |real, part| real.join(part));
        }
        match (current.parent(), current.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                current = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// A `chest.toml` with its lock, and the folders its entries go to.
#[derive(Clone, Debug)]
pub struct Project {
    /// The manifest.
    pub manifest: Manifest,
    /// Its lock, when there is one.
    pub lock: Option<Lock>,
    /// The manifest's folder, which its relative paths start from.
    pub base: PathBuf,
    /// Whether it is a project's manifest or the user's global one.
    pub scope: Scope,
    /// Where git clones are cached and work is staged: `~/.chest/cache`
    /// unless set otherwise.
    pub cache: PathBuf,
    /// Where chest records what it placed on this machine, outside any
    /// repository: `~/.chest/state` unless set otherwise.
    pub state: PathBuf,
    path: PathBuf,
    /// The canonical home folder, which `~` names.
    pub home: PathBuf,
}

/// What startup does with an entry, as its lock and folders stand.
enum AtStartup {
    /// Not locked, or locked for another source: skipped.
    Stale,
    /// In place, or patched: kept.
    Present,
    /// Missing or outdated, and offline: skipped.
    Missing,
    /// Missing, or in place as an older lock pinned it: put back from the
    /// lock.
    Replay,
}

impl Project {
    /// The manifest at `path` and its lock, the file beside it named with
    /// `.lock` for `.toml`: `chest.dev.toml` locks in `chest.dev.lock`.
    ///
    /// The manifest's folder is resolved to its canonical path once, so a
    /// project opened through a link or another spelling of its path places,
    /// records and removes the same paths. A manifest or lock that is itself
    /// a link is refused, as [`refuse_link`] says.
    pub fn open(path: &Path, scope: Scope) -> Result<Project, ChestError> {
        let path = std::path::absolute(path).map_err(io(path))?;
        refuse_link(&path)?;
        let text = std::fs::read_to_string(&path).map_err(io(&path))?;
        let manifest = Manifest::parse(&text, scope)
            .map_err(|err| invalid(format!("{}: {err}", path.display())))?;
        let folder = path.parent().unwrap_or(&path);
        let base = folder.canonicalize().map_err(io(folder))?;
        let home = home_dir();
        let mut project = Project {
            manifest,
            lock: None,
            path: base.join(path.file_name().unwrap_or_default()),
            base,
            scope,
            cache: home.join(".chest/cache"),
            state: home.join(".chest/state"),
            home: home.canonicalize().unwrap_or(home),
        };
        project.lock = project.read_lock()?;
        Ok(project)
    }

    /// The user's global manifest, `~/.chest/chest.toml`.
    pub fn global() -> Result<Project, ChestError> {
        Project::open(&home_dir().join(".chest/chest.toml"), Scope::Global)
    }

    /// Where the lock is.
    pub fn lock_path(&self) -> PathBuf {
        self.path.with_extension("lock")
    }

    /// The folder chest never writes outside of: the project's own for a
    /// project manifest, home for the global one.
    pub fn root(&self) -> PathBuf {
        match self.scope {
            Scope::Project => self.base.clone(),
            Scope::Global => self.home.clone(),
        }
    }

    /// The extension store: `[folders] extensions`, else `.yapi/chest` in a
    /// project and `~/.chest/store` for the global manifest.
    pub fn store(&self) -> PathBuf {
        match (&self.manifest.folders.extensions, self.scope) {
            (Some(store), _) => self.resolve(store),
            (None, Scope::Project) => self.base.join(".yapi/chest"),
            (None, Scope::Global) => self.home.join(".chest/store"),
        }
    }

    /// `path` from the manifest, against its folder, with `~` the canonical
    /// home folder.
    fn resolve(&self, path: &str) -> PathBuf {
        let path = match path.strip_prefix('~') {
            Some("") => self.home.clone(),
            Some(rest) if rest.starts_with('/') => self.home.join(&rest[1..]),
            _ => PathBuf::from(path),
        };
        resolve_lexically(&self.base, &path)
    }

    /// Whether `folder`, an extension's module root, holds far more than one
    /// package, so the extension could load any file there: it is, or holds,
    /// the home folder, the agent folder, the filesystem's root, the
    /// manifest's folder, which is the project's for a project manifest, the
    /// store, which holds the other extensions, or a folder in `also`. A
    /// held folder that does not exist yet counts by the path it will have.
    pub fn too_wide(&self, folder: &Path, also: &[&Path]) -> bool {
        let Ok(folder) = folder.canonicalize() else {
            return false;
        };
        let held = [
            self.home.clone(),
            crate::config::agent_dir(),
            PathBuf::from("/"),
            self.base.clone(),
            self.store(),
        ];
        held.into_iter()
            .chain(also.iter().map(|path| path.to_path_buf()))
            .any(|held| real_path(&held).starts_with(&folder))
    }

    /// Refuses an extension whose path source, or `[patch]`, is a folder
    /// [`Project::too_wide`] for it.
    fn refuse_wide(&self) -> Result<(), ChestError> {
        for (name, entry) in &self.manifest.extensions {
            let patch = self.manifest.patch.get(name);
            let patch = patch.and_then(|patch| patch.path.as_deref());
            let source = entry.path.as_deref();
            let source = source.filter(|_| entry.git.is_none() && entry.npm.is_none());
            let (id, path) = match (patch, source) {
                (Some(path), _) => (format!("patch.{name}"), path),
                (None, Some(path)) => (format!("extensions.{name}"), path),
                (None, None) => continue,
            };
            let folder = self.resolve(path);
            if self.too_wide(&folder, &[]) {
                return Err(invalid(format!(
                    "{id}: {} holds more than the extension, such as the project, the home or the agent folder, so chest does not install it",
                    folder.display()
                )));
            }
        }
        Ok(())
    }

    /// The folders of skill or agent entries, the canonical one first, by
    /// real path. A folder that links to an earlier one, or spells it
    /// another way, is left out, so nothing is placed there twice.
    fn folders(&self, section: Section) -> Result<Vec<PathBuf>, ChestError> {
        let folders = match section {
            Section::Skills => &self.manifest.folders.skills,
            _ => &self.manifest.folders.agents,
        };
        if folders.is_empty() {
            let key = section.key();
            return Err(invalid(format!(
                "Set `{key}` in [folders] to say where {key} go"
            )));
        }
        let mut real: Vec<PathBuf> = Vec::new();
        for folder in folders {
            let path = real_path(&self.resolve(folder));
            if !real.contains(&path) {
                real.push(path);
            }
        }
        Ok(real)
    }

    fn read_lock(&self) -> Result<Option<Lock>, ChestError> {
        let path = self.lock_path();
        refuse_link(&path)?;
        match std::fs::read_to_string(&path) {
            Ok(text) => Lock::parse(&text)
                .map(Some)
                .map_err(|err| invalid(format!("{}: {err}", path.display()))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(io(&path)(err)),
        }
    }

    fn file_name(&self) -> String {
        self.path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }

    /// How messages name the manifest: its file name in a project, and its
    /// path from `~` for the global one, which no project folder holds.
    fn label(&self) -> String {
        match (self.scope, self.path.strip_prefix(&self.home)) {
            (Scope::Global, Ok(path)) => format!("~/{}", path.display()),
            (Scope::Global, Err(_)) => self.path.display().to_string(),
            (Scope::Project, _) => self.file_name(),
        }
    }

    /// The command that installs the manifest's entries, for messages.
    fn install_command(&self) -> String {
        match (self.scope, self.file_name().as_str()) {
            (Scope::Global, _) => "chest -g install".to_owned(),
            (Scope::Project, "chest.toml") => "chest install".to_owned(),
            (Scope::Project, name) => format!("chest --manifest {name} install"),
        }
    }

    /// The skill, agent and extension entries, in manifest order.
    fn entries(&self) -> impl Iterator<Item = (Section, &String, &Entry)> {
        self.manifest.entries()
    }

    /// The folders a section's items go in: its configured folders, or the
    /// store for extensions.
    fn homes(&self, section: Section) -> Vec<PathBuf> {
        match section {
            Section::Extensions => vec![self.store()],
            _ => self.folders(section).unwrap_or_default(),
        }
    }

    /// Each name `locked` occupies in its section's folders, with the hash
    /// the lock pins for it. An extension from a path occupies none. A path
    /// entry that names a package occupies each item the package provides
    /// now, unhashed, as path entries are never hashed.
    pub(crate) fn locked_items(
        &self,
        section: Section,
        locked: &Locked,
    ) -> Vec<(String, Option<String>)> {
        if section == Section::Extensions {
            let stored = locked.git.is_some() || locked.npm.is_some();
            return stored
                .then(|| (locked.name.clone(), locked.hash.clone()))
                .into_iter()
                .collect();
        }
        let path_source = match (&locked.git, &locked.npm) {
            (None, None) => locked.path.as_deref().map(|path| self.resolve(path)),
            _ => None,
        };
        if let Some(package) = path_source.filter(|source| fetch::is_package(section, source)) {
            let items = fetch::items(&package, section, None, &locked.name);
            let items = items.map(|(items, _)| items).unwrap_or_default();
            return items.into_iter().map(|(name, _)| (name, None)).collect();
        }
        if locked.items.is_empty() {
            return vec![(
                fetch::placed_name(section, &locked.name),
                locked.hash.clone(),
            )];
        }
        let items = locked.items.iter();
        items
            .map(|item| (item.name.clone(), Some(item.hash.clone())))
            .collect()
    }

    /// The paths `locked` occupies: each of its items in each of its
    /// section's folders, or its store folder.
    pub fn placed(&self, section: Section, locked: &Locked) -> Vec<PathBuf> {
        let homes = self.homes(section);
        let items = self.locked_items(section, locked);
        items
            .iter()
            .flat_map(|(name, _)| homes.iter().map(move |home| home.join(name)))
            .collect()
    }

    /// The hash the lock pins for each path its entries occupy.
    fn pinned(&self, lock: &Lock) -> HashMap<PathBuf, String> {
        let mut pinned = HashMap::new();
        for section in [Section::Skills, Section::Agents, Section::Extensions] {
            let homes = self.homes(section);
            for locked in lock.section(section) {
                for (name, hash) in self.locked_items(section, locked) {
                    let Some(hash) = hash else { continue };
                    for home in &homes {
                        pinned.insert(home.join(&name), hash.clone());
                    }
                }
            }
        }
        pinned
    }

    /// Whether `record` says each path `locked` occupies holds what the lock
    /// pins, with no placement in progress. A link counts by the copy it
    /// points to.
    fn recorded(&self, section: Section, locked: &Locked, record: &Record) -> bool {
        let homes = self.homes(section);
        let items = self.locked_items(section, locked);
        items.iter().all(|(name, hash)| {
            let holds = |hash: &String| {
                homes
                    .iter()
                    .all(|home| record.holds(&home.join(name), hash))
            };
            hash.as_ref().is_some_and(holds)
        })
    }

    /// The record of what chest placed for this manifest's project, read
    /// without its lock: for startup, which only reads it.
    fn record(&self) -> Record {
        let path = Record::path(&self.state, &self.root(), self.scope);
        Record::load(&path, &self.root(), &self.file_name())
    }

    /// Whether the copies of `locked` that [`Project::present`] finds are
    /// the ones the lock pins: by `record` for a git or npm entry, and
    /// always for one from a path, which has no hash. A copy that an older
    /// lock pinned is present but not pinned.
    fn holds_locked(
        &self,
        section: Section,
        entry: &Entry,
        locked: &Locked,
        record: &Record,
    ) -> bool {
        entry.git.is_none() && entry.npm.is_none() || self.recorded(section, locked, record)
    }

    /// Whether `record` says each item `content` stages is in every folder
    /// of `section` with the hash it brings, unchanged since. An extension
    /// from a path stages nothing, so it always is.
    fn holds_staged(&self, section: Section, content: &Content, record: &Record) -> bool {
        let items = match content {
            Content::Items(items) => items,
            Content::Nothing => return true,
            Content::Store { .. } => return false,
        };
        let homes = self.homes(section);
        items.iter().all(|(item, _, hash)| {
            homes.iter().all(|home| {
                let path = home.join(item);
                record.holds(&path, hash) && record.held(&path) == Held::Owned
            })
        })
    }

    /// The copies `locked` occupies that `record` says changed since chest
    /// placed them: [`Problem::Modified`] when still there, else
    /// [`Problem::Missing`]. A patched extension loads from its patch, so its
    /// store folder may go.
    fn changed(
        &self,
        section: Section,
        locked: &Locked,
        record: &Record,
    ) -> Vec<(PathBuf, Problem)> {
        let placed = self.placed(section, locked).into_iter();
        let changed = placed.filter(|path| record.held(path) == Held::Modified);
        changed
            .filter_map(|path| match path.symlink_metadata() {
                Ok(_) => Some((path, Problem::Modified)),
                Err(_) => (section != Section::Extensions).then_some((path, Problem::Missing)),
            })
            .collect()
    }

    /// The state of entry `name` of `section` that yapi's startup acts on,
    /// as `chest list` shows it: under `[patch]`, `missing` when a copy chest
    /// placed is gone, `edited` when one changed since, else `patched`.
    /// Otherwise, `no source` for a path entry whose source is gone, `stale`
    /// when not locked or locked for another source, `missing` when not in
    /// place, `edited` when a copy chest placed changed since, whatever lock
    /// pinned it, `outdated` when in place as an older lock pinned it, else
    /// `installed`. Startup hashes no copies, so it loads an `edited` entry
    /// that the lock still pins, or that is patched, as it would unedited.
    pub fn state(&self, section: Section, name: &str, entry: &Entry) -> &'static str {
        if self.no_source(name, entry) {
            return "no source";
        }
        let patched = self.manifest.patch.contains_key(name);
        let locked = self.lock.as_ref().and_then(|lock| {
            lock.section(section)
                .iter()
                .find(|locked| locked.name == name)
                .filter(|locked| fresh(entry, locked))
        });
        let record = self.record();
        let changed = |locked: &Locked, problem: Problem| {
            let changed = self.changed(section, locked, &record);
            changed.iter().any(|(_, found)| *found == problem)
        };
        match locked {
            Some(locked) if patched && changed(locked, Problem::Missing) => "missing",
            Some(locked) if patched && changed(locked, Problem::Modified) => "edited",
            _ if patched => "patched",
            None => "stale",
            Some(locked) if !self.present(section, locked) => "missing",
            Some(locked) if changed(locked, Problem::Modified) => "edited",
            Some(locked) if !self.holds_locked(section, entry, locked, &record) => "outdated",
            Some(_) => "installed",
        }
    }

    /// Whether entry `name` is a path entry, without a `[patch]`, whose
    /// source is gone.
    fn no_source(&self, name: &str, entry: &Entry) -> bool {
        let path_source = match (&entry.git, &entry.npm) {
            (None, None) => entry.path.as_deref(),
            _ => None,
        };
        let patched = self.manifest.patch.contains_key(name);
        !patched && path_source.is_some_and(|path| !self.resolve(path).exists())
    }

    /// Whether everything `locked` occupies is there. A link in the first
    /// folder, where the copy belongs, counts as missing, as after the
    /// folders were reordered. An extension from a path loads where it is,
    /// so it always is.
    pub fn present(&self, section: Section, locked: &Locked) -> bool {
        let placed = self.placed(section, locked);
        let in_place = section == Section::Extensions && placed.is_empty();
        let first = self.homes(section).into_iter().next();
        let there = |path: &PathBuf| match path.symlink_metadata() {
            Ok(metadata) => !metadata.is_symlink() || path.parent() != first.as_deref(),
            Err(_) => false,
        };
        in_place || !placed.is_empty() && placed.iter().all(there)
    }

    /// Brings the installed entries in line with the manifest and lock as
    /// `mode` says, holding the lock file's lock and the project's record
    /// lock throughout. Every entry is fetched and checked before anything
    /// is put in place, so a failed download changes nothing. Nothing is
    /// installed outside [`Project::root`]. Only chest's cache and record,
    /// under `~/.chest` unless set otherwise, are written elsewhere.
    /// `progress` hears a line for each entry fetched, and one while another
    /// chest command holds a lock.
    pub async fn sync(
        &mut self,
        mode: Mode,
        npm: &mut Npm,
        progress: &mut (dyn FnMut(&str) + Send),
    ) -> Result<Report, ChestError> {
        let mut report = Report::default();
        // yapi's startup skips such an entry on its own, with a warning.
        if !matches!(mode, Mode::Startup { .. }) {
            self.refuse_wide()?;
        }
        self.path_notices(&mut report);
        let frozen = mode == Mode::Frozen;
        // Frozen with everything in place writes nothing, not even the
        // lock's lock.
        if frozen {
            self.lock = self.read_lock()?;
            if !self.check_frozen(&mut report)? {
                return Ok(report);
            }
        }
        // Startup takes the lock's lock only to put something back.
        if let Mode::Startup { offline } = mode
            && self.startup_without_lock(offline, &mut report)?
        {
            return Ok(report);
        }
        let lock_path = self.lock_path();
        let mut waiting = || progress(&waiting_for(&lock_path));
        let _held = hold_lock(&lock_path, &mut waiting).await?;
        self.lock = self.read_lock()?;
        // Another install may have changed the lock or put things back meanwhile.
        if frozen && !self.check_frozen(&mut report)? {
            return Ok(report);
        }
        let startup = matches!(mode, Mode::Startup { .. });
        let old = match (&self.lock, startup) {
            (Some(lock), _) => lock.clone(),
            (None, false) => Lock::default(),
            (None, true) => {
                report.warnings.push(format!(
                    "{} has no lock, so nothing from it was installed: run `{}`",
                    self.label(),
                    self.install_command()
                ));
                return Ok(report);
            }
        };
        // Every manifest of the project shares its record, so one install at
        // a time places and removes.
        let record_path = Record::path(&self.state, &self.root(), self.scope);
        let state = record_path.parent().unwrap_or(&record_path);
        std::fs::create_dir_all(state).map_err(io(state))?;
        let mut waiting = || progress(&waiting_for(&record_path));
        let _record_held = hold_lock(&record_path, &mut waiting).await?;
        self.prepare_folders()?;
        let mut record = Record::load(&record_path, &self.root(), &self.file_name());
        // `_work` holds the fetched items until they are placed.
        let (staged, _work) = self
            .prepare(&mode, &old, &record, npm, progress, &mut report)
            .await?;
        let committed = self.commit(
            &staged,
            &old,
            frozen,
            &mut record,
            &record_path,
            &mut report,
        );
        let occupied = match committed {
            Ok(occupied) => occupied,
            Err(err) => {
                discard(&staged);
                return Err(err);
            }
        };
        if startup {
            return Ok(report);
        }
        let mut lock = Lock::default();
        for staged in staged {
            let Some(record) = staged.record else {
                continue;
            };
            match staged.section {
                Section::Skills => lock.skills.push(record),
                Section::Agents => lock.agents.push(record),
                _ => lock.extensions.push(record),
            }
        }
        let kept = self.held_back(&old, &occupied, &record, &mut report);
        self.remove_dropped(&old, &kept, &mut record, &record_path, &mut report)?;
        let text = lock.to_toml()?;
        // Frozen installs what the lock pins and never writes it.
        if !frozen && std::fs::read_to_string(&lock_path).ok().as_deref() != Some(text.as_str()) {
            write_atomic(&lock_path, text.as_bytes(), 0o666).map_err(io(&lock_path))?;
        }
        if self.scope == Scope::Project {
            self.write_ignore(&occupied)?;
        }
        self.lock = Some(lock);
        Ok(report)
    }

    /// Before anything is placed: makes the folders of sections with
    /// entries, so that a folder linking to another resolves, refuses
    /// folders whose real paths nest or that two sections share, and removes
    /// the staging and previous copies a stopped install left in them or in
    /// the store, when they are inside [`Project::root`].
    fn prepare_folders(&self) -> Result<(), ChestError> {
        let root = self.root();
        let mut homes: Vec<PathBuf> = Vec::new();
        for section in [Section::Skills, Section::Agents] {
            let configured = match section {
                Section::Skills => &self.manifest.folders.skills,
                _ => &self.manifest.folders.agents,
            };
            if !self.manifest.section(section).is_empty() {
                for folder in configured {
                    let path = self.resolve(folder);
                    // `within` judges an item by its folder.
                    place::within(&root, &path.join("item"))?;
                    // A folder that is a link stays one.
                    if path.symlink_metadata().is_err() {
                        std::fs::create_dir_all(&path).map_err(io(&path))?;
                    }
                }
            }
            let mut reals: Vec<PathBuf> = Vec::new();
            for folder in configured {
                let real = real_path(&self.resolve(folder));
                if !reals.contains(&real) {
                    reals.push(real);
                }
            }
            homes.extend(reals);
        }
        homes.push(real_path(&self.store()));
        for (index, first) in homes.iter().enumerate() {
            for second in &homes[index + 1..] {
                if first.starts_with(second) || second.starts_with(first) {
                    return Err(invalid(format!(
                        "folders: {} and {} overlap",
                        first.display(),
                        second.display()
                    )));
                }
            }
        }
        for home in &homes {
            // A folder of a section without entries may link out of the root.
            if place::within(&root, &home.join("item")).is_err() {
                continue;
            }
            for entry in std::fs::read_dir(home).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let stopped = name.ends_with("-staging") || name.ends_with("-old");
                if name.starts_with('.') && stopped {
                    let path = entry.path();
                    npm::remove_path(&path).map_err(io(&path))?;
                }
            }
        }
        Ok(())
    }

    /// Fetches and stages each entry that needs it, and keeps the others as
    /// they are, with the work folder that holds what it fetched. On
    /// failure, removes what it staged.
    async fn prepare(
        &self,
        mode: &Mode,
        old: &Lock,
        record: &Record,
        npm: &mut Npm,
        progress: &mut (dyn FnMut(&str) + Send),
        report: &mut Report,
    ) -> Result<(Vec<Staged>, fetch::Work), ChestError> {
        if let Mode::Update(names) = mode
            && let Some(unknown) = names
                .iter()
                .find(|name| self.entries().all(|(_, entry, _)| entry != *name))
        {
            return Err(invalid(format!("No entry is named {unknown}")));
        }
        let mut work = fetch::Work::new(&self.cache)?;
        let mut staged: Vec<Staged> = Vec::new();
        let mut stale = Vec::new();
        for (section, name, entry) in self.entries() {
            let locked = old
                .section(section)
                .iter()
                .find(|locked| &locked.name == name);
            let target = Target {
                id: format!("{}.{name}", section.key()),
                section,
                name,
                entry,
                patch: self.manifest.patch.get(name),
                locked,
            };
            let fresh = locked.is_some_and(|locked| fresh(entry, locked));
            let present = locked.is_some_and(|locked| self.present(section, locked));
            let recorded = locked.is_some_and(|locked| self.recorded(section, locked, record));
            let local = target.patch.is_some() || entry.git.is_none() && entry.npm.is_none();
            let named = matches!(mode, Mode::Update(names)
                if names.is_empty() || names.iter().any(|n| n == name));
            // Past `check_frozen`, a copy that differs from the lock is one
            // chest placed for another lock, which goes back as this one pins.
            let matches = |locked: &Locked| self.copy_problems(section, locked).is_empty();
            let keep = match mode {
                Mode::Startup { offline } => {
                    match self.at_startup(section, name, entry, locked, record, *offline) {
                        AtStartup::Stale => {
                            stale.push(target.id.clone());
                            Some(Outcome::Skipped)
                        }
                        AtStartup::Present => Some(Outcome::Unchanged),
                        AtStartup::Missing => Some(Outcome::Skipped),
                        AtStartup::Replay => None,
                    }
                }
                Mode::Frozen
                    if present
                        && (recorded || locked.is_some_and(matches))
                        && !self.no_source(name, entry) =>
                {
                    Some(Outcome::Unchanged)
                }
                // Frozen fails on it as it stages, and the rest still install.
                mode if *mode != Mode::Frozen && self.no_source(name, entry) => {
                    let path = entry.path.as_deref().unwrap_or_default();
                    report.warnings.push(format!(
                        "{}: {} does not exist, so chest skipped it: restore it or remove the entry",
                        target.id,
                        self.resolve(path).display()
                    ));
                    Some(Outcome::Skipped)
                }
                _ if fresh && present && recorded && !local && !named => {
                    let problems =
                        locked.map_or_else(Vec::new, |locked| self.copy_problems(section, locked));
                    for problem in &problems {
                        report.warnings.push(format!(
                            "{}, so chest left it as it is: delete it and run `{}` to put back the locked copy",
                            describe(problem),
                            self.install_command()
                        ));
                    }
                    Some(match problems.is_empty() {
                        true => Outcome::Unchanged,
                        false => Outcome::Edited,
                    })
                }
                _ => None,
            };
            if let Some(outcome) = keep {
                staged.push(Staged {
                    id: target.id,
                    section,
                    record: locked.cloned(),
                    content: Content::Nothing,
                    outcome,
                    warnings: Vec::new(),
                });
                continue;
            }
            // Fresh and missing replays. Anything else resolves anew.
            let resolve = !fresh || named;
            progress(&format!("Installing {}...", target.id));
            match self.stage(&target, resolve, npm, &mut work).await {
                Ok(mut next) => {
                    let id = &next.id;
                    let warnings = next.warnings.drain(..);
                    report
                        .warnings
                        .extend(warnings.map(|warning| format!("{id}: {warning}")));
                    // What resolves to what the lock pins, in place as chest
                    // put it, changes nothing: by the lock's hashes for git and
                    // npm, by the staged ones for a path entry. A patch always
                    // places, as the lock doesn't list the items it brings.
                    let same = next.record.as_ref() == locked && present && target.patch.is_none();
                    let held = match local {
                        true => self.holds_staged(section, &next.content, record),
                        false => recorded && locked.is_some_and(matches),
                    };
                    if same && held {
                        discard(std::slice::from_ref(&next));
                        next.content = Content::Nothing;
                        next.outcome = Outcome::Unchanged;
                    }
                    staged.push(next);
                }
                Err(err) => {
                    discard(&staged);
                    return Err(match err {
                        ChestError::Invalid(message) => {
                            invalid(format!("{}: {message}", target.id))
                        }
                        err => ChestError::Failed(format!("{}: {err}", target.id)),
                    });
                }
            }
        }
        if !stale.is_empty() {
            report.warnings.push(self.stale_warning(&stale));
        }
        Ok((staged, work))
    }

    /// Gets one entry: from its `[patch]` or path source, or its git or npm
    /// source, resolved anew or replayed as locked.
    async fn stage(
        &self,
        target: &Target<'_>,
        resolve: bool,
        npm: &mut Npm,
        work: &mut fetch::Work,
    ) -> Result<Staged, ChestError> {
        let Target {
            section,
            name,
            entry,
            locked,
            ..
        } = *target;
        let staged = |record, content, outcome, warnings| Staged {
            id: target.id.clone(),
            section,
            record,
            content,
            outcome,
            warnings,
        };
        let hashed = |items: Vec<(String, PathBuf)>| -> Result<Vec<_>, ChestError> {
            let hash = |(name, source): (String, PathBuf)| {
                tree_hash(&source).map(|hash| (name, source, hash))
            };
            items.into_iter().map(hash).collect()
        };
        let path_source = match (&entry.git, &entry.npm) {
            (None, None) => entry.path.as_deref(),
            _ => None,
        };
        if let Some(path) = target
            .patch
            .and_then(|patch| patch.path.as_deref())
            .or(path_source)
        {
            // A patch leaves the lock as it was, and a path entry locks its path.
            let record = match target.patch {
                Some(_) => locked.cloned(),
                None => Some(Locked {
                    name: name.to_owned(),
                    path: entry.path.clone(),
                    ..Locked::default()
                }),
            };
            let source = self.resolve(path);
            if !source.exists() {
                return Err(invalid(format!("{} does not exist", source.display())));
            }
            if section == Section::Extensions {
                let outcome = match resolve {
                    true => Outcome::Installed,
                    false => Outcome::Unchanged,
                };
                return Ok(staged(record, Content::Nothing, outcome, Vec::new()));
            }
            // A path that is not one item, and a patch for a package's
            // items, an entry without `path`, are such a package too.
            let patched_package =
                target.patch.is_some() && path_source.is_none() && entry.path.is_none();
            let path_package = target.patch.is_none() && fetch::is_package(section, &source);
            let (items, warnings) = if patched_package || path_package {
                fetch::items(&source, section, None, name)?
            } else {
                fetch::check_item(section, &source, &source)?;
                let item = (fetch::placed_name(section, name), source);
                (vec![item], Vec::new())
            };
            // An item holding a folder chest installs to would copy itself.
            let sections = [Section::Skills, Section::Agents].into_iter();
            let mut installs: Vec<PathBuf> = sections.flat_map(|s| self.homes(s)).collect();
            installs.push(real_path(&self.store()));
            for (_, item) in &items {
                let real = real_path(item);
                if let Some(home) = installs.iter().find(|home| home.starts_with(&real)) {
                    return Err(invalid(format!(
                        "{} holds {}, where chest installs, so it cannot be installed",
                        item.display(),
                        home.display()
                    )));
                }
            }
            let content = Content::Items(hashed(items)?);
            return Ok(staged(record, content, Outcome::Installed, warnings));
        }
        let replay = locked.filter(|_| !resolve);
        let mut record = Locked {
            name: name.to_owned(),
            git: entry.git.clone(),
            rev: entry.rev.clone(),
            npm: entry.npm.clone(),
            path: entry.path.clone(),
            ..Locked::default()
        };
        let package = work.path();
        if let Some(repo) = &entry.git {
            let commit = match replay {
                Some(locked) => locked.commit.clone().unwrap_or_default(),
                None => fetch::resolve(&self.cache, repo, entry.rev.as_deref()).await?,
            };
            fetch::checkout(&self.cache, repo, &commit, &package).await?;
            record.commit = Some(commit);
        }
        let mut warnings = Vec::new();
        let content = if section == Section::Extensions {
            let dir = self.store().join(name);
            place::within(&self.root(), &dir)?;
            let staging = npm::stage(&dir)?;
            let built = self
                .build_extension(entry, replay, &package, &staging, npm, &mut record)
                .await;
            if let Err(err) = built {
                let _ = std::fs::remove_dir_all(&staging);
                return Err(err);
            }
            let hash = record.hash.clone().unwrap_or_default();
            Content::Store { staging, dir, hash }
        } else {
            if let Some(npm_name) = &entry.npm {
                let pin = match replay {
                    Some(locked) => {
                        let pin = Pinned {
                            path: String::new(),
                            name: npm_name.clone(),
                            version: locked.version.clone().unwrap_or_default(),
                            tarball: locked.tarball.clone().unwrap_or_default(),
                            integrity: locked.integrity.clone().unwrap_or_default(),
                        };
                        npm::fetch_pinned(npm, &pin, &package).await?;
                        pin
                    }
                    None => {
                        let pin =
                            npm::fetch(npm, npm_name, entry.version.as_deref(), &package).await?;
                        // Refused now if replay would refuse it, not in every clone.
                        npm.check_pin(&pin)?;
                        pin
                    }
                };
                record.version = Some(pin.version);
                record.tarball = Some(pin.tarball);
                record.integrity = Some(pin.integrity);
            }
            let (items, skipped) = fetch::items(&package, section, entry.path.as_deref(), name)?;
            warnings = skipped;
            let items = hashed(items)?;
            if entry.path.is_some() {
                record.hash = Some(items[0].2.clone());
            } else {
                for (item, _, hash) in &items {
                    record.items.push(lock::Item {
                        name: item.clone(),
                        hash: hash.clone(),
                    });
                }
            }
            Content::Items(items)
        };
        let Some(locked) = replay else {
            return Ok(staged(Some(record), content, Outcome::Installed, warnings));
        };
        if record.hash != locked.hash || record.items != locked.items {
            if let Content::Store { staging, .. } = &content {
                let _ = std::fs::remove_dir_all(staging);
            }
            return Err(invalid(
                "what was fetched does not match the lock: run `chest update` to take it",
            ));
        }
        Ok(staged(
            Some(locked.clone()),
            content,
            Outcome::Replayed,
            warnings,
        ))
    }

    /// Builds extension `entry` in `staging`: an npm package installed into
    /// it with its full tree, or the git checkout `package` with its
    /// dependencies. Replays `replay`'s tree when given.
    async fn build_extension(
        &self,
        entry: &Entry,
        replay: Option<&Locked>,
        package: &Path,
        staging: &Path,
        npm: &mut Npm,
        record: &mut Locked,
    ) -> Result<(), ChestError> {
        match (&entry.npm, replay) {
            (_, Some(locked)) => {
                // The extension loads from the package's own folder, so the
                // lock must pin the package itself there, at its version.
                if let Some(npm_name) = &entry.npm {
                    let root = format!("node_modules/{npm_name}");
                    let version = locked.version.as_deref().unwrap_or_default();
                    let pinned = locked.packages.iter().any(|pin| {
                        pin.path == root && pin.name == *npm_name && pin.version == version
                    });
                    if !pinned {
                        return Err(invalid(format!(
                            "the lock does not pin {npm_name}@{version} at {root}"
                        )));
                    }
                }
                if entry.git.is_some() {
                    copy_checkout(package, entry.path.as_deref(), staging)?;
                }
                // Replay reaches the package at the top of an npm extension's
                // tree from a root package.json, as its install did.
                let root = staging.join("package.json");
                if let Some(npm_name) = &entry.npm {
                    let version = locked.version.as_deref().unwrap_or_default();
                    let manifest = serde_json::json!({"dependencies": {npm_name: version}});
                    let text = yapi_types::json::to_string(&manifest).unwrap_or_default();
                    std::fs::write(&root, text).map_err(io(&root))?;
                }
                npm::replay(npm, staging, &locked.packages).await?;
                if entry.npm.is_some() {
                    std::fs::remove_file(&root).map_err(io(&root))?;
                }
            }
            (Some(npm_name), None) => {
                let installed =
                    npm::install(npm, staging, npm_name, entry.version.as_deref()).await?;
                // Checked as replay will check it, while the root package.json
                // still names the package at the top.
                npm::check_installed(staging, &installed.tree)?;
                // The store keeps only what replay rebuilds, `node_modules`.
                for file in ["package.json", ".gitignore"] {
                    let file = staging.join(file);
                    std::fs::remove_file(&file).map_err(io(&file))?;
                }
                record.version = Some(installed.version);
                record.packages = installed.tree;
            }
            (None, None) => {
                copy_checkout(package, entry.path.as_deref(), staging)?;
                record.packages = npm::install_dependencies(npm, staging).await?;
                npm::check_installed(staging, &record.packages)?;
            }
        }
        // A tree that replay would refuse fails this install, not every later
        // one. A replayed tree records no pins: replay checked them.
        for pin in &record.packages {
            npm.check_pin(pin)?;
        }
        record.hash = Some(tree_hash(staging)?);
        Ok(())
    }

    /// Puts staged entries in place and returns every path the entries
    /// occupy, kept ones included. Before writing anything, it checks each
    /// target: it must lie within the root, no other entry may occupy it, and
    /// what is there now must be chest's. That is a path the record lists and
    /// that still holds what chest put there, or an unrecorded path that
    /// holds exactly what the lock pins and what is about to replace it, with
    /// no `.git` inside, which chest then adopts. A recorded path that
    /// changed since fails a `frozen` sync. Otherwise its item stays as it
    /// is, with a warning, and its entry is [`Outcome::Edited`]. Each target
    /// is recorded before it is written, so a failure partway leaves it
    /// owned.
    fn commit(
        &self,
        staged: &[Staged],
        old: &Lock,
        frozen: bool,
        record: &mut Record,
        record_path: &Path,
        report: &mut Report,
    ) -> Result<Vec<PathBuf>, ChestError> {
        let root = self.root();
        let pinned = self.pinned(old);
        let mut occupied: HashSet<PathBuf> = HashSet::new();
        for staged in staged {
            if let (Content::Nothing, Some(locked)) = (&staged.content, &staged.record) {
                occupied.extend(self.placed(staged.section, locked));
            }
        }
        // Each placement: what it copies, where to, and the hash it brings.
        let mut placements: Vec<Vec<(PathBuf, Vec<PathBuf>, String)>> = Vec::new();
        let mut outcomes = Vec::new();
        for staged in staged {
            let planned = match &staged.content {
                Content::Items(items) => {
                    let folders = self.folders(staged.section)?;
                    let plan = |(name, source, hash): &(String, PathBuf, String)| {
                        let targets = folders.iter().map(|folder| folder.join(name)).collect();
                        (source.clone(), targets, hash.clone())
                    };
                    items.iter().map(plan).collect()
                }
                Content::Store { staging, dir, hash } => {
                    vec![(staging.clone(), vec![dir.clone()], hash.clone())]
                }
                Content::Nothing => Vec::new(),
            };
            let mut placing = Vec::new();
            let mut left = false;
            for (source, targets, hash) in planned {
                let mut edited = false;
                for target in &targets {
                    let refuse =
                        |why: &str| invalid(format!("{}: {why} {}", staged.id, target.display()));
                    place::within(&root, target)
                        .map_err(|err| invalid(format!("{}: {err}", staged.id)))?;
                    if !occupied.insert(target.clone()) {
                        return Err(refuse("another entry also installs"));
                    }
                    if target.symlink_metadata().is_err() {
                        continue;
                    }
                    match record.held(target) {
                        Held::Owned => {}
                        Held::Modified => {
                            let edit = format!(
                                "{}: {} was modified since chest installed it",
                                staged.id,
                                target.display()
                            );
                            let how = format!(
                                "delete it and run `{}` to put back the locked copy",
                                self.install_command()
                            );
                            if frozen {
                                return Err(invalid(format!("{edit}: {how}")));
                            }
                            let mut warning = format!("{edit}, so chest left it as it is: {how}");
                            // yapi loads an extension only where the record holds what the lock pins.
                            if staged.section == Section::Extensions && !record.holds(target, &hash)
                            {
                                warning.push_str(", and until then yapi does not load it");
                            }
                            report.warnings.push(warning);
                            edited = true;
                        }
                        Held::Unrecorded => {
                            let lock_pins = pinned.get(target);
                            let adoptable = lock_pins == Some(&hash)
                                && content_hash(target).as_ref() == lock_pins
                                && !has_git(target);
                            if !adoptable {
                                return Err(refuse(
                                    "refusing to replace what chest did not install at",
                                ));
                            }
                            record.adopt(target, &hash);
                        }
                    }
                }
                // An edited item stays whole, its links included.
                if edited {
                    left = true;
                } else {
                    placing.push((source, targets, hash));
                }
            }
            let outcome = match left {
                true => Outcome::Edited,
                false => staged.outcome,
            };
            if left {
                discard(std::slice::from_ref(staged));
            }
            outcomes.push(outcome);
            placements.push(placing);
        }
        record.save(record_path)?;
        let entries = staged.iter().zip(placements).zip(outcomes);
        for ((staged, planned), outcome) in entries {
            for (source, targets, hash) in planned {
                let before: Vec<_> = targets.iter().map(|target| record.get(target)).collect();
                // Until it is noted, a target is chest's with the old or the new content.
                for target in &targets {
                    record.expect(target, &hash);
                }
                record.save(record_path)?;
                let placed = match &staged.content {
                    Content::Store { dir, .. } => npm::swap(&source, dir, Ok::<(), ChestError>(())),
                    _ => {
                        let folders = self.folders(staged.section)?;
                        let name = targets[0].file_name().unwrap_or_default().to_string_lossy();
                        place::place(&source, &root, &folders, &name).map(drop)
                    }
                };
                // Placed targets hold the new content, links by their target.
                // After a failure, what was not written goes back to its record.
                for (target, before) in targets.iter().zip(before) {
                    if placed.is_ok() || content_hash(target).as_ref() == Some(&hash) {
                        record.noted(target, &hash);
                    } else {
                        record.restore(target, before);
                    }
                }
                record.save(record_path)?;
                placed?;
            }
            report.entries.push((staged.id.clone(), outcome));
        }
        Ok(occupied.into_iter().collect())
    }

    /// `occupied`, with the copies kept for a skipped entry whose folder is
    /// gone and held several items. Without the folder, chest can't name
    /// that entry's copies, so every copy in its section that would be
    /// removed stays, with a warning that lists them. A missing extension
    /// places nothing, and a missing single item occupies its own name.
    fn held_back(
        &self,
        old: &Lock,
        occupied: &[PathBuf],
        record: &Record,
        report: &mut Report,
    ) -> Vec<PathBuf> {
        let recorded = record.paths();
        let mut kept = occupied.to_vec();
        for (section, name, entry) in self.entries() {
            if section == Section::Extensions || !self.no_source(name, entry) {
                continue;
            }
            let locked = old
                .section(section)
                .iter()
                .find(|locked| &locked.name == name);
            let Some(locked) = locked else { continue };
            // A single item is recorded under the name it occupies.
            if self
                .placed(section, locked)
                .iter()
                .any(|path| recorded.contains(path))
            {
                continue;
            }
            let homes = self.homes(section);
            let held: Vec<&PathBuf> = recorded
                .iter()
                .filter(|path| {
                    path.parent()
                        .is_some_and(|parent| homes.iter().any(|home| home == parent))
                })
                .filter(|path| !kept.contains(path))
                .collect();
            if held.is_empty() {
                continue;
            }
            let list: Vec<String> = held.iter().map(|path| path.display().to_string()).collect();
            report.warnings.push(format!(
                "{}.{name}'s folder is missing and held several items, so chest keeps {}, which it would remove, as it can't tell that entry's copies apart: restore the folder or remove the entry",
                section.key(),
                list.join(", ")
            ));
            kept.extend(held.into_iter().cloned());
        }
        kept
    }

    /// Removes what the record lists for this manifest that none of its
    /// entries occupies any more, when it still holds what chest put there.
    /// A path another manifest of the project still occupies stays theirs,
    /// and so does one that is, by its real path, a path an entry occupies.
    /// A path changed since is left with a warning, and one chest cannot
    /// remove is forgotten with a warning. Reports the entries the manifest
    /// dropped.
    fn remove_dropped(
        &self,
        old: &Lock,
        occupied: &[PathBuf],
        record: &mut Record,
        record_path: &Path,
        report: &mut Report,
    ) -> Result<(), ChestError> {
        // The real path of a placed path: its folder resolved, itself not.
        let real = |path: &Path| match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => real_path(parent).join(name),
            _ => path.to_path_buf(),
        };
        let really: HashSet<PathBuf> = occupied.iter().map(|path| real(path)).collect();
        let occupied: HashSet<&PathBuf> = occupied.iter().collect();
        for path in record.paths() {
            if occupied.contains(&path) {
                continue;
            }
            if record.shared(&path) || really.contains(&real(&path)) {
                record.release(&path);
                record.save(record_path)?;
                continue;
            }
            if path.symlink_metadata().is_ok() {
                match record.held(&path) {
                    Held::Modified => {
                        report.warnings.push(format!(
                            "{} was modified since chest installed it, so chest left it as it is: delete it, or undo the edit and run `{}` to remove it",
                            path.display(),
                            self.install_command()
                        ));
                        continue;
                    }
                    _ => {
                        if let Err(err) = place::unplace(&self.root(), std::slice::from_ref(&path))
                        {
                            report.warnings.push(format!(
                                "chest could not remove {} and forgets it: {err}",
                                path.display()
                            ));
                        }
                    }
                }
            }
            record.forget(&path);
            record.save(record_path)?;
        }
        for section in [Section::Skills, Section::Agents, Section::Extensions] {
            for locked in old.section(section) {
                if !self.manifest.section(section).contains_key(&locked.name) {
                    let id = format!("{}.{}", section.key(), locked.name);
                    report.entries.push((id, Outcome::Removed));
                }
            }
        }
        Ok(())
    }

    /// Lists every `occupied` path in the root's `.gitignore` block, with the
    /// staging and previous copies a stopped install can leave beside the
    /// copies it makes: in the first folder of each section and in the store.
    fn write_ignore(&self, occupied: &[PathBuf]) -> Result<(), ChestError> {
        let root = self.root();
        let relative = |path: &Path| -> Result<String, ChestError> {
            let relative = path.strip_prefix(&root).map_err(|_| {
                invalid(format!("{} is outside {}", path.display(), root.display()))
            })?;
            Ok(slashed(relative))
        };
        let mut entries = Vec::new();
        for path in occupied {
            entries.push(relative(path)?);
        }
        let first = |section| self.folders(section).ok()?.into_iter().next();
        let copies = [
            first(Section::Skills),
            first(Section::Agents),
            Some(self.store()),
        ];
        for home in copies.into_iter().flatten() {
            if occupied
                .iter()
                .any(|path| path.parent() == Some(home.as_path()))
            {
                let home = relative(&home)?;
                entries.push(format!("{home}/.*-staging"));
                entries.push(format!("{home}/.*-old"));
            }
        }
        place::write_ignore(&root, &self.file_name(), &entries)
    }

    /// Notes each path source that resolves outside the project.
    fn path_notices(&self, report: &mut Report) {
        if self.scope != Scope::Project {
            return;
        }
        let entries = self
            .entries()
            .map(|(section, name, entry)| (format!("{}.{name}", section.key()), entry));
        let patches = self
            .manifest
            .patch
            .iter()
            .map(|(name, entry)| (format!("patch.{name}"), entry));
        for (id, entry) in entries.chain(patches) {
            let source = entry
                .path
                .as_deref()
                .filter(|_| entry.git.is_none() && entry.npm.is_none());
            let Some(path) = source else { continue };
            let resolved = self.resolve(path);
            if !resolved.starts_with(&self.base) {
                report.notices.push(format!(
                    "{id}: `{path}` is outside the project, at {}",
                    resolved.display()
                ));
            }
        }
    }

    /// Fails on a missing or stale lock, or on an installed copy, copy
    /// fallback or link that does not match the lock, unless it is a copy
    /// chest's record holds, placed for another lock and unchanged since.
    /// Whether something is to be put back: such a copy, or something the
    /// lock pins that is missing. When nothing is, every entry is reported
    /// unchanged.
    fn check_frozen(&self, report: &mut Report) -> Result<bool, ChestError> {
        let (name, install) = (self.label(), self.install_command());
        let lock = self
            .lock
            .as_ref()
            .ok_or_else(|| invalid(format!("{name} has no lock: run `{install}`")))?;
        let drift = lock.drift(&self.manifest);
        if !drift.is_empty() {
            return Err(invalid(format!(
                "The lock of {name} is out of date for {}: run `{install}`",
                drift.join(", ")
            )));
        }
        let problems = self.problems(lock);
        let record = self.record();
        let (outdated, wrong): (Vec<_>, Vec<_>) = problems
            .iter()
            .filter(|(_, _, problem)| !matches!(problem, Problem::Missing | Problem::NoSource))
            .partition(|(_, path, problem)| {
                *problem == Problem::Mismatch && record.held(path) == Held::Owned
            });
        if !wrong.is_empty() {
            let wrong: Vec<String> = wrong.into_iter().map(describe).collect();
            return Err(invalid(wrong.join("\n")));
        }
        // A path entry whose source is gone fails as it is staged.
        let missing = !outdated.is_empty()
            || self.entries().any(|(section, entry_name, entry)| {
                let locked = lock.section(section).iter();
                let mut found = locked.filter(|locked| &locked.name == entry_name);
                self.no_source(entry_name, entry)
                    || found.any(|locked| !self.present(section, locked))
            });
        if !missing {
            for (section, entry_name, _) in self.entries() {
                let id = format!("{}.{entry_name}", section.key());
                report.entries.push((id, Outcome::Unchanged));
            }
        }
        Ok(missing)
    }

    /// Each installed copy of a locked git or npm entry that does not match
    /// the lock, as [`Project::copy_problems`] finds them, each copy of a
    /// path entry, which has no hash, or of a `[patch]` entry, which is not
    /// hashed, that chest's record says changed or is gone, and each path
    /// entry's source that is gone.
    fn problems(&self, lock: &Lock) -> Vec<(String, PathBuf, Problem)> {
        let record = self.record();
        let mut problems = Vec::new();
        for section in [Section::Skills, Section::Agents, Section::Extensions] {
            for locked in lock.section(section) {
                problems.extend(self.copy_problems(section, locked));
                let path_entry = locked.git.is_none() && locked.npm.is_none();
                let patched = self.manifest.patch.contains_key(&locked.name);
                if path_entry || patched {
                    let id = format!("{}.{}", section.key(), locked.name);
                    // A patch's source is the patch, not the entry's `path`.
                    let source = locked.path.as_deref().filter(|_| path_entry && !patched);
                    let source = source.map(|path| self.resolve(path));
                    if let Some(source) = source.filter(|source| !source.exists()) {
                        problems.push((id.clone(), source, Problem::NoSource));
                    }
                    let changed = self.changed(section, locked, &record).into_iter();
                    problems.extend(changed.map(|(path, problem)| (id.clone(), path, problem)));
                }
            }
        }
        problems
    }

    /// Each installed copy of `locked`, a git or npm entry of `section`,
    /// that does not match the lock: canonical copies, copy fallbacks and
    /// store folders by their hash, links by what they point to. `[patch]`
    /// entries are not hashed.
    fn copy_problems(&self, section: Section, locked: &Locked) -> Vec<(String, PathBuf, Problem)> {
        let mut problems = Vec::new();
        if self.manifest.patch.contains_key(&locked.name) {
            return problems;
        }
        let homes = self.homes(section);
        let id = format!("{}.{}", section.key(), locked.name);
        for (item, hash) in self.locked_items(section, locked) {
            let Some(hash) = hash else { continue };
            let canonical = homes.first().map(|home| home.join(&item));
            for (index, home) in homes.iter().enumerate() {
                let path = home.join(&item);
                let problem = match path.symlink_metadata() {
                    Err(_) => Some(Problem::Missing),
                    Ok(metadata) if metadata.is_symlink() => {
                        let target = path.canonicalize().ok();
                        let canonical = canonical.as_ref().and_then(|c| c.canonicalize().ok());
                        (index == 0 || target != canonical)
                            .then(|| Problem::Link(homes[0].join(&item)))
                    }
                    Ok(_) => {
                        (tree_hash(&path).ok().as_ref() != Some(&hash)).then_some(Problem::Mismatch)
                    }
                };
                if let Some(problem) = problem {
                    problems.push((id.clone(), path, problem));
                }
            }
        }
        problems
    }

    /// The folder installed extension `name` loads from: its `[patch]`
    /// path, its store folder, the npm package inside it, or its path
    /// source. `None` for an entry that is not an installed extension.
    pub fn installed_root(&self, name: &str) -> Option<PathBuf> {
        self.extension_roots(name)
            .map(|(root, _)| root)
            .filter(|root| root.exists())
    }

    /// Where extension `name` loads from, as [`Project::installed_root`]
    /// says whether or not it exists, and the folder that holds the package
    /// and its own dependencies, which it may load modules from: the
    /// `[patch]` or path source, or `<store>/<name>` for git and npm.
    pub fn extension_roots(&self, name: &str) -> Option<(PathBuf, PathBuf)> {
        let entry = self.manifest.extensions.get(name)?;
        let patch = self
            .manifest
            .patch
            .get(name)
            .and_then(|patch| patch.path.as_deref());
        let store = self.store().join(name);
        Some(
            match (patch, &entry.npm, &entry.git, entry.path.as_deref()) {
                (Some(path), ..) | (None, None, None, Some(path)) => {
                    let root = self.resolve(path);
                    (root.clone(), root)
                }
                (None, Some(npm), _, _) => (store.join("node_modules").join(npm), store),
                (None, None, Some(_), _) => (store.clone(), store),
                (None, None, None, None) => return None,
            },
        )
    }

    /// For [`Mode::Startup`]: when nothing must be put back, because every
    /// locked, current entry is present or `offline` leaves it missing, the
    /// report of what a sync would find, without taking the lock's lock.
    /// Whether it gave the report.
    fn startup_without_lock(
        &mut self,
        offline: bool,
        report: &mut Report,
    ) -> Result<bool, ChestError> {
        self.lock = self.read_lock()?;
        let Some(lock) = &self.lock else {
            report.warnings.push(format!(
                "{} has no lock, so nothing from it was installed: run `{}`",
                self.label(),
                self.install_command()
            ));
            return Ok(true);
        };
        let record = self.record();
        let mut entries = Vec::new();
        let mut stale = Vec::new();
        let (mut missing, mut loading) = (Vec::new(), Vec::new());
        for (section, name, entry) in self.entries() {
            let locked = lock
                .section(section)
                .iter()
                .find(|locked| &locked.name == name);
            let id = format!("{}.{name}", section.key());
            let outcome = match self.at_startup(section, name, entry, locked, &record, offline) {
                AtStartup::Stale => {
                    stale.push(id.clone());
                    Outcome::Skipped
                }
                AtStartup::Present => Outcome::Unchanged,
                AtStartup::Missing => {
                    match locked.is_some_and(|locked| self.read_by_yapi(section, locked)) {
                        true => loading.push(id.clone()),
                        false => missing.push(id.clone()),
                    }
                    Outcome::Skipped
                }
                AtStartup::Replay => return Ok(false),
            };
            entries.push((id, outcome));
        }
        report.entries = entries;
        if !stale.is_empty() {
            report.warnings.push(self.stale_warning(&stale));
        }
        let label = self.label();
        if !missing.is_empty() {
            let skipped = if missing.len() == 1 {
                "it was"
            } else {
                "they were"
            };
            report.warnings.push(format!(
                "{label}: {} could not be put back offline, so {skipped} left out",
                missing.join(", ")
            ));
        }
        if !loading.is_empty() {
            let copies = match loading.len() {
                1 => "its copy in a folder yapi reads on its own keeps",
                _ => "their copies in folders yapi reads on its own keep",
            };
            report.warnings.push(format!(
                "{label}: {} could not be put back offline, so {copies} loading",
                loading.join(", ")
            ));
        }
        Ok(true)
    }

    /// Whether yapi loads a copy of skill entry `locked` on its own, beside
    /// what chest gives it: one of its items is in a skills folder of
    /// [`Project::homes`] that yapi reads anyway, such as `.agents/skills`.
    fn read_by_yapi(&self, section: Section, locked: &Locked) -> bool {
        if section != Section::Skills {
            return false;
        }
        let own = match self.scope {
            Scope::Project => [".agents/skills", ".yapi/skills"].map(|dir| self.base.join(dir)),
            Scope::Global => [
                self.home.join(".agents/skills"),
                crate::config::agent_dir().join("skills"),
            ],
        };
        let own = own.map(|folder| real_path(&folder));
        let homes = self.homes(section);
        let items = self.locked_items(section, locked);
        let read = homes.iter().filter(|home| own.contains(home));
        read.flat_map(|home| items.iter().map(move |(item, _)| home.join(item)))
            .any(|copy| copy.exists())
    }

    /// What startup does with entry `name` of `section`, locked as
    /// `locked`: a stale or unlocked entry is skipped, a patched one or one
    /// in place as the lock pins it, by `record`, kept, and a missing or
    /// outdated one put back, unless `offline`.
    fn at_startup(
        &self,
        section: Section,
        name: &str,
        entry: &Entry,
        locked: Option<&Locked>,
        record: &Record,
        offline: bool,
    ) -> AtStartup {
        let patched = self.manifest.patch.contains_key(name);
        if !patched && !locked.is_some_and(|locked| fresh(entry, locked)) {
            return AtStartup::Stale;
        }
        // A patched entry loads from its patch, or where `chest install` put it.
        let pinned = |locked: &Locked| {
            self.present(section, locked) && self.holds_locked(section, entry, locked, record)
        };
        if patched || locked.is_some_and(pinned) {
            return AtStartup::Present;
        }
        if offline {
            AtStartup::Missing
        } else {
            AtStartup::Replay
        }
    }

    /// The warning for entries `ids` that are not locked, or whose lock is
    /// stale.
    fn stale_warning(&self, ids: &[String]) -> String {
        let (verb, lock, skipped) = match ids.len() {
            1 => ("is", "its", "it was"),
            _ => ("are", "their", "they were"),
        };
        format!(
            "{}: {} {verb} not locked, or {lock} lock is stale, so {skipped} skipped: run `{}`",
            self.label(),
            ids.join(", "),
            self.install_command()
        )
    }

    /// Checks every installed copy of a locked git or npm entry: canonical
    /// copies, copy fallbacks and store folders by their hash, and links by
    /// what they point to. Copies of path and `[patch]` entries are checked
    /// by chest's record of what it placed, and each path entry's source
    /// must be there. Returns one line per problem, so none means all match.
    pub fn verify(&self) -> Result<Vec<String>, ChestError> {
        let (name, install) = (self.label(), self.install_command());
        let lock = self
            .lock
            .as_ref()
            .ok_or_else(|| invalid(format!("{name} has no lock: run `{install}`")))?;
        Ok(self.problems(lock).iter().map(describe).collect())
    }

    /// The newest revision each locked git or npm entry's source offers:
    /// the commit its `rev` names now, or the newest npm version in its
    /// range with npm's `latest`.
    pub async fn outdated(&self, npm: &mut Npm) -> Result<Vec<Outdated>, ChestError> {
        let Some(lock) = &self.lock else {
            return Ok(Vec::new());
        };
        let mut found = Vec::new();
        for (section, name, entry) in self.entries() {
            let Some(locked) = lock
                .section(section)
                .iter()
                .find(|locked| &locked.name == name)
            else {
                continue;
            };
            let id = format!("{}.{name}", section.key());
            let failed = |err: ChestError| ChestError::Failed(format!("{id}: {err}"));
            if let Some(repo) = &entry.git {
                let wanted = fetch::resolve(&self.cache, repo, entry.rev.as_deref())
                    .await
                    .map_err(failed)?;
                let locked = locked.commit.clone().unwrap_or_default();
                found.push(Outdated {
                    id,
                    locked,
                    wanted,
                    latest: None,
                });
            } else if let Some(package) = &entry.npm {
                let range = entry.version.as_deref().unwrap_or("latest");
                let wanted = npm
                    .latest(package, range)
                    .await
                    .map_err(|err| failed(err.into()))?;
                let latest = npm.latest(package, "latest").await.ok();
                let locked = locked.version.clone().unwrap_or_default();
                found.push(Outdated {
                    id,
                    locked,
                    wanted,
                    latest,
                });
            }
        }
        Ok(found)
    }

    /// Fetches the package `entry` names, without its dependencies, to read
    /// what it provides and requests before it is added: a git source at the
    /// commit its `rev` names, an npm source at the newest version in its
    /// range, or a path source where it is.
    pub async fn fetch_package(&self, entry: &Entry, npm: &mut Npm) -> Result<Fetched, ChestError> {
        if entry.git.is_none() && entry.npm.is_none() {
            let path = entry
                .path
                .as_deref()
                .ok_or_else(|| invalid("needs `git`, `npm` or `path`"))?;
            return Ok(Fetched {
                root: self.resolve(path),
                _work: None,
            });
        }
        let mut work = fetch::Work::new(&self.cache)?;
        let root = work.path();
        if let Some(repo) = &entry.git {
            let commit = fetch::resolve(&self.cache, repo, entry.rev.as_deref()).await?;
            fetch::checkout(&self.cache, repo, &commit, &root).await?;
        } else if let Some(package) = &entry.npm {
            npm::fetch(npm, package, entry.version.as_deref(), &root).await?;
        }
        Ok(Fetched {
            root,
            _work: Some(work),
        })
    }
}

/// What is wrong with one installed copy.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Problem {
    /// Nothing is there.
    Missing,
    /// A copy whose hash differs from the lock's.
    Mismatch,
    /// A copy of a path or `[patch]` entry that changed since chest placed it.
    Modified,
    /// A link that does not point at the canonical copy, given.
    Link(PathBuf),
    /// A path entry's source, which is gone.
    NoSource,
}

/// What `sync` says while another chest command holds the lock of `file`.
fn waiting_for(file: &Path) -> String {
    format!(
        "Waiting for another chest command, which holds the lock of {}...",
        file.display()
    )
}

fn describe((id, path, problem): &(String, PathBuf, Problem)) -> String {
    let what = match problem {
        Problem::NoSource => return format!("{id}: its source {} is missing", path.display()),
        Problem::Missing => "is missing".to_owned(),
        Problem::Mismatch => "does not match the lock".to_owned(),
        Problem::Modified => "was modified since chest installed it".to_owned(),
        Problem::Link(canonical) => format!("does not link to {}", canonical.display()),
    };
    format!("{id}: {} {what}", path.display())
}

/// Whether `path`, or what it links to, has a `.git` anywhere inside. Its
/// hash leaves `.git` out, so a matching hash says nothing about it.
fn has_git(path: &Path) -> bool {
    fn walk(dir: &Path) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        entries.flatten().any(|entry| {
            entry.file_name() == ".git"
                || entry.file_type().is_ok_and(|kind| kind.is_dir()) && walk(&entry.path())
        })
    }
    let held = match path.symlink_metadata() {
        Ok(metadata) if metadata.is_symlink() => path.canonicalize().ok(),
        Ok(_) => Some(path.to_path_buf()),
        Err(_) => None,
    };
    held.is_some_and(|held| walk(&held))
}

/// Copies the checkout `package`, or its `subpath`, into `staging`, without
/// a committed `node_modules`, which the dependency install or replay
/// rebuilds so that the hash does not depend on what the repository commits.
/// The subpath must be a folder: an extension is installed as a package.
fn copy_checkout(package: &Path, subpath: Option<&str>, staging: &Path) -> Result<(), ChestError> {
    let source = package.join(subpath.unwrap_or_default());
    fetch::inside(package, &source)?;
    if !source.is_dir() {
        return Err(invalid(format!(
            "`path` must name the folder that holds the extension, not a file: {}",
            subpath.unwrap_or_default()
        )));
    }
    place::copy(&source, staging)?;
    let modules = staging.join("node_modules");
    if modules.symlink_metadata().is_ok() {
        std::fs::remove_dir_all(&modules).map_err(io(&modules))?;
    }
    Ok(())
}
