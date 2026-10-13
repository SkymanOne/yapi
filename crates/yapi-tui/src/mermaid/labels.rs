//! Port of `src/labels.ts` of grok-mermaid 0.2.3.
//!
//! Label text handling: control stripping, HTML entity and markup removal,
//! line splitting, wrapping and truncation to display columns.

use super::width::{measured, string_width};

/// Node labels wrap to at most this many display columns per line ...
pub(super) const WRAP_WIDTH: usize = 24;
/// ... and at most this many lines; overflow is truncated with an ellipsis.
pub(super) const MAX_LINES: usize = 4;
/// Edge labels are truncated to this many columns.
pub(super) const MAX_LABEL: usize = 28;

/// Identifier-boundary characters preferred as break points when a single word
/// is too wide to fit, so it is not sliced mid-segment.
pub(super) const LABEL_BREAK_CHARS: [char; 4] = ['_', '-', '.', '/'];

// ------------------------------------------------- JavaScript string helpers
//
// The TS code leans on `String.prototype.trim` and the regex class `\s`, whose
// whitespace set (WhiteSpace plus LineTerminator) is not Rust's `White_Space`:
// it includes U+FEFF and excludes U+0085.

/// Whether `c` matches `\s` in a JavaScript regular expression.
pub(super) fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `String.prototype.trim`.
pub(super) fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `String.prototype.trimStart`.
pub(super) fn js_trim_start(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

/// `String.prototype.trimEnd`.
pub(super) fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_space)
}

/// `s.split(/\s+/).filter((w) => w !== '')`.
pub(super) fn js_words(s: &str) -> Vec<&str> {
    s.split(is_js_space).filter(|w| !w.is_empty()).collect()
}

/// `/\s/.test(s)`.
pub(super) fn has_js_space(s: &str) -> bool {
    s.chars().any(is_js_space)
}

// ----------------------------------------------------------------- case/text

/// ASCII-only case folding, matching Rust's `to_ascii_lowercase`.
///
/// `String.prototype.toLowerCase` can change a string's length (`İ` becomes two
/// code points), which would desync the offsets some parsers slice with.
pub(super) fn ascii_lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

pub(super) fn ascii_upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// Remove C0 and C1 controls, less the `\t\n\r` the parsers and `src_lines`
/// read.
///
/// They measure one column and paint none, so a box sized around one is drawn
/// a column short of its own border; NUL also collides with the `CONT`
/// sentinel and is dropped after layout has already paid for its cell; ESC
/// would inject ANSI into the caller's scrollback. `decode_entity_body`
/// refuses to decode an entity into one, and this closes the same hole for
/// literals.
pub(super) fn strip_controls(src: &str) -> String {
    src.chars()
        .filter(|&c| {
            !matches!(c, '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}'..='\u{9f}')
        })
        .collect()
}

/// Split source into lines the way Rust's `str::lines()` does: on `\n`, with a
/// trailing `\r` stripped, and without a final empty line when the input ends
/// in a newline.
pub(super) fn src_lines(src: &str) -> Vec<&str> {
    let mut out: Vec<&str> = src
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    if out.last() == Some(&"") {
        out.pop();
    }
    out
}

/// Matches Rust's `char::is_alphanumeric` (`\p{Alphabetic}` or `\p{N}`).
pub(super) fn is_alphanumeric(c: char) -> bool {
    c.is_alphanumeric()
}

/// Characters allowed in a bare node/state/class identifier.
pub(super) fn is_id_char(c: char) -> bool {
    is_alphanumeric(c) || c == '_'
}

const ENTITY_LOOKAHEAD: usize = 10;

fn decode_entity_body(body: &str) -> Option<char> {
    match body {
        "lt" => return Some('<'),
        "gt" => return Some('>'),
        "amp" => return Some('&'),
        "quot" => return Some('"'),
        "apos" => return Some('\''),
        _ => {}
    }
    let num = body.strip_prefix('#')?;
    let hex = num.starts_with(['x', 'X']);
    let digits = if hex { &num[1..] } else { num };
    let radix = if hex { 16 } else { 10 };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    // The scan window bounds `digits` to eight characters, so u64 cannot overflow.
    let code = digits.chars().try_fold(0u64, |acc, c| {
        Some(acc * u64::from(radix) + u64::from(c.to_digit(radix)?))
    })?;
    // Surrogates and out-of-range values are not characters at all.
    if code > 0x10ffff || (0xd800..=0xdfff).contains(&code) {
        return None;
    }
    // Reject control chars: NUL collides with the CONT sentinel and ESC would
    // inject ANSI into scrollback.
    if code < 0x20 || (0x7f..=0x9f).contains(&code) {
        return None;
    }
    char::from_u32(code as u32)
}

/// Decode HTML entities in label text. Called once per label: via
/// `clean_label` for bracketed labels, or explicitly at each direct-push sink.
pub(super) fn decode_html_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '&' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // Scan a bounded window including the terminating `;`, so a stray `&`
        // or an over-long run stays literal.
        let hi = (i + 1 + ENTITY_LOOKAHEAD).min(chars.len());
        let semi = (i + 1..hi).find(|&j| chars[j] == ';');
        let decoded =
            semi.and_then(|j| decode_entity_body(&chars[i + 1..j].iter().collect::<String>()));
        match (semi, decoded) {
            (Some(j), Some(d)) => {
                // Resume past the `;`. The single pass never re-scans emitted
                // text, so `&amp;lt;` decodes to the literal `&lt;`.
                out.push(d);
                i = j + 1;
            }
            _ => {
                out.push('&');
                i += 1;
            }
        }
    }
    out
}

/// Strip markdown emphasis from a `` `backtick` `` label string.
pub(super) fn strip_markdown(s: &str) -> String {
    let no_code: String = s.chars().filter(|&c| c != '`').collect();
    let no_strong = no_code.replace("**", "").replace("__", "");
    let chars: Vec<char> = no_strong.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        // Keep `*`/`_` only when they sit inside a word, so snake_case survives.
        let in_word = i > 0
            && is_alphanumeric(chars[i - 1])
            && chars.get(i + 1).is_some_and(|&n| is_alphanumeric(n));
        if (c == '*' || c == '_') && !in_word {
            continue;
        }
        out.push(c);
    }
    js_trim(&out).to_string()
}

/// Inline formatting tags that carry no meaning in a terminal. Anything else
/// that looks like a tag (`Vec<String>`, `<id>`) is left alone.
const HTML_FORMAT_TAGS: [&str; 25] = [
    "b", "strong", "i", "em", "u", "s", "strike", "del", "ins", "mark", "small", "big", "sub",
    "sup", "code", "kbd", "samp", "var", "tt", "span", "font", "q", "abbr", "cite", "pre",
];

/// Read a tag starting at `start`, returning its name and the index after `>`.
fn html_tag_at(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start + 1;
    if chars.get(i) == Some(&'/') {
        i += 1;
    }
    let name_start = i;
    while i < chars.len() && chars[i].is_ascii_alphanumeric() {
        i += 1;
    }
    if i == name_start {
        return None;
    }
    let name: String = chars[name_start..i].iter().collect();
    while i < chars.len() && chars[i] != '>' {
        if chars[i] == '<' {
            return None;
        }
        i += 1;
    }
    (chars.get(i) == Some(&'>')).then_some((name, i + 1))
}

pub(super) fn strip_html_tags(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<'
            && let Some((name, end)) = html_tag_at(&chars, i)
        {
            let lower = name.to_ascii_lowercase();
            if lower == "br" {
                out.push(' ');
                i = end;
                continue;
            }
            if HTML_FORMAT_TAGS.contains(&lower.as_str()) {
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Strip one matching pair of wrapping delimiters, if present.
fn unwrap<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    if s.len() >= open.len() + close.len() && s.starts_with(open) && s.ends_with(close) {
        Some(&s[open.len()..s.len() - close.len()])
    } else {
        None
    }
}

/// Normalise raw label text: strip markup, unquote, and decode entities.
///
/// Decoding happens after tag-stripping so `<b>` is removed as markup while
/// `&lt;b&gt;` survives as the literal text `<b>`.
pub(super) fn clean_label(raw: &str) -> String {
    let stripped = strip_html_tags(js_trim(raw));
    let trimmed = js_trim(&stripped);
    let unquoted = js_trim(
        unwrap(trimmed, "\"", "\"")
            .or_else(|| unwrap(trimmed, "'", "'"))
            .unwrap_or(trimmed),
    );
    match unwrap(unquoted, "`", "`") {
        None => decode_html_entities(unquoted),
        Some(md) => decode_html_entities(&strip_markdown(js_trim(md))),
    }
}

/// Byte index of the last identifier-boundary character, if any.
fn last_break(s: &str) -> Option<usize> {
    LABEL_BREAK_CHARS.iter().filter_map(|&c| s.rfind(c)).max()
}

/// Wrap a label to `width` columns over at most `max_lines` lines, truncating
/// the last line with an ellipsis if it overflows.
///
/// A word too wide to fit is broken after the last identifier boundary
/// (`_-./`) that fits, falling back to a per-character break when it has none.
pub(super) fn wrap_label(label: &str, width: usize, max_lines: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0;

    for word in js_words(label) {
        let ww = string_width(word);
        if ww > width {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let mut chunk = String::new();
            let mut chunk_w = 0;
            for (ch, cw) in measured(word) {
                if chunk_w + cw > width && !chunk.is_empty() {
                    // Boundary characters are ASCII, so `p + 1` is a char boundary.
                    let (head, carry) = match last_break(&chunk) {
                        Some(p) => (chunk[..=p].to_string(), chunk[p + 1..].to_string()),
                        None => (chunk.clone(), String::new()),
                    };
                    lines.push(head);
                    chunk_w = string_width(&carry);
                    chunk = carry;
                }
                chunk.push_str(ch);
                chunk_w += cw;
            }
            cur = chunk;
            cur_w = chunk_w;
        } else if cur.is_empty() {
            cur = word.to_string();
            cur_w = ww;
        } else if cur_w + 1 + ww <= width {
            cur.push(' ');
            cur.push_str(word);
            cur_w += 1 + ww;
        } else {
            lines.push(std::mem::replace(&mut cur, word.to_string()));
            cur_w = ww;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        let target = width.saturating_sub(1).max(1);
        let mut s = String::new();
        let mut sw = 0;
        if let Some(last) = lines.last() {
            for (ch, cw) in measured(last) {
                if sw + cw > target {
                    break;
                }
                s.push_str(ch);
                sw += cw;
            }
        }
        s.push('…');
        if let Some(last) = lines.last_mut() {
            *last = s;
        }
    }
    lines
}

/// Truncate to `inner` columns, leaving room for the ellipsis.
pub(super) fn fit_label(label: &str, inner: usize) -> String {
    if string_width(label) <= inner {
        return label.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for (c, cw) in measured(label) {
        if used + cw + 1 > inner {
            break;
        }
        out.push_str(c);
        used += cw;
    }
    out.push('…');
    out
}
