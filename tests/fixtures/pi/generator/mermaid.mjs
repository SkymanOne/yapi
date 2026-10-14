// Records grok-mermaid, the Mermaid renderer pi uses, for
// crates/yapi-tui/tests/mermaid.rs: `render` on a corpus of diagrams and on
// every line prefix of each (what a streaming diagram passes through), and
// pi's Markdown transformer, which turns top-level Mermaid code blocks into
// diagrams, with pi-tui's rendering of its output (identity theme).
import { mkdirSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const agent = join(here, "node_modules/@earendil-works/pi-coding-agent");
const require = createRequire(join(agent, "package.json"));
const { render } = await import(pathToFileURL(require.resolve("grok-mermaid")).href);
const { createMermaidMarkdownTransformer } = await import(
	pathToFileURL(join(agent, "dist/modes/interactive/components/mermaid.js")).href
);
const tui = pathToFileURL(require.resolve("@earendil-works/pi-tui")).href.replace(/index\.js$/, "");
const { Markdown } = await import(`${tui}components/markdown.js`);

const sources = [
	"graph TD\n  A[Start] --> B{Is it?}\n  B -->|Yes| C[OK]\n  C --> D[Rethink]\n  D --> B\n  B ---->|No| E[End]",
	"flowchart LR\n  A[Start] --> B[Done]",
	"flowchart RL\n  A --> B --> C",
	"flowchart BT\n  A((circle)) --> B>flag] --> C{{hex}}",
	"graph LR\n  A[(db)] -.-> B([stadium]) ==> C[[sub]]\n  C --- D\n  D -.- E\n  E === F",
	"flowchart TD\n  A -- text --> B\n  B -. dotted .-> C\n  C == thick ==> D\n  A --> C",
	"flowchart TD\n  A --> B & C --> D",
	"flowchart TD\n  subgraph one[First]\n    a1 --> a2\n  end\n  subgraph two\n    b1 --> b2\n  end\n  a2 --> b1\n  c1 --> a1",
	"flowchart LR\n  subgraph outer\n    subgraph inner\n      x --> y\n    end\n    y --> z\n  end\n  start --> x",
	"graph TD\n  A[\"Quoted <b>label</b> with ; semicolon\"] --> B[\"日本語ラベル\"]\n  B --> C[\"emoji 🚀 test\"]",
	"graph TD\n  A --> B\n  %% a comment\n  B --> C; C --> A",
	"graph TD\n  A[Start --> B",
	"graph TD\n  A --> B\n  C -->",
	"graph LR\n  A --o B\n  B --x C\n  C <--> D\n  D o--o E",
	"flowchart TD\n  A[Multi<br>line<br/>label] --> B[Another]\n  classDef red fill:#f00\n  class A red\n  style B fill:#0f0\n  click A callback",
	"flowchart TD\n  id1(Rounded) --> id2[/Parallelogram/] --> id3[\\Alt\\] --> id4[/Trapezoid\\] --> id5[\\Alt trap/]",
	"flowchart TD\n  A --> B\n  A --> C\n  A --> D\n  B --> E\n  C --> E\n  D --> E\n  E --> F\n  F --> A",
	"stateDiagram-v2\n  [*] --> Still\n  Still --> [*]\n  Still --> Moving\n  Moving --> Still\n  Moving --> Crash\n  Crash --> [*]",
	"stateDiagram\n  state \"Long name\" as LN\n  [*] --> LN\n  LN --> Done: finish\n  Done --> [*]",
	"stateDiagram-v2\n  A: description of A\n  A --> B : go\n  B --> C\n  state C {\n    x --> y\n  }",
	"classDiagram\n  Animal <|-- Duck\n  Animal <|-- Fish\n  Animal : +int age\n  Animal : +String gender\n  Animal: +isMammal()\n  class Duck{\n    +String beakColor\n    +swim()\n    +quack()\n  }\n  class Fish{\n    -int sizeInFeet\n    -canEat()\n  }",
	"classDiagram\n  class Shape~T~ {\n    <<interface>>\n    +area() double\n  }\n  Shape <|.. Circle\n  Circle *-- Point : center\n  Circle o-- Style\n  Circle --> Canvas : draws on\n  Circle ..> Logger",
	"classDiagram\n  A \"1\" --> \"*\" B : has\n  B -- C",
	"erDiagram\n  CUSTOMER ||--o{ ORDER : places\n  ORDER ||--|{ LINE-ITEM : contains\n  CUSTOMER }|..|{ DELIVERY-ADDRESS : uses",
	"erDiagram\n  CUSTOMER {\n    string name\n    string custNumber PK\n    string sector\n  }\n  ORDER {\n    int orderNumber PK\n    string deliveryAddress FK \"comment\"\n  }\n  CUSTOMER ||--o{ ORDER : places",
	"sequenceDiagram\n  Alice->>John: Hello John, how are you?\n  John-->>Alice: Great!\n  Alice-)John: See you later!",
	"sequenceDiagram\n  participant A as Alice\n  actor B as Bob\n  A->>B: Hi\n  Note right of B: Bob thinks\n  B-->>A: Hello\n  Note over A,B: Both\n  A->A: self\n  B-xA: cross",
	"sequenceDiagram\n  autonumber\n  A->>B: one\n  loop Every minute\n    B->>A: ping\n  end\n  alt ok\n    A->>B: yes\n  else bad\n    A->>B: no\n  end\n  opt maybe\n    B->>A: maybe\n  end",
	"sequenceDiagram\n  A->>+B: activate\n  B-->>-A: deactivate\n  A->>B: done",
	"pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85",
	"gantt\n  title A Gantt\n  section A\n  task :a1, 2014-01-01, 30d",
	"",
	"   \n  ",
	"not a diagram at all",
	"graph",
	"graph TD",
	"flowchart TD\n  A --> B\n  B --> C\n  C --> D\n  D --> E\n  E --> F\n  F --> G\n  G --> H\n  H --> I\n  I --> J\n  J --> K\n  K --> L",
	"flowchart LR\n  A[A very long label that goes on and on and on to make the diagram wide] --> B[Another long label that also goes on and on] --> C[And a third label to exceed widths]",
	"graph TD;\n  A-->B;\n  A-->C;\n  B-->D;\n  C-->D;",
	"flowchart TD\n  A[\"label with `backtick`\"] --> B[\"``double``\"]",
	"flowchart TD\n  A --> B\n  B --> A",
	"flowchart TD\n  A --> A",
	"flowchart TD\n  A\n  B\n  C",
	"flowchart TD\n  A --> B\n  linkStyle 0 stroke:#ff3\n  A:::someclass --> C",
	"stateDiagram-v2\n  [*] --> First\n  state fork_state <<fork>>\n  First --> fork_state\n  fork_state --> State2\n  fork_state --> State3\n  state join_state <<join>>\n  State2 --> join_state\n  State3 --> join_state\n  join_state --> [*]",
	"stateDiagram-v2\n  A --> B\n  note right of A : a note\n  B --> C",
	"classDiagram\n  class Foo\n  class Bar {\n    +List~int~ items\n    +get(int i)$ int\n    +abstract()*\n  }\n  Foo <|-- Bar",
	"erDiagram\n  A ||--|| B : one\n  B |o--o| C : zero\n  C }o--o{ D : many",
	"sequenceDiagram\n  participant Alice\n  participant Bob\n  Alice->>Bob: line with ; semicolon\n  rect rgb(0, 255, 0)\n  Bob->>Alice: inside rect\n  end\n  par A to B\n    Alice->>Bob: p1\n  and A to C\n    Alice->>Bob: p2\n  end\n  critical Section\n    Bob->>Alice: c\n  end\n  break when\n    Alice->>Bob: b\n  end",
	"sequenceDiagram\n  A->>B: 日本語メッセージ\n  B->>A: emoji 🎉",
	"graph TD\n  A[Start] --> B[Process]\n  B --> C{Decision}\n  C -->|One| D[Result 1]\n  C -->|Two| E[Result 2]\n  C -->|Three| F[Result 3]\n  D --> G[End]\n  E --> G\n  F --> G",
	"flowchart LR\n  subgraph s1 [Group One]\n    direction TB\n    a --> b\n  end\n  subgraph s2\n    c\n  end\n  s1 --> s2",
	"graph TD\n  A --> B\n  B --> C\n  C -.->|retry| A",
	"flowchart TD\n  A & B --> C & D",
	"flowchart TD\n  a1[\"&quot;quoted&quot; &amp; &#35;\"] --> a2",
	"flowchart TD\n  A[Label with unclosed paren( ] --> B",
	"classDiagram\n  direction LR\n  A --> B\n  B --> C",
	"sequenceDiagram\n  A->>B: hello\n  B->>",
	"stateDiagram-v2\n  [*] --> A\n  A --> B\n  B -->",
	"erDiagram\n  A ||--o{ B : rel\n  B ||--",
	// Audit additions.
	"stateDiagram-v2\n  A --> B\n  :::a　　state X",
	"classDiagram\n  A <|-- B\n  :::a   class C",
	"stateDiagram-v2\n  A --> B\n  :::a state",
	"flowchart TD\n  A -->|loop| A\n  B --> C",
	"flowchart LR\n  A -->|self| A\n  A --> B",
	"flowchart TD\n  A <-- B\n  B x--x C",
	"flowchart TD\n  A[\"&#x41;&#66; hex &#x1F600;\"] --> B[\"&#x3c;tag&#x3e;\"]",
	"flowchart TD\n  --> A\n  A --> B",
	"flowchart TD\n  A --> B\n  B --> C\n  A --> B\n  C --> D",
	"stateDiagram-v2\n  state c <<choice>>\n  [*] --> c\n  c --> A: yes\n  c --> B: no",
	"stateDiagram-v2\n  A --> B\n  note left of A\n    line one\n    line two\n  end note\n  B --> C",
	"stateDiagram-v2\n  direction LR\n  hide empty description\n  scale 350 width\n  A --> B\n  --\n  B --> C",
	"classDiagram\n  A --|> B\n  C ..|> D\n  E --* F\n  G --o H\n  I <-- J\n  K <.. L\n  M .. N",
	"classDiagram\n  class Shape\n  <<interface>> Shape\n  Shape <|-- Circle",
	"classDiagram\n  namespace Zoo {\n    class Duck\n    class Fish\n  }\n  Duck --> Fish",
	"classDiagram\n  class Big {\n    +a\n    +b\n    +c\n    +d\n    +e\n    +f\n    +g\n    +h\n    +i\n    +j\n    +k\n  }\n  Big --> Other",
	"erDiagram\n  A[alias] ||--o{ B : has\n  B {\n    int id PK\n  }",
	"sequenceDiagram\n  participant A\n  participant B\n  Note left of A: left note\n  Note right of B: right note\n  Note over A: solo note\n  A->>B: hi",
	"sequenceDiagram\n  A->>B: x\n  option maybe\n    B->>A: y\n  end",
	"sequenceDiagram\n  box Aqua Group\n    participant A\n    participant B\n  end\n  A->>B: in box",
	"sequenceDiagram\n  A--)B: async\n  A->B: open\n  A-->B: dotted open\n  B--xA: dotted cross",
	"sequenceDiagram\n  activate A\n  A->>B: work\n  deactivate A\n  create participant C\n  B->>C: new\n  destroy C\n  C-->>B: bye",
	"sequenceDiagram\n  create actor D as Dan\n  A->>D: hello\n  destroy D",
];

const cases = [];
const seen = new Set();
const add = (source) => {
	if (seen.has(source)) return;
	seen.add(source);
	const art = render(source);
	cases.push({ source, art: art && { plain: art.plain, styled: art.styled, width: art.width, warnings: art.warnings } });
};
for (const source of sources) {
	add(source);
	const lines = source.split("\n");
	for (let n = 1; n < lines.length; n++) add(lines.slice(0, n).join("\n"));
	// Mid-line states of the last line, cut by code point.
	const last = [...lines[lines.length - 1]];
	for (let k = 1; k < last.length; k += 3) add([...lines.slice(0, -1), last.slice(0, k).join("")].join("\n"));
}

const identity = (text) => text;
const theme = {
	heading: identity, link: identity, linkUrl: identity, code: identity, codeBlock: identity, codeBlockBorder: identity,
	quote: identity, quoteBorder: identity, hr: identity, listBullet: identity, bold: identity, italic: identity,
	strikethrough: identity, underline: identity,
};
const fence = (body, info = "mermaid") => `\`\`\`${info}\n${body}\n\`\`\``;
const flow = "flowchart LR\n  A[Start] --> B[Done]";
const docs = {
	alone: fence(flow),
	inText: `Before the diagram:\n\n${fence("graph TD\n  A --> B\n  B --> C")}\n\nAfter it.`,
	adjacent: `Text\n${fence(flow)}\nmore text`,
	upperInfo: fence(flow, "Mermaid title=x"),
	unsupported: fence('pie title Pets\n  "Dogs" : 386'),
	warning: fence("graph TD\n  A[Start --> B"),
	tooWide: fence("flowchart LR\n  A[A very long label that goes on] --> B[Another long label] --> C[And a third one]"),
	nested: `- item\n\n  ${fence(flow).replaceAll("\n", "\n  ")}\n\n> ${fence(flow).replaceAll("\n", "\n> ")}`,
	unclosed: "Streaming:\n\n```mermaid\nflowchart LR\n  A --> B",
	backticks: fence('flowchart LR\n  A["tick ` here"] --> B["``edge``"]'),
	sequence: fence("sequenceDiagram\n  Alice->>Bob: Hi\n  Bob-->>Alice: Hello"),
	two: `${fence(flow)}\n\n${fence("graph TD\n  X --> Y")}`,
	mixedCase: fence(flow, "MerMaid"),
	crlf: `Intro\r\n\r\n${fence(flow).replaceAll("\n", "\r\n")}\r\n\r\nOutro\r\n`,
	loneCr: `Intro\r\r${fence(flow).replaceAll("\n", "\r")}\r`,
	indented: `Text\n\n  ${fence(flow).replaceAll("\n", "\n  ")}\n\nAfter`,
	indentedWarning: `  ${fence("graph TD\n  A[Start --> B").replaceAll("\n", "\n  ")}\n`,
	crlfWarning: `${fence("graph TD\n  A[Start --> B").replaceAll("\n", "\r\n")}\r\nend`,
	tildes: `~~~mermaid\n${flow}\n~~~`,
};
const transforms = [];
for (const [name, text] of Object.entries(docs)) {
	for (const width of [80, 30]) {
		for (const isStreaming of [false, true]) {
			const transformer = createMermaidMarkdownTransformer({ getMode: () => "streaming" });
			const context = (availableWidth) => ({ messageType: "assistant", isStreaming, availableWidth });
			const transformed = transformer(text, context(width - 2));
			const transform = (markdown, availableWidth) => transformer(markdown, context(availableWidth));
			const lines = new Markdown(text, 1, 0, theme, undefined, { transform }).render(width);
			transforms.push({ name, text, width, isStreaming, transformed, lines });
		}
	}
}

const out = join(here, "..", "mermaid");
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "render.json"), `${JSON.stringify(cases)}\n`);
writeFileSync(join(out, "transform.json"), `${JSON.stringify(transforms, null, "\t")}\n`);
console.log(`${cases.length} render cases (${cases.filter((c) => c.art).length} drawn), ${transforms.length} transform cases`);
