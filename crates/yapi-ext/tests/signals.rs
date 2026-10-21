//! Signals that the process catches leave the extension runtime running,
//! whichever thread they land on.
#![cfg(target_os = "macos")]
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::time::Duration;

extern "C" fn ignore(_: libc::c_int) {}

/// wasmtime's Mach exception thread aborted the process when a caught signal
/// interrupted its wait (bytecodealliance/wasmtime#14393).
#[test]
fn caught_signal_no_abort() {
    let _engine = yapi_ext::Engine::new(None).unwrap();
    let mut threads: libc::thread_act_array_t = std::ptr::null_mut();
    let mut count = 0;
    // SAFETY: a no-op handler is async-signal-safe, and `task_threads` fills
    // `threads` with `count` thread ports of this task.
    unsafe {
        libc::signal(libc::SIGUSR1, ignore as *const () as libc::sighandler_t);
        #[allow(
            deprecated,
            reason = "libc refers Mach calls to the mach2 crate, which yapi does not need"
        )]
        let task = libc::mach_task_self_;
        assert_eq!(
            libc::task_threads(task, &mut threads, &mut count),
            libc::KERN_SUCCESS
        );
        for index in 0..count as usize {
            let thread = libc::pthread_from_mach_thread_np(*threads.add(index));
            if thread != 0 && thread != libc::pthread_self() {
                libc::pthread_kill(thread, libc::SIGUSR1);
            }
        }
    }
    // An abort happens on the signalled thread, after `pthread_kill` returns.
    std::thread::sleep(Duration::from_millis(500));
}
