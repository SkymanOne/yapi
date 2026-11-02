//! What chest placed on this machine for one manifest, kept outside the
//! repository, with the hash of what each path held. A recorded path whose
//! content still has that hash is chest's to replace or remove; a committed
//! lock never makes a path chest's by itself.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aws_lc_rs::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use yapi_ai::auth::lock::write_atomic;
use yapi_types::time::hex;

use super::hash::tree_hash;
use super::{ChestError, invalid, io};
use crate::settings::Scope;

/// Each placed path, relative to the manifest's root, with what chest put
/// there: `link:<target>` for a link, else the content hash.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Record {
    #[serde(default)]
    placed: BTreeMap<String, String>,
    #[serde(skip)]
    root: PathBuf,
}

/// Whose a path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Held {
    /// The record does not list it.
    Unrecorded,
    /// The record lists it, and it holds what chest put there.
    Owned,
    /// The record lists it, but its content changed since.
    Modified,
}

impl Record {
    /// Where the record of `manifest`, a canonical path, lives in `state`:
    /// `global.toml` for the global manifest, else a file named after the
    /// manifest's path, so that each manifest of a project keeps its own.
    pub(super) fn path(state: &Path, manifest: &Path, scope: Scope) -> PathBuf {
        if scope == Scope::Global {
            return state.join("global.toml");
        }
        let digest = hex(digest(&SHA256, manifest.to_string_lossy().as_bytes()).as_ref());
        state
            .join("projects")
            .join(format!("{}.toml", &digest[..16]))
    }

    /// The record at `path` for paths under `root`, the canonical project
    /// or home folder. Empty when there is none or it does not parse, which
    /// only makes chest more careful.
    pub(super) fn load(path: &Path, root: &Path) -> Record {
        let record: Option<Record> = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml_edit::de::from_str(&text).ok());
        Record {
            root: root.to_path_buf(),
            ..record.unwrap_or_default()
        }
    }

    /// Writes the record to `path`.
    pub(super) fn save(&self, path: &Path) -> Result<(), ChestError> {
        let parent = path.parent().unwrap_or(path);
        std::fs::create_dir_all(parent).map_err(io(parent))?;
        let text =
            toml_edit::ser::to_string_pretty(self).map_err(|err| invalid(err.to_string()))?;
        let text = format!("# What chest placed on this machine. Not for editing.\n{text}");
        write_atomic(path, text.as_bytes(), 0o600).map_err(io(path))
    }

    fn key(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        relative.to_string_lossy().replace('\\', "/")
    }

    /// Whose `path` is, judged by what it holds now: a recorded copy by its
    /// hash, a recorded link by where it points. A link the user points
    /// elsewhere is theirs. A link whose record still holds the content hash
    /// it was placed with, as when chest stopped before noting the link,
    /// counts by what it points to.
    pub(super) fn held(&self, path: &Path) -> Held {
        let Some(recorded) = self.placed.get(&self.key(path)) else {
            return Held::Unrecorded;
        };
        let now = fingerprint(path);
        let placed_as_hash = !recorded.starts_with("link:")
            && now.as_ref().is_some_and(|now| now.starts_with("link:"))
            && content_hash(path).as_ref() == Some(recorded);
        if now.as_ref() == Some(recorded) || placed_as_hash {
            Held::Owned
        } else {
            Held::Modified
        }
    }

    /// Records what `path` holds now, or forgets it when nothing is there.
    pub(super) fn note(&mut self, path: &Path) {
        let key = self.key(path);
        match fingerprint(path) {
            Some(held) => self.placed.insert(key, held),
            None => self.placed.remove(&key),
        };
    }

    /// The hash recorded for `path`.
    pub(super) fn get(&self, path: &Path) -> Option<String> {
        self.placed.get(&self.key(path)).cloned()
    }

    /// Records `hash` for `path`, or forgets it with none.
    pub(super) fn set(&mut self, path: &Path, hash: Option<&str>) {
        let key = self.key(path);
        match hash {
            Some(hash) => self.placed.insert(key, hash.to_owned()),
            None => self.placed.remove(&key),
        };
    }

    /// Every recorded path.
    pub(super) fn paths(&self) -> Vec<PathBuf> {
        self.placed.keys().map(|key| self.root.join(key)).collect()
    }
}

/// What `path` holds as a record notes it: `link:<target>` for a link, else
/// its content hash.
fn fingerprint(path: &Path) -> Option<String> {
    let metadata = path.symlink_metadata().ok()?;
    if metadata.is_symlink() {
        let target = std::fs::read_link(path).ok()?;
        return Some(format!("link:{}", target.to_string_lossy()));
    }
    tree_hash(path).ok()
}

/// The hash of what `path` holds now, a link counting by what it points to.
/// `None` when nothing is there or it cannot be hashed.
pub(super) fn content_hash(path: &Path) -> Option<String> {
    let metadata = path.symlink_metadata().ok()?;
    let held = match metadata.is_symlink() {
        true => path.canonicalize().ok()?,
        false => path.to_path_buf(),
    };
    tree_hash(&held).ok()
}
