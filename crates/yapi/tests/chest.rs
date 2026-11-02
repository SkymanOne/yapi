//! chest manifests at yapi's startup: installed extensions run with their
//! manifest's grants and event policy, behind project trust, and placed
//! skills load wherever the manifest puts them.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use yapi_core::chest::{Mode, Project};
use yapi_core::packages::npm::Npm;
use yapi_core::settings::Scope;

/// Writes `files` under `home`, then manifest `name` with `text`, and
/// installs it as `chest install` does, with chest's state in `home`, as
/// yapi started there finds it. `.chest/chest.toml` is the user's global
/// manifest.
async fn install(home: &Path, files: &[(&str, &str)], name: &str, text: &str) {
    for (path, content) in files {
        let path = home.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    let path = home.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    let scope = match name {
        ".chest/chest.toml" => Scope::Global,
        _ => Scope::Project,
    };
    let mut project = Project::open(&path, scope).unwrap();
    project.cache = home.join(".chest/cache");
    project.state = home.join(".chest/state");
    let mut npm = Npm::new(HashMap::new());
    project
        .sync(Mode::Resolve, &mut npm, &mut |_| {})
        .await
        .unwrap();
}

/// An extension that registers command `name`, described by whether it
/// could read `secret` as it loaded.
fn reader(name: &str, secret: &Path) -> String {
    format!(
        "import {{ readFileSync }} from \"node:fs\";\nlet result;\ntry {{ readFileSync({:?}, \"utf8\"); result = \"read\"; }} catch {{ result = \"refused\"; }}\nexport default function (pi) {{ pi.registerCommand({name:?}, {{ description: result, handler: async () => {{}} }}); }}\n",
        secret.display().to_string()
    )
}

/// The commands yapi in `home` with `args` and `env` registers, by name, with
/// their descriptions.
fn commands(home: &Path, args: &[&str], env: &[(&str, &str)]) -> HashMap<String, String> {
    let mut command = common::yapi(home);
    command.args(args).envs(env.iter().copied());
    common::rpc_commands(&mut command)
        .as_array()
        .unwrap()
        .iter()
        .map(|command| {
            let text = |key: &str| command[key].as_str().unwrap_or_default().to_owned();
            (text("name"), text("description"))
        })
        .collect()
}

/// yapi in RPC mode, with its events and stderr.
struct Rpc {
    child: Child,
    events: mpsc::Receiver<Value>,
    stderr: Arc<Mutex<Vec<String>>>,
}

impl Rpc {
    fn start(command: &mut Command) -> Rpc {
        let mut child = command
            .args(["--mode", "rpc"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (sender, events) = mpsc::channel();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                if let Ok(event) = serde_json::from_str(&line) {
                    let _ = sender.send(event);
                }
            }
        });
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let lines = BufReader::new(child.stderr.take().unwrap());
        let sink = Arc::clone(&stderr);
        std::thread::spawn(move || {
            for line in lines.lines().map_while(Result::ok) {
                sink.lock().unwrap().push(line);
            }
        });
        Rpc {
            child,
            events,
            stderr,
        }
    }

    /// Runs command `text` and answers the message it shows.
    fn notice(&mut self, text: &str) -> String {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{}", json!({"type": "prompt", "message": text})).unwrap();
        loop {
            let event = self
                .events
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("no notice from {text}"));
            if event["type"] == "extension_ui_request" && event["method"] == "notify" {
                return event["message"].as_str().unwrap_or_default().to_owned();
            }
        }
    }

    fn stderr(&self) -> Vec<String> {
        self.stderr.lock().unwrap().clone()
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A chest extension runs with its manifest's grants: without `fs` it cannot
/// read a file, with it it can. Both load from their install root, and so
/// do a global manifest's, a patched one's and a package's `index.js` that
/// no manifest lists. The grants follow the file when `-e` or discovery
/// loads it too, even by another path, with a warning. An entry whose folder holds the agent
/// folder does not load.
#[tokio::test(flavor = "multi_thread")]
async fn chest_extensions_load() {
    let home = common::scratch("chest-load");
    let secret = home.join("secret.txt");
    std::fs::write(&secret, "s3cret").unwrap();
    let ext = |name: &str| reader(name, &secret);
    let manifest = "[extensions]\nclosed = { path = \"closed.js\" }\nopen = { path = \"open.js\", capabilities = [\"fs\"] }\nfound = { path = \".yapi/extensions/found.js\" }\npatched = { git = \"localhost/nowhere/repo\", rev = \"v1\" }\nwide = { path = \"agent\" }\nindex = { path = \"index\" }\nlinked = { path = \"linked\" }\n\n[patch]\npatched = { path = \"patched.js\" }\n";
    let wide = r#"{"name": "wide", "pi": {"extensions": ["wide.js"]}}"#;
    let files = [
        ("closed.js", ext("closed")),
        ("open.js", ext("open")),
        (".yapi/extensions/found.js", ext("found")),
        ("patched.js", ext("patched")),
        ("agent/package.json", wide.to_owned()),
        ("agent/wide.js", ext("wide")),
        ("index/package.json", r#"{"name": "index"}"#.to_owned()),
        ("index/index.js", ext("index")),
        ("linked/index.js", ext("linked")),
    ];
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (*path, text.as_str()))
        .collect();
    install(&home, &files, "chest.toml", manifest).await;
    // Discovery names `linked` by another path, which takes precedence.
    #[cfg(unix)]
    std::os::unix::fs::symlink("../../linked", home.join(".yapi/extensions/linked")).unwrap();
    let (global, granted) = (ext("global"), ext("granted"));
    let files = [
        (".chest/global.js", global.as_str()),
        (".chest/granted.js", granted.as_str()),
    ];
    let manifest = "[extensions]\nglobal = { path = \"global.js\" }\ngranted = { path = \"granted.js\", capabilities = [\"fs\"] }\n";
    install(&home, &files, ".chest/chest.toml", manifest).await;
    let args = ["--approve", "-e", "closed.js"];
    let loaded = commands(&home, &args, &[]);
    let described = |name: &str| loaded.get(name).map(String::as_str);
    let mut refused = vec!["closed", "found", "patched", "global", "index"];
    refused.extend(cfg!(unix).then_some("linked"));
    for name in refused {
        assert_eq!(described(name), Some("refused"), "{name}: {loaded:?}");
    }
    for name in ["open", "granted"] {
        assert_eq!(described(name), Some("read"), "{name}: {loaded:?}");
    }
    assert_eq!(described("wide"), None, "{loaded:?}");
    let output = common::yapi(&home)
        .args(args)
        .arg("--help")
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let mut warnings = vec![
        "closed.js is also loaded from cli; chest's grants apply",
        "found.js is also loaded from auto; chest's grants apply",
        "agent holds more than the extension",
    ];
    let linked = "extensions/linked/index.js is also loaded from auto; chest's grants apply";
    warnings.extend(cfg!(unix).then_some(linked));
    for warning in warnings {
        assert_eq!(stderr.matches(warning).count(), 1, "{warning}: {stderr}");
    }
}

/// An untrusted project's manifest loads nothing.
#[tokio::test(flavor = "multi_thread")]
async fn untrusted_chest_ignored() {
    let home = common::scratch("chest-untrusted");
    let manifest = "[extensions]\nopen = { path = \"open.js\", capabilities = [\"fs\"] }\n";
    let open = reader("open", &home.join("missing.txt"));
    install(&home, &[("open.js", &open)], "chest.toml", manifest).await;
    assert!(commands(&home, &["--approve"], &[]).contains_key("open"));
    let loaded = commands(&home, &[], &[]);
    assert!(!loaded.contains_key("open"), "{loaded:?}");
}

/// An entry whose lock is stale is left out with a warning that names
/// `chest install`, and the rest of the run goes on.
#[tokio::test(flavor = "multi_thread")]
async fn stale_lock_warns() {
    let home = common::scratch("chest-stale");
    let manifest = "[extensions]\nopen = { path = \"open.js\" }\n";
    let open = reader("open", &home.join("missing.txt"));
    let files = [("open.js", open.as_str()), ("moved.js", open.as_str())];
    install(&home, &files, "chest.toml", manifest).await;
    std::fs::write(
        home.join("chest.toml"),
        manifest.replace("open.js", "moved.js"),
    )
    .unwrap();
    let output = common::yapi(&home)
        .args(["--approve", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("whose lock is stale, were skipped: run `chest install`"),
        "{stderr}"
    );
    assert!(!commands(&home, &["--approve"], &[]).contains_key("open"));
}

/// A project whose manifest only describes it as a package needs no trust,
/// while one that installs anything does.
#[tokio::test(flavor = "multi_thread")]
async fn package_only_untrusted() {
    let home = common::scratch("chest-package");
    std::fs::write(
        home.join("probe.js"),
        "export default function (pi) { pi.registerCommand(\"trusted\", { description: \"\", handler: async (_args, ctx) => ctx.ui.notify(String(ctx.isProjectTrusted())) }); }\n",
    )
    .unwrap();
    let trusted = |manifest: &str| {
        std::fs::write(home.join("chest.toml"), manifest).unwrap();
        let mut rpc = Rpc::start(common::yapi(&home).args(["-e", "probe.js"]));
        rpc.notice("/trusted")
    };
    assert_eq!(trusted("[package]\nname = \"demo\"\n"), "true");
    // Nor does chest warn of it, even in a trusted project.
    let output = common::yapi(&home)
        .args(["--approve", "--help"])
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("chest.toml"), "{stderr}");
    assert_eq!(
        trusted("[extensions]\nopen = { path = \"probe.js\" }\n"),
        "false"
    );
}

/// `pi.events` from a chest extension reach a `yapi install` package with
/// every grant by default. With `events = { send = [] }` they do not, and the
/// listening package's miss is warned of.
#[tokio::test(flavor = "multi_thread")]
async fn chest_events_linked() {
    const EMITTER: &str = "export default function (pi) { pi.registerCommand(\"emit\", { description: \"\", handler: async (_args, ctx) => { pi.events.emit(\"chest:ping\", {}); ctx.ui.notify(\"emitted\"); } }); }\n";
    const LISTENER: &str = r#"let heard = false;
const waiters = [];
export default function (pi) {
	pi.events.on("chest:ping", () => { heard = true; waiters.splice(0).forEach((wake) => wake()); });
	pi.registerCommand("check", { description: "", handler: async (_args, ctx) => {
		if (!heard) await new Promise((wake) => { waiters.push(wake); setTimeout(wake, 2000); });
		ctx.ui.notify(heard ? "heard" : "missed");
	} });
}
"#;
    for (events, expected) in [("", "heard"), (", events = { send = [] }", "missed")] {
        let home = common::scratch(&format!("chest-events-{expected}"));
        let manifest = format!(
            "[extensions]\nemitter = {{ path = \"emitter.js\", capabilities = [\"fs\"]{events} }}\n"
        );
        install(&home, &[("emitter.js", EMITTER)], "chest.toml", &manifest).await;
        let listener = home.join("listener.js");
        std::fs::write(&listener, LISTENER).unwrap();
        std::fs::create_dir_all(home.join("agent")).unwrap();
        let settings = json!({"packages": [listener.display().to_string()]});
        std::fs::write(home.join("agent/settings.json"), settings.to_string()).unwrap();
        let mut rpc = Rpc::start(common::yapi(&home).arg("--approve"));
        assert_eq!(rpc.notice("/emit"), "emitted");
        assert_eq!(rpc.notice("/check"), expected);
        let missed = rpc
            .stderr()
            .iter()
            .any(|line| line.contains("add \"chest:ping\" to events.send of"));
        assert_eq!(missed, expected == "missed", "{:?}", rpc.stderr());
    }
}

/// A skill chest places only in `.claude/skills`, which yapi does not read
/// on its own, still reaches yapi, and so does a patched one, which has no
/// lock entry.
#[tokio::test(flavor = "multi_thread")]
async fn chest_skills_loaded() {
    let home = common::scratch("chest-skills");
    let skill = |name: &str| format!("---\nname: {name}\ndescription: Shows {name}\n---\nDo it.\n");
    let manifest = "[folders]\nskills = [\".claude/skills\"]\n\n[skills]\ndemo = { path = \"skills/demo\" }\npatched = { git = \"localhost/nowhere/repo\", rev = \"v1\", path = \"skills/patched\" }\n\n[patch]\npatched = { path = \"local/patched\" }\n";
    let (demo, patched) = (skill("demo"), skill("patched"));
    let files = [
        ("skills/demo/SKILL.md", demo.as_str()),
        ("local/patched/SKILL.md", patched.as_str()),
    ];
    install(&home, &files, "chest.toml", manifest).await;
    assert!(home.join(".claude/skills/demo/SKILL.md").exists());
    let loaded = commands(&home, &["--approve"], &[]);
    for name in ["skill:demo", "skill:patched"] {
        assert!(loaded.contains_key(name), "{name}: {loaded:?}");
    }
}

/// `CHEST_MANIFEST` names the project manifest yapi reads, inside the
/// project.
#[tokio::test(flavor = "multi_thread")]
async fn chest_manifest_env() {
    let home = common::scratch("chest-env");
    let missing = home.join("missing.txt");
    let (main, dev) = (reader("main", &missing), reader("dev", &missing));
    install(
        &home,
        &[("main.js", &main)],
        "chest.toml",
        "[extensions]\nmain = { path = \"main.js\" }\n",
    )
    .await;
    install(
        &home,
        &[("dev.js", &dev)],
        "chest.dev.toml",
        "[extensions]\ndev = { path = \"dev.js\" }\n",
    )
    .await;
    let names = |env: &[(&str, &str)]| -> Vec<String> {
        let mut names: Vec<String> = commands(&home, &["--approve"], env)
            .into_keys()
            .filter(|name| name == "main" || name == "dev")
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(&[]), ["main"]);
    assert_eq!(names(&[("CHEST_MANIFEST", "chest.dev.toml")]), ["dev"]);
    // A name outside the project falls back to `chest.toml`, with a warning.
    let outside = home.join("chest.dev.toml").display().to_string();
    assert_eq!(names(&[("CHEST_MANIFEST", &outside)]), ["main"]);
    let output = common::yapi(&home)
        .args(["--approve", "--help"])
        .env("CHEST_MANIFEST", &outside)
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("CHEST_MANIFEST must name a file inside the project"),
        "{stderr}"
    );
}
