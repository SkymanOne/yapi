//! `chest check`: compares the capabilities a package's native extensions
//! use with those it requests. `chest add` shows the same comparison.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::bail;
use yapi_core::chest::manifest::{PackageMeta, package_meta};
use yapi_core::chest::wasm::{Capabilities, capabilities};
use yapi_core::extensions::Grants;
use yapi_core::packages::resolve::{ResourceType, is_extension_file, package_resources};
use yapi_core::tools::path::relative;

use crate::say;

/// The extensions of a package item.
pub struct Inspected {
    /// Each `.wasm` extension, by path relative to the item, with what it
    /// uses or why it could not be read.
    pub components: Vec<(String, Result<Capabilities, String>)>,
    /// The JS and TS extensions, which yapi checks only as they run.
    pub scripts: Vec<String>,
}

impl Inspected {
    /// The capabilities any component uses, each once.
    pub fn used(&self) -> Vec<&'static str> {
        let mut used = Vec::new();
        for (_, found) in &self.components {
            for capability in found.iter().flat_map(|found| &found.used) {
                if !used.contains(&capability.capability) {
                    used.push(capability.capability);
                }
            }
        }
        used
    }
}

/// Why WASI import `import`, one of [`Capabilities::unsupported`], of
/// extension `file` fails in yapi.
pub fn unsupported_import(file: &str, import: &str) -> String {
    format!(
        "{file} imports `{import}`, which yapi does not support. Network access goes through the SDK's `fetch`."
    )
}

/// What line `line`, one of [`Capabilities::unknown`], of extension `file`
/// means for its checks.
pub fn unknown_capability(file: &str, line: &str) -> String {
    format!(
        "{file} declares the capability `{line}`, which this version of chest does not know, so it is not checked"
    )
}

/// How many lines of one of a component's lists chest prints.
const SHOWN: usize = 10;

/// `line` of the first ten of `items` after `prefix`, then one line
/// counting the rest as `rest`, so that a component cannot flood the
/// terminal with lists it controls.
pub fn capped(
    prefix: &str,
    items: &[String],
    rest: &str,
    line: impl Fn(&str) -> String,
) -> Vec<String> {
    let mut lines: Vec<String> = items
        .iter()
        .take(SHOWN)
        .map(|item| format!("{prefix}{}", line(item)))
        .collect();
    if items.len() > SHOWN {
        lines.push(format!("{prefix}and {} more {rest}", items.len() - SHOWN));
    }
    lines
}

/// The extensions of the package or extension file at `item`, as yapi
/// resolves them when it loads `item` with `-e`.
pub fn inspect(item: &Path) -> Inspected {
    let mut inspected = Inspected {
        components: Vec::new(),
        scripts: Vec::new(),
    };
    let base = if item.is_file() {
        item.parent().map_or_else(PathBuf::new, Path::to_path_buf)
    } else {
        item.to_path_buf()
    };
    let resources = package_resources(item, None, true);
    for info in resources.enabled(ResourceType::Extensions) {
        let path = Path::new(&info.path);
        let shown = relative(&base, path);
        if path
            .extension()
            .is_some_and(|extension| extension == "wasm")
        {
            let found = std::fs::read(path)
                .map_err(|err| err.to_string())
                .and_then(|bytes| capabilities(&bytes).map_err(|err| err.to_string()));
            inspected.components.push((shown, found));
        } else {
            inspected.scripts.push(shown);
        }
    }
    inspected
}

/// Fails unless `item`, which the error calls `shown`, is a folder or a
/// file yapi loads as an extension.
pub fn extension_file(item: &Path, shown: &str) -> anyhow::Result<()> {
    let name = item.file_name().unwrap_or_default().to_string_lossy();
    if item.is_file() && !is_extension_file(&name) {
        bail!("{shown} is not an extension: extension files end in `.ts`, `.js` or `.wasm`");
    }
    Ok(())
}

/// Whether `dir` holds a `chest.toml` or `package.json`.
fn has_manifest(dir: &Path) -> bool {
    dir.join("chest.toml").is_file() || dir.join("package.json").is_file()
}

/// The folder of the package that holds `item`: `item` itself when it is a
/// folder, else the nearest folder above it with a `chest.toml` or
/// `package.json`.
fn package_root(item: &Path) -> Option<&Path> {
    if item.is_dir() {
        return Some(item);
    }
    item.ancestors().skip(1).find(|dir| has_manifest(dir))
}

/// Runs `chest check` on the package or extension file at `path`: an
/// error for each capability an extension uses that the package does not
/// request and for each WASI import yapi does not support, a warning for
/// each capability requested that no extension uses.
pub fn run(path: &Path) -> anyhow::Result<ExitCode> {
    let mut item = std::path::absolute(path)?;
    if !item.exists() {
        bail!("{} does not exist", path.display());
    }
    // A manifest stands for its package, not an extension file.
    if item.ends_with("package.json") || item.ends_with("chest.toml") {
        item.pop();
    }
    extension_file(&item, &path.display().to_string())?;
    let meta = match package_root(&item) {
        Some(root) => package_meta(root)?,
        None => PackageMeta::default(),
    };
    let requested = Grants::parse(&meta.capabilities)
        .map_err(|err| anyhow::anyhow!("the package's `capabilities`: {err}"))?;
    let inspected = inspect(&item);
    if inspected.components.is_empty() && inspected.scripts.is_empty() {
        if !package_root(&item).is_some_and(has_manifest) {
            bail!(
                "{} has no chest.toml or package.json, and no extensions to check",
                item.display()
            );
        }
        bail!(
            "{} provides no extensions. Build the ones its manifest lists first.",
            path.display()
        );
    }
    let mut errors = 0;
    let mut unrequested = false;
    for (file, found) in &inspected.components {
        let found = match found {
            Ok(found) => found,
            Err(err) => {
                say(&format!("error: {file}: {err}"));
                errors += 1;
                continue;
            }
        };
        if !found.metadata {
            say(&format!(
                "warning: {file} was built without capability metadata, so only its WASI imports were checked. Build it with yapi-extension-api to record its features."
            ));
        }
        for used in &found.used {
            if requested.covers(used.capability) {
                say(&format!(
                    "{file} uses `{}` ({})",
                    used.capability, used.source
                ));
            } else {
                say(&format!(
                    "error: {file} uses `{}` ({}), which the package does not request",
                    used.capability, used.source
                ));
                errors += 1;
                unrequested = true;
            }
        }
        let rest = format!("unsupported imports in {file}");
        let unsupported = capped("error: ", &found.unsupported, &rest, |import| {
            unsupported_import(file, import)
        });
        errors += found.unsupported.len();
        let rest = format!("unknown capability lines in {file}");
        let unknown = capped("warning: ", &found.unknown, &rest, |line| {
            unknown_capability(file, line)
        });
        unsupported
            .iter()
            .chain(&unknown)
            .for_each(|line| say(line));
    }
    for script in &inspected.scripts {
        say(&format!(
            "note: {script} is a JS or TS extension, which yapi checks only as it runs"
        ));
    }
    // A script may use what no component does, and `all` uses everything.
    let used = inspected.used();
    if inspected.scripts.is_empty() && !used.contains(&"all") {
        let unused = requested
            .names()
            .into_iter()
            .filter(|capability| !used.contains(capability));
        for capability in unused {
            say(&format!(
                "warning: the package requests `{capability}`, which no extension uses"
            ));
        }
    }
    if unrequested {
        say(
            "Add what the extensions use to `capabilities` in chest.toml's [package], or package.json's `yapi.capabilities`.",
        );
    }
    if errors > 0 {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}
