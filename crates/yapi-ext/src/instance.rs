//! One `yapi-js` instance and the thread that owns it.
//!
//! The actor thread owns the wasmtime store and runs one guest export at a
//! time. Calls and operation results arrive as commands. Operations the guest
//! starts run on the tokio runtime and come back as commands, so host code
//! never calls into the guest while the guest is running.

use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use futures_util::future::BoxFuture;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use wasmtime::component::{Component, ResourceTable};
use wasmtime::{CallHook, Store, StoreLimits, StoreLimitsBuilder, UpdateDeadline};
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxView, WasiView};
use yapi_agent::tool::UpdateSink;
use yapi_core::extensions::{EventPolicy, Grants};

use crate::Error;
use crate::engine::{Engine, Extension, yapi as wit};
use crate::loader::Loader;
use crate::requests::{AiStreams, Host};

/// How long one guest export may compute, not counting time spent in host
/// calls, before the instance is stopped.
const CALL_LIMIT: Duration = Duration::from_secs(if cfg!(test) { 2 } else { 60 });

/// [`CALL_LIMIT`] while the instance starts and while a call that loads or
/// reloads its extensions is unanswered, so one that spins as it loads
/// stops startup soon.
const LOAD_LIMIT: Duration = Duration::from_secs(if cfg!(test) { 1 } else { 10 });

/// The canonical paths of `roots` that exist: a symlink inside a root that
/// leads out then lies outside it, and a root moved elsewhere changes.
pub fn canonical_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect()
}

/// The session side of an instance: requests and operations the runtime does
/// not answer itself, and console output.
pub trait Bridge: Send + Sync {
    /// Answers request `kind` at once. Runs on the instance's thread while the
    /// guest waits, so it must not block for long.
    fn request(&self, kind: &str, _payload: &Value) -> Result<Value, String> {
        Err(format!("{kind} is not available here"))
    }

    /// Refuses MCP server `name`, which an extension registers, with pi's
    /// error when an extension of another runtime registered the name or
    /// its tools' namespace. Runs on the instance's thread. Hosts without
    /// other runtimes accept every name.
    fn check_mcp_server(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }

    /// Starts operation `kind`. The guest gets its result when it completes.
    fn start(&self, kind: &str, _payload: Value) -> BoxFuture<'static, Result<Value, String>> {
        let message = format!("{kind} is not available here");
        Box::pin(async move { Err(message) })
    }

    /// Ends a guest step: called on the instance's thread after each guest
    /// call, once the operations it began have started and before its results
    /// are delivered.
    fn step_ended(&self) {}

    /// The instance stopped because of `reason` and restarts: its extensions
    /// load again, without the components they showed. Runs on the
    /// instance's thread before the extensions load.
    fn restarted(&self, _reason: &str) {}

    /// The update sink and cancellation of running extension tool call
    /// `tool_call_id`, for the built-in tools it runs, or `None` when no such
    /// call runs.
    fn caller(&self, _tool_call_id: &str) -> Option<(UpdateSink, CancellationToken)> {
        None
    }

    /// The request hooks of running extension stream `id`, for the requests
    /// it makes through yapi's wire APIs.
    fn stream_hooks(&self, _id: u64) -> yapi_ai::stream::RequestHooks {
        yapi_ai::stream::RequestHooks::default()
    }

    /// Streams `{model, context, options}` through the session's provider
    /// of the model, with the registry's credentials under the options'
    /// own, as pi's `ModelRegistry.stream` does. `cancel` aborts it and
    /// `hooks` observe it.
    fn stream_model(
        &self,
        _payload: Value,
        _cancel: CancellationToken,
        _hooks: yapi_ai::stream::RequestHooks,
    ) -> BoxFuture<'static, Result<yapi_ai::stream::EventStream, String>> {
        Box::pin(async { Err("models.stream is not available here".into()) })
    }

    /// Console output from extensions, at `level` `debug`, `info`, `warn` or
    /// `error`.
    fn log(&self, _level: &str, message: &str) {
        let _ = writeln!(std::io::stderr(), "{message}");
    }
}

/// A bridge to nothing: only the runtime's own requests work.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoBridge;

impl Bridge for NoBridge {}

/// How to start an instance.
#[derive(Clone, Debug)]
pub struct Options {
    /// The working directory extensions see.
    pub cwd: PathBuf,
    /// The project directory of the sessions the extensions load for.
    pub project_dir: PathBuf,
    /// The agent directory extensions see.
    pub agent_dir: PathBuf,
    /// The directory `os.homedir()` names.
    pub home_dir: PathBuf,
    /// The directory `os.tmpdir()` names.
    pub temp_dir: PathBuf,
    /// What the instance may reach.
    pub grants: Grants,
    /// The `pi.events` channels its extensions may send to, and hear from,
    /// extensions whose grants differ.
    pub events: EventPolicy,
    /// The directories file access reaches, when granted.
    pub filesystem_roots: Vec<PathBuf>,
    /// The directories the extensions load modules from without file
    /// access: each package's install root, which holds the package and its
    /// own `node_modules`.
    pub module_roots: Vec<PathBuf>,
    /// The most linear memory the instance may use, in bytes.
    pub memory_limit: usize,
    /// A directory of its own where transpiled modules are cached: other
    /// files in it are removed. `None` disables the cache.
    pub cache_dir: Option<PathBuf>,
    /// The environment variables extensions see when the environment is
    /// granted: these alone, or with `None`, the process's own.
    pub environment: Option<std::collections::BTreeMap<String, String>>,
    /// The terminal's columns and rows while extensions load, in a mode that
    /// draws on it, or `None` without a terminal.
    pub terminal_size: Option<(usize, usize)>,
}

impl Options {
    /// Full grants and a 1 GiB memory limit in `cwd`, which is the project
    /// directory too, with the configured agent directory.
    pub fn new(cwd: PathBuf) -> Options {
        Options {
            project_dir: cwd.clone(),
            cwd,
            agent_dir: yapi_core::config::agent_dir(),
            home_dir: yapi_core::tools::path::home_dir(),
            temp_dir: std::env::temp_dir(),
            grants: Grants::default(),
            events: EventPolicy::shared(),
            filesystem_roots: vec![PathBuf::from("/")],
            module_roots: Vec::new(),
            memory_limit: 1 << 30,
            cache_dir: None,
            environment: None,
            terminal_size: None,
        }
    }
}

/// The store's data: WASI state, limits and the host side of the WIT world.
pub(crate) struct State {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    host: Arc<Host>,
    started: Vec<(u64, String, String)>,
    call_started: Instant,
    /// Whether the export runs under [`LOAD_LIMIT`].
    loading: bool,
    /// When the guest's outermost open host call began, how many host calls
    /// are open, and the time the guest spent in closed ones since the
    /// export began.
    host_called: Instant,
    host_depth: u32,
    host_time: Duration,
    /// Set when the instance is dropped: the running export traps at the next
    /// epoch tick.
    interrupt: Arc<AtomicBool>,
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl wit::extension::types::Host for State {}

impl wit::extension::host::Host for State {
    fn start(&mut self, op: u64, kind: String, payload: String) {
        self.started.push((op, kind, payload));
    }

    fn request(&mut self, kind: String, payload: String) -> Result<String, String> {
        let payload: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
        self.host
            .request(&kind, &payload)
            .map(|value| yapi_types::json::stringify(&value))
    }
}

enum Command {
    Call {
        kind: String,
        payload: String,
        reply: oneshot::Sender<Result<Value, Error>>,
    },
    Resolve {
        generation: u64,
        op: u64,
        value: Result<Value, String>,
    },
    Render {
        handle: u32,
        width: u32,
        reply: oneshot::Sender<Vec<String>>,
    },
    Input {
        handle: u32,
        data: String,
    },
    Settle(oneshot::Sender<u64>),
    Stop,
}

/// A running `yapi-js` instance. Dropping it stops the instance, interrupting
/// the guest if it is computing.
pub struct Instance {
    commands: mpsc::Sender<Command>,
    interrupt: Arc<AtomicBool>,
    /// The actor thread, taken when the instance is dropped.
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance").finish_non_exhaustive()
    }
}

impl Instance {
    /// Starts an instance of the JS runtime on its own thread. Operations run
    /// on the current tokio runtime.
    pub async fn start(
        engine: &Engine,
        options: Options,
        bridge: Arc<dyn Bridge>,
    ) -> Result<Instance, Error> {
        Instance::start_component(engine, engine.component.clone(), options, bridge).await
    }

    /// Starts an instance of a native extension `component`, compiled by
    /// [`Engine::native`].
    pub(crate) async fn start_component(
        engine: &Engine,
        component: Component,
        options: Options,
        bridge: Arc<dyn Bridge>,
    ) -> Result<Instance, Error> {
        let runtime = tokio::runtime::Handle::current();
        let host = Arc::new(Host {
            loader: Loader::new(options.cwd.clone(), options.cache_dir.clone()),
            bridge,
            module_roots: canonical_roots(&options.module_roots),
            options,
            ai_streams: AiStreams::default(),
            streams: crate::ops::Streams::new(runtime.clone()),
        });
        let (commands, receiver) = mpsc::channel();
        let (ready, started) = oneshot::channel();
        let engine = engine.clone();
        let sender = commands.clone();
        let interrupt = Arc::new(AtomicBool::new(false));
        let flag = interrupt.clone();
        let thread = std::thread::Builder::new()
            .name("yapi-ext-instance".into())
            // Guest code runs on this thread's stack.
            .stack_size(crate::engine::WASM_STACK + (2 << 20))
            .spawn(
                move || match Actor::new(engine, component, host, runtime, sender, flag) {
                    Ok(actor) => {
                        let _ = ready.send(Ok(()));
                        actor.run(&receiver);
                    }
                    Err(err) => {
                        let _ = ready.send(Err(err));
                    }
                },
            )
            .map_err(|err| Error::Instantiate(err.to_string()))?;
        started.await.map_err(|_| Error::Stopped)??;
        Ok(Instance {
            commands,
            interrupt,
            thread: Some(thread),
        })
    }

    /// Runs dispatch `kind` with `payload`, queued at once after the calls
    /// and input sent before it, and returns its result.
    pub fn call(
        &self,
        kind: &str,
        payload: &Value,
    ) -> impl Future<Output = Result<Value, Error>> + use<> {
        let result = self.send_call(kind, payload);
        async move {
            result
                .ok_or(Error::Stopped)?
                .await
                .map_err(|_| Error::Stopped)?
        }
    }

    /// Runs dispatch `kind` with `payload` without waiting for its result,
    /// after the calls and input sent before it.
    pub fn post(&self, kind: &str, payload: &Value) {
        self.send_call(kind, payload);
    }

    /// Queues dispatch `kind`. Returns the receiver of its result, or `None`
    /// once the instance stopped.
    fn send_call(
        &self,
        kind: &str,
        payload: &Value,
    ) -> Option<oneshot::Receiver<Result<Value, Error>>> {
        let (reply, result) = oneshot::channel();
        self.commands
            .send(Command::Call {
                kind: kind.to_owned(),
                payload: yapi_types::json::stringify(payload),
                reply,
            })
            .ok()?;
        Some(result)
    }

    /// The lines of component `handle` at `width` columns, or none when the
    /// component or the instance is gone.
    pub async fn render(&self, handle: u32, width: u32) -> Vec<String> {
        let (reply, result) = oneshot::channel();
        if self
            .commands
            .send(Command::Render {
                handle,
                width,
                reply,
            })
            .is_err()
        {
            return Vec::new();
        }
        result.await.unwrap_or_default()
    }

    /// Resolves once the instance has handled what was sent to it before,
    /// such as a call it is running: a call that finished has delivered its
    /// result by then. Returns a count that grows with every call the
    /// instance starts.
    pub async fn settle(&self) -> u64 {
        let (done, settled) = oneshot::channel();
        if self.commands.send(Command::Settle(done)).is_err() {
            return 0;
        }
        settled.await.unwrap_or_default()
    }

    /// Delivers raw terminal input to component `handle`.
    pub fn input(&self, handle: u32, data: &str) {
        let _ = self.commands.send(Command::Input {
            handle,
            data: data.to_owned(),
        });
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.interrupt.store(true, Ordering::Relaxed);
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let mut stopping = yapi_types::sync::lock(&STOPPING);
            stopping.retain(|thread| !thread.is_finished());
            stopping.push(thread);
        }
    }
}

/// How long [`join_stopped`] waits, in total, for the threads of dropped
/// instances.
const STOP_WAIT: Duration = Duration::from_secs(2);

/// Threads of dropped instances that may still be tearing down their stores.
static STOPPING: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

/// Waits for the threads of dropped instances to finish, up to 2s in total,
/// and leaves behind those still running then. Call it before the process
/// exits: exiting while a thread frees an instance's compiled code can abort
/// the process.
pub fn join_stopped() {
    let threads = std::mem::take(&mut *yapi_types::sync::lock(&STOPPING));
    let deadline = Instant::now() + STOP_WAIT;
    while threads.iter().any(|thread| !thread.is_finished()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    for thread in threads {
        if thread.is_finished() {
            let _ = thread.join();
        }
    }
}

struct Actor {
    engine: Engine,
    component: Component,
    host: Arc<Host>,
    runtime: tokio::runtime::Handle,
    commands: mpsc::Sender<Command>,
    store: Store<State>,
    bindings: Extension,
    generation: u64,
    next_id: u64,
    pending: HashMap<u64, oneshot::Sender<Result<Value, Error>>>,
    /// The calls that load or reload extensions and are not answered yet.
    loading: HashSet<u64>,
    /// Calls that shape the instance's state, replayed after a restart.
    replay: Vec<(String, String)>,
    /// Set while a restart replays `replay`.
    replaying: bool,
    interrupt: Arc<AtomicBool>,
}

impl Actor {
    fn new(
        engine: Engine,
        component: Component,
        host: Arc<Host>,
        runtime: tokio::runtime::Handle,
        commands: mpsc::Sender<Command>,
        interrupt: Arc<AtomicBool>,
    ) -> Result<Actor, Error> {
        let (store, bindings) = instantiate(&engine, &component, &host, &interrupt)?;
        Ok(Actor {
            engine,
            component,
            host,
            runtime,
            commands,
            store,
            bindings,
            generation: 0,
            next_id: 1,
            pending: HashMap::new(),
            loading: HashSet::new(),
            replay: Vec::new(),
            replaying: false,
            interrupt,
        })
    }

    fn run(mut self, commands: &mpsc::Receiver<Command>) {
        while let Ok(command) = commands.recv() {
            match command {
                Command::Call {
                    kind,
                    payload,
                    reply,
                } => {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.pending.insert(id, reply);
                    self.track_load(id, &kind);
                    // A call that stops the instance is not replayed.
                    if self.guest(|bindings, store| {
                        bindings
                            .yapi_extension_guest()
                            .call_dispatch(store, id, &kind, &payload)
                    }) {
                        record(&mut self.replay, &kind, &payload);
                    }
                }
                Command::Resolve {
                    generation,
                    op,
                    value,
                } => {
                    if generation != self.generation {
                        continue;
                    }
                    let value = value.map(|value| yapi_types::json::stringify(&value));
                    self.guest(|bindings, store| {
                        bindings.yapi_extension_guest().call_resolve(
                            store,
                            op,
                            value.as_ref().map_err(String::as_str),
                        )
                    });
                }
                Command::Render {
                    handle,
                    width,
                    reply,
                } => {
                    let _ = reply.send(self.render(handle, width));
                }
                Command::Input { handle, data } => {
                    self.guest(|bindings, store| {
                        bindings
                            .yapi_extension_guest()
                            .call_input(store, handle, &data)
                    });
                }
                Command::Settle(done) => {
                    let _ = done.send(self.next_id);
                }
                Command::Stop => break,
            }
            if self.interrupt.load(Ordering::Relaxed) {
                break;
            }
        }
    }

    /// Counts call `id` as loading extensions when `kind` loads or reloads
    /// them.
    fn track_load(&mut self, id: u64, kind: &str) {
        if matches!(kind, "load" | "reload") {
            self.loading.insert(id);
        }
    }

    /// Starts the clock and the epoch deadline of a guest call.
    fn start_call(&mut self) {
        let loading = !self.loading.is_empty();
        let state = self.store.data_mut();
        state.loading = loading;
        state.call_started = Instant::now();
        state.host_depth = 0;
        state.host_time = Duration::ZERO;
        self.store.set_epoch_deadline(1);
    }

    fn render(&mut self, handle: u32, width: u32) -> Vec<String> {
        self.start_call();
        match self
            .bindings
            .yapi_extension_guest()
            .call_render(&mut self.store, handle, width)
        {
            Ok(lines) => lines,
            Err(trap) => {
                self.restart(&trap);
                Vec::new()
            }
        }
    }

    /// Runs one guest export, then delivers its outcomes and starts the
    /// operations it began. Whether the export ran without stopping the
    /// instance.
    fn guest(
        &mut self,
        call: impl FnOnce(
            &Extension,
            &mut Store<State>,
        ) -> wasmtime::Result<Vec<wit::extension::types::Outcome>>,
    ) -> bool {
        self.start_call();
        let result = call(&self.bindings, &mut self.store);
        let started = std::mem::take(&mut self.store.data_mut().started);
        match result {
            Ok(outcomes) => {
                for (op, kind, payload) in started {
                    self.start_op(op, &kind, &payload);
                }
                self.host.bridge.step_ended();
                for outcome in outcomes {
                    let (id, result) = match outcome {
                        wit::extension::types::Outcome::Done((id, json)) => (
                            id,
                            serde_json::from_str(&json).map_err(|err| Error::Call(err.to_string())),
                        ),
                        wit::extension::types::Outcome::Failed((id, message)) => {
                            (id, Err(Error::Call(message)))
                        }
                    };
                    self.loading.remove(&id);
                    if let Some(reply) = self.pending.remove(&id) {
                        let _ = reply.send(result);
                    }
                }
                true
            }
            Err(trap) => {
                self.restart(&trap);
                false
            }
        }
    }

    fn start_op(&self, op: u64, kind: &str, payload: &str) {
        let payload: Value = serde_json::from_str(payload).unwrap_or(Value::Null);
        let future = self.host.start(kind, payload);
        let commands = self.commands.clone();
        let generation = self.generation;
        self.runtime.spawn(async move {
            let value = future.await;
            let _ = commands.send(Command::Resolve {
                generation,
                op,
                value,
            });
        });
    }

    /// Replaces an instance stopped by `trap` with a fresh one and replays
    /// the calls that loaded and configured its extensions. Calls in flight
    /// fail. An instance that the replay stops stays stopped until the next
    /// call, which restarts it again.
    fn restart(&mut self, trap: &wasmtime::Error) {
        self.loading.clear();
        if self.interrupt.load(Ordering::Relaxed) {
            for (_, reply) in self.pending.drain() {
                let _ = reply.send(Err(Error::Stopped));
            }
            return;
        }
        let reason = format!("{trap:#}");
        self.host
            .bridge
            .log("error", &format!("Extension runtime stopped: {reason}"));
        self.host.streams.clear();
        self.host.ai_streams.clear();
        for (_, reply) in self.pending.drain() {
            let _ = reply.send(Err(Error::Crashed(reason.clone())));
        }
        self.generation += 1;
        if self.replaying {
            self.host.bridge.log(
                "error",
                "Extension runtime cannot restart: replaying the extensions stopped it again",
            );
            return;
        }
        self.host.bridge.restarted(&trap.root_cause().to_string());
        match instantiate(&self.engine, &self.component, &self.host, &self.interrupt) {
            Ok((store, bindings)) => {
                self.store = store;
                self.bindings = bindings;
            }
            Err(err) => {
                self.host
                    .bridge
                    .log("error", &format!("Extension runtime cannot restart: {err}"));
                return;
            }
        }
        self.replaying = true;
        for (kind, payload) in self.replay.clone() {
            let id = self.next_id;
            self.next_id += 1;
            self.track_load(id, &kind);
            if !self.guest(|bindings, store| {
                bindings
                    .yapi_extension_guest()
                    .call_dispatch(store, id, &kind, &payload)
            }) {
                break;
            }
        }
        self.replaying = false;
    }
}

/// Keeps call `kind` for the replay after a restart when it shapes the
/// instance's state. Only the latest `reload`, which names the extensions
/// that run now, and the latest `bind` count, as each session runs and binds
/// them again.
fn record(replay: &mut Vec<(String, String)>, kind: &str, payload: &str) {
    if !matches!(kind, "load" | "reload" | "bind" | "flags") {
        return;
    }
    if matches!(kind, "reload" | "bind") {
        replay.retain(|(recorded, _)| recorded != kind);
    }
    replay.push((kind.to_owned(), payload.to_owned()));
}

fn instantiate(
    engine: &Engine,
    component: &Component,
    host: &Arc<Host>,
    interrupt: &Arc<AtomicBool>,
) -> Result<(Store<State>, Extension), Error> {
    let options = &host.options;
    let mut wasi = WasiCtx::builder();
    if options.grants.filesystem {
        for root in &options.filesystem_roots {
            wasi.preopened_dir(root, root.to_string_lossy(), FsPerms::ReadWrite)
                .map_err(|err| Error::Instantiate(format!("{}: {err}", root.display())))?;
        }
    }
    let state = State {
        wasi: wasi.build(),
        table: ResourceTable::new(),
        limits: StoreLimitsBuilder::new()
            .memory_size(options.memory_limit)
            .build(),
        host: host.clone(),
        started: Vec::new(),
        call_started: Instant::now(),
        // Instantiation runs the runtime's own start-up code.
        loading: true,
        host_called: Instant::now(),
        host_depth: 0,
        host_time: Duration::ZERO,
        interrupt: interrupt.clone(),
    };
    let mut store = Store::new(&engine.engine, state);
    store.limiter(|state| &mut state.limits);
    // Host calls, WASI's included, do not count toward the call limit.
    // Lowering a result reenters the guest, whose epoch check calls in again.
    store.call_hook(|mut context, hook| {
        let state = context.data_mut();
        match hook {
            CallHook::CallingHost => {
                if state.host_depth == 0 {
                    state.host_called = Instant::now();
                }
                state.host_depth += 1;
            }
            CallHook::ReturningFromHost => {
                state.host_depth = state.host_depth.saturating_sub(1);
                if state.host_depth == 0 {
                    state.host_time += state.host_called.elapsed();
                }
            }
            CallHook::CallingWasm | CallHook::ReturningFromWasm => {}
        }
        Ok(())
    });
    store.epoch_deadline_callback(|context| {
        let state = context.data();
        if state.interrupt.load(Ordering::Relaxed) {
            return Err(wasmtime::Error::msg("the instance was stopped"));
        }
        let open = if state.host_depth > 0 {
            state.host_called.elapsed()
        } else {
            Duration::ZERO
        };
        let busy = state
            .call_started
            .elapsed()
            .saturating_sub(state.host_time + open);
        let (limit, task) = match state.loading {
            true => (LOAD_LIMIT, " as it loaded"),
            false => (CALL_LIMIT, ""),
        };
        if busy > limit {
            Err(wasmtime::Error::msg(format!(
                "an extension computed for more than {} seconds without yielding{task}",
                limit.as_secs()
            )))
        } else {
            Ok(UpdateDeadline::Continue(1))
        }
    });
    let bindings = Extension::instantiate(&mut store, component, &engine.linker)
        .map_err(|err| Error::Instantiate(format!("{err:#}")))?;
    Ok((store, bindings))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Time the guest spends blocked in WASI, here opening a FIFO that no
    /// one writes yet, does not count toward the call limit. WASI cannot
    /// read the FIFO, but the open waits for the writer.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn host_time_not_counted() {
        let dir = std::env::temp_dir().join(format!("yapi-host-time-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (fifo, main) = (dir.join("fifo"), dir.join("main.js"));
        let made = std::process::Command::new("mkfifo").arg(&fifo).status();
        assert!(made.unwrap().success());
        let read = format!(
            "try {{ readFileSync({:?}); }} catch {{}}",
            fifo.display().to_string()
        );
        let source = format!(
            "import {{ readFileSync }} from \"node:fs\";\nexport default function () {{ {read} }}\n"
        );
        std::fs::write(&main, source).unwrap();
        let engine = Engine::new(None).unwrap();
        let instance = Instance::start(&engine, Options::new(dir.clone()), Arc::new(NoBridge))
            .await
            .unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(CALL_LIMIT * 2);
            std::fs::write(fifo, "x")
        });
        let extensions = serde_json::json!([{"id": 1, "path": main}]);
        let loaded = instance
            .call(
                "load",
                &serde_json::json!({"cwd": dir, "extensions": extensions}),
            )
            .await;
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded.unwrap()["extensions"][0].get("error"), None);
    }

    /// A blocking request whose result the host lowers into guest memory
    /// does not count toward the call limit either.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn host_result_not_counted() {
        let dir = std::env::temp_dir().join(format!("yapi-host-result-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let main = dir.join("main.js");
        let source = format!(
            "import {{ execSync }} from \"node:child_process\";\nexport default function () {{ execSync(\"sleep {}\"); }}\n",
            (CALL_LIMIT * 2).as_secs()
        );
        std::fs::write(&main, source).unwrap();
        let engine = Engine::new(None).unwrap();
        let instance = Instance::start(&engine, Options::new(dir.clone()), Arc::new(NoBridge))
            .await
            .unwrap();
        let extensions = serde_json::json!([{"id": 1, "path": main}]);
        let loaded = instance
            .call(
                "load",
                &serde_json::json!({"cwd": dir, "extensions": extensions}),
            )
            .await;
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded.unwrap()["extensions"][0].get("error"), None);
    }

    /// A load that computes without yielding stops at the load budget,
    /// while a later call computes up to the call limit.
    #[tokio::test(flavor = "multi_thread")]
    async fn load_budget_stops_spin() {
        use serde_json::json;
        let dir = std::env::temp_dir().join(format!("yapi-load-budget-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (spin, command) = (dir.join("spin.js"), dir.join("command.js"));
        std::fs::write(&spin, "while (true) {}\nexport default function () {}\n").unwrap();
        let handler = "pi.registerCommand(\"spin\", { handler: async () => { while (true) {} } });";
        let source = format!("export default function (pi) {{ {handler} }}\n");
        std::fs::write(&command, source).unwrap();
        let engine = Engine::new(None).unwrap();
        let mut errors = Vec::new();
        for (path, spins) in [(spin, false), (command, true)] {
            let options = Options::new(dir.clone());
            let instance = Instance::start(&engine, options, Arc::new(NoBridge))
                .await
                .unwrap();
            let extensions = json!([{"id": 1, "path": path}]);
            let loaded = instance
                .call("load", &json!({"cwd": dir, "extensions": extensions}))
                .await;
            let result = match spins {
                false => loaded,
                true => {
                    assert_eq!(loaded.unwrap()["extensions"][0].get("error"), None);
                    let spin = json!({"extension": 1, "name": "spin"});
                    instance.call("command", &spin).await
                }
            };
            errors.push(result.unwrap_err().to_string());
        }
        let _ = std::fs::remove_dir_all(&dir);
        let stopped = |limit: Duration, task: &str| {
            let secs = limit.as_secs();
            format!("computed for more than {secs} seconds without yielding{task}")
        };
        assert!(
            errors[0].ends_with(&stopped(LOAD_LIMIT, " as it loaded")),
            "{errors:?}"
        );
        assert!(errors[1].ends_with(&stopped(CALL_LIMIT, "")), "{errors:?}");
    }

    /// A long process reloads and binds once per session. A restart replays
    /// the load, the flags, and the latest reload and binding only.
    #[test]
    fn replay_keeps_latest_reload() {
        let mut replay = Vec::new();
        for kind in [
            "load", "flags", "bind", "emit", "reload", "bind", "reload", "bind",
        ] {
            record(&mut replay, kind, kind);
        }
        let kinds: Vec<&str> = replay.iter().map(|(kind, _)| kind.as_str()).collect();
        assert_eq!(kinds, ["load", "flags", "reload", "bind"]);
    }
}
