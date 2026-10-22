// What pi's own modules, bundled in `yapi:vendor/pi-coding-agent`, take from
// yapi in place of pi's: yapi's name and directories (as pi's `piConfig`
// branding sets them), the session's theme, the host's frontmatter parser and
// image processing. The `@earendil-works/pi-coding-agent` facade shares them.
import { backgroundAnsi, foregroundAnsi, parseColor } from "@earendil-works/pi-tui";

const yapi = globalThis.__yapi;
const { Buffer } = globalThis.__yapi_builtins.buffer;

// ----- configuration ------------------------------------------------------
/** yapi keeps project files in `.yapi/` and global ones in `~/.yapi/agent`. */
export const APP_NAME = "yapi";
export const CONFIG_DIR_NAME = ".yapi";
/** The pi extension API version yapi implements. */
export const VERSION = "1.0.0";
export const ENV_AGENT_DIR = "YAPI_CODING_AGENT_DIR";
export const ENV_SESSION_DIR = "YAPI_CODING_AGENT_SESSION_DIR";

export function getAgentDir() {
	return yapi.request("agentDir");
}
export function getPackageDir() {
	return getAgentDir();
}
export function getDocsPath() {
	return `${getAgentDir()}/docs`;
}
export function getExamplesPath() {
	return `${getAgentDir()}/examples`;
}
export function getReadmePath() {
	return `${getAgentDir()}/README.md`;
}
export function getSessionsDir() {
	return `${getAgentDir()}/sessions`;
}
export function getBinDir() {
	return `${getAgentDir()}/bin`;
}

// ----- frontmatter ------------------------------------------------------------
export function parseFrontmatter(content) {
	return yapi.request("frontmatter.parse", { content: String(content) });
}
export const stripFrontmatter = (content) => parseFrontmatter(content).body;

// ----- theme -----------------------------------------------------------------------
export const Theme = yapi.Theme;
// pi's `Theme` constructor turns colors into escape sequences as pi-tui does.
yapi.colorAnsi = (value, mode, background) => (background ? backgroundAnsi : foregroundAnsi)(parseColor(value), mode);
export const theme = globalThis.__yapi_theme;
export const SYSTEM_THEME_NAME = "system";
export function initTheme() {}
/** The names of the themes the session offers. */
export function getAvailableThemes() {
	try {
		return (yapi.request("ui.getAllThemes", {}) ?? []).map((info) => info.name);
	} catch {
		return [];
	}
}
export function parseAutoThemeSetting(themeSetting) {
	if (!themeSetting) return undefined;
	const slashIndex = themeSetting.indexOf("/");
	if (slashIndex === -1 || themeSetting.indexOf("/", slashIndex + 1) !== -1) return undefined;
	const lightTheme = themeSetting.slice(0, slashIndex).trim();
	const darkTheme = themeSetting.slice(slashIndex + 1).trim();
	if (!lightTheme || !darkTheme) return undefined;
	return { lightTheme, darkTheme };
}
export function getLanguageFromPath(path) {
	return yapi.request("language.fromPath", { path: String(path) }) ?? undefined;
}
/**
 * The lines the first code renderer in this runtime draws for `code`, as yapi's
 * transcript asks them; `undefined` without one. A renderer that throws is passed over.
 */
function renderCode(code, lang) {
	for (const extension of yapi.extensions.values()) {
		if (typeof extension.codeRenderer !== "function") continue;
		try {
			const lines = extension.codeRenderer(code, lang, { width: process.stdout.columns || 80 }, theme);
			if (Array.isArray(lines)) return lines.map(String);
		} catch {}
	}
	return undefined;
}
/** pi's `highlightCode`: the code renderers' lines, else the code in the code block color. */
export function highlightCode(code, lang) {
	return renderCode(String(code), lang) ?? String(code).split("\n").map((line) => theme.fg("mdCodeBlock", line));
}
export function getMarkdownTheme() {
	return {
		heading: (text) => theme.fg("mdHeading", text),
		link: (text) => theme.fg("mdLink", text),
		linkUrl: (text) => theme.fg("mdLinkUrl", text),
		code: (text) => theme.fg("mdCode", text),
		codeBlock: (text) => theme.fg("mdCodeBlock", text),
		codeBlockBorder: (text) => theme.fg("mdCodeBlockBorder", text),
		quote: (text) => theme.fg("mdQuote", text),
		quoteBorder: (text) => theme.fg("mdQuoteBorder", text),
		hr: (text) => theme.fg("mdHr", text),
		listBullet: (text) => theme.fg("mdListBullet", text),
		bold: (text) => theme.bold(text),
		italic: (text) => theme.italic(text),
		underline: (text) => theme.underline(text),
		strikethrough: (text) => theme.strikethrough(text),
		highlightCode,
	};
}
export function getSelectListTheme() {
	return {
		selectedPrefix: (text) => theme.fg("accent", text),
		selectedText: (text) => theme.fg("accent", text),
		description: (text) => theme.fg("muted", text),
		scrollInfo: (text) => theme.fg("muted", text),
		noMatch: (text) => theme.fg("muted", text),
	};
}
export function getEditorTheme() {
	return { borderColor: (text) => theme.fg("borderMuted", text), selectList: getSelectListTheme() };
}
export function getSettingsListTheme() {
	return {
		label: (text, selected) => (selected ? theme.fg("accent", text) : text),
		value: (text, selected) => (selected ? theme.fg("accent", text) : theme.fg("muted", text)),
		description: (text) => theme.fg("dim", text),
		get cursor() {
			return theme.fg("accent", "→ ");
		},
		hint: (text) => theme.fg("dim", text),
	};
}

// ----- images, which the host decodes and encodes ------------------------------------------------
/** pi's `resizeImage`: the image fitted to the limits, or `null` when it does not decode or fit. */
export async function resizeImage(inputBytes, mimeType, options) {
	return yapi.request("image.resize", { data: Buffer.from(inputBytes).toString("base64"), mimeType, options: options ?? {} });
}
export function formatDimensionNote(result) {
	if (!result.wasResized) return undefined;
	const scale = result.originalWidth / result.width;
	return `[Image: original ${result.originalWidth}x${result.originalHeight}, displayed at ${result.width}x${result.height}. Multiply coordinates by ${scale.toFixed(2)} to map to original image.]`;
}
/** pi's `convertImageBytesToPng`: the image upright as PNG, or `null` when it does not decode. */
export async function convertImageBytesToPng(bytes) {
	const data = yapi.request("image.png", { data: Buffer.from(bytes).toString("base64") });
	return data === null ? null : new Uint8Array(Buffer.from(data, "base64"));
}
export async function convertToPng(base64Data, mimeType) {
	if (mimeType === "image/png") return { data: base64Data, mimeType };
	const pngBytes = await convertImageBytesToPng(new Uint8Array(Buffer.from(base64Data, "base64")));
	if (!pngBytes) return null;
	return { data: Buffer.from(pngBytes).toString("base64"), mimeType: "image/png" };
}
