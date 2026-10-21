//! Prompt cache misses: prompt tokens that were in the previous request's
//! prompt but were billed again instead of read from the cache.
//!
//! Port of `packages/coding-agent/src/core/cache-stats.ts` in pi `v1.0.0`.

use yapi_ai::registry::ModelRegistry;
use yapi_types::message::{AssistantMessage, Message};
use yapi_types::session::FileEntry;

/// Idle gaps longer than this are worth naming as the likely cause of a
/// miss: Anthropic's default cache lifetime.
pub const CACHE_TTL_MS: u64 = 5 * 60 * 1000;

/// Per-request misses at or below this are cache breakpoint granularity
/// noise.
const NOISE_FLOOR_TOKENS: u64 = 1024;

/// A counted cache miss on one assistant message.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CacheMiss {
    /// Prompt tokens of the previous request that were not read from the
    /// cache.
    pub missed_tokens: u64,
    /// Dollars paid beyond a full cache hit; 0 when prices are unknown.
    pub missed_cost: f64,
    /// Milliseconds since the previous request, which last refreshed the
    /// cache.
    pub idle_ms: u64,
    /// The model changed since the previous request.
    pub model_changed: bool,
}

/// Counted misses across a session.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CacheWaste {
    /// Tokens billed again.
    pub missed_tokens: u64,
    /// Dollars paid beyond full cache hits.
    pub missed_cost: f64,
    /// Misses above the noise floor.
    pub miss_count: usize,
}

/// The last request a scan saw; everything in its prompt should be cached.
struct Previous {
    prompt_tokens: u64,
    model: String,
    /// `None` for a time that does not parse, which counts no idle time,
    /// as pi's `NaN` does.
    timestamp: Option<u64>,
    /// Some request of this scan segment reported cache activity, which
    /// tells a total miss on a provider that reports only cache reads from a
    /// provider that never reports caching.
    reported_cache: bool,
}

fn prompt_tokens(message: &AssistantMessage) -> u64 {
    let usage = &message.usage;
    usage.input + usage.cache_read + usage.cache_write
}

/// The miss `message` paid for after `previous`, unless there is none to
/// count: no previous request, no cache activity ever reported, or a miss
/// within the noise floor.
fn detect(
    previous: Option<&Previous>,
    message: &AssistantMessage,
    registry: &ModelRegistry,
) -> Option<CacheMiss> {
    let usage = &message.usage;
    let prompt = prompt_tokens(message);
    let previous = previous?;
    if prompt == 0 || (usage.cache_read + usage.cache_write == 0 && !previous.reported_cache) {
        return None;
    }
    let missed_tokens = previous
        .prompt_tokens
        .min(prompt)
        .saturating_sub(usage.cache_read);
    if missed_tokens <= NOISE_FLOOR_TOKENS {
        return None;
    }
    // Missed tokens land in the input or cache write buckets, so this
    // message's own costs give the rate paid for them.
    let paid_tokens = usage.input + usage.cache_write;
    let paid_per_token = if paid_tokens > 0 {
        (usage.cost.input + usage.cost.cache_write) / paid_tokens as f64
    } else {
        0.0
    };
    let read_per_token = if usage.cache_read > 0 {
        usage.cost.cache_read / usage.cache_read as f64
    } else {
        registry
            .find(&message.provider, &message.model)
            .map_or(0.0, |model| model.cost.cache_read)
            / 1_000_000.0
    };
    Some(CacheMiss {
        missed_tokens,
        missed_cost: missed_tokens as f64 * (paid_per_token - read_per_token).max(0.0),
        idle_ms: previous
            .timestamp
            .map_or(0, |at| message.timestamp.saturating_sub(at)),
        model_changed: format!("{}/{}", message.provider, message.model) != previous.model,
    })
}

/// Scans `entries` in order, calling `found` for each counted miss with the
/// entry that paid for it. Returns the last request seen.
fn scan<'a>(
    entries: impl IntoIterator<Item = &'a FileEntry>,
    registry: &ModelRegistry,
    mut found: impl FnMut(&'a FileEntry, CacheMiss),
) -> Option<Previous> {
    let mut previous: Option<Previous> = None;
    for entry in entries {
        match entry {
            // The context changed on purpose: the next prompt is new content,
            // not content billed again. A model switch is no such reset.
            FileEntry::Compaction(_) | FileEntry::BranchSummary(_) => previous = None,
            FileEntry::Usage(usage) if usage.kind == "cache_warm" => {
                let tokens = usage.usage.input + usage.usage.cache_read + usage.usage.cache_write;
                if tokens > 0 {
                    previous = Some(Previous {
                        prompt_tokens: tokens,
                        model: format!("{}/{}", usage.provider, usage.model),
                        timestamp: crate::time::parse_iso(&usage.meta.timestamp),
                        reported_cache: true,
                    });
                }
            }
            FileEntry::Message(message_entry) => {
                let Message::Assistant(message) = &message_entry.message else {
                    continue;
                };
                if let Some(miss) = detect(previous.as_ref(), message, registry) {
                    found(entry, miss);
                }
                let usage = &message.usage;
                if prompt_tokens(message) > 0 {
                    previous = Some(Previous {
                        prompt_tokens: prompt_tokens(message),
                        model: format!("{}/{}", message.provider, message.model),
                        timestamp: Some(message.timestamp),
                        reported_cache: previous.as_ref().is_some_and(|p| p.reported_cache)
                            || usage.cache_read + usage.cache_write > 0,
                    });
                }
            }
            _ => {}
        }
    }
    previous
}

/// pi's `computeCacheWaste`: the misses over every entry of a session, in
/// file order.
pub fn compute_cache_waste<'a>(
    entries: impl IntoIterator<Item = &'a FileEntry>,
    registry: &ModelRegistry,
) -> CacheWaste {
    let mut waste = CacheWaste::default();
    scan(entries, registry, |_, miss| {
        waste.missed_tokens += miss.missed_tokens;
        waste.missed_cost += miss.missed_cost;
        waste.miss_count += 1;
    });
    waste
}

/// pi's `collectCacheMisses`: each counted miss with the id of the entry
/// whose message paid for it, for notices when the transcript is drawn
/// again.
pub fn collect_cache_misses<'a>(
    entries: impl IntoIterator<Item = &'a FileEntry>,
    registry: &ModelRegistry,
) -> Vec<(String, CacheMiss)> {
    let mut misses = Vec::new();
    scan(entries, registry, |entry, miss| {
        if let Some(meta) = entry.meta() {
            misses.push((meta.id.clone(), miss));
        }
    });
    misses
}

/// pi's `detectCacheMiss`: the miss a just-finished assistant `message`
/// paid for. `entries` may already hold the message; the scan stops there.
pub fn detect_cache_miss<'a>(
    entries: impl IntoIterator<Item = &'a FileEntry>,
    message: &AssistantMessage,
    registry: &ModelRegistry,
) -> Option<CacheMiss> {
    let before = entries.into_iter().take_while(|entry| {
        !matches!(entry, FileEntry::Message(entry)
            if matches!(&entry.message, Message::Assistant(other) if **other == *message))
    });
    let previous = scan(before, registry, |_, _| {});
    detect(previous.as_ref(), message, registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assistant(id: &str, timestamp: u64, input: u64, read: u64, write: u64) -> FileEntry {
        serde_json::from_value(json!({
            "type": "message", "id": id, "parentId": null, "timestamp": "2026-10-13T10:00:00.000Z",
            "message": {
                "role": "assistant", "content": [], "api": "faux", "provider": "p", "model": "m",
                "usage": {
                    "input": input, "output": 1, "cacheRead": read, "cacheWrite": write,
                    "totalTokens": input + read + write + 1,
                    "cost": {
                        "input": input as f64 * 3e-6, "output": 0.0, "cacheRead": read as f64 * 3e-7,
                        "cacheWrite": write as f64 * 3.75e-6, "total": 0.0,
                    },
                },
                "stopReason": "stop", "timestamp": timestamp,
            },
        }))
        .expect("an assistant entry")
    }

    #[test]
    fn counts_rebilled_prompt_tokens() {
        let registry = ModelRegistry::builtin();
        let compaction: FileEntry = serde_json::from_value(json!({
            "type": "compaction", "id": "c", "parentId": null, "timestamp": "2026-10-13T10:00:00.000Z",
            "summary": "s", "firstKeptEntryId": "c", "tokensBefore": 1,
        }))
        .expect("a compaction entry");
        let entries = [
            assistant("a", 1_000, 100, 0, 20_000),
            // A hit within the noise floor.
            assistant("b", 2_000, 1_000, 20_100, 0),
            // A total miss after an idle gap: 21,100 tokens billed again.
            assistant("c", 2_000 + CACHE_TTL_MS, 21_500, 0, 0),
            compaction,
            assistant("d", 400_000, 30_000, 0, 0),
        ];
        let misses = collect_cache_misses(&entries, &registry);
        assert_eq!(misses.len(), 1);
        let (id, miss) = &misses[0];
        assert_eq!(id, "c");
        assert_eq!(miss.missed_tokens, 21_100);
        assert!((miss.missed_cost - 21_100.0 * 3e-6).abs() < 1e-9);
        assert_eq!(miss.idle_ms, CACHE_TTL_MS);
        assert!(!miss.model_changed);
        assert_eq!(compute_cache_waste(&entries, &registry).miss_count, 1);

        // A message already recorded is compared with the request before it.
        assert_eq!(
            detect_cache_miss(&entries, message(&entries[2]), &registry),
            Some(*miss)
        );
    }

    fn message(entry: &FileEntry) -> &AssistantMessage {
        match entry {
            FileEntry::Message(entry) => match &entry.message {
                Message::Assistant(message) => message,
                _ => panic!("an assistant message"),
            },
            _ => panic!("a message entry"),
        }
    }

    #[test]
    fn uncached_providers_never_miss() {
        let registry = ModelRegistry::builtin();
        let entries = [
            assistant("a", 0, 100_000, 0, 0),
            assistant("b", 1_000, 110_000, 0, 0),
        ];
        assert_eq!(
            compute_cache_waste(&entries, &registry),
            CacheWaste::default()
        );
    }

    #[test]
    fn model_switches_miss() {
        let registry = ModelRegistry::builtin();
        let mut switched = assistant("b", 1_000, 0, 0, 100_000);
        if let FileEntry::Message(entry) = &mut switched
            && let Message::Assistant(message) = &mut entry.message
        {
            message.model = "other".into();
        }
        let entries = [assistant("a", 0, 0, 0, 100_000), switched];
        let misses = collect_cache_misses(&entries, &registry);
        assert_eq!(misses.len(), 1);
        assert_eq!(misses[0].1.missed_tokens, 100_000);
        assert!(misses[0].1.model_changed);
    }

    #[test]
    fn only_warming_refreshes_count() {
        let registry = ModelRegistry::builtin();
        let usage = |kind: &str| -> FileEntry {
            serde_json::from_value(json!({
                "type": "usage", "id": kind, "parentId": null,
                "timestamp": "1970-01-01T00:08:20.000Z",
                "kind": kind, "provider": "p", "model": "m",
                "usage": {"input": 0, "output": 0, "cacheRead": 100_000, "cacheWrite": 0,
                    "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}},
            }))
            .expect("a usage entry")
        };
        let first = assistant("a", 0, 0, 0, 100_000);
        let missed = assistant("b", 600_000, 0, 0, 110_000);
        let after = |kind| {
            detect_cache_miss(&[first.clone(), usage(kind)], message(&missed), &registry)
                .map(|miss| miss.idle_ms)
        };
        assert_eq!(after("cache_warm"), Some(100_000));
        assert_eq!(after("custom_operation"), Some(600_000));
    }
}
