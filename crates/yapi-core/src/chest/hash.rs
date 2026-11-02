//! The content hash chest locks for each installed entry.

use std::path::{Path, PathBuf};

use aws_lc_rs::digest::{Context, SHA256, digest};
use yapi_types::time::hex;

use super::{ChestError, invalid, io};

/// The content hash of the package tree at `root`, as `sha256-<hex>`.
///
/// It covers every regular file below `root` except those in `.git` folders,
/// at any depth, in the byte order of their `/`-separated relative paths.
/// Each file adds its path, a NUL, `x` when it is executable (any execute
/// bit on Unix, never elsewhere) or `-`, then the SHA-256 of its contents.
/// Fails on any symbolic link, `root` included, and on a path that is not
/// UTF-8.
pub fn tree_hash(root: &Path) -> Result<String, ChestError> {
    if root.symlink_metadata().map_err(io(root))?.is_symlink() {
        return Err(refused(root));
    }
    let mut files = Vec::new();
    collect(root, "", &mut files)?;
    files.sort();
    let mut context = Context::new(&SHA256);
    for (relative, path) in files {
        let metadata = path.symlink_metadata().map_err(io(&path))?;
        context.update(relative.as_bytes());
        context.update(b"\0");
        context.update(if executable(&metadata) { b"x" } else { b"-" });
        // The raw 32-byte digest, not its hex form. The scheme is modelled on
        // APM's native-bundle hash, but chest does not aim to match APM's
        // output.
        let contents = std::fs::read(&path).map_err(io(&path))?;
        context.update(digest(&SHA256, &contents).as_ref());
    }
    Ok(format!("sha256-{}", hex(context.finish().as_ref())))
}

fn refused(path: &Path) -> ChestError {
    invalid(format!(
        "{}: symbolic links are not allowed in a package",
        path.display()
    ))
}

/// Adds the regular files below `dir`, whose relative path is `prefix`, to
/// `files` as their relative paths and full paths.
fn collect(dir: &Path, prefix: &str, files: &mut Vec<(String, PathBuf)>) -> Result<(), ChestError> {
    for entry in std::fs::read_dir(dir).map_err(io(dir))? {
        let entry = entry.map_err(io(dir))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(io(&path))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid(format!("{}: the name is not UTF-8", path.display())))?;
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if kind.is_symlink() {
            return Err(refused(&path));
        } else if kind.is_dir() {
            if name != ".git" {
                collect(&path, &relative, files)?;
            }
        } else if kind.is_file() {
            files.push((relative, path));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("yapi-chest-hash-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, path: &str, contents: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    #[cfg(unix)]
    fn chmod(root: &Path, path: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(root.join(path), std::fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn tree_hash_vector() {
        let root = scratch("vector");
        write(&root, "a.txt", "hello\n");
        write(&root, "bin/run", "#!/bin/sh\n");
        chmod(&root, "a.txt", 0o644);
        chmod(&root, "bin/run", 0o755);
        // Computed from the scheme with shell tools alone:
        // { printf 'a.txt\0-'; printf 'hello\n' | shasum -a 256 | cut -c1-64 | xxd -r -p;
        //   printf 'bin/run\0x'; printf '#!/bin/sh\n' | shasum -a 256 | cut -c1-64 | xxd -r -p;
        // } | shasum -a 256
        let vector = "sha256-8ec032806587aaad014b497c7476c1dd2321f8c3acc4e7b03c45101290abc097";
        assert_eq!(tree_hash(&root).unwrap(), vector);

        // `.git` folders at any depth are skipped.
        write(&root, ".git/HEAD", "ref: refs/heads/main\n");
        write(&root, "bin/.git/config", "[core]\n");
        assert_eq!(tree_hash(&root).unwrap(), vector);
        // The execute bit counts, any of the three.
        chmod(&root, "a.txt", 0o654);
        assert_ne!(tree_hash(&root).unwrap(), vector);
        chmod(&root, "a.txt", 0o644);
        // A symbolic link is refused, wherever it is.
        std::os::unix::fs::symlink("a.txt", root.join("bin/link")).unwrap();
        let error = tree_hash(&root).unwrap_err().to_string();
        assert!(
            error.ends_with("bin/link: symbolic links are not allowed in a package"),
            "{error}"
        );
        std::fs::remove_file(root.join("bin/link")).unwrap();
        let link = scratch("vector-link");
        std::fs::remove_dir(&link).unwrap();
        std::os::unix::fs::symlink(&root, &link).unwrap();
        assert!(tree_hash(&link).is_err());

        // Paths and contents cannot trade bytes.
        let (one, two) = (scratch("ab-c"), scratch("a-bc"));
        write(&one, "ab", "c");
        write(&two, "a", "bc");
        assert_ne!(tree_hash(&one).unwrap(), tree_hash(&two).unwrap());
        for dir in [root, link, one, two] {
            let _ = std::fs::remove_dir_all(&dir);
            let _ = std::fs::remove_file(&dir);
        }
    }
}
