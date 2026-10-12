//! A test fixture: names the project directory it read as its factory ran,
//! and the run, in the description of its `/project` command, which reloads
//! the extensions.

use std::cell::Cell;

use yapi_extension_api::{Api, json, op, project_dir};

thread_local! {
    /// Runs of the factory so far.
    static RUNS: Cell<u32> = const { Cell::new(0) };
}

fn init(api: &mut Api) {
    let run = RUNS.with(|runs| {
        runs.set(runs.get() + 1);
        runs.get()
    });
    let dir = project_dir().unwrap_or_else(|| "none".into());
    api.register_command(
        "project",
        format!("Run {run} for {dir}"),
        |_args, _ctx| async move { op("session.reload", &json!({})).await.map(|_| ()) },
    );
}

yapi_extension_api::extension!(init);
