// Records pi-tui's inline images, for crates/yapi-tui/tests/image.rs: the cell sizes
// `calculateImageCellSize` picks, the rows the `Image` component renders in Kitty and
// iTerm2, and the Kitty crops and placement-only commands fullscreen mode draws.
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(here, "node_modules/@earendil-works/pi-coding-agent/package.json"));
const dist = pathToFileURL(require.resolve("@earendil-works/pi-tui")).href.replace(/index\.js$/, "");
const { calculateImageCellSize, cropKittyImageLine, getKittyImagePlacement, setCapabilities, setCellDimensions } =
	await import(`${dist}terminal-image.js`);
const { Image } = await import(`${dist}components/image.js`);

const cellSizes = [
	[[800, 600], 60, undefined, [9, 18], false],
	[[800, 600], 60, 30, [9, 18], true],
	[[100, 100], 60, 30, [9, 18], true],
	[[20, 20], 2, 2, [10, 10], true],
	[[3000, 100], 40, 20, [8, 16], true],
	[[100, 3000], 40, 20, [8, 16], true],
	[[1, 1], 10, 5, [9, 18], false],
	[[0, 0], 10, 5, [9, 18], true],
	[[1920, 1080], 120, 60, [7, 15], true],
].map(([[widthPx, heightPx], maxWidth, maxHeight, [cellWidth, cellHeight], optimize]) => ({
	image: [widthPx, heightPx],
	maxWidth,
	maxHeight: maxHeight ?? null,
	cell: [cellWidth, cellHeight],
	optimize,
	size: calculateImageCellSize({ widthPx, heightPx }, maxWidth, maxHeight, { widthPx: cellWidth, heightPx: cellHeight }, optimize),
}));

// A PNG header for the size, padded to `bytes`, as base64.
const png = (width, height, bytes) => {
	const buffer = Buffer.alloc(bytes);
	Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]).copy(buffer);
	buffer.writeUInt32BE(width, 16);
	buffer.writeUInt32BE(height, 20);
	return buffer.toString("base64");
};
const images = [
	{ size: [800, 600], bytes: 24, width: 80, maxWidthCells: 60, cell: [9, 18] },
	{ size: [800, 600], bytes: 26, width: 30, maxWidthCells: 60, cell: [9, 18] },
	{ size: [64, 64], bytes: 25, width: 80, maxWidthCells: 120, cell: [10, 20] },
	{ size: [2000, 500], bytes: 3200, width: 100, maxWidthCells: 80, cell: [9, 18] },
	{ size: [10, 400], bytes: 24, width: 2, maxWidthCells: 60, cell: [9, 18] },
];
// Each image has its own Kitty id, so the crops below find each one's metadata.
const render = (protocol) =>
	images.map(({ size, bytes, width, maxWidthCells, cell }, index) => {
		const data = png(size[0], size[1], bytes);
		const imageId = 42 + index;
		setCapabilities({ images: protocol, trueColor: true, hyperlinks: false });
		setCellDimensions({ widthPx: cell[0], heightPx: cell[1] });
		const image = new Image(data, "image/png", { fallbackColor: (text) => text }, { maxWidthCells, imageId });
		return { data, size, width, maxWidthCells, cell, imageId, lines: image.render(width) };
	});
const kitty = render("kitty");
const iterm2 = render("iterm2");

// Crops and placements of the Kitty rows above, which registered their metadata.
const crops = [];
const placements = [];
for (const { lines } of kitty) {
	const rows = lines.length;
	for (const [hidden, visible] of [[0, rows], [0, 1], [1, rows], [rows - 1, 1], [1, 1], [rows, 1], [0, 0]]) {
		crops.push({ line: lines[0], hidden, visible, cropped: cropKittyImageLine(lines[0], hidden, visible) });
	}
	const placement = getKittyImagePlacement(lines[0]);
	placements.push({ line: lines[0], sequence: placement?.sequence ?? null, replacement: placement?.replacementLine ?? null });
}

mkdirSync(join(here, "../image"), { recursive: true });
writeFileSync(join(here, "../image/image.json"), `${JSON.stringify({ cellSizes, kitty, iterm2, crops, placements }, null, "\t")}\n`);
