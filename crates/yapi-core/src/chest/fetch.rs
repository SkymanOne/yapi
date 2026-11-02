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
    /// process.
    pub(super) fn new(cache: &Path) -> Result<Work, ChestError> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = cache
            .join("work")
            .join(format!("{}-{id}", std::process::id()));
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

/// The bare clone of `repo` in `cache`, made on first use and fetched again
/// when `refresh`. Only clones and fetches go to the network.
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
    } else if refresh {
        let heads = "+refs/heads/*:refs/heads/*";
        let fetch = [
            "fetch", "--quiet", "--prune", "--force", "--tags", "--", &url, heads,
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

/// The commit that `rev` of `repo` names now, the default branch's without
/// one. The manifest refuses a `rev` that git could read as an option.
pub(super) async fn resolve(
    cache: &Path,
    repo: &str,
    rev: Option<&str>,
) -> Result<String, ChestError> {
    let mirror = mirror(cache, repo, true).await?;
    let rev = rev.unwrap_or("HEAD");
    commit_of(&mirror, rev)
        .await
        .ok_or_else(|| invalid(format!("{repo} has no revision {rev}")))
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
/// `skills` or `agents` folder holds them.
pub(super) fn items(
    package: &Path,
    section: Section,
    subpath: Option<&str>,
    name: &str,
) -> Result<Vec<(String, PathBuf)>, ChestError> {
    if let Some(subpath) = subpath {
        let item = package.join(subpath);
        inside(package, &item)?;
        check_item(section, &item)?;
        return Ok(vec![(placed_name(section, name), item)]);
    }
    let key = section.key();
    let meta = package_meta(package);
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
    if items.is_empty() {
        return Err(invalid(format!("the package provides no {key}")));
    }
    for (_, item) in &items {
        inside(package, item)?;
    }
    if let Some((item, _)) = items.iter().find(|(item, _)| !valid_name(item)) {
        return Err(invalid(format!("the item `{item}` has an invalid name")));
    }
    if let Some(pair) = items.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(invalid(format!("two items are named `{}`", pair[0].0)));
    }
    Ok(items)
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
