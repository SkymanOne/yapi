//! `chest add`: looks at a package, asks which of its items to add and what
//! to grant it, writes its entries and installs them.

use std::path::{Path, PathBuf};

use anyhow::bail;
use clap::Args;
use yapi_core::chest::Mode;
use yapi_core::chest::manifest::{
    Entry, Manifest, ManifestDocument, PackageMeta, Section, inside_package, package_meta,
    ssh_hint, valid_name,
};
use yapi_core::chest::startup::{package_identity, same_package};
use yapi_core::extensions::{CAPABILITIES, EventPolicy, Grants};
use yapi_core::packages::npm::{Npm, check_name};
use yapi_core::packages::resolve::is_extension_file;
use yapi_core::packages::source::{self, Source, local_path};
use yapi_core::settings::Scope;
use yapi_core::tools::path::relative;

use crate::check::{
    Inspected, capped, extension_file, inspect, unknown_capability, unsupported_import,
};
use crate::prompt::{Choice, Prompt};
use crate::{Target, Usage, frontmatter, open, read, requests, source, sync, write};

/// What `all` means, said before it is granted.
pub const ALL_WARNING: &str = "`all` is full access: the extension can run programs as you, outside the sandbox, with yapi's environment and API keys.";

/// What `all` means, said at a terminal before it is granted.
const ALL_DRAWN: &str = "`all` runs the extension unrestricted, outside the sandbox. It can read yapi's environment and API keys.";

/// The warning before `all` is granted, as `prompt` shows it.
fn all_warning(prompt: &Prompt) -> &'static str {
    match prompt.styled() {
        true => ALL_DRAWN,
        false => ALL_WARNING,
    }
}

/// What sharing `pi.events` costs, said before the event choice.
const EVENTS_TEXT: &str = "By default, this extension shares `pi.events` with every other extension, including ones with more access, such as packages from `yapi install`.";

/// `chest add` options.
#[derive(Args)]
pub struct AddArgs {
    /// `npm:<name>[@<range>]`, `git:<repo>[@<rev>]`, a git URL, or a path.
    source: String,
    /// Add it as a skill. With a name, add the skill of that name from the
    /// package's `skills` folder, once per name given.
    #[arg(long, group = "kind", num_args = 0..=1, value_name = "NAME", value_parser = skill_name)]
    skill: Option<Vec<String>>,
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
    /// Isolate the extension's `pi.events`, with no question asked: it sends
    /// and hears none. Shared is the default.
    #[arg(long)]
    isolated: bool,
    /// Ask nothing: add the whole package, grant only `--grant`, and share
    /// events unless `--isolated`.
    #[arg(long)]
    pub yes: bool,
}

/// A `--skill` value. A skill name has no `/` or `:`, so a source written
/// after a bare `--skill` fails instead of becoming a name.
fn skill_name(value: &str) -> Result<String, String> {
    match value.contains(['/', ':']) {
        true => Err("a skill name has no `/` or `:`. To mark the source as a skill, put --skill after it: `chest add <source> --skill`".into()),
        false => Ok(value.to_owned()),
    }
}

/// `entry` narrowed to the item at `inside` its package: that `path` for a
/// git or npm source, and the item's own path for a path source.
fn inner(entry: &Entry, target: &Target, inside: &str) -> Entry {
    let path = match (&entry.git, &entry.npm) {
        (None, None) => {
            let package = local_path(entry.path.as_deref().unwrap_or_default(), &target.base());
            let item = package.join(inside);
            match target.scope {
                Scope::Project => relative(&target.base(), &item),
                Scope::Global => item.to_string_lossy().into_owned(),
            }
        }
        _ => inside.to_owned(),
    };
    Entry {
        path: Some(path),
        ..entry.clone()
    }
}

/// The entries `--skill` names add, one per name, each for the folder of
/// that name in the `skills` folder of the package at `root`, or for the
/// one whose SKILL.md has that `name`.
fn named(
    args: &AddArgs,
    target: &Target,
    (entry, root): (&Entry, &Path),
    names: &[String],
) -> anyhow::Result<Vec<(String, Entry)>> {
    let local = entry.git.is_none() && entry.npm.is_none();
    let Ok(listing) = std::fs::read_dir(root.join("skills")) else {
        let pick = match local {
            true => "give the skill's folder as the source",
            false => "pick the skill with --path",
        };
        return Err(Usage(format!("{} has no `skills` folder: {pick}", args.source)).into());
    };
    let mut folders: Vec<(String, Option<String>)> = listing
        .flatten()
        .filter(|item| item.path().join("SKILL.md").is_file())
        .map(|item| {
            let folder = item.file_name().to_string_lossy().into_owned();
            (folder, frontmatter(Section::Skills, &item.path(), "name"))
        })
        .collect();
    folders.sort();
    let mut entries = Vec::new();
    for name in names {
        let by_folder = folders.iter().find(|(folder, _)| folder == name);
        let by_name = || folders.iter().find(|(_, own)| own.as_deref() == Some(name));
        let Some((folder, _)) = by_folder.or_else(by_name) else {
            let known: Vec<&str> = folders.iter().map(|(folder, _)| folder.as_str()).collect();
            let holds = match known.is_empty() {
                true => "no skills".to_owned(),
                false => known.join(", "),
            };
            return Err(Usage(format!(
                "{} has no skill {name}: its `skills` folder holds {holds}",
                args.source
            ))
            .into());
        };
        let entry = inner(entry, target, &format!("skills/{folder}"));
        entries.push((args.name.clone().unwrap_or_else(|| name.clone()), entry));
    }
    Ok(entries)
}

/// The entries the person picks from the `items` of the package at `root`,
/// one per item with its `path`, or `None` when they keep them all, so that
/// one entry for the whole package installs them, new ones included.
fn pick(
    prompt: &Prompt,
    (section, kind): (Section, &str),
    items: &[(String, PathBuf)],
    (entry, root): (&Entry, &Path),
    target: &Target,
) -> anyhow::Result<Option<Vec<(String, Entry)>>> {
    let options: Vec<Choice<usize>> = items
        .iter()
        .enumerate()
        .map(|(index, (name, path))| {
            let description = frontmatter(section, path, "description");
            (index, name.clone(), description.unwrap_or_default())
        })
        .collect();
    let question = format!(
        "Which {kind} should chest add?\nKeep all for one entry that gets new {kind} on update.\nUntick some for one entry each."
    );
    let all = (0..items.len()).collect();
    let chosen = prompt.multiselect(&question, &options, all, true)?;
    if chosen.len() == items.len() {
        return Ok(None);
    }
    let picks = chosen.into_iter().map(|index| {
        let (name, path) = &items[index];
        let name = name.strip_suffix(".md").unwrap_or(name).to_owned();
        (name, inner(entry, target, &relative(root, path)))
    });
    Ok(Some(picks.collect()))
}

/// The entry `args` describes, before it has a section or name.
fn entry(args: &AddArgs, target: &Target) -> anyhow::Result<Entry> {
    let mut entry = Entry {
        path: args.path.clone(),
        ..Entry::default()
    };
    let conflict = |message: String| Err(Usage(message).into());
    match parse(&args.source)? {
        Source::Npm { name, version, .. } => {
            // pi reads `npm:x@` as the name `x@`.
            if name
                .strip_suffix('@')
                .is_some_and(|name| check_name(name).is_ok())
            {
                return conflict(format!(
                    "`{}` has an empty version: `npm:<name>[@<range>]`",
                    args.source
                ));
            }
            if check_name(&name).is_err() {
                return conflict(format!(
                    "`{}` needs a package name: `npm:<name>[@<range>]`",
                    args.source
                ));
            }
            if args.rev.is_some() {
                return conflict("--rev conflicts with an npm source: use --version".into());
            }
            if let (Some(version), Some(_)) = (&version, &args.version) {
                return conflict(format!(
                    "--version conflicts with the version `{version}` in the source: give one of them"
                ));
            }
            entry.npm = Some(name);
            entry.version = args.version.clone().or(version);
        }
        Source::Git {
            repo, reference, ..
        } => {
            if args.version.is_some() {
                return conflict("--version conflicts with a git source: use --rev".into());
            }
            if let (Some(reference), Some(_)) = (&reference, &args.rev) {
                return conflict(format!(
                    "--rev conflicts with the revision `{reference}` in the source: give one of them"
                ));
            }
            entry.git = Some(repo);
            entry.rev = args.rev.clone().or(reference);
        }
        Source::Local { path } => {
            if args.path.is_some() {
                return conflict(
                    "--path is for git and npm sources. Give the path itself as the source.".into(),
                );
            }
            if args.rev.is_some() || args.version.is_some() {
                let flag = if args.rev.is_some() {
                    "--rev"
                } else {
                    "--version"
                };
                return conflict(format!("{flag} conflicts with a path source"));
            }
            let absolute = std::path::absolute(&path)?;
            if absolute.symlink_metadata().is_err() {
                // `foo:bar` names a source type, unless such a path exists.
                let kind = path.split_once(':').map(|(kind, _)| kind);
                let kind = kind.filter(|kind| {
                    kind.len() > 1
                        && kind.starts_with(|c: char| c.is_ascii_alphabetic())
                        && kind
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
                });
                if let Some(kind) = kind {
                    bail!(
                        "`{kind}:` is not a source type chest knows: use `npm:`, `git:`, a git URL or a path"
                    );
                }
                bail!("{path} does not exist");
            }
            entry.path = Some(match target.scope {
                Scope::Project => relative(&target.base(), &absolute),
                Scope::Global => absolute.to_string_lossy().into_owned(),
            });
        }
    }
    // Checked before anything reads the package, as the manifest checks it.
    if let Some(path) = args.path.as_deref().filter(|path| !inside_package(path)) {
        return conflict(format!(
            "--path: \"{path}\" must be a folder inside the package"
        ));
    }
    Ok(entry)
}

/// `text` as a source: a git URL, or scp-like `user@host:path`, as if
/// `git:` came first. yapi's `install` keeps pi's reading, where only
/// `https://`, `http://` and `ssh://` URLs are git sources.
fn parse(text: &str) -> anyhow::Result<Source> {
    let url = ["git://", "ssh://", "https://", "http://"]
        .iter()
        .any(|scheme| text.starts_with(scheme));
    let scp = text
        .split_once(':')
        .is_some_and(|(left, _)| left.contains('@') && !left.contains('/'));
    let git = url || scp || text.starts_with("git:");
    let parsed = match url || scp {
        true => source::parse(&format!("git:{text}")),
        false => source::parse(text),
    };
    if git && matches!(parsed, Source::Local { .. }) {
        let rest = text.strip_prefix("git:").unwrap_or(text);
        if let Some(hint) = ssh_hint(rest) {
            bail!("{text} is not a git repository chest can read: {hint}");
        }
        bail!("{text} is not a git repository chest can read");
    }
    Ok(parsed)
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
/// entry picks: a `.md` file in a `skills` folder is a single-file skill,
/// another `.md` file a subagent, a folder with a `SKILL.md` a skill, and a
/// package with extensions an extension. Errors call the item `shown`.
fn infer(root: &Path, entry: &Entry, shown: &str) -> anyhow::Result<Section> {
    let item = item(root, entry);
    let name = item.file_name().unwrap_or_default().to_string_lossy();
    if item.is_file() {
        let folder = item.parent().and_then(Path::file_name);
        if name.ends_with(".md") && folder.is_some_and(|folder| folder == "skills") {
            return Ok(Section::Skills);
        }
        if name.ends_with(".md") {
            return Ok(Section::Agents);
        }
        if is_extension_file(&name) {
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
    bail!("Cannot tell what {shown} provides: pass --skill, --agent or --extension")
}

/// What the package says about itself.
fn show(meta: &PackageMeta) -> Vec<String> {
    let mut title = meta.name.clone().unwrap_or_else(|| "This package".into());
    if let Some(version) = &meta.version {
        title.push_str(&format!(" {version}"));
    }
    let mut lines = vec![title];
    if let Some(description) = &meta.description {
        lines.push(format!("  {description}"));
    }
    if !meta.authors.is_empty() {
        lines.push(format!("  by {}", meta.authors.join(", ")));
    }
    lines.push(match meta.capabilities.is_empty() {
        true => "  requests no capabilities".into(),
        false => format!("  requests: {}", meta.capabilities.join(", ")),
    });
    if let Some(events) = &meta.events {
        lines.push(format!(
            "  event channels: sends {}, hears {}",
            channels(&events.send),
            channels(&events.receive)
        ));
    }
    lines
}

/// What each native extension uses, under what the package requests.
fn show_used(inspected: &Inspected) -> Vec<String> {
    let mut lines = Vec::new();
    for (file, found) in &inspected.components {
        let found = match found {
            Ok(found) => found,
            Err(err) => {
                lines.push(format!("  {file}: {err}"));
                continue;
            }
        };
        let used: Vec<String> = found
            .used
            .iter()
            .map(|used| format!("{} ({})", used.capability, used.source))
            .collect();
        lines.push(match (used.is_empty(), found.metadata) {
            (true, true) => format!("  {file} uses no capabilities"),
            (false, _) => format!("  {file} uses: {}", used.join(", ")),
            (true, false) => format!("  {file} uses no capabilities its WASI imports show"),
        });
        if !found.metadata {
            lines.push(
                "    it was built without capability metadata, so only its WASI imports show what it uses".into(),
            );
        }
        let rest = format!("unsupported imports in {file}");
        lines.extend(capped("  warning: ", &found.unsupported, &rest, |import| {
            unsupported_import(file, import)
        }));
        let rest = format!("unknown capability lines in {file}");
        lines.extend(capped("  warning: ", &found.unknown, &rest, |line| {
            unknown_capability(file, line)
        }));
    }
    lines
}

fn channels(patterns: &[String]) -> String {
    if patterns.is_empty() {
        "none".into()
    } else {
        patterns.join(", ")
    }
}

/// What each capability allows, for its question. Each of
/// [`CAPABILITIES`](yapi_core::extensions::CAPABILITIES) has one.
fn meaning(capability: &str) -> &'static str {
    match capability {
        "fs" => "read and write files",
        "net" => "make web requests, call models and read provider credentials",
        "env" => "read environment variables",
        "session" => "start turns and change the session",
        "all" => "runs unrestricted, outside the sandbox",
        _ => "an unknown capability",
    }
}

/// The capabilities to grant: `--grant`, which [`run`] checked, where
/// `none` is the empty start. Asked, without `--grant`, the person grants
/// those the package requests or `used`, or ticks them, starting from
/// those, except `all`, which they tick themselves. Requests that are not
/// capabilities, such as `process`, are skipped with a warning, so they are
/// never offered or written.
fn grants(
    prompt: &Prompt,
    args: &AddArgs,
    meta: &PackageMeta,
    used: &[&str],
) -> anyhow::Result<Vec<String>> {
    let granted: Vec<String> = args
        .grant
        .iter()
        .filter(|value| *value != "none")
        .cloned()
        .collect();
    let all = |grants: &[String]| grants.iter().any(|grant| grant == "all");
    if all(&granted) {
        prompt.warn(all_warning(prompt));
    }
    let (valid, invalid): (Vec<&String>, Vec<&String>) = meta
        .capabilities
        .iter()
        .partition(|capability| Grants::parse(std::slice::from_ref(*capability)).is_ok());
    if !invalid.is_empty() {
        let names: Vec<&str> = invalid.iter().map(|value| value.as_str()).collect();
        prompt.warn(&format!(
            "warning: the package requests values that are not capabilities, which were skipped: {}",
            names.join(", ")
        ));
    }
    // `--grant` answers the question.
    if !prompt.interactive() || !args.grant.is_empty() {
        return Ok(granted);
    }
    let asked = valid
        .into_iter()
        .map(String::as_str)
        .chain(used.iter().copied());
    let requested: Vec<&str> = CAPABILITIES
        .into_iter()
        .filter(|capability| asked.clone().any(|asked| asked == *capability))
        .collect();
    // `all` runs unrestricted, so a request never makes it the default.
    let risky = requested.contains(&"all");
    let question = "Which capabilities should it get?";
    let choose = requested.is_empty() || {
        let mut grant = format!("Grant all requested capabilities: {}", requested.join(", "));
        if risky {
            grant.push_str(" (runs unrestricted)");
        }
        let options = [
            (false, grant, String::new()),
            (true, "Choose capabilities".to_owned(), String::new()),
        ];
        prompt.select(question, &options, risky)?
    };
    let chosen: Vec<String> = match choose {
        false => requested
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect(),
        true => {
            let ticked: Vec<&str> = requested
                .iter()
                .copied()
                .filter(|capability| *capability != "all")
                .collect();
            let options: Vec<Choice<&str>> = CAPABILITIES
                .into_iter()
                .map(|capability| (capability, capability.into(), meaning(capability).into()))
                .collect();
            let question = match requested.is_empty() {
                true => question,
                false => "Choose capabilities",
            };
            let chosen = prompt.multiselect(question, &options, ticked, false)?;
            chosen.into_iter().map(str::to_owned).collect()
        }
    };
    if !all(&chosen) {
        return Ok(chosen);
    }
    // `all` covers the rest.
    prompt.warn(all_warning(prompt));
    Ok(vec!["all".into()])
}

/// The event policy the person picks: shared, the declared channels, or
/// isolated. Shared when chest asks nothing.
fn events(prompt: &Prompt, meta: &PackageMeta) -> anyhow::Result<Option<EventPolicy>> {
    if !prompt.interactive() {
        return Ok(None);
    }
    let declared = meta
        .events
        .clone()
        .filter(|events| *events != EventPolicy::shared());
    prompt.info(&[events_text(declared.is_some())]);
    let mut options = vec![(0, "shared".to_owned(), "the default".to_owned())];
    if let Some(declared) = &declared {
        let channels = format!(
            "sends {}, hears {}",
            channels(&declared.send),
            channels(&declared.receive)
        );
        options.push((1, "declared channels".to_owned(), channels));
    }
    options.push((
        2,
        "isolated".to_owned(),
        "shares events only with extensions that have exactly its grants".to_owned(),
    ));
    let choice = prompt.select("How should it share events?", &options, 0)?;
    Ok(match (choice, &declared) {
        (0, _) => None,
        (1, Some(declared)) => Some(declared.clone()),
        _ => Some(EventPolicy::isolated()),
    })
}

/// What chest says before it asks how an extension shares events, naming
/// the choices the menu offers to limit that: the channels the package
/// declares, when it declares any, and isolated.
fn events_text(declared: bool) -> String {
    let limits = match declared {
        true => "'declared channels' or 'isolated'",
        false => "'isolated'",
    };
    format!("{EVENTS_TEXT} Choose {limits} to limit that.")
}

/// Warns about entries the package's own manifest lists, which chest does
/// not install for it, with the command that adds them to `target`.
fn nested(target: &Target, root: &Path, name: &str, prompt: &Prompt) {
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
        prompt.warn(&format!(
            "warning: {name} lists entries of its own, which chest does not install: {}. Add the ones you want with `{} add`.",
            listed.join(", "),
            target.command()
        ));
    }
}

/// The entry named `name` in any section of `document`.
fn existing(document: &ManifestDocument, name: &str) -> Option<(Section, Entry)> {
    [Section::Skills, Section::Agents, Section::Extensions]
        .into_iter()
        .find_map(|section| {
            Some((
                section,
                document.manifest().section(section).get(name)?.clone(),
            ))
        })
}

/// Runs `chest add`.
pub async fn run(
    target: &Target,
    args: AddArgs,
    npm: &mut Npm,
    prompt: &Prompt,
) -> anyhow::Result<()> {
    let requested = Grants::parse(&args.grant).map_err(|err| Usage(format!("--grant: {err}")))?;
    let before = read(target)?;
    let mut document = ManifestDocument::parse(&before, target.scope)?;
    let package = entry(&args, target)?;
    let mut names: Vec<String> = Vec::new();
    for name in args.skill.iter().flatten() {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    if !names.is_empty() && args.path.is_some() {
        let message = "--skill <name> and --path both pick the item: give one of them";
        return Err(Usage(message.into()).into());
    }
    if names.len() > 1 && args.name.is_some() {
        let message = "--name names one entry: give it with one --skill <name>";
        return Err(Usage(message.into()).into());
    }
    // One named skill is the entry `--path skills/<name>` writes.
    let mut entry = match names.as_slice() {
        [skill] => inner(&package, target, &format!("skills/{skill}")),
        _ => package.clone(),
    };
    let name = match (&args.name, names.as_slice()) {
        (Some(name), _) => name.clone(),
        (None, [skill]) => skill.clone(),
        _ => default_name(&entry),
    };
    let invalid = |name: &str| {
        anyhow::anyhow!(
            "`{name}` is not a valid entry name: pass --name with lowercase letters, digits, `.`, `_` or `-`"
        )
    };
    let single = names.len() <= 1;
    if single && !valid_name(&name) {
        return Err(invalid(&name));
    }
    let flag = [
        (args.skill.is_some(), Section::Skills),
        (args.agent, Section::Agents),
        (args.extension, Section::Extensions),
    ]
    .into_iter()
    .find_map(|(set, section)| set.then_some(section));
    if let Some((section, old)) = existing(&document, &name).filter(|_| single) {
        let same = old.git == entry.git && old.npm == entry.npm && old.path == entry.path;
        if !same || flag.is_some_and(|flag| flag != section) {
            bail!(
                "{}.{name} already comes from {}: choose another --name",
                section.key(),
                source(&old)
            );
        }
        if !args.grant.is_empty() && section != Section::Extensions {
            return Err(Usage("--grant is for extensions only".into()).into());
        }
        if args.isolated && section != Section::Extensions {
            return Err(Usage("--isolated is for extensions only".into()).into());
        }
        if args.isolated && old.events != Some(EventPolicy::isolated()) {
            return Err(Usage(format!(
                "{}.{name} keeps its events, so --isolated can't change them: edit its `events` in {}",
                section.key(),
                target.label()
            ))
            .into());
        }
        let current = Grants::parse(old.capabilities.as_deref().unwrap_or_default());
        if !args.grant.is_empty() && current != Ok(requested) {
            return Err(Usage(format!(
                "{}.{name} keeps its grants, so --grant can't change them: edit its `capabilities` in {}",
                section.key(),
                target.label()
            ))
            .into());
        }
        // The same source again moves its version or rev, keeping its grants.
        let updated = Entry {
            rev: entry.rev.clone().or(old.rev.clone()),
            version: entry.version.clone().or(old.version.clone()),
            ..old
        };
        document.set_entry(section, &name, &updated)?;
        write(target, &document.to_string())?;
        if let Err(err) = sync(target, Mode::Update(vec![name.clone()]), npm, prompt).await {
            write(target, &before)?;
            return Err(err);
        }
        prompt.outro(&format!("Updated {}.{name}", section.key()));
        return Ok(());
    }
    let project = open(target)?;
    let spinner = prompt.spinner(&format!("Fetching {}", args.source));
    let fetched = project.fetch_package(&package, npm).await?;
    spinner.stop(&format!("Fetched {}", args.source), &[]);
    if let Some(path) = &args.path
        && item(&fetched.root, &entry).symlink_metadata().is_err()
    {
        bail!("{} has no {path}", args.source);
    }
    let section = match flag {
        Some(section) => section,
        None => infer(
            &fetched.root,
            &entry,
            args.path.as_deref().unwrap_or(&args.source),
        )?,
    };
    let mut picks = match names.is_empty() {
        true => None,
        false => Some(named(&args, target, (&package, &fetched.root), &names)?),
    };
    if let Some([(_, only)]) = picks.as_deref() {
        entry = only.clone();
    }
    // A person picks from a package of several items, unless flags name one.
    let items = match prompt.interactive() && picks.is_none() && args.path.is_none() {
        true if section != Section::Extensions => fetched.items(section).unwrap_or_default(),
        _ => Vec::new(),
    };
    let picking = items.len() > 1;
    // Another entry for the package and `path`, for the whole package
    // beside one folder of it, or for a path source beside a folder inside
    // it, would load it twice.
    let identity = |entry: &Entry| {
        package_identity(entry)
            .ok_or_else(|| local_path(entry.path.as_deref().unwrap_or_default(), &target.base()))
    };
    let conflict = |document: &ManifestDocument, added: &Entry| {
        let added = identity(added);
        let same = |old: &Entry| match (identity(old), &added) {
            (Ok(old), Ok(added)) => same_package(&old, added),
            (Err(old), Err(added)) => old.starts_with(added) || added.starts_with(&old),
            _ => false,
        };
        let mut entries = document.manifest().section(section).iter();
        entries.find(|(_, old)| same(old)).map(|(other, old)| {
            anyhow::anyhow!(
                "{}.{other} already installs {}: change that entry instead",
                section.key(),
                source(old)
            )
        })
    };
    if single
        && !picking
        && let Some(err) = conflict(&document, &entry)
    {
        return Err(err);
    }
    if section == Section::Skills && item(&fetched.root, &entry).is_file() {
        let shown = args.path.as_deref().unwrap_or(&args.source);
        bail!(
            "{shown} is a single-file skill, and chest installs only skill folders that hold a SKILL.md"
        );
    }
    if section == Section::Extensions {
        let shown = args.path.as_deref().unwrap_or(&args.source);
        extension_file(&item(&fetched.root, &entry), shown)?;
    }
    let meta = package_meta(&fetched.root)?;
    prompt.info(&show(&meta));
    nested(target, &fetched.root, &name, prompt);
    if section == Section::Extensions {
        let inspected = inspect(&item(&fetched.root, &entry));
        if inspected.components.is_empty() && inspected.scripts.is_empty() {
            let shown = args.path.as_deref().unwrap_or(&args.source);
            bail!(
                "{shown} holds no extension. An extension folder needs an `index.ts` or `index.js`, an `extensions` folder, or a `package.json` or `chest.toml` that lists extension files it holds."
            );
        }
        let used = show_used(&inspected);
        if !used.is_empty() {
            prompt.info(&used);
        }
        let granted = grants(prompt, &args, &meta, &inspected.used())?;
        entry.capabilities = (!granted.is_empty()).then_some(granted);
        // `--isolated` answers the events question, and `--grant` keeps the
        // shared default without one.
        entry.events = match (args.isolated, args.grant.is_empty()) {
            (true, _) => Some(EventPolicy::isolated()),
            (false, true) => events(prompt, &meta)?,
            (false, false) => None,
        };
        for warning in requests(&project.manifest, &entry, &meta) {
            prompt.info(&[format!("note: {name} {warning}")]);
        }
    } else if !args.grant.is_empty() {
        return Err(Usage("--grant is for extensions only".into()).into());
    } else if args.isolated {
        return Err(Usage("--isolated is for extensions only".into()).into());
    }
    let (kind, one) = match section {
        Section::Skills => ("skills", "skill"),
        _ => ("subagents", "subagent"),
    };
    if picking {
        let package = (&package, fetched.root.as_path());
        picks = pick(prompt, (section, kind), &items, package, target)?;
    }
    let whole = picking && picks.is_none();
    let several = picking || !single;
    // Flags that answer every question install at once, as `--yes` does.
    let asked =
        picking || section == Section::Extensions && args.grant.is_empty() && prompt.interactive();
    let picks = picks.unwrap_or_else(|| vec![(name.clone(), entry.clone())]);
    let mut lines = Vec::new();
    for (name, entry) in &picks {
        if !valid_name(name) {
            return Err(invalid(name));
        }
        if let Some((section, old)) = existing(&document, name) {
            bail!(
                "{}.{name} already comes from {}: choose another --name",
                section.key(),
                source(&old)
            );
        }
        if let Some(err) = conflict(&document, entry) {
            return Err(err);
        }
        document.set_entry(section, name, entry)?;
        lines.push(format!("{}.{name}  {}", section.key(), source(entry)));
    }
    match picks.len() {
        _ if whole => lines.push(format!(
            "1 entry for the whole package, which gets new {kind} on update"
        )),
        1 if several => lines.push(format!("1 entry, for one {one}")),
        count if several => lines.push(format!("{count} entries, one per {one}")),
        _ => {}
    }
    if section == Section::Extensions {
        let capabilities = entry.capabilities.as_deref().unwrap_or_default();
        let events = match &entry.events {
            None => "shared",
            Some(events) if *events == EventPolicy::isolated() => "isolated",
            Some(_) => "declared channels",
        };
        lines.push(match capabilities.is_empty() {
            true => "capabilities: none".to_owned(),
            false => format!("capabilities: {}", capabilities.join(", ")),
        });
        lines.push(format!("events: {events}"));
    }
    if asked {
        prompt.summary(&format!("Add to {}", target.name()), &lines);
        prompt.confirm("Install?")?;
    }
    write(target, &document.to_string())?;
    if let Err(err) = sync(target, Mode::Resolve, npm, prompt).await {
        // The manifest goes back to what it was, so a failed add leaves no entry.
        write(target, &before)?;
        return Err(err);
    }
    let ids: Vec<String> = picks
        .iter()
        .map(|(name, _)| format!("{}.{name}", section.key()))
        .collect();
    prompt.outro(&format!("Added {}", ids.join(", ")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use yapi_core::packages::source::Source;

    #[test]
    fn reads_git_urls() {
        let repo = |source: &str| match super::parse(source).unwrap() {
            Source::Git { repo, .. } => repo,
            other => panic!("{source}: {other:?}"),
        };
        for (source, expected) in [
            ("git://example.com/a/b", "git://example.com/a/b"),
            ("git:git://example.com/a/b", "git://example.com/a/b"),
            ("ssh://git@example.com/a/b", "ssh://git@example.com/a/b"),
            ("https://example.com/a/b.git", "https://example.com/a/b"),
            ("http://example.com/a/b", "http://example.com/a/b"),
            ("git@example.com:a/b", "git@example.com:a/b"),
            (
                "ssh://me@example.com/srv/repo",
                "ssh://me@example.com/srv/repo",
            ),
        ] {
            assert_eq!(repo(source), expected, "{source}");
        }
        assert!(matches!(
            super::parse("./a@b:c").unwrap(),
            Source::Local { .. }
        ));
        assert!(super::parse("git@example.com:a").is_err());
        // An absolute path on the host takes `ssh://`, as in pi.
        for source in [
            "me@example.com:/srv/repo",
            "git:me@example.com:/srv/repo@v1",
        ] {
            let error = super::parse(source).unwrap_err().to_string();
            assert!(
                error.contains("use ssh://me@example.com/srv/repo"),
                "{error}"
            );
        }
    }

    #[test]
    fn events_text_names_choices() {
        let limits = |declared| {
            super::events_text(declared)
                .rsplit(". ")
                .next()
                .map(str::to_owned)
        };
        let declared = "Choose 'declared channels' or 'isolated' to limit that.";
        assert_eq!(limits(true).as_deref(), Some(declared));
        let isolated = "Choose 'isolated' to limit that.";
        assert_eq!(limits(false).as_deref(), Some(isolated));
    }

    #[test]
    fn capabilities_have_meanings() {
        for capability in yapi_core::extensions::CAPABILITIES {
            assert_ne!(
                super::meaning(capability),
                "an unknown capability",
                "{capability}"
            );
        }
    }
}
