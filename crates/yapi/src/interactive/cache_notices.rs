//! The transcript's cost notices, shown while `showCacheMissNotices` is on:
//! cache warming refreshes, cache misses, summary costs and the thinking
//! blocks Anthropic dropped from a request.
//!
//! Port of those notices in `modes/interactive/interactive-mode.ts` in pi
//! `v1.0.0`.

use yapi_core::agent_session::{failed, format_cache_warming_usage};
use yapi_core::cache_stats::{CACHE_TTL_MS, CacheMiss, collect_cache_misses, detect_cache_miss};
use yapi_types::message::{AssistantMessage, Message, Usage};
use yapi_types::session::{FileEntry, UsageEntry};

use super::App;
use super::footer::format_tokens;

/// A message of the branch's context, or a notice to show after the
/// messages before it, with its theme color.
pub(super) enum Shown {
    Message(Box<Message>),
    Notice(String, &'static str),
}

/// pi's `addCacheMissNotice`: misses under 20,000 tokens and $0.10 are not
/// worth a notice.
fn miss_notice(miss: &CacheMiss) -> Option<String> {
    if miss.missed_tokens < 20_000 && miss.missed_cost < 0.1 {
        return None;
    }
    let cost = if miss.missed_cost >= 0.01 {
        format!(" (~${:.2})", miss.missed_cost)
    } else {
        String::new()
    };
    let label = if miss.model_changed {
        "Cache miss after model switch".to_owned()
    } else if miss.idle_ms >= CACHE_TTL_MS {
        format!(
            "Cache miss after {}m idle",
            (miss.idle_ms as f64 / 60_000.0).round()
        )
    } else {
        "Cache miss".to_owned()
    };
    Some(format!(
        "{label}: {} tokens re-billed{cost}",
        format_tokens(miss.missed_tokens)
    ))
}

/// pi's `addCompactionCostNotice`: what a compaction or branch summary
/// request billed.
fn summary_notice(label: &str, usage: &Usage) -> String {
    let tokens = usage.input + usage.output + usage.cache_read + usage.cache_write;
    let cost = if usage.cost.total >= 0.01 {
        format!(" (~${:.2})", usage.cost.total)
    } else {
        String::new()
    };
    format!("{label}: {} tokens billed{cost}", format_tokens(tokens))
}

/// Thinking blocks Anthropic reports it dropped from the request.
fn dropped_thinking_blocks(message: &AssistantMessage) -> usize {
    message
        .diagnostics
        .iter()
        .flatten()
        .filter(|diagnostic| diagnostic["type"] == "anthropic_input_transformations")
        .filter_map(|diagnostic| diagnostic["details"]["transformations"].as_array())
        .flatten()
        .filter(|transformation| transformation["type"] == "thinking_dropped")
        .count()
}

impl App {
    /// Whether the cost notices show.
    pub(super) fn cache_notices(&self) -> bool {
        self.session
            .settings()
            .show_cache_miss_notices
            .unwrap_or(false)
    }

    /// pi's `addCacheWarmingUsage`.
    pub(super) fn cache_warmed_notice(&mut self, entry: &UsageEntry) {
        if self.cache_notices() {
            self.note(format_cache_warming_usage(entry), "dim");
        }
    }

    /// The notice for a finished compaction's cost.
    pub(super) fn compaction_notice(&mut self, usage: &Usage) {
        if self.cache_notices() {
            self.note(summary_notice("Compaction", usage), "warning");
        }
    }

    /// pi's notices for a response that just finished: the thinking blocks
    /// Anthropic dropped, when more than in the previous response, and the
    /// cache miss it paid for. The session may already hold `message`.
    pub(super) fn response_notices(&mut self, message: &AssistantMessage) {
        if !self.cache_notices() {
            return;
        }
        let registry = self.session.registry();
        let (previous_dropped, miss) = self.session.with_session(|session| {
            let previous =
                session
                    .branch_path(None)
                    .into_iter()
                    .rev()
                    .find_map(|entry| match entry {
                        FileEntry::Message(entry) => match &entry.message {
                            Message::Assistant(other) if **other != *message => {
                                Some(dropped_thinking_blocks(other))
                            }
                            _ => None,
                        },
                        _ => None,
                    });
            (
                previous.unwrap_or(0),
                detect_cache_miss(session.entries(), message, &registry),
            )
        });
        let dropped = dropped_thinking_blocks(message);
        if dropped > previous_dropped {
            let noun = if dropped == 1 {
                "thinking block"
            } else {
                "thinking blocks"
            };
            self.note(
                format!("Anthropic dropped {dropped} {noun} (details in session)"),
                "warning",
            );
        }
        if let Some(text) = miss.as_ref().and_then(miss_notice) {
            self.note(text, "warning");
        }
    }

    /// The current branch's context entry by entry, with the cost notices
    /// for refreshes, summaries and cache misses pi shows when the
    /// transcript is drawn again. Without `summary`, the latest
    /// compaction's summary and its notice are left out.
    pub(super) fn shown_history(&self, summary: bool) -> Vec<Shown> {
        let notices = self.cache_notices();
        let registry = self.session.registry();
        self.session.with_session(|session| {
            let misses = if notices {
                collect_cache_misses(session.entries(), &registry)
            } else {
                Vec::new()
            };
            let projection = session.build_projection();
            let skip = usize::from(
                !summary
                    && projection
                        .entries
                        .first()
                        .is_some_and(|entry| matches!(entry.source, FileEntry::Compaction(_))),
            );
            let mut shown = Vec::new();
            for entry in projection.entries.into_iter().skip(skip) {
                if let FileEntry::Usage(usage) = entry.source
                    && usage.kind == "cache_warm"
                {
                    if notices {
                        shown.push(Shown::Notice(format_cache_warming_usage(usage), "dim"));
                    }
                    continue;
                }
                let id = entry.source.meta().map(|meta| meta.id.as_str());
                let summary_usage = match entry.source {
                    FileEntry::Compaction(compaction) => {
                        compaction.usage.as_ref().map(|usage| ("Compaction", usage))
                    }
                    FileEntry::BranchSummary(branch) => {
                        branch.usage.as_ref().map(|usage| ("Branch summary", usage))
                    }
                    _ => None,
                }
                .filter(|_| notices && !entry.messages.is_empty());
                for message in entry.messages {
                    let miss = match &message {
                        Message::Assistant(assistant) if !failed(assistant) => misses
                            .iter()
                            .find(|(missed, _)| Some(missed.as_str()) == id)
                            .and_then(|(_, miss)| miss_notice(miss)),
                        _ => None,
                    };
                    shown.push(Shown::Message(Box::new(message)));
                    if let Some(text) = miss {
                        shown.push(Shown::Notice(text, "warning"));
                    }
                }
                if let Some((label, usage)) = summary_usage {
                    shown.push(Shown::Notice(summary_notice(label, usage), "warning"));
                }
            }
            shown
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_cache_misses_as_pi() {
        let miss = CacheMiss {
            missed_tokens: 21_100,
            missed_cost: 0.0633,
            idle_ms: 0,
            model_changed: false,
        };
        assert_eq!(
            miss_notice(&miss).as_deref(),
            Some("Cache miss: 21k tokens re-billed (~$0.06)")
        );
        let idle = CacheMiss {
            idle_ms: 7 * 60_000 + 40_000,
            missed_tokens: 5_000,
            missed_cost: 0.25,
            ..miss
        };
        assert_eq!(
            miss_notice(&idle).as_deref(),
            Some("Cache miss after 8m idle: 5.0k tokens re-billed (~$0.25)")
        );
        let small = CacheMiss {
            missed_tokens: 19_999,
            missed_cost: 0.09,
            ..miss
        };
        assert_eq!(miss_notice(&small), None);
    }
}
