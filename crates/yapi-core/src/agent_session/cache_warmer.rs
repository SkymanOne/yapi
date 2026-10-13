//! Prompt cache warming: keeps the prompt cache entry of the session's latest
//! request alive by sending the request again with a one-token output cap
//! before the entry expires, while that is expected to save money.
//!
//! Port of `packages/coding-agent/src/core/cache-warmer.ts` in pi `v1.0.0`.

use std::time::Duration;

use serde_json::Value;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use yapi_ai::stream::{CacheRetention, Request};
use yapi_types::event::AgentEvent;
use yapi_types::message::{Message, StopReason, Usage};
use yapi_types::model::{AnthropicMessagesCompat, Model};
use yapi_types::session::FileEntry;
use yapi_types::settings::CacheWarming;
use yapi_types::sync::lock;

use super::AgentSession;

/// Streaming warming never continues past this long after the real request
/// that started it.
const MAX_WARMING_AGE: Duration = Duration::from_secs(60 * 60);
/// Idle warming has a shorter horizon because continuation estimates become
/// less reliable with age.
const MAX_IDLE_WARMING_AGE: Duration = Duration::from_secs(30 * 60);
/// A refresh is sent only when it is expected to save at least this many
/// dollars.
const MINIMUM_EXPECTED_SAVINGS: f64 = 0.05;
/// Chance that a real request arrives before the cache entry expires while
/// the agent sits idle, as pi measured it.
const IDLE_CONTINUATION_PROBABILITY: f64 = 0.15;

/// When to refresh an entry that lives `ttl_ms`: at 90% of its lifetime,
/// keeping at least ten seconds of margin.
fn delay_ms(ttl_ms: f64) -> Option<u64> {
    (ttl_ms > 10_000.0).then(|| (ttl_ms * 0.9).min(ttl_ms - 10_000.0).floor().max(1.0) as u64)
}

/// Lifetime of the prompt cache entry `request` writes, from the model's
/// `promptCache` tier for the retention the request used.
fn ttl_ms(request: &Request) -> Option<f64> {
    let cache = request.model.prompt_cache.as_ref()?;
    let seconds = match request.options.resolved_cache_retention() {
        CacheRetention::None => None,
        CacheRetention::Short => cache.short,
        CacheRetention::Long => cache.long,
    }?;
    Some(seconds * 1000.0)
}

/// Whether replaying `request` with a one-token output cap leaves its cache
/// entry untouched. Anthropic's budget-based thinking derives its budget from
/// the output cap, and Anthropic keys the message cache on the budget.
fn replayable(request: &Request) -> bool {
    if request.options.reasoning.is_none() || request.model.api != "anthropic-messages" {
        return true;
    }
    request
        .model
        .compat::<AnthropicMessagesCompat>()
        .force_adaptive_thinking
        == Some(true)
}

/// Prompt size of the latest assistant message on the branch, as the
/// provider reported it.
fn last_prompt_tokens(branch: &[&FileEntry]) -> u64 {
    branch
        .iter()
        .rev()
        .find_map(|entry| match entry {
            FileEntry::Message(entry) => match &entry.message {
                Message::Assistant(assistant) => Some(&assistant.usage),
                _ => None,
            },
            _ => None,
        })
        .map_or(0, |usage| {
            usage.input + usage.cache_read + usage.cache_write
        })
}

fn price(model: &Model, mut usage: Usage) -> f64 {
    yapi_ai::cost::calculate_cost(model, &mut usage);
    usage.cost.total
}

/// Whether warming happens while the run that sent the request is active, or
/// between runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheWarmingPhase {
    /// The agent run that sent the request is still active.
    Streaming,
    /// The agent has settled.
    Idle,
}

/// Inputs and outcome of one warm-or-stop decision, as `/session` shows
/// them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CacheWarmingDecision {
    /// When the decision is made.
    pub phase: CacheWarmingPhase,
    /// Price of the refresh: a cache read of the prompt and one output token.
    pub warm_cost: f64,
    /// Extra price of the next real request if the cache entry is lost.
    pub miss_cost: f64,
    /// Estimated chance that a real request arrives before the entry expires.
    pub continuation_probability: f64,
    /// `continuation_probability * miss_cost - warm_cost`.
    pub expected_savings: f64,
    /// False when the prompt size or the model's prices are unknown.
    pub economics_available: bool,
    /// pi's decision: warm when the expected savings are at least $0.05.
    pub warm: bool,
}

impl CacheWarmingDecision {
    fn action(&self) -> &'static str {
        if self.warm { "warm" } else { "stop" }
    }
}

/// What cache warming is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheWarmingState {
    /// Nothing is scheduled.
    Inactive,
    /// A refresh is due at `next_warm_at`.
    Scheduled,
    /// A refresh is being decided or is in flight.
    Refreshing,
}

/// Cache warming's state and the policy inputs that produced it.
#[derive(Clone, Debug, PartialEq)]
pub struct CacheWarmingStatus {
    /// What warming is doing.
    pub state: CacheWarmingState,
    /// Why nothing is scheduled.
    pub reason: Option<&'static str>,
    /// When the next refresh is decided.
    pub next_warm_at: Option<Instant>,
    /// The pending decision, or the one that stopped warming.
    pub decision: Option<CacheWarmingDecision>,
    /// An extension changed the decision's action.
    pub extension_override: bool,
}

impl CacheWarmingStatus {
    fn inactive(reason: &'static str) -> CacheWarmingStatus {
        CacheWarmingStatus {
            state: CacheWarmingState::Inactive,
            reason: Some(reason),
            next_warm_at: None,
            decision: None,
            extension_override: false,
        }
    }
}

/// The request whose cache entry is kept warm, and the schedule.
struct Run {
    /// Tells this run from later ones.
    id: u64,
    /// The request as sent, without its credentials, which each refresh
    /// resolves again.
    request: Request,
    /// The session's context messages when the request was sent.
    messages: Vec<Message>,
    ttl_ms: f64,
    delay: Duration,
    /// Latest safe time to send this refresh, leaving half the original
    /// expiry margin.
    refresh_deadline: Instant,
    started: Instant,
    /// Cancels the timer and an in-flight refresh.
    cancel: CancellationToken,
    phase: CacheWarmingPhase,
    next_warm_at: Instant,
    /// Set while a refresh that an extension forced is in flight.
    extension_override: bool,
    /// The timer has fired: a refresh is being decided or sent.
    refreshing: bool,
}

/// A session's cache warming: at most one run, and why the last one ended.
pub(super) struct CacheWarmer {
    run: Option<Run>,
    inactive: CacheWarmingStatus,
    runs: u64,
}

impl Default for CacheWarmer {
    fn default() -> CacheWarmer {
        CacheWarmer {
            run: None,
            inactive: CacheWarmingStatus::inactive("waiting for first request"),
            runs: 0,
        }
    }
}

impl CacheWarmer {
    fn clear_run(&mut self) {
        if let Some(run) = self.run.take() {
            run.cancel.cancel();
        }
    }

    fn stop(&mut self, reason: &'static str, stopped: Option<(CacheWarmingDecision, bool)>) {
        self.clear_run();
        self.inactive = CacheWarmingStatus {
            decision: stopped.map(|(decision, _)| decision),
            extension_override: stopped.is_some_and(|(_, overridden)| overridden),
            ..CacheWarmingStatus::inactive(reason)
        };
    }

    /// The current run when it is run `id`.
    fn run(&mut self, id: u64) -> Option<&mut Run> {
        self.run.as_mut().filter(|run| run.id == id)
    }
}

fn mode_stop_reason(mode: CacheWarming, run: &Run) -> Option<&'static str> {
    match (mode, run.phase) {
        (CacheWarming::Off, _) => Some("cache warming disabled"),
        (CacheWarming::Streaming, CacheWarmingPhase::Idle) => Some("agent run settled"),
        _ => None,
    }
}

impl AgentSession {
    /// The cache warming mode, from the global settings only.
    pub fn cache_warming_mode(&self) -> CacheWarming {
        lock(&self.inner.settings).cache_warming_mode()
    }

    /// pi's `setCacheWarmingMode`: saves the mode in the global settings and
    /// applies it to the current run at once.
    pub fn set_cache_warming_mode(&self, mode: CacheWarming) -> Result<(), String> {
        self.set_global_setting("cacheWarming", serde_json::to_value(mode).ok())?;
        let mut warmer = lock(&self.inner.cache_warmer);
        if let Some(reason) = warmer
            .run
            .as_ref()
            .and_then(|run| mode_stop_reason(mode, run))
        {
            warmer.stop(reason, None);
        }
        Ok(())
    }

    /// Cache warming's state and the policy inputs that produced it.
    pub fn cache_warming_status(&self) -> CacheWarmingStatus {
        if self.cache_warming_mode() == CacheWarming::Off {
            return CacheWarmingStatus::inactive("cache warming disabled");
        }
        let warmer = lock(&self.inner.cache_warmer);
        let Some(run) = &warmer.run else {
            return warmer.inactive.clone();
        };
        if !self.cache_context_is_current(run) {
            return CacheWarmingStatus::inactive("conversation context changed");
        }
        let decision = self.evaluate_cache_warming(run);
        if !decision.economics_available && !run.refreshing {
            return CacheWarmingStatus::inactive("cache economics unavailable");
        }
        CacheWarmingStatus {
            state: if run.refreshing {
                CacheWarmingState::Refreshing
            } else {
                CacheWarmingState::Scheduled
            },
            reason: None,
            next_warm_at: Some(run.next_warm_at),
            decision: Some(decision),
            extension_override: run.extension_override,
        }
    }

    /// Keeps the cache entry `request` writes warm, in place of any earlier
    /// request's. Only session requests reach this: compaction and summaries
    /// use their own cache entries.
    pub(super) fn start_cache_warming(&self, request: &Request) {
        let mode = self.cache_warming_mode();
        let mut warmer = lock(&self.inner.cache_warmer);
        warmer.clear_run();
        if mode == CacheWarming::Off {
            return warmer.stop("cache warming disabled", None);
        }
        if !replayable(request) {
            return warmer.stop("request cannot be replayed safely", None);
        }
        let Some(ttl_ms) = ttl_ms(request) else {
            let reason = match request.options.cache_retention {
                Some(CacheRetention::None) => "request disabled prompt caching",
                _ => "cache lifetime unavailable",
            };
            return warmer.stop(reason, None);
        };
        let Some(delay) = delay_ms(ttl_ms) else {
            return warmer.stop("cache lifetime unavailable", None);
        };
        let mut request = request.clone();
        request.options.api_key = None;
        request.options.headers.clear();
        request.options.env = None;
        warmer.runs += 1;
        let now = Instant::now();
        warmer.run = Some(Run {
            id: warmer.runs,
            request,
            messages: self.messages(),
            ttl_ms,
            delay: Duration::from_millis(delay),
            refresh_deadline: now,
            started: now,
            cancel: CancellationToken::new(),
            phase: CacheWarmingPhase::Streaming,
            next_warm_at: now,
            extension_override: false,
            refreshing: false,
        });
        self.schedule_cache_refresh(&mut warmer);
    }

    /// pi's `onAgentSettled`: in streaming mode warming stops; otherwise it
    /// continues in the idle phase within its shorter limit.
    pub(super) fn cache_warming_settled(&self) {
        let mode = self.cache_warming_mode();
        let mut warmer = lock(&self.inner.cache_warmer);
        let Some(run) = warmer.run.as_mut() else {
            return;
        };
        if mode == CacheWarming::Streaming {
            return warmer.stop("agent run settled", None);
        }
        run.phase = CacheWarmingPhase::Idle;
        let deadline = run.started + MAX_IDLE_WARMING_AGE;
        if run.next_warm_at > deadline || Instant::now() >= deadline {
            warmer.stop("30-minute idle safety limit reached", None);
        }
    }

    /// Stops warming for good, as when the session ends.
    pub(super) fn cancel_cache_warming(&self) {
        lock(&self.inner.cache_warmer).stop("inactive", None);
    }

    /// pi's `cacheContextIsCurrent`: the model is still the request's, and
    /// the transcript still extends the request's messages.
    fn cache_context_is_current(&self, run: &Run) -> bool {
        let state = lock(&self.inner.state);
        let model = &run.request.model;
        state
            .model
            .as_ref()
            .is_some_and(|current| current.is(&model.provider, &model.id))
            && state
                .session
                .build_context()
                .messages
                .starts_with(&run.messages)
    }

    fn evaluate_cache_warming(&self, run: &Run) -> CacheWarmingDecision {
        let model = &run.request.model;
        let prompt = self.with_session(|session| last_prompt_tokens(&session.branch_path(None)));
        let read = Usage {
            cache_read: prompt,
            ..Usage::default()
        };
        let hit_price = price(model, read.clone());
        let miss_price = price(
            model,
            if model.cost.cache_write > 0.0 {
                Usage {
                    cache_write: prompt,
                    ..Usage::default()
                }
            } else {
                Usage {
                    input: prompt,
                    ..Usage::default()
                }
            },
        );
        let warm_cost = price(model, Usage { output: 1, ..read });
        let miss_cost = (miss_price - hit_price).max(0.0);
        let continuation_probability = match run.phase {
            CacheWarmingPhase::Streaming => 1.0,
            CacheWarmingPhase::Idle => IDLE_CONTINUATION_PROBABILITY,
        };
        let expected_savings = continuation_probability * miss_cost - warm_cost;
        CacheWarmingDecision {
            phase: run.phase,
            warm_cost,
            miss_cost,
            continuation_probability,
            expected_savings,
            economics_available: prompt > 0 && (hit_price > 0.0 || miss_price > 0.0),
            warm: expected_savings >= MINIMUM_EXPECTED_SAVINGS,
        }
    }

    /// Arms the current run's timer, unless its safety limit has passed.
    fn schedule_cache_refresh(&self, warmer: &mut CacheWarmer) {
        let Some(run) = warmer.run.as_mut() else {
            return;
        };
        let now = Instant::now();
        run.extension_override = false;
        run.refreshing = false;
        run.next_warm_at = now + run.delay;
        // A timer can fire late after sleep or a blocked runtime. Half of the
        // planned margin before expiry is kept for that and the request; a
        // later refresh would likely write the cache at full price.
        let margin = ((run.ttl_ms - run.delay.as_millis() as f64) / 2.0).floor();
        run.refresh_deadline = run.next_warm_at + Duration::from_millis(margin as u64);
        let (limit, reason) = match run.phase {
            CacheWarmingPhase::Streaming => (MAX_WARMING_AGE, "one-hour safety limit reached"),
            CacheWarmingPhase::Idle => {
                (MAX_IDLE_WARMING_AGE, "30-minute idle safety limit reached")
            }
        };
        let deadline = run.started + limit;
        if run.next_warm_at > deadline || now >= deadline {
            return warmer.stop(reason, None);
        }
        let (id, at, cancel, session) = (
            run.id,
            run.next_warm_at,
            run.cancel.clone(),
            self.downgrade(),
        );
        tokio::spawn(async move {
            tokio::select! {
                () = cancel.cancelled() => {}
                () = tokio::time::sleep_until(at) => {
                    if let Some(session) = session.upgrade() {
                        session.refresh_cache(id).await;
                    }
                }
            }
        });
    }

    /// pi's `validateRun`: whether run `id` is still current and wanted;
    /// otherwise warming stops for the reason it is not.
    fn cache_run_valid(&self, warmer: &mut CacheWarmer, id: u64) -> bool {
        let mode = self.cache_warming_mode();
        let Some(run) = warmer.run(id) else {
            return false;
        };
        let reason = mode_stop_reason(mode, run).or_else(|| {
            (!self.cache_context_is_current(run)).then_some("conversation context changed")
        });
        match reason {
            Some(reason) => {
                warmer.stop(reason, None);
                false
            }
            None => true,
        }
    }

    /// pi's `refreshDeadlineMissed`.
    fn cache_refresh_late(warmer: &mut CacheWarmer, id: u64) -> bool {
        let late = warmer
            .run(id)
            .is_some_and(|run| Instant::now() > run.refresh_deadline);
        if late {
            warmer.stop("cache refresh deadline missed", None);
        }
        late
    }

    /// Decides and sends one refresh of run `id`, then arms the next.
    async fn refresh_cache(&self, id: u64) {
        let decision = {
            let mut warmer = lock(&self.inner.cache_warmer);
            if let Some(run) = warmer.run(id) {
                run.refreshing = true;
            }
            if !self.cache_run_valid(&mut warmer, id) || Self::cache_refresh_late(&mut warmer, id) {
                return;
            }
            let Some(run) = warmer.run(id) else {
                return;
            };
            self.evaluate_cache_warming(run)
        };
        let action = self.cache_warming_decision(&decision).await;
        let (request, cancel, overridden) = {
            let mut warmer = lock(&self.inner.cache_warmer);
            if !self.cache_run_valid(&mut warmer, id) || Self::cache_refresh_late(&mut warmer, id) {
                return;
            }
            let overridden = action != decision.action();
            if action == "stop" {
                let reason = if overridden {
                    "stopped by extension"
                } else if decision.economics_available {
                    "expected savings below threshold"
                } else {
                    "cache economics unavailable"
                };
                return warmer.stop(reason, Some((decision, overridden)));
            }
            let Some(run) = warmer.run(id) else {
                return;
            };
            run.extension_override = overridden;
            (run.request.clone(), run.cancel.clone(), overridden)
        };
        let message = self.send_cache_refresh(request, cancel).await;
        {
            let mut warmer = lock(&self.inner.cache_warmer);
            if !self.cache_run_valid(&mut warmer, id) {
                return;
            }
        }
        if let Some(message) = message.filter(|message| {
            !matches!(message.stop_reason, StopReason::Error | StopReason::Aborted)
        }) {
            let model = message.response_model.as_deref().unwrap_or(&message.model);
            let note = overridden.then(|| "extension override".to_owned());
            let entry = self.with_session(|session| {
                let id = session
                    .append_usage(
                        "cache_warm",
                        &message.provider,
                        model,
                        message.usage.clone(),
                        note,
                    )
                    .ok()?;
                session.entry(&id).cloned()
            });
            if let Some(entry) = entry {
                self.emit(&AgentEvent::EntryAppended { entry });
            }
        }
        let mut warmer = lock(&self.inner.cache_warmer);
        if warmer.run(id).is_some() {
            self.schedule_cache_refresh(&mut warmer);
        }
    }

    /// pi's `cache_warming_decision` event: each handler sees pi's own
    /// decision, and the last action a handler returns wins.
    async fn cache_warming_decision(&self, decision: &CacheWarmingDecision) -> Value {
        let mut action = Value::from(decision.action());
        let handlers = self.handlers_of("cache_warming_decision");
        if handlers.is_empty() {
            return action;
        }
        let event = serde_json::json!({
            "type": "cache_warming_decision",
            "warmCost": decision.warm_cost,
            "missCost": decision.miss_cost,
            "continuationProbability": decision.continuation_probability,
            "action": decision.action(),
        });
        let ctx = self.extension_context(CancellationToken::new());
        for extension in handlers {
            if let Some(next) = extension
                .handle(&ctx, &event)
                .await
                .and_then(|mut result| result.get_mut("action").map(Value::take))
            {
                action = next;
            }
        }
        action
    }

    /// Sends `request` again with a one-token output cap and fresh
    /// credentials; its response, unless it could not be sent.
    async fn send_cache_refresh(
        &self,
        mut request: Request,
        cancel: CancellationToken,
    ) -> Option<yapi_types::message::AssistantMessage> {
        request.options.max_tokens = Some(1);
        request.options.max_retries = 0;
        request.options.cancel = cancel;
        let auth = self.registry().auth(&request.model).await;
        auth.apply(&mut request).ok()?;
        yapi_ai::stream::RequestHooks::prepare(&mut request).await;
        self.inner.apis.stream(request).result().await
    }
}

/// pi's dollar amounts: three decimals, the sign before the dollar sign.
fn dollars(value: f64) -> String {
    if value < 0.0 {
        format!("-${:.3}", value.abs())
    } else {
        format!("${value:.3}")
    }
}

fn economics(decision: &CacheWarmingDecision) -> String {
    if !decision.economics_available {
        return "cache economics unavailable".into();
    }
    let probability = (decision.continuation_probability * 100.0).round();
    let while_running = match decision.phase {
        CacheWarmingPhase::Streaming => " while agent is running",
        CacheWarmingPhase::Idle => "",
    };
    let comparison = if decision.warm { ">=" } else { "<" };
    format!(
        "{probability}% continuation probability{while_running}, expected savings {} {comparison} ${MINIMUM_EXPECTED_SAVINGS:.3}",
        dollars(decision.expected_savings)
    )
}

fn decision_time(next_warm_at: Option<Instant>, now: Instant) -> String {
    let Some(remaining) = next_warm_at
        .filter(|at| *at > now)
        .map(|at| at.duration_since(now))
    else {
        return "Decision now".into();
    };
    let seconds = remaining.as_millis().div_ceil(1000);
    let (hours, minutes, seconds) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }
    format!("Decision in {}", parts.join(" "))
}

/// pi's one-line cache warming status for `/session`.
pub fn format_cache_warming_status(status: &CacheWarmingStatus, now: Instant) -> String {
    // A decision is attached once pi or an extension acted on it.
    let decision = match &status.decision {
        Some(decision)
            if status.state != CacheWarmingState::Inactive
                || decision.economics_available
                || status.extension_override =>
        {
            decision
        }
        _ => return format!("Inactive ({})", status.reason.unwrap_or("unknown reason")),
    };
    let details = if status.extension_override {
        format!("extension override, {}", economics(decision))
    } else {
        format!("{} -> {}", economics(decision), decision.action())
    };
    match status.state {
        CacheWarmingState::Inactive => format!("Stopped ({details})"),
        CacheWarmingState::Refreshing => format!("Warming cache ({details})"),
        CacheWarmingState::Scheduled => {
            format!("{} ({details})", decision_time(status.next_warm_at, now))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshes_at_ninety_percent_with_ten_seconds_of_margin() {
        assert_eq!(delay_ms(10_000.0), None);
        assert_eq!(delay_ms(11_000.0), Some(1_000));
        assert_eq!(delay_ms(300_000.0), Some(270_000));
        assert_eq!(delay_ms(3_600_000.0), Some(3_240_000));
    }

    #[test]
    fn formats_the_status_as_pi() {
        let decision = CacheWarmingDecision {
            phase: CacheWarmingPhase::Streaming,
            warm_cost: 0.006,
            miss_cost: 0.069,
            continuation_probability: 1.0,
            expected_savings: 0.063,
            economics_available: true,
            warm: true,
        };
        let now = Instant::now();
        let scheduled = CacheWarmingStatus {
            state: CacheWarmingState::Scheduled,
            reason: None,
            next_warm_at: Some(now + Duration::from_millis(3_661_500)),
            decision: Some(decision),
            extension_override: false,
        };
        assert_eq!(
            format_cache_warming_status(&scheduled, now),
            "Decision in 1h 1m 2s (100% continuation probability while agent is running, expected savings $0.063 >= $0.050 -> warm)"
        );
        let stopped = CacheWarmingStatus {
            state: CacheWarmingState::Inactive,
            reason: Some("expected savings below threshold"),
            next_warm_at: None,
            decision: Some(CacheWarmingDecision {
                phase: CacheWarmingPhase::Idle,
                continuation_probability: IDLE_CONTINUATION_PROBABILITY,
                expected_savings: -0.002,
                warm: false,
                ..decision
            }),
            extension_override: false,
        };
        assert_eq!(
            format_cache_warming_status(&stopped, now),
            "Stopped (15% continuation probability, expected savings -$0.002 < $0.050 -> stop)"
        );
        let forced = CacheWarmingStatus {
            state: CacheWarmingState::Refreshing,
            extension_override: true,
            ..stopped
        };
        assert_eq!(
            format_cache_warming_status(&forced, now),
            "Warming cache (extension override, 15% continuation probability, expected savings -$0.002 < $0.050)"
        );
        assert_eq!(
            format_cache_warming_status(&CacheWarmingStatus::inactive("agent run settled"), now),
            "Inactive (agent run settled)"
        );
    }
}
