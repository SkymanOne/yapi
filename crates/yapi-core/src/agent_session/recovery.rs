//! What happens after a run: automatic retries, overflow recovery and
//! compaction, and manual compaction.

use std::sync::atomic::Ordering;

use tokio_util::sync::CancellationToken;
use yapi_ai::errors::{is_context_overflow, is_recoverable_length, is_retryable_assistant_error};
use yapi_types::event::AgentEvent;
use yapi_types::event::{CompactionReason, CompactionResult, SummarySource};
use yapi_types::extension_event::ExtensionEvent;
use yapi_types::message::{AssistantMessage, Message, StopReason, ThinkingLevel};
use yapi_types::model::Model;
use yapi_types::session::FileEntry;
use yapi_types::sync::lock;

use crate::compaction::{
    CompactionSettings, Preparation, RetryPolicy, Summarizer, SummaryRetry,
    calculate_context_tokens, estimate_context_tokens, estimate_projected_context_tokens,
    estimate_tokens, prepare_compaction, should_compact,
};
use crate::session::build_projection;
use crate::time::parse_iso;

use super::{AgentSession, Compacting};

/// What the compaction check decided.
enum CompactionCheck {
    None,
    Overflow { will_retry: bool },
    OverflowFailed(String),
    Threshold,
}

impl AgentSession {
    /// Cancels the wait before an automatic retry; the failed response stands.
    pub fn abort_retry(&self) {
        let cancel = lock(&self.inner.retry_cancel).clone();
        if let Some(cancel) = cancel {
            // The retry's end is reported before this returns, as pi reports
            // it before answering `abort_retry`; the waiting retry then finds
            // nothing left to report.
            self.finish_cancelled_retry();
            cancel.cancel();
        }
    }

    /// The model whose limits apply to a response: the current one when it made it.
    fn model_for_message(&self, message: &AssistantMessage) -> Option<Model> {
        self.model()
            .filter(|model| model.is(&message.provider, &message.model))
    }

    fn is_retryable(&self, message: &AssistantMessage) -> bool {
        let window = self
            .model_for_message(message)
            .or_else(|| self.model())
            .map_or(0, |model| model.context_window);
        !is_context_overflow(message, window) && is_retryable_assistant_error(message)
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::resolve(lock(&self.inner.settings).settings())
    }

    /// Whether the run that just ended will be retried, for `agent_end`.
    pub(super) fn will_retry_after(&self, messages: &[Message]) -> bool {
        let cancelled = lock(&self.inner.cancel)
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled);
        let policy = self.retry_policy();
        if cancelled
            || !policy.enabled
            || lock(&self.inner.recovery).retry_attempt >= policy.max_retries
        {
            return false;
        }
        messages
            .iter()
            .rev()
            .find_map(|message| match message {
                Message::Assistant(assistant) => Some(self.is_retryable(assistant)),
                _ => None,
            })
            .unwrap_or(false)
    }

    pub(super) fn finish_cancelled_retry(&self) {
        let attempt = std::mem::take(&mut lock(&self.inner.recovery).retry_attempt);
        if attempt > 0 {
            self.emit(&AgentEvent::AutoRetryEnd {
                success: false,
                attempt,
                final_error: Some("Retry cancelled".into()),
            });
        }
    }

    /// Drops a failed or truncated attempt from the model's context while keeping
    /// it in the session history.
    fn omit_recovery_attempt(&self, entry_ids: &[String]) {
        for id in entry_ids {
            let entry = self.with_session(|session| {
                let edit = session.append_context_edit(id, None).ok()?;
                session.entry(&edit).cloned()
            });
            if let Some(entry) = entry {
                self.emit(&AgentEvent::EntryAppended { entry });
            }
        }
    }

    async fn prepare_retry(
        &self,
        message: &AssistantMessage,
        entry_id: Option<String>,
        cancel: &CancellationToken,
    ) -> bool {
        let policy = self.retry_policy();
        if !policy.enabled {
            return false;
        }
        let attempt = {
            let mut recovery = lock(&self.inner.recovery);
            if recovery.retry_attempt >= policy.max_retries {
                return false;
            }
            recovery.retry_attempt += 1;
            recovery.retry_attempt
        };
        let delay_ms = policy.delay_ms(attempt);
        self.emit(&AgentEvent::AutoRetryStart {
            attempt,
            max_attempts: policy.max_retries,
            delay_ms,
            error_message: message
                .error_message
                .clone()
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| "Unknown error".into()),
        });
        self.omit_recovery_attempt(&entry_id.into_iter().collect::<Vec<_>>());
        let retry = cancel.child_token();
        *lock(&self.inner.retry_cancel) = Some(retry.clone());
        let waited = tokio::select! {
            () = retry.cancelled() => {
                self.finish_cancelled_retry();
                false
            }
            () = tokio::time::sleep(std::time::Duration::from_millis(delay_ms)) => true,
        };
        *lock(&self.inner.retry_cancel) = None;
        waited
    }

    /// pi's post-run step: whether to run the agent again, or the error of
    /// an invalid compaction setting, which ends the prompt.
    pub(super) async fn handle_post_run(&self, cancel: &CancellationToken) -> Result<bool, String> {
        let (last, tool_results) = {
            let mut recovery = lock(&self.inner.recovery);
            (
                recovery.last_assistant.take(),
                std::mem::take(&mut recovery.last_tool_results),
            )
        };
        if cancel.is_cancelled() {
            self.finish_cancelled_retry();
            return Ok(false);
        }
        let Some((message, entry_id)) = last else {
            return Ok(self.has_queued());
        };
        if self.is_retryable(&message)
            && self.prepare_retry(&message, entry_id.clone(), cancel).await
        {
            return Ok(!cancel.is_cancelled());
        }
        if cancel.is_cancelled() {
            self.finish_cancelled_retry();
            return Ok(false);
        }
        if message.stop_reason == StopReason::Error {
            let attempt = std::mem::take(&mut lock(&self.inner.recovery).retry_attempt);
            if attempt > 0 {
                self.emit(&AgentEvent::AutoRetryEnd {
                    success: false,
                    attempt,
                    final_error: message.error_message.clone(),
                });
            }
        }
        if self
            .check_compaction(&message, entry_id, tool_results, cancel, true)
            .await?
        {
            return Ok(!cancel.is_cancelled());
        }
        Ok(!cancel.is_cancelled() && self.has_queued())
    }

    /// pi's check before a prompt is sent: [`Self::check_compaction`] for
    /// the context's last response, aborted ones included. It catches
    /// aborted responses and sessions resumed above the threshold. An abort
    /// stops the compaction, not the prompt. An invalid compaction setting
    /// fails the prompt with its error, as pi's check throws.
    pub(super) async fn compact_before_prompt(&self) -> Result<(), String> {
        let last = self.with_session(|session| {
            let branch = session.branch_path(None);
            build_projection(&branch)
                .entries
                .iter()
                .rev()
                .find_map(|entry| {
                    let message =
                        entry
                            .messages
                            .iter()
                            .rev()
                            .find_map(|message| match message {
                                Message::Assistant(assistant) => Some((**assistant).clone()),
                                _ => None,
                            })?;
                    Some((message, entry.source.meta().map(|meta| meta.id.clone())))
                })
        });
        let Some((message, entry_id)) = last else {
            return Ok(());
        };
        let cancel = CancellationToken::new();
        *lock(&self.inner.cancel) = Some(cancel.clone());
        let checked = self
            .check_compaction(&message, entry_id, Vec::new(), &cancel, false)
            .await;
        *lock(&self.inner.cancel) = None;
        checked.map(drop)
    }

    /// pi's `_compactBeforeNextAssistantResponse`, before each turn of a run
    /// after the first: threshold compaction when the context the next
    /// request would send exceeds the threshold. An invalid compaction
    /// setting is the error that ends the run, as pi's hook throws.
    pub(super) async fn compact_before_next_response(
        &self,
        cancel: &CancellationToken,
    ) -> Result<(), String> {
        let Some(model) = self.model().filter(|model| model.context_window > 0) else {
            return Ok(());
        };
        if let Some(error) = lock(&self.inner.settings).compaction_error(Some(&model)) {
            return Err(error);
        }
        let settings =
            CompactionSettings::resolve(lock(&self.inner.settings).settings(), Some(&model));
        let tokens = self.with_session(|session| {
            let branch = session.branch_path(None);
            estimate_projected_context_tokens(&build_projection(&branch), &branch).tokens
        });
        if should_compact(tokens, model.context_window, &settings) {
            self.run_auto_compaction(CompactionReason::Threshold, false, cancel)
                .await;
        }
        Ok(())
    }

    /// pi's `_checkCompaction`: overflow recovery, then threshold compaction,
    /// or the error of an invalid compaction setting. An aborted `message` is
    /// skipped when `skip_aborted`.
    async fn check_compaction(
        &self,
        message: &AssistantMessage,
        entry_id: Option<String>,
        tool_results: Vec<String>,
        cancel: &CancellationToken,
        skip_aborted: bool,
    ) -> Result<bool, String> {
        let model = self.model();
        if let Some(error) = lock(&self.inner.settings).compaction_error(model.as_ref()) {
            return Err(error);
        }
        let Some(model) = model else {
            return Ok(false);
        };
        let settings =
            CompactionSettings::resolve(lock(&self.inner.settings).settings(), Some(&model));
        if !settings.enabled || (skip_aborted && message.stop_reason == StopReason::Aborted) {
            return Ok(false);
        }
        let message_model = self.model_for_message(message);
        let context_window = message_model.as_ref().unwrap_or(&model).context_window;
        let overflow_attempted = lock(&self.inner.recovery).overflow_recovery_attempted;
        let check = self.with_session(|session| {
            let branch = session.branch_path(None);
            let latest_compaction = branch.iter().rev().find_map(|entry| match entry {
                FileEntry::Compaction(compaction) => Some(compaction),
                _ => None,
            });
            let compaction_time =
                latest_compaction.and_then(|compaction| parse_iso(&compaction.meta.timestamp));
            if compaction_time.is_some_and(|time| message.timestamp <= time) {
                return CompactionCheck::None;
            }
            let projection = build_projection(&branch);
            let entry_id = entry_id.as_deref();
            let projected = entry_id.is_none_or(|id| {
                projection.entries.iter().any(|entry| {
                    entry.source.meta().is_some_and(|meta| meta.id == id)
                        && entry
                            .messages
                            .iter()
                            .any(|message| matches!(message, Message::Assistant(_)))
                })
            });
            let after: Vec<&FileEntry> = entry_id
                .and_then(|id| {
                    branch
                        .iter()
                        .position(|entry| entry.meta().is_some_and(|meta| meta.id == id))
                })
                .map(|index| branch[index + 1..].to_vec())
                .unwrap_or_default();
            let post_edit = after
                .iter()
                .any(|entry| matches!(entry, FileEntry::ContextEdit(_)));
            let latest_edit = after.iter().rev().find_map(|entry| match entry {
                FileEntry::ContextEdit(edit) if Some(edit.target_id.as_str()) == entry_id => {
                    Some(edit)
                }
                _ => None,
            });
            let retained = entry_id.is_none()
                || (!after
                    .iter()
                    .any(|entry| matches!(entry, FileEntry::Compaction(_)))
                    && latest_edit.is_none_or(|edit| edit.replacement.is_some()));
            let usage_matches = projected && !post_edit;
            let explicit = message.stop_reason == StopReason::Error && is_context_overflow(message, 0);
            let same_model = message_model.is_some();
            let overflow = same_model
                && ((explicit && retained)
                    || (usage_matches && is_context_overflow(message, context_window)));
            let length = message_model.as_ref().is_some_and(|model| {
                projected && is_recoverable_length(message, model.max_tokens)
            });
            if overflow || length {
                let will_retry = message.stop_reason != StopReason::Stop;
                if !will_retry {
                    return CompactionCheck::Overflow { will_retry: false };
                }
                if overflow_attempted {
                    return CompactionCheck::OverflowFailed(if overflow {
                        "Context overflow recovery failed after one compact-and-retry attempt. Try reducing context or switching to a larger-context model.".into()
                    } else {
                        "Truncated response recovery failed after one compact-and-retry attempt.".into()
                    });
                }
                return CompactionCheck::Overflow { will_retry: true };
            }
            let has_edits = projection
                .entries
                .iter()
                .any(|entry| matches!(entry.source, FileEntry::ContextEdit(_)));
            let direct = calculate_context_tokens(&message.usage);
            let tokens = if has_edits {
                estimate_projected_context_tokens(&projection, &branch).tokens
            } else if message.stop_reason == StopReason::Error || direct == 0 {
                let estimate = estimate_context_tokens(&projection.messages);
                if let Some(index) = estimate.last_usage_index
                    && let Message::Assistant(usage_message) = &projection.messages[index]
                    && compaction_time.is_some_and(|time| usage_message.timestamp <= time)
                {
                    return CompactionCheck::None;
                }
                estimate.tokens
            } else {
                direct
            };
            if should_compact(tokens, context_window, &settings) {
                CompactionCheck::Threshold
            } else {
                CompactionCheck::None
            }
        });
        Ok(match check {
            CompactionCheck::None => false,
            CompactionCheck::Threshold => {
                self.run_auto_compaction(CompactionReason::Threshold, false, cancel)
                    .await
            }
            CompactionCheck::OverflowFailed(error_message) => {
                self.compaction_failed(CompactionReason::Overflow, Some(error_message), false)
                    .await;
                false
            }
            CompactionCheck::Overflow { will_retry } => {
                if will_retry {
                    lock(&self.inner.recovery).overflow_recovery_attempted = true;
                    let ids: Vec<String> = entry_id.into_iter().chain(tool_results).collect();
                    self.omit_recovery_attempt(&ids);
                }
                self.run_auto_compaction(CompactionReason::Overflow, will_retry, cancel)
                    .await
            }
        })
    }

    /// A summarizer for `model` with the session's credentials and retry
    /// policy, reporting retries as `source` summaries.
    pub(super) async fn summarizer<'a>(
        &'a self,
        model: &'a Model,
        thinking_level: ThinkingLevel,
        source: SummarySource,
        reason: Option<CompactionReason>,
        cancel: CancellationToken,
    ) -> Summarizer<'a> {
        Summarizer {
            model,
            apis: self.apis(),
            auth: self.registry().auth(model).await,
            thinking_level,
            retry: self.retry_policy(),
            request: self.request_options(),
            hooks: yapi_ai::stream::RequestHooks {
                headers: self.headers_hook(&cancel),
                ..Default::default()
            },
            cancel,
            on_retry: Box::new(move |retry| self.emit_summary_retry(retry, source, reason)),
        }
    }

    /// pi's `summarizeForBugReport`: a bug report the current model writes
    /// from the end of the context, with the user's `hint`, or why it could
    /// not. `cancel` stops the request.
    pub async fn bug_report_summary(
        &self,
        hint: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<String, String> {
        let model = self.model().ok_or_else(|| "No model selected".to_owned())?;
        let mut summarizer = self
            .summarizer(
                &model,
                self.thinking_level(),
                SummarySource::Compaction,
                None,
                cancel,
            )
            .await;
        // pi retries a bug report summary without reporting it.
        summarizer.on_retry = Box::new(|_| {});
        crate::bug_report::summary(&summarizer, &self.messages(), hint).await
    }

    /// What manual and automatic compaction share: pi's
    /// `session_before_compact` event, which may cancel the compaction or
    /// supply its summary; otherwise a summary of `preparation`. The
    /// compaction is recorded with the estimate after it and reported as
    /// `session_compact`, unless `cancel` fired meanwhile.
    async fn run_compaction(
        &self,
        model: &Model,
        preparation: &Preparation,
        custom_instructions: Option<&str>,
        (reason, will_retry): (CompactionReason, bool),
        cancel: &CancellationToken,
    ) -> Compacted {
        let mut from_extension = false;
        let mut supplied = None;
        if self.has_handlers("session_before_compact") {
            let preparation = serde_json::to_value(preparation).unwrap_or_default();
            let event = self.with_session(|session| {
                ExtensionEvent::SessionBeforeCompact {
                    preparation,
                    branch_entries: &session.branch_path(None),
                    custom_instructions,
                    reason,
                    will_retry,
                }
                .to_value()
            });
            let result = self.emit_extension_event(&event, cancel.clone()).await;
            if result
                .as_ref()
                .is_some_and(|result| result["cancel"] == true)
            {
                return Compacted {
                    outcome: Err("Compaction cancelled".into()),
                    from_extension,
                    refused: true,
                };
            }
            supplied = result.and_then(|result| {
                serde_json::from_value::<CompactionResult>(result["compaction"].clone()).ok()
            });
            from_extension = supplied.is_some();
        }
        let outcome = async {
            let mut result = match supplied {
                Some(result) => result,
                None => {
                    let summarizer = self
                        .summarizer(
                            model,
                            self.thinking_level(),
                            SummarySource::Compaction,
                            Some(reason),
                            cancel.clone(),
                        )
                        .await;
                    summarizer.compact(preparation, custom_instructions).await?
                }
            };
            // As pi, a summary cut short by an abort is not recorded.
            if cancel.is_cancelled() {
                return Err("Compaction cancelled".to_owned());
            }
            let (entry, estimate) = self.with_session(|session| {
                let id = session
                    .append_compaction(
                        result.summary.clone(),
                        Some(result.first_kept_entry_id.clone()),
                        result.tokens_before,
                        result.details.clone(),
                        Some(from_extension),
                        result.usage.clone(),
                    )
                    .ok();
                let estimate = session
                    .build_context()
                    .messages
                    .iter()
                    .map(estimate_tokens)
                    .sum();
                (id.and_then(|id| session.entry(&id).cloned()), estimate)
            });
            result.estimated_tokens_after = Some(estimate);
            if let Some(entry) = entry {
                let event = ExtensionEvent::SessionCompact {
                    compaction_entry: &entry,
                    from_extension,
                    reason,
                    will_retry,
                };
                self.emit_extension_event(&event.to_value(), CancellationToken::new())
                    .await;
            }
            Ok(result)
        }
        .await;
        Compacted {
            outcome,
            from_extension,
            refused: false,
        }
    }

    /// pi's `compaction_end` for a compaction that did not happen, then its
    /// `session_compact_failed` event for extensions.
    async fn compaction_failed(
        &self,
        reason: CompactionReason,
        error_message: Option<String>,
        from_extension: bool,
    ) {
        let aborted = error_message.is_none();
        self.emit_compaction_end(reason, Err(error_message.clone()), false);
        if !self.has_handlers("session_compact_failed") {
            return;
        }
        let event = ExtensionEvent::SessionCompactFailed {
            reason,
            error_message: error_message.as_deref(),
            aborted,
            will_retry: false,
            from_extension,
        };
        self.emit_extension_event(&event.to_value(), CancellationToken::new())
            .await;
    }

    /// pi's `compaction_end` event: with the result, or aborted (`Err(None)`),
    /// or failed with an error message.
    fn emit_compaction_end(
        &self,
        reason: CompactionReason,
        outcome: Result<CompactionResult, Option<String>>,
        will_retry: bool,
    ) {
        let (result, aborted, error_message) = match outcome {
            Ok(result) => (Some(result), false, None),
            Err(None) => (None, true, None),
            Err(Some(error)) => (None, false, Some(error)),
        };
        self.emit(&AgentEvent::CompactionEnd {
            reason,
            result,
            aborted,
            will_retry,
            error_message,
        });
    }

    /// pi's `summarization_retry_*` events for a summary request's retries.
    fn emit_summary_retry(
        &self,
        retry: SummaryRetry,
        source: SummarySource,
        reason: Option<CompactionReason>,
    ) {
        self.emit(&match retry {
            SummaryRetry::Scheduled {
                attempt,
                max_attempts,
                delay_ms,
                error_message,
            } => AgentEvent::SummarizationRetryScheduled {
                attempt,
                max_attempts,
                delay_ms,
                error_message,
            },
            SummaryRetry::AttemptStart => {
                AgentEvent::SummarizationRetryAttemptStart { source, reason }
            }
            SummaryRetry::Finished => AgentEvent::SummarizationRetryFinished,
        });
    }

    async fn run_auto_compaction(
        &self,
        reason: CompactionReason,
        will_retry: bool,
        cancel: &CancellationToken,
    ) -> bool {
        let Some(model) = self.model() else {
            return false;
        };
        let settings =
            CompactionSettings::resolve(lock(&self.inner.settings).settings(), Some(&model));
        let Some(preparation) =
            self.with_session(|session| prepare_compaction(&session.branch_path(None), settings))
        else {
            return false;
        };
        self.emit(&AgentEvent::CompactionStart { reason });
        let compacting = Compacting::start(&self.inner.compacting);
        let compacted = self
            .run_compaction(&model, &preparation, None, (reason, will_retry), cancel)
            .await;
        drop(compacting);
        self.inner.idle.notify_waiters();
        match compacted.outcome {
            Ok(result) => {
                self.emit_compaction_end(reason, Ok(result), will_retry);
                will_retry || self.has_queued()
            }
            Err(message) => {
                let aborted = cancel.is_cancelled() || compacted.refused;
                let error_message = (!aborted).then(|| match reason {
                    CompactionReason::Overflow => {
                        format!("Context overflow recovery failed: {message}")
                    }
                    _ => format!("Auto-compaction failed: {message}"),
                });
                self.compaction_failed(reason, error_message, compacted.from_extension)
                    .await;
                false
            }
        }
    }

    /// Compacts the session now, optionally focused by `custom_instructions`.
    /// Aborts a running response first.
    pub async fn compact(
        &self,
        custom_instructions: Option<&str>,
    ) -> Result<CompactionResult, String> {
        self.abort();
        self.wait_for_idle().await;
        let cancel = CancellationToken::new();
        *lock(&self.inner.cancel) = Some(cancel.clone());
        let compacting = Compacting::start(&self.inner.compacting);
        self.inner.manual_compaction.store(true, Ordering::SeqCst);
        self.emit(&AgentEvent::CompactionStart {
            reason: CompactionReason::Manual,
        });
        let outcome = async {
            let model = self
                .model()
                .ok_or_else(|| crate::auth_guidance::no_model_selected(&self.inner.docs))?;
            let settings =
                CompactionSettings::resolve(lock(&self.inner.settings).settings(), Some(&model));
            let preparation = self.with_session(|session| {
                let branch = session.branch_path(None);
                match prepare_compaction(&branch, settings) {
                    Some(preparation) => Ok(preparation),
                    None if matches!(branch.last(), Some(FileEntry::Compaction(_))) => {
                        Err("Already compacted".to_owned())
                    }
                    None => Err("Nothing to compact (session too small)".to_owned()),
                }
            })?;
            Ok(self
                .run_compaction(
                    &model,
                    &preparation,
                    custom_instructions,
                    (CompactionReason::Manual, false),
                    &cancel,
                )
                .await)
        }
        .await;
        let compacted = outcome.unwrap_or_else(|error| Compacted {
            outcome: Err(error),
            from_extension: false,
            refused: false,
        });
        *lock(&self.inner.cancel) = None;
        self.inner.manual_compaction.store(false, Ordering::SeqCst);
        drop(compacting);
        // Waiters for idle run once this returns, after its own outcome.
        self.inner.idle.notify_waiters();
        let reason = CompactionReason::Manual;
        match compacted.outcome {
            Ok(result) => {
                self.emit_compaction_end(reason, Ok(result.clone()), false);
                Ok(result)
            }
            Err(message) => {
                let aborted = cancel.is_cancelled() || compacted.refused;
                let error_message = (!aborted).then(|| format!("Compaction failed: {message}"));
                self.compaction_failed(reason, error_message, compacted.from_extension)
                    .await;
                // pi reports the summarizer's error, such as an aborted request.
                Err(message)
            }
        }
    }
}

/// A compaction's outcome, whether an extension supplied its summary, and
/// whether an extension cancelled it.
struct Compacted {
    outcome: Result<CompactionResult, String>,
    from_extension: bool,
    refused: bool,
}
