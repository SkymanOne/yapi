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
use yapi_ai::auth::lock::FileLock;
use yapi_core::chest::hash::tree_hash;
use yapi_core::chest::lock::Lock;
use yapi_core::chest::manifest::{Entry, Section};
use yapi_core::chest::startup::{file_policies, installed_inputs};
use yapi_core::chest::{Mode, Outcome, Project, Report};
use yapi_core::packages::npm::{Npm, Pinned};
use yapi_core::packages::resolve::ResourceType;
use yapi_core::packages::resolve_resources;
use yapi_core::settings::{Scope, SettingsManager};
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
    let mut patch = open(&dir, &patched);
    sync(&mut patch, Mode::Resolve).await;
    assert_eq!(read(&canonical.join("SKILL.md")), "# local\n");
    // An edited copy of a patched entry stays, except in a frozen install,
    // and list and verify report it.
    std::fs::write(canonical.join("SKILL.md"), "# local edit\n").unwrap();
    let edit = format!(
        "skills.pdf: {} was modified since chest installed it",
        canonical.display()
    );
    let how = "delete it and run `chest install` to put back the locked copy";
    assert_eq!(frozen(&mut patch).await.unwrap_err(), edit);
    let entry = patch.manifest.skills["pdf"].clone();
    assert_eq!(patch.state(Section::Skills, "pdf", &entry), "edited");
    assert_eq!(patch.verify().unwrap(), std::slice::from_ref(&edit));
    let report = sync(&mut patch, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Edited)]);
    assert_eq!(
        report.warnings,
        [format!("{edit}, so chest left it as it is: {how}")]
    );
    std::fs::write(canonical.join("SKILL.md"), "# local\n").unwrap();
    sync(&mut patch, Mode::Resolve).await;
    // A deleted copy of a patched entry is missing, and a frozen install
    // puts it back from the patch.
    std::fs::remove_dir_all(&canonical).unwrap();
    assert_eq!(patch.state(Section::Skills, "pdf", &entry), "missing");
    let gone = format!("skills.pdf: {} is missing", canonical.display());
    assert_eq!(patch.verify().unwrap(), [gone]);
    frozen(&mut patch).await.unwrap();
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

    // An installed copy the user edited is not replaced, also once the lock
    // moves, as an update or a teammate's lock moves it, nor at startup.
    std::fs::write(canonical.join("SKILL.md"), "# edited\n").unwrap();
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Edited)]);
    let before = read(&project.lock_path());
    let left = format!(
        "skills.pdf: {} was modified since chest installed it, so chest left it as it is: delete it and run `chest install` to put back the locked copy",
        canonical.display()
    );
    let entry = project.manifest.skills["pdf"].clone();
    for mode in [
        Mode::Update(Vec::new()),
        Mode::Resolve,
        Mode::Startup { offline: false },
    ] {
        let report = sync(&mut project, mode).await;
        assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Edited)]);
        assert_eq!(report.warnings, [left.as_str()]);
        assert_eq!(read(&canonical.join("SKILL.md")), "# edited\n");
        let locked = &project.lock.as_ref().unwrap().skills[0];
        assert_eq!(locked.commit.as_deref(), Some(second.as_str()));
        assert_eq!(project.state(Section::Skills, "pdf", &entry), "edited");
    }
    let error = frozen(&mut project).await.unwrap_err();
    assert!(error.ends_with(".agents/skills/pdf does not match the lock"));
    std::fs::write(canonical.join("SKILL.md"), "# pdf 1\n").unwrap();
    std::fs::write(project.lock_path(), &before).unwrap();
    let report = sync(&mut project, Mode::Update(Vec::new())).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Installed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 2\n");
    let locked = &project.lock.as_ref().unwrap().skills[0];
    assert_eq!(locked.commit.as_deref(), Some(second.as_str()));
    // An update that finds nothing newer says so.
    let report = sync(&mut project, Mode::Update(Vec::new())).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Unchanged)]);
    // An update stopped before it wrote the lock leaves the new copy, and
    // install puts back what the lock pins.
    let updated = read(&project.lock_path());
    std::fs::write(project.lock_path(), &before).unwrap();
    // Startup never loads a copy that the lock no longer pins, as after a
    // pull brings a teammate's lock.
    let report = sync(&mut project, Mode::Startup { offline: true }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Skipped)]);
    let inputs = yapi_core::chest::startup::installed_inputs(&project, &root, &mut Vec::new());
    assert!(inputs.skills.is_empty(), "{:?}", inputs.skills);
    let entry = project.manifest.skills["pdf"].clone();
    assert_eq!(project.state(Section::Skills, "pdf", &entry), "outdated");
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&canonical.join("SKILL.md")), "# pdf 1\n");
    std::fs::write(project.lock_path(), updated).unwrap();
    let report = sync(&mut project, Mode::Startup { offline: false }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
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

    // An edited extension stays too. yapi loads it while the lock pins what
    // chest placed, and says it leaves it out once the lock moves on.
    remote.commit(&[("tool/index.ts", "export default () => {};\n")], "tool");
    let tool = format!(
        "[extensions]\ntool = {{ git = \"{}\", rev = \"tool\", path = \"tool\" }}\n",
        remote.source
    );
    let mut project = open(&dir, &tool);
    sync(&mut project, Mode::Resolve).await;
    let store = root.join(".yapi/chest/tool");
    std::fs::write(store.join("index.ts"), "// mine\n").unwrap();
    let left = format!(
        "extensions.tool: {} was modified since chest installed it, so chest left it as it is: delete it and run `chest install` to put back the locked copy",
        store.display()
    );
    let loads = |project: &Project| {
        let inputs = installed_inputs(project, &root, &mut Vec::new());
        !inputs.packages.is_empty()
    };
    let report = sync(&mut project, Mode::Update(Vec::new())).await;
    assert_eq!(report.warnings, [left.as_str()]);
    assert!(loads(&project));
    remote.commit(&[("tool/index.ts", "export default () => 2;\n")], "tool");
    let report = sync(&mut project, Mode::Update(Vec::new())).await;
    assert_eq!(outcomes(&report), [("extensions.tool", Outcome::Edited)]);
    let moved = format!("{left}, and until then yapi does not load it");
    assert_eq!(report.warnings, [moved]);
    assert!(!loads(&project));

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
            ("skills/review.md", "# review\n"),
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
    // An item whose name is not an entry name is skipped, not the package,
    // and so is a single-file skill, which chest doesn't install.
    assert_eq!(
        report.warnings,
        [
            "skills.anthropic: skills/review.md was skipped: it is a single-file skill, and chest installs only skill folders that hold a SKILL.md",
            "agents.team: agents/README.md was skipped: `README.md` is not a valid name",
        ]
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
    // A copy chest placed for another lock, unchanged since, goes back as
    // the lock pins it, as after a pull brings an older lock.
    std::fs::write(&skill, "# pdf\n").unwrap();
    sync(
        &mut open(&dir, &skill_manifest(&remote, "v2")),
        Mode::Resolve,
    )
    .await;
    assert_eq!(read(&skill), "# pdf 2\n");
    std::fs::write(&lock_path, &locked).unwrap();
    let mut project = open(&dir, &skill_manifest(&remote, "v1"));
    let report = sync(&mut project, Mode::Frozen).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);
    assert_eq!(read(&skill), "# pdf\n");
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
    // A manifest or lock that is a link is refused in either scope, also
    // when the lock turns into one after the project opened, and nothing is
    // read or written through it.
    #[cfg(unix)]
    {
        let refused = |path: &Path| {
            format!(
                "{} is a symbolic link, which chest does not follow",
                path.display()
            )
        };
        let empty = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n";
        let mut project = open(&dir, empty);
        let (manifest, lock) = (root.join("chest.toml"), root.join("chest.lock"));
        for (file, moved) in
            [&lock, &manifest].map(|file| (file, outside.join(file.file_name().unwrap())))
        {
            std::fs::rename(file, &moved).unwrap();
            std::os::unix::fs::symlink(&moved, file).unwrap();
            for scope in [Scope::Project, Scope::Global] {
                let error = Project::open(&manifest, scope).unwrap_err().to_string();
                assert!(error.starts_with(&refused(file)), "{error}");
            }
            if file == &lock {
                let locked = read(&moved);
                for mode in [Mode::Resolve, Mode::Frozen] {
                    let error = failed(&mut project, mode).await;
                    assert!(error.starts_with(&refused(&lock)), "{error}");
                }
                assert_eq!(read(&moved), locked);
                assert!(root.join(".agents/skills/pdf").exists());
            }
            std::fs::remove_file(file).unwrap();
            std::fs::rename(&moved, file).unwrap();
        }
    }
    // An install waits while another chest command holds the lock, and says
    // so once.
    let lock = root.join("chest.lock");
    let held = FileLock::acquire(&lock, &Default::default()).await.unwrap();
    let mut project = open(
        &dir,
        "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n",
    );
    let heard = std::sync::Mutex::new(Vec::new());
    let mut progress = |line: &str| heard.lock().unwrap().push(line.to_owned());
    let mut npm = Npm::new(HashMap::new());
    let release = async {
        for _ in 0..100 {
            if !heard.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        drop(held);
    };
    let synced = project.sync(Mode::Resolve, &mut npm, &mut progress);
    let (synced, ()) = tokio::join!(synced, release);
    synced.unwrap();
    let waiting = "Waiting for another chest command, which holds the lock of";
    assert_eq!(
        heard.into_inner().unwrap(),
        [format!("{waiting} {}...", lock.display())]
    );
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

        // A `\` in a folder name is part of the name, not a separator: its
        // line escapes it, and the record keeps the copy chest's, so a new
        // commit replaces it.
        let folders = "[\".agents/skills\", \".claude/skills\"]";
        let manifest = skill_manifest(&remote, "v1").replace(folders, "[\"a\\\\b\"]");
        sync(&mut open(&dir, &manifest), Mode::Resolve).await;
        assert!(read(&root.join(".gitignore")).contains("\na\\\\b/pdf\n"));
        remote.commit(&[("skills/pdf/SKILL.md", "# pdf 2\n")], "v2");
        let manifest = manifest.replace("\"v1\"", "\"v2\"");
        let report = sync(&mut open(&dir, &manifest), Mode::Resolve).await;
        assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Installed)]);
        assert_eq!(read(&root.join("a\\b/pdf/SKILL.md")), "# pdf 2\n");
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
    let package = json!({"name": name, "version": "1.0.0", "dependencies": dependencies});
    let package = package.to_string();
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
    // Its index file, which no manifest lists, runs with the entry's
    // grants and loads modules from the whole store folder.
    let (root, agent) = (dir.join("project"), dir.join("agent"));
    // The same package from the user's settings is left out, so only the
    // project's entry loads, with a warning that says how to remove it.
    let user_copy = agent.join("npm/node_modules/ext");
    std::fs::create_dir_all(&user_copy).unwrap();
    std::fs::write(user_copy.join("index.js"), "").unwrap();
    let package_json = r#"{"name": "ext", "pi": {"extensions": ["./index.js"]}}"#;
    std::fs::write(user_copy.join("package.json"), package_json).unwrap();
    std::fs::write(
        agent.join("settings.json"),
        r#"{"packages": ["npm:ext@^1"]}"#,
    )
    .unwrap();
    let inputs = installed_inputs(&project, &root, &mut Vec::new());
    let settings = SettingsManager::load(&agent, &root, true);
    let resolved = resolve_resources(&root, &agent, &settings, &[], &inputs);
    let policies = file_policies(&resolved, &inputs, &root, &mut Vec::new());
    let policy = &policies.files[&store.join("node_modules/ext/index.js")];
    assert!(
        policy.grants.filesystem && !policy.grants.network,
        "{policy:?}"
    );
    assert_eq!(policy.root.as_ref(), Some(&store));
    let sources: Vec<&str> = resolved
        .enabled(ResourceType::Extensions)
        .map(|info| info.source.as_str())
        .collect();
    assert_eq!(sources, ["chest:ext"]);
    let duplicates = yapi_core::chest::startup::duplicates(&inputs, &settings, None, &agent);
    let [(path, message)] = &duplicates[..] else {
        panic!("{duplicates:?}");
    };
    assert!(path.ends_with("chest.toml"), "{path}");
    assert_eq!(
        message,
        &format!(
            "npm:ext@^1 in the global settings is left out, because extensions.ext in {path} installs the same package: remove it with `yapi remove npm:ext@^1`"
        )
    );
    // A project-only run leaves the user's copy out unless it keeps it, so
    // there is nothing to warn of.
    let keep = yapi_core::packages::resolve::ProjectOnly::default();
    let left_out = yapi_core::chest::startup::duplicates(&inputs, &settings, Some(&keep), &agent);
    assert!(left_out.is_empty(), "{left_out:?}");
    // A placed skill in a folder yapi reads on its own resolves once, as
    // chest's.
    let skills: Vec<(&str, &str)> = resolved
        .enabled(ResourceType::Skills)
        .filter(|info| Path::new(&info.path).starts_with(&root))
        .map(|info| (info.path.as_str(), info.source.as_str()))
        .collect();
    let guide_file = guide.join("SKILL.md").display().to_string();
    assert_eq!(skills, [(guide_file.as_str(), "chest:guide")]);
    // The project's settings come before a global entry of the package,
    // which is left out, and pi's rules leave out the user's settings copy.
    let global_manifest = agent.join("chest.toml").display().to_string();
    let global = yapi_core::chest::startup::Installed {
        packages: vec![yapi_core::packages::resolve::PackageInput {
            source: "chest:gext".into(),
            scope: "user".into(),
            root: user_copy.clone(),
            filter: None,
        }],
        identities: vec![(
            "npm:ext".into(),
            "gext".into(),
            global_manifest.clone(),
            true,
        )],
        ..Default::default()
    };
    std::fs::write(
        root.join(".yapi/settings.json"),
        r#"{"packages": ["npm:ext"]}"#,
    )
    .unwrap();
    let settings = SettingsManager::load(&agent, &root, true);
    let resolved = resolve_resources(&root, &agent, &settings, &[], &global);
    assert_eq!(resolved.enabled(ResourceType::Extensions).count(), 0);
    let duplicates = yapi_core::chest::startup::duplicates(&global, &settings, None, &agent);
    let message = format!(
        "extensions.gext in {global_manifest} is left out, because npm:ext in the project's settings installs the same package: remove it with `chest -g remove gext`"
    );
    assert_eq!(duplicates, [(global_manifest.clone(), message)]);

    // Which copy loads and what the warning says, for chest entries by
    // name, scope and source, the packages of the project's and the global
    // settings, a project-only run's keep list, and whether the project's
    // trust is saved.
    let project_manifest = root.join("chest.toml").display().to_string();
    let (p, g) = (project_manifest.as_str(), global_manifest.as_str());
    let npm = |name: &str| Entry {
        npm: Some(name.into()),
        ..Entry::default()
    };
    let git = |path: Option<&str>| Entry {
        git: Some("https://example.test/acme/mono".into()),
        path: path.map(str::to_owned),
        ..Entry::default()
    };
    let left = |manifest: &str, copy: &str, kept: &str, remove: &str| {
        let message = format!(
            "{copy} is left out, because {kept} installs the same package: remove it with `{remove}`"
        );
        (manifest.to_owned(), message)
    };
    // Each entry's copy holds an extension and a theme.
    let copy = dir.join("chest-copy");
    for file in ["extensions/gext.ts", "themes/dark.json"] {
        std::fs::create_dir_all(copy.join(file).parent().unwrap()).unwrap();
        std::fs::write(copy.join(file), "{}").unwrap();
    }
    let installs = |entries: &[(&str, bool, Entry)]| {
        let mut installed = yapi_core::chest::startup::Installed::default();
        for (name, global, entry) in entries {
            let manifest = if *global { g } else { p };
            let id = yapi_core::chest::startup::package_identity(entry).unwrap();
            let name = (*name).to_owned();
            installed
                .packages
                .push(yapi_core::packages::resolve::PackageInput {
                    source: format!("chest:{name}"),
                    scope: if *global { "user" } else { "project" }.into(),
                    root: copy.clone(),
                    filter: None,
                });
            installed
                .identities
                .push((id, name, manifest.to_owned(), *global));
        }
        installed
    };
    // The project's entries, as from a manifest with another name.
    let dev = root.join("chest.dev.toml").display().to_string();
    let in_dev = |mut installed: yapi_core::chest::startup::Installed| {
        for (.., path, global) in &mut installed.identities {
            if !*global {
                path.clone_from(&dev);
            }
        }
        installed
    };
    // A patched entry loads its patch, with no lock.
    std::fs::create_dir_all(root.join("patches/gx")).unwrap();
    let patched = open(
        &dir,
        "[extensions]\ngx = { git = \"https://example.test/acme/mono\", rev = \"v1\" }\n\n[patch]\ngx = { path = \"patches/gx\" }\n",
    );
    let patched = installed_inputs(&patched, &root, &mut Vec::new());
    let all = json!({"extensions": ["*"]});
    let (in_g, in_p) = (
        format!("extensions.gext in {g}"),
        format!("extensions.pext in {p}"),
    );
    let rows = [
        // A kind list keeps a global copy's files, so it counts, after
        // the project's copies and those the run keeps whole.
        (
            installs(&[("gext", true, npm("ext"))]),
            json!([]),
            json!(["npm:ext"]),
            all.clone(),
            false,
            vec![left(
                g,
                "npm:ext in the global settings",
                &in_g,
                "yapi remove npm:ext",
            )],
        ),
        (
            installs(&[("pext", false, npm("ext"))]),
            json!([]),
            json!(["npm:ext"]),
            all,
            false,
            vec![left(
                p,
                "npm:ext in the global settings",
                &in_p,
                "yapi remove npm:ext",
            )],
        ),
        (
            installs(&[("gext", true, npm("ext"))]),
            json!([]),
            json!(["npm:ext"]),
            json!({"packages": ["npm:ext"], "themes": ["*"]}),
            false,
            vec![left(
                g,
                &in_g,
                "npm:ext in the global settings",
                "chest -g remove gext",
            )],
        ),
        // A copy that a kind list keeps no file of doesn't count, so the
        // settings copy keeps the file the pattern names.
        (
            installs(&[("gext", true, npm("ext"))]),
            json!([]),
            json!(["npm:ext"]),
            json!({"extensions": ["npm/node_modules/ext/index.js"]}),
            false,
            vec![],
        ),
        (
            installs(&[("gext", true, npm("ext"))]),
            json!([]),
            json!(["npm:ext"]),
            json!({"skills": ["pdf"]}),
            false,
            vec![],
        ),
        // Entries with a `path` take different folders of one repository,
        // and both load. The whole repository holds each, as an entry or in
        // settings, so it's left out after one loads.
        (
            installs(&[
                ("ma", false, git(Some("ext-a"))),
                ("mono", true, git(None)),
                ("mb", true, git(Some("ext-b"))),
            ]),
            json!([]),
            json!(["git:example.test/acme/mono"]),
            Value::Null,
            false,
            vec![
                left(
                    g,
                    &format!("extensions.mono in {g}"),
                    &format!("extensions.ma in {p}"),
                    "chest -g remove mono",
                ),
                left(
                    p,
                    "git:example.test/acme/mono in the global settings",
                    &format!("extensions.ma in {p}"),
                    "yapi remove git:example.test/acme/mono",
                ),
            ],
        ),
        // Of one manifest's entries, the whole repository after one folder
        // of it, and other spellings of that folder, are left out.
        (
            installs(&[
                ("ma", false, git(Some("ext-a"))),
                ("mono", false, git(None)),
                ("dot", false, git(Some("./ext-a/"))),
                ("root", false, git(Some("."))),
            ]),
            json!([]),
            json!([]),
            Value::Null,
            false,
            ["mono", "dot", "root"]
                .map(|name| {
                    left(
                        p,
                        &format!("extensions.{name} in {p}"),
                        &format!("extensions.ma in {p}"),
                        &format!("chest remove {name}"),
                    )
                })
                .into(),
        ),
        // A patched entry is its package, so its patch loads.
        (
            patched,
            json!(["git:example.test/acme/mono"]),
            json!([]),
            Value::Null,
            false,
            vec![left(
                p,
                "git:example.test/acme/mono in the project's settings",
                &format!("extensions.gx in {p}"),
                "yapi remove -l --approve git:example.test/acme/mono",
            )],
        ),
        // Of two entries of one manifest, the first loads. The warning
        // names the command for that manifest.
        (
            in_dev(installs(&[
                ("x1", false, npm("x")),
                ("x2", false, npm("x")),
                ("y1", true, npm("y")),
                ("y2", true, npm("y")),
            ])),
            json!([]),
            json!([]),
            Value::Null,
            false,
            vec![
                left(
                    &dev,
                    &format!("extensions.x2 in {dev}"),
                    &format!("extensions.x1 in {dev}"),
                    "chest --manifest chest.dev.toml remove x2",
                ),
                left(
                    g,
                    &format!("extensions.y2 in {g}"),
                    &format!("extensions.y1 in {g}"),
                    "chest -g remove y2",
                ),
            ],
        ),
        // `yapi remove -l` needs `--approve` until the trust is saved.
        (
            installs(&[("pext", false, npm("ext"))]),
            json!(["npm:ext"]),
            json!([]),
            Value::Null,
            false,
            vec![left(
                p,
                "npm:ext in the project's settings",
                &in_p,
                "yapi remove -l --approve npm:ext",
            )],
        ),
        (
            installs(&[("pext", false, npm("ext"))]),
            json!(["npm:ext"]),
            json!([]),
            Value::Null,
            true,
            vec![left(
                p,
                "npm:ext in the project's settings",
                &in_p,
                "yapi remove -l npm:ext",
            )],
        ),
    ];
    let trust = yapi_core::trust::TrustStore::new(&agent);
    for (installed, project, global, keep, saved, expected) in rows {
        std::fs::write(
            root.join(".yapi/settings.json"),
            json!({"packages": project}).to_string(),
        )
        .unwrap();
        std::fs::write(
            agent.join("settings.json"),
            json!({"packages": global}).to_string(),
        )
        .unwrap();
        trust.set(&root, Some(saved)).unwrap();
        let settings = SettingsManager::load(&agent, &root, true);
        let keep = (!keep.is_null()).then(|| {
            let document = json!({"projectOnly": keep});
            yapi_core::packages::resolve::ProjectOnly::from_settings(document.as_object().unwrap())
        });
        let warned =
            yapi_core::chest::startup::duplicates(&installed, &settings, keep.as_ref(), &agent);
        assert_eq!(warned, expected, "{:?}", installed.identities);
    }
    trust.set(&root, None).unwrap();
    std::fs::remove_file(root.join(".yapi/settings.json")).unwrap();
    std::fs::remove_file(agent.join("settings.json")).unwrap();

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

    // So does a tarball whose package.json disagrees with its packument, in
    // an npm extension's tree or in a git extension's dependencies.
    let dir = scratch("npm-disagrees");
    let remote = Remote::new(&dir);
    let gitext = json!({"name": "gitext", "dependencies": {"dep": "^1"}}).to_string();
    remote.commit(&[("package.json", &gitext)], "v1");
    let git_manifest = format!(
        "[extensions]\ngitext = {{ git = \"{}\", rev = \"v1\" }}\n",
        remote.source
    );
    for (name, manifest) in [("ext", ext_manifest), ("dep", git_manifest.as_str())] {
        let server = MockServer::local_with(|base| {
            // A second package.json, which unpacking writes over the first.
            let relabeled = json!({"name": name, "version": "1.0.1"}).to_string();
            let (packument, tarball) =
                publish(base, name, json!({}), &[("package.json", &relabeled)]);
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
        let expected = format!(
            "The package.json in node_modules/{name} names {name}@1.0.1, not the pinned {name}@1.0.0"
        );
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

    // Startup takes the lock's lock only to put something back, so one held
    // elsewhere holds up neither a run with everything present nor one offline.
    let held = root.join("chest.lock.lock");
    std::fs::create_dir(&held).unwrap();
    let manifest = skill_manifest(&remote, "v1");
    let report = sync(&mut open(&dir, &manifest), Mode::Startup { offline: false }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Unchanged)]);
    // Offline, a skill missing a link keeps loading from its copy in a
    // folder yapi reads on its own, and one without that copy is left out.
    std::fs::remove_file(root.join(".claude/skills/pdf")).unwrap();
    let report = sync(&mut open(&dir, &manifest), Mode::Startup { offline: true }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Skipped)]);
    assert_eq!(
        report.warnings,
        [
            "chest.toml: skills.pdf could not be put back offline, so its copy in a folder yapi reads on its own keeps loading"
        ]
    );
    std::fs::remove_dir_all(root.join(".agents/skills/pdf")).unwrap();
    let report = sync(&mut open(&dir, &manifest), Mode::Startup { offline: true }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Skipped)]);
    assert_eq!(
        report.warnings,
        ["chest.toml: skills.pdf could not be put back offline, so it was left out"]
    );
    std::fs::remove_dir(&held).unwrap();
    let report = sync(&mut open(&dir, &manifest), Mode::Startup { offline: false }).await;
    assert_eq!(outcomes(&report), [("skills.pdf", Outcome::Replayed)]);

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
    assert_eq!(
        report.warnings,
        [
            "chest.toml: skills.pdf, skills.notes are not locked, or their lock is stale, so they were skipped: run `chest install`"
        ]
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

    // A path entry whose source didn't change stays as it is.
    let manifest = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n\n[skills]\nnotes = { path = \"notes\" }\n";
    let mut project = open(&dir, manifest);
    let report = sync(&mut project, Mode::Resolve).await;
    assert_eq!(
        outcomes(&report),
        [
            ("skills.notes", Outcome::Unchanged),
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
    // while other entries install, and not removed, with a warning.
    #[cfg(unix)]
    {
        let link = root.join(".claude/skills/notes");
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("../../.agents/skills/mine", &link).unwrap();
        std::fs::create_dir_all(root.join("extra")).unwrap();
        std::fs::write(root.join("extra/SKILL.md"), "# extra\n").unwrap();
        let mut project = open(&dir, &format!("{manifest}extra = {{ path = \"extra\" }}\n"));
        let report = sync(&mut project, Mode::Resolve).await;
        assert_eq!(
            outcomes(&report),
            [
                ("skills.notes", Outcome::Edited),
                ("skills.extra", Outcome::Installed)
            ]
        );
        let edited = format!(
            "skills.notes: {} was modified since chest installed it",
            link.display()
        );
        assert_eq!(
            report.warnings,
            [format!(
                "{edited}, so chest left it as it is: delete it and run `chest install` to put back the locked copy"
            )]
        );
        assert_eq!(
            read(&root.join(".claude/skills/extra/SKILL.md")),
            "# extra\n"
        );
        let entry = project.manifest.skills["notes"].clone();
        assert_eq!(project.state(Section::Skills, "notes", &entry), "edited");
        // verify names what is wrong with the link, once.
        let canonical = root.join(".agents/skills/notes");
        let wrong = format!(
            "skills.notes: {} does not link to {}",
            link.display(),
            canonical.display()
        );
        assert_eq!(project.verify().unwrap(), [wrong]);
        let empty = "[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\n";
        let report = sync(&mut open(&dir, empty), Mode::Resolve).await;
        assert!(
            report.warnings.iter().any(|warning| warning.ends_with(
                ".claude/skills/notes was modified since chest installed it, so chest left it as it is: delete it, or undo the edit and run `chest install` to remove it"
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
    // yapi's startup doesn't note it again on every run.
    let report = sync(&mut project, Mode::Startup { offline: true }).await;
    assert!(report.notices.is_empty(), "{:?}", report.notices);
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
    // So is one that holds it through a folder linking there.
    #[cfg(unix)]
    {
        let project = dir.join("project");
        std::fs::create_dir_all(project.join(".claude")).unwrap();
        std::os::unix::fs::symlink("../.agents/skills", project.join(".claude/skills")).unwrap();
        std::fs::write(project.join(".agents/SKILL.md"), "# agents\n").unwrap();
        let linked =
            "[folders]\nskills = [\".claude/skills\"]\n\n[skills]\nme = { path = \".agents\" }\n";
        let error = failed(&mut open(&dir, linked), Mode::Resolve).await;
        let holds = format!(
            "{} holds {}, where chest installs",
            project.join(".agents").display(),
            project.join(".agents/skills").display()
        );
        assert!(error.contains(&holds), "{error}");
        assert!(
            project
                .join(".agents/skills/me")
                .symlink_metadata()
                .is_err()
        );
        std::fs::remove_file(project.join(".agents/SKILL.md")).unwrap();
        std::fs::remove_file(project.join(".claude/skills")).unwrap();
    }

    // An extension whose folder holds the project or the store, as a path
    // source or as a patch, is refused before anything is written.
    std::fs::create_dir_all(dir.join("project/.yapi")).unwrap();
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
        (
            "[extensions]\nwide = { path = \".yapi\" }\n".to_owned(),
            "extensions.wide",
            dir.join("project/.yapi"),
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
    // Startup leaves it where it is, without a lock entry.
    let report = sync(&mut open(&dir, &patched), Mode::Startup { offline: false }).await;
    assert_eq!(outcomes(&report)[0], ("skills.pdf", Outcome::Unchanged));
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
    let mut team = open(&dir, &package);
    sync(&mut team, Mode::Resolve).await;
    for skill in ["one", "two"] {
        let placed = dir.join(format!("project/.agents/skills/{skill}/SKILL.md"));
        assert_eq!(read(&placed), format!("# {skill}\n"));
    }
    // yapi loads those placed items, not the patch's whole folder.
    let inputs = installed_inputs(&team, &dir.join("project"), &mut Vec::new());
    let roots: Vec<PathBuf> = inputs
        .skills
        .iter()
        .map(|skill| skill.root.clone())
        .collect();
    let skills = dir.join("project/.agents/skills");
    assert_eq!(roots, [skills.join("one"), skills.join("two")]);
    // So does a path entry naming that folder, and startup finds its items
    // in place and loads them.
    let mut team = open(
        &dir,
        &format!("{folders}team = {{ path = \"local/pack\" }}\n"),
    );
    sync(&mut team, Mode::Resolve).await;
    let report = sync(&mut team, Mode::Startup { offline: true }).await;
    assert_eq!(outcomes(&report), [("skills.team", Outcome::Unchanged)]);
    let inputs = installed_inputs(&team, &dir.join("project"), &mut Vec::new());
    let roots: Vec<PathBuf> = inputs
        .skills
        .iter()
        .map(|skill| skill.root.clone())
        .collect();
    assert_eq!(roots, [skills.join("one"), skills.join("two")]);
    // Without its source, `list` and `verify` say so, rather than name a
    // copy it never had.
    let pack = dir.join("project/local/pack");
    let moved = dir.join("project/local/moved");
    std::fs::rename(&pack, &moved).unwrap();
    let entry = team.manifest.skills["team"].clone();
    assert_eq!(team.state(Section::Skills, "team", &entry), "no source");
    let gone = format!("skills.team: its source {} is missing", pack.display());
    assert_eq!(team.verify().unwrap(), [gone]);
    std::fs::rename(&moved, &pack).unwrap();
    // `install` skips it with a warning, for a package, a skill folder and an
    // extension alike, keeps what it installed and the entry's lock, and
    // `--frozen` fails on it. A dropped entry's copy is still removed, unless
    // the missing folder held several items, whose copies chest can't name:
    // then every copy in that section stays, and the warning lists them.
    std::fs::write(
        dir.join("project/local/ext.ts"),
        "export default () => {};\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("project/local/extra")).unwrap();
    std::fs::write(dir.join("project/local/extra/SKILL.md"), "# extra\n").unwrap();
    let extra = "extra = { path = \"local/extra\" }\n";
    for (manifest, id, path, copies, holds) in [
        (
            format!("{folders}team = {{ path = \"local/pack\" }}\n"),
            "skills.team",
            "local/pack",
            &["one", "two"][..],
            true,
        ),
        (
            format!("{folders}one = {{ path = \"local/pack/skills/one\" }}\n"),
            "skills.one",
            "local/pack/skills/one",
            &["one"][..],
            false,
        ),
        (
            format!("{folders}\n[extensions]\next = {{ path = \"local/ext.ts\" }}\n"),
            "extensions.ext",
            "local/ext.ts",
            &[][..],
            false,
        ),
    ] {
        let with_extra = manifest.replacen("[skills]\n", &format!("[skills]\n{extra}"), 1);
        sync(&mut open(&dir, &with_extra), Mode::Resolve).await;
        let extra_copy = dir.join("project/.agents/skills/extra");
        assert!(extra_copy.exists());
        let mut project = open(&dir, &manifest);
        let source = dir.join("project").join(path);
        std::fs::rename(&source, &moved).unwrap();
        let report = sync(&mut project, Mode::Resolve).await;
        assert_eq!(outcomes(&report)[0], (id, Outcome::Skipped));
        let skipped = format!(
            "{id}: {} does not exist, so chest skipped it: restore it or remove the entry",
            source.display()
        );
        assert!(report.warnings.contains(&skipped), "{:?}", report.warnings);
        for copy in copies {
            assert!(
                dir.join("project/.agents/skills").join(copy).exists(),
                "{copy}"
            );
        }
        let held = format!("{id}'s folder is missing and held several items, so chest keeps ");
        let warned = report.warnings.iter().find(|w| w.starts_with(&held));
        assert_eq!(warned.is_some(), holds, "{:?}", report.warnings);
        assert!(warned.is_none_or(|warned| warned.contains(&extra_copy.display().to_string())));
        assert_eq!(extra_copy.exists(), holds, "{id}");
        let name = id.split_once('.').unwrap().1;
        let lock = read(&dir.join("project/chest.lock"));
        assert!(lock.contains(&format!("name = \"{name}\"")), "{lock}");
        let error = failed(&mut project, Mode::Frozen).await;
        assert_eq!(error, format!("{id}: {} does not exist", source.display()));
        assert_eq!(read(&dir.join("project/chest.lock")), lock);
        std::fs::rename(&moved, &source).unwrap();
        // With the folder back, the held copy goes.
        sync(&mut project, Mode::Resolve).await;
        assert!(!extra_copy.exists(), "{id}");
    }

    // A git extension loads modules from its store folder, whatever its
    // `path`, and an npm one from its store folder too.
    let project = open(
        &dir,
        "[extensions]\ngit = { git = \"localhost/nowhere/repo\", rev = \"v1\", path = \"ext\" }\nnpm = { npm = \"ext\" }\n",
    );
    let store = dir.join("project/.yapi/chest");
    assert_eq!(
        project.extension_roots("git"),
        Some((store.join("git"), store.join("git")))
    );
    assert_eq!(
        project.extension_roots("npm"),
        Some((store.join("npm/node_modules/ext"), store.join("npm")))
    );
}
