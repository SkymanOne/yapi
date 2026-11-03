//! `chest`: one manifest and lock for agent skills, subagents and yapi
//! extensions, built on the engine in `yapi_core::chest`. The commands and
//! their exit codes are described in docs/chest.md.

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
use tokio_util::sync::CancellationToken;
use yapi_ai::auth::lock::{FileLock, write_atomic};
use yapi_core::chest::manifest::{
    Entry, Manifest, ManifestDocument, PackageMeta, Section, package_meta,
};
use yapi_core::chest::{Mode, Outcome, Project, Report};
use yapi_core::packages::npm::{self, Npm};
use yapi_core::settings::Scope;
use yapi_core::tools::path::home_dir;

#[derive(Parser)]
#[command(
    name = "chest",
    version,
    about = "One manifest and lock for agent skills, subagents and yapi extensions"
)]
struct Cli {
    /// Use the global manifest, ~/.chest/chest.toml.
    #[arg(short, long, global = true)]
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
        /// Install exactly what the lock pins, missing items included, and
        /// fail when the lock is missing or out of date or a copy no longer
        /// matches it. Never writes chest.toml or the lock. The default when
        /// CI is set.
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
    /// Turn settings.json packages or a skills-lock.json into entries.
    Import(import::ImportArgs),
    /// Compare the capabilities a package's native extensions use with
    /// those it requests.
    Check {
        /// The package folder or `.wasm` file, the current folder by default.
        path: Option<PathBuf>,
    },
}

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

    /// The manifest's file name, as messages name it.
    pub fn name(&self) -> String {
        let name = self.path.file_name().unwrap_or_default();
        name.to_string_lossy().into_owned()
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    let result = match runtime {
        Ok(runtime) => runtime.block_on(run(cli)),
        Err(err) => Err(err.into()),
    };
    result.unwrap_or_else(|err| {
        // Errors quote the manifest and lock, and git's stderr keeps its lines.
        let error = printable(&format!("{err:#}"), &['\t', '\n']);
        let _ = writeln!(std::io::stderr(), "error: {error}");
        ExitCode::FAILURE
    })
}

async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
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
    // Commands that write the manifest hold its lock from their first read
    // to their last write, a rollback included. Init and import may create
    // its folder, and a missing folder holds no manifest to protect.
    let writes = match &cli.command {
        Command::Init(_) | Command::Import(_) => {
            std::fs::create_dir_all(target.base())?;
            true
        }
        Command::Add(_) | Command::Remove { .. } | Command::Update { .. } => target.base().is_dir(),
        _ => false,
    };
    let _held = match writes {
        true => Some(lock(&target).await?),
        false => None,
    };
    let mut npm = Npm::new(npm::config());
    match cli.command {
        Command::Init(args) => init::run(&target, &args)?,
        Command::Add(args) => add::run(&target, args, &mut npm).await?,
        Command::Remove { name } => remove(&target, &name, &mut npm).await?,
        Command::Install { frozen } => {
            let mode = if frozen || ci() {
                Mode::Frozen
            } else {
                Mode::Resolve
            };
            sync(&target, mode, &mut npm).await?;
        }
        Command::Update { names } => update(&target, names, &mut npm).await?,
        Command::List => list(&target)?,
        Command::Outdated => outdated(&target, &mut npm).await?,
        Command::Verify => return verify(&target),
        Command::Import(args) => import::run(&target, &args)?,
        Command::Check { path } => {
            return check::run(path.as_deref().unwrap_or(Path::new(".")));
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Locks the manifest against other chest commands that write it, waiting
/// for them. The lock is an empty folder, which git does not list, named
/// apart from the lock `sync` takes on the lock file, even for a manifest
/// without an extension.
async fn lock(target: &Target) -> anyhow::Result<FileLock> {
    let file = target.base().join(format!(".{}", target.name()));
    FileLock::acquire(&file, &CancellationToken::new())
        .await
        .with_context(|| format!("locking {}", target.path.display()))
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

/// Refuses a manifest that is a symbolic link, which a cloned project could
/// point at a file outside it.
fn refuse_link(target: &Target) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(&target.path);
    if metadata.is_ok_and(|metadata| metadata.is_symlink()) {
        bail!(
            "{} is a symbolic link, which chest does not follow: replace it with the file itself",
            target.path.display()
        );
    }
    Ok(())
}

/// The manifest's text, with a pointer to `chest init` when it is missing.
pub fn read(target: &Target) -> anyhow::Result<String> {
    refuse_link(target)?;
    if !target.path.exists() {
        bail!(
            "{} does not exist: run `chest init` first",
            target.path.display()
        );
    }
    std::fs::read_to_string(&target.path)
        .with_context(|| format!("reading {}", target.path.display()))
}

/// Writes the manifest's new text, whole or not at all.
pub fn write(target: &Target, text: &str) -> anyhow::Result<()> {
    refuse_link(target)?;
    write_atomic(&target.path, text.as_bytes(), 0o666)
        .with_context(|| format!("writing {}", target.path.display()))
}

/// Opens the manifest and its lock.
pub fn open(target: &Target) -> anyhow::Result<Project> {
    read(target)?;
    Ok(Project::open(&target.path, target.scope)?)
}

/// Syncs the manifest as `mode` says and prints what happened.
pub async fn sync(target: &Target, mode: Mode, npm: &mut Npm) -> anyhow::Result<Project> {
    let mut project = open(target)?;
    let report = project.sync(mode, npm, &mut |line| say(line)).await?;
    print_report(&report);
    Ok(project)
}

/// Prints each entry's outcome on stdout, then notices and warnings on
/// stderr.
pub fn print_report(report: &Report) {
    for (id, outcome) in &report.entries {
        let word = match outcome {
            Outcome::Installed => "installed",
            Outcome::Replayed => "replayed",
            Outcome::Unchanged => "unchanged",
            Outcome::Skipped => "skipped",
            Outcome::Removed => "removed",
        };
        out(&format!("{word:<9}  {id}"));
    }
    for notice in &report.notices {
        say(&format!("note: {notice}"));
    }
    for warning in &report.warnings {
        say(&format!("warning: {warning}"));
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

async fn remove(target: &Target, name: &str, npm: &mut Npm) -> anyhow::Result<()> {
    let mut document = ManifestDocument::parse(&read(target)?, target.scope)?;
    let mut found = false;
    for section in Section::ALL {
        found |= document.remove_entry(section, name)?;
    }
    if !found {
        bail!("No entry is named {name}");
    }
    write(target, &document.to_string())?;
    sync(target, Mode::Resolve, npm).await?;
    Ok(())
}

async fn update(target: &Target, names: Vec<String>, npm: &mut Npm) -> anyhow::Result<()> {
    let project = sync(target, Mode::Update(names), npm).await?;
    for (name, entry) in &project.manifest.extensions {
        if let Some(root) = project.installed_root(name) {
            let meta = match package_meta(&root) {
                Ok(meta) => meta,
                Err(err) => {
                    say(&format!("warning: extensions.{name}: {err}"));
                    continue;
                }
            };
            for warning in requests(&project.manifest, entry, &meta) {
                say(&format!("warning: extensions.{name} {warning}"));
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
        let state = match locked {
            Some(locked) if project.present(section, locked) => "installed",
            _ => "missing",
        };
        out(&format!("{id}  {pinned}  {state}  {}", source(entry)));
        if section == Section::Extensions {
            out(&format!(
                "    capabilities: {}",
                project.manifest.grants(entry)
            ));
        }
        match project.installed_root(name).map(|root| package_meta(&root)) {
            Some(Ok(meta)) => {
                if let Some(description) = meta.description {
                    out(&format!("    {description}"));
                }
            }
            Some(Err(err)) => say(&format!("warning: {id}: {err}")),
            None => {}
        }
    }
    Ok(())
}

async fn outdated(target: &Target, npm: &mut Npm) -> anyhow::Result<()> {
    let project = open(target)?;
    let short = |text: &str| text[..text.len().min(12)].to_owned();
    let mut any = false;
    for row in project.outdated(npm).await? {
        let latest = row.latest.filter(|latest| *latest != row.wanted);
        if row.wanted == row.locked && latest.is_none() {
            continue;
        }
        any = true;
        let mut line = format!(
            "{}  {} -> {}",
            row.id,
            short(&row.locked),
            short(&row.wanted)
        );
        if let Some(latest) = latest {
            line.push_str(&format!("  (latest {latest})"));
        }
        out(&line);
    }
    if !any {
        say("Everything is up to date.");
    }
    Ok(())
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
