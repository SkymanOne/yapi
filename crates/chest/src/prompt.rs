//! Questions on stderr, answered on stdin, asked only of a person at a
//! terminal.

use std::io::{BufRead as _, IsTerminal as _, Write as _};

/// Asks questions when stdin is a terminal and `--yes` is not given.
/// Otherwise every question takes its default answer.
pub struct Prompt {
    interactive: bool,
}

impl Prompt {
    /// A prompt that asks unless `yes` or stdin is not a terminal.
    pub fn new(yes: bool) -> Prompt {
        Prompt {
            interactive: !yes && std::io::stdin().is_terminal(),
        }
    }

    /// Whether questions are asked.
    pub fn interactive(&self) -> bool {
        self.interactive
    }

    /// Prints `line` on stderr, as [`crate::say`] does.
    pub fn say(&self, line: &str) {
        crate::say(line);
    }

    /// The trimmed answer to `question`, empty at the end of input.
    fn ask(&self, question: &str) -> String {
        let mut stderr = std::io::stderr();
        let _ = write!(stderr, "{question} ");
        let _ = stderr.flush();
        let mut line = String::new();
        let read = || std::io::stdin().lock().read_line(&mut line);
        let _ = tokio::task::block_in_place(read);
        line.trim().to_owned()
    }

    /// Whether the person answers yes. No unless asked.
    pub fn confirm(&self, question: &str) -> bool {
        self.interactive
            && matches!(
                self.ask(&format!("{question} [y/N]"))
                    .to_ascii_lowercase()
                    .as_str(),
                "y" | "yes"
            )
    }

    /// The index of the option chosen by number, `default` unless asked or
    /// when the answer is empty.
    pub fn choose(&self, question: &str, options: &[String], default: usize) -> usize {
        self.choose_many(question, options, &[default])
            .first()
            .copied()
            .filter(|_| self.interactive)
            .unwrap_or(default)
    }

    /// The indexes of the options chosen by comma-separated numbers,
    /// `defaults` unless asked or when the answer is empty.
    pub fn choose_many(
        &self,
        question: &str,
        options: &[String],
        defaults: &[usize],
    ) -> Vec<usize> {
        if !self.interactive {
            return defaults.to_vec();
        }
        self.say(question);
        for (index, option) in options.iter().enumerate() {
            self.say(&format!("  {}) {option}", index + 1));
        }
        let shown: Vec<String> = defaults
            .iter()
            .map(|index| (index + 1).to_string())
            .collect();
        loop {
            let answer = self.ask(&format!("Numbers [{}]:", shown.join(",")));
            if answer.is_empty() {
                return defaults.to_vec();
            }
            let chosen: Option<Vec<usize>> = answer
                .split(',')
                .map(|part| {
                    let number = part.trim().parse::<usize>().ok()?;
                    (1..=options.len()).contains(&number).then(|| number - 1)
                })
                .collect();
            if let Some(chosen) = chosen {
                return chosen;
            }
            self.say(&format!("Enter numbers from 1 to {}.", options.len()));
        }
    }
}
