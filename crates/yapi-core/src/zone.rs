//! The local time zone, chosen as Node chooses it: the zone `TZ` names when
//! it is set, else the system's zone.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use tz::{TimeZone, TimeZoneSettings};

/// A time zone and its IANA name, when it has one.
#[derive(Debug)]
pub struct Zone {
    zone: TimeZone,
    name: Option<String>,
}

static LOCAL: LazyLock<Zone> = LazyLock::new(|| Zone::from_tz(std::env::var("TZ").ok().as_deref()));

/// The process's local zone, loaded on first use.
pub fn local() -> &'static Zone {
    &LOCAL
}

/// Zones found by name, keyed by lowercased name. At most one entry per
/// zone file, so it stays small.
static NAMED: LazyLock<Mutex<HashMap<String, &'static Zone>>> = LazyLock::new(Mutex::default);

/// The zone IANA name `name` spells, matched without regard to case as
/// `Intl` does, or `None` when no zone file has that name. The zone is
/// shared and lives for the process.
pub fn named(name: &str) -> Option<&'static Zone> {
    let key = name.to_ascii_lowercase();
    let mut cache = NAMED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(zone) = cache.get(&key) {
        return Some(zone);
    }
    let canonical = canonical_name(name)?;
    let zone = Box::leak(Box::new(Zone::from_tz(Some(&canonical))));
    // `from_tz` falls back to UTC for what it cannot read, which is no zone.
    zone.name()?;
    Some(*cache.entry(key).or_insert(zone))
}

/// The name of the zone file that `name` matches without regard to case,
/// searched in the zoneinfo directories.
fn canonical_name(name: &str) -> Option<String> {
    if name.is_empty()
        || name.starts_with('/')
        || name.split('/').any(|part| part == ".." || part == ".")
    {
        return None;
    }
    TimeZoneSettings::DEFAULT_DIRECTORIES
        .iter()
        .find_map(|root| {
            let mut path = Path::new(root).to_path_buf();
            let mut found = Vec::new();
            for part in name.split('/') {
                let entry = std::fs::read_dir(&path).ok()?.flatten().find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(part)
                })?;
                found.push(entry.file_name().to_string_lossy().into_owned());
                path = entry.path();
            }
            path.is_file().then(|| found.join("/"))
        })
}

impl Zone {
    /// The zone for the value of `TZ`. Without one it is the system's zone,
    /// from `/etc/localtime`. A value names a zone file or spells a POSIX
    /// rule. A value that does neither means UTC, as in Node, and so does an
    /// empty value or a system without a zone.
    pub fn from_tz(tz: Option<&str>) -> Zone {
        let Some(tz) = tz else {
            let name = std::fs::read_link("/etc/localtime")
                .ok()
                .and_then(|target| {
                    let target = target.to_string_lossy();
                    target
                        .rsplit_once("zoneinfo/")
                        .map(|(_, name)| name.to_owned())
                });
            return Zone::new(TimeZone::local().ok(), name);
        };
        if tz.is_empty() {
            // Node's name for the zone of an empty value.
            return Zone {
                zone: TimeZone::utc(),
                name: Some("Etc/Unknown".to_owned()),
            };
        }
        let file = tz.strip_prefix(':').unwrap_or(tz);
        // Node names only zones it finds by name, not files given by path.
        let named = !file.starts_with('/')
            && TimeZoneSettings::DEFAULT_DIRECTORIES
                .iter()
                .any(|dir| Path::new(dir).join(file).is_file());
        Zone::new(
            TimeZone::from_posix_tz(tz).ok(),
            named.then(|| file.to_owned()),
        )
    }

    fn new(zone: Option<TimeZone>, name: Option<String>) -> Zone {
        let Some(zone) = zone else {
            return Zone {
                zone: TimeZone::utc(),
                name: None,
            };
        };
        // Node reports the UTC zone's aliases by its canonical name.
        let name = name.map(|name| match name.as_str() {
            "Etc/UTC" | "Etc/UCT" | "UCT" | "Etc/Universal" | "Universal" | "Etc/Zulu" | "Zulu" => {
                "UTC".to_owned()
            }
            _ => name,
        });
        Zone { zone, name }
    }

    /// The zone's IANA name, such as `Europe/London`, when it has one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The zone's offset from UTC at `unix_seconds`, in seconds east.
    pub fn offset(&self, unix_seconds: i64) -> i32 {
        self.zone
            .find_local_time_type(unix_seconds)
            .map_or(0, |kind| kind.ut_offset())
    }

    /// The offset that turns wall-clock time `local`, in seconds as if UTC,
    /// into UTC, as Node chooses it: the earlier instant of a time that
    /// occurs twice, and the offset before the change for a skipped time.
    pub fn local_offset(&self, local: i64) -> i32 {
        const DAY: i64 = 86_400;
        let before = self.offset(local.saturating_sub(DAY));
        let after = self.offset(local.saturating_add(DAY));
        [before, after]
            .into_iter()
            .find(|&offset| self.offset(local.saturating_sub(i64::from(offset))) == offset)
            .unwrap_or(before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-01-15 and 2026-07-15, at noon UTC.
    const WINTER: i64 = 1_768_478_400;
    const SUMMER: i64 = 1_784_116_800;

    #[test]
    fn named_zone_lookup() {
        let tokyo = named("asia/TOKYO").expect("Asia/Tokyo is in zoneinfo");
        assert_eq!(tokyo.name(), Some("Asia/Tokyo"));
        assert_eq!(tokyo.offset(WINTER), 9 * 3600);
        assert_eq!(
            named("America/New_York").map(|zone| zone.offset(SUMMER)),
            Some(-4 * 3600)
        );
        for name in [
            "Foo/Bar",
            "",
            "Asia",
            "../Asia/Tokyo",
            "/etc/localtime",
            "EST5EDT,M3.2.0,M11.1.0",
        ] {
            assert!(named(name).is_none(), "{name:?}");
        }
    }

    #[test]
    fn invalid_tz_means_utc() {
        for tz in ["Invalid/Zone", "", ":", "Asia/Tokyo ", "/no/such/file"] {
            let zone = Zone::from_tz(Some(tz));
            assert_eq!((zone.offset(WINTER), zone.offset(SUMMER)), (0, 0), "{tz:?}");
            let name = tz.is_empty().then_some("Etc/Unknown");
            assert_eq!(zone.name(), name, "{tz:?}");
        }
    }

    #[test]
    fn unnamed_posix_rules() {
        let zone = Zone::from_tz(Some("CET-1CEST,M3.5.0,M10.5.0/3"));
        assert_eq!((zone.offset(WINTER), zone.offset(SUMMER)), (3600, 7200));
        assert_eq!(zone.name(), None);
        assert_eq!(Zone::from_tz(Some("EST5")).offset(SUMMER), -5 * 3600);
    }

    #[test]
    fn wall_clock_like_node() {
        const HOUR: i32 = 3600;
        let new_york = Zone::from_tz(Some("EST5EDT,M3.2.0,M11.1.0"));
        // 2026-03-08 02:30 is skipped, 03:30 follows the change, 2026-11-01
        // 01:30 occurs twice and 02:30 follows the change.
        assert_eq!(new_york.local_offset(1_772_937_000), -5 * HOUR);
        assert_eq!(new_york.local_offset(1_772_940_600), -4 * HOUR);
        assert_eq!(new_york.local_offset(1_793_496_600), -4 * HOUR);
        assert_eq!(new_york.local_offset(1_793_500_200), -5 * HOUR);
        // East of UTC the day of the change starts before it: 2026-10-04 00:00,
        // the skipped 02:30, and 03:30.
        let sydney = Zone::from_tz(Some("AEST-10AEDT,M10.1.0,M4.1.0/3"));
        assert_eq!(sydney.local_offset(1_791_072_000), 10 * HOUR);
        assert_eq!(sydney.local_offset(1_791_081_000), 10 * HOUR);
        assert_eq!(sydney.local_offset(1_791_084_600), 11 * HOUR);
    }

    #[test]
    fn zone_files_keep_name() {
        // Skipped on systems without a time zone database.
        if !Path::new("/usr/share/zoneinfo/America/New_York").is_file() {
            return;
        }
        let zone = Zone::from_tz(Some("America/New_York"));
        assert_eq!(
            (zone.offset(WINTER), zone.offset(SUMMER)),
            (-5 * 3600, -4 * 3600)
        );
        assert_eq!(zone.name(), Some("America/New_York"));
        assert_eq!(
            Zone::from_tz(Some(":America/New_York")).name(),
            Some("America/New_York")
        );
        assert_eq!(Zone::from_tz(Some("Etc/UTC")).name(), Some("UTC"));
        assert_eq!(
            Zone::from_tz(Some("/usr/share/zoneinfo/America/New_York")).name(),
            None
        );
    }
}
