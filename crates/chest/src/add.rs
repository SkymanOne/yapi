//! `chest add`: looks at a package, asks what to grant it, writes its entry
//! and installs it.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};
use clap::Args;
use yapi_core::chest::Mode;
use yapi_core::chest::manifest::{
    Entry, Manifest, ManifestDocument, PackageMeta, Section, package_meta, valid_name,
};
use yapi_core::extensions::{EventPolicy, Grants};
use yapi_core::packages::npm::Npm;
use yapi_core::packages::source::{self, Source};
use yapi_core::settings::Scope;
use yapi_core::tools::path::relative;

use crate::check::{Inspected, inspect, unsupported_import};
use crate::prompt::Prompt;
use crate::{Target, open, read, requests, say, source, sync, write};

/// What `all` means, said before it is granted.
pub const ALL_WARNING: &str = "`all` is full access: the extension can run programs as you, outside the sandbox, with yapi's environment and API keys.";

/// What sharing `pi.events` costs, said before the event choice.
const EVENTS_TEXT: &str = "By default this extension shares `pi.events` with every other extension. Extensions with more access, including packages from `yapi install`, can act on its events, and it sees theirs. Choose 'declared channels' or 'isolated' to limit that.";

/// `chest add` options.
#[derive(Args)]
pub struct AddArgs {
    /// `npm:<name>[@<range>]`, `git:<repo>[@<rev>]`, a git URL, or a path.
    source: String,
    /// Add it as a skill.
    #[arg(long, group = "kind")]
    skill: bool,
    /// Add it as a subagent.
    #[arg(long, group = "kind")]
    agent: bool,
    /// Add it as an extension.
    #[arg(long, group = "kind")]
    extension: bool,
    /// The entry's name, else one derived from the source.
    #[arg(long)]
    name: Option<String>,
    /// The item's folder or file inside the package.
    #[arg(long)]
    path: Option<String>,
    /// The git branch, tag or commit.
    #[arg(long, conflicts_with = "version")]
    rev: Option<String>,
    /// The npm version range or dist-tag.
    #[arg(long)]
    version: Option<String>,
    /// Capabilities to grant, comma-separated, with no question asked.
    #[arg(long, value_delimiter = ',')]
    grant: Vec<String>,
    /// Ask nothing: grant only `--grant`, and share events.
    #[arg(long)]
    yes: bool,
}

/// The entry `args` describes, before it has a section or name.
fn entry(args: &AddArgs, target: &Target) -> anyhow::Result<Entry> {
    let mut entry = Entry {
        path: args.path.clone(),
        ..Entry::default()
    };
    match source::parse(&args.source) {
        Source::Npm { name, version, .. } => {
            entry.npm = Some(name);
            entry.version = args.version.clone().or(version);
        }
        Source::Git {
            repo, reference, ..
        } => {
            entry.git = Some(repo);
            entry.rev = args.rev.clone().or(reference);
        }
        Source::Local { path } => {
            if args.path.is_some() {
                bail!("--path is for git and npm sources. Give the path itself as the source.");
            }
            let absolute = std::path::absolute(&path)?;
            entry.path = Some(match target.scope {
                Scope::Project => relative(&target.base(), &absolute),
                Scope::Global => absolute.to_string_lossy().into_owned(),
            });
        }
    }
    Ok(entry)
}

/// A name for the entry from its source: the last part of its path, npm
/// name or repository, lowercased, without `.md`.
fn default_name(entry: &Entry) -> String {
    let from = entry
        .path
        .as_deref()
        .or(entry.npm.as_deref())
        .or(entry.git.as_deref())
        .unwrap_or_default();
    let last = from
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let last = last.strip_suffix(".git").unwrap_or(last);
    let last = last.strip_suffix(".md").unwrap_or(last);
    last.to_ascii_lowercase()
}

/// The item `entry` picks from the package fetched at `root`.
fn item(root: &Path, entry: &Entry) -> PathBuf {
    match (&entry.git, &entry.npm) {
        (None, None) => root.to_path_buf(),
        _ => root.join(entry.path.as_deref().unwrap_or_default()),
    }
}

/// The section a package at `root` belongs in, judged from the item the
/// entry picks: a `.md` file is a subagent, a folder with a `SKILL.md` a
/// skill, and a package with extensions an extension.
fn infer(root: &Path, entry: &Entry) -> anyhow::Result<Section> {
    let item = item(root, entry);
    let extension =
        |name: &str| name.ends_with(".wasm") || name.ends_with(".ts") || name.ends_with(".js");
    let name = item.file_name().unwrap_or_default().to_string_lossy();
    if item.is_file() {
        if name.ends_with(".md") {
            return Ok(Section::Agents);
        }
        if extension(&name) {
            return Ok(Section::Extensions);
        }
    }
    if item.join("SKILL.md").is_file() {
        return Ok(Section::Skills);
    }
    let meta = package_meta(&item)?;
    let lists = |kind: &str| {
        meta.resources.contains_key(kind) || !meta.manifest && item.join(kind).is_dir()
    };
    let entry_file = ["index.ts", "index.js"]
        .iter()
        .any(|file| item.join(file).is_file());
    if lists("extensions") || entry_file {
        return Ok(Section::Extensions);
    }
    if lists("skills") {
        return Ok(Section::Skills);
    }
    if lists("agents") {
        return Ok(Section::Agents);
    }
    bail!(
        "Cannot tell what {} provides: pass --skill, --agent or --extension",
        item.display()
    )
}

/// Prints what the package says about itself.
fn show(meta: &PackageMeta) {
    let mut title = meta.name.clone().unwrap_or_else(|| "This package".into());
    if let Some(version) = &meta.version {
        title.push_str(&format!(" {version}"));
    }
    say(&title);
    if let Some(description) = &meta.description {
        say(&format!("  {description}"));
    }
    if !meta.authors.is_empty() {
        say(&format!("  by {}", meta.authors.join(", ")));
    }
    match meta.capabilities.is_empty() {
        true => say("  requests no capabilities"),
        false => say(&format!("  requests: {}", meta.capabilities.join(", "))),
    }
    if let Some(events) = &meta.events {
        say(&format!(
            "  event channels: sends {}, hears {}",
            channels(&events.send),
            channels(&events.receive)
        ));
    }
}

/// Prints what each native extension uses, under what the package
/// requests.
fn show_used(inspected: &Inspected) {
    for (file, found) in &inspected.components {
        let found = match found {
            Ok(found) => found,
            Err(err) => {
                say(&format!("  {file}: {err}"));
                continue;
            }
        };
        let used: Vec<String> = found
            .used
            .iter()
            .map(|used| format!("{} ({})", used.capability, used.source))
            .collect();
        match (used.is_empty(), found.metadata) {
            (true, true) => say(&format!("  {file} uses no capabilities")),
            (false, _) => say(&format!("  {file} uses: {}", used.join(", "))),
            (true, false) => say(&format!(
                "  {file} uses no capabilities its WASI imports show"
            )),
        }
        if !found.metadata {
            say(
                "    it was built without capability metadata, so only its WASI imports show what it uses",
            );
        }
        for import in &found.unsupported {
            say(&format!("  warning: {}", unsupported_import(file, import)));
        }
    }
}

fn channels(patterns: &[String]) -> String {
    if patterns.is_empty() {
        "none".into()
    } else {
        patterns.join(", ")
    }
}

/// What each capability allows, for its question.
fn meaning(capability: &str) -> &'static str {
    match capability {
        "fs" => "read and write files",
        "net" => "fetch, call models and read provider credentials",
        "env" => "read environment variables",
        "session" => "start turns and change the session",
        "all" => "everything, including running programs",
        _ => "an unknown capability",
    }
}

/// The capabilities to grant: `--grant`, where `none` is the empty start,
/// then each requested or `used` one the person accepts. Requests that are
/// not capabilities, such as `process`, are skipped with a warning, so they
/// are never asked about or written.
fn grants(
    prompt: &Prompt,
    args: &AddArgs,
    meta: &PackageMeta,
    used: &[&str],
) -> anyhow::Result<Vec<String>> {
    Grants::parse(&args.grant)
        .map_err(anyhow::Error::msg)
        .context("--grant")?;
    let mut granted: Vec<String> = args
        .grant
        .iter()
        .filter(|value| *value != "none")
        .cloned()
        .collect();
    let (valid, invalid): (Vec<&String>, Vec<&String>) = meta
        .capabilities
        .iter()
        .partition(|capability| Grants::parse(std::slice::from_ref(*capability)).is_ok());
    if !invalid.is_empty() {
        let names: Vec<&str> = invalid.iter().map(|value| value.as_str()).collect();
        say(&format!(
            "warning: the package requests values that are not capabilities, which were skipped: {}",
            names.join(", ")
        ));
    }
    if !prompt.interactive() {
        return Ok(granted);
    }
    let mut offered: Vec<String> = valid.into_iter().cloned().collect();
    for capability in used {
        if !offered.iter().any(|offer| offer == capability) {
            offered.push((*capability).to_owned());
        }
    }
    for capability in &offered {
        // `all` covers the rest.
        if granted.iter().any(|grant| grant == "all") {
            break;
        }
        if granted.contains(capability) || capability == "none" {
            continue;
        }
        if capability == "all" {
            say(ALL_WARNING);
        }
        if prompt.confirm(&format!("Grant `{capability}` ({})?", meaning(capability))) {
            granted.push(capability.clone());
        }
    }
    Ok(granted)
}

/// The event policy the person picks: shared, the declared channels, or
/// isolated. Shared, unasked.
fn events(prompt: &Prompt, meta: &PackageMeta) -> Option<EventPolicy> {
    if !prompt.interactive() {
        return None;
    }
    say(EVENTS_TEXT);
    let declared = meta
        .events
        .clone()
        .filter(|events| *events != EventPolicy::shared());
    let mut options = vec!["shared, the default".to_owned()];
    if let Some(declared) = &declared {
        options.push(format!(
            "declared channels: sends {}, hears {}",
            channels(&declared.send),
            channels(&declared.receive)
        ));
    }
    options.push("isolated".to_owned());
    let choice = prompt.choose("How should it share events?", &options, 0);
    match (choice, &declared) {
        (0, _) => None,
        (1, Some(declared)) => Some(declared.clone()),
        _ => Some(EventPolicy::isolated()),
    }
}

/// Warns about entries the package's own manifest lists, which chest does
/// not install for it.
fn nested(root: &Path, name: &str) {
    let Ok(text) = std::fs::read_to_string(root.join("chest.toml")) else {
        return;
    };
    let Ok(manifest) = Manifest::parse(&text, Scope::Project) else {
        return;
    };
    let listed: Vec<String> = manifest
        .entries()
        .map(|(section, name, _)| format!("{}.{name}", section.key()))
        .collect();
    if !listed.is_empty() {
        say(&format!(
            "warning: {name} lists entries of its own, which chest does not install: {}. Add the ones you want with `chest add`.",
            listed.join(", ")
        ));
    }
}

/// Runs `chest add`.
pub async fn run(target: &Target, args: AddArgs, npm: &mut Npm) -> anyhow::Result<()> {
    let before = read(target)?;
    let mut document = ManifestDocument::parse(&before, target.scope)?;
    let mut entry = entry(&args, target)?;
    let name = args.name.clone().unwrap_or_else(|| default_name(&entry));
    if !valid_name(&name) {
        bail!(
            "`{name}` is not a valid entry name: pass --name with lowercase letters, digits, `.`, `_` or `-`"
        );
    }
    let flag = [
        (args.skill, Section::Skills),
        (args.agent, Section::Agents),
        (args.extension, Section::Extensions),
    ]
    .into_iter()
    .find_map(|(set, section)| set.then_some(section));
    let existing = [Section::Skills, Section::Agents, Section::Extensions]
        .into_iter()
        .find_map(|section| {
            Some((
                section,
                document.manifest().section(section).get(&name)?.clone(),
            ))
        });
    let section = if let Some((section, old)) = existing {
        let same = old.git == entry.git && old.npm == entry.npm && old.path == entry.path;
        if !same || flag.is_some_and(|flag| flag != section) {
            bail!(
                "{}.{name} already comes from {}: choose another --name",
                section.key(),
                source(&old)
            );
        }
        // The same source again moves its version or rev, keeping its grants.
        let updated = Entry {
            rev: entry.rev.clone().or(old.rev.clone()),
            version: entry.version.clone().or(old.version.clone()),
            ..old
        };
        document.set_entry(section, &name, &updated)?;
        write(target, &document.to_string())?;
        if let Err(err) = sync(target, Mode::Update(vec![name.clone()]), npm).await {
            write(target, &before)?;
            return Err(err);
        }
        return Ok(());
    } else {
        let project = open(target)?;
        let fetched = project.fetch_package(&entry, npm).await?;
        let section = match flag {
            Some(section) => section,
            None => infer(&fetched.root, &entry)?,
        };
        let meta = package_meta(&fetched.root)?;
        show(&meta);
        nested(&fetched.root, &name);
        if section == Section::Extensions {
            let inspected = inspect(&item(&fetched.root, &entry));
            show_used(&inspected);
            let prompt = Prompt::new(args.yes);
            let granted = grants(&prompt, &args, &meta, &inspected.used())?;
            entry.capabilities = (!granted.is_empty()).then_some(granted);
            entry.events = events(&prompt, &meta);
            for warning in requests(&project.manifest, &entry, &meta) {
                say(&format!("note: {name} {warning}"));
            }
        } else if !args.grant.is_empty() {
            bail!("--grant is for extensions only");
        }
        section
    };
    document.set_entry(section, &name, &entry)?;
    write(target, &document.to_string())?;
    if let Err(err) = sync(target, Mode::Resolve, npm).await {
        // The manifest goes back to what it was, so a failed add leaves no entry.
        write(target, &before)?;
        return Err(err);
    }
    Ok(())
}
