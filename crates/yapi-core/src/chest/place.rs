//! Where installed skills and agents go: a canonical copy in the first
//! configured folder, relative links to it from the others, and the
//! `.gitignore` block that keeps them out of git.

use std::io;
use std::path::{Component, Path, PathBuf};

use yapi_ai::auth::lock::write_atomic;

use super::manifest::valid_name;
use super::{ChestError, invalid, io};
use crate::packages::npm;
use crate::tools::path::relative;

/// Copies the folder or file `source` to `<folders[0]>/<name>`, then links
/// `<folder>/<name>` in each other folder to that copy, copying it where a
/// link cannot be made. What is at those paths is replaced, so the caller
/// must own them. Returns the placed paths, the canonical copy first.
///
/// Every path must stay within `root`, the project folder or, for the
/// global manifest, the home folder: a folder that is a link must resolve
/// inside it. `name` must be a valid entry name. Either refusal comes
/// before anything is written.
pub fn place(
    source: &Path,
    root: &Path,
    folders: &[PathBuf],
    name: &str,
) -> Result<Vec<PathBuf>, ChestError> {
    place_with(source, root, folders, name, link)
}

/// [`place`], making each link with `link(target, path)`, where `target` is
/// relative to the link's folder.
pub fn place_with(
    source: &Path,
    root: &Path,
    folders: &[PathBuf],
    name: &str,
    link: impl Fn(&Path, &Path) -> io::Result<()>,
) -> Result<Vec<PathBuf>, ChestError> {
    let Some((first, others)) = folders.split_first() else {
        return Err(invalid(format!("No folder to place {name} in")));
    };
    let targets: Vec<PathBuf> = folders.iter().map(|folder| folder.join(name)).collect();
    check_name(name)?;
    targets.iter().try_for_each(|target| within(root, target))?;
    let canonical = first.join(name);
    put(source, &canonical)?;
    let mut placed = vec![canonical.clone()];
    // Real paths, so the link holds when a folder is itself a link.
    let real = |path: &Path| path.canonicalize().map_err(io(path));
    let first_real = real(first)?;
    for folder in others {
        let path = folder.join(name);
        // A folder that is the first one under another name already holds it.
        if folder
            .canonicalize()
            .is_ok_and(|folder| folder == first_real)
        {
            continue;
        }
        remove(&path)?;
        std::fs::create_dir_all(folder).map_err(io(folder))?;
        let target = relative(&real(folder)?, &real(&canonical)?);
        if link(Path::new(&target), &path).is_err() {
            put(&canonical, &path)?;
        }
        placed.push(path);
    }
    Ok(placed)
}

/// Removes `paths`, links themselves rather than what they point to, and
/// nothing else. Refuses, before removing anything, a path outside `root`
/// as [`place`] judges it, or one whose last part is not a valid entry
/// name.
pub fn unplace(root: &Path, paths: &[PathBuf]) -> Result<(), ChestError> {
    for path in paths {
        check_name(&path.file_name().unwrap_or_default().to_string_lossy())?;
        within(root, path)?;
    }
    paths.iter().try_for_each(|path| remove(path))
}

/// Copies `source` to `target` through a staged sibling, so that `target`
/// holds the old copy or the new one, never part of either. A link at
/// `target`, which the caller owns, goes first.
fn put(source: &Path, target: &Path) -> Result<(), ChestError> {
    if target
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.is_symlink())
    {
        remove(target)?;
    }
    if source.symlink_metadata().map_err(io(source))?.is_dir() {
        let staging = npm::stage(target)?;
        let built = copy(source, &staging);
        return npm::swap(&staging, target, built);
    }
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let staging = target.with_file_name(format!(".{name}-staging"));
    remove(&staging)?;
    copy(source, &staging)?;
    std::fs::rename(&staging, target).map_err(|err| {
        let _ = remove(&staging);
        io(target)(err)
    })
}

fn check_name(name: &str) -> Result<(), ChestError> {
    if valid_name(name) {
        Ok(())
    } else {
        Err(invalid(format!("Invalid entry name: {name}")))
    }
}

/// Fails unless `path` has no `..` and the deepest part of its folder that
/// exists resolves inside `root`. A link at `path` itself is not followed:
/// placing replaces the link, and removing removes it.
pub(super) fn within(root: &Path, path: &Path) -> Result<(), ChestError> {
    let outside = || {
        invalid(format!(
            "Refusing {}: it is outside {}",
            path.display(),
            root.display()
        ))
    };
    if path.components().any(|part| part == Component::ParentDir) {
        return Err(outside());
    }
    let root = root.canonicalize().map_err(io(root))?;
    let mut folder = path.parent();
    while let Some(dir) = folder {
        match dir.symlink_metadata() {
            // A link that resolves nowhere fails here, so nothing is ever
            // created through it.
            Ok(_) => match dir.canonicalize() {
                Ok(real) if real.starts_with(&root) => return Ok(()),
                _ => return Err(outside()),
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => folder = dir.parent(),
            Err(err) => return Err(io(dir)(err)),
        }
    }
    Err(outside())
}

#[cfg(unix)]
fn link(target: &Path, path: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, path)
}

#[cfg(windows)]
fn link(target: &Path, path: &Path) -> io::Result<()> {
    let resolved = path.parent().map(|parent| parent.join(target));
    if resolved.is_some_and(|resolved| resolved.is_dir()) {
        std::os::windows::fs::symlink_dir(target, path)
    } else {
        std::os::windows::fs::symlink_file(target, path)
    }
}

#[cfg(not(any(unix, windows)))]
fn link(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

fn remove(path: &Path) -> Result<(), ChestError> {
    npm::remove_path(path).map_err(io(path))
}

/// Copies the folder or file `from` to `to`, leaving out `.git` folders and
/// refusing links.
pub(super) fn copy(from: &Path, to: &Path) -> Result<(), ChestError> {
    let metadata = from.symlink_metadata().map_err(io(from))?;
    if metadata.is_symlink() {
        return Err(invalid(format!(
            "{}: symbolic links are not allowed in a package",
            from.display()
        )));
    }
    if !metadata.is_dir() {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        return std::fs::copy(from, to).map(drop).map_err(io(to));
    }
    std::fs::create_dir_all(to).map_err(io(to))?;
    for entry in std::fs::read_dir(from).map_err(io(from))? {
        let entry = entry.map_err(io(from))?;
        if entry.file_name() != ".git" {
            copy(&entry.path(), &to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Rewrites the block of manifest `manifest` (a file name such as
/// `chest.toml`) in `root/.gitignore` to list `entries`, sorted. The block
/// runs from `# BEGIN chest (<manifest>)` to `# END chest`, and every line
/// outside it stays as it was. With no entries the block goes.
///
/// Each entry must be a path relative to `root` with no `.` or `..` part,
/// and must not start with `#` or `!` or hold a line break, so it reads as
/// that one path. Fails, writing nothing, on such an entry, when the block
/// has no end, or when `root/.gitignore` is a link.
pub fn write_ignore(root: &Path, manifest: &str, entries: &[String]) -> Result<(), ChestError> {
    let path = &root.join(".gitignore");
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.is_symlink())
    {
        return Err(invalid(format!(
            "Refusing to write {} through a link",
            path.display()
        )));
    }
    let plain = |entry: &str| {
        let parts = Path::new(entry).components();
        !entry.is_empty()
            && !entry.starts_with(['#', '!'])
            && !entry.contains(['\n', '\r'])
            && parts
                .into_iter()
                .all(|part| matches!(part, Component::Normal(_)))
    };
    if let Some(entry) = entries.iter().find(|entry| !plain(entry)) {
        return Err(invalid(format!(
            "Refusing the .gitignore entry {entry:?}: it must be a plain path inside {}",
            root.display()
        )));
    }
    let begin = format!("# BEGIN chest ({manifest})");
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(io(path)(err)),
    };
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let is = |line: &str, marker: &str| line.trim_end_matches(['\n', '\r']) == marker;
    let (before, after) = match lines.iter().position(|line| is(line, &begin)) {
        Some(start) => {
            let end = lines[start..]
                .iter()
                .position(|line| is(line, "# END chest"))
                .ok_or_else(|| {
                    invalid(format!(
                        "{}: `{begin}` has no `# END chest`",
                        path.display()
                    ))
                })?;
            (&lines[..start], &lines[start + end + 1..])
        }
        None => (&lines[..], &[][..]),
    };
    let mut entries = entries.to_vec();
    entries.sort();
    entries.dedup();
    let mut out = before.concat();
    if !entries.is_empty() {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&begin);
        out.push('\n');
        for entry in &entries {
            out.push_str(entry);
            out.push('\n');
        }
        out.push_str("# END chest\n");
    }
    out.push_str(&after.concat());
    if out == text {
        return Ok(());
    }
    write_atomic(path, out.as_bytes(), 0o666).map_err(io(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("yapi-chest-place-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn placement_links_copies() {
        let dir = scratch("links");
        let root = dir.join("project");
        let outside = dir.join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        let source = dir.join("checkout/skills/pdf");
        std::fs::create_dir_all(source.join("scripts")).unwrap();
        std::fs::write(source.join("SKILL.md"), "# pdf\n").unwrap();
        std::fs::write(source.join("scripts/run.sh"), "echo\n").unwrap();
        std::fs::create_dir_all(source.join(".git")).unwrap();
        std::fs::write(source.join(".git/HEAD"), "x").unwrap();
        let folders = [".agents/skills", ".claude/skills", ".cursor/skills"].map(|f| root.join(f));
        // A hand-written skill beside the placed one stays.
        std::fs::create_dir_all(folders[1].join("mine")).unwrap();
        std::fs::write(folders[1].join("mine/SKILL.md"), "mine").unwrap();

        let placed = place(&source, &root, &folders[..2], "pdf").unwrap();
        assert_eq!(placed, [folders[0].join("pdf"), folders[1].join("pdf")]);
        let canonical = &placed[0];
        assert!(canonical.symlink_metadata().unwrap().is_dir());
        assert_eq!(
            std::fs::read_to_string(canonical.join("scripts/run.sh")).unwrap(),
            "echo\n"
        );
        assert!(!canonical.join(".git").exists());
        #[cfg(unix)]
        {
            let link = std::fs::read_link(&placed[1]).unwrap();
            assert_eq!(link, Path::new("../../.agents/skills/pdf"));
        }
        assert_eq!(
            std::fs::read_to_string(placed[1].join("SKILL.md")).unwrap(),
            "# pdf\n"
        );

        // A folder where links fail gets a copy, and placing again replaces.
        std::fs::write(source.join("SKILL.md"), "# pdf 2\n").unwrap();
        let failing = |_: &Path, _: &Path| Err(io::Error::other("no links here"));
        let pair = [folders[0].clone(), folders[2].clone()];
        let placed = place_with(&source, &root, &pair, "pdf", failing).unwrap();
        assert!(placed[1].symlink_metadata().unwrap().is_dir());
        assert_eq!(
            std::fs::read_to_string(placed[1].join("SKILL.md")).unwrap(),
            "# pdf 2\n"
        );
        // A copy that fails partway leaves nothing at the path or beside it.
        #[cfg(unix)]
        {
            let breaking = |target: &Path, path: &Path| {
                let canonical = path.parent().unwrap().join(target);
                std::os::unix::fs::symlink("SKILL.md", canonical.join("link"))?;
                Err(io::Error::other("no links here"))
            };
            assert!(place_with(&source, &root, &pair, "pdf", breaking).is_err());
            assert!(placed[1].symlink_metadata().is_err());
            assert!(folders[2].join(".pdf-staging").symlink_metadata().is_err());
            std::fs::remove_file(folders[0].join("pdf/link")).unwrap();
        }

        // Nothing goes outside the root: not through a folder that links out,
        // nor under a name that climbs, nor by removing what lies outside.
        #[cfg(unix)]
        {
            std::fs::create_dir_all(root.join(".evil")).unwrap();
            std::os::unix::fs::symlink(&outside, root.join(".evil/skills")).unwrap();
            std::os::unix::fs::symlink(dir.join("nowhere"), root.join(".dangling")).unwrap();
            for folder in [
                root.join(".evil/skills"),
                root.join(".evil/skills/deep"),
                root.join(".dangling/skills"),
            ] {
                let pair = [folders[0].clone(), folder];
                assert!(place(&source, &root, &pair, "evil").is_err(), "{pair:?}");
            }
            assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
            assert!(!dir.join("nowhere").exists());
            assert!(!folders[0].join("evil").exists());
            // A folder that links within the root is fine.
            std::fs::create_dir_all(root.join("shared/skills")).unwrap();
            std::os::unix::fs::symlink("shared", root.join(".codex")).unwrap();
            let pair = [folders[0].clone(), root.join(".codex/skills")];
            place(&source, &root, &pair, "pdf").unwrap();
            assert!(root.join("shared/skills/pdf/SKILL.md").exists());
        }
        std::fs::write(outside.join("keep"), "").unwrap();
        assert!(unplace(&root, &[outside.join("keep")]).is_err());
        assert!(unplace(&root, &[folders[0].join("../../outside/keep")]).is_err());
        assert!(outside.join("keep").exists());
        for name in ["../x", "Bad", ""] {
            assert!(
                place(&source, &root, &folders[..1], name).is_err(),
                "{name}"
            );
        }
        assert!(unplace(&root, &[folders[0].join("Bad")]).is_err());

        unplace(&root, &[folders[1].join("pdf"), folders[2].join("pdf")]).unwrap();
        assert!(folders[1].join("pdf").symlink_metadata().is_err());
        assert!(!folders[2].join("pdf").exists());
        // Unplacing the link left its target and the hand-written skill alone.
        assert!(folders[0].join("pdf/SKILL.md").exists());
        assert!(folders[1].join("mine/SKILL.md").exists());

        // The block is rebuilt in place; lines around it, CRLF ones too, stay.
        let ignore = root.join(".gitignore");
        let handwritten = "target/\r\n# mine\n# BEGIN chest (chest.toml)\nold/path\n# END chest\n.env\n# BEGIN chest (chest.dev.toml)\ndev/path\n# END chest\nlast";
        std::fs::write(&ignore, handwritten).unwrap();
        let entries = [
            ".claude/skills/pdf".to_owned(),
            ".agents/skills/pdf".into(),
            ".agents/skills/pdf".into(),
        ];
        write_ignore(&root, "chest.toml", &entries).unwrap();
        assert_eq!(
            std::fs::read_to_string(&ignore).unwrap(),
            "target/\r\n# mine\n# BEGIN chest (chest.toml)\n.agents/skills/pdf\n.claude/skills/pdf\n# END chest\n.env\n# BEGIN chest (chest.dev.toml)\ndev/path\n# END chest\nlast"
        );
        write_ignore(&root, "chest.toml", &[]).unwrap();
        assert_eq!(
            std::fs::read_to_string(&ignore).unwrap(),
            "target/\r\n# mine\n.env\n# BEGIN chest (chest.dev.toml)\ndev/path\n# END chest\nlast"
        );
        // A new block goes at the end, after a final line without a newline.
        write_ignore(&root, "chest.toml", &["a".into()]).unwrap();
        assert!(
            std::fs::read_to_string(&ignore)
                .unwrap()
                .ends_with("# END chest\nlast\n# BEGIN chest (chest.toml)\na\n# END chest\n")
        );
        // Entries are plain paths inside the root, one per line.
        let before = std::fs::read_to_string(&ignore).unwrap();
        for entry in [
            "../x", "/abs", "./a", "a/../b", "!keep", "# note", "a\nb", "",
        ] {
            assert!(
                write_ignore(&root, "chest.toml", &[entry.into()]).is_err(),
                "{entry:?}"
            );
        }
        assert_eq!(std::fs::read_to_string(&ignore).unwrap(), before);
        std::fs::write(&ignore, "# BEGIN chest (chest.toml)\nkept\n").unwrap();
        assert!(write_ignore(&root, "chest.toml", &[]).is_err());
        assert_eq!(
            std::fs::read_to_string(&ignore).unwrap(),
            "# BEGIN chest (chest.toml)\nkept\n"
        );
        #[cfg(unix)]
        {
            let linked = dir.join("linked");
            std::fs::create_dir_all(&linked).unwrap();
            std::os::unix::fs::symlink(outside.join("keep"), linked.join(".gitignore")).unwrap();
            assert!(write_ignore(&linked, "chest.toml", &["a".into()]).is_err());
            assert_eq!(std::fs::read_to_string(outside.join("keep")).unwrap(), "");
        }
        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        write_ignore(&empty, "chest.toml", &[]).unwrap();
        assert!(!empty.join(".gitignore").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
