//! What chest placed on this machine in one project, or in the home folder
//! for the global manifest, kept outside the repository: the hash of what
//! each path held and the manifests whose entries put it there. A recorded
//! path whose content still has that hash is chest's to replace or remove. A
//! committed lock never makes a path chest's by itself.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use aws_lc_rs::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use yapi_ai::auth::lock::write_atomic;
use yapi_types::time::hex;

use super::hash::tree_hash;
use super::{ChestError, invalid, io};
use crate::settings::Scope;

/// Each placed path, relative to the project's root, with what chest put
/// there. The manifest it is loaded for is the one whose sync it serves.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Record {
    #[serde(default)]
    placed: BTreeMap<String, Placed>,
    #[serde(skip)]
    root: PathBuf,
    #[serde(skip)]
    manifest: String,
}

/// What chest put at one path.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Placed {
    /// `link:<target>` for a link, else the content hash.
    held: String,
    /// What a placement in progress brings. Until it is noted, the path is
    /// chest's holding either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    next: Option<String>,
    /// The manifests, by file name, whose entries occupy the path.
    #[serde(default)]
    by: BTreeSet<String>,
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
    /// Where the record of the project at `root`, a canonical path, lives
    /// in `state`: `global.toml` for the global manifest, else a file named
    /// after the root, which every manifest of the project shares.
    pub(super) fn path(state: &Path, root: &Path, scope: Scope) -> PathBuf {
        if scope == Scope::Global {
            return state.join("global.toml");
        }
        let digest = hex(digest(&SHA256, root.to_string_lossy().as_bytes()).as_ref());
        state
            .join("projects")
            .join(format!("{}.toml", &digest[..16]))
    }

    /// The record at `path` for paths under `root`, the canonical project
    /// or home folder, as the sync of manifest `manifest`, a file name, sees
    /// it. Empty when there is none or it does not parse, which only makes
    /// chest more careful.
    pub(super) fn load(path: &Path, root: &Path, manifest: &str) -> Record {
        let record: Option<Record> = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml_edit::de::from_str(&text).ok());
        Record {
            root: root.to_path_buf(),
            manifest: manifest.to_owned(),
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
    /// hash, a recorded link by where it points, accepting what a placement
    /// in progress brings as well. A link the user points elsewhere is
    /// theirs. A link whose record still holds a content hash, as when chest
    /// stopped before noting the link, counts by what it points to.
    pub(super) fn held(&self, path: &Path) -> Held {
        let Some(placed) = self.placed.get(&self.key(path)) else {
            return Held::Unrecorded;
        };
        let now = fingerprint(path);
        let linked = now.as_ref().is_some_and(|now| now.starts_with("link:"));
        let accepted = std::iter::once(&placed.held).chain(&placed.next);
        let owned = accepted.into_iter().any(|recorded| {
            now.as_ref() == Some(recorded)
                || !recorded.starts_with("link:")
                    && linked
                    && content_hash(path).as_ref() == Some(recorded)
        });
        if owned { Held::Owned } else { Held::Modified }
    }

    /// Whether the record says `path` holds `hash`, or a link chest made,
    /// with no placement in progress.
    pub(super) fn holds(&self, path: &Path, hash: &str) -> bool {
        let placed = self.placed.get(&self.key(path));
        placed.is_some_and(|placed| {
            placed.next.is_none() && (placed.held == hash || placed.held.starts_with("link:"))
        })
    }

    /// Records that `path` holds `hash`, as chest found it, for this
    /// manifest.
    pub(super) fn adopt(&mut self, path: &Path, hash: &str) {
        let key = self.key(path);
        let by = BTreeSet::from([self.manifest.clone()]);
        let placed = Placed {
            held: hash.to_owned(),
            next: None,
            by,
        };
        self.placed.insert(key, placed);
    }

    /// Records, before it is written, that this manifest is putting `hash`
    /// at `path`: what it held stays accepted until [`Record::noted`].
    pub(super) fn expect(&mut self, path: &Path, hash: &str) {
        let key = self.key(path);
        let manifest = self.manifest.clone();
        let placed = self.placed.entry(key).or_insert_with(|| Placed {
            held: hash.to_owned(),
            ..Placed::default()
        });
        if placed.held != hash {
            placed.next = Some(hash.to_owned());
        }
        placed.by.insert(manifest);
    }

    /// Records what `path` holds once placed: where it points for a link,
    /// else `hash`, the new content. Forgets it when nothing is
    /// there.
    pub(super) fn noted(&mut self, path: &Path, hash: &str) {
        let key = self.key(path);
        let held = match path.symlink_metadata() {
            Ok(metadata) if metadata.is_symlink() => fingerprint(path),
            Ok(_) => Some(hash.to_owned()),
            Err(_) => None,
        };
        match (held, self.placed.get_mut(&key)) {
            (Some(held), Some(placed)) => {
                placed.held = held;
                placed.next = None;
            }
            (Some(held), None) => self.adopt(path, &held),
            (None, _) => {
                self.placed.remove(&key);
            }
        }
    }

    /// What the record says of `path`, to put back with [`Record::restore`].
    pub(super) fn get(&self, path: &Path) -> Option<Placed> {
        self.placed.get(&self.key(path)).cloned()
    }

    /// Puts back what [`Record::get`] returned for `path`.
    pub(super) fn restore(&mut self, path: &Path, placed: Option<Placed>) {
        let key = self.key(path);
        match placed {
            Some(placed) => self.placed.insert(key, placed),
            None => self.placed.remove(&key),
        };
    }

    /// Forgets `path` altogether.
    pub(super) fn forget(&mut self, path: &Path) {
        self.placed.remove(&self.key(path));
    }

    /// Whether a manifest other than this one also occupies `path`.
    pub(super) fn shared(&self, path: &Path) -> bool {
        let placed = self.placed.get(&self.key(path));
        placed.is_some_and(|placed| placed.by.iter().any(|by| *by != self.manifest))
    }

    /// Releases this manifest's hold on `path`, which stays recorded for any
    /// other that occupies it, and is forgotten otherwise.
    pub(super) fn release(&mut self, path: &Path) {
        let key = self.key(path);
        if let Some(placed) = self.placed.get_mut(&key) {
            placed.by.remove(&self.manifest);
            if placed.by.is_empty() {
                self.placed.remove(&key);
            }
        }
    }

    /// Every path this manifest's entries occupy, as recorded.
    pub(super) fn paths(&self) -> Vec<PathBuf> {
        let mine = self.placed.iter();
        mine.filter(|(_, placed)| placed.by.contains(&self.manifest))
            .map(|(key, _)| self.root.join(key))
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Until a placement is noted, the path is chest's with what it held or
    /// what is being written, so a stop partway leaves nothing "modified".
    #[test]
    fn pending_hash_accepted() {
        let dir = std::env::temp_dir().join(format!("yapi-chest-record-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (pdf, new) = (dir.join("pdf"), dir.join("new"));
        for (folder, text) in [(&pdf, "old"), (&new, "new")] {
            std::fs::create_dir_all(folder).unwrap();
            std::fs::write(folder.join("SKILL.md"), text).unwrap();
        }
        let (old_hash, new_hash) = (tree_hash(&pdf).unwrap(), tree_hash(&new).unwrap());
        let mut record = Record::load(&dir.join("record.toml"), &dir, "chest.toml");
        record.adopt(&pdf, &old_hash);
        record.expect(&pdf, &new_hash);
        assert_eq!(record.held(&pdf), Held::Owned);
        std::fs::write(pdf.join("SKILL.md"), "new").unwrap();
        assert_eq!(record.held(&pdf), Held::Owned);
        record.noted(&pdf, &new_hash);
        std::fs::write(pdf.join("SKILL.md"), "old").unwrap();
        assert_eq!(record.held(&pdf), Held::Modified);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
