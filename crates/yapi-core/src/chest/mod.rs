//! chest, a project package manager for skills, subagents and extensions:
//! the `chest.toml` manifest, the `chest.lock`, content hashes and the
//! placement of installed items. It has no pi counterpart.

pub mod hash;
pub mod lock;
pub mod manifest;
pub mod place;

use std::path::{Path, PathBuf};

/// Why a chest operation failed.
#[derive(Debug, thiserror::Error)]
pub enum ChestError {
    /// A manifest, lock or package tree that chest refuses.
    #[error("{0}")]
    Invalid(String),
    /// A file operation failed.
    #[error("{path}: {source}")]
    Io {
        /// Where.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
}

fn invalid(message: impl Into<String>) -> ChestError {
    ChestError::Invalid(message.into())
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ChestError + '_ {
    move |source| ChestError::Io {
        path: path.to_path_buf(),
        source,
    }
}
