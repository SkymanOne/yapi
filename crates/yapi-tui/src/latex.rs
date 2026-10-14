//! Renders basic LaTeX math as terminal-friendly Unicode text.
//!
//! A port of `packages/tui/src/latex.ts` in pi `v1.0.0`. Expressions map to
//! Unicode symbols, super- and subscripts, and linear fractions. In display
//! mode, fractions, operator limits, unsupported scripts, matrices and `cases`
//! environments are laid out over several lines. Unsupported or malformed
//! input yields `None`.

mod letters;

use crate::text::{is_js_whitespace as is_space, visible_width};

fn symbol(key: &str) -> Option<&'static str> {
    Some(match key {
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ϵ",
        "varepsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "vartheta" => "ϑ",
        "iota" => "ι",
        "kappa" => "κ",
        "varkappa" => "ϰ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "varpi" => "ϖ",
        "rho" => "ρ",
        "varrho" => "ϱ",
        "sigma" => "σ",
        "varsigma" => "ς",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "ϕ",
        "varphi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Upsilon" => "Υ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        "pm" => "±",
        "mp" => "∓",
        "times" => "×",
        "div" => "÷",
        "cdot" => "·",
        "ast" => "∗",
        "star" => "⋆",
        "circ" => "∘",
        "bullet" => "•",
        "oplus" => "⊕",
        "ominus" => "⊖",
        "otimes" => "⊗",
        "oslash" => "⊘",
        "odot" => "⊙",
        "bigcirc" => "○",
        "dagger" => "†",
        "ddagger" => "‡",
        "amalg" => "⨿",
        "uplus" => "⊎",
        "sqcap" => "⊓",
        "sqcup" => "⊔",
        "bowtie" => "⋈",
        "Join" => "⋈",
        "ltimes" => "⋉",
        "rtimes" => "⋊",
        "leftouterjoin" => "⟕",
        "rightouterjoin" => "⟖",
        "fullouterjoin" => "⟗",
        "triangleleft" => "◁",
        "triangleright" => "▷",
        "wr" => "≀",
        "cap" => "∩",
        "cup" => "∪",
        "bigcap" => "⋂",
        "bigcup" => "⋃",
        "bigwedge" => "⋀",
        "bigvee" => "⋁",
        "bigsqcup" => "⨆",
        "biguplus" => "⨄",
        "bigoplus" => "⨁",
        "bigotimes" => "⨂",
        "bigodot" => "⨀",
        "setminus" => "∖",
        "in" => "∈",
        "notin" => "∉",
        "ni" => "∋",
        "subset" => "⊂",
        "supset" => "⊃",
        "subseteq" => "⊆",
        "supseteq" => "⊇",
        "sqsubset" => "⊏",
        "sqsupset" => "⊐",
        "sqsubseteq" => "⊑",
        "sqsupseteq" => "⊒",
        "prec" => "≺",
        "preceq" => "≼",
        "succ" => "≻",
        "succeq" => "≽",
        "ll" => "≪",
        "gg" => "≫",
        "le" => "≤",
        "leq" => "≤",
        "leqslant" => "≤",
        "ge" => "≥",
        "geq" => "≥",
        "geqslant" => "≥",
        "ne" => "≠",
        "neq" => "≠",
        "equiv" => "≡",
        "approx" => "≈",
        "sim" => "∼",
        "simeq" => "≃",
        "cong" => "≅",
        "asymp" => "≍",
        "doteq" => "≐",
        "propto" => "∝",
        "parallel" => "∥",
        "perp" => "⊥",
        "mid" => "∣",
        "vdash" => "⊢",
        "dashv" => "⊣",
        "models" => "⊨",
        "Vdash" => "⊩",
        "Vvdash" => "⊪",
        "nvdash" => "⊬",
        "nvDash" => "⊭",
        "forall" => "∀",
        "exists" => "∃",
        "nexists" => "∄",
        "neg" => "¬",
        "land" => "∧",
        "wedge" => "∧",
        "lor" => "∨",
        "vee" => "∨",
        "to" => "→",
        "rightarrow" => "→",
        "longrightarrow" => "→",
        "leftarrow" => "←",
        "longleftarrow" => "←",
        "gets" => "←",
        "leftrightarrow" => "↔",
        "longleftrightarrow" => "↔",
        "hookleftarrow" => "↩",
        "hookrightarrow" => "↪",
        "twoheadleftarrow" => "↞",
        "twoheadrightarrow" => "↠",
        "leftharpoonup" => "↼",
        "leftharpoondown" => "↽",
        "rightharpoonup" => "⇀",
        "rightharpoondown" => "⇁",
        "rightleftharpoons" => "⇌",
        "leftrightharpoons" => "⇋",
        "nearrow" => "↗",
        "searrow" => "↘",
        "swarrow" => "↙",
        "nwarrow" => "↖",
        "rightsquigarrow" => "⇝",
        "leadsto" => "⇝",
        "Rightarrow" => "⇒",
        "Longrightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "Longleftarrow" => "⇐",
        "Leftrightarrow" => "⇔",
        "Longleftrightarrow" => "⇔",
        "implies" => "⇒",
        "iff" => "⇔",
        "mapsto" => "↦",
        "longmapsto" => "↦",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "partial" => "∂",
        "nabla" => "∇",
        "int" => "∫",
        "iint" => "∬",
        "iiint" => "∭",
        "oint" => "∮",
        "sum" => "∑",
        "prod" => "∏",
        "coprod" => "∐",
        "infty" => "∞",
        "emptyset" => "∅",
        "varnothing" => "∅",
        "angle" => "∠",
        "therefore" => "∴",
        "because" => "∵",
        "aleph" => "ℵ",
        "beth" => "ℶ",
        "gimel" => "ℷ",
        "daleth" => "ℸ",
        "top" => "⊤",
        "bot" => "⊥",
        "triangle" => "△",
        "square" => "□",
        "lozenge" => "◊",
        "checkmark" => "✓",
        "complement" => "∁",
        "wp" => "℘",
        "prime" => "′",
        "ldots" => "…",
        "dots" => "…",
        "cdots" => "⋯",
        "vdots" => "⋮",
        "ddots" => "⋱",
        "ell" => "ℓ",
        "hbar" => "ℏ",
        "Im" => "ℑ",
        "Re" => "ℜ",
        "langle" => "⟨",
        "rangle" => "⟩",
        "vert" => "|",
        "lvert" => "|",
        "rvert" => "|",
        "Vert" => "‖",
        "lVert" => "‖",
        "rVert" => "‖",
        "lbrace" => "{",
        "rbrace" => "}",
        "backslash" => "\\",
        "lfloor" => "⌊",
        "rfloor" => "⌋",
        "lceil" => "⌈",
        "rceil" => "⌉",
        "colon" => ":",
        _ => return None,
    })
}

fn negated_symbol(key: &str) -> Option<&'static str> {
    Some(match key {
        "<" => "≮",
        ">" => "≯",
        "=" => "≠",
        "∈" => "∉",
        "∋" => "∌",
        "∣" => "∤",
        "∥" => "∦",
        "∼" => "≁",
        "≃" => "≄",
        "≅" => "≇",
        "≈" => "≉",
        "≡" => "≢",
        "≤" => "≰",
        "≥" => "≱",
        "≺" => "⊀",
        "≻" => "⊁",
        "⊂" => "⊄",
        "⊃" => "⊅",
        "⊆" => "⊈",
        "⊇" => "⊉",
        "⊢" => "⊬",
        "⊨" => "⊭",
        "↔" => "↮",
        "←" => "↚",
        "→" => "↛",
        "⇒" => "⇏",
        "⇐" => "⇍",
        "⇔" => "⇎",
        "≼" => "⋠",
        "≽" => "⋡",
        _ => return None,
    })
}

fn blackboard(key: char) -> Option<&'static str> {
    Some(match key {
        'C' => "ℂ",
        'H' => "ℍ",
        'N' => "ℕ",
        'P' => "ℙ",
        'Q' => "ℚ",
        'R' => "ℝ",
        'Z' => "ℤ",
        _ => return None,
    })
}

fn superscript(key: char) -> Option<&'static str> {
    Some(match key {
        '0' => "⁰",
        '1' => "¹",
        '2' => "²",
        '3' => "³",
        '4' => "⁴",
        '5' => "⁵",
        '6' => "⁶",
        '7' => "⁷",
        '8' => "⁸",
        '9' => "⁹",
        '+' => "⁺",
        '-' => "⁻",
        '=' => "⁼",
        '(' => "⁽",
        ')' => "⁾",
        'a' => "ᵃ",
        'b' => "ᵇ",
        'c' => "ᶜ",
        'd' => "ᵈ",
        'e' => "ᵉ",
        'f' => "ᶠ",
        'g' => "ᵍ",
        'h' => "ʰ",
        'i' => "ⁱ",
        'j' => "ʲ",
        'k' => "ᵏ",
        'l' => "ˡ",
        'm' => "ᵐ",
        'n' => "ⁿ",
        'o' => "ᵒ",
        'p' => "ᵖ",
        'r' => "ʳ",
        's' => "ˢ",
        't' => "ᵗ",
        'u' => "ᵘ",
        'v' => "ᵛ",
        'w' => "ʷ",
        'x' => "ˣ",
        'y' => "ʸ",
        'z' => "ᶻ",
        _ => return None,
    })
}

fn subscript(key: char) -> Option<&'static str> {
    Some(match key {
        '0' => "₀",
        '1' => "₁",
        '2' => "₂",
        '3' => "₃",
        '4' => "₄",
        '5' => "₅",
        '6' => "₆",
        '7' => "₇",
        '8' => "₈",
        '9' => "₉",
        '+' => "₊",
        '-' => "₋",
        '=' => "₌",
        '(' => "₍",
        ')' => "₎",
        'a' => "ₐ",
        'e' => "ₑ",
        'h' => "ₕ",
        'i' => "ᵢ",
        'j' => "ⱼ",
        'k' => "ₖ",
        'l' => "ₗ",
        'm' => "ₘ",
        'n' => "ₙ",
        'o' => "ₒ",
        'p' => "ₚ",
        'r' => "ᵣ",
        's' => "ₛ",
        't' => "ₜ",
        'u' => "ᵤ",
        'v' => "ᵥ",
        'x' => "ₓ",
        _ => return None,
    })
}

fn accent(key: &str) -> Option<&'static str> {
    Some(match key {
        "acute" => "\u{0301}",
        "bar" => "\u{0305}",
        "breve" => "\u{0306}",
        "check" => "\u{030c}",
        "ddot" => "\u{0308}",
        "dot" => "\u{0307}",
        "grave" => "\u{0300}",
        "hat" => "\u{0302}",
        "mathring" => "\u{030a}",
        "overleftarrow" => "\u{20d6}",
        "overleftrightarrow" => "\u{20e1}",
        "overline" => "\u{0305}",
        "overrightarrow" => "\u{20d7}",
        "tilde" => "\u{0303}",
        "underline" => "\u{0332}",
        "vec" => "\u{20d7}",
        "widehat" => "\u{0302}",
        "widetilde" => "\u{0303}",
        _ => return None,
    })
}

const NAMED_OPERATORS: &[&str] = &[
    "arccos", "arcsin", "arctan", "arg", "cos", "cosh", "cot", "coth", "csc", "deg", "det", "dim",
    "exp", "gcd", "hom", "inf", "ker", "lg", "lim", "liminf", "limsup", "ln", "log", "max", "min",
    "Pr", "sec", "sin", "sinh", "sup", "tan", "tanh",
];
const LIMIT_OPERATORS: &[&str] = &[
    "argmax", "argmin", "inf", "injlim", "lim", "liminf", "limsup", "max", "min", "projlim", "sup",
];
const DISPLAY_LIMIT_SYMBOLS: &[&str] = &[
    "bigcap",
    "bigcup",
    "bigodot",
    "bigoplus",
    "bigotimes",
    "bigsqcup",
    "biguplus",
    "bigvee",
    "bigwedge",
    "coprod",
    "int",
    "iint",
    "iiint",
    "oint",
    "prod",
    "sum",
];
const RELATION_COMMANDS: &[&str] = &[
    "Leftarrow",
    "Leftrightarrow",
    "Longleftarrow",
    "Longleftrightarrow",
    "Longrightarrow",
    "Rightarrow",
    "Join",
    "Vdash",
    "Vvdash",
    "approx",
    "asymp",
    "bowtie",
    "cong",
    "dashv",
    "fullouterjoin",
    "doteq",
    "downarrow",
    "equiv",
    "ge",
    "geq",
    "geqslant",
    "gets",
    "gg",
    "hookleftarrow",
    "hookrightarrow",
    "iff",
    "implies",
    "in",
    "leadsto",
    "le",
    "leftarrow",
    "leftharpoondown",
    "leftharpoonup",
    "leftrightarrow",
    "leftrightharpoons",
    "leftouterjoin",
    "leq",
    "leqslant",
    "ll",
    "longleftarrow",
    "longleftrightarrow",
    "longmapsto",
    "longrightarrow",
    "ltimes",
    "mapsto",
    "mid",
    "models",
    "ne",
    "nearrow",
    "neq",
    "ni",
    "notin",
    "nvdash",
    "nvDash",
    "nwarrow",
    "parallel",
    "perp",
    "prec",
    "preceq",
    "propto",
    "rightharpoondown",
    "rightharpoonup",
    "rightleftharpoons",
    "rightouterjoin",
    "rightarrow",
    "rightsquigarrow",
    "rtimes",
    "searrow",
    "sim",
    "simeq",
    "sqsubset",
    "sqsubseteq",
    "sqsupset",
    "sqsupseteq",
    "subset",
    "subseteq",
    "succ",
    "succeq",
    "supset",
    "supseteq",
    "swarrow",
    "to",
    "triangleleft",
    "triangleright",
    "twoheadleftarrow",
    "twoheadrightarrow",
    "uparrow",
    "vdash",
];
const SPACING_COMMANDS: &[&str] = &[
    ",",
    ":",
    ";",
    " ",
    ">",
    "enspace",
    "enskip",
    "medspace",
    "quad",
    "qquad",
    "thickspace",
    "thinspace",
];
const NEGATIVE_SPACING_COMMANDS: &[&str] = &["!", "negmedspace", "negthickspace", "negthinspace"];
const FONT_SWITCH_COMMANDS: &[&str] = &["bf", "cal", "it", "rm", "sf", "sl", "tt"];
const IGNORED_COMMANDS: &[&str] = &[
    "displaystyle",
    "limits",
    "nolimits",
    "scriptstyle",
    "scriptscriptstyle",
    "textstyle",
];
const SIZE_COMMANDS: &[&str] = &[
    "big", "Big", "bigg", "Bigg", "bigl", "Bigl", "biggl", "Biggl", "bigr", "Bigr", "biggr",
    "Biggr",
];
const PLAIN_WRAPPERS: &[&str] = &[
    "emph",
    "mathcal",
    "mathbf",
    "mathfrak",
    "mathit",
    "mathrm",
    "mathnormal",
    "mathscr",
    "mathsf",
    "mathtt",
    "mathup",
    "mbox",
    "overbrace",
    "pmb",
    "smash",
    "substack",
    "text",
    "textbf",
    "textit",
    "textmd",
    "textnormal",
    "textrm",
    "textsc",
    "textsf",
    "textsl",
    "texttt",
    "textup",
    "underbrace",
    "bm",
    "boldsymbol",
];

const NEGATIVE_SPACE: &str = "\u{0000}";

/// JS `String.prototype.trim`.
fn trim(value: &str) -> &str {
    value.trim_matches(is_space)
}

fn trim_start(value: &str) -> &str {
    value.trim_start_matches(is_space)
}

fn trim_end(value: &str) -> &str {
    value.trim_end_matches(is_space)
}

/// `\p{L}`.
fn is_letter(c: char) -> bool {
    let c = u32::from(c);
    letters::LETTERS
        .binary_search_by(|&(lo, hi)| {
            if hi < c {
                std::cmp::Ordering::Less
            } else if lo > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// JS `String.prototype.slice(start, end)` on a code point sequence.
fn slice(chars: &[char], start: usize, end: usize) -> String {
    let end = end.min(chars.len());
    let start = start.min(end);
    chars[start..end].iter().collect()
}

fn find_chars(haystack: &[char], needle: &[char], from: usize) -> Option<usize> {
    if from > haystack.len() || needle.len() > haystack.len() - from {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| haystack[i..i + needle.len()] == *needle)
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

fn replace_characters(
    value: &str,
    replacements: fn(char) -> Option<&'static str>,
) -> Option<String> {
    let mut result = String::new();
    for character in value.chars() {
        result.push_str(replacements(character)?);
    }
    Some(result)
}

/// `value.trim().replace(/\s*([=+-])\s*/g, "$1")`.
fn normalize_script_value(value: &str) -> String {
    let chars: Vec<char> = trim(value).chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        let mut j = i;
        while j < chars.len() && is_space(chars[j]) {
            j += 1;
        }
        if j < chars.len() && matches!(chars[j], '=' | '+' | '-') {
            result.push(chars[j]);
            i = j + 1;
            while i < chars.len() && is_space(chars[i]) {
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScriptKind {
    Sub,
    Sup,
}

fn format_unicode_script(value: &str, kind: ScriptKind) -> Option<String> {
    replace_characters(
        &normalize_script_value(value),
        if kind == ScriptKind::Sub {
            subscript
        } else {
            superscript
        },
    )
}

fn format_script(value: &str, kind: ScriptKind) -> String {
    let value = normalize_script_value(value);
    if let Some(unicode) = format_unicode_script(&value, kind) {
        return unicode;
    }

    let prefix = if kind == ScriptKind::Sub { "_" } else { "^" };
    if char_count(&value) == 1
        || (kind == ScriptKind::Sub
            && !value.is_empty()
            && value.chars().all(|c| c.is_ascii_alphabetic()))
    {
        return format!("{prefix}{value}");
    }
    format!("{prefix}({value})")
}

/// `/^[\p{L}\p{N}.]+$/u`.
fn is_simple_term(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| is_letter(c) || c.is_numeric() || c == '.')
}

fn format_fraction(numerator: &str, denominator: &str) -> String {
    let numerator = trim(numerator);
    let denominator = trim(denominator);
    let simple_numerator = is_simple_term(numerator);
    let simple_denominator = (!denominator.is_empty()
        && denominator.chars().all(|c| c.is_numeric() || c == '.'))
        || char_count(denominator) == 1;
    format!(
        "{}/{}",
        if simple_numerator {
            numerator.to_string()
        } else {
            format!("({numerator})")
        },
        if simple_denominator {
            denominator.to_string()
        } else {
            format!("({denominator})")
        },
    )
}

fn format_root(value: &str, symbol: &str) -> String {
    let value = trim(value);
    if is_simple_term(value) {
        format!("{symbol}{value}")
    } else {
        format!("{symbol}({value})")
    }
}

const NAMED_OPERATOR_START: char = '\u{f0004}';
const NAMED_OPERATOR_END: char = '\u{f0005}';

/// The class `[\p{L}\p{N})\]}\u{f0001}]` that gates the left spacing of a named operator.
fn is_named_operator_left_context(c: char) -> bool {
    is_letter(c) || c.is_numeric() || matches!(c, ')' | ']' | '}' | '\u{f0001}')
}

/// The class `[\p{L}\p{N}√\u{f0000}]` that gates the right spacing of a named operator.
fn is_named_operator_right_context(c: char) -> bool {
    is_letter(c) || c.is_numeric() || matches!(c, '√' | '\u{f0000}')
}

fn normalize_output(value: &str) -> String {
    // Left spacing: `(?<=[...])\u{f0004}` -> " ", judged against the input.
    let chars: Vec<char> = value.chars().collect();
    let mut left_spaced = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == NAMED_OPERATOR_START && i > 0 && is_named_operator_left_context(chars[i - 1]) {
            left_spaced.push(' ');
        } else {
            left_spaced.push(c);
        }
    }
    let without_start: Vec<char> = left_spaced
        .chars()
        .filter(|&c| c != NAMED_OPERATOR_START)
        .collect();
    // Right spacing: `\u{f0005}(?=[...])` -> " ".
    let mut right_spaced = String::new();
    for (i, &c) in without_start.iter().enumerate() {
        if c == NAMED_OPERATOR_END
            && without_start
                .get(i + 1)
                .is_some_and(|&next| is_named_operator_right_context(next))
        {
            right_spaced.push(' ');
        } else {
            right_spaced.push(c);
        }
    }
    let without_end: String = right_spaced
        .chars()
        .filter(|&c| c != NAMED_OPERATOR_END)
        .collect();

    let lines: Vec<String> = without_end
        .split('\n')
        .map(|line| trim(&collapse_blanks(line)).to_string())
        .collect();
    let count = lines.len();
    let joined = lines
        .into_iter()
        .enumerate()
        .filter(|(index, line)| !line.is_empty() || (*index > 0 && *index + 1 < count))
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    trim(&joined).to_string()
}

/// `line.replace(/[ \t]+/g, " ")`.
fn collapse_blanks(line: &str) -> String {
    let mut result = String::new();
    let mut in_run = false;
    for c in line.chars() {
        if is_blank(c) {
            if !in_run {
                result.push(' ');
            }
            in_run = true;
        } else {
            result.push(c);
            in_run = false;
        }
    }
    result
}

enum LayoutNode {
    Fraction {
        numerator: String,
        denominator: String,
    },
    Operator {
        operator: String,
        lower: Option<String>,
        upper: Option<String>,
    },
    Script {
        lower: Option<String>,
        upper: Option<String>,
    },
    Matrix {
        lines: Vec<String>,
        baseline: usize,
    },
}

struct Layout {
    lines: Vec<String>,
    width: usize,
    baseline: usize,
}

const LAYOUT_MARKER_START: char = '\u{f0000}';
const LAYOUT_MARKER_END: char = '\u{f0001}';
const PROTECTED_SPACE: &str = "\u{f0002}";

fn layout_marker(index: usize) -> String {
    format!("{LAYOUT_MARKER_START}{index}{LAYOUT_MARKER_END}")
}

/// Finds the next `\u{f0000}(\d+)\u{f0001}` at or after `from`, as (start, end, digits).
fn find_layout_marker(chars: &[char], from: usize) -> Option<(usize, usize, String)> {
    let mut start = from;
    while start < chars.len() {
        if chars[start] == LAYOUT_MARKER_START {
            let mut end = start + 1;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            if end > start + 1 && end < chars.len() && chars[end] == LAYOUT_MARKER_END {
                return Some((start, end + 1, chars[start + 1..end].iter().collect()));
            }
        }
        start += 1;
    }
    None
}

/// Index of a layout marker that ends `value`, i.e. `/\u{f0000}(\d+)\u{f0001}$/u`.
fn trailing_layout_marker(value: &str) -> Option<usize> {
    let chars: Vec<char> = value.chars().collect();
    if chars.last() != Some(&LAYOUT_MARKER_END) {
        return None;
    }
    let mut start = chars.len() - 1;
    while start > 0 && chars[start - 1].is_ascii_digit() {
        start -= 1;
    }
    if start == chars.len() - 1 || start == 0 || chars[start - 1] != LAYOUT_MARKER_START {
        return None;
    }
    chars[start..chars.len() - 1]
        .iter()
        .collect::<String>()
        .parse()
        .ok()
}

fn pad_layout_line(line: &str, width: usize, centered: bool) -> String {
    let padding = width.saturating_sub(visible_width(line));
    let left = if centered { padding / 2 } else { 0 };
    format!("{}{}{}", " ".repeat(left), line, " ".repeat(padding - left))
}

fn join_layouts(layouts: &[Layout]) -> Layout {
    if layouts.is_empty() {
        return Layout {
            lines: vec![String::new()],
            width: 0,
            baseline: 0,
        };
    }
    let baseline = layouts
        .iter()
        .map(|layout| layout.baseline)
        .max()
        .unwrap_or(0);
    let below = layouts
        .iter()
        .map(|layout| layout.lines.len() as i64 - layout.baseline as i64 - 1)
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();
    let mut row: i64 = 0;
    while row <= baseline as i64 + below {
        let mut line = String::new();
        for layout in layouts {
            let source_row = row - baseline as i64 + layout.baseline as i64;
            if source_row >= 0 && (source_row as usize) < layout.lines.len() {
                line.push_str(&pad_layout_line(
                    &layout.lines[source_row as usize],
                    layout.width,
                    false,
                ));
            } else {
                line.push_str(&" ".repeat(layout.width));
            }
        }
        lines.push(trim_end(&line).to_string());
        row += 1;
    }
    Layout {
        lines,
        width: layouts.iter().map(|layout| layout.width).sum(),
        baseline,
    }
}

fn render_layout(source: &str, nodes: &[LayoutNode]) -> Layout {
    let mut rendered_lines: Vec<String> = Vec::new();
    let mut first_baseline = 0;
    for source_line in source.split('\n') {
        let source_line: Vec<char> = source_line.chars().collect();
        let mut layouts: Vec<Layout> = Vec::new();
        let mut position = 0;
        let mut previous_node: Option<&LayoutNode> = None;
        let mut search_from = 0;
        while let Some((index, match_end, digits)) = find_layout_marker(&source_line, search_from) {
            search_from = match_end;
            let Some(node) = digits.parse::<usize>().ok().and_then(|n| nodes.get(n)) else {
                continue;
            };
            if index > position {
                let sliced = slice(&source_line, position, index);
                let trimmed = trim_end(if previous_node.is_some() {
                    trim_start(&sliced)
                } else {
                    sliced.as_str()
                });
                let preserve_leading_space =
                    matches!(previous_node, Some(LayoutNode::Matrix { .. }))
                        && sliced.starts_with(is_space);
                let preserve_trailing_space =
                    matches!(node, LayoutNode::Matrix { .. }) && sliced.ends_with(is_space);
                let text = if !trimmed.is_empty() {
                    format!(
                        "{}{}{}",
                        if preserve_leading_space { " " } else { "" },
                        trimmed,
                        if preserve_trailing_space { " " } else { "" }
                    )
                } else if preserve_leading_space || preserve_trailing_space {
                    " ".to_string()
                } else {
                    String::new()
                };
                layouts.push(Layout {
                    width: visible_width(&text),
                    lines: vec![text],
                    baseline: 0,
                });
            }
            match node {
                LayoutNode::Fraction {
                    numerator,
                    denominator,
                } => {
                    let numerator = render_layout(numerator, nodes);
                    let denominator = render_layout(denominator, nodes);
                    let content_width = numerator.width.max(denominator.width).max(1);
                    let width = content_width + 2;
                    let mut lines: Vec<String> = numerator
                        .lines
                        .iter()
                        .map(|line| pad_layout_line(line, width, true))
                        .collect();
                    lines.push(format!(" {} ", "─".repeat(content_width)));
                    lines.extend(
                        denominator
                            .lines
                            .iter()
                            .map(|line| pad_layout_line(line, width, true)),
                    );
                    layouts.push(Layout {
                        lines,
                        width,
                        baseline: numerator.lines.len(),
                    });
                }
                LayoutNode::Operator {
                    operator,
                    lower,
                    upper,
                } => {
                    let content_width = visible_width(operator)
                        .max(lower.as_deref().map_or(0, visible_width))
                        .max(upper.as_deref().map_or(0, visible_width));
                    let mut lines = Vec::new();
                    if let Some(upper) = upper {
                        lines.push(format!("{} ", pad_layout_line(upper, content_width, true)));
                    }
                    lines.push(format!(
                        "{} ",
                        pad_layout_line(operator, content_width, true)
                    ));
                    if let Some(lower) = lower {
                        lines.push(format!("{} ", pad_layout_line(lower, content_width, true)));
                    }
                    layouts.push(Layout {
                        lines,
                        width: content_width + 1,
                        baseline: if upper.is_none() { 0 } else { 1 },
                    });
                }
                LayoutNode::Script { lower, upper } => {
                    let upper = upper.as_deref().map(|upper| render_layout(upper, nodes));
                    let lower = lower.as_deref().map(|lower| render_layout(lower, nodes));
                    let width = upper
                        .as_ref()
                        .map_or(0, |l| l.width)
                        .max(lower.as_ref().map_or(0, |l| l.width));
                    let mut lines: Vec<String> = Vec::new();
                    if let Some(upper) = &upper {
                        lines.extend(
                            upper
                                .lines
                                .iter()
                                .map(|line| pad_layout_line(line, width, false)),
                        );
                    }
                    lines.push(" ".repeat(width));
                    if let Some(lower) = &lower {
                        lines.extend(
                            lower
                                .lines
                                .iter()
                                .map(|line| pad_layout_line(line, width, false)),
                        );
                    }
                    layouts.push(Layout {
                        lines,
                        width,
                        baseline: upper.as_ref().map_or(0, |l| l.lines.len()),
                    });
                }
                LayoutNode::Matrix { lines, baseline } => {
                    let width = lines
                        .iter()
                        .map(|line| visible_width(line))
                        .max()
                        .unwrap_or(0);
                    layouts.push(Layout {
                        lines: lines
                            .iter()
                            .map(|line| pad_layout_line(line, width, false))
                            .collect(),
                        width,
                        baseline: *baseline,
                    });
                }
            }
            position = match_end;
            previous_node = Some(node);
        }
        if position < source_line.len() {
            let sliced = slice(&source_line, position, source_line.len());
            let trimmed = if previous_node.is_some() {
                trim_start(&sliced)
            } else {
                sliced.as_str()
            };
            let text = if matches!(previous_node, Some(LayoutNode::Matrix { .. }))
                && sliced.starts_with(is_space)
            {
                format!(" {trimmed}")
            } else {
                trimmed.to_string()
            };
            layouts.push(Layout {
                width: visible_width(&text),
                lines: vec![text],
                baseline: 0,
            });
        }
        let line_layout = join_layouts(&layouts);
        if rendered_lines.is_empty() {
            first_baseline = line_layout.baseline;
        }
        rendered_lines.extend(line_layout.lines);
    }
    Layout {
        width: rendered_lines
            .iter()
            .map(|line| visible_width(line))
            .max()
            .unwrap_or(0),
        lines: rendered_lines,
        baseline: first_baseline,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LowerStyle {
    Bracket,
    Script,
}

/// How deep groups, arguments and environments may nest. pi recurses until
/// the JavaScript stack overflows, and then shows the source as written. The
/// bound keeps model text from overflowing the thread's stack.
const MAX_DEPTH: usize = 128;

struct LatexParser<'a> {
    source: Vec<char>,
    layout_nodes: &'a mut Vec<LayoutNode>,
    display: bool,
    position: usize,
    supported: bool,
    stack_fractions: bool,
    script_depth: usize,
    /// Nesting depth, shared with the parsers of nested sources.
    depth: usize,
}

impl<'a> LatexParser<'a> {
    fn new(source: &str, layout_nodes: &'a mut Vec<LayoutNode>, display: bool) -> Self {
        Self {
            source: source.chars().collect(),
            layout_nodes,
            display,
            position: 0,
            supported: true,
            stack_fractions: true,
            script_depth: 0,
            depth: 0,
        }
    }

    /// Runs `parse` one level deeper. Past [`MAX_DEPTH`] the source is
    /// unsupported and the rest of it is skipped.
    fn deeper<T: Default>(&mut self, parse: impl FnOnce(&mut Self) -> T) -> T {
        if self.depth >= MAX_DEPTH {
            self.supported = false;
            self.position = self.source.len();
            return T::default();
        }
        self.depth += 1;
        let value = parse(self);
        self.depth -= 1;
        value
    }

    /// `this.source[index]`.
    fn at(&self, index: usize) -> Option<char> {
        self.source.get(index).copied()
    }

    fn skip_while(&mut self, predicate: fn(char) -> bool) {
        while self.position < self.source.len() && predicate(self.source[self.position]) {
            self.position += 1;
        }
    }

    fn render(&mut self) -> Option<String> {
        let rendered = self.parse_sequence(None);
        if !self.supported || self.position != self.source.len() {
            return None;
        }
        Some(normalize_output(&rendered))
    }

    fn parse_sequence(&mut self, end_character: Option<char>) -> String {
        let mut result = String::new();
        while self.position < self.source.len() {
            let character = self.source[self.position];
            if end_character == Some(character) {
                self.position += 1;
                return result;
            }

            if character == '}' {
                self.supported = false;
                return result;
            }

            if character == '{' {
                self.position += 1;
                result.push_str(&self.deeper(|parser| parser.parse_sequence(Some('}'))));
                continue;
            }

            if character == '\\' {
                let command = self.parse_command();
                if command == NEGATIVE_SPACE {
                    result = trim_end(&result).to_string();
                    if result.ends_with(NAMED_OPERATOR_END) {
                        result.pop();
                    }
                } else {
                    result.push_str(&command);
                }
                continue;
            }

            if character == '^' || character == '_' {
                self.position += 1;
                result = trim_end(&result).to_string();
                let script = self.parse_scripts(character);
                if result.ends_with(NAMED_OPERATOR_END) {
                    result.pop();
                    result.push_str(&script);
                    result.push(NAMED_OPERATOR_END);
                } else {
                    result.push_str(&script);
                }
                continue;
            }

            if is_space(character) {
                result.push_str(&self.parse_whitespace());
                continue;
            }

            if character == '=' || character == '<' || character == '>' {
                result = format!("{} {} ", trim_end(&result), character);
                self.position += 1;
                continue;
            }

            if character == '&' {
                self.position += 1;
                continue;
            }

            if character == '~' {
                self.position += 1;
                result.push(' ');
                continue;
            }

            if character == '.'
                && let Some(index) = trailing_layout_marker(&result)
                && let Some(LayoutNode::Matrix { lines, .. }) = self.layout_nodes.get_mut(index)
            {
                if let Some(last_line) = lines.last_mut() {
                    last_line.push(character);
                }
                self.position += 1;
                continue;
            }

            result.push(character);
            self.position += 1;
        }

        if end_character.is_some() {
            self.supported = false;
        }
        result
    }

    fn parse_script(&mut self) -> String {
        self.script_depth += 1;
        let value = self.parse_required_argument(false);
        self.script_depth -= 1;
        value
    }

    fn parse_scripts(&mut self, initial_marker: char) -> String {
        let mut sub: Option<String> = None;
        let mut sup: Option<String> = None;
        let mut order: Vec<ScriptKind> = Vec::new();

        let value = self.parse_script();
        if initial_marker == '_' {
            sub = Some(value);
            order.push(ScriptKind::Sub);
        } else {
            sup = Some(value);
            order.push(ScriptKind::Sup);
        }
        let mut next_position = self.position;
        while next_position < self.source.len() && is_space(self.source[next_position]) {
            next_position += 1;
        }
        let next_marker = self.at(next_position);
        if let Some(next_marker) = next_marker
            && (next_marker == '^' || next_marker == '_')
            && next_marker != initial_marker
        {
            self.position = next_position + 1;
            let value = self.parse_script();
            if next_marker == '_' {
                sub = Some(value);
                order.push(ScriptKind::Sub);
            } else {
                sup = Some(value);
                order.push(ScriptKind::Sup);
            }
        }

        let sub_unicode = sub
            .as_deref()
            .and_then(|value| format_unicode_script(value, ScriptKind::Sub));
        let sup_unicode = sup
            .as_deref()
            .and_then(|value| format_unicode_script(value, ScriptKind::Sup));
        let can_use_layout = ![sub.as_deref(), sup.as_deref()]
            .iter()
            .flatten()
            .any(|value| {
                value.contains('/')
                    || (!value.contains(LAYOUT_MARKER_START)
                        && char_count(value) > 1
                        && !value
                            .chars()
                            .any(|c| c.is_ascii_uppercase() || c == '*' || c == '∗'))
            });
        let needs_layout = self.display
            && can_use_layout
            && (self.script_depth > 0
                || (sub.is_some() && sub_unicode.is_none())
                || (sup.is_some() && sup_unicode.is_none()));
        if !needs_layout {
            return order
                .iter()
                .map(|kind| match kind {
                    ScriptKind::Sub => sub_unicode
                        .clone()
                        .unwrap_or_else(|| format_script(sub.as_deref().unwrap_or(""), *kind)),
                    ScriptKind::Sup => sup_unicode
                        .clone()
                        .unwrap_or_else(|| format_script(sup.as_deref().unwrap_or(""), *kind)),
                })
                .collect();
        }

        self.layout_nodes.push(LayoutNode::Script {
            lower: sub.as_deref().map(normalize_output),
            upper: sup.as_deref().map(normalize_output),
        });
        layout_marker(self.layout_nodes.len() - 1)
    }

    fn parse_whitespace(&mut self) -> String {
        self.skip_while(is_space);
        " ".to_string()
    }

    fn parse_command(&mut self) -> String {
        self.position += 1;
        if self.position >= self.source.len() {
            self.supported = false;
            return String::new();
        }

        let first = self.at(self.position).unwrap_or('\0');
        if first == '\n' || first == '\r' {
            self.position += 1;
            if first == '\r' && self.at(self.position) == Some('\n') {
                self.position += 1;
            }
            return " ".to_string();
        }
        let command: String;
        if first.is_ascii_alphabetic() {
            let start = self.position;
            self.skip_while(|c| c.is_ascii_alphabetic());
            command = slice(&self.source, start, self.position);
        } else {
            command = first.to_string();
            self.position += 1;
        }
        let command = command.as_str();

        if command == "\\" {
            return "\n".to_string();
        }
        if SPACING_COMMANDS.contains(&command) {
            return " ".to_string();
        }
        if NEGATIVE_SPACING_COMMANDS.contains(&command) {
            return NEGATIVE_SPACE.to_string();
        }
        if FONT_SWITCH_COMMANDS.contains(&command) {
            self.skip_while(is_space);
            return String::new();
        }
        if IGNORED_COMMANDS.contains(&command) {
            return String::new();
        }
        if matches!(command, "{" | "}" | "$" | "%" | "#" | "_" | "&") {
            return command.to_string();
        }
        if command == "|" {
            return "‖".to_string();
        }
        if command == "not" {
            let value = trim(&self.parse_required_argument(false)).to_string();
            if let Some(negated) = negated_symbol(&value) {
                return format!(" {negated} ");
            }
            let mut characters = value.chars();
            let Some(head) = characters.next() else {
                self.supported = false;
                return String::new();
            };
            return format!(" {head}\u{0338}{} ", characters.as_str());
        }
        if LIMIT_OPERATORS.contains(&command) {
            return self.parse_operator(command, LowerStyle::Bracket, true, true);
        }

        if let Some(symbol) = symbol(command) {
            if DISPLAY_LIMIT_SYMBOLS.contains(&command) {
                return self.parse_operator(symbol, LowerStyle::Script, true, false);
            }
            return if command == "cdot"
                || command == "times"
                || RELATION_COMMANDS.contains(&command)
            {
                format!(" {symbol} ")
            } else {
                symbol.to_string()
            };
        }
        if NAMED_OPERATORS.contains(&command) {
            return format!("{NAMED_OPERATOR_START}{command}{NAMED_OPERATOR_END}");
        }
        if SIZE_COMMANDS.contains(&command) {
            return String::new();
        }
        if command == "left" || command == "middle" || command == "right" {
            if self.at(self.position) == Some('.') {
                self.position += 1;
            }
            return String::new();
        }
        if command == "frac" || command == "dfrac" || command == "tfrac" {
            let should_stack = self.display && self.stack_fractions && command != "tfrac";
            let numerator = self.parse_required_argument(!should_stack);
            let denominator = self.parse_required_argument(!should_stack);
            if should_stack {
                self.layout_nodes.push(LayoutNode::Fraction {
                    numerator: normalize_output(&numerator),
                    denominator: normalize_output(&denominator),
                });
                return layout_marker(self.layout_nodes.len() - 1);
            }
            return format_fraction(&numerator, &denominator);
        }
        if command == "sqrt" {
            let degree = self
                .parse_optional_argument()
                .map(|degree| trim(&degree).to_string());
            let value = self.parse_required_argument(true);
            return match degree.as_deref() {
                None | Some("2") => format_root(&value, "√"),
                Some("3") => format_root(&value, "∛"),
                Some("4") => format_root(&value, "∜"),
                Some(degree) => format!(
                    "{}{}",
                    format_script(degree, ScriptKind::Sup),
                    format_root(&value, "√")
                ),
            };
        }
        if command == "boxed" || command == "fbox" {
            return format!("[{}]", trim(&self.parse_required_argument(true)));
        }
        if command == "binom" || command == "dbinom" || command == "tbinom" {
            let top = self.parse_required_argument(true);
            let bottom = self.parse_required_argument(true);
            return format!("({top} choose {bottom})");
        }
        if let Some(accent) = accent(command) {
            let value = self.parse_required_argument(true);
            return if char_count(&value) == 1 {
                format!("{value}{accent}")
            } else {
                format!("{command}({value})")
            };
        }
        if command == "mathbb" {
            let value = self.parse_required_argument(true);
            return value
                .chars()
                .map(|c| blackboard(c).map_or_else(|| c.to_string(), str::to_string))
                .collect();
        }
        if command == "operatorname" {
            let starred = self.at(self.position) == Some('*');
            if starred {
                self.position += 1;
            }
            let argument = self.parse_required_argument(true);
            let operator = trim(&normalize_output(&argument)).to_string();
            return self.parse_operator(&operator, LowerStyle::Bracket, starred, true);
        }
        if command == "mod" || command == "bmod" {
            return " mod ".to_string();
        }
        if command == "pmod" || command == "pod" {
            let value = trim(&self.parse_required_argument(true)).to_string();
            return if command == "pmod" {
                format!(" (mod {value})")
            } else {
                format!(" ({value})")
            };
        }
        if command == "overset" || command == "stackrel" {
            let upper = self.parse_required_argument(true);
            let value = trim(&self.parse_required_argument(true)).to_string();
            return format!("{value}{}", format_script(&upper, ScriptKind::Sup));
        }
        if command == "underset" {
            let lower = self.parse_required_argument(true);
            let value = trim(&self.parse_required_argument(true)).to_string();
            return format!("{value}{}", format_script(&lower, ScriptKind::Sub));
        }
        if PLAIN_WRAPPERS.contains(&command) {
            let value = self.parse_required_argument(true);
            return if command.starts_with("text") || command == "mbox" {
                value
            } else {
                trim(&value).to_string()
            };
        }
        if command == "begin" {
            return self.parse_environment();
        }
        if command == "end" {
            self.supported = false;
            return String::new();
        }

        self.supported = false;
        format!("\\{command}")
    }

    fn parse_operator(
        &mut self,
        operator: &str,
        inline_lower_style: LowerStyle,
        display_limits: bool,
        spaced: bool,
    ) -> String {
        let mut use_display_limits = display_limits;
        let mut modifier_position = self.position;
        while modifier_position < self.source.len() && is_blank(self.source[modifier_position]) {
            modifier_position += 1;
        }
        // `^\\(limits|nolimits)(?![A-Za-z])`
        if self.at(modifier_position) == Some('\\') {
            let rest = slice(&self.source, modifier_position + 1, self.source.len());
            let word = if rest.starts_with("limits") {
                Some("limits")
            } else if rest.starts_with("nolimits") {
                Some("nolimits")
            } else {
                None
            };
            if let Some(word) = word
                && !rest[word.len()..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
            {
                use_display_limits = word == "limits";
                self.position = modifier_position + 1 + word.len();
            }
        }

        let mut lower: Option<String> = None;
        let mut upper: Option<String> = None;
        loop {
            let mut script_position = self.position;
            while script_position < self.source.len() && is_blank(self.source[script_position]) {
                script_position += 1;
            }
            let kind = self.at(script_position);
            if kind != Some('_') && kind != Some('^') {
                break;
            }
            self.position = script_position + 1;
            let argument = self.parse_required_argument(false);
            let value = normalize_output(&argument).replace(' ', "");
            if kind == Some('_') {
                if lower.is_some() {
                    self.supported = false;
                }
                lower = Some(value);
            } else {
                if upper.is_some() {
                    self.supported = false;
                }
                upper = Some(value);
            }
        }

        if self.display && use_display_limits && (lower.is_some() || upper.is_some()) {
            self.layout_nodes.push(LayoutNode::Operator {
                operator: operator.to_string(),
                lower,
                upper,
            });
            return layout_marker(self.layout_nodes.len() - 1);
        }

        let mut rendered = operator.to_string();
        if let Some(lower) = &lower {
            rendered.push_str(&if inline_lower_style == LowerStyle::Bracket {
                format!("[{lower}]")
            } else {
                format_script(lower, ScriptKind::Sub)
            });
        }
        if let Some(upper) = &upper {
            rendered.push_str(&format_script(upper, ScriptKind::Sup));
        }
        if spaced {
            format!(" {rendered} ")
        } else {
            rendered
        }
    }

    fn parse_required_argument(&mut self, stack_fractions: bool) -> String {
        let previous_stack_fractions = self.stack_fractions;
        self.stack_fractions = previous_stack_fractions && stack_fractions;
        let value = self.parse_required_argument_value();
        self.stack_fractions = previous_stack_fractions;
        value
    }

    fn parse_required_argument_value(&mut self) -> String {
        self.deeper(Self::parse_argument)
    }

    fn parse_argument(&mut self) -> String {
        self.skip_while(is_space);
        if self.position >= self.source.len() {
            self.supported = false;
            return String::new();
        }
        if self.source[self.position] == '{' {
            self.position += 1;
            return self.parse_sequence(Some('}'));
        }
        if self.source[self.position] == '\\' {
            return self.parse_command();
        }
        let value = self.source[self.position];
        self.position += 1;
        value.to_string()
    }

    fn parse_optional_argument(&mut self) -> Option<String> {
        self.skip_while(is_blank);
        if self.at(self.position) != Some('[') {
            return None;
        }
        let Some(end) = (self.position + 1..self.source.len()).find(|&i| self.source[i] == ']')
        else {
            self.supported = false;
            return None;
        };
        let value = slice(&self.source, self.position + 1, end);
        self.position = end + 1;
        Some(self.render_nested(&value, true))
    }

    fn read_raw_group(&mut self) -> Option<String> {
        self.skip_while(is_blank);
        if self.at(self.position) != Some('{') {
            self.supported = false;
            return None;
        }

        self.position += 1;
        let start = self.position;
        let mut depth = 1;
        while self.position < self.source.len() {
            let character = self.source[self.position];
            if character == '\\' {
                self.position += 2;
                continue;
            }
            if character == '{' {
                depth += 1;
            }
            if character == '}' {
                depth -= 1;
            }
            if depth == 0 {
                let value = slice(&self.source, start, self.position);
                self.position += 1;
                return Some(value);
            }
            self.position += 1;
        }
        self.supported = false;
        None
    }

    /// `body.split(/\\\\(?:\[[^\]\n]*\])?/)`.
    fn split_environment_rows(body: &str) -> Vec<String> {
        let chars: Vec<char> = body.chars().collect();
        let mut rows = Vec::new();
        let mut row_start = 0;
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' && chars.get(i + 1) == Some(&'\\') {
                let mut end = i + 2;
                if chars.get(end) == Some(&'[') {
                    let mut close = end + 1;
                    while close < chars.len() && chars[close] != ']' && chars[close] != '\n' {
                        close += 1;
                    }
                    if chars.get(close) == Some(&']') {
                        end = close + 1;
                    }
                }
                rows.push(slice(&chars, row_start, i));
                row_start = end;
                i = end;
            } else {
                i += 1;
            }
        }
        rows.push(slice(&chars, row_start, chars.len()));
        rows
    }

    /// Splits each row of `body` into trimmed, rendered cells and drops empty rows.
    fn render_cells(&mut self, body: &str) -> Vec<Vec<String>> {
        let mut rows = Vec::new();
        for row in Self::split_environment_rows(body) {
            let mut cells = Vec::new();
            for cell in row.split('&') {
                cells.push(trim(&self.render_nested(cell, false)).to_string());
            }
            if cells.iter().any(|cell| !cell.is_empty()) {
                rows.push(cells);
            }
        }
        rows
    }

    fn parse_environment(&mut self) -> String {
        let Some(environment) = self
            .read_raw_group()
            .filter(|environment| !environment.is_empty())
        else {
            return String::new();
        };
        let end_marker: Vec<char> = format!("\\end{{{environment}}}").chars().collect();
        let Some(end) = find_chars(&self.source, &end_marker, self.position) else {
            self.supported = false;
            return String::new();
        };
        let body = slice(&self.source, self.position, end);
        self.position = end + end_marker.len();
        let environment = environment.as_str();

        if matches!(environment, "equation" | "equation*" | "displaymath") {
            return trim(&self.render_nested(&body, true)).to_string();
        }

        if matches!(
            environment,
            "aligned"
                | "align"
                | "align*"
                | "alignedat"
                | "alignat"
                | "alignat*"
                | "gather"
                | "gathered"
                | "multline"
                | "multline*"
                | "split"
        ) {
            let aligned_at = matches!(environment, "alignedat" | "alignat" | "alignat*");
            let aligned_body = if aligned_at {
                strip_leading_group(&body)
            } else {
                body.clone()
            };
            let mut rendered_rows = Vec::new();
            for row in Self::split_environment_rows(&aligned_body) {
                let cells: Vec<&str> = row.split('&').collect();
                let source = if aligned_at {
                    cells
                        .chunks(2)
                        .map(|pair| pair.concat())
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    cells.concat()
                };
                let rendered = trim(&self.render_nested(&source, true)).to_string();
                if !rendered.is_empty() {
                    rendered_rows.push(rendered);
                }
            }
            return rendered_rows.join("\n");
        }

        if environment == "cases" || environment == "cases*" {
            return self.render_cases(&body);
        }

        if [
            "array",
            "matrix",
            "smallmatrix",
            "pmatrix",
            "bmatrix",
            "Bmatrix",
            "vmatrix",
            "Vmatrix",
        ]
        .contains(&environment)
        {
            let matrix_body = if environment == "array" {
                strip_leading_group(&body)
            } else {
                body.clone()
            };
            return self.render_matrix(environment, &matrix_body);
        }

        self.supported = false;
        body
    }

    fn render_cases(&mut self, body: &str) -> String {
        let rows = self.render_cells(body);
        let value_of = |row: &Vec<String>| {
            strip_trailing_comma(row.first().map_or("", String::as_str)).to_string()
        };
        let value_width = rows
            .iter()
            .map(|row| visible_width(&value_of(row)))
            .max()
            .unwrap_or(0);
        let contents: Vec<String> = rows
            .iter()
            .map(|row| {
                let value = value_of(row);
                let condition = row.get(1).map_or("", String::as_str);
                if condition.is_empty() {
                    return value;
                }
                let condition_prefix = if starts_with_condition_word(condition) {
                    " "
                } else {
                    " if "
                };
                format!(
                    "{value}{}{condition_prefix}{condition}",
                    PROTECTED_SPACE.repeat(value_width.saturating_sub(visible_width(&value)))
                )
            })
            .collect();
        if contents.len() <= 1 {
            return match contents.first() {
                None => String::new(),
                Some(content) => format!("⎧ {content}"),
            };
        }

        let middle = contents.len() / 2;
        let mut visual_rows: Vec<Option<String>> = contents.iter().cloned().map(Some).collect();
        if contents.len().is_multiple_of(2) {
            visual_rows.insert(middle, None);
        }
        let last = visual_rows.len() - 1;
        let lines: Vec<String> = visual_rows
            .iter()
            .enumerate()
            .map(|(index, content)| {
                let delimiter = if index == 0 {
                    "⎧"
                } else if index == last {
                    "⎩"
                } else {
                    "⎨"
                };
                match content {
                    None => delimiter.to_string(),
                    Some(content) => format!("{delimiter} {content}"),
                }
            })
            .collect();
        self.layout_nodes.push(LayoutNode::Matrix {
            lines,
            baseline: middle,
        });
        layout_marker(self.layout_nodes.len() - 1)
    }

    fn render_matrix(&mut self, environment: &str, body: &str) -> String {
        let matrix = self.render_cells(body);
        let column_count = matrix.iter().map(Vec::len).max().unwrap_or(0);
        let column_widths: Vec<usize> = (0..column_count)
            .map(|column| {
                matrix
                    .iter()
                    .map(|row| visible_width(row.get(column).map_or("", String::as_str)))
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let rows: Vec<String> = matrix
            .iter()
            .map(|row| {
                (0..column_count)
                    .map(|column| {
                        let cell = row.get(column).map_or("", String::as_str);
                        format!(
                            "{cell}{}",
                            PROTECTED_SPACE
                                .repeat(column_widths[column].saturating_sub(visible_width(cell)))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" │ ")
            })
            .collect();

        let lines: Vec<String> =
            if environment == "array" || environment == "matrix" || environment == "smallmatrix" {
                rows
            } else {
                let delimiter: Option<[&str; 6]> = match environment {
                    "pmatrix" => Some(["⎛", "⎞", "⎜", "⎟", "⎝", "⎠"]),
                    "bmatrix" => Some(["⎡", "⎤", "⎢", "⎥", "⎣", "⎦"]),
                    "Bmatrix" => Some(["⎧", "⎫", "⎨", "⎬", "⎩", "⎭"]),
                    "vmatrix" => Some(["│", "│", "│", "│", "│", "│"]),
                    "Vmatrix" => Some(["║", "║", "║", "║", "║", "║"]),
                    _ => None,
                };
                let Some(delimiter) = delimiter else {
                    self.supported = false;
                    return rows.join("\n");
                };
                rows.iter()
                    .enumerate()
                    .map(|(index, row)| {
                        let left = if index == 0 {
                            delimiter[0]
                        } else if index == rows.len() - 1 {
                            delimiter[4]
                        } else {
                            delimiter[2]
                        };
                        let right = if index == 0 {
                            delimiter[1]
                        } else if index == rows.len() - 1 {
                            delimiter[5]
                        } else {
                            delimiter[3]
                        };
                        format!("{left} {row} {right}")
                    })
                    .collect()
            };

        if lines.len() <= 1 {
            return lines.into_iter().next().unwrap_or_default();
        }
        self.layout_nodes
            .push(LayoutNode::Matrix { lines, baseline: 0 });
        layout_marker(self.layout_nodes.len() - 1)
    }

    fn render_nested(&mut self, source: &str, stack_fractions: bool) -> String {
        let display = self.display && stack_fractions;
        let rendered = self.deeper(|parser| {
            let mut nested = LatexParser::new(source, &mut *parser.layout_nodes, display);
            nested.depth = parser.depth;
            nested.render()
        });
        match rendered {
            Some(rendered) => rendered,
            None => {
                self.supported = false;
                source.to_string()
            }
        }
    }
}

/// `value.replace(/^\s*\{[^}]*\}/, "")`.
fn strip_leading_group(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;
    while i < chars.len() && is_space(chars[i]) {
        i += 1;
    }
    if chars.get(i) == Some(&'{')
        && let Some(close) = (i + 1..chars.len()).find(|&j| chars[j] == '}')
    {
        return slice(&chars, close + 1, chars.len());
    }
    value.to_string()
}

/// `value.replace(/,\s*$/, "")`.
fn strip_trailing_comma(value: &str) -> &str {
    let trimmed = value.trim_end_matches(is_space);
    trimmed.strip_suffix(',').unwrap_or(value)
}

/// `/^(?:if|when|for|otherwise)\b/i`.
fn starts_with_condition_word(condition: &str) -> bool {
    let chars: Vec<char> = condition.chars().collect();
    ["if", "when", "for", "otherwise"].iter().any(|word| {
        let length = word.chars().count();
        chars.len() >= length
            && chars
                .iter()
                .zip(word.chars())
                .all(|(c, w)| c.to_ascii_lowercase() == w)
            && !chars
                .get(length)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
    })
}

/// Renders a basic LaTeX math expression as terminal-friendly Unicode text.
///
/// With `display`, fractions and operator limits are stacked vertically, so the
/// result may span several lines. Returns `None` when the expression contains
/// unsupported or malformed syntax.
pub fn render_latex(source: &str, display: bool) -> Option<String> {
    // pi's own markers, which in the source could point a layout at itself.
    if source.contains(|c| ('\u{f0000}'..='\u{f0005}').contains(&c)) {
        return None;
    }
    let mut layout_nodes: Vec<LayoutNode> = Vec::new();
    let rendered = LatexParser::new(source, &mut layout_nodes, display).render()?;
    if layout_nodes.is_empty() {
        return Some(rendered.replace(PROTECTED_SPACE, " "));
    }
    let lines = render_layout(&rendered, &layout_nodes).lines;
    let indentation = lines
        .iter()
        .filter(|line| !trim(line).is_empty())
        .map(|line| line.chars().count() - trim_start(line).chars().count())
        .min();
    let unindented: Vec<String> = lines
        .iter()
        .map(|line| match indentation {
            Some(indentation) => {
                trim_end(&line.chars().skip(indentation).collect::<String>()).to_string()
            }
            None => String::new(),
        })
        .collect();
    Some(trim_end(&unindented.join("\n")).replace(PROTECTED_SPACE, " "))
}

#[cfg(test)]
mod tests {
    use super::render_latex;

    fn check(source: &str, display: bool, expected: &str) {
        assert_eq!(
            render_latex(source, display).as_deref(),
            Some(expected),
            "source: {source:?}"
        );
    }

    fn check_none(source: &str, display: bool) {
        assert_eq!(render_latex(source, display), None, "source: {source:?}");
    }

    #[test]
    fn jacobian_conjecture_session_using_dollar_delimiters() {
        check(r#"\mathbb{C}^3 \to \mathbb{C}^3"#, false, "ℂ³ → ℂ³");
        check(
            r#"\{3x+2y,\; 27x^2-4z-1,\; x(x-1)(x+1)\} \quad\Rightarrow\quad x \in \{0, \pm 1\},"#,
            false,
            "{3x+2y, 27x²-4z-1, x(x-1)(x+1)} ⇒ x ∈ {0, ± 1},",
        );
        check(r#"F_1 = -\frac{1}{4x^2}."#, false, "F₁ = -1/(4x²).");
        check("-2", false, "-2");
        check("(0,0,-1/4)", false, "(0,0,-1/4)");
        check("(1,-3/2,13/2)", false, "(1,-3/2,13/2)");
        check("(1,1,1)", false, "(1,1,1)");
        check("(2,1,0)", false, "(2,1,0)");
        check("(-1/4, 0, 0)", false, "(-1/4, 0, 0)");
        check(
            r#"\{(0,0,-1/4), (1,-3/2,13/2), (-1,3/2,13/2)\}"#,
            false,
            "{(0,0,-1/4), (1,-3/2,13/2), (-1,3/2,13/2)}",
        );
        check("(2,1,1)", false, "(2,1,1)");
        check("(7/3,-2/5,11/7)", false, "(7/3,-2/5,11/7)");
        check(r#"\{y - p(x),\; q(x)\}"#, false, "{y - p(x), q(x)}");
        check(r#"\deg q = 3"#, false, "deg q = 3");
        check(
            r#"[\mathbb{C}(x,y,z):\mathbb{C}(F_1,F_2,F_3)] = 3"#,
            false,
            "[ℂ(x,y,z):ℂ(F₁,F₂,F₃)] = 3",
        );
        check("u = 1+xy", false, "u = 1+xy");
        check("G = u^2 z + y^2(4+3xy)", false, "G = u² z + y²(4+3xy)");
        check("F_1 = uG", false, "F₁ = uG");
        check("F_2 = y + 3xG", false, "F₂ = y + 3xG");
        check("x=0", false, "x = 0");
        check("F_2 = F_3 = 0", false, "F₂ = F₃ = 0");
        check("xy = -3/2", false, "xy = -3/2");
        check("x^2 z = 13/2", false, "x² z = 13/2");
        check(r#"\mathbb{C}^*"#, false, "ℂ^*");
        check(
            r#"s \mapsto (s,\, -\tfrac{3}{2s},\, \tfrac{13}{2s^2})"#,
            false,
            "s ↦ (s, -3/(2s), 13/(2s²))",
        );
        check("X", false, "X");
        check(r#"p_\pm"#, false, "p_±");
        check(
            "F(-x,-y,z) = (F_1, -F_2, -F_3)",
            false,
            "F(-x,-y,z) = (F₁, -F₂, -F₃)",
        );
        check("p_0", false, "p₀");
        check(r#"s \to \infty"#, false, "s → ∞");
        check("(0,0,0)", false, "(0,0,0)");
        check(r#"\Rightarrow"#, false, "⇒");
        check(r#"\ge 2"#, false, "≥ 2");
        check(r#"\ge 3"#, false, "≥ 3");
        check("1", false, "1");
        check(
            r#"\mathrm{diag}(-1/2,1,1),\quad F_{\rm intrinsic}(\lambda)"#,
            false,
            "diag(-1/2,1,1), F_intrinsic(λ)",
        );
        check("4+3xy", false, "4+3xy");
    }

    #[test]
    fn satellite_calculation_session_using_bracket_delimiters() {
        check(
            r#"E \approx \frac{0.1\ \text{lux}}{100\ \text{lm/W}} = 0.001\ \text{W/m}^2"#,
            false,
            "E ≈ (0.1 lux)/(100 lm/W) = 0.001 W/m²",
        );
        check(
            r#"\boxed{1\ \text{milliwatt per square metre}}"#,
            false,
            "[1 milliwatt per square metre]",
        );
        check(
            r#"5\ \text{km}^2 = 5{,}000{,}000\ \text{m}^2"#,
            false,
            "5 km² = 5,000,000 m²",
        );
        check(
            r#"P_{\text{light}} = 0.001 \times 5{,}000{,}000
= \boxed{5{,}000\ \text{W}}"#,
            false,
            "P_light = 0.001 × 5,000,000 = [5,000 W]",
        );
        check(
            r#"P_{\text{electric}} = 5\ \text{kW} \times 0.2
= \boxed{1\ \text{kW}}"#,
            false,
            "P_electric = 5 kW × 0.2 = [1 kW]",
        );
        check(
            r#"\pi(2.5\ \text{km})^2 = 19.6\ \text{km}^2"#,
            false,
            "π(2.5 km)² = 19.6 km²",
        );
        check(
            r#"0.001\ \text{W/m}^2 \times 19.6 \times 10^6\ \text{m}^2
\approx \boxed{20\ \text{kW optical}}"#,
            false,
            "0.001 W/m² × 19.6 × 10⁶ m² ≈ [20 kW optical]",
        );
        check(
            r#"1\ \text{kW} \times \frac{1}{3600}\ \text{hour}
= \boxed{0.28\ \text{Wh}}"#,
            false,
            "1 kW × 1/3600 hour = [0.28 Wh]",
        );
    }

    #[test]
    fn jacobian_conjecture_sessions_using_parenthesis_and_bracket_delimiters() {
        check(
            r#"\det\!\left(\frac{\partial(F_1,F_2,F_3)}{\partial(x,y,z)}\right)=-2."#,
            false,
            "det((∂(F₁,F₂,F₃))/(∂(x,y,z))) = -2.",
        );
        check(
            r#"\begin{aligned}
F(0,0,-\tfrac14)&=(-\tfrac14,0,0),\\
F(1,-\tfrac32,\tfrac{13}2)&=(-\tfrac14,0,0),\\
F(-1,\tfrac32,\tfrac{13}2)&=(-\tfrac14,0,0).
\end{aligned}"#,
            false,
            "F(0,0,-1/4) = (-1/4,0,0),\nF(1,-3/2,13/2) = (-1/4,0,0),\nF(-1,3/2,13/2) = (-1/4,0,0).",
        );
        check("F=(F_1,F_2,F_3)", false, "F = (F₁,F₂,F₃)");
        check("F", false, "F");
        check("3", false, "3");
    }

    #[test]
    fn jacobian_matrix_session_using_dollar_delimiters() {
        check(
            r#"J = \begin{pmatrix}
\frac{\partial f_1}{\partial x} & \frac{\partial f_1}{\partial y} & \frac{\partial f_1}{\partial z} \\
\frac{\partial f_2}{\partial x} & \frac{\partial f_2}{\partial y} & \frac{\partial f_2}{\partial z} \\
\frac{\partial f_3}{\partial x} & \frac{\partial f_3}{\partial y} & \frac{\partial f_3}{\partial z}
\end{pmatrix}"#,
            false,
            "J = ⎛ (∂ f₁)/(∂ x) │ (∂ f₁)/(∂ y) │ (∂ f₁)/(∂ z) ⎞\n    ⎜ (∂ f₂)/(∂ x) │ (∂ f₂)/(∂ y) │ (∂ f₂)/(∂ z) ⎟\n    ⎝ (∂ f₃)/(∂ x) │ (∂ f₃)/(∂ y) │ (∂ f₃)/(∂ z) ⎠",
        );
        check(
            r#"\begin{aligned}
f_1 &= (1+xy)^3 z + y^2(1+xy)(4+3xy) \\
f_2 &= y + 3x(1+xy)^2 z + 3xy^2(4+3xy) \\
f_3 &= 2x - 3x^2y - x^3z
\end{aligned}"#,
            false,
            "f₁ = (1+xy)³ z + y²(1+xy)(4+3xy)\nf₂ = y + 3x(1+xy)² z + 3xy²(4+3xy)\nf₃ = 2x - 3x²y - x³z",
        );
        check("x, y, z", false, "x, y, z");
        check("(x, y, z)", false, "(x, y, z)");
        check(r#"(0,\; 0,\; -\tfrac14)"#, false, "(0, 0, -1/4)");
        check(r#"(-\tfrac14,\; 0,\; 0)"#, false, "(-1/4, 0, 0)");
        check(
            r#"(1,\; -\tfrac32,\; \tfrac{13}{2})"#,
            false,
            "(1, -3/2, 13/2)",
        );
        check(
            r#"(-1,\; \tfrac32,\; \tfrac{13}{2})"#,
            false,
            "(-1, 3/2, 13/2)",
        );
        check(r#"(-\frac14, 0, 0)"#, false, "(-1/4, 0, 0)");
        check(r#"F: \mathbb{C}^3 \to \mathbb{C}^3"#, false, "F: ℂ³ → ℂ³");
        check(
            r#"F(0,0,-\tfrac14) = F(1,-\tfrac32,\tfrac{13}{2}) = F(-1,\tfrac32,\tfrac{13}{2}) = (-\tfrac14, 0, 0)"#,
            false,
            "F(0,0,-1/4) = F(1,-3/2,13/2) = F(-1,3/2,13/2) = (-1/4, 0, 0)",
        );
        check(r#"\mathbb{C}^3"#, false, "ℂ³");
        check(
            r#"\begin{aligned}
f_1 &= \frac{f_1^{\text{ut}}(u,t)}{x^2}, \quad
f_2 = \frac{f_2^{\text{ut}}(u,t)}{x}, \quad
f_3 = x\,(2 - 3u - t)
\end{aligned}"#,
            false,
            "f₁ = (f₁ᵘᵗ(u,t))/(x²), f₂ = (f₂ᵘᵗ(u,t))/x, f₃ = x (2 - 3u - t)",
        );
        check(r#"\det J_F"#, false, "det J_F");
        check(r#"(-\tfrac14, 0, 0)"#, false, "(-1/4, 0, 0)");
        check("u = xy", false, "u = xy");
        check("t = x^2z", false, "t = x²z");
        check(r#"x \neq 0"#, false, "x ≠ 0");
        check(r#"f_1^{\text{ut}}, f_2^{\text{ut}}"#, false, "f₁ᵘᵗ, f₂ᵘᵗ");
        check("u,t", false, "u,t");
        check("x", false, "x");
        check("x, x^2", false, "x, x²");
        check(r#"\mathbb{C}^n \to \mathbb{C}^n"#, false, "ℂⁿ → ℂⁿ");
        check(r#"n \geq 2"#, false, "n ≥ 2");
        check(r#"\mathbb{P}^3"#, false, "ℙ³");
    }

    #[test]
    fn extended_formulas_from_a_renderer_stress_test_session() {
        check(r#"e^{i\pi}+1=0"#, false, "e^(iπ)+1 = 0");
        check(
            r#"\boxed{
\mathcal{Z}(\beta)
=
\int_{\mathcal M}
\exp\!\left(
-\beta\left[
\frac12 g^{ij}(x)\,\partial_i\phi\,\partial_j\phi
+V(\phi)
\right]\right)
\mathcal D\phi
}"#,
            false,
            "[Z(β) = ∫_M exp( -β[ 1/2 gⁱʲ(x) ∂ᵢϕ ∂ⱼϕ +V(ϕ) ]) Dϕ]",
        );
        check(
            r#"\begin{aligned}
\nabla_\mu T^{\mu\nu}
&=
\frac{1}{\sqrt{-g}}
\partial_\mu\!\left(\sqrt{-g}\,T^{\mu\nu}\right)
+\Gamma^\nu_{\mu\lambda}T^{\mu\lambda}
=0, \\[4pt]
R_{\mu\nu}-\frac12 Rg_{\mu\nu}+\Lambda g_{\mu\nu}
&=
\frac{8\pi G}{c^4}T_{\mu\nu}.
\end{aligned}"#,
            false,
            "∇_μ T^(μν) = 1/(√(-g)) ∂_μ(√(-g) T^(μν)) +Γ^ν_(μλ)T^(μλ) = 0,\nR_(μν)-1/2 Rg_(μν)+Λ g_(μν) = (8π G)/(c⁴)T_(μν).",
        );
        check(
            r#"f(z)
=
\frac{1}{2\pi i}
\oint_{\gamma}
\frac{f(\zeta)}{\zeta-z}\,d\zeta,
\qquad
\det\!\begin{pmatrix}
\lambda-a & -b & 0\\
-c & \lambda-d & -e\\
0 & -f & \lambda-g
\end{pmatrix}
=0."#,
            false,
            "f(z) = 1/(2π i) ∮_γ (f(ζ))/(ζ-z) dζ, det⎛ λ-a │ -b  │ 0   ⎞ = 0.\n                                        ⎜ -c  │ λ-d │ -e  ⎟\n                                        ⎝ 0   │ -f  │ λ-g ⎠",
        );
        check(
            r#"\Psi(x,t)=
\sum_{n=1}^{\infty}
\underbrace{
c_n
\sqrt{\frac{2}{L}}
\sin\!\left(\frac{n\pi x}{L}\right)
}_{\text{spatial eigenmode}}
\exp\!\left(-\frac{i\hbar n^2\pi^2}{2mL^2}t\right),
\qquad
|\Psi(x,t)|^2
=
\begin{cases}
\Psi^\ast\Psi, & 0<x<L,\\
0, & \text{otherwise}.
\end{cases}"#,
            false,
            "                                                                                                 ⎧ Ψ^∗Ψ if 0 < x < L,\nΨ(x,t) = ∑ₙ₌₁^∞ cₙ √(2/L) sin((nπ x)/L)_(spatial eigenmode) exp(-(iℏ n²π²)/(2mL²)t), |Ψ(x,t)|² = ⎨\n                                                                                                 ⎩ 0    otherwise.",
        );
        check(
            r#"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}"#,
            false,
            "x = (-b±√(b²-4ac))/(2a)",
        );
        check(
            r#"\int_0^\infty e^{-x^2}\,dx=\frac{\sqrt{\pi}}{2}"#,
            false,
            "∫₀^∞ e^(-x²) dx = (√π)/2",
        );
        check(
            r#"e^{i\theta}=\cos\theta+i\sin\theta"#,
            false,
            "e^(iθ) = cos θ+i sin θ",
        );
        check(
            r#"\sum_{n=1}^{\infty}\frac{1}{n^2}=\frac{\pi^2}{6}"#,
            false,
            "∑ₙ₌₁^∞1/(n²) = π²/6",
        );
        check(
            r#"\lim_{x\to 0}\frac{\sin x}{x}=1"#,
            false,
            "lim[x→0] (sin x)/x = 1",
        );
        check(
            r#"\lim_{n\to\infty}
\left(1+\frac{1}{n}\right)^n=e"#,
            false,
            "lim[n→∞] (1+1/n)ⁿ = e",
        );
        check(
            r#"\int_0^1 \frac{x^2}{1+x^3}\,dx
=\frac{1}{3}\ln 2"#,
            false,
            "∫₀¹ x²/(1+x³) dx = 1/3 ln 2",
        );
        check(
            r#"\sum_{k=1}^{n}\frac{k}{k+1}
=n+1-H_{n+1}"#,
            false,
            "∑ₖ₌₁ⁿk/(k+1) = n+1-Hₙ₊₁",
        );
        check(
            r#"\frac{
  \displaystyle \frac{x^2+1}{x-1}
  -
  \displaystyle \frac{2x}{x+1}
}{
  \displaystyle \frac{x}{x^2-1}
}"#,
            false,
            "((x²+1)/(x-1) - 2x/(x+1))/(x/(x²-1))",
        );
        check(
            r#"\lim_{x\to 0}
\frac{
  \displaystyle \frac{\sin x}{x}-1
}{
  \displaystyle \frac{e^x-1}{x}-1
}
=0"#,
            false,
            "lim[x→0] ((sin x)/x-1)/((eˣ-1)/x-1) = 0",
        );
        check(
            r#"\frac{
  1+\displaystyle\frac{1}{1+\frac{1}{x}}
}{
  1-\displaystyle\frac{1}{1-\frac{1}{x}}
}"#,
            false,
            "(1+1/(1+1/x))/(1-1/(1-1/x))",
        );
        check(
            r#"\sum_{n=1}^{\infty}
\frac{
  \displaystyle \frac{1}{n}-\frac{1}{n+1}
}{
  \displaystyle 1+\frac{1}{n^2}
}"#,
            false,
            "∑ₙ₌₁^∞ (1/n-1/(n+1))/(1+1/(n²))",
        );
    }

    #[test]
    fn renders_common_symbols_roots_sums_and_integrals() {
        check(
            r#"\sum_{i=0}^n \alpha_i + \int_0^\infty e^{-x^2}\,dx = \sqrt{\pi}"#,
            false,
            "∑ᵢ₌₀ⁿ αᵢ + ∫₀^∞ e^(-x²) dx = √π",
        );
    }

    #[test]
    fn renders_common_accents_and_binomial_notation() {
        check(
            r#"\binom{n}{k}+\vec{x}+\hat{y}+\overline{AB}"#,
            false,
            "(n choose k)+x⃗+ŷ+overline(AB)",
        );
    }

    #[test]
    fn renders_extended_symbols_and_negated_relations() {
        check(
            r#"\epsilon+\varepsilon+\varsigma+\varkappa+\oplus+\otimes+\therefore+\because"#,
            false,
            "ϵ+ε+ς+ϰ+⊕+⊗+∴+∵",
        );
        check(
            r#"A\not\subseteq B,\quad x\not\in X"#,
            false,
            "A ⊈ B, x ∉ X",
        );
    }

    #[test]
    fn renders_relational_algebra_join_operators() {
        check(r#"R\bowtie S,\quad R\Join S"#, false, "R ⋈ S, R ⋈ S");
        check(r#"R\ltimes S,\quad R\rtimes S"#, false, "R ⋉ S, R ⋊ S");
        check(
            r#"R\leftouterjoin S,\quad R\rightouterjoin S,\quad R\fullouterjoin S"#,
            false,
            "R ⟕ S, R ⟖ S, R ⟗ S",
        );
    }

    #[test]
    fn renders_delimiter_commands_and_invisible_delimiters() {
        check(
            r#"\lvert{x}\rvert+\lVert{v}\rVert+\left.\frac{dy}{dx}\right|_{x=0}"#,
            false,
            "|x|+‖v‖+dy/(dx)|ₓ₌₀",
        );
        check(
            r#"\left\lbrace x \middle| x>0 \right\rbrace"#,
            false,
            "{ x | x > 0 }",
        );
    }

    #[test]
    fn renders_named_modular_overlaid_and_underlaid_operators() {
        check(
            r#"\operatorname*{arg\,max}_{x\in X} f(x)"#,
            false,
            "arg max[x∈X] f(x)",
        );
        check(
            r#"a\bmod n,\quad a\equiv b\pmod n"#,
            false,
            "a mod n, a ≡ b (mod n)",
        );
        check(
            r#"\overset{!}{=}+\underset{n}{x}+\stackrel{def}{=}"#,
            false,
            "=^!+xₙ+=ᵈᵉᶠ",
        );
    }

    #[test]
    fn renders_indexed_roots_and_additional_accents_and_wrappers() {
        check(
            r#"\sqrt[2]{x}+\sqrt[3]{x}+\sqrt[4]{x}+\sqrt[n]{x}+\sqrt[k]{x+1}"#,
            false,
            "√x+∛x+∜x+ⁿ√x+ᵏ√(x+1)",
        );
        check(
            r#"\acute{x}+\grave{y}+\widehat{xyz}+\overrightarrow{AB}"#,
            false,
            "x́+ỳ+widehat(xyz)+overrightarrow(AB)",
        );
        check(
            r#"\textnormal{hello}+\mbox{world}+\boldsymbol{x}+{\rm roman}+{\bf bold}+{\it italic}+{\sf sans}+{\tt mono}+{\cal calligraphic}+{\sl slanted}"#,
            false,
            "hello+world+x+roman+bold+italic+sans+mono+calligraphic+slanted",
        );
    }

    #[test]
    fn renders_additional_display_environments() {
        check(
            r#"\begin{equation}\begin{split}a&=b\\&=c\end{split}\end{equation}"#,
            false,
            "a = b\n= c",
        );
        check(
            r#"\begin{alignedat}{2}a&=b&\quad c&=d\\e&=f&g&=h\end{alignedat}"#,
            false,
            "a = b c = d\ne = f g = h",
        );
    }

    #[test]
    fn uses_natural_case_conditions_and_aligns_matrix_columns() {
        check(
            r#"f(x)=\begin{cases}a & x<0 \\ b & \text{if }x=0 \\ c & \text{otherwise}\end{cases}"#,
            false,
            "       ⎧ a if x < 0\nf(x) = ⎨ b if x = 0\n       ⎩ c otherwise",
        );
        check(
            r#"\begin{pmatrix}1&200\\3000&4\end{pmatrix}"#,
            false,
            "⎛ 1    │ 200 ⎞\n⎝ 3000 │ 4   ⎠",
        );
    }

    #[test]
    fn composes_matrices_with_fractions_and_adjacent_matrices() {
        check(
            r#"R\left(\frac{\pi}{4}\right)
=
\begin{pmatrix}
\frac{\sqrt{2}}{2} & -\frac{\sqrt{2}}{2}\\
\frac{\sqrt{2}}{2} & \frac{\sqrt{2}}{2}
\end{pmatrix}."#,
            true,
            "   π\nR( ─ ) = ⎛ (√2)/2 │ -(√2)/2 ⎞\n   4     ⎝ (√2)/2 │ (√2)/2  ⎠.",
        );
        check(
            r#"\mathbf w
=
R\left(\frac{\pi}{4}\right)
\begin{pmatrix}1\\0\end{pmatrix}
=
\begin{pmatrix}\frac{\sqrt{2}}{2}\\\frac{\sqrt{2}}{2}\end{pmatrix}."#,
            true,
            "       π\nw = R( ─ ) ⎛ 1 ⎞ = ⎛ (√2)/2 ⎞\n       4   ⎝ 0 ⎠   ⎝ (√2)/2 ⎠.",
        );
        check(
            r#"A\mathbf e_1=\begin{pmatrix}\pi\\0\end{pmatrix},\qquad A\mathbf e_2=\begin{pmatrix}0\\\frac{1}{\pi}\end{pmatrix}."#,
            true,
            "Ae₁ = ⎛ π ⎞, Ae₂ = ⎛ 0   ⎞\n      ⎝ 0 ⎠        ⎝ 1/π ⎠.",
        );
        check(
            r#"\sum_{i=0}^n x_i=\begin{pmatrix}a&b\\c&d\end{pmatrix}."#,
            true,
            " n\n ∑  xᵢ = ⎛ a │ b ⎞\ni=0      ⎝ c │ d ⎠.",
        );
    }

    #[test]
    fn normalizes_relation_multiplication_and_named_operator_spacing() {
        check("x=y", false, "x = y");
        check("x =y", false, "x = y");
        check("x=\ny", false, "x = y");
        check("x\n=\ny", false, "x = y");
        check("x_{i=0}", false, "xᵢ₌₀");
        check(r#"x\neq0"#, false, "x ≠ 0");
        check(r#"A\to B"#, false, "A → B");
        check(r#"\pi\cdot\frac{1}{\pi}"#, false, "π · 1/π");
        check(r#"\sin\theta"#, false, "sin θ");
        check(r#"\sin^2 x"#, false, "sin² x");
        check(r#"-\sin\theta"#, false, "-sin θ");
        check(r#"i\sin\theta"#, false, "i sin θ");
        check(r#"\det(A)"#, false, "det(A)");
    }

    #[test]
    fn treats_a_backslash_followed_by_a_line_ending_as_control_space() {
        check(
            r#"\boxed{
(1,1,1),\ (1,1,2),\ (1,2,5),\ (1,5,13),\ (2,5,29),\
(1,13,34),\ (1,34,89)
}."#,
            true,
            "[(1,1,1), (1,1,2), (1,2,5), (1,5,13), (2,5,29), (1,13,34), (1,34,89)].",
        );
        check("a\\\r\nb", false, "a b");
    }

    #[test]
    fn stacks_operator_limits_in_display_mode() {
        check(r#"\sum_{i=0}^n x_i"#, true, " n\n ∑  xᵢ\ni=0");
        check(r#"\min_{x\in X} f(x)"#, true, "min f(x)\nx∈X");
        check(
            r#"\operatorname*{arg\,max}_{x\in X} f(x)"#,
            true,
            "arg max f(x)\n  x∈X",
        );
        check(r#"\int\nolimits_0^1 f(x)\,dx"#, true, "∫₀¹ f(x) dx");
        check(r#"\int\limits_0^1 f(x)\,dx"#, true, "1\n∫ f(x) dx\n0");
    }

    #[test]
    fn centers_even_case_rows_around_a_middle_brace() {
        check(
            r#"f(x) = \begin{cases} x^{2} & x \geq 0 \\ -x & x < 0 \end{cases}"#,
            false,
            "       ⎧ x² if x ≥ 0\nf(x) = ⎨\n       ⎩ -x if x < 0",
        );
    }

    #[test]
    fn stacks_fractions_in_display_mode() {
        check(
            r#"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}"#,
            true,
            "    -b±√(b²-4ac)\nx = ────────────\n         2a",
        );
        check(r#"\frac{x^2+1}{x-1}"#, true, "x²+1\n────\nx-1");
        check(
            r#"\frac{1}
{2}"#,
            true,
            "1\n─\n2",
        );
    }

    #[test]
    fn keeps_nested_display_fractions_linear() {
        check(
            r#"\frac{\frac{x^2+1}{x-1}-\frac{2x}{x+1}}{\frac{x}{x^2-1}}"#,
            true,
            "(x²+1)/(x-1)-2x/(x+1)\n─────────────────────\n      x/(x²-1)",
        );
        check(
            r#"\lim_{x\to 0}\frac{\frac{\sin x}{x}-1}{\frac{e^x-1}{x}-1}=0"#,
            true,
            "     (sin x)/x-1\nlim  ─────────── = 0\nx→0  (eˣ-1)/x-1",
        );
        check(
            r#"\frac{1+\frac{1}{1+\frac{1}{x}}}{1-\frac{1}{1-\frac{1}{x}}}"#,
            true,
            "1+1/(1+1/x)\n───────────\n1-1/(1-1/x)",
        );
    }

    #[test]
    fn lays_out_unsupported_and_nested_scripts_while_keeping_script_fractions_linear() {
        check(
            r#"\partial_tU_2(t,0)=Aj_*(1-t)^{-A-1}.\qquad x^{n^2}+x_{i_j}"#,
            true,
            "                            2\n                    -A-1   n\n∂ₜU₂(t,0) = Aj (1-t)    . x  +x\n              *                i\n                                j",
        );
        check(r#"e^{\frac{1}{2}}+\tfrac{1}{2}"#, true, "e^(1/2)+1/2");
    }

    #[test]
    fn returns_undefined_for_unsupported_commands() {
        check_none(r#"x + \unknown{y}"#, false);
    }

    #[test]
    fn returns_undefined_for_malformed_groups_and_environments() {
        check_none(r#"\frac{1}{x"#, false);
        check_none("x}", false);
        check_none(r#"\begin{matrix}1 & 2"#, false);
        check_none(r#"x\"#, false);
    }

    #[test]
    fn deep_nesting_is_unsupported_without_overflowing_the_stack() {
        check(
            &format!("{}x{}", "{".repeat(100), "}".repeat(100)),
            false,
            "x",
        );
        let fractions = format!("{}x{}", r"\frac{".repeat(60), "}{y}".repeat(60));
        assert!(render_latex(&fractions, true).is_some());
        for display in [false, true] {
            check_none(
                &format!("{}x{}", "{".repeat(5000), "}".repeat(5000)),
                display,
            );
            check_none(&"{".repeat(100_000), display);
            check_none(
                &format!("{}x{}", r"\frac{".repeat(2000), "}{y}".repeat(2000)),
                display,
            );
            check_none(&format!("{}x", r"\sqrt".repeat(5000)), display);
            check_none(&format!("{}x", r"\not".repeat(5000)), display);
            check_none(
                &format!("x{}y{}", "^{".repeat(5000), "}".repeat(5000)),
                display,
            );
            check_none(
                &format!("{}x{}", r"\sum_{".repeat(5000), "}".repeat(5000)),
                display,
            );
        }
    }

    #[test]
    fn layout_markers_in_the_source_are_unsupported() {
        check_none("\\frac{\u{f0000}0\u{f0001}}{2}", true);
        check_none("\\frac{\u{f0000}1\u{f0001}}{2}\\frac{a}{b}", true);
        check_none("x\u{f0002}y", false);
    }

    #[test]
    fn matches_javascript_character_classes() {
        // `\s` and `trim` leave U+0085 alone and take U+FEFF.
        check("a\u{85}b", false, "a\u{85}b");
        check("x^{a\u{85}}", false, "x^(a\u{85})");
        check("\u{feff}x", false, "x");
        check("x^{\u{feff}2}", false, "x²");
        // `\p{L}` is the letter category, not the alphabetic property.
        check(r"\frac{Ⓐ}{2}", false, "(Ⓐ)/2");
        check(r"\sqrt{Ⓐ}", false, "√(Ⓐ)");
        check(r"\sqrt{Ⅻ}", false, "√Ⅻ");
        check("\\sqrt{\u{93e}}", false, "√(\u{93e})");
        check(r"Ⓐ\sin x", false, "Ⓐsin x");
        check(r"x\sin Ⓐ", false, "x sin Ⓐ");
    }
}
