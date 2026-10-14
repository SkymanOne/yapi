//! pi's Mermaid Markdown transformer, `components/mermaid.ts` in pi-coding-agent
//! `v1.0.0`: top-level Mermaid code blocks become their diagrams, as rows of
//! inline code that Markdown keeps on their own lines.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::{Cls, MermaidArt, render};
use crate::ansi::sgr;
use crate::theme::Theme;

/// pi's `codeSpan`: `line` as inline code that keeps its spacing. A blank row
/// becomes a no-break space, as an empty code span has no height, and the
/// delimiter is one backtick longer than any run in the line.
fn code_span(line: &str) -> String {
    let content = if line.is_empty() { "\u{a0}" } else { line };
    let mut longest = 0;
    let mut run = 0;
    for c in content.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest + 1);
    let padding = if content.starts_with('`') || content.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{padding}{content}{padding}{fence}")
}

/// pi's `theme.fg`: `text` in the theme color `token`.
fn fg(theme: &Theme, token: &str, text: &str) -> String {
    let reset = if theme.is_dim(token) {
        "\x1b[22;39m"
    } else {
        "\x1b[39m"
    };
    format!("{}{text}{reset}", sgr(theme.fg(token)))
}

/// pi's `themedLines`: each row with its runs in the theme's colors.
fn themed_lines(art: &MermaidArt, theme: &Theme) -> Vec<String> {
    art.styled
        .iter()
        .map(|row| {
            row.iter()
                .map(|span| match span.cls {
                    Cls::Border => fg(theme, "borderMuted", &span.text),
                    Cls::Text => fg(theme, "text", &span.text),
                    Cls::Edge => fg(theme, "accent", &span.text),
                    Cls::EdgeLabel => fg(theme, "muted", &span.text),
                    Cls::Title => fg(theme, "accent", &format!("\x1b[1m{}\x1b[22m", span.text)),
                    Cls::None => span.text.clone(),
                })
                .collect()
        })
        .collect()
}

/// The language of a code block's info string, as pi compares it.
fn is_mermaid(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .is_some_and(|lang| lang.eq_ignore_ascii_case("mermaid"))
}

/// Where marked's code token for a fenced block that pulldown-cmark ends at
/// `end` ends: after its closing fence, with the line break after it when no
/// blank line follows, or at the end of the source when it is not closed.
fn token_end(markdown: &str, end: usize) -> usize {
    let Some(rest) = markdown[end..].strip_prefix('\n') else {
        return end;
    };
    // marked hands a lone line break to the token before it.
    let next = rest.split('\n').next().unwrap_or_default();
    if rest.is_empty() || !next.trim_matches([' ', '\t']).is_empty() {
        end + 1
    } else {
        end
    }
}

/// pi's Mermaid transformer, for a mode that draws diagrams: each top-level
/// Mermaid code block in `markdown` as its diagram when one is drawn and it
/// fits `available_width`, else as written. Once the message is final
/// (`streaming` false), a diagram with warnings stays as written, followed by
/// the first warning. With a theme, the diagram and warning are colored as in
/// pi.
pub fn transform(
    markdown: &str,
    available_width: usize,
    streaming: bool,
    theme: Option<&Theme>,
) -> String {
    if !markdown
        .as_bytes()
        .windows(7)
        .any(|w| w.eq_ignore_ascii_case(b"mermaid"))
    {
        return markdown.to_owned();
    }
    // marked's lexer reads every line break as `\n`, so pi's output has no `\r`.
    let normalized;
    let markdown = if markdown.contains('\r') {
        normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
        normalized.as_str()
    } else {
        markdown
    };
    let mut out = String::with_capacity(markdown.len());
    let mut at = 0;
    let mut depth = 0usize;
    let mut events = Parser::new_ext(markdown, Options::empty()).into_offset_iter();
    while let Some((event, range)) = events.next() {
        match event {
            Event::Start(Tag::BlockQuote(_) | Tag::List(_) | Tag::FootnoteDefinition(_)) => {
                depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::FootnoteDefinition) => {
                depth -= 1;
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                if depth == 0 && is_mermaid(&info) =>
            {
                let mut code = String::new();
                for (event, _) in events.by_ref() {
                    match event {
                        Event::Text(text) => code.push_str(&text),
                        Event::End(TagEnd::CodeBlock) => break,
                        _ => {}
                    }
                }
                let end = token_end(markdown, range.end);
                // marked's token includes the fence's indentation.
                let line_start = markdown[..range.start].rfind('\n').map_or(0, |i| i + 1);
                let start = if markdown[line_start..range.start].bytes().all(|b| b == b' ') {
                    line_start
                } else {
                    range.start
                };
                let code = code.strip_suffix('\n').unwrap_or(&code);
                let raw = &markdown[start..end];
                out.push_str(&markdown[at..start]);
                at = end;
                let Some(art) = render(code).filter(|art| art.width <= available_width) else {
                    out.push_str(raw);
                    continue;
                };
                if !streaming && let Some(first) = art.warnings.first() {
                    let more = match art.warnings.len() {
                        1 => String::new(),
                        count => format!(" (+{} more)", count - 1),
                    };
                    let warning = format!("Mermaid diagram not rendered: {first}{more}");
                    let warning = match theme {
                        Some(theme) => fg(theme, "warning", &warning),
                        None => warning,
                    };
                    out.push_str(&format!("{raw}\n{}  \n", code_span(&warning)));
                    continue;
                }
                let lines = match theme {
                    Some(theme) => themed_lines(&art, theme),
                    None => art.plain.clone(),
                };
                let rows: Vec<String> = lines.iter().map(|line| code_span(line)).collect();
                out.push_str(&rows.join("  \n"));
                out.push('\n');
            }
            _ => {}
        }
    }
    out.push_str(&markdown[at..]);
    out
}
