//! A test fixture: its `/probe` command sends each request kind that needs
//! a capability, then a few that need none, and notifies what yapi said to
//! each, as JSON from kind to `{"ok"}` or `{"err"}` and the `feature` the
//! SDK gates it behind, `null` for none. A command runs outside a turn, so
//! `agent.abort` stops nothing. Built with every capability feature, so the
//! SDK lets each request through.

use yapi_extension_api::{Api, KIND_FEATURES, Value, json, notify, op, request};

/// `kind`'s answer as a request, else as an operation: yapi serves each
/// kind one way, and answers the other with an unknown kind.
async fn send(kind: &str, payload: &Value) -> Value {
    let answer = match request(kind, payload) {
        Err(err) if !err.contains("not permitted") => op(kind, payload).await,
        answer => answer,
    };
    match answer {
        Ok(value) => json!({ "ok": value }),
        Err(err) => json!({ "err": err }),
    }
}

fn init(api: &mut Api) {
    api.register_command(
        "probe",
        "Sends every request that needs a capability",
        |_args, _ctx| async move {
            // Kinds that need no capability, with their payloads.
            let baseline = [
                ("cwd", None, json!({})),
                ("timer", None, json!({"ms": 0})),
                (
                    "session.read",
                    None,
                    json!({"method": "getSessionId", "args": []}),
                ),
            ];
            let needing = KIND_FEATURES
                .iter()
                .map(|(kind, feature)| (*kind, Some(*feature), json!({})));
            let mut answers = json!({});
            for (kind, feature, payload) in needing.chain(baseline) {
                let mut answer = send(kind, &payload).await;
                answer["feature"] = json!(feature);
                answers[kind] = answer;
            }
            notify(&answers.to_string(), "info");
            Ok(())
        },
    );
}

yapi_extension_api::extension!(init);
