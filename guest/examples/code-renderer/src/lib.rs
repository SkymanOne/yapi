//! An example native extension: a code renderer, yapi's own API, that colors
//! Rust keywords in code blocks and code tool rows with the theme's keyword
//! color, and leaves other languages to yapi. A Markdown transformer makes
//! each `TODO` in messages bold.

use yapi_extension_api::{Api, theme};

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "const", "else", "enum", "fn", "for", "if", "impl", "let", "loop",
    "match", "mod", "move", "mut", "pub", "return", "self", "Self", "static", "struct", "trait",
    "type", "use", "where", "while",
];

/// `line` with each keyword in `paint`.
fn color_keywords(line: &str, paint: impl Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for c in line.chars().chain(std::iter::once('\n')) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        if KEYWORDS.contains(&word.as_str()) {
            out.push_str(&paint(&word));
        } else {
            out.push_str(&word);
        }
        word.clear();
        if c != '\n' {
            out.push(c);
        }
    }
    out
}

fn init(api: &mut Api) {
    api.register_markdown_transformer(|markdown, _context| {
        markdown
            .contains("TODO")
            .then(|| markdown.replace("TODO", "**TODO**"))
    });
    api.register_code_renderer(|code, lang, _width| {
        if lang != Some("rust") {
            return None;
        }
        let theme = theme();
        let paint = |word: &str| theme.fg("syntaxKeyword", word);
        Some(
            code.split('\n')
                .map(|line| color_keywords(line, paint))
                .collect(),
        )
    });
}

yapi_extension_api::extension!(init);
