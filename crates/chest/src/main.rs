//! `chest`: one manifest and lock for agent skills, subagents and yapi
//! extensions, built on the engine in `yapi_core::chest`.
//! docs/chest-commands.md describes the commands and their exit codes.

mod add;
mod check;
mod import;
mod init;
mod prompt;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, bail};
use clap::{Parser, Subcommand};
use yapi_ai::auth::lock::{FileLock, write_atomic};
use yapi_core::chest::lock::Locked;
use yapi_core::chest::manifest::{
    Entry, Manifest, ManifestDocument, PackageMeta, Section, package_meta,
};
use yapi_core::chest::{Mode, Outcome, Outdated, Project};
use yapi_core::packages::npm::{self, Npm};
use yapi_core::settings::Scope;
use yapi_core::tools::path::home_dir;

use crate::prompt::{Cancelled, Prompt};

#[derive(Parser)]
#[command(
    name = "chest",
    version,
    about = "One manifest and lock for agent skills, subagents and yapi extensions"
)]
struct Cli {
    /// Use the global manifest, ~/.chest/chest.toml.
    #[arg(short, long, global = true, conflicts_with = "manifest")]
    global: bool,
    /// Use this manifest instead of chest.toml in the current folder.
    /// CHEST_MANIFEST sets the default.
    #[arg(long, global = true, value_name = "PATH")]
    manifest: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create chest.toml with folders for the agents found here.
    Init(init::InitArgs),
    /// Add a skill, subagent or extension and install it.
    Add(add::AddArgs),
    /// Remove an entry and what chest installed for it.
    Remove {
        /// The entry's name.
        name: String,
    },
    /// Install what the manifest lists, as the lock pins it.
    Install {
        /// Install exactly what the lock pins, missing items and copies an
        /// older lock pinned included, and fail when the lock is missing or
        /// out of date or a copy that someone edited, or that chest didn't
        /// install, no longer matches it. Never writes chest.toml or the
        /// lock. The default when CI is set.
        #[arg(long)]
        frozen: bool,
    },
    /// Move entries to the newest version or commit their source allows.
    Update {
        /// The entries to update, all of them when none is given.
        names: Vec<String>,
    },
    /// List each entry with its locked version, state and capabilities.
    List,
    /// Show entries whose source has a newer version or commit.
    Outdated,
    /// Check every installed copy against the lock.
    Verify,
    /// Turn settings packages or a skills-lock.json into entries.
    Import(import::ImportArgs),
    /// Compare the capabilities a package's native extensions use with
    /// those it requests.
    Check {
        /// The package folder or `.wasm` file, the current folder by default.
        path: Option<PathBuf>,
    },
}

/// A usage error that clap cannot see, such as a flag that conflicts with
/// the source. chest exits with 2 on it, as on clap's own.
#[derive(Debug)]
pub struct Usage(pub String);

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Usage {}

/// The manifest a command works on.
pub struct Target {
    /// Its path.
    pub path: PathBuf,
    /// Whether it is a project's or the global one.
    pub scope: Scope,
}

impl Target {
    /// The folder its relative paths start from.
    pub fn base(&self) -> PathBuf {
        self.path
            .parent()
            .map_or_else(PathBuf::new, Path::to_path_buf)
    }

    /// The manifest's file name.
    pub fn name(&self) -> String {
        let name = self.path.file_name().unwrap_or_default();
        name.to_string_lossy().into_owned()
    }

    /// `chest` with the flag that selects the manifest, for the commands
    /// messages name, such as `chest -g`.
    pub fn command(&self) -> String {
        yapi_core::chest::command(&self.path, self.scope)
    }

    /// The manifest as messages name it: its file name in a project, and
    /// `~/.chest/chest.toml` for the global one.
    pub fn label(&self) -> String {
        let home = yapi_core::tools::path::home_dir();
        yapi_core::chest::manifest_label(&self.path, self.scope, &home)
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // The commands that change things draw their questions and progress at
    // a terminal. The rest print plain lines.
    let (title, yes) = match &cli.command {
        Command::Init(args) => ("chest init", args.yes),
        Command::Add(args) => ("chest add", args.yes),
        Command::Remove { .. } => ("chest remove", false),
        Command::Install { .. } => ("chest install", false),
        Command::Update { .. } => ("chest update", false),
        Command::Import(_) => ("chest import", false),
        _ => ("", true),
    };
    let prompt = Prompt::new(yes);
    prompt.intro(title);
    // A worker of its own keeps held locks fresh while a question waits.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build();
    let result = match runtime {
        Ok(runtime) => runtime.block_on(run(cli, &prompt)),
        Err(err) => Err(err.into()),
    };
    result.unwrap_or_else(|err| {
        if err.is::<Cancelled>() {
            return ExitCode::FAILURE;
        }
        // Errors quote the manifest and lock, and git's stderr keeps its lines.
        let error = printable(&format!("{err:#}"), &['\t', '\n']);
        prompt.error(&format!("error: {error}"));
        match err.is::<Usage>() {
            true => ExitCode::from(2),
            false => ExitCode::FAILURE,
        }
    })
}

async fn run(cli: Cli, prompt: &Prompt) -> anyhow::Result<ExitCode> {
    let scope = if cli.global {
        Scope::Global
    } else {
        Scope::Project
    };
    // Without --manifest, CHEST_MANIFEST names the project's manifest, by
    // the rule yapi reads it with.
    let manifest = match &cli.manifest {
        Some(path) => Some(path.clone()),
        None if cli.global => None,
        None => yapi_core::chest::manifest::from_env()
            .map_err(anyhow::Error::msg)?
            .map(PathBuf::from),
    };
    let path = match (&manifest, cli.global) {
        (Some(path), _) => std::path::absolute(path)?,
        (None, true) => home_dir().join(".chest/chest.toml"),
        (None, false) => std::env::current_dir()?.join("chest.toml"),
    };
    let target = Target { path, scope };
    // Commands that write the manifest, and installs, which remove what it
    // no longer lists, hold its lock from their first read to their last
    // write, a rollback included. Init and import may create its folder, and
    // a missing folder holds no manifest to protect. A frozen install skips
    // it, so a read-only project passes. It writes neither file, and fails
    // when the lock it reads again under the lock's lock no longer fits the
    // manifest.
    let frozen = matches!(cli.command, Command::Install { frozen } if frozen || ci());
    let writes = match &cli.command {
        Command::Init(_) | Command::Import(_) => {
            std::fs::create_dir_all(target.base())?;
            true
        }
        Command::Install { .. } if frozen => false,
        Command::Add(_)
        | Command::Remove { .. }
        | Command::Update { .. }
        | Command::Install { .. } => target.base().is_dir(),
        _ => false,
    };
    let _held = match writes {
        true => Some(lock(&target, prompt).await?),
        false => None,
    };
    yapi_core::settings::apply_http_proxy();
    let mut npm = Npm::new(npm::config());
    match cli.command {
        Command::Init(args) => init::run(&target, &args, prompt)?,
        Command::Add(args) => add::run(&target, args, &mut npm, prompt).await?,
        Command::Remove { name } => {
            remove(&target, &name, &mut npm, prompt).await?;
            prompt.outro("Done");
        }
        Command::Install { .. } => {
            let mode = if frozen { Mode::Frozen } else { Mode::Resolve };
            sync(&target, mode, &mut npm, prompt).await?;
            prompt.outro("Done");
        }
        Command::Update { names } => {
            update(&target, names, &mut npm, prompt).await?;
            prompt.outro("Done");
        }
        Command::List => list(&target)?,
        Command::Outdated => outdated(&target, &mut npm).await?,
        Command::Verify => return verify(&target),
        Command::Import(args) => import::run(&target, &args, prompt)?,
        Command::Check { path } => {
            return check::run(path.as_deref().unwrap_or(Path::new(".")));
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Locks the manifest against other chest commands that write it, waiting
/// for them for as long as they run, and saying once on stderr that it
/// waits. The lock is an empty folder, which git does not list. Its name
/// differs from the lock `sync` takes on the lock file, even for a manifest
/// without an extension.
async fn lock(target: &Target, prompt: &Prompt) -> anyhow::Result<FileLock> {
    let file = target.base().join(format!(".{}", target.name()));
    let path = target.path.display();
    let mut waiting = || {
        prompt.info(&[format!(
            "Waiting for another chest command that changes {path}..."
        )])
    };
    yapi_core::chest::hold_lock(&file, &mut waiting)
        .await
        .with_context(|| format!("locking {path}"))
}

/// Whether `CI` asks for frozen installs: set, and not empty, `0` or
/// `false`.
fn ci() -> bool {
    std::env::var("CI").is_ok_and(|value| {
        !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false"
        )
    })
}

/// `text` without control characters other than those in `keep`, so text
/// from a package or project cannot move the cursor or redraw the terminal.
fn printable(text: &str, keep: &[char]) -> String {
    text.chars()
        .filter(|c| keep.contains(c) || !c.is_control())
        .collect()
}

/// Prints `line` on stderr, without control characters.
pub fn say(line: &str) {
    let _ = writeln!(std::io::stderr(), "{}", printable(line, &['\t']));
}

/// Prints `line` on stdout, without control characters.
pub fn out(line: &str) {
    let _ = writeln!(std::io::stdout(), "{}", printable(line, &['\t']));
}

/// The manifest's text, with a pointer to `chest init` when it is missing.
pub fn read(target: &Target) -> anyhow::Result<String> {
    yapi_core::chest::refuse_link(&target.path)?;
    if !target.path.exists() {
        bail!(
            "{} does not exist: run `{} init` first",
            target.path.display(),
            target.command()
        );
    }
    std::fs::read_to_string(&target.path)
        .with_context(|| format!("reading {}", target.path.display()))
}

/// Writes the manifest's new text, whole or not at all.
pub fn write(target: &Target, text: &str) -> anyhow::Result<()> {
    yapi_core::chest::refuse_link(&target.path)?;
    write_atomic(&target.path, text.as_bytes(), 0o666)
        .with_context(|| format!("writing {}", target.path.display()))
}

/// Opens the manifest and its lock.
pub fn open(target: &Target) -> anyhow::Result<Project> {
    read(target)?;
    Ok(Project::open(&target.path, target.scope)?)
}

/// Syncs the manifest as `mode` says and prints what happened: each
/// entry's outcome on stdout, then notices and warnings on stderr. When
/// drawn, the outcomes are a marked and coloured summary.
pub async fn sync(
    target: &Target,
    mode: Mode,
    npm: &mut Npm,
    prompt: &Prompt,
) -> anyhow::Result<Project> {
    let mut project = open(target)?;
    let spinner = prompt.spinner(&format!("Checking {}", target.label()));
    let report = project
        .sync(mode, npm, &mut |line| spinner.step(line))
        .await?;
    let mut counts: Vec<(&str, usize)> = Vec::new();
    let mut lines = Vec::new();
    for (id, outcome) in &report.entries {
        let (mark, word) = outcome_mark(*outcome);
        match counts.iter_mut().find(|(known, _)| *known == word) {
            Some((_, count)) => *count += 1,
            None => counts.push((word, 1)),
        }
        let line = format!("{word:<9}  {id}");
        if !prompt.styled() {
            out(&line);
            continue;
        }
        let line = console::style(format!("{mark} {}", printable(&line, &[])));
        lines.push(
            match outcome {
                Outcome::Installed | Outcome::Replayed => line.green(),
                Outcome::Unchanged => line,
                Outcome::Edited | Outcome::Skipped => line.yellow(),
                Outcome::Removed => line.dim(),
            }
            .to_string(),
        );
    }
    let counts: Vec<String> = counts
        .iter()
        .map(|(word, count)| format!("{count} {word}"))
        .collect();
    let head = match counts.is_empty() {
        true => "Nothing to install".to_owned(),
        false => counts.join(", "),
    };
    spinner.stop(&head, &lines);
    for notice in &report.notices {
        prompt.info(&[format!("note: {notice}")]);
    }
    for warning in &report.warnings {
        prompt.warn(&format!("warning: {warning}"));
    }
    Ok(project)
}

/// The mark and word for an outcome in reports.
fn outcome_mark(outcome: Outcome) -> (char, &'static str) {
    match outcome {
        Outcome::Installed => ('+', "installed"),
        Outcome::Replayed => ('+', "replayed"),
        Outcome::Unchanged => ('=', "unchanged"),
        Outcome::Edited => ('~', "edited"),
        Outcome::Skipped => ('!', "skipped"),
        Outcome::Removed => ('-', "removed"),
    }
}

/// One line for an entry's source, such as `git <url> v1`.
pub fn source(entry: &Entry) -> String {
    let (kind, location, version) = match (&entry.git, &entry.npm) {
        (Some(git), _) => ("git", git.as_str(), entry.rev.as_deref()),
        (None, Some(npm)) => ("npm", npm.as_str(), entry.version.as_deref()),
        (None, None) => ("path", entry.path.as_deref().unwrap_or_default(), None),
    };
    let mut line = format!("{kind} {location}");
    if let Some(version) = version {
        line.push_str(&format!(" {version}"));
    }
    if let (Some(path), true) = (&entry.path, kind != "path") {
        line.push_str(&format!(" ({path})"));
    }
    line
}

async fn remove(target: &Target, name: &str, npm: &mut Npm, prompt: &Prompt) -> anyhow::Result<()> {
    let mut document = ManifestDocument::parse(&read(target)?, target.scope)?;
    let mut found = false;
    for section in Section::ALL {
        found |= document.remove_entry(section, name)?;
    }
    if !found {
        bail!("No entry is named {name}");
    }
    write(target, &document.to_string())?;
    sync(target, Mode::Resolve, npm, prompt).await?;
    Ok(())
}

async fn update(
    target: &Target,
    names: Vec<String>,
    npm: &mut Npm,
    prompt: &Prompt,
) -> anyhow::Result<()> {
    let project = sync(target, Mode::Update(names), npm, prompt).await?;
    for (name, entry) in &project.manifest.extensions {
        if let Some(root) = project.installed_root(name) {
            let meta = match package_meta(&root) {
                Ok(meta) => meta,
                Err(err) => {
                    prompt.warn(&format!("warning: extensions.{name}: {err}"));
                    continue;
                }
            };
            for warning in requests(&project.manifest, entry, &meta) {
                prompt.warn(&format!("warning: extensions.{name} {warning}"));
            }
        }
    }
    Ok(())
}

/// What a package at `meta` asks for beyond what `entry` grants it: one line
/// for capabilities, one for event channels.
pub fn requests(manifest: &Manifest, entry: &Entry, meta: &PackageMeta) -> Vec<String> {
    let granted = manifest.grants(entry);
    let covered =
        |capability: &str| granted.is_all() || capability == "none" || granted.covers(capability);
    let mut warnings = Vec::new();
    let missing: Vec<&str> = meta
        .capabilities
        .iter()
        .map(String::as_str)
        .filter(|capability| !covered(capability))
        .collect();
    if !missing.is_empty() {
        warnings.push(format!(
            "requests capabilities it is not granted: {}. Add them to its `capabilities` to allow them.",
            missing.join(", ")
        ));
    }
    if let (Some(policy), Some(asked)) = (&entry.events, &meta.events) {
        let unsent = asked.send.iter().filter(|channel| !policy.sends(channel));
        let unheard = asked
            .receive
            .iter()
            .filter(|channel| !policy.receives(channel));
        let outside: Vec<&str> = unsent.chain(unheard).map(String::as_str).collect();
        if !outside.is_empty() {
            warnings.push(format!(
                "uses event channels outside its `events` lists: {}",
                outside.join(", ")
            ));
        }
    }
    warnings
}

fn list(target: &Target) -> anyhow::Result<()> {
    let project = open(target)?;
    for (section, name, entry) in project.manifest.entries() {
        let id = format!("{}.{name}", section.key());
        let locked = project.lock.as_ref().and_then(|lock| {
            let entries = lock.section(section).iter();
            entries.into_iter().find(|locked| &locked.name == name)
        });
        let pinned = locked
            .and_then(|locked| {
                let commit = locked
                    .commit
                    .as_deref()
                    .map(|commit| &commit[..commit.len().min(12)]);
                commit.or(locked.version.as_deref()).map(str::to_owned)
            })
            .unwrap_or_else(|| {
                if locked.is_some() {
                    "locked".into()
                } else {
                    "not locked".into()
                }
            });
        let state = project.state(section, name, entry);
        out(&format!("{id}  {pinned}  {state}  {}", source(entry)));
        if section == Section::Extensions {
            out(&format!(
                "    capabilities: {}",
                project.manifest.grants(entry)
            ));
        }
        let description = match project.installed_root(name).map(|root| package_meta(&root)) {
            Some(Ok(meta)) => meta.description,
            Some(Err(err)) => {
                say(&format!("warning: {id}: {err}"));
                None
            }
            None => locked.and_then(|locked| item_description(&project, section, locked)),
        };
        if let Some(description) = description {
            out(&format!("    {description}"));
        }
    }
    Ok(())
}

/// The frontmatter `description` of the skill or subagent `locked` places,
/// from its canonical copy. `None` for an entry that places several, or
/// none.
fn item_description(project: &Project, section: Section, locked: &Locked) -> Option<String> {
    let placed = project.placed(section, locked);
    let copy = placed.first()?;
    if placed
        .iter()
        .any(|path| path.file_name() != copy.file_name())
    {
        return None;
    }
    frontmatter(section, copy, "description")
}

/// The frontmatter value `key` of the skill folder or subagent file at
/// `item`.
pub fn frontmatter(section: Section, item: &Path, key: &str) -> Option<String> {
    let file = match section {
        Section::Skills => item.join("SKILL.md"),
        _ => item.to_path_buf(),
    };
    let text = std::fs::read_to_string(file).ok()?;
    let (frontmatter, _) = yapi_core::resources::parse_frontmatter(&text).ok()?;
    frontmatter.get(key)?.as_str().map(str::to_owned)
}

async fn outdated(target: &Target, npm: &mut Npm) -> anyhow::Result<()> {
    let project = open(target)?;
    let mut any = false;
    for line in project
        .outdated(npm)
        .await?
        .iter()
        .filter_map(outdated_line)
    {
        any = true;
        out(&line);
    }
    if !any {
        say("Everything is up to date.");
    }
    Ok(())
}

/// What `chest outdated` says of `row`, if anything: the update it has,
/// and npm's `latest` when that's newer still, outside the entry's range.
fn outdated_line(row: &Outdated) -> Option<String> {
    let short = |text: &str| text[..text.len().min(12)].to_owned();
    // A `latest` the range already passed, as when the tag lags, is no news.
    let latest = row.latest.as_ref();
    let latest = latest.filter(|latest| npm::newer(latest, &row.wanted) != Some(false));
    let (id, locked) = (&row.id, short(&row.locked));
    match latest {
        None if row.wanted == row.locked => None,
        Some(latest) if row.wanted == row.locked => Some(format!(
            "{id}  {locked}  (latest {latest} is outside its range)"
        )),
        _ => {
            let update = format!("{id}  {locked} -> {}", short(&row.wanted));
            let latest = latest.map(|latest| format!("  (latest {latest})"));
            Some(update + &latest.unwrap_or_default())
        }
    }
}

fn verify(target: &Target) -> anyhow::Result<ExitCode> {
    let project = open(target)?;
    let problems = project.verify()?;
    if problems.is_empty() {
        say("Every installed copy matches the lock.");
        return Ok(ExitCode::SUCCESS);
    }
    for problem in problems {
        out(&problem);
    }
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outdated_lines_read() {
        let row = |locked: &str, wanted: &str, latest: Option<&str>| Outdated {
            id: "skills.review".into(),
            locked: locked.into(),
            wanted: wanted.into(),
            latest: latest.map(Into::into),
        };
        let cases = [
            (row("1.2.0", "1.2.0", None), None),
            (row("1.2.0", "1.2.0", Some("1.2.0")), None),
            (row("1.2.0", "1.2.0", Some("1.1.0")), None),
            (
                row("1.2.0", "1.2.0", Some("2.0.0")),
                Some("skills.review  1.2.0  (latest 2.0.0 is outside its range)"),
            ),
            (
                row("1.0.0", "1.2.0", Some("1.2.0")),
                Some("skills.review  1.0.0 -> 1.2.0"),
            ),
            (
                row("1.0.0", "1.2.0", Some("2.0.0")),
                Some("skills.review  1.0.0 -> 1.2.0  (latest 2.0.0)"),
            ),
        ];
        for (row, line) in cases {
            assert_eq!(outdated_line(&row).as_deref(), line, "{row:?}");
        }
    }
}
