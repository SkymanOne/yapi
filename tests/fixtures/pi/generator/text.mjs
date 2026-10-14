// Records pi-tui's text layout, for crates/yapi-tui/tests/text.rs: wrapping,
// truncation and markdown rendering, as plain text (identity theme), with
// links as text or as OSC 8 hyperlinks.
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(here, "node_modules/@earendil-works/pi-coding-agent/package.json"));
const dist = pathToFileURL(require.resolve("@earendil-works/pi-tui")).href.replace(/index\.js$/, "");
const { wrapTextWithAnsi, truncateToWidth } = await import(`${dist}utils.js`);
const { Markdown } = await import(`${dist}components/markdown.js`);
const { setCapabilities } = await import(`${dist}terminal-image.js`);
setCapabilities({ images: null, trueColor: true, hyperlinks: false });

const wrapCases = [
	["hello world foo bar", 7], ["hello world", 11], ["abcdefghijklmnop", 5], ["a  b   c", 3], ["  leading spaces here", 8],
	["trailing spaces    ", 9], ["日本語のテキストです", 7], ["mixed 日本語 text", 6], ["one\ntwo three\n\nfour", 5],
	["", 5], ["x", 1], ["word-with-hyphens and/slashes", 10], ["tab\there", 6], ["a b", 1], ["emoji 😀 test", 7],
];
const wrap = wrapCases.map(([text, width]) => ({ text, width, lines: wrapTextWithAnsi(text, width) }));

const truncateCases = [
	["hello world", 8, "..."], ["hello", 8, "..."], ["hello", 2, "..."], ["日本語テキスト", 7, "..."], ["abcdef", 4, ""],
	["abc", 0, "..."], ["hello world", 8, "…"],
];
const truncate = truncateCases.map(([text, width, ellipsis]) => ({ text, width, ellipsis, line: truncateToWidth(text, width, ellipsis) }));

const identity = (text) => text;
const theme = {
	heading: identity, link: identity, linkUrl: identity, code: identity, codeBlock: identity, codeBlockBorder: identity,
	quote: identity, quoteBorder: identity, hr: identity, listBullet: identity, bold: identity, italic: identity,
	strikethrough: identity, underline: identity,
};
const docs = {
	paragraphs: "First paragraph with **bold**, *italic* and `code`.\n\nSecond paragraph\nwith a soft break.",
	headings: "# Title\nIntro text.\n\n## Section\n\n### Sub section\n#### Deep\ntext",
	lists: "- one\n- two\n  - nested a\n  - nested b\n- three\n\n1. first\n2. second\n\n5. five\n6. six",
	loose: "- a\n\n- b\n\n- c\n\nafter",
	tasks: "- [x] done\n- [ ] todo",
	code: "Run this:\n\n```bash\necho hi\nls -la\n```\n\nAnd indented:\n\n    plain code\n    more",
	quote: "> Quoted line one\n> line two\n>\n> - list in quote\n\nafter quote",
	rule: "above\n\n---\n\nbelow",
	links: "See [the docs](https://example.com/docs) and <https://example.com> or mail <me@example.com>.",
	table: "| Name | Value |\n|------|-------|\n| alpha | 1 |\n| beta | two words |",
	wideTable: "| Column one | Column two with a much longer header | Three |\n|---|---|---|\n| short | a cell with several words that must wrap somewhere | x |",
	escapes: "Use \\*literal\\* asterisks and a \\_ underscore.",
	strike: "~~gone~~ and ~single~ tilde",
	html: "<div>block html</div>\n\nInline <b>tag</b> text.",
	cjk: "日本語の文章です。長い文章を折り返します。これはテストです。",
	longWord: "averyveryveryverylongwordthatcannotfit in a narrow column",
	mixed: "Text before list:\n- item one\n- item two\nText after?",
	headingCode: "## Using `cargo` here",
	nestedEmphasis: "***both*** and **bold _inner italic_ end**",
	numbers: "3. three\n4. four",
	trailing: "ends with blank lines\n\n\n",
};
// Bare URLs and email addresses, with links underlined so the lines show where each starts and ends.
const underlined = { ...theme, underline: (text) => `\x1b[4m${text}\x1b[24m` };
const autolinks = {
	bareUrls: "Visit https://example.com/docs, or http://x.io. Also www.example.com and ftp://files.example.org/a.txt!",
	bareEmails: "Mail me@example.com or first.last+tag@sub.example.co.uk. Not a@b or user@host_ or a@b.c- here.",
	urlParens: "(see https://en.wikipedia.org/wiki/Rust_(programming_language)) and https://x.com/a(b and https://x.com/q?a=1&b=2; done",
	urlPunctuation: "End https://example.com/path... and https://example.com/a_b_ and 'https://example.com/q?x=1' \"www.quoted.com\"",
	urlContext: "Midword xhttps://example.com and awww.example.com, `https://code.example.com`, [link https://in.link](https://target.com), **https://bold.example.com** and <https://auto.example.com>.",
	urlEdges: "HTTPS://EXAMPLE.COM, Www.example.com, http:// and www.-x. https://example.com/?a=1&amp; x!foo@bar.com a*b@c.com a@b.com.c@d.com",
	urlBlocks: "## See www.heading.com\n\n- item https://example.com/list\n- mail list@example.com\n\n| Site | Mail |\n|---|---|\n| www.cell.com | cell@example.com |",
	urlLatex: "See https://example.com/$x$ now, www.a.com/$y$ and $z$.\n\nhttps://api.example.com/odata?$filter=Price gt 5&$top=10",
};
// LaTeX, which pi-tui's tokenizers find before marked's own.
const latex = {
	latexInline: String.raw`A map $\mathbb{C}^3 \to \mathbb{C}^3$, $xy$, $x-y$, $\frac{1}{2}$, and \(s \to \infty\).`,
	latexDisplay: String.raw`Before

$$\{3x+2y,\; x \in \{0, \pm 1\}\}$$

\[
E \approx \frac{0.1\ \text{lux}}{100\ \text{lm/W}}
\]

after`,
	latexCut: "Intro line\n$$\nx^2 + y^2\n$$\nafter text\nmore\n$$ not math\nlast",
	latexContainers: "- Formula: $F_1 = u^2$\n- block:\n  $$\n  \\frac{1}{2}\n  $$\n- end $x$\n\n> quote $a^2$\n> $$\n> x_1\n> $$\n> after",
	latexTable: "| Value | Math |\n|---|---|\n| a | $\\mathbb{C}^3$ |\n| b | $x|y$ |",
	latexNotMath: "Costs $5 and $10 or $8k–$12k; use `$x$`, $HOME, and $" + "{PATH}. Paths: $HOME/$USER and $XDG_CONFIG_HOME/$APP_CONFIG",
	latexPending: String.raw`Streaming $\mathbb{C}^3 and *more*
next line

Text
\[
x^2
*not italic*`,
	latexEscapes: String.raw`Escaped \$x-y\$ and \\$x$ end, and \$$y$$.`,
	latexEmphasis: "*a $b$ c* and $x*y*z$ and *a $b* c$ and **$\\alpha$**",
	latexLinks: "[cost $x^2$](http://a.com/$y$) and <http://a.com/$z$> and ![alt $q$](i.png)",
	latexHeading: "## Area $\\pi r^2$\n\nUnknown $x + \\unknown{y}$ after\n\n$$\n\\unknown{x}\n$$",
	latexBlank: "$$\na\n\nb\n$$\nafter\n\n$$a$$ inline and $$\nb\n$$",
	latexEmphasisPairs:
		"$a*b$ and $c*d$, $2*3$ and $4*5$ and $6*7$.\n\nLet $f(x)=2*x$ and $g(x)=3*x$, $a _b$ and $c_ d$, $a~~b$ and $c~~d$.\n\n" +
		"- $a*b$ and $c*d$\n- $a _b$ and $c_ d$\n\n> $a~~b$ and $c~~d$\n\n## $2*3$ and $4*5$\n\n*a $x-2* b, $a*b$ $c*$d$",
	latexLinkText: "See [\\[x](u) and [\\(y\\)](u)",
};
const markdown = [];
for (const [name, text] of [...Object.entries(docs), ...Object.entries(autolinks), ...Object.entries(latex)]) {
	const underline = name in autolinks || undefined;
	for (const [width, paddingX, paddingY] of [[40, 0, 0], [24, 1, 0], [80, 1, 1]]) {
		for (const preserve of [false, true]) {
			const options = preserve ? { preserveOrderedListMarkers: true, preserveBackslashEscapes: true } : {};
			const component = new Markdown(text, paddingX, paddingY, underline ? underlined : theme, undefined, options);
			markdown.push({ name, text, width, paddingX, paddingY, preserve, underline, lines: component.render(width) });
		}
	}
}

// Links as OSC 8 hyperlinks, in a terminal that shows them.
setCapabilities({ images: null, trueColor: true, hyperlinks: true });
for (const name of ["links", "urlContext", "urlBlocks"]) {
	const text = docs[name] ?? autolinks[name];
	for (const [width, paddingX, paddingY] of [[40, 0, 0], [24, 1, 0]]) {
		const lines = new Markdown(text, paddingX, paddingY, theme).render(width);
		markdown.push({ name, text, width, paddingX, paddingY, preserve: false, hyperlinks: true, lines });
	}
}

const out = join(here, "..", "text");
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "text.json"), `${JSON.stringify({ wrap, truncate, markdown }, null, "\t")}\n`);
console.log(`${wrap.length} wrap, ${truncate.length} truncate, ${markdown.length} markdown cases`);
