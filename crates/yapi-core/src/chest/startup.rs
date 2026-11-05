//! What yapi loads from chest manifests: the installed extensions, with the
//! grants, event policy and module root each runs with, and the placed
//! skills.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use yapi_types::settings::FilteredPackage;

use super::fetch::placed_name;
use super::lock::fresh;
use super::manifest::{Entry, MANIFEST_ENV, Manifest, Section, from_env};
use super::{Mode, Project, locked_items};
use crate::extensions::{EventPolicy, Grants};
use crate::packages::npm::{Npm, escaped};
use crate::packages::resolve::{
    PackageInput, ProjectOnly, ResolvedPaths, ResourceType, package_resources,
};
use crate::settings::{Scope, SettingsManager};
use crate::tools::path::home_dir;

/// The project manifest of `project_root`: the file [`MANIFEST_ENV`] names
/// relative to it, else `chest.toml`. `None` when the variable names a file
/// outside the project, as chest refuses it too. Its lock is the file beside
/// it named with `.lock`.
pub fn project_manifest(project_root: &Path) -> Option<PathBuf> {
    let name = from_env().ok()?;
    Some(project_root.join(name.as_deref().unwrap_or("chest.toml")))
}

/// Whether `manifest` installs anything, beyond describing its folder as a
/// package with `[package]`.
fn consumes(manifest: &Manifest) -> bool {
    [Section::Skills, Section::Agents, Section::Extensions]
        .into_iter()
        .any(|section| !manifest.section(section).is_empty())
        || !manifest.patch.is_empty()
}

/// Whether the project manifest of `project_root` installs anything, so it
/// needs the project's trust as other project files that run code do. One
/// with only `[package]`, which describes the project as a package, does not.
/// A manifest that does not parse needs trust too.
pub fn needs_trust(project_root: &Path) -> bool {
    let Some(Ok(text)) = project_manifest(project_root).map(std::fs::read_to_string) else {
        return false;
    };
    Manifest::parse(&text, Scope::Project).map_or(true, |manifest| consumes(&manifest))
}

/// What an extension runs with: its grants, its `pi.events` policy, and the
/// folder it loads modules from without file access. It has no default, so
/// every file says which one it gets.
#[derive(Clone, Debug, PartialEq)]
pub struct Policy {
    /// What it may reach.
    pub grants: Grants,
    /// The channels it may send to, and hear from, extensions whose grants
    /// differ.
    pub events: EventPolicy,
    /// The folder that holds its package and the package's own
    /// dependencies.
    pub root: Option<PathBuf>,
}

impl Policy {
    /// What a package from `yapi install` runs with, as in pi: every grant
    /// and the shared event policy.
    pub fn unrestricted() -> Policy {
        Policy {
            grants: Grants::default(),
            events: EventPolicy::shared(),
            root: None,
        }
    }

    /// What a file chest cannot vouch for runs with: no grant, the
    /// isolated event policy, and modules only from `root`, a folder or
    /// the file itself.
    pub fn closed(root: &Path) -> Policy {
        Policy {
            grants: Grants::none(),
            events: EventPolicy::isolated(),
            root: Some(root.to_path_buf()),
        }
    }

    /// What both `self` and `other` allow: the grants and channels of both,
    /// and the narrower of two nested module roots, or none.
    fn intersection(&self, other: &Policy) -> Policy {
        let canonical = |root: &PathBuf| root.canonicalize().unwrap_or_else(|_| root.clone());
        let roots = (
            self.root.as_ref().map(canonical),
            other.root.as_ref().map(canonical),
        );
        let root = match roots {
            (Some(left), Some(right)) if left.starts_with(&right) => Some(left),
            (Some(left), Some(right)) if right.starts_with(&left) => Some(right),
            _ => None,
        };
        Policy {
            grants: self.grants.intersection(other.grants),
            events: self.events.intersection(&other.events),
            root,
        }
    }
}

/// The policies of chest's packages, by the scope and source the resolver
/// gives their files, such as `("project", "chest:todo")`.
/// [`installed_inputs`] builds the packages and their policies in one place,
/// so they always match.
pub type Policies = HashMap<(String, String), Policy>;

/// The policies of extension files, so a policy applies whichever source of
/// the same file loads it.
#[derive(Clone, Debug, Default)]
pub struct FilePolicies {
    /// The policy of each file of chest's packages, by canonical path: what
    /// every entry that gives it allows.
    pub files: HashMap<PathBuf, Policy>,
    /// The canonical folders and files of entries chest left out, whose
    /// files run with no grant, whatever loads them.
    pub closed: Vec<PathBuf>,
}

impl FilePolicies {
    /// What extension file `file`, a canonical path, runs with: no grant
    /// and no module but itself in an entry chest left out, else its
    /// entries' policy, and for a file chest does not know, every grant, as
    /// packages from `yapi install` keep pi's grants.
    pub fn of(&self, file: &Path) -> Policy {
        // Not the entry's folder as the root, which may hold far more.
        if self.closes(file) {
            return Policy::closed(file);
        }
        self.files
            .get(file)
            .cloned()
            .unwrap_or_else(Policy::unrestricted)
    }

    /// Whether chest decides what `file` runs with.
    pub fn manages(&self, file: &Path) -> bool {
        self.files.contains_key(file) || self.closes(file)
    }

    /// Whether `file` belongs to an entry chest left out.
    pub fn closes(&self, file: &Path) -> bool {
        self.closed.iter().any(|root| file.starts_with(root))
    }

    /// The warning for extension file `file`, a canonical path, that
    /// `source` names by `path` though chest decides what it runs with, as
    /// the path and message to show, or `None` when chest does not. Both are
    /// escaped, since a cloned project's file names can hold anything.
    pub fn also_loaded(&self, file: &Path, path: &str, source: &str) -> Option<(String, String)> {
        if !self.manages(file) {
            return None;
        }
        let message = match self.closes(file) {
            true => format!(
                "{path} is also loaded from {source}, but its chest entry does not load, so it runs with no capabilities"
            ),
            false => format!("{path} is also loaded from {source}; chest's grants apply"),
        };
        Some((escaped(path), escaped(&message)))
    }
}

/// The policy of each extension file the resolver gave a `chest:` source in
/// `resolved`, loaded or shadowed, from its package's entry in
/// `installed`, with the entries `installed` left out. A file that several
/// entries give gets what all of them allow. A chest file whose package has
/// no entry runs with no grant and the isolated event policy, and adds a
/// line to `problems` under its source.
pub fn file_policies(
    resolved: &ResolvedPaths,
    installed: &Installed,
    problems: &mut Vec<(String, String)>,
) -> FilePolicies {
    let files = resolved.of(ResourceType::Extensions).iter();
    let files = files.map(|file| &file.info).chain(&resolved.shadowed);
    let mut found = FilePolicies {
        files: HashMap::new(),
        closed: installed.closed.clone(),
    };
    for info in files.filter(|info| info.source.starts_with("chest:")) {
        let key = (info.scope.clone(), info.source.clone());
        let file = Path::new(&info.path);
        let file = file.canonicalize().unwrap_or_else(|_| file.into());
        let policy = installed.policies.get(&key).cloned().unwrap_or_else(|| {
            let message = format!(
                "{}: no chest entry grants this file, so it runs with no capabilities",
                info.path
            );
            problems.push((escaped(&info.source), escaped(&message)));
            Policy::closed(&file)
        });
        let policy = match found.files.get(&file) {
            Some(other) => other.intersection(&policy),
            None => policy,
        };
        found.files.insert(file, policy);
    }
    found
}

/// What the installed entries of chest manifests give yapi.
#[derive(Clone, Debug, Default)]
pub struct Installed {
    /// Extension packages, each as source `chest:<name>`.
    pub packages: Vec<PackageInput>,
    /// The canonical copies of placed skills, each a skill folder, with the
    /// source and scope of its entry.
    pub skills: Vec<PackageInput>,
    /// The packages' policies.
    pub policies: Policies,
    /// The canonical folders and files of extension entries left out
    /// because their lock is stale or missing, and the extension files of
    /// those whose folder holds too much, which run with no grant if
    /// something else loads them.
    pub closed: Vec<PathBuf>,
    /// The package identity of each loaded npm or git extension entry, such
    /// as `npm:@acme/todo`, with the entry's name, its manifest's path, and
    /// whether that is the user's global manifest.
    pub identities: Vec<(String, String, String, bool)>,
}

impl Installed {
    fn extend(&mut self, other: Installed) {
        self.packages.extend(other.packages);
        self.skills.extend(other.skills);
        self.policies.extend(other.policies);
        self.closed.extend(other.closed);
        self.identities.extend(other.identities);
    }
}

/// Whether a run reads the user's global manifest: always, except in a
/// project-only run whose `projectOnly` setting in `settings` can keep none
/// of its entries, so nothing global is put back for nothing.
pub fn reads_global(settings: &SettingsManager) -> bool {
    !crate::config::project_only()
        || crate::packages::resolve::ProjectOnly::from_settings(settings.document(Scope::Global))
            .needs_chest()
}

/// Problems, by manifest path, for each package that two managers install
/// so that both copies load: a chest extension entry and a `packages` entry
/// of `settings`, or entries of the project and the global manifest. With
/// `keep`, a project-only run's keep list, a global copy it leaves out
/// doesn't count.
pub fn duplicates(
    installed: &Installed,
    settings: &SettingsManager,
    keep: Option<&ProjectOnly>,
) -> Vec<(String, String)> {
    let loads = |source: &str, global: bool| {
        !global || keep.is_none_or(|keep| keep.keeps_package(source, Path::new("")))
    };
    let identities: Vec<_> = installed
        .identities
        .iter()
        .filter(|(_, name, _, global)| loads(&format!("chest:{name}"), *global))
        .collect();
    let mut problems = Vec::new();
    for scope in [Scope::Project, Scope::Global] {
        for package in crate::packages::settings_packages(settings, scope) {
            let Some(source) = crate::packages::entry_source(&package) else {
                continue;
            };
            if !loads(source, scope == Scope::Global) {
                continue;
            }
            let id = crate::packages::source::parse(source).identity(Path::new(""));
            for (other, name, path, _) in &identities {
                if *other == id {
                    problems.push((
                        path.clone(),
                        format!(
                            "extensions.{name} is also in the packages of settings as {source}, so both copies load: remove one of them"
                        ),
                    ));
                }
            }
        }
    }
    for (index, (id, name, path, _)) in identities.iter().enumerate() {
        let earlier = identities[..index]
            .iter()
            .find(|(other, _, other_path, _)| other == id && other_path != path);
        if let Some((_, other, other_path, _)) = earlier {
            problems.push((
                path.clone(),
                format!(
                    "extensions.{name} installs the same package as extensions.{other} in {other_path}, so both copies load: remove one of them"
                ),
            ));
        }
    }
    problems
        .into_iter()
        .map(|(path, message)| (escaped(&path), escaped(&message)))
        .collect()
}

/// What chest gives one run of yapi, with the problems to show by manifest
/// path: warnings and notices of the startup sync, entries left out, and
/// manifests that fail to load.
#[derive(Clone, Debug, Default)]
pub struct Loaded {
    /// The installed entries.
    pub installed: Installed,
    /// Each problem's manifest path and message.
    pub problems: Vec<(String, String)>,
}

impl Loaded {
    /// Adds problem `message` under manifest `path`, both with their
    /// unprintable characters escaped: they quote files a cloned project
    /// holds, and reach the terminal.
    fn warn(&mut self, (path, message): (String, String)) {
        self.problems.push((escaped(&path), escaped(&message)));
    }
}

/// The extensions and skills `project` has installed, locked and up to
/// date, or patched, for a run in `project_root`: a stale or unlocked entry
/// gives nothing until `chest install`. Entries left out for a reason the
/// user should see add a line to `problems`.
pub fn installed_inputs(
    project: &Project,
    project_root: &Path,
    problems: &mut Vec<String>,
) -> Installed {
    let scope = match project.scope {
        Scope::Project => "project",
        Scope::Global => "user",
    };
    let mut installed = Installed::default();
    let lock = project.lock.clone().unwrap_or_default();
    let record = project.record();
    // A copy an older lock pinned stays out until it is put back.
    let current = |section: Section, name: &str, entry: &Entry| {
        lock.section(section)
            .iter()
            .find(|locked| locked.name == name)
            .filter(|locked| fresh(entry, locked))
            .filter(|locked| project.present(section, locked))
            .filter(|locked| project.holds_locked(section, entry, locked, &record))
            .cloned()
    };
    for (name, entry) in &project.manifest.extensions {
        let patched = project.manifest.patch.contains_key(name);
        let roots = project.extension_roots(name);
        // Locked or not, it never loads. Only the files it would load run
        // closed, as other files there belong to other sources.
        if let Some((root, module_root)) = &roots
            && project.too_wide(module_root, &[project_root])
        {
            problems.push(format!(
                "extensions.{name}: {} holds more than the extension, such as the project, the home or the agent folder, so it does not load",
                module_root.display()
            ));
            let local = crate::packages::source::is_local(&format!("chest:{name}"));
            let files = package_resources(root, None, local).extensions;
            let files = files.iter().map(|file| Path::new(&file.info.path));
            installed
                .closed
                .extend(files.filter_map(|file| file.canonicalize().ok()));
            continue;
        }
        if !patched && current(Section::Extensions, name, entry).is_none() {
            // Its files may still load through pi's discovery or settings.
            if let Some(root) = roots.and_then(|(_, root)| root.canonicalize().ok()) {
                installed.closed.push(root);
            }
            continue;
        }
        let Some((root, module_root)) = roots else {
            continue;
        };
        if !root.exists() {
            // A store folder that is missing is the sync's to put back, or
            // to warn of.
            if patched || entry.git.is_none() && entry.npm.is_none() {
                problems.push(format!(
                    "extensions.{name}: {} does not exist",
                    root.display()
                ));
            }
            continue;
        }
        let source = format!("chest:{name}");
        let filter = [&entry.skills, &entry.prompts, &entry.themes]
            .iter()
            .any(|list| list.is_some())
            .then(|| FilteredPackage {
                source: source.clone(),
                autoload: None,
                extensions: None,
                skills: entry.skills.clone(),
                prompts: entry.prompts.clone(),
                themes: entry.themes.clone(),
            });
        let policy = Policy {
            grants: project.manifest.grants(entry),
            events: entry.events.clone().unwrap_or_default(),
            root: Some(module_root),
        };
        let key = (scope.to_owned(), source.clone());
        installed.policies.insert(key, policy);
        let identity = match (&entry.npm, &entry.git) {
            (Some(npm), _) => Some(format!("npm:{npm}")),
            (None, Some(git)) => {
                let parsed = crate::packages::source::parse(&format!("git:{git}"));
                Some(parsed.identity(Path::new("")))
            }
            (None, None) => None,
        };
        if let Some(identity) = identity.filter(|_| !patched) {
            let path = project.path.display().to_string();
            let global = project.scope == Scope::Global;
            installed
                .identities
                .push((identity, name.clone(), path, global));
        }
        installed.packages.push(PackageInput {
            source,
            scope: scope.to_owned(),
            root,
            filter,
        });
    }
    // Skills load from their canonical copy, wherever the folders put it.
    let Some(home) = project.homes(Section::Skills).into_iter().next() else {
        return installed;
    };
    for (name, entry) in &project.manifest.skills {
        let patch = project.manifest.patch.get(name);
        let roots: Vec<PathBuf> = match (
            patch.and_then(|patch| patch.path.as_deref()),
            current(Section::Skills, name, entry),
        ) {
            // A patched skill loads where `chest install` placed it, else
            // from its patch. A patch of a whole package, as `chest install`
            // reads it, places each of its skills under its own name.
            (Some(path), _) => {
                let patch = project.resolve(path);
                if !patch.exists() {
                    problems.push(format!("skills.{name}: {} does not exist", patch.display()));
                    continue;
                }
                let package = entry.path.is_none() && (entry.git.is_some() || entry.npm.is_some());
                let items = if package {
                    match super::fetch::items(&patch, Section::Skills, None, name) {
                        Ok((items, _)) => items,
                        Err(err) => {
                            problems.push(format!("skills.{name}: {err}"));
                            continue;
                        }
                    }
                } else {
                    vec![(placed_name(Section::Skills, name), patch)]
                };
                items
                    .into_iter()
                    .map(|(item, from)| {
                        let placed = home.join(item);
                        if placed.exists() { placed } else { from }
                    })
                    .collect()
            }
            (None, Some(locked)) => locked_items(Section::Skills, &locked)
                .into_iter()
                .map(|(item, _)| home.join(item))
                .collect(),
            (None, None) => continue,
        };
        for root in roots {
            installed.skills.push(PackageInput {
                source: format!("chest:{name}"),
                scope: scope.to_owned(),
                root,
                filter: None,
            });
        }
    }
    installed
}

/// The manifests yapi reads for `project_root` that install anything: the
/// project's, only when `trusted`, then the user's global one, only when
/// `global`, or why each cannot be read.
fn manifests(
    project_root: &Path,
    trusted: bool,
    global: bool,
) -> Vec<Result<Project, (String, String)>> {
    let mut found = Vec::new();
    if trusted && let Err(problem) = from_env() {
        let problem = format!("{problem}, so no project manifest loads");
        found.push(Err((MANIFEST_ENV.to_owned(), problem)));
    }
    let project = trusted.then(|| project_manifest(project_root)).flatten();
    let project = project.map(|path| (path, Scope::Project));
    let global = global.then(|| (home_dir().join(".chest/chest.toml"), Scope::Global));
    for (path, scope) in [project, global].into_iter().flatten() {
        if !path.exists() {
            continue;
        }
        match Project::open(&path, scope) {
            Ok(project) if consumes(&project.manifest) => found.push(Ok(project)),
            Ok(_) => {}
            Err(err) => found.push(Err((path.display().to_string(), err.to_string()))),
        }
    }
    found
}

/// Adds the inputs of `project` for a run in `project_root` to `loaded`,
/// with its problems under its path.
fn add(loaded: &mut Loaded, project: &Project, project_root: &Path) {
    let mut problems = Vec::new();
    loaded
        .installed
        .extend(installed_inputs(project, project_root, &mut problems));
    let path = project.path.display().to_string();
    // Print mode shows only the message, so it names its manifest.
    for problem in problems {
        loaded.warn((path.clone(), format!("{}: {problem}", project.label())));
    }
}

/// What chest gives a run in `project_root`, as [`load`] would, without
/// installing anything: for `yapi config`, which shows what a run loads.
pub fn installed(project_root: &Path, trusted: bool, global: bool) -> Loaded {
    let mut loaded = Loaded::default();
    for project in manifests(project_root, trusted, global) {
        match project {
            Ok(project) => add(&mut loaded, &project, project_root),
            Err(problem) => loaded.warn(problem),
        }
    }
    loaded
}

/// What chest gives a run of yapi in `project_root`: the project manifest,
/// only when `trusted`, and the user's global one, only when `global`, each
/// when it installs anything. Each puts back what its lock pins and is
/// missing, unless `offline`, and skips stale or unlocked entries with a
/// warning. It never writes a lock or `.gitignore`.
pub async fn load(project_root: &Path, trusted: bool, offline: bool, global: bool) -> Loaded {
    let mut loaded = Loaded::default();
    let mut npm = None;
    for project in manifests(project_root, trusted, global) {
        let mut project = match project {
            Ok(project) => project,
            Err(problem) => {
                loaded.warn(problem);
                continue;
            }
        };
        let path = project.path.display().to_string();
        let npm = npm.get_or_insert_with(|| Npm::new(crate::packages::npm::config()));
        match project
            .sync(Mode::Startup { offline }, npm, &mut |_| {})
            .await
        {
            Ok(report) => {
                for message in report.warnings.into_iter().chain(report.notices) {
                    loaded.warn((path.clone(), message));
                }
            }
            Err(err) => loaded.warn((path.clone(), err.to_string())),
        }
        add(&mut loaded, &project, project_root);
    }
    loaded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packages::resolve::Resolved;
    use yapi_types::rpc::SourceInfo;

    /// yapi's startup runs `load` in futures that move between threads.
    #[test]
    fn startup_load_send() {
        fn send<T: Send>(_: T) {}
        send(load(Path::new("."), false, true, true));
    }

    /// A chest file whose package has no policy runs with none, with a
    /// warning, while other sources' files get no policy at all. Warnings
    /// escape a file name's control characters. A file
    /// that two entries give loads modules from the narrower root, and one
    /// in an entry chest left out from itself alone.
    #[test]
    fn chest_files_fail_closed() {
        let file = |path: &str, source: &str, scope: &str| SourceInfo {
            path: path.into(),
            source: source.into(),
            scope: scope.into(),
            origin: "package".into(),
            base_dir: None,
        };
        let enabled = |info| Resolved {
            info,
            enabled: true,
        };
        // A cloned project's file name may hold a terminal escape.
        let spoof = "/p/\x1b[2J.js";
        let resolved = ResolvedPaths {
            extensions: vec![
                enabled(file("/p/a.js", "chest:a", "project")),
                enabled(file("/p/b.js", "npm:b", "project")),
                enabled(file(spoof, "chest:a", "project")),
            ],
            ..ResolvedPaths::default()
        };
        let mut problems = Vec::new();
        let policies = file_policies(&resolved, &Installed::default(), &mut problems);
        let (file_a, file_spoof) = (PathBuf::from("/p/a.js"), PathBuf::from(spoof));
        let files = HashMap::from([
            (file_a.clone(), Policy::closed(&file_a)),
            (file_spoof.clone(), Policy::closed(&file_spoof)),
        ]);
        assert_eq!(policies.files, files);
        assert_eq!(problems.len(), 2, "{problems:?}");
        let warned = policies.also_loaded(&file_spoof, spoof, "cli");
        let warned = warned
            .into_iter()
            .flat_map(|(path, message)| [path, message]);
        let texts: Vec<String> = problems
            .iter()
            .flat_map(|(source, message)| [source.clone(), message.clone()])
            .chain(warned)
            .collect();
        assert!(
            texts.len() == 6 && texts.iter().all(|text| !text.contains('\x1b')),
            "{texts:?}"
        );
        let rooted = |root: &str| Policy {
            root: Some(root.into()),
            ..Policy::unrestricted()
        };
        let key = |scope: &str| (scope.to_owned(), "chest:s".to_owned());
        let installed = Installed {
            policies: HashMap::from([
                (key("project"), rooted("/p")),
                (key("user"), rooted("/p/sub")),
            ]),
            closed: vec!["/q".into()],
            ..Installed::default()
        };
        let resolved = ResolvedPaths {
            extensions: vec![
                enabled(file("/p/sub/a.js", "chest:s", "project")),
                enabled(file("/p/sub/b.js", "chest:s", "user")),
            ],
            shadowed: vec![
                file("/p/sub/a.js", "chest:s", "user"),
                file("/p/sub/b.js", "chest:s", "project"),
            ],
            ..ResolvedPaths::default()
        };
        let policies = file_policies(&resolved, &installed, &mut problems);
        let root = |path: &str| policies.of(Path::new(path)).root;
        for file in ["/p/sub/a.js", "/p/sub/b.js"] {
            assert_eq!(root(file), Some("/p/sub".into()), "{file}");
        }
        assert_eq!(root("/q/a.js"), Some("/q/a.js".into()));
    }
}
