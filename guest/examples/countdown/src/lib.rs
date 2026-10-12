//! A `countdown` tool that shows each second as progress and, when the run
//! is aborted, stops with its own result. Stopping takes `cleanup` seconds,
//! as stopping a process can. A `wait` tool ignores aborts, so yapi stops it
//! at once.

use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

use yapi_extension_api::{Api, Tool, ToolResult, json, sleep};

/// `Ok` with `first`'s output when it finishes before `second`, else `Err`
/// with `second`'s.
async fn race<A, B>(
    first: impl Future<Output = A>,
    second: impl Future<Output = B>,
) -> Result<A, B> {
    let (mut first, mut second) = (pin!(first), pin!(second));
    poll_fn(|cx| match first.as_mut().poll(cx) {
        Poll::Ready(output) => Poll::Ready(Ok(output)),
        Poll::Pending => second.as_mut().poll(cx).map(Err),
    })
    .await
}

fn init(api: &mut Api) {
    api.register_tool(Tool::new(
        "countdown",
        "Counts down from `seconds`. Stops when the run is aborted.",
        json!({
            "type": "object",
            "properties": {
                "seconds": {"type": "integer", "description": "Where to start"},
                "cleanup": {"type": "integer", "description": "Seconds stopping takes"},
            },
            "required": ["seconds"],
        }),
        |params, ctx| async move {
            let seconds = params["seconds"].as_u64().unwrap_or_default();
            let cleanup = params["cleanup"].as_u64().unwrap_or_default();
            let details = json!({"toolCallId": ctx.tool_call_id()});
            for left in (1..=seconds).rev() {
                ctx.update(&ToolResult::text(format!("{left}...")));
                if race(sleep(1000), ctx.aborted()).await.is_err() {
                    sleep(cleanup * 1000).await;
                    return Ok(
                        ToolResult::text(format!("Stopped with {left} seconds left"))
                            .with_details(details),
                    );
                }
            }
            Ok(ToolResult::text("Liftoff").with_details(details))
        },
    ));
    api.register_tool(Tool::new(
        "wait",
        "Waits `seconds` seconds.",
        json!({
            "type": "object",
            "properties": {"seconds": {"type": "integer", "description": "How long"}},
            "required": ["seconds"],
        }),
        |params, ctx| async move {
            let seconds = params["seconds"].as_u64().unwrap_or_default();
            ctx.update(&ToolResult::text(format!("Waiting {seconds} seconds")));
            sleep(seconds * 1000).await;
            Ok(ToolResult::text("Done waiting"))
        },
    ));
}

yapi_extension_api::extension!(init);
