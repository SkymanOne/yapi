//! `chest import`: entries from settings.json packages or from skills.sh's
//! `skills-lock.json`. It writes the manifest and leaves the sources alone.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use clap::Args;
use serde_json::Value;
use yapi_core::chest::manifest::{Entry, ManifestDocument, Section, valid_name};
use yapi_core::packages::source::{self, Source};
use yapi_core::settings::{Scope, SettingsManager};
use yapi_core::tools::path::relative;
use yapi_core::trust::{TrustStore, resolve_trusted};
use yapi_types::settings::PackageSource;

use crate::{Target, out, say, write};

/// `chest import` options: one of the two sources, and its file.
#[derive(Args)]
#[command(group(clap::ArgGroup::new("from").required(true).args(["settings", "skills_lock"])))]
pub struct ImportArgs {
    /// Import the packages of settings.json or settings.toml: the project's
    /// `.yapi`, or the agent folder's with `-g`.
    #[arg(long)]
    settings: bool,
    /// Import the skills of a skills.sh `skills-lock.json`.
    #[arg(long)]
    skills_lock: bool,
    /// The file to read instead of the default.
    path: Option<PathBuf>,
}

/// Runs `chest import`.
pub fn run(target: &Target, args: &ImportArgs) -> anyhow::Result<()> {
    let text = if target.path.exists() {
        crate::read(target)?
    } else {
        String::new()
    };
    let mut document = ManifestDocument::parse(&text, target.scope)?;
    let added = if args.settings {
        let path = match args.path.clone() {
            Some(path) => path,
            None => yapi_core::settings::settings_file(&match target.scope {
                Scope::Project => target.base().join(".yapi"),
                Scope::Global => yapi_core::config::agent_dir(),
            })?,
        };
        settings(target, &path, &mut document)?
    } else {
        let path = args
            .path
            .clone()
            .unwrap_or_else(|| target.base().join("skills-lock.json"));
        skills_lock(&path, &target.base(), &mut document)?
    };
    write(target, &document.to_string())?;
    let (entries, them) = match added {
        1 => ("entry", "it"),
        _ => ("entries", "them"),
    };
    let home = yapi_core::tools::path::home_dir();
    let (manifest, install) = (
        yapi_core::chest::manifest_label(&target.path, target.scope, &home),
        yapi_core::chest::install_command(&target.path, target.scope),
    );
    say(&format!(
        "Added {added} {entries} to {manifest}. Run `{install}` to install {them}."
    ));
    Ok(())
}

fn json(path: &Path) -> anyhow::Result<Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    yapi_types::json::parse(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Adds `entry` as `name` in `section`, unless the manifest has its
/// package or another package holds the name. Then it warns, naming the
/// entry as `what`, and adds nothing. Paths resolve against `base`.
fn add(
    document: &mut ManifestDocument,
    (section, name): (Section, &str),
    entry: &Entry,
    what: &str,
    base: &Path,
) -> anyhow::Result<bool> {
    let manifest = document.manifest();
    let entries = || {
        Section::ALL.into_iter().flat_map(|section| {
            let entries = manifest.section(section).iter();
            entries.map(move |(name, entry)| (section, name.as_str(), entry))
        })
    };
    let package = identity(entry, base);
    // A patch names an entry's package, not one of its own.
    let same = entries().find(|&(section, _, other)| {
        section != Section::Patch && package.is_some() && identity(other, base) == package
    });
    let reason = match same {
        Some((section, other, _)) => Some(format!(
            "the manifest already has it as {}.{other}",
            section.key()
        )),
        None => entries()
            .find(|&(_, other, _)| other == name)
            .map(|(section, ..)| {
                format!("the manifest's {}.{name} is another package", section.key())
            }),
    };
    if let Some(reason) = reason {
        say(&format!("warning: skipped {what}: {reason}"));
        return Ok(false);
    }
    document.set_entry(section, name, entry)?;
    Ok(true)
}

/// The settings packages as extensions with every capability, as `yapi
/// install` gives them, and their `skills`, `prompts` and `themes` filters.
/// A package with a filter chest cannot express is left out.
fn settings(
    target: &Target,
    path: &Path,
    document: &mut ManifestDocument,
) -> anyhow::Result<usize> {
    // Either form of the settings file, as yapi reads it.
    if !path.exists() {
        bail!("reading {}: No such file or directory", path.display());
    }
    let settings = Value::Object(
        yapi_core::settings::read_settings(path)
            .with_context(|| format!("parsing {}", path.display()))?,
    );
    let packages: Vec<PackageSource> = match settings.get("packages") {
        Some(packages) => serde_json::from_value(packages.clone()).context("reading `packages`")?,
        None => Vec::new(),
    };
    // Sources resolve as yapi resolves them: against the settings' folder.
    let folder = std::path::absolute(path)?
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    // Messages name the file read, `settings.json` or `settings.toml`.
    let file = path.file_name().unwrap_or_default().to_string_lossy();
    let base = target.base();
    // `yapi remove -l` changes the settings of a project yapi trusts, and
    // `--approve` trusts it for that command.
    let remove = match target.scope {
        Scope::Global => "yapi remove",
        Scope::Project => {
            let agent_dir = yapi_core::config::agent_dir();
            let settings = SettingsManager::load(&agent_dir, &base, false);
            let default = settings.settings().default_project_trust;
            let store = TrustStore::new(&agent_dir);
            match resolve_trusted(&base, &store, None, default).unwrap_or(false) {
                true => "yapi remove -l",
                false => "yapi remove -l --approve",
            }
        }
    };
    let mut added = 0;
    for package in &packages {
        let (spec, filtered) = match package {
            PackageSource::Source(spec) => (spec.clone(), None),
            PackageSource::Filtered(filtered) => (filtered.source.clone(), Some(filtered)),
        };
        let mut entry = Entry {
            capabilities: Some(vec!["all".into()]),
            ..Entry::default()
        };
        // `yapi remove` reads a relative path from the current folder.
        let mut removed = spec.clone();
        match source::parse(&spec) {
            Source::Npm { name, version, .. } => {
                entry.npm = Some(name);
                entry.version = version;
            }
            Source::Git {
                repo, reference, ..
            } => {
                entry.git = Some(repo);
                entry.rev = reference;
            }
            Source::Local { path } => {
                let absolute = source::local_path(&path, &folder);
                removed = absolute.to_string_lossy().into_owned();
                entry.path = Some(match target.scope {
                    Scope::Project => relative(&target.base(), &absolute),
                    Scope::Global => absolute.to_string_lossy().into_owned(),
                });
            }
        }
        if let Some(filtered) = filtered {
            // Importing without these filters would load more than settings.json does.
            if filtered.extensions.is_some() || filtered.autoload == Some(false) {
                say(&format!(
                    "warning: skipped {spec}: chest has no `extensions` filter or `autoload: false`, so it keeps loading from {file}"
                ));
                continue;
            }
            entry.skills = filtered.skills.clone();
            entry.prompts = filtered.prompts.clone();
            entry.themes = filtered.themes.clone();
        }
        let name = name_of(&entry);
        if !valid_name(&name) {
            say(&format!(
                "warning: skipped {spec}: `{name}` is not a valid entry name"
            ));
            continue;
        }
        if add(document, (Section::Extensions, &name), &entry, &spec, &base)? {
            added += 1;
            out(&format!(
                "extensions.{name}  from {spec}. Once chest installs it, remove it from {file} with `{remove} {removed}`."
            ));
        }
    }
    Ok(added)
}

/// The package `entry` installs, as settings packages are matched, with
/// the folder it takes from that package.
fn identity(entry: &Entry, base: &Path) -> Option<(String, Option<String>)> {
    let (source, folder) = match (&entry.npm, &entry.git, &entry.path) {
        (Some(npm), ..) => (source::parse(&format!("npm:{npm}")), entry.path.clone()),
        (None, Some(git), _) => (source::parse(&format!("git:{git}")), entry.path.clone()),
        (None, None, Some(path)) => (Source::Local { path: path.clone() }, None),
        (None, None, None) => return None,
    };
    Some((source.identity(base), folder))
}

/// The last part of an entry's source, lowercased, as its name.
fn name_of(entry: &Entry) -> String {
    let from = entry
        .npm
        .as_deref()
        .or(entry.git.as_deref())
        .or(entry.path.as_deref());
    let last = from
        .unwrap_or_default()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    last.strip_suffix(".git")
        .unwrap_or(last)
        .to_ascii_lowercase()
}

/// The skills of a `skills-lock.json` (skills.sh's `LocalSkillLockFile`)
/// as skill entries, without their hashes, which chest computes its own way.
/// A skill of a source type chest doesn't know is left out, with a warning.
fn skills_lock(path: &Path, base: &Path, document: &mut ManifestDocument) -> anyhow::Result<usize> {
    let lock = json(path)?;
    if lock.get("version").and_then(Value::as_u64) != Some(1) {
        bail!(
            "{}: only version 1 of skills-lock.json is known",
            path.display()
        );
    }
    let Some(skills) = lock.get("skills").and_then(Value::as_object) else {
        bail!("{}: no `skills` map", path.display());
    };
    let mut added = 0;
    for (name, skill) in skills {
        let text = |key: &str| skill.get(key).and_then(Value::as_str).map(str::to_owned);
        let source = text("source").unwrap_or_default();
        // `skillPath` names the SKILL.md, and chest names its folder.
        let folder = text("skillPath").map(|path| {
            let parent = Path::new(&path)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            parent.to_string_lossy().into_owned()
        });
        let mut entry = Entry::default();
        match text("sourceType").as_deref() {
            Some("node_modules") => {
                entry.npm = Some(source.clone());
                entry.version = text("version");
                entry.path = folder;
            }
            Some("local") => entry.path = Some(source.clone()),
            Some(kind @ ("github" | "gitlab" | "git")) => {
                let has_host = source.contains("://")
                    || source
                        .split('/')
                        .next()
                        .is_some_and(|first| first.contains('.'));
                let host = if kind == "gitlab" {
                    "gitlab.com"
                } else {
                    "github.com"
                };
                let repo = text("sourceUrl").unwrap_or_else(|| match has_host {
                    true => source.clone(),
                    false => format!("{host}/{source}"),
                });
                entry.git = Some(repo.strip_prefix("git:").unwrap_or(&repo).to_owned());
                entry.rev = text("ref");
                entry.path = folder.filter(|folder| !folder.is_empty());
            }
            kind => {
                let reason = match kind {
                    Some(kind) => format!("chest can't install its sourceType {kind:?}"),
                    None => "it has no sourceType".to_owned(),
                };
                say(&format!("warning: skipped {name}: {reason}"));
                continue;
            }
        }
        if !valid_name(name) {
            say(&format!("warning: skipped {name}: not a valid entry name"));
            continue;
        }
        if add(document, (Section::Skills, name), &entry, name, base)? {
            added += 1;
            out(&format!("skills.{name}  from {source}"));
        }
    }
    Ok(added)
}
