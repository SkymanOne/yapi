//! The `chest` binary end to end, with a temporary home folder, a local git
//! remote and a mock npm registry.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::ffi::OsString;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use aws_lc_rs::digest::{SHA512, digest};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use indexmap::IndexMap;
use serde_json::json;
use yapi_ai::auth::lock::FileLock;
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
/// host, and `localhost/<path>` and `git://localhost/<path>` git sources
/// mapped to local repositories.
fn environment(scratch: &Scratch) -> Vec<(String, OsString)> {
    vec![
        ("HOME".into(), scratch.home.clone().into_os_string()),
        ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
        ("GIT_CONFIG_COUNT".into(), "2".into()),
        ("GIT_CONFIG_KEY_0".into(), "url.file:///.insteadOf".into()),
        ("GIT_CONFIG_VALUE_0".into(), "https://localhost/".into()),
        ("GIT_CONFIG_KEY_1".into(), "url.file:///.insteadOf".into()),
        ("GIT_CONFIG_VALUE_1".into(), "git://localhost/".into()),
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
    // A command waits while another holds the manifest's lock, and says so
    // once.
    let held = scratch.project.join(".chest.toml.lock");
    std::fs::create_dir(&held).unwrap();
    let waiting = command(&scratch, &["init", "--yes"], &[]).spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));
    std::fs::remove_dir(&held).unwrap();
    let again = waiting.wait_with_output().unwrap();
    assert_eq!(again.status.code(), Some(1));
    let said = text(&again);
    let wait = "Waiting for another chest command that changes";
    assert_eq!(said.matches(wait).count(), 1, "{said}");
    assert!(said.contains("chest.toml already exists"), "{said}");
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
    // A git URL is a git source without `git:`. pdf goes first, since a
    // second entry for its package and path is refused.
    ok(&scratch, &["remove", "pdf"]);
    let url = remote.source.replace("git:localhost", "git://localhost");
    let args = ["add", &url, "--path", "skills/pdf", "--name", "url"];
    ok(&scratch, &args);
    let manifest = read(&scratch.project.join("chest.toml"));
    let entry = format!(
        "url = {{ git = \"{}\", path = ",
        url.trim_end_matches(".git")
    );
    assert!(manifest.contains(&entry), "{manifest}");
    // A local folder that provides several skills installs each of them.
    for skill in ["one", "two"] {
        let folder = scratch.dir.join("pack/skills").join(skill);
        write(&folder.join("SKILL.md"), &format!("# {skill}\n"));
    }
    let said = text(&ok(&scratch, &["add", "../pack"]));
    assert!(said.contains("installed  skills.pack"), "{said}");
    // So does the folder again, or a folder inside it.
    for source in ["../pack/", "../pack/skills/one"] {
        let again = chest(&scratch, &["add", source, "--name", "pack2"], &[]);
        assert!(
            text(&again).contains("skills.pack already installs path ../pack:"),
            "{source}: {}",
            text(&again)
        );
    }
    for skill in ["one", "two"] {
        let placed = scratch.project.join(".agents/skills").join(skill);
        assert_eq!(read(&placed.join("SKILL.md")), format!("# {skill}\n"));
    }

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
    let v2 = [
        ("index.ts", "export default () => { /* 2 */ };\n"),
        ("ext-a/index.ts", "export default () => {};\n"),
    ];
    remote.commit(&v2, "v2");
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
    // `--grant` must parse, and may only repeat the grants it keeps.
    let again = |grant: &str| {
        let args = ["add", &remote.source, "--name", "todo", "--grant", grant];
        chest(&scratch, &[&args[..], &["--yes"]].concat(), &[])
    };
    for (grant, error) in [
        (
            "net",
            "extensions.todo keeps its grants, so --grant can't change them: edit its `capabilities` in chest.toml",
        ),
        ("bogus", "--grant: unknown capability \"bogus\""),
        (
            "none,fs",
            "--grant: \"none\" cannot be combined with other capabilities",
        ),
    ] {
        let output = again(grant);
        assert_eq!(output.status.code(), Some(2), "{grant}: {}", text(&output));
        assert!(text(&output).contains(error), "{grant}: {}", text(&output));
        assert_eq!(read(&scratch.project.join("chest.toml")), manifest);
    }
    assert!(again("fs").status.success());
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
    // So is the same package under another name, whatever its rev.
    let twice = chest(
        &scratch,
        &[
            "add",
            &format!("{}@v1", remote.source),
            "--name",
            "todo2",
            "--yes",
        ],
        &[],
    );
    assert_eq!(twice.status.code(), Some(1));
    let installs = format!(
        "extensions.todo already installs git {} v2: change that entry instead",
        remote
            .source
            .replace("git:", "https://")
            .trim_end_matches(".git")
    );
    assert!(text(&twice).contains(&installs), "{}", text(&twice));
    assert_eq!(read(&scratch.project.join("chest.toml")), manifest);
    // And one folder of it, which would load beside the whole package.
    let v2 = format!("{}@v2", remote.source);
    let args = ["add", &v2, "--path", "ext-a", "--name", "part", "--yes"];
    let part = chest(&scratch, &args, &[]);
    assert_eq!(part.status.code(), Some(1), "{}", text(&part));
    assert!(text(&part).contains(&installs), "{}", text(&part));
    assert_eq!(read(&scratch.project.join("chest.toml")), manifest);
    // Granting `all` says what it means, asked or not.
    ok(&scratch, &["remove", "todo"]);
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
fn add_rejects_misuse() {
    let scratch = scratch("misuse");
    let remote = skill_remote(&scratch);
    remote.commit(&[("skills/single.md", "# single\n")], "v2");
    write(&scratch.dir.join("loose/skills/single.md"), "# single\n");
    init(&scratch);
    let manifest = read(&scratch.project.join("chest.toml"));
    let git = format!("{}@v2", remote.source);
    let single =
        "is a single-file skill, and chest installs only skill folders that hold a SKILL.md";
    let not_extension = "is not an extension: extension files end in `.ts`, `.js` or `.wasm`";
    let cases: Vec<(Vec<&str>, i32, String)> = vec![
        (
            vec!["add", &git, "--path", "skills/single.md"],
            1,
            format!("skills/single.md {single}"),
        ),
        (
            vec!["add", &git, "--path", "skills", "--skill"],
            1,
            "skills.skills: skills is not a skill folder with a SKILL.md".into(),
        ),
        (
            vec!["add", &git, "--path", "skills"],
            1,
            "Cannot tell what skills provides: pass --skill, --agent or --extension".into(),
        ),
        (
            vec!["add", &git, "--path", "/etc"],
            2,
            "--path: \"/etc\" must be a folder inside the package".into(),
        ),
        (
            vec!["add", "../loose/skills/single.md"],
            1,
            format!("../loose/skills/single.md {single}"),
        ),
        (
            vec!["add", "../loose/skills/single.md", "--skill"],
            1,
            format!("../loose/skills/single.md {single}"),
        ),
        (
            vec!["add", "git://localhost/repo"],
            1,
            "git://localhost/repo is not a git repository chest can read".into(),
        ),
        (
            vec!["add", &git, "--path", "skills/pdf", "--extension"],
            1,
            "skills/pdf holds no extension. An extension folder needs an `index.ts` or `index.js`, an `extensions` folder, or a `package.json` or `chest.toml` that lists extension files it holds.".into(),
        ),
        (
            vec!["add", &git, "--path", "skills/pdf/SKILL.md", "--extension"],
            1,
            format!("skills/pdf/SKILL.md {not_extension}"),
        ),
        (
            vec!["add", "../loose/skills/single.md", "--extension"],
            1,
            format!("../loose/skills/single.md {not_extension}"),
        ),
        // Flags that conflict with the source are usage errors.
        (
            vec!["add", &git, "--rev", "v1"],
            2,
            "--rev conflicts with the revision `v2` in the source: give one of them".into(),
        ),
        (
            vec!["add", &remote.source, "--version", "1"],
            2,
            "--version conflicts with a git source: use --rev".into(),
        ),
        (
            vec!["add", "npm:pdf", "--rev", "v1"],
            2,
            "--rev conflicts with an npm source: use --version".into(),
        ),
        (
            vec!["add", "npm:pdf@1", "--version", "2"],
            2,
            "--version conflicts with the version `1` in the source: give one of them".into(),
        ),
        (
            vec!["add", "../loose", "--version", "1"],
            2,
            "--version conflicts with a path source".into(),
        ),
        (
            vec!["add", &git, "--path", "skills/pdf", "--grant", "fs"],
            2,
            "--grant is for extensions only".into(),
        ),
        (
            vec!["-g", "--manifest", "chest.toml", "list"],
            2,
            "the argument '--global' cannot be used with '--manifest <PATH>'".into(),
        ),
        // Errors name what is wrong as the person wrote it.
        (vec!["add", "./nope"], 1, "./nope does not exist".into()),
        (
            vec!["add", "foo:bar"],
            1,
            "`foo:` is not a source type chest knows".into(),
        ),
        (
            vec!["add", &git, "--path", "skills/nope"],
            1,
            format!("{git} has no skills/nope"),
        ),
        (
            vec!["add", "npm:"],
            2,
            "`npm:` needs a package name: `npm:<name>[@<range>]`".into(),
        ),
        (
            vec!["add", "npm:@1.0.0"],
            2,
            "`npm:@1.0.0` needs a package name: `npm:<name>[@<range>]`".into(),
        ),
        (
            vec!["add", "npm:@acme/", "--name", "acme"],
            2,
            "`npm:@acme/` needs a package name: `npm:<name>[@<range>]`".into(),
        ),
        (
            vec!["add", "npm:x@"],
            2,
            "`npm:x@` has an empty version: `npm:<name>[@<range>]`".into(),
        ),
    ];
    for (args, code, message) in &cases {
        let output = chest(&scratch, args, &[]);
        let said = text(&output);
        assert_eq!(output.status.code(), Some(*code), "{args:?}: {said}");
        assert!(said.contains(message.as_str()), "{args:?}: {said}");
        assert!(!said.contains(".chest/cache"), "{args:?}: {said}");
        assert_eq!(read(&scratch.project.join("chest.toml")), manifest);
    }
    // `--agent` still adds such a file as a subagent.
    ok(&scratch, &["add", "../loose/skills/single.md", "--agent"]);
    assert!(scratch.project.join(".yapi/agents/single.md").exists());
    // An entry written by hand for one fails, named inside its source.
    let repo = remote.source.trim_start_matches("git:");
    let solo = format!(
        "\n[skills]\nsolo = {{ git = \"{repo}\", rev = \"v2\", path = \"skills/single.md\" }}\n"
    );
    let manifest = read(&scratch.project.join("chest.toml")) + &solo;
    write(&scratch.project.join("chest.toml"), &manifest);
    let output = chest(&scratch, &["install"], &[]);
    let said = text(&output);
    assert_eq!(output.status.code(), Some(1), "{said}");
    assert!(
        said.contains("skills.solo: skills/single.md is not a skill folder with a SKILL.md"),
        "{said}"
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
    // `list` shows the frontmatter description of a skill or a subagent.
    let skill = "---\nname: hello\ndescription: Says hello\n---\nHi.\n";
    write(&scratch.dir.join("hello/SKILL.md"), skill);
    let agent = "---\nname: helper\ndescription: Helps out\n---\nHelp.\n";
    write(&scratch.dir.join("helper.md"), agent);
    ok(&scratch, &["add", "../hello"]);
    ok(&scratch, &["add", "../helper.md"]);
    let listed = text(&ok(&scratch, &["list"]));
    for (id, description) in [
        ("skills.hello", "Says hello"),
        ("agents.helper", "Helps out"),
    ] {
        let row = listed.lines().position(|line| line.starts_with(id));
        let next = row.and_then(|row| listed.lines().nth(row + 1));
        assert_eq!(next, Some(&*format!("    {description}")), "{listed}");
    }
    // A path entry has no hash, but verify checks where its links point.
    #[cfg(unix)]
    {
        let link = scratch.project.join(".claude/skills/hello");
        let target = std::fs::read_link(&link).unwrap();
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(target.with_file_name("pdf"), &link).unwrap();
        let output = chest(&scratch, &["verify"], &[]);
        assert_eq!(output.status.code(), Some(1));
        let wrong = ".claude/skills/hello does not link to";
        assert!(text(&output).contains(wrong), "{}", text(&output));
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(target, &link).unwrap();
        ok(&scratch, &["verify"]);
        // A link in the first folder, where chest places the copy.
        let copy = scratch.project.join(".agents/skills/pdf");
        let moved = scratch.dir.join("pdf");
        std::fs::rename(&copy, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &copy).unwrap();
        let output = chest(&scratch, &["verify"], &[]);
        assert_eq!(output.status.code(), Some(1));
        let linked = ".agents/skills/pdf is a link where chest places the copy";
        assert!(text(&output).contains(linked), "{}", text(&output));
        std::fs::remove_file(&copy).unwrap();
        std::fs::rename(&moved, &copy).unwrap();
    }
    // `list` shows the state yapi acts on at startup.
    let state_of = |scratch: &Scratch, id: &str| {
        let listed = text(&ok(scratch, &["list"]));
        let row = listed
            .lines()
            .find(|line| line.starts_with(&format!("{id}  ")))
            .unwrap_or_default();
        row.split("  ").nth(2).unwrap_or_default().to_owned()
    };
    let state = |scratch: &Scratch| state_of(scratch, "skills.pdf");
    assert_eq!(state(&scratch), "installed");
    let manifest = scratch.project.join("chest.toml");
    let pinned = read(&manifest);
    write(&manifest, &pinned.replace("skills/pdf", "skills/other"));
    assert_eq!(state(&scratch), "stale");
    write(&manifest, &pinned);
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
    // Install keeps the edited copy and says so, and how to put back the
    // locked one, which list shows too.
    let said = text(&ok(&scratch, &["install"]));
    assert!(said.contains("edited     skills.pdf"), "{said}");
    let how = ".agents/skills/pdf does not match the lock, so chest left it as it is: delete it and run `chest install` to put back the locked copy";
    assert!(said.contains(how), "{said}");
    let skill = scratch.project.join(".agents/skills/pdf");
    assert_eq!(read(&skill.join("SKILL.md")), "# tampered\n");
    assert_eq!(state(&scratch), "edited");
    std::fs::remove_dir_all(&skill).unwrap();
    let said = text(&ok(&scratch, &["install"]));
    assert!(said.contains("replayed   skills.pdf"), "{said}");
    assert_eq!(state(&scratch), "installed");

    // An edited copy of a path entry stays too, while a new entry installs,
    // and list and verify say so.
    write(&scratch.dir.join("hello/SKILL.md"), "# hello\n");
    write(&scratch.dir.join("new/SKILL.md"), "# new\n");
    ok(&scratch, &["add", "../hello"]);
    let hello = scratch.project.join(".agents/skills/hello/SKILL.md");
    write(&hello, "# mine\n");
    let added = read(&manifest).replace("[skills]\n", "[skills]\nnew = { path = \"../new\" }\n");
    write(&manifest, &added);
    let said = text(&ok(&scratch, &["install"]));
    assert!(said.contains("edited     skills.hello"), "{said}");
    assert!(said.contains("installed  skills.new"), "{said}");
    let how = ".agents/skills/hello was modified since chest installed it, so chest left it as it is: delete it and run `chest install` to put back the locked copy";
    assert!(said.contains(how), "{said}");
    assert_eq!(read(&hello), "# mine\n");
    assert_eq!(
        read(&scratch.project.join(".agents/skills/new/SKILL.md")),
        "# new\n"
    );
    assert_eq!(state_of(&scratch, "skills.hello"), "edited");
    // A deleted copy of a path entry is missing.
    std::fs::remove_dir_all(scratch.project.join(".agents/skills/new")).unwrap();
    let said = text(&chest(&scratch, &["verify"], &[]));
    assert!(said.contains(".agents/skills/new is missing"), "{said}");
    let output = chest(&scratch, &["verify"], &[]);
    assert_eq!(output.status.code(), Some(1));
    let edited = ".agents/skills/hello was modified since chest installed it";
    assert!(text(&output).contains(edited), "{}", text(&output));

    // Install and update leave path entries whose source didn't change as
    // they are.
    write(
        &scratch.dir.join("ext/index.ts"),
        "export default () => {};\n",
    );
    let added = read(&manifest) + "\n[extensions]\next = { path = \"../ext\" }\n";
    write(&manifest, &added);
    ok(&scratch, &["install"]);
    for command in ["install", "update"] {
        let said = text(&ok(&scratch, &[command]));
        for id in ["skills.new", "extensions.ext"] {
            assert!(said.contains(&format!("unchanged  {id}")), "{said}");
        }
    }
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
    // With no agent found, subagents go to yapi's agent folder, wherever
    // `YAPI_CODING_AGENT_DIR` puts it.
    let moved = scratch.home.join("moved").display().to_string();
    let args = ["-g", "init", "--yes"];
    let output = chest(&scratch, &args, &[("YAPI_CODING_AGENT_DIR", &moved)]);
    assert!(output.status.success(), "{}", text(&output));
    let manifest = read(&scratch.home.join(".chest/chest.toml"));
    assert!(
        manifest.contains("agents = [\"~/moved/agents\"]"),
        "{manifest}"
    );
    std::fs::remove_file(scratch.home.join(".chest/chest.toml")).unwrap();
    // An agent folder outside the home folder, where the global manifest
    // never installs, is left out with the reason.
    let outside = scratch.project.join("agent");
    let outside = &outside.display().to_string();
    let output = chest(&scratch, &args, &[("YAPI_CODING_AGENT_DIR", outside)]);
    assert!(output.status.success(), "{}", text(&output));
    let said = text(&output);
    let reason = format!(
        "Left out yapi's subagent folder {outside}/agents: the global manifest installs only inside your home folder"
    );
    assert!(said.contains(&reason), "{said}");
    let manifest = read(&scratch.home.join(".chest/chest.toml"));
    assert!(!manifest.contains("agents ="), "{manifest}");
    std::fs::remove_file(scratch.home.join(".chest/chest.toml")).unwrap();
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
    // `-g` ignores CHEST_MANIFEST, which names a project's manifest.
    let dev = [("CHEST_MANIFEST", "chest.dev.toml")];
    let listed = text(&chest(&scratch, &["-g", "list"], &dev));
    assert!(listed.starts_with("skills.pdf  "), "{listed}");
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
    // The project isn't trusted yet, so `yapi remove -l` needs `--approve`,
    // and a path names the package from any folder.
    let local = scratch.project.join("local-ext");
    for remove in [
        "with `yapi remove -l --approve npm:@acme/ext@^1`".to_owned(),
        format!("with `yapi remove -l --approve {}`", local.display()),
    ] {
        assert!(said.contains(&remove), "{remove}\n{said}");
    }
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
    // Messages name the file read, such as a `settings.toml` a path names.
    // Once the project is trusted, the command needs no `--approve`.
    let trust = json!({ scratch.project.to_str().unwrap(): true });
    write(
        &scratch.home.join(".yapi/agent/trust.json"),
        &trust.to_string(),
    );
    let toml =
        "packages = [\"npm:tomled\", { source = \"npm:@acme/picky\", extensions = [\"x.ts\"] }]\n";
    write(&scratch.dir.join("toml/settings.toml"), toml);
    let said = text(&ok(
        &scratch,
        &["import", "--settings", "../toml/settings.toml"],
    ));
    for message in [
        "remove it from settings.toml with `yapi remove -l npm:tomled`",
        "so it keeps loading from settings.toml",
    ] {
        assert!(said.contains(message), "{said}");
    }
    // A path names another file for either flag, and one flag is required.
    // Its local sources resolve against its own folder, as yapi resolves
    // them, also when the path is relative or a `file://` URL.
    let file_url = format!("file://{}", scratch.dir.join("filed").display());
    let other = json!({"packages": ["npm:other", "./tools/pkg", file_url]});
    write(&scratch.dir.join("other.json"), &other.to_string());
    ok(&scratch, &["import", "--settings", "../other.json"]);
    let manifest = read(&scratch.project.join("chest.toml"));
    for line in [
        "other = { npm = \"other\", capabilities = [\"all\"] }",
        "pkg = { path = \"../tools/pkg\", capabilities = [\"all\"] }",
        "filed = { path = \"../filed\", capabilities = [\"all\"] }",
    ] {
        assert!(manifest.contains(line), "{line}\n{manifest}");
    }
    // So does a bare file name in the current folder.
    write(
        &scratch.project.join("bare.json"),
        r#"{"packages": ["npm:bare"]}"#,
    );
    let said = text(&ok(&scratch, &["import", "--settings", "bare.json"]));
    let added = "Added 1 entry to chest.toml. Run `chest install` to install it.";
    assert!(said.contains(added), "{said}");
    let manifest = read(&scratch.project.join("chest.toml"));
    assert!(manifest.contains("bare = { npm = \"bare\""), "{manifest}");
    // With `-g`, the message names the global manifest and its command.
    write(
        &scratch.home.join(".yapi/agent/settings.json"),
        r#"{"packages": ["npm:mine"]}"#,
    );
    let said = text(&ok(&scratch, &["-g", "import", "--settings"]));
    let added = "Added 1 entry to ~/.chest/chest.toml. Run `chest -g install` to install it.";
    assert!(said.contains(added), "{said}");
    // A package the manifest has is skipped, also at another version, and
    // so is another package whose name it has.
    let clash = r#"{"packages": ["npm:@acme/ext@^2", "npm:@other/ext"]}"#;
    write(&scratch.project.join("clash.json"), clash);
    let said = text(&ok(&scratch, &["import", "--settings", "clash.json"]));
    for message in [
        "warning: skipped npm:@acme/ext@^2: the manifest already has it as extensions.ext",
        "warning: skipped npm:@other/ext: the manifest's extensions.ext is another package",
    ] {
        assert!(said.contains(message), "{said}");
    }
    assert_eq!(read(&scratch.project.join("chest.toml")), manifest);
    let flagless = chest(&scratch, &["import", "../other.json"], &[]);
    assert_eq!(flagless.status.code(), Some(2), "{}", text(&flagless));

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
        // So is a lock that is one, for every command that reads it.
        std::fs::remove_file(&link).unwrap();
        std::fs::rename(&outside, &link).unwrap();
        let lock = scratch.project.join("chest.lock");
        let moved = scratch.dir.join("outside.lock");
        match lock.exists() {
            true => std::fs::rename(&lock, &moved).unwrap(),
            false => write(&moved, "version = 1\n"),
        }
        std::os::unix::fs::symlink(&moved, &lock).unwrap();
        for args in [&["list"][..], &["verify"], &["install", "--frozen"]] {
            let output = chest(&scratch, args, &[]);
            assert_eq!(output.status.code(), Some(1), "{args:?}: {}", text(&output));
            assert!(
                text(&output).contains("chest.lock is a symbolic link"),
                "{}",
                text(&output)
            );
        }
    }
}

#[test]
fn import_skills_lock() {
    let scratch = scratch("import-lock");
    let lock = json!({"version": 1, "skills": {
        "pdf": {"source": "anthropics/skills", "sourceType": "github", "ref": "v1", "skillPath": "skills/pdf/SKILL.md", "computedHash": "abc"},
        "docx": {"source": "acme/docs", "sourceUrl": "https://gitlab.com/acme/docs.git", "sourceType": "git", "skillPath": "docx/SKILL.md", "computedHash": "def"},
        "notes": {"source": "tools/notes", "sourceType": "local", "computedHash": "ghi"},
        "sheets": {"source": "acme/sheets", "sourceType": "gitlab", "computedHash": "jkl"},
        "weird": {"source": "a/b", "sourceType": "npm", "computedHash": "mno"},
    }});
    write(&scratch.project.join("skills-lock.json"), &lock.to_string());
    init(&scratch);
    let said = text(&ok(&scratch, &["import", "--skills-lock"]));
    let manifest = read(&scratch.project.join("chest.toml"));
    for line in [
        "pdf = { git = \"github.com/anthropics/skills\", rev = \"v1\", path = \"skills/pdf\" }",
        "docx = { git = \"https://gitlab.com/acme/docs.git\", path = \"docx\" }",
        "notes = { path = \"tools/notes\" }",
        "sheets = { git = \"gitlab.com/acme/sheets\" }",
    ] {
        assert!(manifest.contains(line), "{line}\n{manifest}");
    }
    assert!(!manifest.contains("abc"));
    // A source type chest doesn't know is skipped, not taken for git.
    let unknown = "warning: skipped weird: chest can't install its sourceType \"npm\"";
    assert!(said.contains(unknown), "{said}");
    assert!(!manifest.contains("weird"), "{manifest}");
    // Skills match by package and folder: another folder of a package the
    // manifest has is added, and another package by a taken name isn't.
    let again = json!({"version": 1, "skills": {
        "xlsx": {"source": "anthropics/skills", "sourceType": "github", "skillPath": "skills/xlsx/SKILL.md"},
        "word": {"source": "acme/docs", "sourceUrl": "https://gitlab.com/acme/docs.git", "sourceType": "git", "skillPath": "docx/SKILL.md"},
        "pdf": {"source": "other/skills", "sourceType": "github", "skillPath": "skills/pdf/SKILL.md"},
    }});
    write(&scratch.project.join("again.json"), &again.to_string());
    let said = text(&ok(&scratch, &["import", "--skills-lock", "again.json"]));
    for message in [
        "skills.xlsx  from anthropics/skills",
        "warning: skipped word: the manifest already has it as skills.docx",
        "warning: skipped pdf: the manifest's skills.pdf is another package",
    ] {
        assert!(said.contains(message), "{said}");
    }
    assert!(!read(&scratch.project.join("chest.toml")).contains("other/skills"));
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

    // An install that waits for the manifest's lock reads the manifest once
    // it holds the lock, so it keeps what `chest add` added meanwhile, which
    // the lock already pins.
    ok(&scratch, &["add", "../s1"]);
    ok(&scratch, &["add", "../s2"]);
    let path = scratch.project.join("chest.toml");
    let added = read(&path);
    write(&path, &added.replace("s2 = { path = \"../s2\" }\n", ""));
    let held = FileLock::acquire_blocking(&scratch.project.join(".chest.toml")).unwrap();
    let mut install = command(&scratch, &["install"], &[]).spawn().unwrap();
    let mut stderr = BufReader::new(install.stderr.take().unwrap());
    let mut line = String::new();
    // Until install says it waits, or ends without waiting.
    while stderr.read_line(&mut line).unwrap() > 0 && !line.contains("Waiting") {
        line.clear();
    }
    write(&path, &added);
    drop(held);
    let output = install.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", text(&output));
    assert!(!text(&output).contains("removed"), "{}", text(&output));
    assert!(read(&scratch.project.join("chest.lock")).contains("name = \"s2\""));
    assert!(scratch.project.join(".agents/skills/s2/SKILL.md").exists());
}

#[test]
fn add_prompts_capabilities() {
    let scratch = scratch("prompts");
    let remote = Remote::new(&scratch.dir);
    let package = json!({
        "name": "risky",
        "pi": {"extensions": ["./index.ts", "./hello.wasm", "./http.wasm"]},
        "yapi": {"capabilities": ["all", "process"], "events": {"send": ["risky:*"], "receive": []}},
    });
    // It uses `session`, which the package does not request.
    std::fs::copy(fixture("hello"), remote.work.join("hello.wasm")).unwrap();
    std::fs::write(remote.work.join("http.wasm"), http_component(0)).unwrap();
    remote.commit(
        &[
            ("package.json", &package.to_string()),
            ("index.ts", "export default () => {};\n"),
        ],
        "v1",
    );
    init(&scratch);
    // `--grant none` is the empty start, so `none` is never written.
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
    // it refreshes every 15s.
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
    let uses = "hello.wasm uses: session (SDK feature `session`)";
    assert!(
        pty.rows().iter().any(|row| row.contains(uses)),
        "{:#?}",
        pty.rows()
    );
    for warning in [
        "warning: http.wasm imports `wasi:http/outgoing-handler@0.2.6`, which yapi does not support. Network access goes through the SDK's `fetch`.",
        "warning: http.wasm declares the capability `exec:git`, which this version of chest does not know, so it is not checked",
    ] {
        assert!(
            pty.rows().iter().any(|row| row.contains(warning)),
            "{:#?}",
            pty.rows()
        );
    }
    // Declining `all` leaves the capability the extension uses on offer.
    pty.write("n\r").unwrap();
    assert!(
        pty.wait_for(Duration::from_secs(10), shown("Grant `session`"))
            .is_some(),
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
        manifest.contains("capabilities = [\"session\"], events = \"isolated\" }"),
        "{manifest}"
    );
}

/// A native extension's build in yapi-ext's fixtures.
fn fixture(name: &str) -> PathBuf {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../yapi-ext/tests/fixtures");
    Path::new(dir).join(format!("{name}.wasm"))
}

/// `bytes` after their LEB128 length.
fn sized(bytes: &[u8]) -> Vec<u8> {
    let (mut size, mut length) = (bytes.len(), Vec::new());
    while size >= 0x80 {
        length.push((size as u8) | 0x80);
        size >>= 7;
    }
    length.push(size as u8);
    [&length[..], bytes].concat()
}

/// An empty component that imports `wasi:http`, which yapi does not link,
/// and declares `exec:git`, a capability chest does not know, then `extra`
/// more of each.
fn http_component(extra: u8) -> Vec<u8> {
    let mut imports = vec![extra + 1];
    let mut lines = String::from("yapi-extension-api\nexec:git\n");
    for n in 0..=extra {
        let name = match n {
            0 => "wasi:http/outgoing-handler@0.2.6".to_owned(),
            n => format!("wasi:http/h{n}@0.2.6"),
        };
        imports.extend([&[0x00][..], &sized(name.as_bytes()), &[0x05, 0x00]].concat());
        if n > 0 {
            lines.push_str(&format!("exec:{n}\n"));
        }
    }
    // The component header, a type section with one empty instance type,
    // and the id of the import section.
    let head = b"\0asm\x0d\x00\x01\x00\x07\x03\x01\x42\x00\x0a";
    let custom = [&sized(b"yapi:capabilities")[..], lines.as_bytes()].concat();
    [&head[..], &sized(&imports), &[0x00], &sized(&custom)].concat()
}

#[test]
fn chest_check_mismatch() {
    let scratch = scratch("check");
    let empty = chest(&scratch, &["check"], &[]);
    assert_eq!(empty.status.code(), Some(1), "{}", text(&empty));
    let none = format!(
        "{} has no chest.toml or package.json, and no extensions to check",
        scratch.project.display()
    );
    assert!(text(&empty).contains(&none), "{}", text(&empty));
    let package = scratch.project.join("chest.toml");
    write(
        &package,
        "[package]\nname = \"probe\"\ncapabilities = [\"net\"]\nextensions = [\"extensions/hello.wasm\"]\n",
    );
    std::fs::create_dir_all(scratch.project.join("extensions")).unwrap();
    std::fs::copy(
        fixture("hello"),
        scratch.project.join("extensions/hello.wasm"),
    )
    .unwrap();
    let mismatch = chest(&scratch, &["check"], &[]);
    assert_eq!(mismatch.status.code(), Some(1), "{}", text(&mismatch));
    let report = text(&mismatch);
    assert!(
        report.contains("error: extensions/hello.wasm uses `session` (SDK feature `session`), which the package does not request"),
        "{report}"
    );
    assert!(
        report.contains("warning: the package requests `net`, which no extension uses"),
        "{report}"
    );

    write(
        &package,
        "[package]\nname = \"probe\"\ncapabilities = [\"session\"]\nextensions = [\"extensions/hello.wasm\", \"index.ts\"]\n",
    );
    write(
        &scratch.project.join("index.ts"),
        "export default () => {};\n",
    );
    let report = text(&ok(&scratch, &["check", "."]));
    assert!(
        report.contains("extensions/hello.wasm uses `session` (SDK feature `session`)"),
        "{report}"
    );
    assert!(
        report
            .contains("note: index.ts is a JS or TS extension, which yapi checks only as it runs"),
        "{report}"
    );
    // A `.wasm` file is compared with the package that holds it.
    let report = text(&ok(&scratch, &["check", "extensions/hello.wasm"]));
    assert_eq!(
        report,
        "hello.wasm uses `session` (SDK feature `session`)\n"
    );
    // A `[package]` that does not parse is an error, not an empty request.
    write(&package, "[package]\nauthor = \"Ada\"\n");
    let broken = chest(&scratch, &["check", "extensions/hello.wasm"], &[]);
    assert_eq!(broken.status.code(), Some(1), "{}", text(&broken));
    assert!(
        text(&broken).contains("unknown field `author`"),
        "{}",
        text(&broken)
    );

    // `yapi new`'s template, whose source is the `hello` example, passes.
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../yapi/templates/extension");
    let created = scratch.dir.join("created");
    for file in ["chest.toml", "package.json"] {
        let text = read(&template.join(file)).replace("{{crate_name}}", "hello");
        write(&created.join(file), &text);
    }
    std::fs::create_dir_all(created.join("extensions")).unwrap();
    std::fs::copy(fixture("hello"), created.join("extensions/hello.wasm")).unwrap();
    let report = text(&ok(&scratch, &["check", created.to_str().unwrap()]));
    assert_eq!(
        report,
        "extensions/hello.wasm uses `session` (SDK feature `session`)\n"
    );
    // Its manifests stand for it.
    for manifest in ["package.json", "chest.toml"] {
        let path = created.join(manifest);
        let output = ok(&scratch, &["check", path.to_str().unwrap()]);
        assert_eq!(text(&output), report, "{manifest}");
    }
    // Another file is not an extension.
    write(&created.join("stray.json"), "{}\n");
    let stray = created.join("stray.json");
    let output = chest(&scratch, &["check", stray.to_str().unwrap()], &[]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    let expected = format!(
        "{} is not an extension: extension files end in `.ts`, `.js` or `.wasm`",
        stray.display()
    );
    assert!(text(&output).contains(&expected), "{}", text(&output));

    // A component built without the SDK shows only its WASI imports.
    let bare = scratch.dir.join("bare.wasm");
    std::fs::write(&bare, b"\0asm\x0d\x00\x01\x00").unwrap();
    let report = text(&ok(&scratch, &["check", bare.to_str().unwrap()]));
    assert!(
        report.contains("warning: bare.wasm was built without capability metadata"),
        "{report}"
    );
    let http = scratch.dir.join("http.wasm");
    std::fs::write(&http, http_component(0)).unwrap();
    let failed = chest(&scratch, &["check", http.to_str().unwrap()], &[]);
    assert_eq!(failed.status.code(), Some(1), "{}", text(&failed));
    assert!(
        text(&failed).contains("error: http.wasm imports `wasi:http/outgoing-handler@0.2.6`, which yapi does not support. Network access goes through the SDK's `fetch`."),
        "{}",
        text(&failed)
    );
    assert!(
        text(&failed).contains("warning: http.wasm declares the capability `exec:git`, which this version of chest does not know, so it is not checked"),
        "{}",
        text(&failed)
    );
    // A component's own lists print ten lines each, then a count.
    std::fs::write(&http, http_component(20)).unwrap();
    let failed = text(&chest(&scratch, &["check", http.to_str().unwrap()], &[]));
    for (line, more) in [
        (
            "which yapi does not support",
            "and 11 more unsupported imports in http.wasm",
        ),
        (
            "which this version of chest does not know",
            "and 11 more unknown capability lines in http.wasm",
        ),
    ] {
        assert_eq!(failed.matches(line).count(), 10, "{failed}");
        assert!(failed.contains(more), "{failed}");
    }
    let module = scratch.dir.join("module.wasm");
    std::fs::write(&module, b"\0asm\x01\x00\x00\x00").unwrap();
    let failed = chest(&scratch, &["check", module.to_str().unwrap()], &[]);
    assert_eq!(failed.status.code(), Some(1), "{}", text(&failed));
    assert!(
        text(&failed).contains("not a component"),
        "{}",
        text(&failed)
    );
}
