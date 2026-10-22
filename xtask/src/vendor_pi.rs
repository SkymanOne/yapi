//! `cargo xtask vendor-pi`: bundle the npm packages pi extensions import and
//! yapi provides, `typebox`, `@earendil-works/pi-tui` and the parts of
//! `@earendil-works/pi-ai` that `xtask/vendor-pi/pi-ai/entry.mjs` names, into
//! `guest/yapi-js/js/vendor`. The bundles share chunks, so each package is
//! included once. Also copies pi's HTML export template, unchanged, into
//! `crates/yapi/assets/export-html`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

/// The pinned esbuild release.
const ESBUILD: &str = "esbuild@0.25.10";
const VENDOR: &str = "guest/yapi-js/js/vendor";
/// Where the fixture generator's `npm ci` installs pi and its dependencies.
const NODE_MODULES: &str =
    "tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/node_modules";
/// Module name in the runtime, and what it re-exports.
const ENTRIES: [(&str, &str); 4] = [
    ("typebox", "typebox"),
    ("typebox-value", "typebox/value"),
    ("typebox-compile", "typebox/compile"),
    ("pi-tui", "@earendil-works/pi-tui"),
];
/// pi-ai's wire API modules, which yapi's host implements, with their exports
/// other than `stream` and `streamSimple`. In the bundle, the host streams
/// each API and the other exports stay pi's, so no provider SDK is bundled.
const HOST_APIS: [(&str, &str); 10] = [
    ("anthropic-messages", ""),
    ("azure-openai-responses", ""),
    ("bedrock-converse-stream", ""),
    ("google-generative-ai", ""),
    ("google-vertex", ""),
    ("mistral-conversations", ""),
    ("openai-codex-responses", ""),
    ("openai-completions", "convertMessages"),
    ("openai-responses", ""),
    ("pi-messages", "PiMessagesResponseError"),
];
/// The pi-ai entry, the files that replace pi-ai's, and the SDK stand-ins.
const PI_AI_SOURCES: &str = "xtask/vendor-pi/pi-ai";

/// pi's HTML export template in the installed package, and where yapi keeps it.
const EXPORT_TEMPLATE: &str = "tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent/dist/core/export-html";
const EXPORT_ASSETS: &str = "crates/yapi/assets/export-html";
const EXPORT_FILES: [&str; 5] = [
    "template.html",
    "template.css",
    "template.js",
    "vendor/marked.min.js",
    "vendor/highlight.min.js",
];

/// Node built-ins the bundles import by their bare names.
const BARE_BUILTINS: [&str; 7] = ["events", "fs", "path", "os", "child_process", "util", "url"];

/// Regenerate the vendored bundles, then rebuild the runtime with
/// `cargo xtask js-runtime`. Install the sources first with `npm ci` in
/// `tests/fixtures/pi/generator`; needs Node and network access for `npx`.
#[derive(clap::Args)]
pub struct Args {}

pub fn run(_args: Args) -> anyhow::Result<()> {
    let modules = Path::new(NODE_MODULES).canonicalize().with_context(|| {
        format!("{NODE_MODULES} is missing; run npm ci in tests/fixtures/pi/generator")
    })?;
    // A fixed path: esbuild's output names its inputs, so this keeps it stable.
    let scratch = Path::new("target/vendor-pi").to_path_buf();
    let _ = fs::remove_dir_all(&scratch);
    let entries_dir = scratch.join("entries");
    fs::create_dir_all(&entries_dir)?;
    let mut entry_paths: Vec<PathBuf> = Vec::new();
    for (name, module) in ENTRIES {
        let path = entries_dir.join(format!("{name}.mjs"));
        fs::write(&path, format!("export * from '{module}';\n"))?;
        entry_paths.push(path);
    }
    let pi_ai = scratch.join("pi-ai");
    prepare_pi_ai(&modules.join("@earendil-works/pi-ai"), &pi_ai)?;
    let out = scratch.join("out");
    let mut command = Command::new("npx");
    command
        .args(["--yes", ESBUILD])
        .args(&entry_paths)
        .arg(format!("pi-ai={}", pi_ai.join("entry.mjs").display()))
        .arg(format!(
            "pi-ai-subpaths={}",
            pi_ai.join("subpaths.mjs").display()
        ))
        .arg(format!("--alias:openai=./{PI_AI_SOURCES}/sdk/openai.mjs"))
        .arg(format!(
            "--alias:@google/genai=./{PI_AI_SOURCES}/sdk/google-genai.mjs"
        ))
        .args([
            "--bundle",
            "--splitting",
            "--format=esm",
            "--platform=neutral",
            "--main-fields=module,main",
            "--conditions=import,default",
            "--external:node:*",
            "--legal-comments=inline",
            "--target=es2022",
            "--out-extension:.js=.mjs",
            "--chunk-names=chunk-[hash]",
            "--log-level=warning",
        ])
        .arg(format!("--outdir={}", out.display()))
        .env("NODE_PATH", &modules);
    for builtin in BARE_BUILTINS {
        command.arg(format!("--external:{builtin}"));
    }
    let status = command.status().context("running esbuild through npx")?;
    if !status.success() {
        bail!("esbuild failed");
    }
    // Replace the bundles; the license notes stay.
    for entry in fs::read_dir(VENDOR)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "mjs") {
            fs::remove_file(path)?;
        }
    }
    let mut written = 0;
    for entry in fs::read_dir(&out)? {
        let path = entry?.path();
        if let Some(name) = path.file_name() {
            fs::copy(&path, Path::new(VENDOR).join(name))?;
            written += 1;
        }
    }
    let _ = fs::remove_dir_all(&scratch);
    for file in EXPORT_FILES {
        let target = Path::new(EXPORT_ASSETS).join(file);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(Path::new(EXPORT_TEMPLATE).join(file), &target)
            .with_context(|| format!("copying the export template's {file}"))?;
    }
    eprintln!(
        "wrote {written} modules to {VENDOR} and the export template to {EXPORT_ASSETS}; now run cargo xtask js-runtime"
    );
    Ok(())
}

/// Copies pi-ai's `package.json` and `dist` from `package` to `copy` with
/// yapi's replacements, next to the entries in `PI_AI_SOURCES`: the files
/// under `PI_AI_SOURCES/dist` replace pi's, which stay beside them as
/// `*.pi.js`; the wire API modules stream through the host; the catalog data
/// is empty, as the generated catalogs read the host's.
fn prepare_pi_ai(package: &Path, copy: &Path) -> anyhow::Result<()> {
    copy_dir(&package.join("dist"), &copy.join("dist"))?;
    fs::copy(package.join("package.json"), copy.join("package.json"))?;
    let dist = copy.join("dist");
    let replace = |relative: &Path, text: &str| -> anyhow::Result<()> {
        let target = dist.join(relative);
        if target.exists() {
            fs::rename(&target, target.with_extension("pi.js"))?;
        }
        fs::write(&target, text).with_context(|| format!("replacing {}", target.display()))
    };
    let overlay = Path::new(PI_AI_SOURCES).join("dist");
    for path in files(&overlay)? {
        replace(path.strip_prefix(&overlay)?, &fs::read_to_string(&path)?)?;
    }
    for (api, others) in HOST_APIS {
        let mut text = format!(
            "// yapi: the host's implementation of `{api}`.\n\
             const host = () => globalThis.__yapi.hostApi(\"{api}\");\n\
             export const stream = (model, context, options) => host().stream(model, context, options);\n\
             export const streamSimple = (model, context, options) => host().streamSimple(model, context, options);\n"
        );
        if !others.is_empty() {
            text.push_str(&format!("export {{ {others} }} from \"./{api}.pi.js\";\n"));
        }
        replace(Path::new(&format!("api/{api}.js")), &text)?;
    }
    for path in files(&dist.join("providers/data"))? {
        if !path.ends_with(".manifest.json") {
            fs::write(path, "{}")?;
        }
    }
    let mut providers = String::new();
    for path in files(&dist.join("providers"))? {
        let relative = path
            .strip_prefix(copy)?
            .to_string_lossy()
            .replace('\\', "/");
        if relative.ends_with(".js")
            && !relative.contains("/data/")
            && !relative.contains("/images/")
        {
            providers.push_str(&format!("export * from \"./{relative}\";\n"));
        }
    }
    fs::write(copy.join("providers.mjs"), providers)?;
    for entry in ["entry.mjs", "subpaths.mjs"] {
        fs::copy(Path::new(PI_AI_SOURCES).join(entry), copy.join(entry))?;
    }
    Ok(())
}

/// The files under `dir`, recursively, in path order.
fn files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

fn copy_dir(from: &Path, to: &Path) -> anyhow::Result<()> {
    for path in files(from)? {
        let target = to.join(path.strip_prefix(from)?);
        if let Some(dir) = target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(&path, target)?;
    }
    Ok(())
}
