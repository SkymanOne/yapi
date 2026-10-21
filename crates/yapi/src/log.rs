//! `--enable-logging`: `tracing` events appended to a file, one line each.

use std::fmt::{Debug, Write as _};
use std::fs::File;
use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;

use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Metadata, Subscriber, span};

/// Appends this process's `tracing` events to `path` from now on: yapi's own
/// from the debug level up, its dependencies' from info up.
pub fn enable(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let file = options.open(path)?;
    tracing::subscriber::set_global_default(FileLog(Mutex::new(file)))
        .map_err(std::io::Error::other)
}

/// Writes each event as `<time> <pid> <LEVEL> <target>: <message> <field>=<value>`.
/// Spans are not tracked.
struct FileLog(Mutex<File>);

impl Subscriber for FileLog {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        let most = if metadata.target().starts_with("yapi") {
            Level::DEBUG
        } else {
            Level::INFO
        };
        *metadata.level() <= most
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::DEBUG)
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        let metadata = event.metadata();
        let mut fields = Fields::default();
        event.record(&mut fields);
        let line = format!(
            "{} {} {} {}:{}{}\n",
            yapi_types::time::now_iso(),
            std::process::id(),
            metadata.level(),
            metadata.target(),
            fields.message,
            fields.rest
        );
        // One write per line, so lines of processes sharing the file stay whole.
        if let Ok(mut file) = self.0.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}
}

/// An event's message and its other fields, each with a leading space.
#[derive(Default)]
struct Fields {
    message: String,
    rest: String,
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        let _ = match field.name() {
            "message" => write!(self.message, " {value:?}"),
            name => write!(self.rest, " {name}={value:?}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_event_lines() {
        let path = std::env::temp_dir().join(format!("yapi-log-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let file = File::create(&path).unwrap();
        tracing::subscriber::with_default(FileLog(Mutex::new(file)), || {
            tracing::info!(target: "yapi_ext::extensions", extension = %"todo", "loaded");
            tracing::debug!(target: "yapi_core", "kept");
            tracing::debug!(target: "h2::codec", "dropped");
            tracing::warn!(target: "h2::codec", "shown");
        });
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = text
            .lines()
            .map(|line| line.splitn(3, ' ').nth(2).unwrap())
            .collect();
        assert_eq!(
            lines,
            [
                "INFO yapi_ext::extensions: loaded extension=todo",
                "DEBUG yapi_core: kept",
                "WARN h2::codec: shown",
            ]
        );
        std::fs::remove_file(&path).unwrap();
    }
}
