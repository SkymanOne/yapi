//! `/bug`: the dialogs of `modes/interactive/bug-report.ts` in pi `v1.0.0`.
//! The report is always written as a zip in the working directory, then
//! offered as a public GitHub issue instead of pi's upload.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use yapi_core::agent_session::AgentSession;
use yapi_core::bug_report::{self, ENTRY_TYPE, ISSUES_URL, REPO, Report};

use super::selectors::{ChoiceDialog, Selector, TextDialog};
use super::{App, Dialog, Event, ExtensionReply};

const DISCLAIMER: &str = "The report can include your yapi version, operating system, the current model and provider configuration (without API keys), loaded extensions, settings, and provider error diagnostics from this session. yapi saves it as a zip file and can post it as a GitHub issue. A GitHub issue is public, unlike Pi's private upload to radius.pi.dev.";
const TRANSCRIPT_NOTE: &str = "The transcript contains your messages, model output, tool calls and their results, including file contents and command output read during this session. It goes in the zip file only and is never posted to GitHub.";
const CANCELLED: &str = "Bug report cancelled";

/// Opens dialogs from `/bug`'s task the way extension dialogs open: the app
/// shows each one and answers through a channel. A dialog the app drops,
/// such as one of a replaced session, answers as cancelled.
struct Ask {
    tx: UnboundedSender<Event>,
    epoch: u64,
}

impl Ask {
    fn then(&self, then: impl FnOnce(&mut App) + Send + 'static) {
        let _ = self.tx.send(Event::Then(self.epoch, Box::new(then)));
    }

    /// pi's editor dialog with `description`, starting with `prefill`.
    async fn text(&self, title: &str, description: String, prefill: &str) -> Option<String> {
        let (reply, answer) = oneshot::channel();
        let (title, prefill) = (title.to_owned(), prefill.to_owned());
        self.then(move |app| {
            let mut dialog = TextDialog::new(&title, super::editor_theme(&app.theme));
            dialog.description = Some(description);
            dialog.prefill(&prefill);
            let selector = Selector::Text(Box::new(dialog));
            app.open_extension_dialog(selector, Some(ExtensionReply::Text(reply)));
        });
        answer.await.ok().flatten()
    }

    /// pi's selector dialog: the chosen option.
    async fn choose(
        &self,
        title: &str,
        description: Option<String>,
        options: &[&str],
    ) -> Option<String> {
        let (reply, answer) = oneshot::channel();
        let title = title.to_owned();
        let options: Vec<String> = options.iter().map(|option| (*option).to_owned()).collect();
        self.then(move |app| {
            let labels: Vec<&str> = options.iter().map(String::as_str).collect();
            let mut dialog = ChoiceDialog::new(&title, &labels);
            dialog.description = description;
            let reply = ExtensionReply::Select(options, reply);
            app.open_extension_dialog(Selector::Choice(dialog), Some(reply));
        });
        answer.await.ok().flatten()
    }

    /// `yes` or "No": whether the user chose `yes`.
    async fn confirm(&self, title: &str, description: &str, yes: &str) -> Option<bool> {
        let chosen = self
            .choose(title, Some(description.to_owned()), &[yes, "No"])
            .await?;
        Some(chosen == yes)
    }

    /// Closes the open dialog when nothing waits for its answer any more.
    fn close_abandoned(&self) {
        self.then(|app| {
            if let Some(Dialog::Extension(ExtensionReply::Select(_, reply))) = &app.dialog
                && reply.is_closed()
            {
                app.dialog = None;
                app.selector = None;
            }
        });
    }
}

impl App {
    /// `/bug`, with `hint` as the description's first text.
    pub(super) fn report_bug(&mut self, hint: String) {
        let ask = Ask {
            tx: self.tx.clone(),
            epoch: self.epoch,
        };
        let session = self.session.clone();
        let dir = std::env::current_dir().unwrap_or_else(|_| self.cwd.clone());
        let offline = yapi_core::config::env_set("PI_OFFLINE");
        tokio::spawn(async move {
            let outcome = run(&ask, &session, &hint, &dir, offline, OsStr::new("gh")).await;
            ask.then(move |app| match outcome {
                Ok(text) => app.status(text),
                Err(error) => app.error(error),
            });
        });
    }
}

/// pi's `reportBug`, writing the zip to `dir` and filing the issue with
/// the GitHub CLI at `gh` unless `offline`: the status to show, or the
/// error.
async fn run(
    ask: &Ask,
    session: &AgentSession,
    hint: &str,
    dir: &Path,
    offline: bool,
    gh: &OsStr,
) -> Result<String, String> {
    let description = format!("{DISCLAIMER}\n\nWhat went wrong? (optional)");
    let Some(hint) = ask.text("Report a bug", description, hint).await else {
        return Ok(CANCELLED.to_owned());
    };
    let transcript = "Include the session transcript?";
    let Some(include_session) = ask
        .confirm(transcript, TRANSCRIPT_NOTE, "Yes, include the transcript")
        .await
    else {
        return Ok(CANCELLED.to_owned());
    };
    let model = session.model();
    let name = model
        .as_ref()
        .map_or("the current model", |model| model.name.as_str());
    let mut include_summary = false;
    if !include_session {
        let provider = model
            .as_ref()
            .map_or("your provider", |model| model.provider.as_str());
        let note = format!(
            "The transcript is sent to {provider} with your credentials and tokens. Only the generated summary is kept, and the transcript stays on your machine."
        );
        let title = format!("Attach a summary written by {name} instead?");
        match ask.confirm(&title, &note, "Yes, generate a summary").await {
            Some(yes) => include_summary = yes,
            None => return Ok(CANCELLED.to_owned()),
        }
    }
    let hint = hint.trim();
    let mut summary = None;
    if include_summary {
        let cancel = CancellationToken::new();
        let title = format!("Writing summary with {name}...");
        tokio::select! {
            written = session.bug_report_summary(Some(hint), cancel.clone()) => {
                ask.close_abandoned();
                let written = written.map_err(|error| format!("Failed to write bug report summary: {error}"))?;
                summary = Some(written);
            }
            _ = ask.choose(&title, None, &["Cancel"]) => {
                cancel.cancel();
                return Ok(CANCELLED.to_owned());
            }
        }
    }
    let report = Report::collect(session, hint, include_session, summary);
    let path = dir.join(report.file_name());
    report
        .write_zip(&path)
        .map_err(|error| format!("Failed to write bug report: {error}"))?;
    let saved = format!(
        "Bug report exported to: {}\nReport ID: {}",
        path.display(),
        report.id
    );
    let issue = file_issue(ask, &report, offline, gh.to_owned()).await;
    let url = issue.as_ref().ok().and_then(Option::as_deref);
    let entry = report.entry(&path, url);
    let _ = session.with_session(|file| file.append_custom_entry(ENTRY_TYPE, Some(entry)));
    match issue {
        Ok(Some(url)) => Ok(format!(
            "Bug report issue created: {url}\n{saved}\nAttach the zip file to the issue yourself if you want to share it."
        )),
        Ok(None) => Ok(format!("{saved}\nReport it at {ISSUES_URL}")),
        Err(error) => Err(format!(
            "Failed to create the GitHub issue: {error}\n{saved}"
        )),
    }
}

/// Files `report` as a public issue when `gh` is ready and the user agrees:
/// the issue's URL, `None` when none was filed, or why `gh` failed.
async fn file_issue(
    ask: &Ask,
    report: &Report,
    offline: bool,
    gh: OsString,
) -> Result<Option<String>, String> {
    let program = gh.clone();
    let ready = tokio::task::spawn_blocking(move || bug_report::github_ready(&program, offline));
    if !ready.await.unwrap_or(false) {
        return Ok(None);
    }
    let title = report.title();
    let note = format!(
        "Title: {title}\n\nThe issue on {REPO} is public. It holds your description, the summary and report.json. The transcript is not posted."
    );
    let create = "Create a public GitHub issue?";
    if ask.confirm(create, &note, "Yes, create the issue").await != Some(true) {
        return Ok(None);
    }
    let body = report.issue_body(&yapi_core::tools::path::home_dir());
    let body_file: PathBuf = std::env::temp_dir().join(format!("yapi-bug-report-{}.md", report.id));
    let created = tokio::task::spawn_blocking(move || {
        bug_report::create_issue(&gh, &title, &body, &body_file)
    });
    created.await.map_err(|error| error.to_string())?.map(Some)
}

#[cfg(test)]
mod tests {
    use yapi_types::session::FileEntry;

    use super::super::selectors::{Action, Outcome};
    use super::*;

    /// Runs `/bug` to its end, answering its dialogs in order: `None`
    /// presses Escape, text answers a text dialog, and a number picks a
    /// choice dialog's option. Returns the outcome and the session.
    async fn drive(
        dir: &Path,
        offline: bool,
        gh: &Path,
        answers: &[Option<&str>],
    ) -> (Result<String, String>, AgentSession) {
        let (mut app, mut events) = super::super::tests::app();
        let ask = Ask {
            tx: app.tx.clone(),
            epoch: app.epoch,
        };
        let session = app.session.clone();
        let (dir, gh) = (dir.to_owned(), gh.as_os_str().to_owned());
        let mut task =
            tokio::spawn(async move { run(&ask, &session, "", &dir, offline, &gh).await });
        let mut answers = answers.iter();
        loop {
            tokio::select! {
                outcome = &mut task => return (outcome.unwrap(), app.session.clone()),
                Some(event) = events.recv() => {
                    app.on_event(event);
                    if app.selector.is_none() {
                        continue;
                    }
                    let outcome = match answers.next().expect("an answer for the dialog") {
                        None => Outcome::Cancel,
                        Some(text) if matches!(app.selector, Some(Selector::Text(_))) => {
                            Outcome::Done(Action::Text((*text).to_owned()))
                        }
                        Some(index) => Outcome::Done(Action::Choice(index.parse().unwrap())),
                    };
                    app.selector_outcome(outcome);
                }
            }
        }
    }

    /// The session's custom entries: their types and data.
    fn custom_entries(session: &AgentSession) -> Vec<(String, serde_json::Value)> {
        session.with_session(|file| {
            file.entries()
                .filter_map(|entry| match entry {
                    FileEntry::Custom(entry) => Some((
                        entry.custom_type.clone(),
                        entry.data.clone().unwrap_or_default(),
                    )),
                    _ => None,
                })
                .collect()
        })
    }

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("yapi-bug-{name}-{}", yapi_core::time::uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A signed-out `gh` that logs its arguments.
    #[cfg(unix)]
    fn signed_out_gh(dir: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let gh = dir.join("gh");
        let script = format!(
            "#!/bin/sh\necho \"$*\" >> '{}/log'\nexit 1\n",
            dir.display()
        );
        std::fs::write(&gh, script).unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
        gh
    }

    #[tokio::test]
    async fn cancelling_before_the_zip_writes_nothing() {
        let (out, bin) = (temp("out"), temp("bin"));
        for answers in [
            &[None][..],
            &[Some("it broke"), None],
            &[Some(""), Some("1"), None],
        ] {
            let (outcome, session) = drive(&out, false, &bin.join("gh"), answers).await;
            assert_eq!(outcome, Ok(CANCELLED.to_owned()));
            assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
            assert!(custom_entries(&session).is_empty());
        }
        let _ = std::fs::remove_dir_all(&out);
        let _ = std::fs::remove_dir_all(&bin);
    }

    /// Offline, and with `gh` signed out, the report stops at the zip:
    /// `gh` does not start, or answers only `auth status`.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_zip_is_the_report_when_offline_or_signed_out() {
        for (offline, log) in [(true, None), (false, Some("auth status\n"))] {
            let (out, bin) = (temp("out"), temp("bin"));
            let gh = signed_out_gh(&bin);
            let answers = [Some("it broke"), Some("1"), Some("1")];
            let (outcome, session) = drive(&out, offline, &gh, &answers).await;
            let outcome = outcome.unwrap();
            assert!(
                outcome.ends_with(&format!("Report it at {ISSUES_URL}")),
                "{outcome}"
            );
            assert_eq!(
                std::fs::read_to_string(bin.join("log")).ok().as_deref(),
                log
            );
            let zips: Vec<PathBuf> = std::fs::read_dir(&out)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            assert_eq!(zips.len(), 1);
            let name = zips[0].file_name().unwrap().to_str().unwrap().to_owned();
            assert!(name.starts_with("yapi-bug-report-") && name.ends_with(".zip"));
            let entries = custom_entries(&session);
            let [(kind, data)] = &entries[..] else {
                panic!("one custom entry: {entries:?}");
            };
            assert_eq!(kind, "yapi.bug-report");
            assert_eq!(data["hint"], "it broke");
            assert_eq!(data["delivery"], "zip");
            assert_eq!(data["sessionIncluded"], false);
            assert_eq!(data["path"], zips[0].display().to_string());
            assert!(data.get("issueUrl").is_none());
            let _ = std::fs::remove_dir_all(&out);
            let _ = std::fs::remove_dir_all(&bin);
        }
    }
}
