//! Questions and progress for a person at a terminal, drawn on stderr with
//! cliclack. Elsewhere every question takes its default answer and chest
//! prints the plain lines that scripts read.

use std::io::{IsTerminal as _, Write as _};

use cliclack::ThemeState;
use console::{Emoji, style};

use crate::{ci, out, printable, say};

/// The person cancelled a question, with Esc or Ctrl+C, or declined a
/// confirmation. The prompt has said so, so chest exits with 1 and prints
/// nothing more.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// How chest talks to the person running it.
pub struct Prompt {
    /// Whether questions are asked: stdin and stderr, where cliclack draws,
    /// are terminals, `CI` is unset and `--yes` is not given.
    asks: bool,
    /// Whether progress and reports are drawn too, which needs stdout to be
    /// a terminal as well, since the plain report goes there.
    styled: bool,
}

/// One option of a question: its value, label and hint.
pub type Choice<T> = (T, String, String);

/// `text` on one line without control characters, for cliclack.
fn clean(text: &str) -> String {
    printable(text, &[])
}

/// A question's first line, with the lines after it as a dimmed hint.
fn question(text: &str) -> String {
    let mut lines = text.lines().map(clean);
    let first = lines.next().unwrap_or_default();
    lines.fold(first, |text, hint| format!("{text}\n{}", style(hint).dim()))
}

/// `text` wrapped to the terminal beside a log line's symbol.
fn wrapped(text: &str) -> Vec<String> {
    let text = cliclack::termwrap(&clean(text), 3);
    text.lines().map(str::to_owned).collect()
}

/// Clack's theme, whose symbols are text, never emoji, with the option
/// under the cursor, the ticked options and the spinner in cyan, as the
/// active step is. console leaves out colour for a stream that is not a
/// terminal, and when `NO_COLOR` is set.
struct Theme;

impl cliclack::Theme for Theme {
    fn radio_symbol(&self, state: &ThemeState, selected: bool) -> String {
        match state {
            ThemeState::Active if selected => style(Emoji("●", ">")).cyan(),
            ThemeState::Active => style(Emoji("○", " ")).dim(),
            _ => style(Emoji("", "")),
        }
        .to_string()
    }

    fn checkbox_symbol(&self, state: &ThemeState, selected: bool, active: bool) -> String {
        match state {
            ThemeState::Active | ThemeState::Error(_) if selected => {
                style(Emoji("◼", "[+]")).cyan()
            }
            ThemeState::Active | ThemeState::Error(_) if active => style(Emoji("◻", "[•]")).cyan(),
            ThemeState::Active | ThemeState::Error(_) => style(Emoji("◻", "[ ]")).dim(),
            _ => style(Emoji("", "")),
        }
        .to_string()
    }

    fn format_progress_start(&self, template: &str, grouped: bool, last: bool) -> String {
        let space = if grouped { " " } else { "  " };
        let template = format!("{{spinner:.cyan}}{space}{template}");
        self.format_progress_with_state(&template, grouped, last, &ThemeState::Active)
    }
}

/// The rail between steps, dimmed.
pub fn rail() -> String {
    style(Emoji("│", "|")).black().bright().to_string()
}

/// A question's answer, with Esc and Ctrl+C as [`Cancelled`].
fn answer<T>(result: std::io::Result<T>) -> anyhow::Result<T> {
    result.map_err(|err| match err.kind() {
        std::io::ErrorKind::Interrupted => Cancelled.into(),
        _ => anyhow::Error::from(err),
    })
}

impl Prompt {
    /// A prompt that asks only a person at a terminal, unless `yes`.
    pub fn new(yes: bool) -> Prompt {
        let terminal = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
        let asks = !yes && !ci() && terminal;
        if asks {
            cliclack::set_theme(Theme);
        }
        Prompt {
            asks,
            styled: asks && std::io::stdout().is_terminal(),
        }
    }

    /// Whether questions are asked.
    pub fn interactive(&self) -> bool {
        self.asks
    }

    /// Whether progress and reports are drawn rather than printed as lines.
    pub fn styled(&self) -> bool {
        self.styled
    }

    /// Opens a drawn session.
    pub fn intro(&self, title: &str) {
        if self.styled {
            let _ = cliclack::intro(clean(title));
        }
    }

    /// Closes a drawn session with `line`.
    pub fn outro(&self, line: &str) {
        if self.styled {
            let _ = cliclack::outro(clean(line));
        }
    }

    /// Closes a drawn session with `line`, or prints it on stderr.
    pub fn done(&self, line: &str) {
        match self.styled {
            true => self.outro(line),
            false => say(line),
        }
    }

    /// Closes a drawn session with the error `line` without its `error: `
    /// prefix, or prints it on stderr as it is. It keeps its line breaks.
    pub fn error(&self, line: &str) {
        match self.styled {
            true => {
                let text = line.strip_prefix("error: ").unwrap_or(line);
                let _ = cliclack::outro_cancel(printable(text, &['\n']));
            }
            false => {
                let _ = writeln!(std::io::stderr(), "{line}");
            }
        }
    }

    /// Prints `lines` as one info block, with indented lines as dimmed
    /// details and without a `note: ` prefix, or each on stderr as it is.
    pub fn info(&self, lines: &[impl AsRef<str>]) {
        if self.styled {
            let mut text = Vec::new();
            for line in lines {
                let line = line.as_ref();
                let parts = wrapped(line.strip_prefix("note: ").unwrap_or(line));
                text.extend(parts.into_iter().map(|part| match line.starts_with(' ') {
                    true => style(part).dim().to_string(),
                    false => part,
                }));
            }
            let _ = cliclack::log::info(text.join("\n"));
        } else {
            lines.iter().for_each(|line| say(line.as_ref()));
        }
    }

    /// Prints the warning `line`, as a warning line without its `warning: `
    /// prefix, or on stderr as it is.
    pub fn warn(&self, line: &str) {
        match self.styled {
            true => {
                let text = line.strip_prefix("warning: ").unwrap_or(line);
                let _ = cliclack::log::warning(wrapped(text).join("\n"));
            }
            false => say(line),
        }
    }

    /// Prints the result `line`, as a success line, or on stdout.
    pub fn data(&self, line: &str) {
        match self.styled {
            true => {
                let _ = cliclack::log::success(clean(line));
            }
            false => out(line),
        }
    }

    /// Shows `lines` in a box titled `title`, before a confirmation, when
    /// chest asks.
    pub fn summary(&self, title: &str, lines: &[String]) {
        if self.asks && !lines.is_empty() {
            let lines: Vec<String> = lines.iter().map(|line| clean(line)).collect();
            let _ = cliclack::note(clean(title), lines.join("\n"));
        }
    }

    /// The value the person selects, `default` unless asked. Lines of
    /// `question` after the first are a hint.
    pub fn select<T: Clone + Eq>(
        &self,
        question: &str,
        options: &[Choice<T>],
        default: T,
    ) -> anyhow::Result<T> {
        if !self.asks {
            return Ok(default);
        }
        let mut select = cliclack::select(self::question(question)).initial_value(default);
        for (value, label, hint) in options {
            select = select.item(value.clone(), clean(label), clean(hint));
        }
        answer(tokio::task::block_in_place(|| select.interact()))
    }

    /// The values the person ticks, in the order of `options`, `defaults`
    /// unless asked. With `required`, at least one. Lines of `question`
    /// after the first are a hint.
    pub fn multiselect<T: Clone + Eq>(
        &self,
        question: &str,
        options: &[Choice<T>],
        defaults: Vec<T>,
        required: bool,
    ) -> anyhow::Result<Vec<T>> {
        if !self.asks {
            return Ok(defaults);
        }
        let mut select = cliclack::multiselect(self::question(question))
            .initial_values(defaults)
            .required(required);
        for (value, label, hint) in options {
            select = select.item(value.clone(), clean(label), clean(hint));
        }
        answer(tokio::task::block_in_place(|| select.interact()))
    }

    /// Passes when the person answers yes, or when chest asks nothing.
    /// Fails with [`Cancelled`] otherwise, before anything is written.
    pub fn confirm(&self, question: &str) -> anyhow::Result<()> {
        if !self.asks {
            return Ok(());
        }
        let mut confirm = cliclack::confirm(clean(question)).initial_value(true);
        if !answer(tokio::task::block_in_place(|| confirm.interact()))? {
            let _ = cliclack::outro_cancel("Cancelled. Nothing was written.");
            return Err(Cancelled.into());
        }
        Ok(())
    }

    /// A spinner showing `message`, when drawn.
    pub fn spinner(&self, message: &str) -> Spinner {
        Spinner(self.styled.then(|| {
            let bar = cliclack::spinner();
            bar.start(clean(message));
            (bar, clean(message))
        }))
    }
}

/// A step under way. Dropped before it stops, it shows its first message as
/// failed.
pub struct Spinner(Option<(cliclack::ProgressBar, String)>);

impl Spinner {
    /// Shows `line` as the step under way, or prints it on stderr.
    pub fn step(&self, line: &str) {
        match &self.0 {
            Some((bar, _)) => bar.set_message(clean(line)),
            None => say(line),
        }
    }

    /// Ends the step with `line`, and `details` under it on the rail. The
    /// details may be styled, so they are not cleaned.
    pub fn stop(&self, line: &str, details: &[String]) {
        if let Some((bar, _)) = &self.0 {
            let rail = rail();
            let details = details.iter().map(|detail| format!("\n{rail}  {detail}"));
            bar.stop(clean(line) + &details.collect::<String>());
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some((bar, message)) = &self.0 {
            bar.cancel(message);
        }
    }
}
