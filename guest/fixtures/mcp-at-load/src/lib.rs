//! A test fixture: reports the MCP server `at-load` with the internal
//! `mcp.servers` request as it loads, and other servers with
//! `/servers <names>`. The session gets each answer as a `servers` entry,
//! the one from loading as the session starts.

use std::cell::RefCell;

use yapi_extension_api::{Api, Value, append_entry, extension_path, json, request};

thread_local! {
    /// The answer to the report made as the extension loaded.
    static AT_LOAD: RefCell<Value> = RefCell::default();
}

/// Reports servers `names`, which never connect: `"ok"`, or the error.
fn report(names: &[&str]) -> Value {
    let path = extension_path().file;
    let servers: Vec<Value> = names
        .iter()
        .map(|name| json!({"name": name, "config": {"command": "python3", "enabled": false}, "extensionPath": path}))
        .collect();
    match request("mcp.servers", &json!({"servers": servers})) {
        Ok(_) => json!("ok"),
        Err(error) => json!(error),
    }
}

fn init(api: &mut Api) {
    let answer = report(&["at-load"]);
    AT_LOAD.with(|at_load| *at_load.borrow_mut() = answer);
    api.on("session_start", |_event, _ctx| async move {
        append_entry("servers", AT_LOAD.with(|at_load| at_load.borrow().clone()))?;
        Ok(None)
    });
    api.register_command(
        "servers",
        "Reports these MCP servers",
        |args, _ctx| async move {
            let names: Vec<&str> = args.split_whitespace().collect();
            append_entry("servers", report(&names))
        },
    );
}

yapi_extension_api::extension!(init);
