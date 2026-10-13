//! Local time for QuickJS's `Date`, in the host's time zone.
//!
//! QuickJS reads a zone's offset through libc's `localtime_r`, which WASI's
//! libc answers in UTC. This module defines `localtime_r` in its place, so
//! local dates follow the host's zone in extensions and in codemode scripts
//! alike. pi's codemode runtime hooks the same QuickJS function.
//!
//! QuickJS also asks for the offset at a wall-clock time as if it were an
//! instant, which is wrong for a few hours around a change of offset. The
//! extension runtime sets [`set_wall_clock`] while it converts wall-clock
//! times, and the offset then comes from the wall-clock time itself, as
//! Node resolves it. Codemode scripts keep QuickJS's rule, as in pi.

use core::ffi::{c_char, c_int, c_long};
use std::cell::Cell;

use crate::bindings::yapi::extension::host;

thread_local! {
    static WALL_CLOCK: Cell<bool> = const { Cell::new(false) };
    /// The last answer, by whether the time was a wall-clock time and the
    /// time: QuickJS asks once per field it formats.
    static LAST: Cell<Option<(bool, i64, i32)>> = const { Cell::new(None) };
}

/// Whether `localtime_r` receives wall-clock times instead of instants.
pub fn set_wall_clock(on: bool) {
    WALL_CLOCK.set(on);
}

/// WASI libc's `struct tm`.
#[repr(C)]
pub struct Tm {
    sec: c_int,
    min: c_int,
    hour: c_int,
    mday: c_int,
    mon: c_int,
    year: c_int,
    wday: c_int,
    yday: c_int,
    isdst: c_int,
    gmtoff: c_long,
    zone: *const c_char,
    nsec: c_int,
}

unsafe extern "C" {
    fn gmtime_r(time: *const i64, tm: *mut Tm) -> *mut Tm;
}

/// libc's `localtime_r`: `*time` in the host's zone. QuickJS reads only
/// `tm_gmtoff`.
///
/// # Safety
///
/// `time` and `tm` must be valid, as libc requires.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn localtime_r(time: *const i64, tm: *mut Tm) -> *mut Tm {
    // SAFETY: the caller passes a valid `time`.
    let time = unsafe { *time };
    let wall_clock = WALL_CLOCK.get();
    let offset = match LAST.get() {
        Some((wall, last, offset)) if (wall, last) == (wall_clock, time) => offset,
        _ => {
            let kind = if wall_clock {
                "timezone.localOffset"
            } else {
                "timezone.offset"
            };
            let offset: i32 = host::request(kind, &time.to_string())
                .ok()
                .and_then(|offset| offset.parse().ok())
                .unwrap_or_default();
            LAST.set(Some((wall_clock, time, offset)));
            offset
        }
    };
    let local = time.saturating_add(i64::from(offset));
    // SAFETY: `local` lives through the call and the caller passes a valid
    // `tm`; `gmtime_r` returns it filled in, or null.
    unsafe {
        let result = gmtime_r(&local, tm);
        if !result.is_null() {
            (*result).gmtoff = offset as c_long;
        }
        result
    }
}
