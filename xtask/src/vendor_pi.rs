//! `cargo xtask vendor-pi`: bundle the npm packages pi extensions import and
//! yapi provides, `typebox`, `@earendil-works/pi-tui`,
//! `@earendil-works/pi-agent-core` and the parts of
//! `@earendil-works/pi-coding-agent` its facade serves, into
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
/// Module name in the runtime, and its source.
const ENTRIES: [(&str, &str); 6] = [
    ("typebox", "export * from 'typebox';"),
    ("typebox-value", "export * from 'typebox/value';"),
    ("typebox-compile", "export * from 'typebox/compile';"),
    ("pi-tui", "export * from '@earendil-works/pi-tui';"),
    (
        "pi-agent-core",
        "export * from '@earendil-works/pi-agent-core';",
    ),
    ("pi-coding-agent", PI_CODING_AGENT),
];
/// The pi-coding-agent exports its facade takes from pi, from the copy of
/// pi's `dist` that [`OVERRIDES`] patches.
const PI_CODING_AGENT: &str = "\
export { buildContextEntries, buildSessionContext, buildSessionProjection, migrateSessionEntries, parseSessionEntries, sessionEntryToContextMessages, SessionManager } from '../dist/core/session-manager.js';
export { calculateContextTokens, collectEntriesForBranchSummary, compact, estimateTokens, findCutPoint, findTurnStartIndex, generateBranchSummary, generateSummary, generateSummaryWithUsage, getLastAssistantUsage, prepareBranchEntries, shouldCompact } from '../dist/core/compaction/index.js';
export { formatSkillsForPrompt, loadSkills, loadSkillsFromDir } from '../dist/core/skills.js';
export { generateDiffString, generateUnifiedPatch } from '../dist/core/tools/edit-diff.js';
export { resolveCliModel, resolveModelScopeWithDiagnostics } from '../dist/core/model-resolver.js';
export { parseArgs } from '../dist/cli/args.js';
export { readStoredCredential } from '../dist/core/auth-storage.js';
export { hasTrustRequiringProjectResources, ProjectTrustStore } from '../dist/core/trust-manager.js';
export { SettingsManager } from '../dist/core/settings-manager.js';
export { wrapRegisteredTool, wrapRegisteredTools } from '../dist/core/extensions/wrapper.js';
export { getPowerShellConfig } from '../dist/utils/shell.js';
export { detectSupportedImageMimeTypeFromFile } from '../dist/utils/mime.js';
export { createToolSearchExtension } from '../dist/extensions/tool-search/index.js';
export * from '../dist/core/tools/index.js';
export { ArminComponent, AssistantMessageComponent, BashExecutionComponent, BranchSummaryMessageComponent, CompactionSummaryMessageComponent, CustomMessageComponent, ExtensionEditorComponent, ExtensionInputComponent, ExtensionSelectorComponent, FooterComponent, LoginDialogComponent, ModelSelectorComponent, OAuthSelectorComponent, renderDiff, SessionSelectorComponent, SettingsSelectorComponent, ShowImagesSelectorComponent, SkillInvocationMessageComponent, ThemeSelectorComponent, ThinkingSelectorComponent, ToolExecutionComponent, TreeSelectorComponent, UserMessageComponent, UserMessageSelectorComponent } from '../dist/modes/interactive/components/index.js';
";
/// pi's installed package, whose `dist` the bundle copies.
const PI_PACKAGE: &str = "tests/fixtures/pi/generator/node_modules/@earendil-works/pi-coding-agent";
/// pi modules replaced in the copy by yapi's: its configuration (yapi's
/// directories and names), theme, frontmatter parser and image processing,
/// which the runtime's `yapi:pi/host` provides.
const OVERRIDES: [&str; 5] = [
    "config.js",
    "modes/interactive/theme/theme.js",
    "utils/frontmatter.js",
    "utils/image-convert.js",
    "utils/image-resize.js",
];
/// `undici`, which pi's HTTP dispatcher setup reads and the bundle leaves
/// out: pi reaches that code only from its own modes, and yapi's host makes
/// the HTTP requests. Each export throws, so a pi release that reached it
/// would fail visibly.
const UNDICI: &str = "\
const unavailable = (name) => function () {
\tthrow new Error(`undici.${name} is not available in yapi extensions; yapi's host makes HTTP requests`);
};
export const Client = /* @__PURE__ */ unavailable('Client');
export const EnvHttpProxyAgent = /* @__PURE__ */ unavailable('EnvHttpProxyAgent');
export const install = /* @__PURE__ */ unavailable('install');
export const Pool = /* @__PURE__ */ unavailable('Pool');
export const setGlobalDispatcher = /* @__PURE__ */ unavailable('setGlobalDispatcher');
";
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
const BARE_BUILTINS: [&str; 17] = [
    "assert",
    "child_process",
    "constants",
    "crypto",
    "events",
    "fs",
    "fs/promises",
    "module",
    "os",
    "path",
    "readline",
    "stream",
    "stream/promises",
    "string_decoder",
    "timers/promises",
    "url",
    "util",
];

/// Copies the JavaScript files of directory `from` into `to`.
fn copy_dir(from: &Path, to: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let path = entry?.path();
        let target = to.join(path.file_name().unwrap_or_default());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else if path.extension().is_some_and(|ext| ext == "js") {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

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
    // A copy of pi's `dist` with yapi's modules in place of the overridden ones.
    copy_dir(&Path::new(PI_PACKAGE).join("dist"), &scratch.join("dist"))?;
    for file in OVERRIDES {
        fs::write(
            scratch.join("dist").join(file),
            "export * from 'yapi:pi/host';\n",
        )?;
    }
    let undici = scratch.join("undici.mjs");
    fs::write(&undici, UNDICI)?;
    let entries_dir = scratch.join("entries");
    fs::create_dir_all(&entries_dir)?;
    let mut entry_paths: Vec<PathBuf> = Vec::new();
    for (name, source) in ENTRIES {
        let path = entries_dir.join(format!("{name}.mjs"));
        fs::write(&path, format!("{source}\n"))?;
        entry_paths.push(path);
    }
    let out = scratch.join("out");
    let mut command = Command::new("npx");
    command
        .args(["--yes", ESBUILD])
        .args(&entry_paths)
        .args([
            "--bundle",
            "--splitting",
            "--format=esm",
            "--platform=neutral",
            "--main-fields=module,main",
            "--conditions=import,default",
            "--external:node:*",
            // The runtime's own modules, and pi-ai, which its facade serves.
            "--external:yapi:*",
            "--external:@earendil-works/pi-ai",
            // The CommonJS packages require Node built-ins at run time.
            "--banner:js=const require = globalThis.__yapi_require_for('yapi:vendor');",
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
    command.arg(format!("--alias:undici=./{}", undici.display()));
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
