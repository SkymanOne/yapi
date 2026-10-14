//! Port of `src/parse.ts` of grok-mermaid 0.2.3.
//!
//! Source text to diagram model.
//!
//! Every `parse_x` returns `None` when the source is not that kind of diagram,
//! or when it exceeds a cap; `render` tries the parser the header names and
//! gives up on the whole diagram when it declines.

use super::graph::{
    ClassInfo, Edge, Graph, Group, Head, LineKind, MAX_EDGES, MAX_GROUP_DEPTH, MAX_GROUPS,
    MAX_MEMBERS, MAX_NODES, Shape, parse_dir,
};
use super::labels::{
    clean_label, decode_html_entities, is_id_char, js_trim, js_trim_end, js_trim_start, js_words,
    src_lines,
};
use crate::text::has_whitespace;
use std::collections::HashMap;

// ---------------------------------------------------------------- statements

fn flush_statement(cur: &mut String, out: &mut Vec<String>) {
    let trimmed = js_trim(cur);
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    cur.clear();
}

/// Split one source line into statements on `;`, stopping at a `%%` comment.
///
/// Quoted spans are opaque, so a label may contain `;` and `%%`.
fn split_statements(line: &str, out: &mut Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let mut cur = String::new();
    let mut in_quotes = false;
    for (i, &c) in chars.iter().enumerate() {
        if in_quotes {
            if c == '"' {
                in_quotes = false;
            }
            cur.push(c);
        } else if c == '"' {
            in_quotes = true;
            cur.push(c);
        } else if c == '%' && chars.get(i + 1) == Some(&'%') {
            break;
        } else if c == ';' {
            flush_statement(&mut cur, out);
        } else {
            cur.push(c);
        }
    }
    flush_statement(&mut cur, out);
}

/// All statements in a source block, in order.
fn statements_of(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src_lines(src) {
        split_statements(line, &mut out);
    }
    out
}

fn first_word(s: &str) -> &str {
    js_words(s).first().copied().unwrap_or("")
}

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// Diagram kind from the header statement, lowercased.
fn header_kind(statements: &[String]) -> Option<String> {
    let kind = first_word(statements.first()?);
    (!kind.is_empty()).then(|| kind.to_ascii_lowercase())
}

/// A diagram type this renderer draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiagramKind {
    Flowchart,
    State,
    Class,
    Er,
    Sequence,
}

/// The kind of diagram `src` declares, or `None` if its header names no type
/// this renderer draws.
///
/// Reads the header only; it says nothing about whether the body parses. Each
/// branch mirrors the header test in the matching `parse_x`, so the two always
/// agree on what they recognise.
pub(super) fn diagram_kind(src: &str) -> Option<DiagramKind> {
    let kind = header_kind(&statements_of(src))?;
    if kind == "graph" || kind == "flowchart" {
        Some(DiagramKind::Flowchart)
    } else if kind.starts_with("statediagram") {
        Some(DiagramKind::State)
    } else if kind.starts_with("classdiagram") {
        Some(DiagramKind::Class)
    } else if kind == "erdiagram" {
        Some(DiagramKind::Er)
    } else if kind == "sequencediagram" {
        Some(DiagramKind::Sequence)
    } else {
        None
    }
}

// ----------------------------------------------------------------- flowchart

pub(super) fn parse_graph(src: &str) -> Option<Graph> {
    let statements = statements_of(src);
    let kind = header_kind(&statements)?;
    if kind != "graph" && kind != "flowchart" {
        return None;
    }

    let mut graph = Graph::new(parse_dir(
        js_words(&statements[0]).get(1).copied().unwrap_or("TB"),
    ));
    let mut stack: Vec<usize> = Vec::new();

    for st in &statements[1..] {
        match first_word(st).to_ascii_lowercase().as_str() {
            "subgraph" => {
                if graph.groups.len() >= MAX_GROUPS || stack.len() >= MAX_GROUP_DEPTH {
                    return None;
                }
                // The keyword is eight ASCII bytes, so the slice is on a boundary.
                let (id, label) = parse_subgraph_decl(js_trim(&st["subgraph".len()..]));
                graph.groups.push(Group {
                    id,
                    label,
                    parent: stack.last().copied(),
                });
                stack.push(graph.groups.len() - 1);
                graph.cur_group = stack.last().copied();
                continue;
            }
            "end" => {
                stack.pop();
                graph.cur_group = stack.last().copied();
                continue;
            }
            "classdef" | "class" | "style" | "linkstyle" | "click" | "direction" => continue,
            _ => {}
        }
        parse_statement(st, &mut graph);
        if graph.over_cap {
            return None;
        }
    }

    if graph.nodes.is_empty() {
        None
    } else {
        Some(graph)
    }
}

/// `subgraph id[Title]`, `subgraph "Title"`, or a bare title.
fn parse_subgraph_decl(rest: &str) -> (String, String) {
    if rest.starts_with('"')
        && let Some(close) = rest[1..].find('"').map(|p| p + 1)
    {
        let label = &rest[1..close];
        return (label.to_string(), decode_html_entities(label));
    }
    if let Some(open) = rest.find('[') {
        let id = js_trim(&rest[..open]);
        let label = clean_label(js_trim(rest[open + 1..].trim_end_matches(']')));
        if !id.is_empty() && !label.is_empty() {
            return (id.to_string(), label);
        }
    }
    (rest.to_string(), rest.to_string())
}

/// A chain of `node link node link node ...`, each link fanning out over `&`.
///
/// Parses as far as it can and keeps the prefix, matching upstream and
/// mermaid.js. Whatever it could not read is recorded in `graph.warnings`
/// rather than failing the diagram.
fn parse_statement(st: &str, graph: &mut Graph) {
    let chars: Vec<char> = st.chars().collect();

    let Some(head) = parse_node_group(&chars, 0, graph) else {
        graph
            .warnings
            .push(format!("dropped, does not start with a node: \"{st}\""));
        return;
    };
    let mut prev = head.group;
    let mut i = head.next;

    loop {
        i = skip_spaces(&chars, i);
        if i >= chars.len() {
            break;
        }
        let Some(link) = parse_link(&chars, i) else {
            let rest: String = chars[i..].iter().collect();
            graph
                .warnings
                .push(format!("dropped, expected a link: \"{rest}\""));
            break;
        };
        i = skip_spaces(&chars, link.next);
        let Some(target) = parse_node_group(&chars, i, graph) else {
            graph
                .warnings
                .push(format!("dropped, link has no target: \"{st}\""));
            break;
        };
        i = target.next;
        for &f in &prev {
            for &t in &target.group {
                // `A <-- B` reads right-to-left: swap the endpoints so the arrow
                // that was written on the left becomes a normal forward head.
                let reversed = link.left == Head::Arrow && link.right != Head::Arrow;
                let pushed = graph.push_edge(Edge {
                    from: if reversed { t } else { f },
                    to: if reversed { f } else { t },
                    label: link.label.clone(),
                    head_to: if reversed { Head::Arrow } else { link.right },
                    head_from: if reversed { link.right } else { link.left },
                    line: link.line,
                });
                if !pushed {
                    return;
                }
            }
        }
        prev = target.group;
    }
}

struct NodeGroup {
    group: Vec<usize>,
    next: usize,
}

/// One or more nodes joined by `&`, which fan out into a cross product.
fn parse_node_group(chars: &[char], start: usize, graph: &mut Graph) -> Option<NodeGroup> {
    let first = parse_node(chars, start, graph)?;
    let mut group = vec![first.0];
    let mut i = first.1;
    loop {
        let j = skip_spaces(chars, i);
        if chars.get(j) != Some(&'&') {
            break;
        }
        let next = parse_node(chars, j + 1, graph)?;
        group.push(next.0);
        i = next.1;
    }
    Some(NodeGroup { group, next: i })
}

fn skip_spaces(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
        i += 1;
    }
    i
}

/// Returns the node index and the position after it.
fn parse_node(chars: &[char], start: usize, graph: &mut Graph) -> Option<(usize, usize)> {
    let mut i = skip_spaces(chars, start);
    let id_start = i;
    while i < chars.len() && is_id_char(chars[i]) {
        i += 1;
    }
    if i == id_start {
        return None;
    }
    let id: String = chars[id_start..i].iter().collect();

    let shaped = read_shape_at(chars, i);
    if let Some(unclosed) = shaped.unclosed {
        graph.warnings.push(format!(
            "node \"{id}\": label is missing its closing `{unclosed}`"
        ));
    }
    let index = graph.node_index(&id, shaped.label.as_deref(), shaped.shape)?;

    // `id:::name` (after any shape) attaches a style class; swallow it so the
    // statement keeps parsing, where upstream drops the rest of the line.
    let mut next = shaped.after;
    if style_tag_at(chars, next) {
        let k = style_tag_end(chars, next);
        if k > next + 3 {
            next = k;
        }
    }
    Some((index, next))
}

/// Whether `:::` starts at `i`.
fn style_tag_at(chars: &[char], i: usize) -> bool {
    chars.get(i) == Some(&':') && chars.get(i + 1) == Some(&':') && chars.get(i + 2) == Some(&':')
}

/// End of the name after the `:::` at `i`; equal to `i + 3` when it is empty.
fn style_tag_end(chars: &[char], i: usize) -> usize {
    let mut k = i + 3;
    while k < chars.len() && (is_id_char(chars[k]) || chars[k] == '-') {
        k += 1;
    }
    // A name never ends in `-`: back off so `A:::x-->B` keeps its link.
    while k > i + 3 && chars[k - 1] == '-' {
        k -= 1;
    }
    k
}

/// What a shape bracket yielded.
struct Shaped {
    shape: Shape,
    label: Option<String>,
    after: usize,
    /// The closing token that was expected but never found.
    unclosed: Option<&'static str>,
}

/// Dispatch on the bracket following an id to pick shape and closing token.
fn read_shape_at(chars: &[char], i: usize) -> Shaped {
    let c = chars.get(i).copied();
    let n = chars.get(i + 1).copied();
    match c {
        Some('[') => match n {
            Some('[') => read_shape(chars, i + 2, "]]", Shape::Rect),
            Some('(') => read_shape(chars, i + 2, ")]", Shape::Round),
            _ => read_shape(chars, i + 1, "]", Shape::Rect),
        },
        Some('(') => match n {
            Some('(') => read_shape(chars, i + 2, "))", Shape::Round),
            Some('[') => read_shape(chars, i + 2, "])", Shape::Round),
            _ => read_shape(chars, i + 1, ")", Shape::Round),
        },
        Some('{') => match n {
            Some('{') => read_shape(chars, i + 2, "}}", Shape::Diamond),
            _ => read_shape(chars, i + 1, "}", Shape::Diamond),
        },
        Some('>') => read_shape(chars, i + 1, "]", Shape::Rect),
        _ => Shaped {
            shape: Shape::Rect,
            label: None,
            after: i,
            unclosed: None,
        },
    }
}

/// Whether `closer` occurs in `chars` at `i`.
fn closer_at(chars: &[char], i: usize, closer: &str) -> bool {
    closer
        .chars()
        .enumerate()
        .all(|(k, c)| chars.get(i + k) == Some(&c))
}

/// Read label text up to `closer`.
///
/// Quoting is decided by the first non-space character: inside a quoted label
/// the closer is ignored until the quote closes, so `A["a] b"]` is one node.
/// An unquoted label ends at the first closer, so `A[5" pipe]` keeps its quote.
fn read_shape(chars: &[char], start: usize, closer: &'static str, shape: Shape) -> Shaped {
    let mut j = start;
    while chars.get(j) == Some(&' ') || chars.get(j) == Some(&'\t') {
        j += 1;
    }
    let quoted = chars.get(j) == Some(&'"');

    let mut i = start;
    let mut text = String::new();
    let mut in_quotes = false;
    while i < chars.len() {
        let c = chars[i];
        if quoted && c == '"' {
            in_quotes = !in_quotes;
            text.push(c);
            i += 1;
            continue;
        }
        if !in_quotes && closer_at(chars, i, closer) {
            return Shaped {
                shape,
                label: Some(clean_label(&text)),
                after: i + closer.chars().count(),
                unclosed: None,
            };
        }
        text.push(c);
        i += 1;
    }
    // Ran off the end still looking for the closer: everything after the
    // opening bracket became label text, so any link operator in it was
    // swallowed.
    Shaped {
        shape,
        label: Some(clean_label(&text)),
        after: chars.len(),
        unclosed: Some(closer),
    }
}

fn is_link_char(c: char) -> bool {
    matches!(c, '-' | '.' | '=' | '<' | '>')
}

struct Link {
    left: Head,
    right: Head,
    line: LineKind,
    label: Option<String>,
    next: usize,
}

/// Read a link operator and its label.
///
/// Labels come in two forms: `-->|text|` and the inline `-- text -->`, the
/// latter only when the first operator carried no head.
fn parse_link(chars: &[char], start: usize) -> Option<Link> {
    let mut i = skip_spaces(chars, start);
    let mut left = Head::None;
    // A leading `o`/`x` decorates the tail, but only directly before an operator.
    if matches!(chars.get(i), Some('o' | 'x')) && matches!(chars.get(i + 1), Some('-' | '.' | '='))
    {
        left = if chars[i] == 'o' {
            Head::Circle
        } else {
            Head::Cross
        };
        i += 1;
    }

    let op_start = i;
    while i < chars.len() && is_link_char(chars[i]) {
        i += 1;
    }
    if i == op_start {
        return None;
    }
    let op1: String = chars[op_start..i].iter().collect();
    if left == Head::None && op1.starts_with('<') {
        left = Head::Arrow;
    }

    let mut line = line_kind(&op1);
    let mut right = if op1.contains('>') {
        Head::Arrow
    } else {
        Head::None
    };
    if right == Head::None
        && let Some((head, next)) = trailing_head(chars, i)
    {
        right = head;
        i = next;
    }

    if chars.get(i) == Some(&'|') {
        i += 1;
        let l_start = i;
        while i < chars.len() && chars[i] != '|' {
            i += 1;
        }
        let label = clean_label(&chars[l_start..i].iter().collect::<String>());
        if chars.get(i) == Some(&'|') {
            i += 1;
        }
        return Some(Link {
            left,
            right,
            line,
            label: non_empty(&label),
            next: i,
        });
    }

    if right == Head::None {
        let text_start = skip_spaces(chars, i);
        let mut j = text_start;
        while j < chars.len() && !is_link_char(chars[j]) {
            j += 1;
        }
        if j < chars.len() && j > text_start && chars[j] != '<' {
            let text: String = chars[text_start..j].iter().collect();
            let op2_start = j;
            while j < chars.len() && is_link_char(chars[j]) {
                j += 1;
            }
            let op2: String = chars[op2_start..j].iter().collect();
            if op2.contains('>') {
                right = Head::Arrow;
            } else if let Some((head, next)) = trailing_head(chars, j) {
                right = head;
                j = next;
            }
            if line == LineKind::Solid {
                line = line_kind(&op2);
            }
            return Some(Link {
                left,
                right,
                line,
                label: non_empty(&clean_label(&text)),
                next: j,
            });
        }
    }

    Some(Link {
        left,
        right,
        line,
        label: None,
        next: i,
    })
}

fn line_kind(op: &str) -> LineKind {
    if op.contains('=') {
        LineKind::Thick
    } else if op.contains('.') {
        LineKind::Dotted
    } else {
        LineKind::Solid
    }
}

/// A trailing `o`/`x` head, only when followed by a statement boundary.
fn trailing_head(chars: &[char], i: usize) -> Option<(Head, usize)> {
    let head = match chars.get(i) {
        Some('o') => Head::Circle,
        Some('x') => Head::Cross,
        _ => return None,
    };
    let boundary = matches!(chars.get(i + 1), None | Some(' ' | '\t' | '|' | '&' | ';'));
    boundary.then_some((head, i + 1))
}

// --------------------------------------------------------------------- state

pub(super) fn parse_state(src: &str) -> Option<Graph> {
    let statements = statements_of(src);
    let kind = header_kind(&statements)?;
    if !kind.starts_with("statediagram") {
        return None;
    }

    let mut graph = Graph::new(super::graph::Dir::Down);
    let mut in_note = false;

    for st in &statements[1..] {
        if in_note {
            if st.eq_ignore_ascii_case("end note") {
                in_note = false;
            }
            continue;
        }
        let st = drop_style_tags(st);
        let first = first_word(&st).to_ascii_lowercase();
        if first == "direction" {
            graph.dir = parse_dir(js_words(&st).get(1).copied().unwrap_or(""));
        } else if first == "note" {
            // A single-line `note ... : text` needs no terminator.
            if !st.contains(':') {
                in_note = true;
            }
        } else if first == "state" {
            parse_state_decl(&st, &mut graph)?;
        } else if ["classdef", "class", "hide", "scale", "}", "--"].contains(&first.as_str()) {
            // Styling and composite-state punctuation carry no layout meaning.
        } else if st.contains("-->") {
            parse_transition(&st, &mut graph)?;
        } else {
            parse_state_desc(&st, &mut graph)?;
        }
        if graph.over_cap {
            return None;
        }
    }

    if graph.nodes.is_empty() {
        None
    } else {
        Some(graph)
    }
}

/// `state "Label" as id`, `state id <<choice>>`, or `state id {`.
fn parse_state_decl(st: &str, graph: &mut Graph) -> Option<()> {
    let rest = js_trim(skip_utf16(st, "state".len()));
    let rest = js_trim(rest.strip_suffix('{').unwrap_or(rest));
    if rest.is_empty() {
        return Some(());
    }

    if let Some(quoted) = rest.strip_prefix('"') {
        let close = quoted.find('"')?;
        let label = &quoted[..close];
        let after = js_trim(&quoted[close + 1..]);
        let id = match after.strip_prefix("as") {
            Some(a) => js_trim(a),
            None => label,
        };
        return graph
            .node_label(id, &decode_html_entities(label))
            .map(|_| ());
    }

    let mut shape = Shape::Round;
    let mut id = rest;
    let mut stereotyped = false;
    if let Some(pos) = rest.find("<<") {
        let tail = &rest[pos + 2..];
        let stereo = js_trim(tail.strip_suffix(">>").unwrap_or(tail));
        if stereo == "choice" {
            shape = Shape::Diamond;
        }
        id = js_trim(&rest[..pos]);
        stereotyped = true;
    }
    if id.is_empty() || has_whitespace(id) {
        return None;
    }
    graph
        .node_index(id, stereotyped.then_some(id), shape)
        .map(|_| ())
}

/// `A --> B: label`, including chains `A --> B --> C`.
fn parse_transition(st: &str, graph: &mut Graph) -> Option<()> {
    let mut rest = st;
    let mut prev: Option<usize> = None;

    while let Some((lhs, rhs)) = rest.split_once("-->") {
        let from_id = js_trim(js_trim_end(lhs).trim_end_matches('-'));
        let from = match prev {
            // Mid-chain: the source is the previous target, so nothing may precede.
            Some(p) => {
                if !from_id.is_empty() {
                    return None;
                }
                p
            }
            None => {
                if from_id.is_empty() {
                    return None;
                }
                state_endpoint(graph, from_id, true)?
            }
        };

        let (to_part_raw, tail) = match rhs.find("-->") {
            Some(p) => (&rhs[..p], &rhs[p..]),
            None => (rhs, ""),
        };

        let (to_part, label) = match to_part_raw.split_once(':') {
            Some((a, b)) => (a, non_empty(&decode_html_entities(js_trim(b)))),
            None => (to_part_raw, None),
        };

        let to_id = js_trim(
            js_trim_end(js_trim_start(to_part).trim_start_matches('>')).trim_end_matches('-'),
        );
        if to_id.is_empty() {
            return None;
        }
        let to = state_endpoint(graph, to_id, false)?;

        if !graph.push_edge(Edge {
            from,
            to,
            label,
            head_to: Head::Arrow,
            head_from: Head::None,
            line: LineKind::Solid,
        }) {
            return Some(());
        }
        prev = Some(to);
        rest = tail;
    }
    Some(())
}

/// `s` without its first `n` UTF-16 units, as JS `s.slice(n)`. Style tags can
/// leave multibyte whitespace before the keyword, so `n` is not a byte offset.
/// A cut inside a surrogate pair keeps the whole character.
fn skip_utf16(s: &str, n: usize) -> &str {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        if units >= n {
            return &s[i..];
        }
        units += c.len_utf16();
    }
    ""
}

/// Remove every `:::name` style tag from a statement, parsed with the same
/// name scan as the flowchart shorthand. State and class statements are read
/// by string splits, and the `:` label split would cut inside a `:::`, so tags
/// are dropped before dispatch rather than at each id.
fn drop_style_tags(st: &str) -> String {
    let chars: Vec<char> = st.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if style_tag_at(&chars, i) {
            let k = style_tag_end(&chars, i);
            if k > i + 3 {
                i = k;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `[*]` is start or end depending on which side of the arrow it sits.
fn state_endpoint(graph: &mut Graph, id: &str, is_source: bool) -> Option<usize> {
    if id == "[*]" {
        return graph.node_index(
            if is_source { "[*]start" } else { "[*]end" },
            Some("●"),
            Shape::Round,
        );
    }
    graph.node_index(id, None, Shape::Round)
}

/// `id: description`, or a bare state name.
fn parse_state_desc(st: &str, graph: &mut Graph) -> Option<()> {
    if let Some((a, b)) = st.split_once(':') {
        let id = js_trim(a);
        let desc = js_trim(b);
        if id.is_empty() || has_whitespace(id) || desc.is_empty() {
            return None;
        }
        return graph
            .node_label(id, &decode_html_entities(desc))
            .map(|_| ());
    }
    if has_whitespace(st) {
        return None;
    }
    graph.node_index(st, None, Shape::Round).map(|_| ())
}

// --------------------------------------------------------------------- class

/// Relation operators, longest-first so `--|>` wins over `--`.
const CLASS_OPS: [(&str, Head, Head, LineKind); 14] = [
    ("<|--", Head::Triangle, Head::None, LineKind::Solid),
    ("--|>", Head::None, Head::Triangle, LineKind::Solid),
    ("<|..", Head::Triangle, Head::None, LineKind::Dotted),
    ("..|>", Head::None, Head::Triangle, LineKind::Dotted),
    ("*--", Head::DiamondFill, Head::None, LineKind::Solid),
    ("--*", Head::None, Head::DiamondFill, LineKind::Solid),
    ("o--", Head::DiamondOpen, Head::None, LineKind::Solid),
    ("--o", Head::None, Head::DiamondOpen, LineKind::Solid),
    ("<--", Head::Arrow, Head::None, LineKind::Solid),
    ("-->", Head::None, Head::Arrow, LineKind::Solid),
    ("<..", Head::Arrow, Head::None, LineKind::Dotted),
    ("..>", Head::None, Head::Arrow, LineKind::Dotted),
    ("--", Head::None, Head::None, LineKind::Solid),
    ("..", Head::None, Head::None, LineKind::Dotted),
];

const MAX_CLASS_OP: usize = 4;

/// Declare a class, keeping `infos` aligned with `graph.nodes`.
fn declare_class(graph: &mut Graph, infos: &mut Vec<ClassInfo>, name: &str) -> Option<usize> {
    let idx = graph.node_index(name, None, Shape::Rect);
    infos.resize_with(graph.nodes.len(), ClassInfo::default);
    idx
}

pub(super) fn parse_class(src: &str) -> Option<(Graph, Vec<ClassInfo>)> {
    let statements = statements_of(src);
    let kind = header_kind(&statements)?;
    if !kind.starts_with("classdiagram") {
        return None;
    }

    let mut graph = Graph::new(super::graph::Dir::Down);
    let mut infos: Vec<ClassInfo> = Vec::new();
    let mut cur_class: Option<usize> = None;

    for st in &statements[1..] {
        if let Some(c) = cur_class {
            if st == "}" {
                cur_class = None;
            } else {
                push_member(&mut infos[c], st);
            }
            continue;
        }
        let st = drop_style_tags(st);

        let first = first_word(&st).to_ascii_lowercase();
        if first == "direction" {
            graph.dir = parse_dir(js_words(&st).get(1).copied().unwrap_or(""));
            continue;
        }
        if [
            "note",
            "callback",
            "click",
            "link",
            "style",
            "cssclass",
            "classdef",
            "namespace",
            "}",
        ]
        .contains(&first.as_str())
        {
            continue;
        }
        if first == "class" {
            let rest = js_trim(skip_utf16(&st, "class".len()));
            let open = rest.ends_with('{');
            let name = if open {
                js_trim(&rest[..rest.len() - 1])
            } else {
                rest
            };
            if name.is_empty() || has_whitespace(name) {
                return None;
            }
            let idx = declare_class(&mut graph, &mut infos, name)?;
            if open {
                cur_class = Some(idx);
            }
            continue;
        }

        if let Some(after) = st.strip_prefix("<<") {
            let (annotation, name) = after.split_once(">>")?;
            let name = js_trim(name);
            if name.is_empty() || has_whitespace(name) {
                return None;
            }
            let idx = declare_class(&mut graph, &mut infos, name)?;
            infos[idx].annotation = Some(js_trim(annotation).to_string());
            continue;
        }

        if let Some(rel) = parse_class_relation(&st) {
            let f = declare_class(&mut graph, &mut infos, &rel.from)?;
            let t = declare_class(&mut graph, &mut infos, &rel.to)?;
            if graph.edges.len() >= MAX_EDGES {
                return None;
            }
            graph.edges.push(Edge {
                from: f,
                to: t,
                label: rel.label,
                head_to: rel.head_to,
                head_from: rel.head_from,
                line: rel.line,
            });
            continue;
        }

        if let Some((a, b)) = st.split_once(':') {
            let id = js_trim(a);
            let text = js_trim(b);
            if id.is_empty() || has_whitespace(id) || text.is_empty() {
                return None;
            }
            let idx = declare_class(&mut graph, &mut infos, id)?;
            push_member(&mut infos[idx], text);
            continue;
        }
        return None;
    }

    if graph.nodes.is_empty() {
        return None;
    }
    infos.resize_with(graph.nodes.len(), ClassInfo::default);
    Some((graph, infos))
}

/// Add a member to the attribute or method compartment, eliding past the cap.
fn push_member(info: &mut ClassInfo, raw: &str) {
    if let Some(after) = raw.strip_prefix("<<") {
        if let Some((annotation, _)) = after.split_once(">>") {
            info.annotation = Some(js_trim(annotation).to_string());
        }
        return;
    }
    let member = decode_html_entities(&display_generics(js_trim(raw)));
    let list = if member.contains('(') {
        &mut info.methods
    } else {
        &mut info.attrs
    };
    if list.len() < MAX_MEMBERS {
        list.push(member);
    } else if list.len() == MAX_MEMBERS {
        list.push("…".to_string());
    }
}

struct ClassRelation {
    from: String,
    to: String,
    head_from: Head,
    head_to: Head,
    line: LineKind,
    label: Option<String>,
}

fn parse_class_relation(st: &str) -> Option<ClassRelation> {
    let chars: Vec<char> = st.chars().collect();
    let mut found: Option<(usize, usize, Head, Head, LineKind)> = None;

    'outer: for pos in 0..chars.len() {
        let tail: String = chars[pos..(pos + MAX_CLASS_OP).min(chars.len())]
            .iter()
            .collect();
        for &(op, head_from, head_to, line) in &CLASS_OPS {
            if !tail.starts_with(op) {
                continue;
            }
            // `o` is also an identifier character: skip a match glued to a name.
            if op.starts_with('o') && pos > 0 && is_id_char(chars[pos - 1]) {
                continue;
            }
            let op_len = op.chars().count();
            if op.ends_with('o') && chars.get(pos + op_len).is_some_and(|&a| is_id_char(a)) {
                continue;
            }
            found = Some((pos, op_len, head_from, head_to, line));
            break 'outer;
        }
    }
    let (pos, op_len, head_from, head_to, line) = found?;

    let lhs_raw = js_trim(&chars[..pos].iter().collect::<String>()).to_string();
    let rhs_raw = js_trim(&chars[pos + op_len..].iter().collect::<String>()).to_string();

    let (lhs, card_from) = strip_cardinality_suffix(&lhs_raw);
    let (rhs, card_to) = strip_cardinality_prefix(&rhs_raw);

    let (to_id_raw, rel_label) = match rhs.split_once(':') {
        Some((a, b)) => (a, non_empty(&decode_html_entities(js_trim(b)))),
        None => (rhs, None),
    };
    let to_id = js_trim(to_id_raw);

    if lhs.is_empty() || to_id.is_empty() || has_whitespace(lhs) || has_whitespace(to_id) {
        return None;
    }

    let label = non_empty(
        &[card_from, rel_label.as_deref().unwrap_or(""), card_to]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" "),
    );
    Some(ClassRelation {
        from: lhs.to_string(),
        to: to_id.to_string(),
        head_from,
        head_to,
        line,
        label,
    })
}

/// `Class "1"`: a quoted cardinality trailing the left-hand name.
fn strip_cardinality_suffix(s: &str) -> (&str, &str) {
    let t = js_trim_end(s);
    if let Some(rest) = t.strip_suffix('"')
        && let Some(q) = rest.rfind('"')
    {
        return (js_trim_end(&rest[..q]), &rest[q + 1..]);
    }
    (t, "")
}

/// `"0..*" Class`: a quoted cardinality leading the right-hand name.
fn strip_cardinality_prefix(s: &str) -> (&str, &str) {
    let t = js_trim_start(s);
    if let Some(rest) = t.strip_prefix('"')
        && let Some(q) = rest.find('"')
    {
        return (js_trim_start(&rest[q + 1..]), &rest[..q]);
    }
    (t, "")
}

/// Mermaid writes generics as `List~T~`; show them as `List<T>`.
pub(super) fn display_generics(s: &str) -> String {
    let mut out = String::new();
    let mut open = false;
    for c in s.chars() {
        if c == '~' {
            out.push(if open { '>' } else { '<' });
            open = !open;
        } else {
            out.push(c);
        }
    }
    out
}

// ------------------------------------------------------------------------ ER

pub(super) fn parse_er(src: &str) -> Option<(Graph, Vec<ClassInfo>)> {
    let statements = statements_of(src);
    if header_kind(&statements).as_deref() != Some("erdiagram") {
        return None;
    }

    let mut graph = Graph::new(super::graph::Dir::Down);
    let mut infos: Vec<ClassInfo> = Vec::new();
    let mut cur_entity: Option<usize> = None;

    for st in &statements[1..] {
        if let Some(c) = cur_entity {
            if st == "}" {
                cur_entity = None;
            } else {
                push_er_attribute(&mut infos[c], st);
            }
            continue;
        }

        if let Some((rel, rel_label)) = split_er_relationship(st) {
            let tokens = js_words(rel);
            if tokens.len() != 3 {
                return None;
            }
            let op = parse_er_op(tokens[1])?;
            let f = er_entity(&mut graph, &mut infos, tokens[0])?;
            let t = er_entity(&mut graph, &mut infos, tokens[2])?;
            if graph.edges.len() >= MAX_EDGES {
                return None;
            }
            let rel_label = rel_label.map(clean_label).unwrap_or_default();
            let label = [op.card_l, rel_label.as_str(), op.card_r]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            graph.edges.push(Edge {
                from: f,
                to: t,
                label: non_empty(&label),
                head_to: Head::None,
                head_from: Head::None,
                line: op.line,
            });
            continue;
        }

        let open = st.ends_with('{');
        let decl = if open {
            js_trim(&st[..st.len() - 1])
        } else {
            st.as_str()
        };
        if decl.is_empty() || js_words(decl).len() != 1 {
            return None;
        }
        let idx = er_entity(&mut graph, &mut infos, decl)?;
        if open {
            cur_entity = Some(idx);
        }
    }

    if graph.nodes.is_empty() {
        return None;
    }
    infos.resize_with(graph.nodes.len(), ClassInfo::default);
    Some((graph, infos))
}

fn er_entity(graph: &mut Graph, infos: &mut Vec<ClassInfo>, token: &str) -> Option<usize> {
    let idx = if let Some(open) = token.find('[') {
        let id = &token[..open];
        let label = clean_label(token[open + 1..].trim_end_matches(']'));
        if id.is_empty() || label.is_empty() {
            return None;
        }
        graph.node_label(id, &label)?
    } else {
        graph.node_index(token, None, Shape::Rect)?
    };
    infos.resize_with(graph.nodes.len(), ClassInfo::default);
    Some(idx)
}

/// Splits a statement into its relationship part and trimmed label text, when
/// the part holds a crow's-foot operator.
fn split_er_relationship(st: &str) -> Option<(&str, Option<&str>)> {
    let (rel, label) = match st.split_once(':') {
        Some((a, b)) => (a, Some(js_trim(b))),
        None => (st, None),
    };
    js_words(rel)
        .iter()
        .any(|t| parse_er_op(t).is_some())
        .then_some((rel, label))
}

struct ErOp {
    card_l: &'static str,
    card_r: &'static str,
    line: LineKind,
}

/// A crow's-foot operator: two cardinality glyphs around `--` or `..`.
fn parse_er_op(tok: &str) -> Option<ErOp> {
    if tok.len() != 6 || !tok.is_ascii() {
        return None;
    }
    let line = match &tok[2..4] {
        "--" => LineKind::Solid,
        ".." => LineKind::Dotted,
        _ => return None,
    };
    let card_l = er_card(&tok[0..2])?;
    let card_r = er_card(&tok[4..6])?;
    Some(ErOp {
        card_l,
        card_r,
        line,
    })
}

fn er_card(tok: &str) -> Option<&'static str> {
    match tok {
        "|o" | "o|" => Some("0..1"),
        "||" => Some("1"),
        "}o" | "o{" => Some("0..*"),
        "}|" | "|{" => Some("1..*"),
        _ => None,
    }
}

/// ER attributes are `type name`; a trailing quoted comment is dropped.
fn push_er_attribute(info: &mut ClassInfo, raw: &str) {
    let mut parts: Vec<&str> = Vec::new();
    for tok in js_words(raw) {
        if tok.starts_with('"') {
            break;
        }
        parts.push(tok);
    }
    if parts.is_empty() {
        return;
    }
    let line = decode_html_entities(&parts.join(" "));
    if info.attrs.len() < MAX_MEMBERS {
        info.attrs.push(line);
    } else if info.attrs.len() == MAX_MEMBERS {
        info.attrs.push("…".to_string());
    }
}

// ------------------------------------------------------------------ sequence

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SeqHead {
    Arrow,
    Cross,
}

/// Message operators, longest-first so `-->>` wins over `-->`.
const SEQ_OPS: [(&str, bool, SeqHead); 8] = [
    ("-->>", true, SeqHead::Arrow),
    ("->>", false, SeqHead::Arrow),
    ("--x", true, SeqHead::Cross),
    ("-x", false, SeqHead::Cross),
    ("--)", true, SeqHead::Arrow),
    ("-)", false, SeqHead::Arrow),
    ("-->", true, SeqHead::Arrow),
    ("->", false, SeqHead::Arrow),
];

const MAX_SEQ_OP: usize = 4;

#[derive(Clone, Copy, Debug)]
pub(super) enum NoteAnchor {
    Over { from: usize, to: usize },
    Left { at: usize },
    Right { at: usize },
}

#[derive(Clone, Debug)]
pub(super) enum SeqItem {
    Message {
        from: usize,
        to: usize,
        text: Option<String>,
        dashed: bool,
        head: SeqHead,
    },
    Note {
        anchor: NoteAnchor,
        text: String,
    },
    Divider {
        text: String,
    },
}

pub(super) struct Sequence {
    pub(super) labels: Vec<String>,
    pub(super) index: HashMap<String, usize>,
    pub(super) items: Vec<SeqItem>,
}

impl Sequence {
    fn new() -> Sequence {
        Sequence {
            labels: Vec::new(),
            index: HashMap::new(),
            items: Vec::new(),
        }
    }

    fn participant(&mut self, id: &str, label: Option<&str>) -> Option<usize> {
        if let Some(&existing) = self.index.get(id) {
            if let Some(label) = label {
                self.labels[existing] = label.to_string();
            }
            return Some(existing);
        }
        if self.labels.len() >= MAX_NODES {
            return None;
        }
        self.index.insert(id.to_string(), self.labels.len());
        self.labels.push(label.unwrap_or(id).to_string());
        Some(self.labels.len() - 1)
    }
}

pub(super) fn parse_sequence(src: &str) -> Option<Sequence> {
    let statements = statements_of(src);
    if header_kind(&statements).as_deref() != Some("sequencediagram") {
        return None;
    }

    let mut seq = Sequence::new();
    let mut autonumber = false;
    let mut msg_count = 0usize;
    // One entry per open block; `true` when it draws a divider on `end`.
    let mut blocks: Vec<bool> = Vec::new();

    for st in &statements[1..] {
        let first = first_word(st);
        let lower = first.to_ascii_lowercase();
        let lower = lower.as_str();

        if lower == "participant" || lower == "actor" {
            // The keyword matched case-insensitively in ASCII, so `first.len()`
            // is a char boundary.
            let rest = js_trim(&st[first.len()..]);
            if rest.is_empty() {
                return None;
            }
            let (id, label) = match rest.split_once(" as ") {
                Some((a, b)) => (js_trim(a), Some(clean_label(b))),
                None => (rest, None),
            };
            seq.participant(id, label.as_deref())?;
            continue;
        }
        if lower == "autonumber" {
            autonumber = true;
            continue;
        }
        if [
            "activate",
            "deactivate",
            "create",
            "destroy",
            "title",
            "acctitle",
            "accdescr",
            "links",
            "link",
            "properties",
        ]
        .contains(&lower)
        {
            continue;
        }
        if lower == "note" {
            let (anchor, text) = parse_note_anchor(js_trim(&st[first.len()..]), &mut seq)?;
            if seq.items.len() >= MAX_EDGES {
                return None;
            }
            seq.items.push(SeqItem::Note { anchor, text });
            continue;
        }
        if [
            "loop", "alt", "opt", "par", "critical", "break", "else", "and", "option",
        ]
        .contains(&lower)
        {
            if ["else", "and", "option"].contains(&lower) {
                // A continuation only divides a block that opened one.
                if blocks.last() != Some(&true) {
                    continue;
                }
            } else {
                blocks.push(true);
            }
            if seq.items.len() >= MAX_EDGES {
                return None;
            }
            seq.items.push(SeqItem::Divider {
                text: decode_html_entities(st),
            });
            continue;
        }
        if lower == "rect" || lower == "box" {
            blocks.push(false);
            continue;
        }
        if lower == "end" {
            if blocks.pop() == Some(true) {
                if seq.items.len() >= MAX_EDGES {
                    return None;
                }
                seq.items.push(SeqItem::Divider {
                    text: "end".to_string(),
                });
            }
            continue;
        }

        let msg = parse_seq_message(st, &mut seq)?;
        let mut text = msg.text;
        if autonumber {
            msg_count += 1;
            text = Some(match text {
                None => format!("{msg_count}."),
                Some(t) => format!("{msg_count}. {t}"),
            });
        }
        if seq.items.len() >= MAX_EDGES {
            return None;
        }
        seq.items.push(SeqItem::Message {
            from: msg.from,
            to: msg.to,
            text,
            dashed: msg.dashed,
            head: msg.head,
        });
    }

    if seq.labels.is_empty() {
        None
    } else {
        Some(seq)
    }
}

fn parse_note_anchor(rest: &str, seq: &mut Sequence) -> Option<(NoteAnchor, String)> {
    #[derive(PartialEq)]
    enum Kind {
        Over,
        Left,
        Right,
    }
    let lower = rest.to_ascii_lowercase();
    // Prefixes are ASCII and `ascii_lower` preserves byte lengths.
    let (kind, ids_and_text) = if lower.starts_with("over ") {
        (Kind::Over, &rest["over ".len()..])
    } else if lower.starts_with("left of ") {
        (Kind::Left, &rest["left of ".len()..])
    } else if lower.starts_with("right of ") {
        (Kind::Right, &rest["right of ".len()..])
    } else {
        return None;
    };

    let (ids, text) = ids_and_text.split_once(':')?;
    let text = decode_html_entities(js_trim(text));
    let parts: Vec<&str> = ids
        .split(',')
        .map(js_trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    let a = seq.participant(parts[0], None)?;

    match kind {
        Kind::Left => return Some((NoteAnchor::Left { at: a }, text)),
        Kind::Right => return Some((NoteAnchor::Right { at: a }, text)),
        Kind::Over => {}
    }
    let mut b = a;
    if let Some(second) = parts.get(1) {
        b = seq.participant(second, None)?;
    }
    Some((
        NoteAnchor::Over {
            from: a.min(b),
            to: a.max(b),
        },
        text,
    ))
}

struct SeqMessage {
    from: usize,
    to: usize,
    text: Option<String>,
    dashed: bool,
    head: SeqHead,
}

fn parse_seq_message(st: &str, seq: &mut Sequence) -> Option<SeqMessage> {
    let chars: Vec<char> = st.chars().collect();
    let mut found: Option<(usize, usize, bool, SeqHead)> = None;
    'outer: for pos in 0..chars.len() {
        let tail: String = chars[pos..(pos + MAX_SEQ_OP).min(chars.len())]
            .iter()
            .collect();
        for &(op, dashed, head) in &SEQ_OPS {
            if tail.starts_with(op) {
                found = Some((pos, op.chars().count(), dashed, head));
                break 'outer;
            }
        }
    }
    let (pos, op_len, dashed, head) = found?;

    let from_id = js_trim(&chars[..pos].iter().collect::<String>()).to_string();
    if from_id.is_empty() {
        return None;
    }
    // `+`/`-` activate and deactivate the target; they carry no layout meaning.
    let after: String = chars[pos + op_len..].iter().collect();
    let rest = js_trim_start(&after).trim_start_matches(['+', '-']);

    let (to_raw, text) = match rest.split_once(':') {
        Some((a, b)) => (a, non_empty(&decode_html_entities(js_trim(b)))),
        None => (rest, None),
    };
    let to_id = js_trim(to_raw);
    if to_id.is_empty() {
        return None;
    }

    let from = seq.participant(&from_id, None)?;
    let to = seq.participant(to_id, None)?;
    Some(SeqMessage {
        from,
        to,
        text,
        dashed,
        head,
    })
}
