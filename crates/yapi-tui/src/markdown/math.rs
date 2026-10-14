//! pi's LaTeX tokenizers in `packages/tui/src/components/markdown.ts`.
//!
//! marked runs them before its own tokenizers at every block and inline
//! position, so an expression hides any Markdown inside it, and a display
//! block ends the paragraph before it. pulldown-cmark has no such hook: the
//! expressions are found on a first parse, each is replaced with a
//! placeholder that Markdown leaves alone, and the source is parsed again.
//! The replacements keep every line and container prefix, so the second parse
//! has the same blocks and blank lines.

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use crate::text::is_js_whitespace;

/// A LaTeX expression as pi's tokenizers find it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Math {
    /// The expression as written, with its delimiters.
    pub raw: String,
    /// The expression inside its delimiters.
    pub text: String,
    /// A display block rather than inline math.
    pub display: bool,
    /// Its closing delimiter has not arrived, as while it streams.
    pub pending: bool,
}

/// Starts a placeholder: `$`, this, the expression's index in private use
/// digits, [`END`] and `$`. The dollar signs make it punctuation at both
/// ends, as the delimiters are, for the emphasis rules around it.
const START: char = '\u{e000}';
const END: char = '\u{e001}';
/// Replaces each further line of a multi-line expression; the line is dropped.
const CONTINUATION: char = '\u{e002}';
const DIGIT_ZERO: u32 = 0xe010;

fn placeholder(index: usize) -> String {
    let digits: String = index
        .to_string()
        .chars()
        .filter_map(|digit| char::from_u32(DIGIT_ZERO + digit.to_digit(10)?))
        .collect();
    format!("${START}{digits}{END}$")
}

/// A piece of text from the source [`extract`] returns.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Segment<'a> {
    Text(&'a str),
    /// The placeholder of the expression with this index.
    Math(usize),
    /// A further line of an expression, to drop with the line break before
    /// it.
    Continuation,
}

/// Whether `text` holds anything [`extract`] put there.
pub(super) fn marked(text: &str) -> bool {
    text.contains([START, CONTINUATION])
}

/// Splits `text` into its pieces.
pub(super) fn segments(text: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find([START, CONTINUATION]) {
        let marker = rest[at..].chars().next();
        let (before, segment, after) = match marker {
            Some(CONTINUATION) => (
                &rest[..at],
                Segment::Continuation,
                &rest[at + CONTINUATION.len_utf8()..],
            ),
            _ => {
                // `$`, START, digits, END, `$`.
                let digits_start = at + START.len_utf8();
                let Some(digits_len) = rest[digits_start..].find(END) else {
                    break;
                };
                let index = rest[digits_start..digits_start + digits_len]
                    .chars()
                    .try_fold(0usize, |index, digit| {
                        let value = (digit as u32)
                            .checked_sub(DIGIT_ZERO)
                            .filter(|value| *value < 10)?;
                        Some(index * 10 + value as usize)
                    });
                let end = digits_start + digits_len + END.len_utf8();
                match (
                    index,
                    rest[..at].strip_suffix('$'),
                    rest[end..].strip_prefix('$'),
                ) {
                    (Some(index), Some(before), Some(after)) => {
                        (before, Segment::Math(index), after)
                    }
                    _ => break,
                }
            }
        };
        if !before.is_empty() {
            out.push(Segment::Text(before));
        }
        out.push(segment);
        rest = after;
    }
    if !rest.is_empty() {
        out.push(Segment::Text(rest));
    }
    out
}

/// `text` with each placeholder put back as written.
pub(super) fn restore(text: &str, maths: &[Math]) -> String {
    segments(text)
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => text,
            Segment::Math(index) => maths.get(index).map_or("", |math| math.raw.as_str()),
            Segment::Continuation => "",
        })
        .collect()
}

/// What contains a line, outermost first, for its prefix.
#[derive(Clone, Copy, Debug)]
enum Container {
    Quote,
    /// A list item whose content starts this many columns in.
    Item(usize),
}

/// The length of the container prefixes at the start of `line`: quote
/// markers and list item indentation. A lazy continuation line has fewer.
fn prefix_len(line: &str, containers: &[Container]) -> usize {
    let mut at = 0;
    for container in containers {
        let rest = &line[at..];
        match container {
            Container::Quote => {
                let spaces = rest.len() - rest.trim_start_matches(' ').len();
                if spaces > 3 || !rest[spaces..].starts_with('>') {
                    break;
                }
                at += spaces + 1;
                if line[at..].starts_with(' ') {
                    at += 1;
                }
            }
            Container::Item(indent) => {
                let spaces = rest.len() - rest.trim_start_matches(' ').len();
                at += spaces.min(*indent);
            }
        }
    }
    at
}

/// pi's `isEscaped`: an odd number of backslashes precedes `index`.
fn is_escaped(source: &str, index: usize) -> bool {
    source[..index]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1
}

/// pi's `findClosingDelimiter`.
fn find_closing(source: &str, closing: &str, start: usize) -> Option<usize> {
    let mut from = start;
    while let Some(found) = source.get(from..)?.find(closing) {
        let index = from + found;
        if !is_escaped(source, index) {
            return Some(index);
        }
        from = index + closing.len();
    }
    None
}

/// pi's `looksLikePendingDollarMath`.
fn looks_like_pending_dollar_math(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes
        .windows(2)
        .any(|pair| pair[0] == b'\\' && pair[1].is_ascii_alphabetic())
        || source
            .chars()
            .any(|c| "_^=+*/<>()[]|±≤≥≠≈∈→⇒∞∫∑√-".contains(c))
}

/// An expression a tokenizer found at the start of its source: the length
/// of its raw source, where its text is and whether it is pending.
type Found = (usize, Range<usize>, bool);

/// pi's `tokenizeInlineLatex`.
fn tokenize_inline(source: &str) -> Option<Found> {
    let (opening, closing) = if source.starts_with("$$") {
        ("$$", "$$")
    } else if source.starts_with("\\(") {
        ("\\(", "\\)")
    } else if source.starts_with("\\[") {
        ("\\[", "\\]")
    } else if source.starts_with('$') && !source[1..].starts_with(is_js_whitespace) {
        ("$", "$")
    } else {
        return None;
    };
    let closing_index = find_closing(source, closing, opening.len());
    if let Some(index) = closing_index
        && opening == "$"
    {
        let inner = &source[1..index];
        let after = &source[index + 1..];
        let constant = {
            let mut chars = inner.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
                && {
                    let rest = chars.as_str();
                    let body = rest.trim_start_matches(|c: char| {
                        c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'
                    });
                    let mut tail = body.chars();
                    match (tail.next(), tail.next()) {
                        (None, _) => true,
                        (Some(c), None) => {
                            !(c.is_ascii_alphanumeric() || c == '_' || is_js_whitespace(c))
                        }
                        _ => false,
                    }
                }
        };
        if inner.ends_with(is_js_whitespace)
            || after.starts_with(|c: char| c.is_ascii_digit())
            || (constant && after.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_'))
            || inner.contains('`')
        {
            return None;
        }
    }
    let Some(index) = closing_index else {
        let pending = &source[opening.len()..];
        return (opening.starts_with('\\') || looks_like_pending_dollar_math(pending)).then_some((
            source.len(),
            opening.len()..source.len(),
            true,
        ));
    };
    let text = &source[opening.len()..index];
    if text.is_empty() || text.contains('\n') {
        return None;
    }
    Some((index + closing.len(), opening.len()..index, false))
}

/// The end of `[ \t]*(?:\n|$)` from `at` in `source`, if it matches.
fn line_end_after(source: &str, at: usize) -> Option<usize> {
    let rest = &source[at..];
    let blanks = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    match rest[blanks..].chars().next() {
        None => Some(at + blanks),
        Some('\n') => Some(at + blanks + 1),
        Some(_) => None,
    }
}

/// pi's `tokenizeBlockLatex`, for a source starting at the delimiter.
fn tokenize_block(source: &str) -> Option<Found> {
    let content_start = |opening: &str| {
        let rest = &source[opening.len()..];
        let blanks = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let at = opening.len() + blanks;
        if source[at..].starts_with('\n') {
            at + 1
        } else {
            at
        }
    };
    for (opening, closing) in [("$$", "$$"), ("\\[", "\\]")] {
        if !source.starts_with(opening) {
            continue;
        }
        let start = content_start(opening);
        let mut from = start;
        while let Some(found) = source[from..].find(closing) {
            let index = from + found;
            if let Some(end) = line_end_after(source, index + closing.len()) {
                if index > start {
                    return Some((end, start..index, false));
                }
                break;
            }
            from = index + 1;
        }
    }
    if source.starts_with("\\[") {
        return Some((source.len(), content_start("\\[")..source.len(), true));
    }
    if source.starts_with("$$") {
        let start = content_start("$$");
        if start < source.len() && looks_like_pending_dollar_math(&source[start..]) {
            return Some((source.len(), start..source.len(), true));
        }
    }
    None
}

/// Where an expression may start: a line of a paragraph for a display
/// block, or a run of text for inline math, with the end of the text the
/// tokenizer may read.
struct Candidate {
    at: usize,
    limit: usize,
    /// The containers of its line, whose prefixes an expression's text
    /// leaves out on its further lines.
    containers: Vec<Container>,
    display: bool,
}

/// `source[range]` without the container prefixes of the lines that start
/// in it.
fn strip_prefixes(source: &str, range: Range<usize>, containers: &[Container]) -> String {
    let mut out = String::new();
    let mut at = range.start;
    loop {
        let line_end = source[at..range.end]
            .find('\n')
            .map_or(range.end, |newline| at + newline);
        let line = &source[at..line_end];
        if at == 0 || source.as_bytes()[at - 1] == b'\n' {
            out.push_str(&line[prefix_len(line, containers)..]);
        } else {
            out.push_str(line);
        }
        if line_end == range.end {
            return out;
        }
        out.push('\n');
        at = line_end + 1;
    }
}

/// The content indentation of a list item starting at `at` inside
/// `containers`, relative to their content: the marker and the spaces after
/// it.
fn item_indent(source: &str, at: usize, containers: &[Container]) -> usize {
    let line = &source[at..source[at..].find('\n').map_or(source.len(), |end| at + end)];
    let marker = line
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '-' | '+' | '*' | '.' | ')')))
        .unwrap_or(line.len());
    let spaces = line[marker..].len() - line[marker..].trim_start_matches(' ').len();
    let line_start = source[..at].rfind('\n').map_or(0, |newline| newline + 1);
    let leading = at - line_start - prefix_len(&source[line_start..at], containers);
    leading
        + marker
        + if (1..=4).contains(&spaces) && marker < line.len() {
            spaces
        } else {
            1
        }
}

/// The source with each LaTeX expression replaced by a placeholder, and the
/// expressions; `None` when it has none.
pub(super) fn extract(source: &str, options: Options) -> Option<(String, Vec<Math>)> {
    if !(source.contains('$') || source.contains("\\(") || source.contains("\\["))
        || source.contains([START, END, CONTINUATION])
    {
        return None;
    }
    let events: Vec<(Event<'_>, Range<usize>)> = Parser::new_ext(source, options)
        .into_offset_iter()
        .collect();
    let trimmed_end = |end: usize| source[..end].trim_end_matches(['\n', '\r']).len();

    let mut candidates = Vec::new();
    // Block containers with the end of each, and inline containers' ends,
    // innermost last.
    let mut containers: Vec<(Container, usize)> = Vec::new();
    let mut limits: Vec<usize> = Vec::new();
    // Inside an image or an autolink, whose text marked does not lex.
    let mut opaque = 0usize;
    // Whether each open link is an autolink.
    let mut links: Vec<bool> = Vec::new();
    let mut line_start = false;
    for (index, (event, range)) in events.iter().enumerate() {
        let starts_line = std::mem::take(&mut line_start);
        match event {
            Event::Start(Tag::BlockQuote(_)) => containers.push((Container::Quote, range.end)),
            Event::Start(Tag::Item) => {
                let outer: Vec<Container> =
                    containers.iter().map(|(container, _)| *container).collect();
                let indent = item_indent(source, range.start, &outer);
                containers.push((Container::Item(indent), range.end));
                line_start = true;
                // A tight item's text is one inline run.
                let end = events[index + 1..]
                    .iter()
                    .take_while(|(event, _)| {
                        super::is_inline(event) || matches!(event, Event::End(_))
                    })
                    .map(|(_, range)| range.end)
                    .filter(|end| *end <= range.end)
                    .last()
                    .unwrap_or(range.start);
                limits.push(trimmed_end(end));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                containers.pop();
            }
            Event::End(TagEnd::Item) => {
                containers.pop();
                limits.pop();
            }
            Event::Start(Tag::Paragraph) => {
                line_start = true;
                limits.push(trimmed_end(range.end));
            }
            Event::Start(Tag::Heading { .. } | Tag::TableCell) => {
                limits.push(trimmed_end(range.end));
            }
            Event::Start(Tag::Emphasis) => limits.push(range.end - 1),
            Event::Start(Tag::Strong) => limits.push(range.end - 2),
            Event::Start(Tag::Strikethrough) => {
                let tildes = source[range.clone()].len()
                    - source[range.clone()].trim_start_matches('~').len();
                limits.push(range.end - tildes);
            }
            Event::Start(Tag::Link {
                link_type: LinkType::Autolink | LinkType::Email,
                ..
            }) => {
                opaque += 1;
                links.push(true);
            }
            Event::Start(Tag::Image { .. } | Tag::CodeBlock(_)) => opaque += 1,
            Event::Start(Tag::Link { .. }) => {
                // The link text ends where its last child does.
                let mut depth = 0usize;
                let mut end = range.start;
                for (event, child) in &events[index + 1..] {
                    match event {
                        Event::Start(_) => depth += 1,
                        Event::End(TagEnd::Link) if depth == 0 => break,
                        Event::End(_) => depth -= 1,
                        _ => {}
                    }
                    end = child.end;
                }
                limits.push(end);
                links.push(false);
            }
            Event::End(TagEnd::Link) => {
                if links.pop() == Some(true) {
                    opaque -= 1;
                } else {
                    limits.pop();
                }
            }
            Event::End(TagEnd::Image | TagEnd::CodeBlock) => opaque -= 1,
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::TableCell
                | TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough,
            ) => {
                limits.pop();
            }
            Event::SoftBreak | Event::HardBreak => line_start = true,
            Event::Text(text) if opaque == 0 => {
                let Some(&limit) = limits.last() else {
                    continue;
                };
                let block_containers: Vec<Container> =
                    containers.iter().map(|(container, _)| *container).collect();
                // `\(` and `\[`, whose backslash pulldown-cmark reads as an
                // escape.
                let bracket = text.starts_with(['(', '['])
                    && source[..range.start].ends_with('\\')
                    && !is_escaped(source, range.start - 1);
                let display_start = (text.starts_with("$$") && !is_escaped(source, range.start))
                    || (bracket && text.starts_with('['));
                if starts_line && display_start {
                    // A display block on a paragraph's line. A continuation
                    // line may be indented by at most three columns.
                    let at = if bracket {
                        range.start - 1
                    } else {
                        range.start
                    };
                    let line = source[..at].rfind('\n').map_or(0, |newline| newline + 1);
                    let indent = at - line - prefix_len(&source[line..at], &block_containers);
                    let first = matches!(
                        events.get(index.wrapping_sub(1)),
                        Some((Event::Start(Tag::Paragraph | Tag::Item), _))
                    );
                    if first || indent <= 3 {
                        candidates.push(Candidate {
                            at,
                            limit: containers.last().map_or(source.len(), |(_, end)| *end),
                            containers: block_containers.clone(),
                            display: true,
                        });
                    }
                }
                if source.get(range.clone()) != Some(&**text) {
                    continue;
                }
                if bracket {
                    candidates.push(Candidate {
                        at: range.start - 1,
                        limit,
                        containers: block_containers.clone(),
                        display: false,
                    });
                }
                for (offset, _) in text.match_indices('$') {
                    let at = range.start + offset;
                    if !is_escaped(source, at) {
                        candidates.push(Candidate {
                            at,
                            limit,
                            containers: block_containers.clone(),
                            display: false,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // Display blocks first, as marked splits blocks before it lexes their
    // text, then inline math, each where no earlier expression covers it.
    candidates.sort_by_key(|candidate| (!candidate.display, candidate.at));
    let mut maths = Vec::new();
    let mut found: Vec<(Range<usize>, usize, Vec<Container>)> = Vec::new();
    for candidate in &candidates {
        let covered = |at: usize| found.iter().any(|(raw, ..)| raw.contains(&at));
        if covered(candidate.at) {
            continue;
        }
        let limit = if candidate.display {
            candidate.limit
        } else {
            // A paragraph ends at the line break before a display block.
            found
                .iter()
                .map(|(raw, ..)| raw.start)
                .filter(|start| *start > candidate.at)
                .map(|start| source[..start].rfind('\n').unwrap_or(start))
                .fold(candidate.limit, usize::min)
        };
        let token = if candidate.display {
            tokenize_block(&source[candidate.at..limit])
        } else {
            tokenize_inline(&source[candidate.at..limit])
        };
        let Some((len, text, pending)) = token else {
            continue;
        };
        let raw = candidate.at..candidate.at + len;
        let text = strip_prefixes(
            source,
            candidate.at + text.start..candidate.at + text.end,
            &candidate.containers,
        );
        maths.push(Math {
            raw: strip_prefixes(source, raw.clone(), &candidate.containers),
            // pi trims a complete display block's text.
            text: if candidate.display && !pending {
                text.trim().to_owned()
            } else {
                text
            },
            display: candidate.display,
            pending,
        });
        found.push((raw, maths.len() - 1, candidate.containers.clone()));
    }
    if maths.is_empty() {
        return None;
    }

    // Each expression's first line becomes its placeholder and its other
    // lines a continuation mark, after their container prefixes.
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    for (raw, index, containers) in &found {
        let mut line_start = raw.start;
        let mut first = true;
        while line_start < raw.end || first {
            let line_end = source[line_start..]
                .find('\n')
                .map_or(source.len(), |newline| line_start + newline);
            let content = if first {
                line_start
            } else {
                line_start + prefix_len(&source[line_start..line_end], containers)
            };
            let end = line_end.min(raw.end);
            edits.push((
                content..end.max(content),
                if first {
                    placeholder(*index)
                } else {
                    CONTINUATION.to_string()
                },
            ));
            first = false;
            line_start = line_end + 1;
        }
    }
    edits.sort_by_key(|(range, _)| range.start);
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for (range, text) in edits {
        out.push_str(&source[at..range.start]);
        out.push_str(&text);
        at = range.end;
    }
    out.push_str(&source[at..]);
    Some((out, maths))
}
