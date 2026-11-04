//! `pi.events` across runtimes: Pi extensions and native extensions, each
//! runtime in an instance of its own, hear each other's events.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Notes, cli_source, custom_entries, engine, eventually, options, scratch, session};
use serde_json::{Value, json};
use yapi_ai::faux::Faux;
use yapi_core::agent_session::AgentSession;
use yapi_core::extensions::{EventPolicy, Mode};
use yapi_ext::{Engine, ExtensionHost, Grants};

/// Records what it hears on `my:notification`: `heard` until `/stop`,
/// `all` always.
const LISTENER: &str = r#"
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	let heard = 0;
	const stop = pi.events.on("my:notification", (data) => {
		heard++;
		pi.appendEntry("heard", data);
	});
	pi.events.on("my:notification", (data) => pi.appendEntry("all", data));
	pi.registerCommand("tell", {
		description: "Emits my:notification",
		handler: async (args) => {
			pi.events.emit("my:notification", { message: args, from: "js" });
			pi.appendEntry("heard-during-emit", heard);
		},
	});
	pi.registerCommand("stop", { description: "Stops heard", handler: async () => stop() });
}
"#;

async fn js(engine: &Engine, dir: &Path, name: &str, source: &str) -> Arc<ExtensionHost> {
    let path = dir.join(format!("{name}.ts"));
    std::fs::write(&path, source).unwrap();
    let host = ExtensionHost::load(engine, options(dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    host
}

fn messages(entries: Vec<Value>) -> Vec<String> {
    entries
        .iter()
        .map(|data| {
            format!(
                "{}: {}",
                data["from"].as_str().unwrap(),
                data["message"].as_str().unwrap()
            )
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn cross_runtime_events() {
    let dir = scratch("bus");
    let engine = engine();
    let listener = js(&engine, &dir, "listener", LISTENER).await;
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/event-bus.wasm");
    let native = ExtensionHost::load_native(&engine, options(&dir), &cli_source(&example))
        .await
        .unwrap();
    let extensions = [listener.for_session(), native.for_session()].concat();
    let session = session(&Faux::new([]), &dir, extensions);
    let notes = Arc::new(Notes::default());
    session
        .bind_extensions(notes.clone(), Mode::Print, None, None)
        .await;
    let heard = || messages(custom_entries(&session, "heard"));

    // The emitter's own listener runs during `emit`, the other runtime's after.
    assert_eq!(
        notes.all(),
        ["Event from event-bus-example: Session started"]
    );
    eventually("the start", || {
        heard() == ["event-bus-example: Session started"]
    })
    .await;
    session.prompt("/emit hi", Vec::new()).await.unwrap();
    assert_eq!(notes.all()[1], "Event from /emit command: hi");
    eventually("hi", || heard().len() == 2).await;
    assert_eq!(heard()[1], "/emit command: hi");

    session.prompt("/tell yo", Vec::new()).await.unwrap();
    assert_eq!(custom_entries(&session, "heard-during-emit"), [json!(3)]);
    eventually("yo", || notes.all().len() == 3).await;
    assert_eq!(notes.all()[2], "Event from js: yo");

    // A Pi listener that unsubscribed hears nothing more.
    session.prompt("/stop", Vec::new()).await.unwrap();
    session.prompt("/emit later", Vec::new()).await.unwrap();
    eventually("later", || {
        messages(custom_entries(&session, "all"))
            .last()
            .map(String::as_str)
            == Some("/emit command: later")
    })
    .await;
    assert_eq!(heard().len(), 3);

    // Nor does a native one, until it subscribes again.
    session.prompt("/mute", Vec::new()).await.unwrap();
    let extensions = session.extensions();
    let native = &extensions[1];
    let calls = native.settle().await;
    session.prompt("/tell quiet", Vec::new()).await.unwrap();
    // The event's call has run once the native runtime has started one more.
    for _ in 0..1000 {
        if native.settle().await > calls {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    session.prompt("/mute", Vec::new()).await.unwrap();
    session.prompt("/tell loud", Vec::new()).await.unwrap();
    eventually("loud", || notes.all().len() == 5).await;
    assert_eq!(notes.all()[4], "Event from js: loud");
}

#[tokio::test(flavor = "multi_thread")]
async fn busy_runtime_queue_bounded() {
    const BUSY: &str = r#"
export default function (pi) {
	let ticks = 0;
	pi.events.on("tick", () => ticks++);
	pi.registerCommand("busy", {
		description: "Computes for a while",
		handler: async () => {
			const end = Date.now() + 1500;
			while (Date.now() < end) {}
		},
	});
	pi.registerCommand("count", { description: "Records the ticks", handler: async () => pi.appendEntry("ticks", ticks) });
}
"#;
    const BURST: &str = r#"
export default function (pi) {
	pi.registerCommand("burst", {
		description: "Emits many ticks",
		handler: async () => {
			for (let i = 0; i < 1000; i++) pi.events.emit("tick", i);
		},
	});
}
"#;
    let dir = scratch("bus-bounded");
    let engine = engine();
    // Two instances of the JS runtime, as trust domains will get.
    let busy = js(&engine, &dir, "busy", BUSY).await;
    let burst = js(&engine, &dir, "burst", BURST).await;
    let busy = session(&Faux::new([]), &dir, busy.for_session());
    let burst = session(&Faux::new([]), &dir, burst.for_session());
    for session in [&busy, &burst] {
        session
            .bind_extensions(Arc::new(Notes::default()), Mode::Print, None, None)
            .await;
    }
    let computing = tokio::spawn({
        let busy = busy.clone();
        async move { busy.prompt("/busy", Vec::new()).await }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    burst.prompt("/burst", Vec::new()).await.unwrap();
    computing.await.unwrap().unwrap();

    let ticks = || custom_entries(&busy, "ticks").last().cloned();
    for _ in 0..100 {
        busy.prompt("/count", Vec::new()).await.unwrap();
        if ticks() == Some(json!(64)) || ticks() == Some(json!(65)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    busy.prompt("/count", Vec::new()).await.unwrap();
    // The queue's 64, and the one on its way to the runtime.
    let count = ticks().unwrap().as_u64().unwrap();
    assert!((64..=65).contains(&count), "{count} ticks");
}

/// Loads, with `grants` and event policy `events`, an extension `name` that
/// records each of `channels` it hears as `name:channel`, and emits the
/// channels its `/emit-<name>` command names.
async fn scoped(
    engine: &Engine,
    dir: &Path,
    name: &str,
    channels: &[&str],
    (grants, events): (Grants, EventPolicy),
) -> Arc<ExtensionHost> {
    let source = format!(
        r#"export default function (pi) {{
	for (const channel of {channels:?}) pi.events.on(channel, () => pi.appendEntry("heard", `{name}:${{channel}}`));
	pi.registerCommand("emit-{name}", {{ description: "", handler: async (args) => {{ for (const channel of args.split(" ")) pi.events.emit(channel, {{}}); }} }});
}}
"#
    );
    let path = dir.join(format!("{name}.ts"));
    std::fs::write(&path, source).unwrap();
    let mut options = options(dir);
    options.grants = grants;
    options.events = events;
    let host = ExtensionHost::load(engine, options, &[cli_source(&path)])
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    host
}

/// Three senders whose grants lack the network: shared, isolated, and
/// sending only `todo:*`. Receivers: `same`, with the senders' grants,
/// `differs`, with every grant, which does not listen on `other`, and the
/// native event-bus example, with every grant. Each sender emits its
/// channels, then the shared one emits `done` to all.
async fn scoped_bus(name: &str) -> (AgentSession, std::path::PathBuf) {
    let dir = scratch(name);
    let engine = engine();
    let no_net = Grants {
        network: false,
        ..Grants::default()
    };
    let todo = EventPolicy::new(vec!["todo:*".into()], Vec::new()).unwrap();
    let everything = ["a:shared", "a:isolated", "todo:changed", "other", "done"];
    let mut extensions = Vec::new();
    // Listens on `my:notification`, which the isolated sender's grants keep from it.
    let native = ExtensionHost::load_native(
        &engine,
        options(&dir),
        &cli_source(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/event-bus.wasm")),
    )
    .await
    .unwrap();
    extensions.extend(native.for_session());
    for (host, channels, policy) in [
        ("shared", &[][..], (no_net, EventPolicy::shared())),
        ("isolated", &[][..], (no_net, EventPolicy::isolated())),
        ("table", &[][..], (no_net, todo)),
        ("same", &everything[..], (no_net, EventPolicy::shared())),
        (
            "differs",
            &["a:shared", "a:isolated", "todo:changed", "done"][..],
            (Grants::default(), EventPolicy::shared()),
        ),
    ] {
        extensions.extend(
            scoped(&engine, &dir, host, channels, policy)
                .await
                .for_session(),
        );
    }
    let session = session(&Faux::new([]), &dir, extensions);
    session
        .bind_extensions(Arc::new(Notes::default()), Mode::Print, None, None)
        .await;
    for command in [
        "/emit-shared a:shared",
        "/emit-isolated a:isolated",
        "/emit-isolated a:isolated my:notification",
        "/emit-table todo:changed other",
        "/emit-shared done",
    ] {
        session.prompt(command, Vec::new()).await.unwrap();
    }
    // Each runtime hears its events in order, so `done` comes last.
    let heard = |who: &str| custom_entries(&session, "heard").contains(&json!(who));
    eventually("done", || heard("same:done") && heard("differs:done")).await;
    (session, dir)
}

/// Extensions share `pi.events` by default whatever their grants. An
/// isolated extension reaches only extensions with identical grants, and a
/// channel list lets only its channels across.
#[tokio::test(flavor = "multi_thread")]
async fn bus_scoped_grants() {
    let (session, _) = scoped_bus("bus-scoped").await;
    // Each receiver hears in order, but the two record independently.
    let heard = |who: &str| -> Vec<String> {
        custom_entries(&session, "heard")
            .iter()
            .filter_map(|entry| entry.as_str()?.strip_prefix(who).map(str::to_owned))
            .collect()
    };
    assert_eq!(
        heard("same:"),
        [
            "a:shared",
            "a:isolated",
            "a:isolated",
            "todo:changed",
            "other",
            "done"
        ]
    );
    assert_eq!(heard("differs:"), ["a:shared", "todo:changed", "done"]);
}

/// A runtime that listens on a channel routing keeps from it is warned of
/// once, starting with the lists that would let the event across, whether a
/// Pi or a native extension listens. One that does not listen is not.
#[tokio::test(flavor = "multi_thread")]
async fn missed_event_warns() {
    yapi_ext::capture_output();
    let (_session, dir) = scoped_bus("bus-missed").await;
    let file = |name: &str| dir.join(format!("{name}.ts")).display().to_string();
    let output = yapi_ext::extension_output();
    let missed: Vec<&String> = output
        .iter()
        .filter(|line| line.contains("did not reach") && line.contains(&dir.display().to_string()))
        .collect();
    let native = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/event-bus.wasm")
        .display()
        .to_string();
    let warning = |channel: &str, receiver: &str| {
        format!(
            "add \"{channel}\" to events.send of {}: pi.events \"{channel}\" from {} (fs, env, session, process) did not reach {receiver} (all)",
            file("isolated"),
            file("isolated")
        )
    };
    let expected = [
        warning("a:isolated", &file("differs")),
        warning("my:notification", &native),
    ];
    let found = |warning: &String| missed.iter().any(|line| line.ends_with(warning.as_str()));
    assert!(
        missed.len() == 2 && expected.iter().all(found),
        "{missed:#?}"
    );
    listeners_tracked().await;
}

/// What a runtime listens on follows its handlers: one unsubscribed is gone,
/// a channel with a handler left still listens, and a reload or a restart
/// forgets the channels its extensions no longer listen on. The host keeps
/// at most 1024 channels of 256 bytes, and warns at most 8 times from one
/// runtime to another before it says the rest are suppressed.
async fn listeners_tracked() {
    const PROBE: &str = r#"
let loads = 0;
export default function (pi) {
	loads++;
	const handler = () => {};
	pi.events.on("gone", handler)();
	pi.events.on("twice", handler);
	pi.events.on("twice", handler)();
	if (loads === 1) pi.events.on("dropped", handler);
	for (let i = 0; i < 10; i++) pi.events.on(`capped:${i}`, handler);
	pi.events.on("x".repeat(300), handler);
	for (let i = 0; i < 1100; i++) pi.events.on(`many:${i}`, handler);
}
"#;
    const RESTARTS: &str = r#"
const overflow = () => overflow();
export default function (pi) {
	pi.registerCommand("late", { description: "", handler: async () => { pi.events.on("late", () => {}); } });
	pi.registerCommand("crash", { description: "", handler: async () => { setTimeout(overflow, 0); } });
	pi.registerCommand("noop", { description: "", handler: async () => {} });
}
"#;
    let dir = scratch("bus-listeners");
    let engine = engine();
    let no_net = Grants {
        network: false,
        ..Grants::default()
    };
    let todo = EventPolicy::new(vec!["todo:*".into()], Vec::new()).unwrap();
    let hosts = [
        scoped(
            &engine,
            &dir,
            "sender",
            &[],
            (no_net, EventPolicy::isolated()),
        )
        .await,
        scoped(&engine, &dir, "table", &[], (no_net, todo)).await,
        js(&engine, &dir, "probe", PROBE).await,
        js(&engine, &dir, "restarts", RESTARTS).await,
    ];
    let start = || async {
        let extensions = hosts.iter().flat_map(|host| host.for_session()).collect();
        let session = session(&Faux::new([]), &dir, extensions);
        session
            .bind_extensions(Arc::new(Notes::default()), Mode::Print, None, None)
            .await;
        session
    };
    let session = start().await;
    let long = "x".repeat(300);
    // The probe's first load listens on `twice`, `dropped` and ten `capped`
    // channels before the `many` ones, so `many:1011` is the 1024th channel
    // the host keeps and `many:1012` the first it does not.
    for command in [
        "/late".to_owned(),
        format!("/emit-sender gone twice many:5 many:1011 many:1012 many:1050 {long}"),
        format!(
            "/emit-table {}",
            (0..10)
                .map(|i| format!("capped:{i}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        "/crash".to_owned(),
    ] {
        session.prompt(&command, Vec::new()).await.unwrap();
    }
    let restarts = file_in(&dir, "restarts");
    eventually("the restart", || {
        yapi_ext::extension_output()
            .iter()
            .any(|line| line.contains(&format!("[{restarts}] error Extension runtime stopped")))
    })
    .await;
    // Its next command runs once the replay has loaded the extensions again.
    session.prompt("/noop", Vec::new()).await.unwrap();
    session
        .prompt("/emit-sender late", Vec::new())
        .await
        .unwrap();
    // The next session's factories no longer listen on `dropped`.
    for host in &hosts {
        host.reload(&host.sources()).await;
    }
    let session = start().await;
    session
        .prompt("/emit-sender dropped", Vec::new())
        .await
        .unwrap();

    let (sender, table, probe) = (
        file_in(&dir, "sender"),
        file_in(&dir, "table"),
        file_in(&dir, "probe"),
    );
    let warning = |channel: &str, from: &str| {
        format!(
            "add \"{channel}\" to events.send of {from}: pi.events \"{channel}\" from {from} (fs, env, session, process) did not reach {probe} (all)"
        )
    };
    let mut expected = vec![
        warning("twice", &sender),
        warning("many:5", &sender),
        warning("many:1011", &sender),
    ];
    expected.extend((0..8).map(|i| warning(&format!("capped:{i}"), &table)));
    expected.push(format!(
        "further pi.events warnings from {table} to {probe} suppressed"
    ));
    let output = yapi_ext::extension_output();
    let warnings: Vec<&str> = output
        .iter()
        .filter(|line| line.contains(&dir.display().to_string()) && line.contains("] warn "))
        .filter_map(|line| line.split("] warn ").nth(1))
        .collect();
    assert_eq!(warnings, expected);
    drop(session);
}

fn file_in(dir: &Path, name: &str) -> String {
    dir.join(format!("{name}.ts")).display().to_string()
}

/// MCP server names stay unique across every runtime, whatever its event
/// policy.
#[tokio::test(flavor = "multi_thread")]
async fn mcp_check_stays_global() {
    let dir = scratch("bus-mcp");
    let engine = engine();
    let register = "export default function (pi) {\n\tpi.registerMcpServer(\"shared\", { command: \"python3\", enabled: false });\n}\n";
    let (mut errors, mut hosts) = (Vec::new(), Vec::new());
    for (file, events) in [
        ("owner.ts", EventPolicy::isolated()),
        ("taker.ts", EventPolicy::shared()),
    ] {
        let path = dir.join(file);
        std::fs::write(&path, register).unwrap();
        let mut options = options(&dir);
        options.grants = Grants {
            network: file == "taker.ts",
            ..Grants::default()
        };
        options.events = events;
        let host = ExtensionHost::load(&engine, options, &[cli_source(&path)])
            .await
            .unwrap();
        errors.extend(host.errors().iter().map(|error| error.error.clone()));
        // The bus holds runtimes only while they are in use.
        hosts.push(host);
    }
    let owner = dir.join("owner.ts").display().to_string();
    assert_eq!(
        errors,
        [format!(
            "Failed to load extension: MCP server \"shared\" is already registered by extension \"{owner}\""
        )]
    );
}
