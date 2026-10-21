//! Cross-process safety for shared files: a lock compatible with
//! `proper-lockfile`, which pi uses for `auth.json`, `settings.json` and
//! `trust.json`, and an atomic replacement of a file's contents. The lock is a
//! `<file>.lock` directory, and one whose modification time is older than the
//! stale period may be taken over.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use tokio_util::sync::CancellationToken;

/// pi's `stale` option for `auth.json`.
const STALE: Duration = Duration::from_secs(30);
const MAX_DELAY: Duration = Duration::from_secs(1);

/// Why the lock could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// The operation was cancelled while waiting.
    #[error("Login cancelled")]
    Cancelled,
    /// Another holder kept the lock for the whole stale period.
    #[error("Lock file is already being held")]
    Held,
    /// Creating the lock directory failed for another reason.
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// A held lock; dropping it releases the lock.
#[derive(Debug)]
pub struct FileLock {
    dir: PathBuf,
}

impl FileLock {
    /// Locks `file`, retrying with jittered backoff until the stale period has
    /// passed.
    pub async fn acquire(file: &Path, cancel: &CancellationToken) -> Result<FileLock, LockError> {
        let dir = lock_dir(file);
        let deadline = Instant::now() + STALE;
        let mut retry = 0u32;
        loop {
            if cancel.is_cancelled() {
                return Err(LockError::Cancelled);
            }
            if let Some(lock) = try_lock(&dir)? {
                return Ok(lock);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(LockError::Held);
            }
            let base =
                Duration::from_millis(10u64.saturating_mul(1 << retry.min(10))).min(MAX_DELAY / 2);
            retry += 1;
            let delay = (base + base.mul_f64(jitter())).min(remaining);
            cancel
                .run_until_cancelled(tokio::time::sleep(delay))
                .await
                .ok_or(LockError::Cancelled)?;
        }
    }

    /// Locks `file` from synchronous code, as pi's `lockSync` with its retries
    /// for `settings.json` and `trust.json`: ten attempts 20 ms apart, then
    /// [`LockError::Held`].
    pub fn acquire_blocking(file: &Path) -> Result<FileLock, LockError> {
        let dir = lock_dir(file);
        for attempt in 1..=10 {
            if let Some(lock) = try_lock(&dir)? {
                return Ok(lock);
            }
            if attempt < 10 {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        Err(LockError::Held)
    }
}

fn lock_dir(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".lock");
    PathBuf::from(name)
}

/// Takes the lock, or `None` while another holder keeps it fresh.
fn try_lock(dir: &Path) -> std::io::Result<Option<FileLock>> {
    loop {
        match std::fs::create_dir(dir) {
            Ok(()) => {
                return Ok(Some(FileLock {
                    dir: dir.to_path_buf(),
                }));
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                // Another process may take a stale lock over first; either way, try again.
                if !is_stale(dir) || std::fs::remove_dir(dir).is_err() {
                    return Ok(None);
                }
            }
            Err(err) => return Err(err),
        }
    }
}

/// Replaces `path`'s contents so that readers see the old or the new bytes,
/// never a partial file: writes a temporary file beside it, flushes it to disk
/// and renames it over `path`. An existing file keeps its permissions; a new
/// one is created with `create_mode` (Unix, less the umask). A symbolic link
/// keeps pointing at its target, which receives the contents. On error the
/// file is unchanged.
pub fn write_atomic(path: &Path, contents: &[u8], create_mode: u32) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut name = std::ffi::OsString::from(".");
    name.push(target.file_name().unwrap_or_default());
    name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let temp = target.with_file_name(name);
    let written = write_temp(&temp, &target, contents, create_mode)
        .and_then(|()| std::fs::rename(&temp, &target));
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

fn write_temp(
    temp: &Path,
    target: &Path,
    contents: &[u8],
    create_mode: u32,
) -> std::io::Result<()> {
    let existing = std::fs::metadata(target)
        .ok()
        .map(|meta| meta.permissions());
    // A file a crashed process with the same id left behind would keep its mode.
    let _ = std::fs::remove_file(temp);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, create_mode);
    #[cfg(not(unix))]
    let _ = create_mode;
    let mut file = options.open(temp)?;
    if let Some(permissions) = existing {
        file.set_permissions(permissions)?;
    }
    file.write_all(contents)?;
    file.sync_all()
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.dir);
    }
}

fn is_stale(dir: &Path) -> bool {
    std::fs::metadata(dir)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age > STALE)
}

/// A value in `[0, 1)`; spreads retries of competing processes.
fn jitter() -> f64 {
    // Like pi's `Math.random`, jitter never fails; without the generator it is 0.
    let bytes = yapi_types::time::try_random_bytes().unwrap_or_default();
    f64::from(u16::from_le_bytes(bytes)) / 65536.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn excludes_and_releases() {
        let dir = std::env::temp_dir().join(format!("yapi-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("auth.json");
        let cancel = CancellationToken::new();
        let held = FileLock::acquire(&file, &cancel).await.unwrap();
        assert!(dir.join("auth.json.lock").is_dir());
        let waiting = tokio::spawn({
            let file = file.clone();
            let cancel = cancel.clone();
            async move { FileLock::acquire(&file, &cancel).await.map(drop) }
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!waiting.is_finished());
        drop(held);
        waiting.await.unwrap().unwrap();
        assert!(!dir.join("auth.json.lock").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn stale_lock_taken_over() {
        let dir = std::env::temp_dir().join(format!("yapi-lock-stale-{}", std::process::id()));
        let lock = dir.join("auth.json.lock");
        std::fs::create_dir_all(&lock).unwrap();
        let old = SystemTime::now() - STALE - Duration::from_secs(1);
        std::fs::File::open(&lock)
            .unwrap()
            .set_modified(old)
            .unwrap();
        let (file, cancel) = (dir.join("auth.json"), CancellationToken::new());
        let acquire = FileLock::acquire(&file, &cancel);
        let held = tokio::time::timeout(Duration::from_secs(1), acquire).await;
        assert!(matches!(held, Ok(Ok(_))));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn blocking_lock_gives_up() {
        let dir = std::env::temp_dir().join(format!("yapi-lock-sync-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let held = FileLock::acquire_blocking(&file).unwrap();
        assert!(matches!(
            FileLock::acquire_blocking(&file),
            Err(LockError::Held)
        ));
        drop(held);
        FileLock::acquire_blocking(&file).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_mode_links() {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        let dir = std::env::temp_dir().join(format!("yapi-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let auth = dir.join("auth.json");
        write_atomic(&auth, b"{}", 0o600).unwrap();
        assert_eq!(mode(&auth), 0o600);
        std::fs::set_permissions(&auth, std::fs::Permissions::from_mode(0o640)).unwrap();
        write_atomic(&auth, b"{\"a\":1}", 0o600).unwrap();
        assert_eq!(std::fs::read_to_string(&auth).unwrap(), "{\"a\":1}");
        assert_eq!(mode(&auth), 0o640);

        let link = dir.join("link.json");
        std::os::unix::fs::symlink(&auth, &link).unwrap();
        write_atomic(&link, b"{\"b\":2}", 0o666).unwrap();
        assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
        assert_eq!(std::fs::read_to_string(&auth).unwrap(), "{\"b\":2}");

        // Without room for the temporary file, the old contents stay.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let failed = write_atomic(&auth, b"{}", 0o600);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(failed.is_err());
        assert_eq!(std::fs::read_to_string(&auth).unwrap(), "{\"b\":2}");
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(names.len(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
