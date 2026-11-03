//! The `chest` binary end to end, with a temporary home folder, a local git
//! remote and a mock npm registry.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use aws_lc_rs::digest::{SHA512, digest};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use indexmap::IndexMap;
use serde_json::json;
use yapi_mock::{Cassette, Interaction, MockServer, RequestMatch, Response};

/// A scratch folder with a home folder and a project inside.
struct Scratch {
    home: PathBuf,
    project: PathBuf,
    dir: PathBuf,
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("chest-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (home, project) = (dir.join("home"), dir.join("project"));
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    let dir = dir.canonicalize().unwrap();
    Scratch {
        home: dir.join("home"),
        project: dir.join("project"),
        dir,
    }
}

/// What the child gets: the scratch home, no CI or CHEST_MANIFEST from the
/// host, and `localhost/<path>` git sources mapped to local repositories.
fn environment(scratch: &Scratch) -> Vec<(String, OsString)> {
    vec![
        ("HOME".into(), scratch.home.clone().into_os_string()),
        ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
        ("GIT_CONFIG_COUNT".into(), "1".into()),
        ("GIT_CONFIG_KEY_0".into(), "url.file:///.insteadOf".into()),
        ("GIT_CONFIG_VALUE_0".into(), "https://localhost/".into()),
    ]
}

/// chest in the project with `args` and `extra` environment, no input, and
/// its output captured.
fn command(scratch: &Scratch, args: &[&str], extra: &[(&str, &str)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chest"));
    command
        .args(args)
        .current_dir(&scratch.project)
        .env_clear()
        .envs(environment(scratch))
        .envs(extra.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Runs chest in the project with `args` and `extra` environment, no input.
fn chest(scratch: &Scratch, args: &[&str], extra: &[(&str, &str)]) -> Output {
    command(scratch, args, extra).output().unwrap()
}

/// Runs chest and fails the test unless it succeeds.
fn ok(scratch: &Scratch, args: &[&str]) -> Output {
    let output = chest(scratch, args, &[]);
    assert!(output.status.success(), "chest {args:?}: {}", text(&output));
    output
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
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

/// A bare remote with a working clone that commits and tags into it.
struct Remote {
    work: PathBuf,
    /// The repository as a `chest add` source.
    source: String,
}

impl Remote {
    fn new(dir: &Path) -> Remote {
        git(
            dir,
            &["init", "--quiet", "--bare", "-b", "main", "remote.git"],
        );
        git(dir, &["init", "--quiet", "-b", "main", "work"]);
        let work = dir.join("work");
        let remote = dir.join("remote.git");
        git(
            &work,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        Remote {
            work,
            source: format!("git:localhost{}", remote.display()),
        }
    }

    fn commit(&self, files: &[(&str, &str)], tag: &str) {
        for (path, contents) in files {
            write(&self.work.join(path), contents);
        }
        git(&self.work, &["add", "-A"]);
        git(&self.work, &["commit", "--quiet", "-m", tag]);
        git(&self.work, &["tag", "-f", tag]);
        git(
            &self.work,
            &["push", "--quiet", "--force", "--tags", "origin", "main"],
        );
    }
}

/// A remote with the skill `skills/pdf` at `v1`.
fn skill_remote(scratch: &Scratch) -> Remote {
    let remote = Remote::new(&scratch.dir);
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf\n")], "v1");
    remote
}

fn init(scratch: &Scratch) {
    ok(
        scratch,
        &["init", "--yes", "--skills", ".agents/skills,.claude/skills"],
    );
}

#[test]
fn init_detects_agents() {
    let scratch = scratch("init");
    std::fs::create_dir_all(scratch.project.join(".claude")).unwrap();
    std::fs::create_dir_all(scratch.project.join(".cursor")).unwrap();
    ok(&scratch, &["init"]);
    let expected = "# Installed with `chest install`, pinned in the lock beside this file.\n\n[folders]\nskills = [\".agents/skills\", \".claude/skills\"]\nagents = [\".claude/agents\", \".cursor/agents\"]\nextensions = \".yapi/chest\"\n";
    assert_eq!(read(&scratch.project.join("chest.toml")), expected);
    let again = chest(&scratch, &["init", "--yes"], &[]);
    assert_eq!(again.status.code(), Some(1));
    assert!(
        text(&again).contains("chest.toml already exists"),
        "{}",
        text(&again)
    );
    // CHEST_MANIFEST names the manifest when --manifest is absent.
    let dev = chest(
        &scratch,
        &["init", "--yes"],
        &[("CHEST_MANIFEST", "chest.dev.toml")],
    );
    assert!(dev.status.success(), "{}", text(&dev));
    assert_eq!(read(&scratch.project.join("chest.dev.toml")), expected);
    // One outside the folder is an error, as yapi loads no manifest for it.
    let outside = scratch.dir.join("chest.out.toml");
    let path = outside.display().to_string();
    let refused = chest(&scratch, &["init", "--yes"], &[("CHEST_MANIFEST", &path)]);
    assert_eq!(refused.status.code(), Some(1), "{}", text(&refused));
    assert!(
        text(&refused).contains("CHEST_MANIFEST must name a file inside the project"),
        "{}",
        text(&refused)
    );
    assert!(!outside.exists());
}

#[test]
fn init_package_scaffold() {
    let scratch = scratch("package");
    write(&scratch.project.join("skills/pdf/SKILL.md"), "# pdf\n");
    write(&scratch.project.join("agents/reviewer.md"), "# reviewer\n");
    ok(&scratch, &["init", "--yes"]);
    ok(&scratch, &["init", "--package", "--yes"]);
    let manifest = read(&scratch.project.join("chest.toml"));
    assert!(
        manifest.ends_with("\n[package]\nname = \"project\"\nversion = \"0.1.0\"\ncapabilities = []\nskills = [\"skills/*\"]\nagents = [\"agents/*.md\"]\n"),
        "{manifest}"
    );
    // With no agent found, skills and subagents still get a folder.
    assert!(
        manifest.contains("\nskills = [\".agents/skills\"]\nagents = [\".yapi/agents\"]\n"),
        "{manifest}"
    );
    let again = chest(&scratch, &["init", "--package", "--yes"], &[]);
    assert!(text(&again).contains("already has a [package] table"));
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

fn get(path: &str, body: Option<String>, base64: Option<String>) -> Interaction {
    Interaction {
        request: RequestMatch {
            method: "GET".into(),
            path: path.into(),
        },
        response: Response {
            status: 200,
            headers: IndexMap::new(),
            chunks: body.into_iter().collect(),
            body_base64: base64,
            chunk_delay_ms: 0,
            close: None,
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn add_writes_entry() {
    let scratch = scratch("add");
    let remote = skill_remote(&scratch);
    init(&scratch);
    let add = ok(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--path",
            "skills/pdf",
        ],
    );
    assert!(
        text(&add).contains("installed  skills.pdf"),
        "{}",
        text(&add)
    );
    let manifest = read(&scratch.project.join("chest.toml"));
    let repo = remote.source.replace("git:localhost", "https://localhost");
    let repo = repo.trim_end_matches(".git");
    assert!(
        manifest.contains(&format!(
            "pdf = {{ git = \"{repo}\", rev = \"v1\", path = \"skills/pdf\" }}"
        )),
        "{manifest}"
    );
    assert!(read(&scratch.project.join("chest.lock")).contains("name = \"pdf\""));
    assert_eq!(
        read(&scratch.project.join(".claude/skills/pdf/SKILL.md")),
        "# pdf\n"
    );
    assert!(
        read(&scratch.project.join(".gitignore"))
            .contains(".agents/skills/pdf\n.claude/skills/pdf\n")
    );

    // An npm extension, granted only what --grant lists.
    let package = json!({"name": "ext", "version": "1.0.0", "description": "To\u{1b}[2J\u{1b}[Hdo\u{9b}2J", "pi": {"extensions": ["./index.js"]}, "yapi": {"capabilities": ["fs", "net"]}});
    let bytes = tarball(&[
        ("package.json", &package.to_string()),
        ("index.js", "export default () => {};\n"),
    ]);
    let server = MockServer::local_with(|base| {
        let manifest = json!({
            "name": "ext",
            "version": "1.0.0",
            "dist": {
                "tarball": format!("{base}/ext/-/ext-1.0.0.tgz"),
                "integrity": format!("sha512-{}", STANDARD.encode(digest(&SHA512, &bytes))),
            },
        });
        let packument = json!({"name": "ext", "dist-tags": {"latest": "1.0.0"}, "versions": {"1.0.0": manifest}});
        let tgz = || get("/ext/-/ext-1.0.0.tgz", None, Some(STANDARD.encode(&bytes)));
        Cassette {
            interactions: vec![get("/ext", Some(packument.to_string()), None), tgz(), tgz()],
        }
    })
    .await
    .unwrap();
    let registry = format!("{}/", server.url());
    let output = chest(
        &scratch,
        &["add", "npm:ext@^1", "--grant", "fs", "--yes"],
        &[("npm_config_registry", &registry)],
    );
    assert!(output.status.success(), "{}", text(&output));
    server.finish().unwrap();
    let said = text(&output);
    assert!(said.contains("requests: fs, net"), "{said}");
    // Control characters in package text never reach the terminal.
    assert!(said.contains("  To[2J[Hdo2J\n"), "{said:?}");
    assert!(!said.contains(['\u{1b}', '\u{9b}']), "{said:?}");
    assert!(
        said.contains("requests capabilities it is not granted: net"),
        "{said}"
    );
    let manifest = read(&scratch.project.join("chest.toml"));
    assert!(
        manifest.contains("ext = { npm = \"ext\", version = \"^1\", capabilities = [\"fs\"] }"),
        "{manifest}"
    );
    assert!(
        scratch
            .project
            .join(".yapi/chest/ext/node_modules/ext/index.js")
            .exists()
    );
}

#[test]
fn add_existing_entry() {
    let scratch = scratch("existing");
    let remote = Remote::new(&scratch.dir);
    let package = json!({"name": "todo", "pi": {"extensions": ["./index.ts"]}});
    remote.commit(
        &[
            ("package.json", &package.to_string()),
            ("index.ts", "export default () => {};\n"),
        ],
        "v1",
    );
    remote.commit(&[("index.ts", "export default () => { /* 2 */ };\n")], "v2");
    init(&scratch);
    ok(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--name",
            "todo",
            "--grant",
            "fs",
            "--yes",
        ],
    );
    // The same source again moves its rev and keeps its grants.
    ok(
        &scratch,
        &[
            "add",
            &format!("{}@v2", remote.source),
            "--name",
            "todo",
            "--yes",
        ],
    );
    let manifest = read(&scratch.project.join("chest.toml"));
    assert!(
        manifest.contains("rev = \"v2\", capabilities = [\"fs\"] }"),
        "{manifest}"
    );
    assert_eq!(
        read(&scratch.project.join(".yapi/chest/todo/index.ts")),
        "export default () => { /* 2 */ };\n"
    );
    // Another source under the same name is refused, and nothing changes.
    let clash = chest(
        &scratch,
        &["add", "npm:todo", "--name", "todo", "--yes"],
        &[],
    );
    assert_eq!(clash.status.code(), Some(1));
    assert!(
        text(&clash).contains("extensions.todo already comes from git"),
        "{}",
        text(&clash)
    );
    assert_eq!(read(&scratch.project.join("chest.toml")), manifest);
    // Granting `all` says what it means, asked or not.
    let all = ok(
        &scratch,
        &[
            "add",
            &format!("{}@v2", remote.source),
            "--name",
            "all",
            "--grant",
            "all",
            "--yes",
        ],
    );
    assert!(
        text(&all).contains("`all` is full access"),
        "{}",
        text(&all)
    );
}

#[test]
fn nested_deps_warn() {
    let scratch = scratch("nested");
    let remote = Remote::new(&scratch.dir);
    remote.commit(
        &[
            ("skills/pdf/SKILL.md", "# pdf\n"),
            (
                "chest.toml",
                "[skills]\ninner = { path = \"skills/inner\" }\n",
            ),
        ],
        "v1",
    );
    init(&scratch);
    let output = ok(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--path",
            "skills/pdf",
        ],
    );
    let said = text(&output);
    assert!(
        said.contains("lists entries of its own, which chest does not install: skills.inner"),
        "{said}"
    );
    assert!(!read(&scratch.project.join("chest.toml")).contains("inner"));
}

#[test]
fn verify_detects_tamper() {
    let scratch = scratch("verify");
    let remote = skill_remote(&scratch);
    init(&scratch);
    ok(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--path",
            "skills/pdf",
        ],
    );
    ok(&scratch, &["verify"]);
    write(
        &scratch.project.join(".agents/skills/pdf/SKILL.md"),
        "# tampered\n",
    );
    let output = chest(&scratch, &["verify"], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        text(&output).contains(".agents/skills/pdf does not match the lock"),
        "{}",
        text(&output)
    );
}

#[test]
fn ci_implies_frozen() {
    let scratch = scratch("ci");
    let remote = skill_remote(&scratch);
    remote.commit(&[("skills/pdf/SKILL.md", "# pdf 2\n")], "v2");
    init(&scratch);
    ok(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--path",
            "skills/pdf",
        ],
    );
    let manifest = scratch.project.join("chest.toml");
    write(
        &manifest,
        &read(&manifest).replace("rev = \"v1\"", "rev = \"v2\""),
    );
    let lock = read(&scratch.project.join("chest.lock"));
    for value in ["true", "1", "yes"] {
        let output = chest(&scratch, &["install"], &[("CI", value)]);
        assert_eq!(output.status.code(), Some(1), "CI={value}");
        assert!(
            text(&output).contains("is out of date for skills.pdf"),
            "{}",
            text(&output)
        );
        assert_eq!(read(&scratch.project.join("chest.lock")), lock);
    }
    let output = chest(&scratch, &["install"], &[("CI", "false")]);
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(
        read(&scratch.project.join(".agents/skills/pdf/SKILL.md")),
        "# pdf 2\n"
    );
    // An error quoting the lock prints no control characters from it.
    let lock =
        "version = 1\n\n[[extension]]\nname = \"ext\"\nnpm = \"ext\"\nversion = \"1\\u001b[2J\"\n";
    write(&scratch.project.join("chest.lock"), lock);
    let output = chest(&scratch, &["install"], &[]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output);
    assert!(said.contains("invalid version `1[2J`"), "{said:?}");
    assert!(!said.contains('\u{1b}'), "{said:?}");
}

#[test]
fn global_scope_home() {
    let scratch = scratch("global");
    let remote = skill_remote(&scratch);
    std::fs::create_dir_all(scratch.home.join(".claude")).unwrap();
    ok(&scratch, &["-g", "init", "--yes"]);
    let manifest = read(&scratch.home.join(".chest/chest.toml"));
    assert!(manifest.contains("skills = [\"~/.claude/skills\"]\nagents = [\"~/.claude/agents\"]\nextensions = \"~/.chest/store\"\n"), "{manifest}");
    ok(
        &scratch,
        &[
            "-g",
            "add",
            &format!("{}@v1", remote.source),
            "--path",
            "skills/pdf",
        ],
    );
    assert_eq!(
        read(&scratch.home.join(".claude/skills/pdf/SKILL.md")),
        "# pdf\n"
    );
    assert!(scratch.home.join(".chest/chest.lock").exists());
    assert!(scratch.home.join(".chest/state/global.toml").exists());
    assert!(!scratch.home.join(".gitignore").exists());
    assert!(
        !scratch.project.join("chest.toml").exists()
            && !scratch.project.join(".gitignore").exists()
    );
}

#[test]
fn import_settings_packages() {
    let scratch = scratch("import-settings");
    let settings = json!({"packages": [
        "npm:@acme/ext@^1",
        {"source": "git:github.com/acme/tools@v2", "skills": ["!legacy/**"]},
        {"source": "npm:@acme/picky", "extensions": ["x.ts"]},
        {"source": "npm:@acme/quiet", "autoload": false, "skills": ["+a"]},
        "../local-ext",
    ]});
    let path = scratch.project.join(".yapi/settings.json");
    write(&path, &settings.to_string());
    init(&scratch);
    let output = ok(&scratch, &["import", "--settings"]);
    let manifest = read(&scratch.project.join("chest.toml"));
    for line in [
        "ext = { npm = \"@acme/ext\", version = \"^1\", capabilities = [\"all\"] }",
        "tools = { git = \"https://github.com/acme/tools\", rev = \"v2\", capabilities = [\"all\"], skills = [\"!legacy/**\"] }",
        "local-ext = { path = \"local-ext\", capabilities = [\"all\"] }",
    ] {
        assert!(manifest.contains(line), "{line}\n{manifest}");
    }
    let said = text(&output);
    assert!(said.contains("yapi remove -l npm:@acme/ext@^1"), "{said}");
    // A filter chest cannot express leaves the package in settings.json.
    for spec in ["npm:@acme/picky", "npm:@acme/quiet"] {
        let skipped = format!(
            "warning: skipped {spec}: chest has no `extensions` filter or `autoload: false`, so it keeps loading from settings.json"
        );
        assert!(said.contains(&skipped), "{said}");
    }
    assert!(
        !manifest.contains("picky") && !manifest.contains("quiet"),
        "{manifest}"
    );
    assert_eq!(read(&path), settings.to_string());

    // A manifest that is a symbolic link is refused, so a cloned project
    // cannot point chest at a file outside it.
    #[cfg(unix)]
    {
        let (link, outside) = (
            scratch.project.join("chest.toml"),
            scratch.dir.join("outside.toml"),
        );
        std::fs::rename(&link, &outside).unwrap();
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        for args in [
            &["import", "--settings"][..],
            &["add", "../local-ext"],
            &["list"],
        ] {
            let output = chest(&scratch, args, &[]);
            assert_eq!(output.status.code(), Some(1), "{args:?}: {}", text(&output));
            assert!(
                text(&output).contains("chest.toml is a symbolic link"),
                "{}",
                text(&output)
            );
        }
        assert_eq!(read(&outside), manifest);
    }
}

#[test]
fn import_skills_lock() {
    let scratch = scratch("import-lock");
    let lock = json!({"version": 1, "skills": {
        "pdf": {"source": "anthropics/skills", "sourceType": "github", "ref": "v1", "skillPath": "skills/pdf/SKILL.md", "computedHash": "abc"},
        "docx": {"source": "acme/docs", "sourceUrl": "https://gitlab.com/acme/docs.git", "sourceType": "git", "skillPath": "docx/SKILL.md", "computedHash": "def"},
        "notes": {"source": "tools/notes", "sourceType": "local", "computedHash": "ghi"},
    }});
    write(&scratch.project.join("skills-lock.json"), &lock.to_string());
    init(&scratch);
    ok(&scratch, &["import", "--skills-lock"]);
    let manifest = read(&scratch.project.join("chest.toml"));
    for line in [
        "pdf = { git = \"github.com/anthropics/skills\", rev = \"v1\", path = \"skills/pdf\" }",
        "docx = { git = \"https://gitlab.com/acme/docs.git\", path = \"docx\" }",
        "notes = { path = \"tools/notes\" }",
    ] {
        assert!(manifest.contains(line), "{line}\n{manifest}");
    }
    assert!(!manifest.contains("abc"));
}

#[test]
fn remove_keeps_handwritten() {
    let scratch = scratch("remove");
    let remote = skill_remote(&scratch);
    init(&scratch);
    write(
        &scratch.project.join(".agents/skills/mine/SKILL.md"),
        "# mine\n",
    );
    ok(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--path",
            "skills/pdf",
        ],
    );
    let output = ok(&scratch, &["remove", "pdf"]);
    assert!(
        text(&output).contains("removed    skills.pdf"),
        "{}",
        text(&output)
    );
    for folder in [".agents/skills", ".claude/skills"] {
        assert!(
            scratch
                .project
                .join(folder)
                .join("pdf")
                .symlink_metadata()
                .is_err()
        );
    }
    assert_eq!(
        read(&scratch.project.join(".agents/skills/mine/SKILL.md")),
        "# mine\n"
    );
    assert!(!read(&scratch.project.join("chest.lock")).contains("pdf"));
    assert!(!read(&scratch.project.join(".gitignore")).contains("pdf"));
    let missing = chest(&scratch, &["remove", "pdf"], &[]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(text(&missing).contains("No entry is named pdf"));

    // Parallel adds, then parallel removes, each keep the others' changes.
    let names = ["s1", "s2", "s3", "s4"];
    for name in names {
        write(
            &scratch.dir.join(name).join("SKILL.md"),
            &format!("# {name}\n"),
        );
    }
    let parallel = |verb: &str, argument: &dyn Fn(&str) -> String| {
        let children: Vec<_> = names
            .iter()
            .map(|name| {
                command(&scratch, &[verb, &argument(name)], &[])
                    .spawn()
                    .unwrap()
            })
            .collect();
        for child in children {
            let output = child.wait_with_output().unwrap();
            assert!(output.status.success(), "{}", text(&output));
        }
    };
    parallel("add", &|name| format!("../{name}"));
    let (manifest, lock) = (
        read(&scratch.project.join("chest.toml")),
        read(&scratch.project.join("chest.lock")),
    );
    for name in names {
        let entry = format!("{name} = {{ path = \"../{name}\" }}");
        assert!(manifest.contains(&entry), "{manifest}");
        assert!(lock.contains(&format!("name = \"{name}\"")), "{lock}");
    }
    parallel("remove", &|name| name.to_owned());
    let manifest = read(&scratch.project.join("chest.toml"));
    assert!(
        !names.iter().any(|name| manifest.contains(name)),
        "{manifest}"
    );
    assert!(!scratch.project.join(".chest.toml.lock").exists());
}

#[test]
fn add_prompts_capabilities() {
    let scratch = scratch("prompts");
    let remote = Remote::new(&scratch.dir);
    let package = json!({
        "name": "risky",
        "pi": {"extensions": ["./index.ts"]},
        "yapi": {"capabilities": ["all", "process"], "events": {"send": ["risky:*"], "receive": []}},
    });
    remote.commit(
        &[
            ("package.json", &package.to_string()),
            ("index.ts", "export default () => {};\n"),
        ],
        "v1",
    );
    init(&scratch);
    // `--grant none` is the empty start, so accepting `all` writes `all` alone.
    let source = format!("{}@v1", remote.source);
    let args = ["add", &source, "--name", "risky", "--grant", "none"].map(str::to_owned);
    let mut pty = yapi_mock::pty::Pty::spawn(
        Path::new(env!("CARGO_BIN_EXE_chest")),
        &args,
        &scratch.project,
        &environment(&scratch),
        (200, 40),
        false,
    )
    .unwrap();
    let shown =
        |needle: &'static str| move |rows: &[String]| rows.iter().any(|row| row.contains(needle));
    assert!(
        pty.wait_for(Duration::from_secs(30), shown("Grant `all`"))
            .is_some(),
        "{:#?}",
        pty.rows()
    );
    // While the question waits, chest keeps the manifest's lock fresh, which
    // it refreshes every 15 seconds.
    let lock = scratch.project.join(".chest.toml.lock");
    let modified = || lock.metadata().and_then(|lock| lock.modified()).unwrap();
    let (taken, deadline) = (modified(), Instant::now() + Duration::from_secs(25));
    while modified() == taken && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(250));
    }
    assert_ne!(modified(), taken, "the lock went stale");
    // `process` is not a capability, so it is skipped rather than asked about.
    let skipped = "not capabilities, which were skipped: process";
    assert!(
        pty.rows().iter().any(|row| row.contains(skipped)),
        "{:#?}",
        pty.rows()
    );
    assert!(
        pty.rows()
            .iter()
            .any(|row| row.contains("`all` is full access")),
        "{:#?}",
        pty.rows()
    );
    pty.write("y\r").unwrap();
    assert!(
        pty.wait_for(Duration::from_secs(10), shown("3) isolated"))
            .is_some(),
        "{:#?}",
        pty.rows()
    );
    assert!(
        pty.rows()
            .iter()
            .any(|row| row.contains("2) declared channels: sends risky:*")),
        "{:#?}",
        pty.rows()
    );
    pty.write("3\r").unwrap();
    // chest prints its report once the lock and `.gitignore` are written.
    let installed = shown("installed  extensions.risky");
    assert!(
        pty.wait_for(Duration::from_secs(30), installed).is_some(),
        "{:#?}",
        pty.rows()
    );
    let _ = pty.finish();
    let manifest = read(&scratch.project.join("chest.toml"));
    assert!(
        manifest.contains("capabilities = [\"all\"], events = \"isolated\" }"),
        "{manifest}"
    );
}
