//! `/status-card [message]` stores a status card in the session, which an
//! entry renderer draws in a box. The card is never sent to the model.
//!
//! A port of pi's `entry-renderer.ts`. pi shows the card's local time when
//! expanded, and this port its UTC time.

use yapi_extension_api::widgets::tui::lines::{box_content_width, boxed, text};
use yapi_extension_api::widgets::{bg_style, parse, to_ansi};
use yapi_extension_api::{Api, Component, append_entry, json, theme};

/// pi-tui's `Box` with padding 1 around a `Text`, over the custom message
/// background.
struct StatusCard {
    text: String,
}

impl Component for StatusCard {
    fn render(&mut self, width: usize) -> Vec<String> {
        let inner = text(
            &[parse(&self.text)],
            box_content_width(width, 1),
            0,
            0,
            None,
        );
        to_ansi(
            &boxed(inner, width, 1, 1, Some(bg_style("customMessageBg"))),
            None,
        )
    }
}

fn init(api: &mut Api) {
    api.register_entry_renderer("status-card", |entry, options| {
        let th = theme();
        let data = &entry["data"];
        let message = data["message"].as_str().unwrap_or("No data");
        let mut text = format!("{} {message}", th.fg("accent", "[status]"));
        if options["expanded"] == true
            && let Some(timestamp) = data["timestamp"].as_u64()
        {
            let seconds = timestamp / 1000 % 86_400;
            let time = format!(
                "{:02}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            );
            text += &format!("\n{}", th.fg("dim", &time));
        }
        Some(Box::new(StatusCard { text }))
    });
    api.register_command(
        "status-card",
        "Render a durable status card that is not sent to the LLM",
        |args, _ctx| async move {
            let message = match args.trim() {
                "" => "Status card",
                message => message,
            };
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |now| now.as_millis() as u64);
            append_entry(
                "status-card",
                json!({"message": message, "timestamp": timestamp}),
            )
        },
    );
}

yapi_extension_api::extension!(init);
