//! A test fixture: its `probe` tool sends each request kind that needs a
//! capability, then a few that need none, and answers with what yapi said
//! to each, as JSON from kind to `{"ok"}` or `{"err"}`. Built with every
//! capability feature, so the SDK lets each request through.

use yapi_extension_api::{Api, KIND_FEATURES, Tool, ToolResult, Value, json, op, request};

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
    api.register_tool(Tool::new(
        "probe",
        "Sends every request that needs a capability",
        json!({"type": "object", "properties": {}}),
        |_params, _ctx| async move {
            // Kinds that need no capability, with their payloads.
            let baseline = [
                ("cwd", json!({})),
                ("timer", json!({"ms": 0})),
                (
                    "session.read",
                    json!({"method": "getSessionId", "args": []}),
                ),
            ];
            let needing = KIND_FEATURES.iter().map(|(kind, _)| (*kind, json!({})));
            let mut answers = json!({});
            for (kind, payload) in needing.chain(baseline) {
                answers[kind] = send(kind, &payload).await;
            }
            Ok(ToolResult::text(answers.to_string()))
        },
    ));
}

yapi_extension_api::extension!(init);
