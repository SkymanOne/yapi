//! `chest add`: looks at a package, asks what to grant it, writes its entry
//! and installs it.

use std::path::{Path, PathBuf};

use anyhow::bail;
use clap::Args;
use yapi_core::chest::Mode;
use yapi_core::chest::manifest::{
    Entry, Manifest, ManifestDocument, PackageMeta, Section, inside_package, package_meta,
    ssh_hint, valid_name,
};
use yapi_core::chest::startup::{package_identity, same_package};
use yapi_core::extensions::{EventPolicy, Grants};
use yapi_core::packages::npm::{Npm, check_name};
use yapi_core::packages::resolve::is_extension_file;
use yapi_core::packages::source::{self, Source, local_path};
use yapi_core::settings::Scope;
use yapi_core::tools::path::relative;

use crate::check::{
    Inspected, extension_file, inspect, say_capped, unknown_capability, unsupported_import,
};
use crate::prompt::Prompt;
use crate::{Target, Usage, open, read, requests, say, source, sync, write};

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
        let rest = format!("unsupported imports in {file}");
        say_capped("  warning: ", &found.unsupported, &rest, |import| {
            unsupported_import(file, import)
        });
        let rest = format!("unknown capability lines in {file}");
        say_capped("  warning: ", &found.unknown, &rest, |line| {
            unknown_capability(file, line)
        });
    }
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
        "net" => "fetch, call models and read provider credentials",
        "env" => "read environment variables",
        "session" => "start turns and change the session",
        "all" => "everything, including running programs",
        _ => "an unknown capability",
    }
}

/// The capabilities to grant: `--grant`, which [`run`] checked, where
/// `none` is the empty start, then each requested or `used` one the person
/// accepts. Requests that are not capabilities, such as `process`, are
/// skipped with a warning, so they are never asked about or written.
fn grants(
    prompt: &Prompt,
    args: &AddArgs,
    meta: &PackageMeta,
    used: &[&str],
) -> anyhow::Result<Vec<String>> {
    let mut granted: Vec<String> = args
        .grant
        .iter()
        .filter(|value| *value != "none")
        .cloned()
        .collect();
    if granted.iter().any(|grant| grant == "all") {
        say(ALL_WARNING);
    }
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
/// isolated. Shared when chest asks nothing.
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
    let requested = Grants::parse(&args.grant).map_err(|err| Usage(format!("--grant: {err}")))?;
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
        if !args.grant.is_empty() && section != Section::Extensions {
            return Err(Usage("--grant is for extensions only".into()).into());
        }
        let current = Grants::parse(old.capabilities.as_deref().unwrap_or_default());
        if !args.grant.is_empty() && current != Ok(requested) {
            return Err(Usage(format!(
                "{}.{name} keeps its grants, so --grant can't change them: edit its `capabilities` in {}",
                section.key(),
                target.name()
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
        if let Err(err) = sync(target, Mode::Update(vec![name.clone()]), npm).await {
            write(target, &before)?;
            return Err(err);
        }
        return Ok(());
    } else {
        let project = open(target)?;
        let fetched = project.fetch_package(&entry, npm).await?;
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
        // Another entry for the package and `path`, for the whole package
        // beside one folder of it, or for a path source beside a folder
        // inside it, would load it twice.
        let identity = |entry: &Entry| {
            package_identity(entry).ok_or_else(|| {
                local_path(entry.path.as_deref().unwrap_or_default(), &target.base())
            })
        };
        let added = identity(&entry);
        let same = |old: &Entry| match (identity(old), &added) {
            (Ok(old), Ok(added)) => same_package(&old, added),
            (Err(old), Err(added)) => old.starts_with(added) || added.starts_with(&old),
            _ => false,
        };
        let mut entries = document.manifest().section(section).iter();
        if let Some((other, old)) = entries.find(|(_, old)| same(old)) {
            bail!(
                "{}.{other} already installs {}: change that entry instead",
                section.key(),
                source(old)
            );
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
        show(&meta);
        nested(&fetched.root, &name);
        if section == Section::Extensions {
            let inspected = inspect(&item(&fetched.root, &entry));
            if inspected.components.is_empty() && inspected.scripts.is_empty() {
                let shown = args.path.as_deref().unwrap_or(&args.source);
                bail!(
                    "{shown} holds no extension. An extension folder needs an `index.ts` or `index.js`, an `extensions` folder, or a `package.json` or `chest.toml` that lists extension files it holds."
                );
            }
            show_used(&inspected);
            let prompt = Prompt::new(args.yes);
            let granted = grants(&prompt, &args, &meta, &inspected.used())?;
            entry.capabilities = (!granted.is_empty()).then_some(granted);
            entry.events = events(&prompt, &meta);
            for warning in requests(&project.manifest, &entry, &meta) {
                say(&format!("note: {name} {warning}"));
            }
        } else if !args.grant.is_empty() {
            return Err(Usage("--grant is for extensions only".into()).into());
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
