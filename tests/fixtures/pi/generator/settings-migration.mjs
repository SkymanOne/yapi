// Writes ../settings/migrations.json: settings files in pi's legacy forms, and
// each file as pi's SettingsManager writes it back after one unrelated
// setting changes. Usage: node settings-migration.mjs   (Node >= 22.19)

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { SettingsManager } from "@earendil-works/pi-coding-agent";

const here = dirname(fileURLToPath(import.meta.url));
const inputs = {
	"queue-mode": { queueMode: "all", theme: "dark" },
	"queue-mode-kept": { queueMode: "all", steeringMode: "one-at-a-time" },
	websockets: { websockets: true, theme: "dark" },
	"websockets-kept": { websockets: false, transport: "auto" },
	"skills-object": { skills: { enableSkillCommands: false, customDirectories: ["~/skills", "./more"] }, theme: "dark" },
	"skills-object-empty": { theme: "dark", skills: { customDirectories: [] } },
	"skills-object-kept": { enableSkillCommands: true, skills: { enableSkillCommands: false } },
	"retry-max-delay": { retry: { enabled: true, maxDelayMs: 30000 }, theme: "dark" },
	"retry-max-delay-provider": { retry: { maxDelayMs: 30000, provider: { maxRetries: 2 }, baseDelayMs: 10 } },
	"retry-max-delay-kept": { retry: { maxDelayMs: 30000, provider: { maxRetryDelayMs: 5000 } } },
	"retry-max-delay-null": { retry: { maxDelayMs: 30000, provider: { maxRetryDelayMs: null, timeoutMs: 1 } } },
	"retry-max-delay-not-number": { retry: { maxDelayMs: "30000" } },
	all: {
		queueMode: "one-at-a-time",
		websockets: false,
		skills: { customDirectories: ["./skills"], enableSkillCommands: true },
		retry: { maxDelayMs: 1000 },
	},
};

const cases = [];
for (const [name, settings] of Object.entries(inputs)) {
	const dir = mkdtempSync(join(tmpdir(), "pi-settings-migration-"));
	const input = `${JSON.stringify(settings, null, 2)}\n`;
	writeFileSync(join(dir, "settings.json"), input);
	const manager = SettingsManager.create(join(dir, "cwd"), dir);
	manager.setLastChangelogVersion("1.0.0");
	await manager.flush();
	cases.push({ name, input, output: readFileSync(join(dir, "settings.json"), "utf8") });
	rmSync(dir, { recursive: true, force: true });
}

const out = join(here, "../settings/migrations.json");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(cases, null, "\t")}\n`);
console.log(`wrote ${cases.length} cases to ${out}`);
