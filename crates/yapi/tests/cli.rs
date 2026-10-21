//! Command-line contract tests for the `yapi` binary.

mod common;

use std::process::Command;

#[test]
fn version_prints_bare_version() {
    for flag in ["--version", "-v"] {
        let output = Command::new(env!("CARGO_BIN_EXE_yapi"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(output.status.success(), "{flag} failed");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\n", env!("CARGO_PKG_VERSION"))
        );
    }
}

/// `--export` writes the same HTML pi `v1.0.0` does: the SHA-256 of pi's
/// export of each session fixture, made with `pi --export` in a clean
/// environment.
#[test]
fn export_matches_pi() {
    use sha2::Digest;
    let fixtures = common::fixtures().join("pi/sessions");
    let out = common::scratch("export");
    for (name, digest) in [
        (
            "main",
            "fccaf043e37ff3994301e1823521b6d23b529e809776c2db498c3ef5f83bbe37",
        ),
        (
            "branched",
            "8310b9e08a3749c9d1e5f34ada5d60b269ddeba676cd4d94dbda762ee5aebb7a",
        ),
        (
            "legacy-before-compaction",
            "b8142182de87fd3551ee16136670c00dfd53860f47d02ab70ba60146e9d5fdde",
        ),
    ] {
        let target = out.join(format!("{name}.html"));
        let output = common::yapi(&out)
            .arg("--export")
            .arg(fixtures.join(format!("{name}.jsonl")))
            .arg(&target)
            .output()
            .unwrap();
        assert!(output.status.success(), "{name}: {output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("Exported to: {}\n", target.display())
        );
        let html = std::fs::read(&target).unwrap();
        let actual: String = sha2::Sha256::digest(&html)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(actual, digest, "{name}.html differs from pi's export");
    }
    let missing = Command::new(env!("CARGO_BIN_EXE_yapi"))
        .args(["--export", "/nonexistent/session.jsonl"])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(missing.stderr).unwrap(),
        "Error: File not found: /nonexistent/session.jsonl\n"
    );
}

/// Once `-p` or `--mode` owns stdout, help and the model list go to stderr,
/// as in pi; on their own they print to stdout.
#[test]
fn metadata_on_stderr() {
    let home = common::scratch("metadata");
    let run = |args: &[&str]| {
        let output = common::yapi(&home)
            .args(args)
            .env("ANTHROPIC_API_KEY", "unused")
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        (
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };
    let (stdout, stderr) = run(&["--help"]);
    assert!(stdout.starts_with("yapi - AI coding assistant"));
    assert!(stderr.is_empty());
    for args in [&["-p", "--help"][..], &["--mode", "json", "--help"]] {
        let (stdout, stderr) = run(args);
        assert!(stdout.is_empty(), "{args:?}");
        assert!(stderr.starts_with("yapi - AI coding assistant"), "{args:?}");
    }
    let (stdout, stderr) = run(&["-p", "--list-models", "claude-sonnet-4-5"]);
    assert!(stdout.is_empty());
    assert!(stderr.contains("claude-sonnet-4-5"));
    let (stdout, _) = run(&["--list-models", "claude-sonnet-4-5"]);
    assert!(stdout.contains("claude-sonnet-4-5"));
}

/// The question whether to fork another project's session, and its
/// "Aborted.", go to stderr, so they never mix with JSON output.
#[test]
fn fork_question_on_stderr() {
    let home = common::scratch("fork-question");
    let other = home.join("agent/sessions/--tmp-yapi-pi-fixtures-project--");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(
        common::fixtures().join("pi/sessions/main.jsonl"),
        other.join("main.jsonl"),
    )
    .unwrap();
    let mut child = common::yapi(&home)
        .args([
            "--mode",
            "json",
            "--session",
            "019a0000-0000-7000-8000-000000000001",
            "hi",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write as _;
    writeln!(child.stdin.take().unwrap(), "n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("Fork this session into current directory? [y/N] "),
        "{stderr}"
    );
    assert!(stderr.ends_with("Aborted.\n"), "{stderr}");
}

/// Help lists the flags of the extensions that loaded when another fails,
/// without reporting the failure, as in pi.
#[test]
fn help_despite_extension_errors() {
    let home = common::scratch("help-extensions");
    std::fs::write(
        home.join("ext.ts"),
        "export default function (pi) {\n\tpi.registerFlag(\"mood\", { type: \"string\", description: \"Mood\" });\n}\n",
    )
    .unwrap();
    std::fs::write(
        home.join("broken.ts"),
        "export default function () {\n\tthrow new Error(\"broken\");\n}\n",
    )
    .unwrap();
    let output = common::yapi(&home)
        .args(["--help", "-e", "ext.ts", "-e", "broken.ts"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(String::from_utf8(output.stdout).unwrap().contains("--mood"));
}

/// As pi's `setRuntimeApiKey`, `--api-key` is the credential a pi-ai
/// provider's `filterModels` gets, so print mode lists the models the key
/// unlocks.
#[test]
fn runtime_key_filters() {
    let home = common::scratch("runtime-key-filters");
    std::fs::write(
        home.join("ext.ts"),
        r#"import { createProvider } from "@earendil-works/pi-ai";

export default function (pi) {
	const model = (id) => ({ id, name: id, api: "tiered-api", provider: "tiered", baseUrl: "http://127.0.0.1:9", reasoning: false, input: ["text"], cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 1000, maxTokens: 100 });
	const unused = () => {
		throw new Error("unused");
	};
	pi.registerProvider(
		createProvider({
			id: "tiered",
			auth: { apiKey: { name: "Tiered key", resolve: async ({ credential }) => (credential?.key ? { auth: { apiKey: credential.key } } : undefined) } },
			models: [model("basic"), model("pro")],
			api: { stream: unused, streamSimple: unused },
			filterModels: (models, credential) => (credential?.key === "pro" ? models : models.filter((entry) => entry.id === "basic")),
		}),
	);
	pi.registerCommand("available", {
		description: "Lists the available tiered models",
		handler: async (_args, ctx) => {
			const ids = ctx.modelRegistry.getAvailable().filter((entry) => entry.provider === "tiered").map((entry) => entry.id);
			console.error(`available: ${ids.join(",")}`);
		},
	});
}
"#,
    )
    .unwrap();
    let output = common::yapi(&home)
        .env("PI_OFFLINE", "1")
        .args([
            "-e",
            "ext.ts",
            "--model",
            "tiered/basic",
            "--api-key",
            "pro",
        ])
        .args(["--no-session", "-p", "/available"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("available: basic,pro\n"), "{stderr}");
}

/// `--enable-logging` appends extension messages to `yapi.log` in the agent
/// directory; without it, yapi writes no log.
#[test]
fn logging_flag() {
    let home = common::scratch("logging");
    std::fs::write(
        home.join("ext.ts"),
        "export default function () {\n\tconsole.error(\"loaded\");\n}\n",
    )
    .unwrap();
    let log = home.join("agent/yapi.log");
    let run = |args: &[&str]| {
        let output = common::yapi(&home).args(args).output().unwrap();
        assert!(output.status.success(), "{output:?}");
    };
    run(&["--help", "-e", "ext.ts"]);
    assert!(!log.exists());
    run(&["--enable-logging", "--help", "-e", "ext.ts"]);
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains(" ERROR yapi_ext::extensions: loaded extension="),
        "{text}"
    );
}

/// pi's RPC mode starts without any model, reporting pi-agent-core's
/// placeholder, and exits with 143 on SIGTERM even while its input stays
/// open.
#[cfg(unix)]
#[test]
fn rpc_starts_without_models() {
    use std::io::{BufRead, Write};
    let home = common::scratch("rpc-signal");
    let mut child = common::yapi(&home)
        .args(["--mode", "rpc"])
        .env("PI_OFFLINE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"id":"s","type":"get_state"}}"#).unwrap();
    let mut line = String::new();
    std::io::BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(
        line.contains(r#""model":{"id":"unknown","name":"unknown","api":"unknown""#),
        "{line}"
    );
    assert!(line.contains(r#""thinkingLevel":"off""#), "{line}");
    let killed = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(killed.success());
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "still running with stdin open"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(143));
    drop(stdin);
    let _ = std::fs::remove_dir_all(&home);
}

/// A yapi started with SIGINT ignored, as a shell starts a background job,
/// runs commands with SIGINT at its default action, as Node gives pi's.
#[cfg(unix)]
#[test]
fn commands_reset_ignored_signals() {
    use std::io::{BufRead, Write};
    let home = common::scratch("rpc-ignored-signal");
    let mut child = Command::new("/bin/sh")
        .args(["-c", r#"trap '' INT; exec "$0" --mode rpc"#])
        .arg(env!("CARGO_BIN_EXE_yapi"))
        .current_dir(&home)
        .env_clear()
        .env("HOME", &home)
        .env("YAPI_CODING_AGENT_DIR", home.join("agent"))
        .env("PI_OFFLINE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let script = "/bin/sh -c 'kill -INT $$; echo ignored'; echo done";
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"id": "b", "type": "bash", "command": script})
    )
    .unwrap();
    let response = std::io::BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(&line.unwrap()).unwrap())
        .find(|line| line["type"] == "response" && line["id"] == "b")
        .unwrap();
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(response["data"]["output"], "done\n", "{response}");
}

/// On SIGTERM, print mode kills the commands its `bash` tool is running and
/// ends the session, as pi does, instead of leaving them behind.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn print_sigterm_kills_commands() {
    let cassette = yapi_mock::Cassette::load(
        &common::fixtures().join("cassettes/anthropic-messages/sleep-tool.json"),
    )
    .unwrap();
    let server = yapi_mock::MockServer::start("127.0.0.1:0".parse().unwrap(), cassette)
        .await
        .unwrap();
    let home = common::scratch("print-signal");
    std::fs::create_dir_all(home.join("agent")).unwrap();
    let log = home.join("events.log");
    std::fs::write(
        home.join("log.ts"),
        format!(
            "import {{ appendFileSync }} from \"node:fs\";\nexport default function (pi) {{\n\tpi.on(\"session_start\", (event) => appendFileSync({log:?}, `start ${{event.reason}}\\n`));\n\tpi.on(\"session_shutdown\", (event) => appendFileSync({log:?}, `shutdown ${{event.reason}}\\n`));\n}}\n",
            log = log.display().to_string()
        ),
    )
    .unwrap();
    std::fs::write(
        home.join("agent/models.json"),
        format!(
            r#"{{"providers":{{"anthropic":{{"baseUrl":"{}"}}}}}}"#,
            server.url()
        ),
    )
    .unwrap();
    let mut child = common::yapi(&home)
        .args([
            "-p",
            "--no-session",
            "--model",
            "anthropic/claude-sonnet-4-5",
            "-e",
            "log.ts",
            "go",
        ])
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("ANTHROPIC_API_KEY", "mock")
        .env("PI_OFFLINE", "1")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let running = || {
        Command::new("pgrep")
            .args(["-f", r"^sleep 31\.7$"])
            .output()
            .is_ok_and(|output| !output.stdout.is_empty())
    };
    let started = std::time::Instant::now();
    while !running() {
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    let status = child.wait().unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let left = running();
    if left {
        let _ = Command::new("pkill")
            .args(["-f", r"^sleep 31\.7$"])
            .status();
    }
    let events = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(status.code(), Some(143));
    assert!(!left, "the command outlived yapi");
    assert_eq!(events, "start startup\nshutdown quit\n");
}

/// `list` tags each installed package by its extensions: `[npm]` for pi
/// extensions, `[wasm]` for native ones, both for a package with each, and
/// nothing for a package without extensions or one that is not installed.
#[test]
fn list_tags_extension_kinds() {
    let root = common::scratch("list");
    let files = [
        "js/index.ts",
        "native/extensions/shout.wasm",
        "mixed/extensions/a.ts",
        "mixed/extensions/b.wasm",
        "skills/skills/demo/SKILL.md",
        "filtered/index.ts",
        "single.wasm",
    ];
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }
    let agent = root.join("agent");
    std::fs::create_dir_all(&agent).unwrap();
    let path = |name: &str| root.join(name).display().to_string();
    let settings = serde_json::json!({"packages": [
        path("js"),
        path("native"),
        path("mixed"),
        path("skills"),
        {"source": path("filtered"), "extensions": []},
        path("single.wasm"),
        "npm:not-installed",
    ]});
    std::fs::write(agent.join("settings.json"), settings.to_string()).unwrap();
    let output = common::yapi(&root).arg("list").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let package = |name: &str, tag: &str| format!("  {}{tag}\n    {}\n", path(name), path(name));
    let expected = [
        "User packages:\n".to_owned(),
        package("js", " [npm]"),
        package("native", " [wasm]"),
        package("mixed", " [npm, wasm]"),
        package("skills", ""),
        format!(
            "  {} (filtered)\n    {}\n",
            path("filtered"),
            path("filtered")
        ),
        package("single.wasm", " [wasm]"),
        "  npm:not-installed\n".to_owned(),
    ]
    .concat();
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

/// `-e npm:` loads the temporary package installed in pi's folder for it, and
/// offline skips one that is not installed, as pi does.
#[test]
fn offline_temporary_packages_load() {
    let root = common::scratch("temporary-packages");
    let package = root.join("agent/tmp/extensions/npm/f35b2129/node_modules/hello-pkg");
    std::fs::create_dir_all(package.join("extensions")).unwrap();
    std::fs::write(
        package.join("package.json"),
        r#"{"name": "hello-pkg", "version": "1.0.0"}"#,
    )
    .unwrap();
    std::fs::copy(
        common::repo().join("crates/yapi-ext/tests/fixtures/hello.wasm"),
        package.join("extensions/hello.wasm"),
    )
    .unwrap();
    let commands = common::rpc_commands(common::yapi(&root).args([
        "--no-session",
        "--offline",
        "-ne",
        "-e",
        "npm:hello-pkg",
        "-e",
        "npm:not-installed",
    ]));
    let names: Vec<&str> = commands
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|command| command["name"].as_str())
        .collect();
    assert!(names.contains(&"hello"), "{names:?}");
    assert!(!root.join("agent/settings.json").exists());
}

/// pi's service and easter egg commands report that yapi does not offer them,
/// without suggesting that a later version will.
#[test]
fn no_service_commands() {
    use std::time::Duration;
    let root = common::scratch("unavailable");
    let agent = root.join("agent");
    std::fs::create_dir_all(&agent).unwrap();
    let env = [
        ("HOME", root.clone().into_os_string()),
        ("YAPI_CODING_AGENT_DIR", agent.into_os_string()),
        ("PI_OFFLINE", "1".into()),
        ("ANTHROPIC_API_KEY", "mock".into()),
    ];
    let args = ["--model", "anthropic/claude-sonnet-4-5", "--no-session"].map(str::to_owned);
    let mut pty = yapi_mock::pty::Pty::spawn(
        std::path::Path::new(env!("CARGO_BIN_EXE_yapi")),
        &args,
        &root,
        &env,
        (100, 30),
        true,
    )
    .unwrap();
    let painted = |rows: &[String]| rows.iter().any(|row| row.contains("claude-sonnet-4-5"));
    assert!(pty.wait_for(Duration::from_secs(20), painted).is_some());
    pty.write("/share\r").unwrap();
    let reported = |rows: &[String]| {
        rows.iter()
            .any(|row| row.trim_end().ends_with("/share is not available in yapi"))
    };
    assert!(
        pty.wait_for(Duration::from_secs(10), reported).is_some(),
        "{:#?}",
        pty.rows()
    );
    pty.finish().unwrap();
}

/// Input split across reads waits as in pi-tui: a startup picker reads ESC
/// and `[B` written apart as Down, not Escape, and a device attributes reply
/// in two pieces is read as the reply, not typed into the editor.
#[test]
fn split_terminal_input() {
    use std::time::Duration;
    let root = common::scratch("split-input");
    let agent = root.join("agent");
    std::fs::create_dir_all(&agent).unwrap();
    let env = [
        ("HOME", root.clone().into_os_string()),
        ("YAPI_CODING_AGENT_DIR", agent.into_os_string()),
        ("PI_OFFLINE", "1".into()),
        ("ANTHROPIC_API_KEY", "mock".into()),
        // A long escape timeout keeps the split inside it on a slow machine.
        ("PI_TUI_ESC_TIMEOUT", "2000".into()),
    ];
    let spawn = |args: &[&str], answer: bool| {
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
        let path = std::path::Path::new(env!("CARGO_BIN_EXE_yapi"));
        yapi_mock::pty::Pty::spawn(path, &args, &root, &env, (100, 30), answer).unwrap()
    };

    let mut pty = spawn(&["config"], true);
    let selected = |rows: &[String]| {
        rows.iter()
            .position(|row| row.starts_with('>') && row.contains("[x]"))
    };
    let first = pty.wait_for(Duration::from_secs(20), |rows| selected(rows).is_some());
    assert!(first.is_some(), "{:#?}", pty.rows());
    let first = selected(&pty.rows());
    pty.write("\x1b").unwrap();
    std::thread::sleep(Duration::from_millis(30));
    pty.write("[B").unwrap();
    let moved = pty.wait_for(Duration::from_secs(5), |rows| {
        selected(rows).is_some_and(|row| Some(row) > first)
    });
    assert!(moved.is_some(), "{:#?}", pty.rows());
    assert_eq!(pty.finish().unwrap(), -1);

    // Silent, so the only device attributes reply is the split one.
    let mut pty = spawn(
        &["--model", "anthropic/claude-sonnet-4-5", "--no-session"],
        false,
    );
    let shown =
        |text: &'static str| move |rows: &[String]| rows.iter().any(|row| row.contains(text));
    assert!(
        pty.wait_for(Duration::from_secs(20), shown("claude-sonnet-4-5"))
            .is_some()
    );
    pty.write("\x1b[?62;").unwrap();
    // Longer than the input buffer's 50 ms, shorter than the reply's 150 ms.
    std::thread::sleep(Duration::from_millis(100));
    pty.write("22ctyped").unwrap();
    let typed = pty.wait_for(Duration::from_secs(5), shown("typed"));
    assert!(typed.is_some(), "{:#?}", pty.rows());
    let rows = pty.rows();
    assert!(!rows.iter().any(|row| row.contains("22c")), "{rows:#?}");
    pty.finish().unwrap();
}

/// `PI_HARDWARE_CURSOR=1` shows the terminal cursor when the setting is unset,
/// and `/reload` applies the setting once it is set, as pi's
/// `applyRuntimeSettings` does.
#[test]
fn hardware_cursor_env() {
    use std::time::Duration;
    let root = common::scratch("hardware-cursor");
    let agent = root.join("agent");
    std::fs::create_dir_all(&agent).unwrap();
    let env = [
        ("HOME", root.clone().into_os_string()),
        ("YAPI_CODING_AGENT_DIR", agent.into_os_string()),
        ("PI_OFFLINE", "1".into()),
        ("ANTHROPIC_API_KEY", "mock".into()),
        ("PI_HARDWARE_CURSOR", "1".into()),
    ];
    let args = ["--model", "anthropic/claude-sonnet-4-5", "--no-session"].map(str::to_owned);
    let mut pty = yapi_mock::pty::Pty::spawn(
        std::path::Path::new(env!("CARGO_BIN_EXE_yapi")),
        &args,
        &root,
        &env,
        (100, 30),
        true,
    )
    .unwrap();
    let painted = |rows: &[String]| rows.iter().any(|row| row.contains("claude-sonnet-4-5"));
    assert!(pty.wait_for(Duration::from_secs(20), painted).is_some());
    pty.settle();
    assert!(!pty.cursor_hidden());
    std::fs::write(
        root.join("agent/settings.json"),
        r#"{"showHardwareCursor": false}"#,
    )
    .unwrap();
    pty.write("/reload\r").unwrap();
    let reloaded = |rows: &[String]| rows.iter().any(|row| row.contains("Reloaded keybindings"));
    assert!(pty.wait_for(Duration::from_secs(20), reloaded).is_some());
    pty.settle();
    assert!(pty.cursor_hidden());
    pty.finish().unwrap();
}

/// Extension console output goes to stderr in print mode. In interactive
/// mode it stays off the screen, and `/debug` writes it.
#[test]
fn extension_output_off_screen() {
    use std::time::Duration;
    let root = common::scratch("extension-output");
    let agent = root.join("agent");
    std::fs::create_dir_all(&agent).unwrap();
    std::fs::write(
        root.join("noisy.ts"),
        r#"export default function (pi) {
  console.log("loaded-line");
  pi.registerCommand("noisy", {
    description: "Logs",
    handler: async (_args, ctx) => {
      console.log("noisy-line");
      console.error("noisy-error");
      process.stderr.write("raw-stderr\n");
      console.log("é".repeat(5000));
      ctx.ui.notify("noisy done", "info");
    },
  });
}
"#,
    )
    .unwrap();
    let env = [
        ("HOME", root.clone().into_os_string()),
        ("YAPI_CODING_AGENT_DIR", agent.clone().into_os_string()),
        ("PI_OFFLINE", "1".into()),
        ("ANTHROPIC_API_KEY", "mock".into()),
    ];
    let args = [
        "-e",
        "noisy.ts",
        "--model",
        "anthropic/claude-sonnet-4-5",
        "--no-session",
    ]
    .map(str::to_owned);

    let output = common::yapi(&root)
        .args(&args)
        .args(["-p", "/noisy"])
        .envs(env.clone())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        format!(
            "loaded-line\nnoisy-line\nnoisy-error\nraw-stderr\n{}\n",
            "é".repeat(5000)
        )
    );

    let mut pty = yapi_mock::pty::Pty::spawn(
        std::path::Path::new(env!("CARGO_BIN_EXE_yapi")),
        &args,
        &root,
        &env,
        (100, 30),
        true,
    )
    .unwrap();
    let shown =
        |text: &'static str| move |rows: &[String]| rows.iter().any(|row| row.contains(text));
    assert!(
        pty.wait_for(Duration::from_secs(20), shown("claude-sonnet-4-5"))
            .is_some()
    );
    pty.write("/noisy\r").unwrap();
    assert!(
        pty.wait_for(Duration::from_secs(10), shown("noisy done"))
            .is_some()
    );
    pty.settle();
    let rows = pty.rows();
    assert!(
        !rows.iter().any(|row| ["-line", "-error", "stderr"]
            .iter()
            .any(|text| row.contains(text))),
        "{rows:#?}"
    );
    pty.write("/debug\r").unwrap();
    assert!(
        pty.wait_for(Duration::from_secs(10), shown("Debug log written"))
            .is_some()
    );
    pty.finish().unwrap();
    let log = std::fs::read_to_string(agent.join("yapi-debug.log")).unwrap();
    let (_, output) = log.split_once("\n=== Extension output ===\n").unwrap();
    let lines: Vec<&str> = output
        .lines()
        .filter_map(|line| line.split_once("noisy.ts] "))
        .map(|(_, rest)| rest)
        .collect();
    assert_eq!(
        lines[..4],
        [
            "info loaded-line",
            "info noisy-line",
            "error noisy-error",
            "warn raw-stderr"
        ],
        "{log}"
    );
    // A huge line is kept cut to 4 KiB, on a character boundary.
    let long = output.lines().nth(4).unwrap();
    let (kept, cut) = long.split_once("… (").unwrap();
    assert!((4095..=4096).contains(&kept.len()), "{}", kept.len());
    let prefix = kept.trim_end_matches('é').len();
    assert!(kept[..prefix].ends_with("noisy.ts] info "), "{kept}");
    let total = prefix + "é".repeat(5000).len();
    assert_eq!(cut, format!("{} bytes cut)", total - kept.len()));
}

#[test]
fn update_self_names_installer() {
    let dir = common::scratch("update-self");
    // pi's `--force` reinstalls pi; yapi accepts it for `update` only.
    for args in [&["update", "self"][..], &["update", "--force"]] {
        let output = common::yapi(&dir)
            .args(args)
            .env("PI_OFFLINE", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "yapi cannot update itself. Install the latest release the way you installed yapi, such as:\n  curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh\n"
        );
    }
    let output = common::yapi(&dir)
        .args(["install", "./pkg", "--force"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Unknown option --force for \"install\".\n")
    );
}

/// `yapi mcp login` signs in through the browser, here a script that follows
/// the authorization redirect to the callback, against the OAuth test server
/// in `tests/fixtures/mcp`. Needs `python3` on `PATH`.
#[cfg(unix)]
#[test]
fn mcp_login_and_logout() {
    use std::io::BufRead;
    use std::os::unix::fs::PermissionsExt;

    let home = common::scratch("mcp-login");
    let mut server = Command::new("python3")
        .arg(common::fixtures().join("mcp/server.py"))
        .args(["--http", "--oauth"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut url = String::new();
    std::io::BufReader::new(server.stdout.take().unwrap())
        .read_line(&mut url)
        .unwrap();
    let url = url.trim().to_owned();
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for name in ["open", "xdg-open"] {
        let script = bin.join(name);
        std::fs::write(
            &script,
            "#!/bin/sh\nexec python3 -c 'import sys, urllib.request; urllib.request.urlopen(sys.argv[1]).read()' \"$1\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::create_dir_all(home.join("agent")).unwrap();
    std::fs::write(
        home.join("agent/mcp.json"),
        format!("{{\"mcpServers\": {{\"demo\": {{\"url\": \"{url}\"}}}}}}\n"),
    )
    .unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |args: &[&str]| {
        let output = common::yapi(&home)
            .env("PATH", &path)
            .args(args)
            .output()
            .unwrap();
        (
            output.status.code(),
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };

    let (code, stdout, _) = run(&["mcp", "list"]);
    assert_eq!(code, Some(1));
    assert_eq!(
        stdout,
        format!(
            "demo: needs sign-in (codemode, global)\n  {url}\n  sign in with: yapi mcp login demo\n"
        )
    );
    let (code, stdout, stderr) = run(&["mcp", "login", "demo", "--timeout", "30"]);
    assert_eq!(code, Some(0), "{stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[0], "Sign in to MCP server \"demo\" in your browser:");
    assert!(
        lines[1]
            .starts_with(&url.replace("/mcp", "/authorize?response_type=code&client_id=client-1&"))
    );
    assert_eq!(lines[2..], ["Signed in to MCP server \"demo\" (9 tools)."]);
    let (code, stdout, _) = run(&["mcp", "login", "demo"]);
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        "Already signed in to MCP server \"demo\" (9 tools).\n"
    );
    let (code, stdout, _) = run(&["mcp", "list"]);
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with("demo: connected, 9 tools (codemode, global)\n"));
    assert_eq!(
        run(&["mcp", "logout", "demo"]),
        (
            Some(0),
            "Signed out of MCP server \"demo\".\n".into(),
            String::new()
        )
    );
    assert_eq!(
        run(&["mcp", "logout", "demo"]),
        (
            Some(0),
            "No stored credentials for MCP server \"demo\".\n".into(),
            String::new()
        )
    );
    assert_eq!(
        run(&["mcp", "login", "demo", "--timeout", "0"]).2,
        "--timeout must be a positive number of seconds.\n"
    );
    server.kill().unwrap();
    server.wait().unwrap();
}

/// An Anthropic Messages reply that streams `text` one character at a time.
fn streamed_text(text: &str) -> yapi_mock::Interaction {
    let event = |name: &str, data: serde_json::Value| format!("event: {name}\ndata: {data}\n\n");
    let mut chunks = vec![
        event(
            "message_start",
            serde_json::json!({"type": "message_start", "message": {"id": "msg", "type": "message",
                "role": "assistant", "model": "claude-sonnet-4-5", "content": [], "stop_reason": null,
                "stop_sequence": null, "usage": {"input_tokens": 10, "output_tokens": 1}}}),
        ),
        event(
            "content_block_start",
            serde_json::json!({"type": "content_block_start", "index": 0,
                "content_block": {"type": "text", "text": ""}}),
        ),
    ];
    for char in text.chars() {
        chunks.push(event(
            "content_block_delta",
            serde_json::json!({"type": "content_block_delta", "index": 0,
                "delta": {"type": "text_delta", "text": char.to_string()}}),
        ));
    }
    chunks.push(event(
        "content_block_stop",
        serde_json::json!({"type": "content_block_stop", "index": 0}),
    ));
    chunks.push(event(
        "message_delta",
        serde_json::json!({"type": "message_delta", "delta": {"stop_reason": "end_turn",
            "stop_sequence": null}, "usage": {"output_tokens": 5}}),
    ));
    chunks.push(event(
        "message_stop",
        serde_json::json!({"type": "message_stop"}),
    ));
    yapi_mock::Interaction {
        request: yapi_mock::RequestMatch {
            method: "POST".into(),
            path: "/v1/messages".into(),
        },
        response: yapi_mock::Response {
            status: 200,
            headers: [("content-type".to_owned(), "text/event-stream".to_owned())]
                .into_iter()
                .collect(),
            chunks,
            body_base64: None,
            chunk_delay_ms: 0,
            close: None,
        },
    }
}

/// At the end of input, RPC mode waits only for commands that have not
/// answered. A prompt answers before its run, so a run whose events an
/// extension handles stops there, as pi shuts down then, instead of
/// streaming to its end.
#[tokio::test(flavor = "multi_thread")]
async fn rpc_eof_cuts_run() {
    use std::io::Write as _;
    let text = "x".repeat(200);
    let server = yapi_mock::MockServer::local(yapi_mock::Cassette {
        interactions: vec![streamed_text(&text)],
    })
    .await
    .unwrap();
    let home = common::scratch("rpc-end-of-run");
    std::fs::create_dir_all(home.join("agent")).unwrap();
    std::fs::write(
        home.join("agent/models.json"),
        format!(
            r#"{{"providers":{{"anthropic":{{"baseUrl":"{}"}}}}}}"#,
            server.url()
        ),
    )
    .unwrap();
    // Each update keeps the runtime busy for a while.
    std::fs::write(
        home.join("follow.ts"),
        "export default function (pi) {\n\tpi.on(\"message_update\", () => {\n\t\tconst end = Date.now() + 5;\n\t\twhile (Date.now() < end) {}\n\t});\n}\n",
    )
    .unwrap();
    let mut child = common::yapi(&home)
        .args([
            "--mode",
            "rpc",
            "--no-session",
            "--model",
            "anthropic/claude-sonnet-4-5",
            "-e",
            "follow.ts",
        ])
        .env("ANTHROPIC_API_KEY", "mock")
        .env("PI_OFFLINE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"id":"p","type":"prompt","message":"hi"}}"#).unwrap();
    // The input ends while the reply streams.
    let stdout = child.stdout.take().unwrap();
    let (lines, status) = tokio::task::spawn_blocking(move || {
        use std::io::BufRead as _;
        let mut lines = Vec::new();
        let mut stdin = Some(stdin);
        for line in std::io::BufReader::new(stdout).lines() {
            let line: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            if line["type"] == "message_update" {
                stdin.take();
            }
            lines.push(line);
        }
        (lines, child.wait().unwrap())
    })
    .await
    .unwrap();
    let _ = std::fs::remove_dir_all(&home);
    assert!(
        lines
            .iter()
            .any(|line| line["type"] == "response" && line["id"] == "p" && line["success"] == true),
        "{lines:?}"
    );
    let updates = lines
        .iter()
        .filter(|line| line["type"] == "message_update")
        .count();
    assert!(updates < 100, "{updates} updates: the run went on");
    assert_eq!(status.code(), Some(0));
}

/// RPC mode exits on SIGTERM while it waits at the end of input for
/// extension code that earlier commands started.
#[cfg(unix)]
#[test]
fn rpc_sigterm_at_eof() {
    use std::io::{BufRead as _, Write as _};
    let home = common::scratch("rpc-end-signal");
    std::fs::write(
        home.join("spin.ts"),
        "export default function (pi) {\n\tpi.registerCommand(\"spin\", {\n\t\tdescription: \"Computes for a while\",\n\t\thandler: async () => {\n\t\t\tpi.sendMessage({ customType: \"spin\", content: \"started\", display: true });\n\t\t\tconst end = Date.now() + 3000;\n\t\t\twhile (Date.now() < end) {}\n\t\t},\n\t});\n}\n",
    )
    .unwrap();
    let mut child = common::yapi(&home)
        .args(["--mode", "rpc", "--no-session", "-e", "spin.ts"])
        .env("PI_OFFLINE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"id":"s","type":"prompt","message":"/spin"}}"#).unwrap();
    drop(stdin);
    // The command computes once its message is out.
    let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let started = lines
        .by_ref()
        .map(|line| serde_json::from_str::<serde_json::Value>(&line.unwrap()).unwrap())
        .any(|line| line["type"] == "message_end");
    assert!(started);
    std::thread::sleep(std::time::Duration::from_millis(300));
    Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    let status = child.wait().unwrap();
    drop(lines);
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(status.code(), Some(143));
}

/// `--project-dir` takes the project's files, its trust and the context
/// files from another directory, as if yapi started there. Tools, the
/// prompt's working directory and extensions' `ctx.cwd` stay in the working
/// directory, whose own `.yapi` never loads, trusted or not.
#[tokio::test(flavor = "multi_thread")]
async fn project_dir_flag() {
    use serde_json::json;
    let call = |id: &str, name: &str, input: serde_json::Value| json!({"type": "tool_use", "id": id, "name": name, "input": input});
    let text = |text: &str| common::reply(&[json!({"type": "text", "text": text})], "end_turn", 0);
    let server = yapi_mock::MockServer::local(yapi_mock::Cassette {
        interactions: vec![
            text("untrusted"),
            common::reply(
                &[
                    call("toolu_1", "bash", json!({"command": "pwd"})),
                    call("toolu_2", "project_tool", json!({})),
                ],
                "tool_use",
                0,
            ),
            text("approved"),
            text("cwd trusted"),
            text("project trusted"),
            common::reply(&[call("toolu_3", "cwd_tool", json!({}))], "tool_use", 0),
            text("no project dir"),
        ],
    })
    .await
    .unwrap();
    let home = common::scratch("project-dir");
    let write = |path: std::path::PathBuf, text: &str| {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        home.join("agent/models.json"),
        &json!({"providers": {"anthropic": {"baseUrl": server.url()}}}).to_string(),
    );
    let (work, project) = (home.join("work"), home.join("project"));
    for (dir, name) in [(&work, "cwd"), (&project, "project")] {
        write(dir.join("AGENTS.md"), &format!("{name}-agents-marker"));
        write(
            dir.join(".yapi/APPEND_SYSTEM.md"),
            &format!("{name}-append-marker"),
        );
        write(
            dir.join(format!(".yapi/skills/{name}-skill/SKILL.md")),
            &format!("---\nname: {name}-skill\ndescription: The {name} skill.\n---\nBody\n"),
        );
        write(
            dir.join(".yapi/extensions/marker.ts"),
            &format!(
                "export default function (pi) {{\n\tpi.registerTool({{\n\t\tname: \"{name}_tool\", label: \"{name}\", description: \"The {name} tool.\", parameters: {{ type: \"object\", properties: {{}} }},\n\t\tasync execute(_id, _params, _signal, _update, ctx) {{\n\t\t\tconst {{ projectDir }} = globalThis.__yapi.request(\"session.context\", {{}});\n\t\t\treturn {{ content: [{{ type: \"text\", text: `ctx.cwd=${{ctx.cwd}} projectDir=${{projectDir}}` }}] }};\n\t\t}},\n\t}});\n}}\n"
            ),
        );
    }
    // Only the project's settings name this skill directory.
    write(
        project.join(".yapi/settings.json"),
        r#"{"skills": ["../shared"]}"#,
    );
    write(
        project.join("shared/settings-skill/SKILL.md"),
        "---\nname: settings-skill\ndescription: The settings skill.\n---\nBody\n",
    );
    let (work, project) = (
        work.canonicalize().unwrap(),
        project.canonicalize().unwrap(),
    );
    let run = |extra: &[&str]| {
        let output = common::yapi(&work)
            .args([
                "-p",
                "--no-session",
                "--model",
                "anthropic/claude-sonnet-4-5:off",
            ])
            .args(extra)
            .arg("hi")
            .env("HOME", &home)
            .env("YAPI_CODING_AGENT_DIR", home.join("agent"))
            .env("ANTHROPIC_API_KEY", "mock")
            .env("PI_OFFLINE", "1")
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        server.requests().last().unwrap().body.clone()
    };
    let loaded = |request: &str, name: &str| {
        let markers = [
            format!("{name}-append-marker"),
            format!("{name}-skill"),
            format!("{name}_tool"),
        ];
        let found = markers.iter().filter(|marker| request.contains(*marker));
        match found.count() {
            0 => false,
            3 => true,
            _ => panic!("{name}'s project files loaded in part: {request}"),
        }
    };

    // Untrusted, the project gives its context files only.
    let project_flag = ["--project-dir", "../project"];
    let request = run(&project_flag);
    assert!(request.contains("project-agents-marker"), "{request}");
    assert!(!request.contains("cwd-agents-marker"), "{request}");
    assert!(!loaded(&request, "project") && !loaded(&request, "cwd"));
    assert!(!request.contains("settings-skill"), "{request}");

    // `--approve` trusts the project; tools run in the working directory.
    run(&[&project_flag[..], &["--approve"]].concat());
    let requests = server.requests();
    let first = &requests[1].body;
    assert!(loaded(first, "project") && !loaded(first, "cwd"), "{first}");
    assert!(first.contains("settings-skill"), "{first}");
    assert!(!first.contains("cwd-agents-marker"), "{first}");
    let shown = work.display().to_string();
    assert!(
        first.contains(&format!("<cwd>\\n{shown}\\n</cwd>")),
        "{first}"
    );
    let results: serde_json::Value = serde_json::from_str(&requests[2].body).unwrap();
    let result = |id: &str| {
        let messages = results["messages"].as_array().unwrap().iter();
        let mut blocks = messages.flat_map(|message| message["content"].as_array().unwrap());
        blocks.find(|block| block["tool_use_id"] == id).unwrap()["content"].to_string()
    };
    assert!(result("toolu_1").contains(&shown), "{results}");
    let project_shown = project.display().to_string();
    assert!(
        result("toolu_2").contains(&format!("ctx.cwd={shown} projectDir={project_shown}")),
        "{results}"
    );

    // Trusting the working directory trusts nothing else.
    let trust = |dir: &std::path::Path| {
        write(
            home.join("agent/trust.json"),
            &json!({dir.display().to_string(): true}).to_string(),
        );
    };
    trust(&work);
    let request = run(&project_flag);
    assert!(!loaded(&request, "project") && !loaded(&request, "cwd"));
    trust(&project);
    let request = run(&project_flag);
    assert!(loaded(&request, "project") && !loaded(&request, "cwd"));

    // Without the flag, the working directory is the project.
    run(&["--approve"]);
    let requests = server.requests();
    let results = &requests[requests.len() - 1].body;
    assert!(
        results.contains(&format!("ctx.cwd={shown} projectDir={shown}")),
        "{results}"
    );

    let output = common::yapi(&home)
        .args(["-p", "--project-dir", "missing", "hi"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "Error: Project directory not found: {}\n",
            home.canonicalize().unwrap().join("missing").display()
        )
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// A native extension reads the session's project as its factory runs: the
/// working directory, or the one `--project-dir` names, at startup, in a new
/// session, which keeps the runtime, and after a reload, which starts a new
/// one whose runs count from one. Without the flag, a session in another
/// directory, which starts a new runtime too, takes that directory's project,
/// in its new sessions and reloads as well.
#[test]
fn native_extensions_read_project() {
    use serde_json::{Value, json};
    use std::io::{BufRead as _, Write as _};
    let home = common::scratch("load-project-dir");
    let (work, project) = (home.join("work"), home.join("project"));
    for dir in [&work, &project] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let (work, project) = (
        work.canonicalize().unwrap(),
        project.canonicalize().unwrap(),
    );
    let elsewhere = home.join("elsewhere.jsonl");
    std::fs::write(
        &elsewhere,
        format!(
            "{}\n",
            json!({"type": "session", "version": 3, "id": "elsewhere", "timestamp": "2026-01-01T00:00:00.000Z", "cwd": project})
        ),
    )
    .unwrap();
    let fixture = common::repo().join("crates/yapi-ext/tests/fixtures/project-dir.wasm");
    // The description of `/project` after each command: the run of the
    // factory and the directory it read.
    let described = |extra: &[&str], commands: &[Value]| {
        let mut child = common::yapi(&work)
            .args(["--mode", "rpc", "--no-session", "-e"])
            .arg(&fixture)
            .args(extra)
            .env("PI_OFFLINE", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
        let mut ask = |command: &Value| {
            writeln!(stdin, "{command}").unwrap();
            loop {
                let event: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
                if event["type"] == "response" && event["id"] == command["id"] {
                    return event;
                }
            }
        };
        let mut seen = Vec::new();
        for command in [&[json!({"type": "get_commands"})][..], commands].concat() {
            let mut command = command.clone();
            command["id"] = json!(seen.len());
            ask(&command);
            // `/project` reloads after its prompt answers.
            for _ in 0..500 {
                let commands = ask(&json!({"id": "get", "type": "get_commands"}));
                let project = commands["data"]["commands"].as_array().unwrap().iter();
                let text = project
                    .filter(|it| it["name"] == "project")
                    .map(|it| it["description"].as_str().unwrap().to_owned())
                    .next()
                    .unwrap();
                if seen.last() != Some(&text) {
                    seen.push(text);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        drop(stdin);
        child.wait().unwrap();
        seen
    };
    let reload = json!({"type": "prompt", "message": "/project"});
    let new_and_reload = [json!({"type": "new_session"}), reload.clone()];
    let (work, project) = (work.display(), project.display());
    assert_eq!(
        described(&["--project-dir", "../project"], &new_and_reload),
        [
            format!("Run 1 for {project}"),
            format!("Run 2 for {project}"),
            format!("Run 1 for {project}"),
        ]
    );
    let switch = json!({"type": "switch_session", "sessionPath": elsewhere});
    assert_eq!(
        described(
            &[],
            &[&new_and_reload[..], &[switch], &new_and_reload[..]].concat()
        ),
        [
            format!("Run 1 for {work}"),
            format!("Run 2 for {work}"),
            format!("Run 1 for {work}"),
            format!("Run 1 for {project}"),
            format!("Run 2 for {project}"),
            format!("Run 1 for {project}"),
        ]
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// At startup, interactive mode checks for a newer release and for package
/// updates, as pi does, and shows a notice for each it finds, wrapped to the
/// terminal. `PI_SKIP_VERSION_CHECK` turns off the release check, and a
/// non-empty `PI_OFFLINE` or `--offline` both.
#[tokio::test(flavor = "multi_thread")]
async fn startup_shows_update_notices() {
    use std::time::Duration;
    use yapi_mock::{Cassette, Interaction, MockServer, RequestMatch, Response};
    let get = |path: &str, status: u16, location: Option<String>, body: &str| Interaction {
        request: RequestMatch {
            method: "GET".into(),
            path: path.into(),
        },
        response: Response {
            status,
            headers: location
                .map(|location| ("location".to_owned(), location))
                .into_iter()
                .collect(),
            chunks: vec![body.to_owned()],
            body_base64: None,
            chunk_delay_ms: 0,
            close: None,
        },
    };
    // The checks run at once, so releases and packages each have a server.
    let releases = |release: bool| {
        MockServer::local_with(move |base| Cassette {
            interactions: release
                .then(|| {
                    let tag = format!("{base}/releases/tag/v99.0.0");
                    get("/releases/latest", 302, Some(tag), "")
                })
                .into_iter()
                .collect(),
        })
    };
    let registry = |packument: bool| {
        let body = r#"{"name": "ext-pkg", "dist-tags": {"latest": "1.1.0"}, "versions": {"1.0.0": {}, "1.1.0": {}}}"#;
        MockServer::local(Cassette {
            interactions: packument
                .then(|| get("/ext-pkg", 200, None, body))
                .into_iter()
                .collect(),
        })
    };
    let home = common::scratch("update-notices");
    let agent = home.join("agent");
    let installed = agent.join("npm/node_modules/ext-pkg");
    std::fs::create_dir_all(&installed).unwrap();
    std::fs::write(
        installed.join("package.json"),
        r#"{"name": "ext-pkg", "version": "1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(
        agent.join("settings.json"),
        r#"{"packages": ["npm:ext-pkg"]}"#,
    )
    .unwrap();
    // The agent's own `fd`, so that startup downloads nothing.
    std::fs::create_dir_all(agent.join("bin")).unwrap();
    std::fs::write(agent.join("bin/fd"), "").unwrap();
    let start = |releases: &MockServer,
                 registry: &MockServer,
                 extra: &[(&'static str, &str)],
                 args: &[&str],
                 cols: u16| {
        let mut env = vec![
            ("HOME", home.clone().into_os_string()),
            ("YAPI_CODING_AGENT_DIR", agent.clone().into_os_string()),
            ("YAPI_NO_DOCS", "1".into()),
            // A trailing slash is harmless.
            (
                "YAPI_RELEASES_URL",
                format!("{}/releases/", releases.url()).into(),
            ),
            ("npm_config_registry", format!("{}/", registry.url()).into()),
        ];
        env.extend(extra.iter().map(|(name, value)| (*name, (*value).into())));
        let args: Vec<String> = ["--no-session"]
            .iter()
            .chain(args)
            .map(|arg| (*arg).to_owned())
            .collect();
        yapi_mock::pty::Pty::spawn(
            std::path::Path::new(env!("CARGO_BIN_EXE_yapi")),
            &args,
            &home,
            &env,
            (cols, 40),
            true,
        )
        .unwrap()
    };
    let text = |rows: Vec<String>| {
        rows.iter()
            .map(|row| row.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let shown = |texts: &'static [&'static str]| {
        move |rows: &[String]| {
            texts
                .iter()
                .all(|text| rows.iter().any(|row| row.contains(text)))
        }
    };

    let (releases_server, registry_server) =
        (releases(true).await.unwrap(), registry(true).await.unwrap());
    // pi reads an empty `PI_OFFLINE` as unset.
    let pty = start(
        &releases_server,
        &registry_server,
        &[("PI_OFFLINE", "")],
        &[],
        100,
    );
    const BOTH: &[&str] = &["Update Available", "Package Updates Available"];
    assert!(
        pty.wait_for(Duration::from_secs(20), shown(BOTH)).is_some(),
        "{:#?}",
        pty.rows()
    );
    let shown_text = text(pty.rows());
    let border = "─".repeat(100);
    let version = [
        "",
        &border,
        " Update Available",
        " New version 99.0.0 is available. Run yapi update",
        &format!(" Changelog: {}/releases", releases_server.url()),
        &border,
    ]
    .join("\n");
    let packages = [
        "",
        &border,
        " Package Updates Available",
        " Package updates are available. Run yapi update --extensions",
        " Packages:",
        " - ext-pkg",
        &border,
    ]
    .join("\n");
    assert!(shown_text.contains(&version), "{shown_text}");
    assert!(shown_text.contains(&packages), "{shown_text}");
    pty.finish().unwrap();
    releases_server.finish().unwrap();
    registry_server.finish().unwrap();

    let (releases_server, registry_server) = (
        releases(false).await.unwrap(),
        registry(true).await.unwrap(),
    );
    let pty = start(
        &releases_server,
        &registry_server,
        &[("PI_SKIP_VERSION_CHECK", "1")],
        &[],
        40,
    );
    assert!(
        pty.wait_for(
            Duration::from_secs(20),
            shown(&["Package Updates Available"])
        )
        .is_some(),
        "{:#?}",
        pty.rows()
    );
    pty.settle();
    let rows = pty.rows();
    assert!(!shown(&["Update Available"])(&rows), "{rows:#?}");
    let narrow = [
        "─".repeat(40).as_str(),
        " Package Updates Available",
        " Package updates are available. Run",
        " yapi update --extensions",
        " Packages:",
        " - ext-pkg",
    ]
    .join("\n");
    let shown_text = text(rows);
    assert!(shown_text.contains(&narrow), "{shown_text}");
    pty.finish().unwrap();
    assert!(releases_server.finish().unwrap().is_empty());
    registry_server.finish().unwrap();

    for (env, args) in [(&[("PI_OFFLINE", "1")][..], &[][..]), (&[], &["--offline"])] {
        let (releases_server, registry_server) = (
            releases(false).await.unwrap(),
            registry(false).await.unwrap(),
        );
        let pty = start(&releases_server, &registry_server, env, args, 100);
        assert!(
            pty.wait_for(Duration::from_secs(20), shown(&["No models available"]))
                .is_some(),
            "{:#?}",
            pty.rows()
        );
        pty.settle();
        let rows = pty.rows();
        assert!(!shown(&["Update Available"])(&rows), "{rows:#?}");
        assert!(!shown(&["Package Updates Available"])(&rows), "{rows:#?}");
        pty.finish().unwrap();
        assert!(releases_server.finish().unwrap().is_empty());
        assert!(registry_server.finish().unwrap().is_empty());
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// `--offline` is pi's `PI_OFFLINE=1` and `PI_SKIP_VERSION_CHECK=1` for the
/// whole process: extensions and the programs they run see both.
#[test]
fn offline_flag_propagates() {
    let home = common::scratch("offline-env");
    std::fs::create_dir_all(home.join("agent")).unwrap();
    std::fs::write(
        home.join("env.ts"),
        r#"export default function (pi) {
	pi.registerCommand("env", {
		handler: async () => {
			const child = await pi.exec("sh", ["-c", "echo $PI_OFFLINE $PI_SKIP_VERSION_CHECK"]);
			const own = `${process.env.PI_OFFLINE} ${process.env.PI_SKIP_VERSION_CHECK}`;
			pi.sendMessage({ customType: "env", content: `${own} ${child.stdout.trim()}`, display: true });
		},
	});
}
"#,
    )
    .unwrap();
    let output = common::yapi(&home)
        .args([
            "--offline",
            "--mode",
            "json",
            "--no-session",
            "-e",
            "env.ts",
            "--model",
            "anthropic/claude-sonnet-4-5",
            "/env",
        ])
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("ANTHROPIC_API_KEY", "mock")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _ = std::fs::remove_dir_all(&home);
    assert!(stdout.contains(r#""content":"1 1 1 1""#), "{stdout}");
}

/// `newSession`'s `setup` names the `SessionManager` writes yapi does not
/// make when it throws.
#[test]
fn setup_names_unsupported_writes() {
    use serde_json::{Value, json};
    use std::io::{BufRead as _, Write as _};
    let home = common::scratch("setup-writes");
    std::fs::write(
        home.join("setup.ts"),
        r#"export default function (pi) {
	pi.registerCommand("seed", {
		handler: async (_args, ctx) => {
			await ctx.newSession({
				setup: async (manager) => {
					try {
						manager.appendLabelChange("x", "y");
					} catch (error) {
						manager.appendSessionInfo(error.message);
					}
				},
			});
		},
	});
}
"#,
    )
    .unwrap();
    let mut child = common::yapi(&home)
        .args(["--mode", "rpc", "--no-session", "-e", "setup.ts"])
        .env("PI_OFFLINE", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let mut ask = |command: Value| {
        writeln!(stdin, "{command}").unwrap();
        loop {
            let event: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
            if event["type"] == "response" && event["id"] == command["id"] {
                return event;
            }
        }
    };
    ask(json!({"id": "seed", "type": "prompt", "message": "/seed"}));
    let mut name = Value::Null;
    for _ in 0..500 {
        name = ask(json!({"id": "state", "type": "get_state"}))["data"]["sessionName"].clone();
        if !name.is_null() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(stdin);
    child.wait().unwrap();
    assert_eq!(
        name,
        "sessionManager.appendLabelChange() is not available in yapi extensions"
    );
    let _ = std::fs::remove_dir_all(&home);
}
