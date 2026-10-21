//! Serializes writes to the same file across concurrently running tool calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Runs `work` once every earlier mutation of the same file (by real path) has
/// finished.
pub async fn with_file_lock<T>(path: &Path, work: impl Future<Output = T>) -> T {
    let _guard = file_lock(path).await;
    work.await
}

/// pi's `withFileMutationQueue` for work outside the host: waits until every
/// earlier mutation of the same file (by real path) has finished, and holds
/// the file until the guard drops.
pub async fn file_lock(path: &Path) -> tokio::sync::OwnedMutexGuard<()> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let lock = {
        let mut locks = yapi_types::sync::lock(LOCKS.get_or_init(Default::default));
        locks.retain(|_, lock| Arc::strong_count(lock) > 1);
        locks.entry(key).or_default().clone()
    };
    lock.lock_owned().await
}
