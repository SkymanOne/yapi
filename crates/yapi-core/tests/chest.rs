//! The chest engine against a local git remote and a mock npm registry:
//! locked commits, package items, frozen checks, startup replays, removal,
//! the `.gitignore` block and path sources.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use aws_lc_rs::digest::{SHA512, digest};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use indexmap::IndexMap;
use serde_json::{Value, json};
use yapi_core::chest::hash::tree_hash;
use yapi_core::chest::lock::Lock;
use yapi_core::chest::{Mode, Outcome, Project, Report};
use yapi_core::packages::npm::{Npm, Pinned};
use yapi_core::settings::Scope;
use yapi_mock::{Cassette, Interaction, MockServer, RequestMatch, Response};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yapi-chest-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("project")).unwrap();
    dir.canonicalize().unwrap()
}

/// Lets `localhost/<path>` git sources, which the manifest accepts, clone
/// the local repository at `/<path>`, for every test in this file, without
/// the user's or the system's git config. It also gives git a line-ending
/// conversion, a smudge filter, and a template whose config asks for CRLF
/// and defines that filter again, none of which may change the bytes a
/// checkout produces.
fn local_git() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // SAFETY: every test here calls this first, and `Once` holds the
        // others back until the variables are set.
        unsafe {
            let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
            std::env::set_var("GIT_CONFIG_GLOBAL", null);
            std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
            std::env::set_var("GIT_CONFIG_COUNT", "3");
            std::env::set_var("GIT_CONFIG_KEY_0", "url.file:///.insteadOf");
            std::env::set_var("GIT_CONFIG_VALUE_0", "https://localhost/");
            std::env::set_var("GIT_CONFIG_KEY_1", "core.autocrlf");
            std::env::set_var("GIT_CONFIG_VALUE_1", "true");
            std::env::set_var("GIT_CONFIG_KEY_2", "filter.upper.smudge");
            std::env::set_var("GIT_CONFIG_VALUE_2", "tr a-z A-Z");
        }
        let template =
            std::env::temp_dir().join(format!("yapi-chest-template-{}", std::process::id()));
        std::fs::create_dir_all(&template).unwrap();
        let config = "[core]\n\teol = crlf\n[filter \"upper\"]\n\tsmudge = tr a-z A-Z\n";
        std::fs::write(template.join("config"), config).unwrap();
        // SAFETY: as above, before any test of this file runs git.
        unsafe { std::env::set_var("GIT_TEMPLATE_DIR", &template) };
    });
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=chest",
            "-c",
            "user.email=chest@example.com",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// A bare remote and a working clone that commits and tags into it.
struct Remote {
    work: PathBuf,
    /// The repository as a manifest's `git` value.
    source: String,
}

impl Remote {
    fn new(dir: &Path) -> Remote {
        local_git();
        let remote = dir.join("remote.git");
        let work = dir.join("work");
        git(
            dir,
            &["init", "--quiet", "--bare", "-b", "main", "remote.git"],
        );
        git(dir, &["init", "--quiet", "-b", "main", "work"]);
        git(
            &work,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        let source = format!("localhost{}", remote.display());
        Remote { work, source }
    }

    /// Commits `files` and points `tag` at the commit, upstream too.
    fn commit(&self, files: &[(&str, &str)], tag: &str) -> String {
        for (path, text) in files {
            let path = self.work.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        git(&self.work, &["add", "-A"]);
        git(&self.work, &["commit", "--quiet", "-m", tag]);
        git(&self.work, &["tag", "-f", tag]);
        git(
            &self.work,
            &["push", "--quiet", "--force", "--tags", "origin", "main"],
        );
        git(&self.work, &["rev-parse", "HEAD"])
    }
}

/// Writes `manifest` as the project's `chest.toml` and opens it, caching
/// git clones in the scratch folder.
fn open(dir: &Path, manifest: &str) -> Project {
    let path = dir.join("project/chest.toml");
    std::fs::write(&path, manifest).unwrap();
    let mut project = Project::open(&path, Scope::Project).unwrap();
    project.cache = dir.join("cache");
    project.state = dir.join("state");
    project
}

async fn failed(project: &mut Project, mode: Mode) -> String {
    let mut npm = Npm::new(HashMap::new());
    let result = project.sync(mode, &mut npm, &mut |_| {}).await;
    result.unwrap_err().to_string()
}

async fn sync(project: &mut Project, mode: Mode) -> Report {
    let mut npm = Npm::new(HashMap::new());
    project.sync(mode, &mut npm, &mut |_| {}).await.unwrap()
}

fn outcomes(report: &Report) -> Vec<(&str, Outcome)> {
    report
        .entries
        .iter()
        .map(|(id, outcome)| (id.as_str(), *outcome))
        .collect()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

async fn frozen(project: &mut Project) -> Result<(), String> {
    let mut npm = Npm::new(HashMap::new());
    let result = project.sync(Mode::Frozen, &mut npm, &mut |_| {}).await;
    result.map(|_| ()).map_err(|err| err.to_string())
}

fn skill_manifest(remote: &Remote, rev: &str) -> String {
    format!(
        "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n\n[skills]\npdf = {{ git = \"{}\", rev = \"{rev}\", path = \"skills/pdf\" }}\n",
        remote.source
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn git_skill_locked_commit() {
    let dir = scratch("locked");
    let remote = Remote::new(&dir);
    let first = remote.commit(
        &[
            ("skills/pdf/SKILL.md", "# pdf 1\n"),
            ("skills/pdf/run.sh", "echo\n"),
            ("skills/pdf/notes.txt", "lower\n"),
            (".gitattributes", "*.txt filter=upper\n*.md text\n"),
        ],
        "v1",
    );
    let root = dir.join("project");
    let canonical = root.join(".agents/skills/pdf");

    // An entry that fails leaves the one fetched before it unplaced.
    let broken = format!(
        "{}bad = {{ git = \"{}\", rev = \"nope\", path = \"skills/pdf\" }}\n",
        skill_manifest(&remote, "v1"),
        remote.source
    );
    let error = failed(&mut open(&dir, &broken), Mode::Resolve).await;
    assert!(error.starts_with("skills.bad: "), "{error}");
    assert!(!canonical.exists() && !root.join("chest.lock").exists());
    assert!(!root.join(".gitignore").exists());

    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Installed)]);
    let locked = project.lock.as_ref().unwrap().skills[0].clone();
    assert_eq!(locked.commit.as_deref(), Some(first.as_str()));
    assert_eq!(locked.hash, Some(tree_hash(&canonical).unwrap()));
    // Neither this machine's config nor a git template changes the bytes.
    assert_eq!(read(&root.join(".claude/skills/pdf/SKILL.md")), "# pdf 1\n");
    assert_eq!(read(&canonical.join("notes.txt")), "lower\n");

    // Without its record, chest adopts a copy that is exactly what it would
    // install, but never one that differs, nor one holding a `.git`, which
    // the hash leaves out.
    std::fs::remove_dir_all(dir.join("state")).unwrap();
    std::fs::remove_file(root.join(".claude/skills/pdf")).unwrap();
    std::fs::write(canonical.join("SKILL.md"), "# mine\n").unwrap();
    let error = failed(
        &mut open(&dir, &skill_manifest(&remote, "v1")),
        Mode::Resolve,
    )
    .await;
    assert!(
        error.contains("refusing to replace what chest did not install"),
        "{error}"
    );
    assert_eq!(read(&canonical.join("SKILL.md")), "# mine\n");
    std::fs::write(canonical.join("SKILL.md"), "# pdf 1\n").unwrap();
    std::fs::create_dir_all(canonical.join(".git")).unwrap();
    let error = failed(
        &mut open(&dir, &skill_manifest(&remote, "v1")),
        Mode::Resolve,
    )
    .await;
    assert!(error.contains("chest did not install"), "{error}");
    std::fs::remove_dir_all(canonical.join(".git")).unwrap();
    let report = sync(
        &mut open(&dir, &skill_manifest(&remote, "v1")),
        Mode::Resolve,
    )
    .await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);

    // A copy that no longer holds what the lock pins, as once a [patch] is
    // dropped, is put back.
    std::fs::create_dir_all(root.join("local/pdf")).unwrap();
    std::fs::write(root.join("local/pdf/SKILL.md"), "# local\n").unwrap();
    let patched = skill_manifest(&remote, "v1") + "\n[patch]\npdf = { path = \"local/pdf\" }\n";
    sync(&mut open(&dir, &patched), Mode::Resolve).await;
    assert_eq!(read(&canonical.join("SKILL.md")), "# local\n");
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 1\n");

    // The tag moves, and every mode but update keeps the locked commit.
    let second = remote.commit(&[("skills/pdf/SKILL.md", "# pdf 2\n")], "v1");
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Unchanged)]);
    let report = sync(&mut project, Mode::Frozen).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Unchanged)]);
    std::fs::remove_dir_all(&canonical).unwrap();
    let report = sync(&mut project, Mode::Startup { offline: true }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Skipped)]);
    let report = sync(&mut project, Mode::Startup { offline: false }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 1\n");
    std::fs::remove_dir_all(&canonical).unwrap();
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 1\n");

    // An installed copy the user edited is not replaced.
    std::fs::write(canonical.join("SKILL.md"), "# edited\n").unwrap();
    let error = failed(&mut project, Mode::Update(Vec::new())).await;
    assert!(
        error.contains("was modified since chest installed it"),
        "{error}"
    );
    assert_eq!(read(&canonical.join("SKILL.md")), "# edited\n");
    std::fs::write(canonical.join("SKILL.md"), "# pdf 1\n").unwrap();
    let before = read(&project.lock_path());
    let report = sync(&mut project, Mode::Update(Vec::new())).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Installed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 2\n");
    let locked = &project.lock.as_ref().unwrap().skills[0];
    assert_eq!(locked.commit.as_deref(), Some(second.as_str()));
    // An update stopped before it wrote the lock leaves the new copy, and
    // install puts back what the lock pins.
    let updated = read(&project.lock_path());
    std::fs::write(project.lock_path(), &before).unwrap();
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 1\n");
    std::fs::write(project.lock_path(), updated).unwrap();
    sync(&mut project, Mode::Resolve).await;
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 2\n");

    // A lock moved to another commit, or to no commit, is refused on replay.
    let lock_path = project.lock_path();
    let pinned = read(&lock_path);
    for (commit, expected) in [
        (
            first.as_str(),
            "skills.pdf: what was fetched does not match the lock",
        ),
        ("HEAD", "skills.pdf: Not a commit: HEAD"),
    ] {
        std::fs::write(&lock_path, pinned.replace(&second, commit)).unwrap();
        let _ = std::fs::remove_dir_all(&canonical);
        let mut moved = open(&dir, &skill_manifest(&remote, "v1"));
        let error = failed(&mut moved, Mode::Startup { offline: false }).await;
        assert!(error.starts_with(expected), "{error}");
        assert!(!canonical.exists());
    }

    // Without `rev`, an entry follows the remote's default branch, also
    // once it is renamed.
    let headless = skill_manifest(&remote, "v1").replace(", rev = \"v1\"", "");
    sync(&mut open(&dir, &headless), Mode::Update(Vec::new())).await;
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 2\n");
    git(&dir.join("remote.git"), &["branch", "-m", "main", "trunk"]);
    git(&remote.work, &["branch", "-m", "main", "trunk"]);
    std::fs::write(remote.work.join("skills/pdf/SKILL.md"), "# pdf 3\n").unwrap();
    git(&remote.work, &["commit", "--quiet", "-am", "v3"]);
    git(&remote.work, &["push", "--quiet", "origin", "trunk"]);
    sync(&mut open(&dir, &headless), Mode::Update(Vec::new())).await;
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 3\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn skills_without_path() {
    let dir = scratch("items");
    let remote = Remote::new(&dir);
    remote.commit(
        &[
            ("skills/pdf/SKILL.md", "# pdf\n"),
            ("skills/xlsx/SKILL.md", "# xlsx\n"),
            ("skills/xlsx/data/table.csv", "a,b\n"),
            ("agents/reviewer.md", "# reviewer\n"),
            ("agents/README.md", "# about\n"),
            ("README.md", "# skills\n"),
        ],
        "v1",
    );
    let manifest = format!(
        "[folders]\nskills = [\".agents/skills\"]\nagents = [\".yapi/agents\"]\n\n[skills]\nanthropic = {{ git = \"{0}\", rev = \"v1\" }}\n\n[agents]\nteam = {{ git = \"{0}\", rev = \"v1\" }}\n",
        remote.source
    );
    let mut project = open(&dir, &manifest);
    let report = sync(&mut project, Mode::Resolve).await;
    // An item whose name is not an entry name is skipped, not the package.
    assert_eq!(
        report.warnings,
        ["agents.team: agents/README.md was skipped: `README.md` is not a valid name"]
    );
    let root = dir.join("project");
    let lock = project.lock.clone().unwrap();
    let items: Vec<(&str, &str)> = lock.skills[0]
        .items
        .iter()
        .map(|item| (item.name.as_str(), item.hash.as_str()))
        .collect();
    let pdf = tree_hash(&root.join(".agents/skills/pdf")).unwrap();
    let xlsx = tree_hash(&root.join(".agents/skills/xlsx")).unwrap();
    assert_eq!(items, [("pdf", pdf.as_str()), ("xlsx", xlsx.as_str())]);
    assert_eq!(
        read(&root.join(".agents/skills/xlsx/data/table.csv")),
        "a,b\n"
    );
    let reviewer = root.join(".yapi/agents/reviewer.md");
    assert_eq!(read(&reviewer), "# reviewer\n");
    assert_eq!(lock.agents[0].items[0].hash, tree_hash(&reviewer).unwrap());
    assert!(lock.skills[0].hash.is_none());

    // A skill holding a symbolic link is refused, naming the link.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("SKILL.md", remote.work.join("skills/pdf/link")).unwrap();
        remote.commit(&[], "v2");
        let mut project = open(&dir, &manifest.replace("\"v1\"", "\"v2\""));
        let mut npm = Npm::new(HashMap::new());
        let error = project
            .sync(Mode::Resolve, &mut npm, &mut |_| {})
            .await
            .unwrap_err();
        let error = error.to_string();
        assert!(error.starts_with("skills.anthropic: "), "{error}");
        assert!(
            error.ends_with("skills/pdf/link: symbolic links are not allowed in a package"),
            "{error}"
        );
        assert_eq!(read(&root.join(".agents/skills/pdf/SKILL.md")), "# pdf\n");

        // A list entry, or a committed link, that leaves the package is refused.
        let outside = dir.join("outside");
        std::fs::create_dir_all(outside.join("skills/x")).unwrap();
        std::fs::write(outside.join("skills/x/SKILL.md"), "# private\n").unwrap();
        let list = json!({"pi": {"skills": [outside.join("skills").to_str().unwrap()]}});
        remote.commit(&[("package.json", &list.to_string())], "v3");
        let escaping = manifest.replace("\"v1\"", "\"v3\"");
        let error = failed(&mut open(&dir, &escaping), Mode::Resolve).await;
        assert!(error.ends_with("is outside the package"), "{error}");
        std::os::unix::fs::symlink(&outside, remote.work.join("h")).unwrap();
        remote.commit(&[], "v4");
        let linked = format!(
            "[folders]\nskills = [\".agents/skills\"]\n\n[skills]\npick = {{ git = \"{}\", rev = \"v4\", path = \"h/skills/x\" }}\n",
            remote.source
        );
        let error = failed(&mut open(&dir, &linked), Mode::Resolve).await;
        assert!(
            error.ends_with("h/skills/x is outside the package"),
            "{error}"
        );
        assert!(!root.join(".agents/skills/x").exists());
        assert!(!root.join(".agents/skills/pick").exists());
    }

    // A new entry never takes the place of a kept entry's item.
    let mut project = open(&dir, &manifest);
    sync(&mut project, Mode::Resolve).await;
    std::fs::create_dir_all(root.join("local-pdf")).unwrap();
    std::fs::write(root.join("local-pdf/SKILL.md"), "# mine\n").unwrap();
    let clash = manifest.replace("[agents]", "pdf = { path = \"local-pdf\" }\n\n[agents]");
    let error = failed(&mut open(&dir, &clash), Mode::Resolve).await;
    assert!(error.contains("another entry also installs"), "{error}");
    assert_eq!(read(&root.join(".agents/skills/pdf/SKILL.md")), "# pdf\n");

    // A git extension installs a folder, never one file of the package.
    let single = format!(
        "[extensions]\nsolo = {{ git = \"{}\", rev = \"v1\", path = \"agents/reviewer.md\" }}\n",
        remote.source
    );
    let error = failed(&mut open(&dir, &single), Mode::Resolve).await;
    assert!(
        error.contains("`path` must name the folder that holds the extension, not a file"),
        "{error}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn frozen_rejects_drift() {
    let dir = scratch("frozen");
    let remote = Remote::new(&dir);
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf\n")], "v1");
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf 2\n")], "v2");
    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    sync(&mut project, Mode::Resolve).await;
    let lock_path = project.lock_path();
    let locked = read(&lock_path);

    let mut drifted = open(&dir, &skill_manifest(&remote, "v2"));
    let error = frozen(&mut drifted).await.unwrap_err();
    assert_eq!(
        error,
        "The lock of chest.toml is out of date for skills.pdf: run `chest install`"
    );
    assert_eq!(read(&lock_path), locked);

    // A fresh clone gets exactly what the lock pins, which stays as it is.
    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    let root = dir.join("project");
    std::fs::remove_dir_all(root.join(".agents")).unwrap();
    std::fs::remove_dir_all(root.join(".claude")).unwrap();
    std::fs::remove_file(root.join(".gitignore")).unwrap();
    std::fs::remove_dir_all(dir.join("state")).unwrap();
    let report = sync(&mut project, Mode::Frozen).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&root.join(".claude/skills/pdf/SKILL.md")), "# pdf\n");
    assert!(read(&root.join(".gitignore")).contains("\n.agents/skills/pdf\n"));
    assert_eq!(read(&lock_path), locked);
    // With everything in place, frozen writes nothing at all, so a
    // read-only project passes.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let root = dir.join("project");
        let mode = |bits| std::fs::Permissions::from_mode(bits);
        std::fs::set_permissions(&root, mode(0o555)).unwrap();
        let result = frozen(&mut project).await;
        std::fs::set_permissions(&root, mode(0o755)).unwrap();
        assert!(result.is_ok(), "{result:?}");

        // A link that points elsewhere fails, as a tampered copy does.
        let link = dir.join("project/.claude/skills/pdf");
        std::fs::remove_file(&link).unwrap();
        std::fs::create_dir_all(dir.join("elsewhere")).unwrap();
        std::os::unix::fs::symlink(dir.join("elsewhere"), &link).unwrap();
        let error = frozen(&mut project).await.unwrap_err();
        assert!(
            error.contains(".claude/skills/pdf does not link to"),
            "{error}"
        );
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("../../.agents/skills/pdf", &link).unwrap();
        assert!(frozen(&mut project).await.is_ok());
    }
    let skill = dir.join("project/.agents/skills/pdf/SKILL.md");
    std::fs::write(&skill, "# tampered\n").unwrap();
    let error = frozen(&mut project).await.unwrap_err();
    assert!(
        error.ends_with(".agents/skills/pdf does not match the lock"),
        "{error}"
    );
    assert_eq!(read(&skill), "# tampered\n");
    assert_eq!(read(&lock_path), locked);

    std::fs::remove_file(&lock_path).unwrap();
    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    let error = frozen(&mut project).await.unwrap_err();
    assert_eq!(error, "chest.toml has no lock: run `chest install`");
    assert!(!lock_path.exists());

    // A tag deleted upstream no longer resolves from the cached clone.
    git(
        &remote.work,
        &["push", "--quiet", "origin", ":refs/tags/v2"],
    );
    let mut gone = open(&dir, &skill_manifest(&remote, "v2"));
    let error = failed(&mut gone, Mode::Update(Vec::new())).await;
    assert!(error.ends_with("has no revision v2"), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn gitignore_block_managed() {
    let dir = scratch("ignore");
    let remote = Remote::new(&dir);
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf\n")], "v1");
    let root = dir.join("project");
    std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
    // What a stopped install left goes: a staging folder, a previous copy,
    // and the work folder of a process that has ended.
    let staging = root.join(".agents/skills/.gone-staging");
    std::fs::create_dir_all(&staging).unwrap();
    let previous = root.join(".agents/skills/.gone-old");
    std::fs::create_dir_all(&previous).unwrap();
    let mut ended = Command::new("true").spawn().unwrap();
    ended.wait().unwrap();
    let work = dir.join(format!("cache/work/{}-0", ended.id()));
    std::fs::create_dir_all(&work).unwrap();
    // But not from a folder that links out of the project, here that of a
    // section without entries.
    let outside = dir.join("outside");
    std::fs::create_dir_all(outside.join(".x-staging")).unwrap();
    std::fs::create_dir_all(root.join(".claude")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join(".claude/agents")).unwrap();
    let agents = "[folders]\nagents = [\".claude/agents\"]\n";
    let manifest = skill_manifest(&remote, "v1").replace("[folders]\n", agents);
    let mut project = open(&dir, &manifest);
    sync(&mut project, Mode::Resolve).await;
    assert!(!staging.exists() && !previous.exists() && !work.exists());
    assert!(outside.join(".x-staging").exists());
    assert_eq!(
        read(&root.join(".gitignore")),
        "target/\n# BEGIN chest (chest.toml)\n.agents/skills/.*-old\n.agents/skills/.*-staging\n.agents/skills/pdf\n.claude/skills/pdf\n# END chest\n"
    );
    // A lock that is a link is not written through.
    #[cfg(unix)]
    {
        let lock = root.join("chest.lock");
        let moved = outside.join("chest.lock");
        std::fs::rename(&lock, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &lock).unwrap();
        let locked = read(&moved);
        let empty = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n";
        let error = failed(&mut open(&dir, empty), Mode::Resolve).await;
        assert!(error.ends_with("chest.lock through a link"), "{error}");
        assert_eq!(read(&moved), locked);
        assert!(root.join(".agents/skills/pdf").exists());
        std::fs::remove_file(&lock).unwrap();
        std::fs::rename(&moved, &lock).unwrap();
    }
    let mut project = open(
        &dir,
        "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n",
    );
    sync(&mut project, Mode::Resolve).await;
    assert_eq!(read(&root.join(".gitignore")), "target/\n");

    // `.claude/skills` linking to `.agents/skills`, which does not exist
    // yet: the copy is placed once, and a second install keeps it.
    #[cfg(unix)]
    {
        std::fs::remove_dir_all(root.join(".agents")).unwrap();
        std::fs::remove_dir_all(root.join(".claude")).unwrap();
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        std::os::unix::fs::symlink("../.agents/skills", root.join(".claude/skills")).unwrap();
        for outcome in [Outcome::Installed, Outcome::Unchanged] {
            let report = sync(
                &mut open(&dir, &skill_manifest(&remote, "v1")),
                Mode::Resolve,
            )
            .await;
            assert_eq!(outcomes(&report), [("skills.pdf", outcome)]);
            assert_eq!(read(&root.join(".agents/skills/pdf/SKILL.md")), "# pdf\n");
        }
        assert_eq!(
            read(&root.join(".gitignore")),
            "target/\n# BEGIN chest (chest.toml)\n.agents/skills/.*-old\n.agents/skills/.*-staging\n.agents/skills/pdf\n# END chest\n"
        );
    }
}

/// An npm tarball with `files` under `package/`.
fn tarball(files: &[(&str, &str)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, text) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(text.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("package/{name}"), text.as_bytes())
            .unwrap();
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&builder.into_inner().unwrap()).unwrap();
    gz.finish().unwrap()
}

fn get(path: &str, response: Response) -> Interaction {
    Interaction {
        request: RequestMatch {
            method: "GET".into(),
            path: path.into(),
        },
        response,
    }
}

fn body(text: String, base64: Option<String>) -> Response {
    Response {
        status: 200,
        headers: IndexMap::new(),
        chunks: base64.is_none().then_some(text).into_iter().collect(),
        body_base64: base64,
        chunk_delay_ms: 0,
        close: None,
    }
}

/// The packument and the tarball of `name@1.0.0` with `dependencies`, and
/// `files` beside its `package.json` and `index.js`.
fn publish(
    base: &str,
    name: &str,
    dependencies: Value,
    files: &[(&str, &str)],
) -> (Interaction, Interaction) {
    let package = json!({"name": name, "version": "1.0.0"}).to_string();
    let index = format!("module.exports = '{name}';\n");
    let mut entries = vec![("package.json", package.as_str()), ("index.js", &index)];
    entries.extend_from_slice(files);
    let bytes = tarball(&entries);
    let manifest = json!({
        "name": name,
        "version": "1.0.0",
        "dependencies": dependencies,
        "dist": {
            "tarball": format!("{base}/{name}/-/{name}-1.0.0.tgz"),
            "integrity": format!("sha512-{}", STANDARD.encode(digest(&SHA512, &bytes))),
        },
    });
    let packument =
        json!({"name": name, "dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": manifest}});
    (
        get(&format!("/{name}"), body(packument.to_string(), None)),
        get(
            &format!("/{name}/-/{name}-1.0.0.tgz"),
            body(String::new(), Some(STANDARD.encode(bytes))),
        ),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn npm_extension_locked() {
    let dir = scratch("npm");
    let skill = [("skills/guide/SKILL.md", "# guide\n")];
    let server = MockServer::local_with(|base| {
        let (guide, guide_tarball) = publish(base, "guide", json!({}), &skill);
        let (ext, ext_tarball) = publish(base, "ext", json!({"dep": "^1"}), &[]);
        let (dep, dep_tarball) = publish(base, "dep", json!({}), &[]);
        Cassette {
            interactions: vec![guide, guide_tarball, ext, ext_tarball, dep, dep_tarball],
        }
    })
    .await
    .unwrap();
    let manifest =
        "[extensions]\next = { npm = \"ext\", version = \"^1\", capabilities = [\"fs\"] }\n";
    // A store under a folder that links out is refused before anything is staged.
    #[cfg(unix)]
    {
        let outside = dir.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("project/.yapi")).unwrap();
        let error = failed(&mut open(&dir, manifest), Mode::Resolve).await;
        assert!(error.contains("is outside"), "{error}");
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
        std::fs::remove_file(dir.join("project/.yapi")).unwrap();
    }
    // With an npm skill too, which places one folder of its package.
    let guide_manifest = "[folders]\nskills = [\".agents/skills\"]\n\n[skills]\nguide = { npm = \"guide\", version = \"^1\", path = \"skills/guide\" }\n";
    let manifest = &format!("{guide_manifest}\n{manifest}");
    let mut project = open(&dir, manifest);
    let mut npm = Npm::new([("registry".to_owned(), format!("{}/", server.url()))].into());
    let report = project
        .sync(Mode::Resolve, &mut npm, &mut |_| {})
        .await
        .unwrap();
    assert_eq!(
        outcomes(&report),
        [
            ("skills.guide", Outcome::Installed),
            ("extensions.ext", Outcome::Installed)
        ]
    );
    server.finish().unwrap();
    let guide = dir.join("project/.agents/skills/guide");
    assert_eq!(read(&guide.join("SKILL.md")), "# guide\n");
    let store = dir.join("project/.yapi/chest/ext");
    let installed = tree_hash(&store).unwrap();
    let locked = project.lock.as_ref().unwrap().extensions[0].clone();
    assert_eq!(locked.version.as_deref(), Some("1.0.0"));
    assert_eq!(locked.hash.as_deref(), Some(installed.as_str()));
    let paths: Vec<&str> = locked
        .packages
        .iter()
        .map(|pin| pin.path.as_str())
        .collect();
    assert_eq!(paths, ["node_modules/dep", "node_modules/ext"]);
    assert!(store.join("node_modules/ext/index.js").exists());
    assert!(!store.join("package.json").exists());

    // A mirror that serves only the tarballs rebuilds the same store. The
    // replay downloads them at once, so they may come in either order.
    let mirror = MockServer::local_any_order(|base| {
        let (_, dep) = publish(base, "dep", json!({}), &[]);
        let (_, ext) = publish(base, "ext", json!({"dep": "^1"}), &[]);
        let (_, guide) = publish(base, "guide", json!({}), &skill);
        Cassette {
            interactions: vec![dep, ext, guide],
        }
    })
    .await
    .unwrap();
    let lock_path = project.lock_path();
    // The tree of a lock that pins dep in ext's place.
    let swapped_tree = dir.join("swapped");
    for folder in ["dep", "ext"] {
        let to = swapped_tree.join("node_modules").join(folder);
        std::fs::create_dir_all(&to).unwrap();
        for file in ["package.json", "index.js"] {
            let from = store.join("node_modules/dep").join(file);
            std::fs::copy(from, to.join(file)).unwrap();
        }
    }
    std::fs::remove_dir_all(&store).unwrap();
    // A lock that points the package at another's tarball is refused.
    let npmjs = read(&lock_path).replace(&server.url(), "https://registry.npmjs.org");
    // So is one that slips a scope in front of the package's own path.
    for (from, to) in [
        ("/ext/-/ext-1.0.0.tgz", "/dep/-/dep-1.0.0.tgz"),
        ("/ext/-/ext-1.0.0.tgz", "/@evil/ext/-/ext-1.0.0.tgz"),
    ] {
        std::fs::write(&lock_path, npmjs.replace(from, to)).unwrap();
        let mut npm = Npm::new([("registry".to_owned(), format!("{}/", mirror.url()))].into());
        let error = open(&dir, manifest)
            .sync(Mode::Startup { offline: false }, &mut npm, &mut |_| {})
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("a pinned tarball must be at /ext/-/ext-1.0.0.tgz"),
            "{error}"
        );
    }
    // So is one that keeps ext@1.0.0 but pins dep, with the hash of its
    // tree, in ext's folder, by install and frozen install alike.
    let mut swapped = Lock::parse(&npmjs).unwrap();
    let ext = &mut swapped.extensions[0];
    ext.packages[1] = Pinned {
        path: "node_modules/ext".into(),
        ..ext.packages[0].clone()
    };
    ext.hash = Some(tree_hash(&swapped_tree).unwrap());
    std::fs::write(&lock_path, swapped.to_toml().unwrap()).unwrap();
    for mode in [Mode::Resolve, Mode::Frozen] {
        let mut npm = Npm::new([("registry".to_owned(), format!("{}/", mirror.url()))].into());
        let error = open(&dir, manifest)
            .sync(mode, &mut npm, &mut |_| {})
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("the lock does not pin ext@1.0.0 at node_modules/ext"),
            "{error}"
        );
        assert!(!store.exists());
    }
    // Tarballs pinned on registry.npmjs.org come from the configured mirror,
    // the skill's included.
    std::fs::write(&lock_path, npmjs).unwrap();
    std::fs::remove_dir_all(&guide).unwrap();
    let mut project = open(&dir, manifest);
    let mut npm = Npm::new([("registry".to_owned(), format!("{}/", mirror.url()))].into());
    let report = project
        .sync(Mode::Startup { offline: false }, &mut npm, &mut |_| {})
        .await
        .unwrap();
    assert_eq!(
        outcomes(&report),
        [
            ("skills.guide", Outcome::Replayed),
            ("extensions.ext", Outcome::Replayed)
        ]
    );
    mirror.finish().unwrap();
    assert_eq!(tree_hash(&store).unwrap(), installed);
    assert_eq!(read(&guide.join("SKILL.md")), "# guide\n");

    // A registry that keeps tarballs elsewhere fails the first install of a
    // skill or an extension, rather than every replay of its lock.
    let ext_manifest = "[extensions]\next = { npm = \"ext\", version = \"^1\" }\n";
    for (name, files, manifest) in [
        ("guide", &skill[..], guide_manifest),
        ("ext", &[][..], ext_manifest),
    ] {
        let dir = scratch(&format!("npm-elsewhere-{name}"));
        let server = MockServer::local_with(|base| {
            let (mut packument, mut tarball) = publish(base, name, json!({}), files);
            let (pinned, elsewhere) = (
                format!("/{name}/-/{name}-1.0.0.tgz"),
                format!("/files/{name}-1.0.0.tgz"),
            );
            packument.response.chunks[0] =
                packument.response.chunks[0].replace(&pinned, &elsewhere);
            tarball.request.path = elsewhere;
            Cassette {
                interactions: vec![packument, tarball],
            }
        })
        .await
        .unwrap();
        let mut npm = Npm::new([("registry".to_owned(), format!("{}/", server.url()))].into());
        let error = open(&dir, manifest)
            .sync(Mode::Resolve, &mut npm, &mut |_| {})
            .await
            .unwrap_err()
            .to_string();
        let expected = format!("a pinned tarball must be at /{name}/-/{name}-1.0.0.tgz");
        assert!(error.contains(&expected), "{error}");
        assert!(!dir.join("project/chest.lock").exists());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn startup_skips_stale() {
    let dir = scratch("stale");
    let remote = Remote::new(&dir);
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf\n")], "v1");
    let root = dir.join("project");
    std::fs::create_dir_all(root.join("notes")).unwrap();
    std::fs::write(root.join("notes/SKILL.md"), "# notes\n").unwrap();
    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    sync(&mut project, Mode::Resolve).await;
    let lock_path = project.lock_path();
    let (locked, ignored) = (read(&lock_path), read(&root.join(".gitignore")));

    // An unlocked entry and a stale one are skipped, with one warning.
    let manifest = skill_manifest(&remote, "v2") + "notes = { path = \"notes\" }\n";
    let mut project = open(&dir, &manifest);
    let report = sync(&mut project, Mode::Startup { offline: false }).await;
    assert_eq!(
        outcomes(&report),
        [
            ("skills.pdf", Outcome::Skipped),
            ("skills.notes", Outcome::Skipped)
        ]
    );
    assert_eq!(report.warnings.len(), 1);
    assert!(
        report.warnings[0].ends_with("run `chest install`"),
        "{report:?}"
    );
    assert!(!root.join(".agents/skills/notes").exists());
    assert_eq!(
        (read(&lock_path), read(&root.join(".gitignore"))),
        (locked, ignored)
    );

    // Without a lock, startup installs nothing.
    std::fs::remove_file(&lock_path).unwrap();
    std::fs::remove_dir_all(root.join(".agents/skills/pdf")).unwrap();
    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    let report = sync(&mut project, Mode::Startup { offline: false }).await;
    assert!(report.entries.is_empty());
    assert_eq!(
        report.warnings,
        ["chest.toml has no lock, so nothing from it was installed: run `chest install`"]
    );
    assert!(!root.join(".agents/skills/pdf").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn removed_entry_unplaced() {
    let dir = scratch("removed");
    let remote = Remote::new(&dir);
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf\n")], "v1");
    let root = dir.join("project");
    std::fs::create_dir_all(root.join("notes")).unwrap();
    std::fs::write(root.join("notes/SKILL.md"), "# notes\n").unwrap();
    std::fs::create_dir_all(root.join(".agents/skills/mine")).unwrap();
    std::fs::write(root.join(".agents/skills/mine/SKILL.md"), "# mine\n").unwrap();
    let both = skill_manifest(&remote, "v1") + "notes = { path = \"notes\" }\n";
    let mut project = open(&dir, &both);
    sync(&mut project, Mode::Resolve).await;
    assert!(root.join(".claude/skills/pdf").symlink_metadata().is_ok());
    // Dropping the first folder, or swapping the two, puts the copies in the
    // new first folder, and nothing is lost.
    for (folders, first) in [
        ("[\".claude/skills\"]", ".claude/skills"),
        ("[\".claude/skills\", \".agents/skills\"]", ".claude/skills"),
        ("[\".agents/skills\", \".claude/skills\"]", ".agents/skills"),
    ] {
        let moved = both.replace("[\".agents/skills\", \".claude/skills\"]", folders);
        let mut project = open(&dir, &moved);
        sync(&mut project, Mode::Resolve).await;
        assert_eq!(project.verify().unwrap(), Vec::<String>::new());
        for name in ["pdf", "notes"] {
            let copy = root.join(first).join(name).symlink_metadata().unwrap();
            assert!(copy.is_dir(), "{folders} {name}");
        }
        assert_eq!(read(&root.join(".claude/skills/pdf/SKILL.md")), "# pdf\n");
    }

    let manifest = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n\n[skills]\nnotes = { path = \"notes\" }\n";
    let mut project = open(&dir, manifest);
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(
        outcomes(&report),
        [
            ("skills.notes", Outcome::Installed),
            ("skills.pdf", Outcome::Removed)
        ]
    );
    for folder in [".agents/skills", ".claude/skills"] {
        assert!(root.join(folder).join("pdf").symlink_metadata().is_err());
        assert_eq!(read(&root.join(folder).join("notes/SKILL.md")), "# notes\n");
    }
    assert_eq!(read(&root.join(".agents/skills/mine/SKILL.md")), "# mine\n");
    let names: Vec<String> = project
        .lock
        .unwrap()
        .skills
        .into_iter()
        .map(|locked| locked.name)
        .collect();
    assert_eq!(names, ["notes"]);

    // Another manifest of the project shares the folders and their record:
    // it places what chest.toml placed, and what it drops stays while
    // chest.toml still lists it.
    let dev = root.join("chest.dev.toml");
    let open_dev = |text: &str| {
        std::fs::write(&dev, text).unwrap();
        let mut project = Project::open(&dev, Scope::Project).unwrap();
        project.cache = dir.join("cache");
        project.state = dir.join("state");
        project
    };
    let report = sync(&mut open_dev(manifest), Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.notes", Outcome::Installed)]);
    let empty = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n";
    sync(&mut open_dev(empty), Mode::Resolve).await;
    assert_eq!(
        read(&root.join(".claude/skills/notes/SKILL.md")),
        "# notes\n"
    );
    sync(&mut open(&dir, empty), Mode::Resolve).await;
    assert!(
        root.join(".agents/skills/notes")
            .symlink_metadata()
            .is_err()
    );
    sync(&mut open(&dir, manifest), Mode::Resolve).await;

    // A committed lock naming a hand-written skill does not get it removed.
    let lock_path = root.join("chest.lock");
    let zeros = "0".repeat(40);
    let mine = tree_hash(&root.join(".agents/skills/mine")).unwrap();
    let attack = format!(
        "{}\n[[skill]]\nname = \"mine\"\ngit = \"https://example.com/a/b\"\ncommit = \"{zeros}\"\nhash = \"{mine}\"\n",
        read(&lock_path)
    );
    std::fs::write(&lock_path, attack).unwrap();
    let report = sync(&mut open(&dir, manifest), Mode::Resolve).await;
    assert!(
        report
            .entries
            .contains(&("skills.mine".into(), Outcome::Removed))
    );
    assert_eq!(read(&root.join(".agents/skills/mine/SKILL.md")), "# mine\n");

    // A folder dropped from [folders] loses what chest placed there.
    let narrowed = manifest.replace(", \".claude/skills\"", "");
    sync(&mut open(&dir, &narrowed), Mode::Resolve).await;
    assert!(
        root.join(".claude/skills/notes")
            .symlink_metadata()
            .is_err()
    );
    assert_eq!(
        read(&root.join(".agents/skills/notes/SKILL.md")),
        "# notes\n"
    );

    // A project opened through a link places, records and keeps the same paths.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&root, dir.join("link")).unwrap();
        let path = dir.join("link/chest.toml");
        std::fs::write(&path, manifest).unwrap();
        let mut linked = Project::open(&path, Scope::Project).unwrap();
        linked.cache = dir.join("cache");
        linked.state = dir.join("state");
        sync(&mut linked, Mode::Resolve).await;
        for folder in [".agents/skills", ".claude/skills"] {
            assert_eq!(read(&root.join(folder).join("notes/SKILL.md")), "# notes\n");
        }
        sync(&mut open(&dir, manifest), Mode::Resolve).await;
        assert_eq!(
            read(&root.join(".claude/skills/notes/SKILL.md")),
            "# notes\n"
        );
    }

    // A hand-written skill is never replaced.
    let clash = format!("{manifest}mine = {{ path = \"notes\" }}\n");
    let mut project = open(&dir, &clash);
    let mut npm = Npm::new(HashMap::new());
    let error = project
        .sync(Mode::Resolve, &mut npm, &mut |_| {})
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("chest did not install"),
        "{error}"
    );
    assert_eq!(read(&root.join(".agents/skills/mine/SKILL.md")), "# mine\n");

    // A link the user points at their own skill is theirs: not replaced,
    // and not removed, with a warning.
    #[cfg(unix)]
    {
        let link = root.join(".claude/skills/notes");
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("../../.agents/skills/mine", &link).unwrap();
        let error = failed(&mut open(&dir, manifest), Mode::Resolve).await;
        assert!(
            error.contains(".claude/skills/notes was modified since chest installed it"),
            "{error}"
        );
        let empty = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n";
        let report = sync(&mut open(&dir, empty), Mode::Resolve).await;
        assert!(
            report.warnings.iter().any(|warning| warning.contains(
                ".claude/skills/notes was modified since chest installed it, so it stays"
            )),
            "{report:?}"
        );
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("../../.agents/skills/mine")
        );
        assert!(
            root.join(".agents/skills/notes")
                .symlink_metadata()
                .is_err()
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn path_source_notice() {
    let dir = scratch("paths");
    let folders = "[folders]\nskills = [\".agents/skills\"]\n\n[skills]\n";
    let path = dir.join("project/chest.toml");
    std::fs::write(
        &path,
        format!("{folders}notes = {{ path = \"/abs/notes\" }}\n"),
    )
    .unwrap();
    let error = Project::open(&path, Scope::Project)
        .unwrap_err()
        .to_string();
    assert!(error.ends_with("skills.notes: `path`: \"/abs/notes\" must be relative to the manifest, not absolute or under `~`"), "{error}");

    std::fs::create_dir_all(dir.join("outside/notes")).unwrap();
    std::fs::write(dir.join("outside/notes/SKILL.md"), "# notes\n").unwrap();
    let mut project = open(
        &dir,
        &format!("{folders}notes = {{ path = \"../outside/notes\" }}\n"),
    );
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(
        report.notices,
        [format!(
            "skills.notes: `../outside/notes` is outside the project, at {}",
            dir.join("outside/notes").display()
        )]
    );
    assert_eq!(
        read(&dir.join("project/.agents/skills/notes/SKILL.md")),
        "# notes\n"
    );

    // A path source holding the folder chest installs to, as an entry or as
    // a patch, would copy itself, and is refused before anything is copied.
    std::fs::write(dir.join("project/SKILL.md"), "# self\n").unwrap();
    let pdf = "pdf = { git = \"localhost/nowhere/repo\", rev = \"v1\", path = \"skills/pdf\" }";
    for manifest in [
        format!("{folders}me = {{ path = \".\" }}\n"),
        format!("{folders}{pdf}\n\n[patch]\npdf = {{ path = \".\" }}\n"),
    ] {
        let error = failed(&mut open(&dir, &manifest), Mode::Resolve).await;
        let holds = format!(
            "{} holds {}, where chest installs",
            dir.join("project").display(),
            dir.join("project/.agents/skills").display()
        );
        assert!(error.contains(&holds), "{error}");
        for name in ["me", ".me-staging", "pdf", ".pdf-staging"] {
            let placed = dir.join("project/.agents/skills").join(name);
            assert!(placed.symlink_metadata().is_err(), "{name}");
        }
    }
    std::fs::remove_file(dir.join("project/SKILL.md")).unwrap();

    // An extension whose folder holds the project, as a path source or as a
    // patch, is refused before anything is written.
    let lock = read(&dir.join("project/chest.lock"));
    let ext = "ext = { git = \"localhost/nowhere/repo\", rev = \"v1\" }";
    for (manifest, id, folder) in [
        (
            "[extensions]\nwide = { path = \"x/..\" }\n".to_owned(),
            "extensions.wide",
            dir.join("project"),
        ),
        (
            format!("[extensions]\n{ext}\n\n[patch]\next = {{ path = \"x/../..\" }}\n"),
            "patch.ext",
            dir.clone(),
        ),
    ] {
        let error = failed(&mut open(&dir, &manifest), Mode::Resolve).await;
        let refused = format!("{id}: {} holds more than the extension", folder.display());
        assert!(error.contains(&refused), "{error}");
        assert_eq!(read(&dir.join("project/chest.lock")), lock);
    }

    // A patched entry installs from its patch, again and again, and is
    // ignored by git, while the lock stays as it was.
    std::fs::create_dir_all(dir.join("project/local/pdf")).unwrap();
    std::fs::write(dir.join("project/local/pdf/SKILL.md"), "# local\n").unwrap();
    let patched = format!(
        "{folders}pdf = {{ git = \"localhost/nowhere/repo\", rev = \"v1\", path = \"skills/pdf\" }}\n\n[patch]\npdf = {{ path = \"local/pdf\" }}\n"
    );
    for _ in 0..2 {
        let report = sync(&mut open(&dir, &patched), Mode::Resolve).await;
        assert_eq!(outcomes(&report)[0], ("skills.pdf", Outcome::Installed));
    }
    assert_eq!(
        read(&dir.join("project/.agents/skills/pdf/SKILL.md")),
        "# local\n"
    );
    assert!(read(&dir.join("project/.gitignore")).contains("\n.agents/skills/pdf\n"));
    assert!(!read(&dir.join("project/chest.lock")).contains("pdf"));

    // A patch for an entry that takes a package's items installs the items
    // its folder provides, as the package would.
    for skill in ["one", "two"] {
        let folder = dir.join(format!("project/local/pack/skills/{skill}"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("SKILL.md"), format!("# {skill}\n")).unwrap();
    }
    let package = format!(
        "{folders}team = {{ git = \"localhost/nowhere/repo\", rev = \"v1\" }}\n\n[patch]\nteam = {{ path = \"local/pack\" }}\n"
    );
    sync(&mut open(&dir, &package), Mode::Resolve).await;
    for skill in ["one", "two"] {
        let placed = dir.join(format!("project/.agents/skills/{skill}/SKILL.md"));
        assert_eq!(read(&placed), format!("# {skill}\n"));
    }
}
