//! Print mode: run the prompts, then print the final answer (text) or every event
//! (JSON). Port of `modes/print-mode.ts` in pi `v1.0.0`.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use yapi_core::agent_session::failed;
use yapi_core::extensions::{Mode, NoUi};
use yapi_types::message::ContentBlock;

use crate::runtime::{Bind, Runtime, SessionFactory, forward_newest};
use crate::startup::Startup;

/// Writes one line to stdout, ignoring a closed pipe.
fn write_line(line: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(line.as_bytes());
    let _ = stdout.write_all(b"\n");
    let _ = stdout.flush();
}

/// Runs print mode; returns the process exit code. Extension commands
/// replace the session through `factory`, and later prompts go to the
/// replacement.
pub async fn run(startup: Startup, json: bool, factory: SessionFactory) -> u8 {
    let Startup {
        session,
        initial_message,
        initial_images,
        messages,
        ..
    } = startup;
    let mode = if json { Mode::Json } else { Mode::Print };
    if json && let Some(header) = session.header_json() {
        write_line(&header);
    }
    // The current session's number; listeners of older ones stay silent.
    let epoch = Arc::new(AtomicU64::new(0));
    // As in pi, a session's events stream once its extensions have started.
    let bind: Bind = Box::new(move |session, actions, replaced| {
        let epoch = Arc::clone(&epoch);
        Box::pin(async move {
            session
                .bind_extensions(Arc::new(NoUi), mode, Some(actions), replaced)
                .await;
            if json {
                forward_newest(&session, &epoch, |event| {
                    if let Ok(line) = yapi_types::json::to_string(event) {
                        write_line(&line);
                    }
                });
            }
        })
    });

    let mut prompts = Vec::new();
    // pi skips an empty first message, images included.
    if let Some(message) = initial_message.filter(|message| !message.is_empty()) {
        prompts.push((message, initial_images));
    }
    prompts.extend(messages.into_iter().map(|message| (message, Vec::new())));
    // Signals that arrive while the session starts end it once it has.
    let termination = super::rpc::termination();
    let local = tokio::task::LocalSet::new();
    let session = local
        .run_until(async move {
            let runtime = Runtime::start(session, factory, bind).await;
            let run = async {
                for (message, images) in prompts {
                    if let Err(error) = runtime.session().prompt(&message, images).await {
                        eprintln!("{error}");
                        return false;
                    }
                }
                true
            };
            // pi kills running commands and ends the session on SIGTERM and
            // SIGHUP, then exits with the signal's code. The run stays alive
            // until they are killed: dropping it would stop tracking its
            // commands without killing them.
            tokio::pin!(run);
            let outcome = tokio::select! {
                ok = &mut run => Ok(ok),
                code = termination => {
                    yapi_core::tools::bash::kill_tracked_children();
                    Err(code)
                }
            };
            runtime.session().shutdown().await;
            match outcome {
                Ok(true) => Ok(runtime.session()),
                Ok(false) => Err(1),
                Err(code) => Err(code),
            }
        })
        .await;
    let session = match session {
        Ok(session) => session,
        Err(code) => return code,
    };

    if !json
        && let Some(last) = session.messages().last()
        && let yapi_types::message::Message::Assistant(assistant) = last
    {
        if failed(assistant) {
            let reason = match assistant.stop_reason {
                yapi_types::message::StopReason::Aborted => "aborted",
                _ => "error",
            };
            eprintln!(
                "{}",
                assistant
                    .error_message
                    .clone()
                    .unwrap_or_else(|| format!("Request {reason}"))
            );
            return 1;
        }
        for block in &assistant.content {
            if let ContentBlock::Text(text) = block {
                write_line(&text.text);
            }
        }
    }
    0
}
