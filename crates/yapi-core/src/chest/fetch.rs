//! Getting entries' contents: git checkouts through a cached bare clone,
//! and the items a package provides.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio_util::sync::CancellationToken;
use yapi_ai::auth::lock::FileLock;

use super::manifest::{Section, package_meta, valid_name};
use super::{ChestError, invalid, io};
use crate::packages::resolve::{ResourceType, selected_files};
use crate::packages::source::{self, Source};

/// A scratch folder for one sync, removed when dropped.
pub(super) struct Work {
    dir: PathBuf,
    next: usize,
}

impl Work {
    /// A new, empty work folder under `cache`, its own even within one
    /// process. Folders that processes no longer running left behind go.
    pub(super) fn new(cache: &Path) -> Result<Work, ChestError> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let work = cache.join("work");
        for entry in std::fs::read_dir(&work).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let pid = name.split_once('-').and_then(|(pid, _)| pid.parse().ok());
            if pid.is_some_and(|pid| !running(pid)) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
        let dir = work.join(format!("{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(io(&dir))?;
        Ok(Work { dir, next: 0 })
    }

    /// A path in the work folder that nothing uses yet.
    pub(super) fn path(&mut self) -> PathBuf {
        self.next += 1;
        self.dir.join(self.next.to_string())
    }
}

/// Whether process `pid` is running, or may be: elsewhere than Unix, always.
fn running(pid: i32) -> bool {
    #[cfg(unix)]
    {
        let Some(pid) = rustix::process::Pid::from_raw(pid) else {
            return false;
        };
        // A process of another user answers with a permission error.
        !matches!(
            rustix::process::test_kill_process(pid),
            Err(rustix::io::Errno::SRCH)
        )
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn git(args: &[&str], dir: Option<&Path>) -> Result<String, ChestError> {
    crate::packages::run_git(args, dir)
        .await
        .map_err(|err| ChestError::Failed(err.to_string()))
}

/// Git run without the user's or the system's config, so that no filter,
/// LFS hook or line-ending conversion changes the bytes it checks out. The
/// clone it makes also takes no template. Only fetches into the cache use
/// the user's config, for credentials.
async fn clean_git(args: &[&str], dir: Option<&Path>) -> Result<String, ChestError> {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let env = [
        ("GIT_CONFIG_GLOBAL", null),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_COUNT", "0"),
        ("GIT_CONFIG_PARAMETERS", ""),
    ];
    let clean = ["-c", "core.autocrlf=false", "-c", "core.eol=lf"];
    let args: Vec<&str> = clean.into_iter().chain(args.iter().copied()).collect();
    crate::packages::run_git_with(&args, dir, &env)
        .await
        .map_err(|err| ChestError::Failed(err.to_string()))
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Where a mirror keeps the commit its remote's default branch is at, as of
/// the last fetch, so that a renamed default branch is followed.
const HEAD: &str = "refs/chest/head";

/// The bare clone of `repo` in `cache`, made on first use and fetched again
/// when `refresh`, with [`HEAD`] where the remote's `HEAD` is. Only clones
/// and fetches go to the network.
async fn mirror(cache: &Path, repo: &str, refresh: bool) -> Result<PathBuf, ChestError> {
    let Source::Git {
        repo: url,
        host,
        path,
        ..
    } = source::parse(&format!("git:{repo}"))
    else {
        return Err(invalid(format!("Not a git repository: {repo}")));
    };
    let dir = cache.join("git").join(host).join(path);
    let parent = dir.parent().unwrap_or(cache);
    std::fs::create_dir_all(parent).map_err(io(parent))?;
    // Projects share the cache, so one process changes a clone at a time.
    let _held = FileLock::acquire(&dir, &CancellationToken::new())
        .await
        .map_err(|err| ChestError::Failed(format!("{}: {err}", dir.display())))?;
    if !dir.join("HEAD").is_file() {
        let _ = std::fs::remove_dir_all(&dir);
        git(
            &["clone", "--bare", "--quiet", "--", &url, &text(&dir)],
            None,
        )
        .await?;
        // An empty repository has no `HEAD` commit to note.
        let _ = git(&["update-ref", HEAD, "HEAD"], Some(&dir)).await;
    } else if refresh {
        let heads = "+refs/heads/*:refs/heads/*";
        // Named, not `--tags`, so that `--prune` drops tags deleted upstream.
        let tags = "+refs/tags/*:refs/tags/*";
        let head = format!("+HEAD:{HEAD}");
        let fetch = [
            "fetch", "--quiet", "--prune", "--force", "--", &url, heads, tags, &head,
        ];
        git(&fetch, Some(&dir)).await?;
    }
    Ok(dir)
}

/// The commit `rev` names in `mirror`, if any.
async fn commit_of(mirror: &Path, rev: &str) -> Option<String> {
    let spec = format!("{rev}^{{commit}}");
    let out = git(&["rev-parse", "--verify", "--quiet", &spec], Some(mirror)).await;
    out.ok().map(|out| out.trim().to_owned())
}

/// The commit that `rev` of `repo` names now, the remote's default branch's
/// without one. The manifest refuses a `rev` that git could read as an
/// option.
pub(super) async fn resolve(
    cache: &Path,
    repo: &str,
    rev: Option<&str>,
) -> Result<String, ChestError> {
    let mirror = mirror(cache, repo, true).await?;
    commit_of(&mirror, rev.unwrap_or(HEAD))
        .await
        .ok_or_else(|| {
            let rev = rev.unwrap_or("HEAD");
            invalid(format!("{repo} has no revision {rev}"))
        })
}

/// Checks out `commit` of `repo` into `dest`, a path that does not exist yet,
/// without git's own files.
pub(super) async fn checkout(
    cache: &Path,
    repo: &str,
    commit: &str,
    dest: &Path,
) -> Result<(), ChestError> {
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid(format!("Not a commit: {commit}")));
    }
    let mut cached = mirror(cache, repo, false).await?;
    if commit_of(&cached, commit).await.is_none() {
        cached = mirror(cache, repo, true).await?;
        if commit_of(&cached, commit).await.is_none() {
            return Err(invalid(format!("{repo} no longer has commit {commit}")));
        }
    }
    // An empty template keeps template config, hooks and filters out of the clone.
    let clone = [
        "clone",
        "--quiet",
        "--no-checkout",
        "--template=",
        "--",
        &text(&cached),
        &text(dest),
    ];
    clean_git(&clone, None).await?;
    clean_git(&["checkout", "--quiet", "--detach", commit], Some(dest)).await?;
    let dot_git = dest.join(".git");
    std::fs::remove_dir_all(&dot_git).map_err(io(&dot_git))
}

/// An item of a package: the name it is placed under, and its folder or file.
pub(super) type Item = (String, PathBuf);

/// The name an entry's one item takes in its section's folders: a subagent
/// is a `.md` file, a skill a folder.
pub(super) fn placed_name(section: Section, name: &str) -> String {
    if section == Section::Agents && !name.ends_with(".md") {
        format!("{name}.md")
    } else {
        name.to_owned()
    }
}

/// The items of entry `name` in the package at `package`, each with the
/// name it is placed under. With `subpath`, the one item there, under
/// [`placed_name`]. Without, every skill folder or subagent file the
/// package provides, under its own name, as its manifest lists them or its
/// `skills` or `agents` folder holds them. Items whose names are not valid
/// entry names, such as a `README.md`, or that another item already has,
/// are left out, each with a warning, returned second.
pub(super) fn items(
    package: &Path,
    section: Section,
    subpath: Option<&str>,
    name: &str,
) -> Result<(Vec<Item>, Vec<String>), ChestError> {
    if let Some(subpath) = subpath {
        let item = package.join(subpath);
        inside(package, &item)?;
        check_item(section, &item)?;
        return Ok((vec![(placed_name(section, name), item)], Vec::new()));
    }
    let key = section.key();
    let meta = package_meta(package)?;
    let listed = meta.resources.get(key);
    let files = if meta.manifest && listed.is_none() {
        Vec::new()
    } else {
        let kind = match section {
            Section::Skills => ResourceType::Skills,
            _ => ResourceType::Prompts,
        };
        selected_files(package, listed.map(Vec::as_slice), key, kind)
    };
    let mut items: Vec<(String, PathBuf)> = files
        .into_iter()
        .filter_map(|file| match section {
            Section::Skills if file.file_name()? == "SKILL.md" => {
                file.parent().map(Path::to_path_buf)
            }
            Section::Skills => None,
            _ => Some(file),
        })
        .filter_map(|item| Some((item.file_name()?.to_string_lossy().into_owned(), item)))
        .collect();
    items.sort();
    items.dedup();
    for (_, item) in &items {
        inside(package, item)?;
    }
    let mut warnings = Vec::new();
    let mut kept: Vec<Item> = Vec::new();
    for (item, path) in items {
        let shown = path
            .strip_prefix(package)
            .unwrap_or(&path)
            .display()
            .to_string();
        if !valid_name(&item) {
            warnings.push(format!("{shown} was skipped: `{item}` is not a valid name"));
        } else if kept.iter().any(|(other, _)| *other == item) {
            warnings.push(format!(
                "{shown} was skipped: another item is named `{item}`"
            ));
        } else {
            kept.push((item, path));
        }
    }
    if kept.is_empty() {
        return Err(invalid(format!("the package provides no {key}")));
    }
    Ok((kept, warnings))
}

/// Fails unless `path` resolves inside the package at `package`, links
/// followed, so that no list entry or committed link reads beyond it.
pub(super) fn inside(package: &Path, path: &Path) -> Result<(), ChestError> {
    let root = package.canonicalize().map_err(io(package))?;
    let real = path
        .canonicalize()
        .map_err(|_| invalid(format!("{} does not exist", path.display())))?;
    if real.starts_with(&root) {
        Ok(())
    } else {
        Err(invalid(format!(
            "{} is outside the package",
            path.display()
        )))
    }
}

/// Fails unless `item` is a skill folder with a `SKILL.md`, or a subagent
/// `.md` file.
pub(super) fn check_item(section: Section, item: &Path) -> Result<(), ChestError> {
    let metadata = item
        .symlink_metadata()
        .map_err(|_| invalid(format!("{} does not exist", item.display())))?;
    let fits = match section {
        Section::Skills => metadata.is_dir() && item.join("SKILL.md").is_file(),
        Section::Agents => metadata.is_file() && item.extension().is_some_and(|ext| ext == "md"),
        _ => true,
    };
    if fits {
        Ok(())
    } else {
        let what = match section {
            Section::Skills => "a skill folder with a SKILL.md",
            _ => "a subagent .md file",
        };
        Err(invalid(format!("{} is not {what}", item.display())))
    }
}
