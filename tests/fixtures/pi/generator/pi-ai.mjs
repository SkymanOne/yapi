// Writes ../pi-ai/exports.json: what ../pi-ai/probe.js, an extension that calls
// pi-ai's public exports, reports when it runs on pi's pi-ai. No request
// leaves the process. Usage: node pi-ai.mjs   (Node >= 22.19)

import { copyFileSync, mkdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const scratch = "/tmp/yapi-pi-ai-probe";
rmSync(scratch, { recursive: true, force: true });
mkdirSync(join(scratch, "node_modules/@earendil-works"), { recursive: true });
symlinkSync(
	join(here, "node_modules/@earendil-works/pi-coding-agent/node_modules/@earendil-works/pi-ai"),
	join(scratch, "node_modules/@earendil-works/pi-ai"),
);
copyFileSync(join(here, "../pi-ai/probe.js"), join(scratch, "probe.mjs"));

const probe = (await import(pathToFileURL(join(scratch, "probe.mjs")).href)).default;
let description;
await probe({ registerCommand: (_name, command) => (description = command.description) });
writeFileSync(join(here, "../pi-ai/exports.json"), `${JSON.stringify(JSON.parse(description), null, "\t")}\n`);
