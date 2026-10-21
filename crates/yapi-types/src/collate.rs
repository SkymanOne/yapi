//! String ordering as JavaScript's `localeCompare` gives it for identifiers and
//! file names.

use std::cmp::Ordering;

/// Primary weight of a character in ICU root collation, for ASCII: whitespace,
/// then punctuation and symbols in DUCET order, then digits, then letters
/// without case.
fn primary(c: char) -> (u8, u32) {
    const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
    if c.is_whitespace() {
        (0, c as u32)
    } else if let Some(rank) = PUNCTUATION.find(c) {
        (1, rank as u32)
    } else if c.is_ascii_digit() {
        (2, c as u32)
    } else if c.is_alphabetic() {
        (3, c.to_lowercase().next().unwrap_or(c) as u32)
    } else {
        (4, c as u32)
    }
}

/// Combining marks of Latin letters in ICU's secondary order; other marks
/// follow them by code point.
const MARKS: &str = "\u{301}\u{300}\u{306}\u{302}\u{30c}\u{30a}\u{308}\u{30b}\u{303}\u{307}\u{327}\u{328}\u{304}\u{309}\u{30f}\u{311}\u{31b}\u{323}\u{324}\u{325}\u{326}\u{32d}\u{32e}\u{330}\u{331}";

/// A collation element: the primary weight, none for a mark; the secondary
/// weight, 0 for a base letter; and whether the base letter is uppercase.
type Element = (Option<(u8, u32)>, u32, bool);

/// The collation elements of `text` after canonical decomposition, so an
/// accented letter is its base letter and its marks.
fn elements(text: &str) -> Vec<Element> {
    let mut elements = Vec::new();
    for c in text.chars() {
        unicode_normalization::char::decompose_canonical(c, |c| {
            if unicode_normalization::char::is_combining_mark(c) {
                let rank = MARKS.chars().position(|mark| mark == c);
                let rank = rank.map_or(0x100 + c as u32, |rank| rank as u32 + 1);
                elements.push((None, rank, false));
            } else {
                elements.push((Some(primary(c)), 0, c.is_uppercase()));
            }
        });
    }
    elements
}

/// `a.localeCompare(b)` for identifiers and names: primary weights first,
/// with accented letters next to their base letters, then accents, then
/// lowercase before uppercase, then code points.
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    let (a_elements, b_elements) = (elements(a), elements(b));
    let primaries = |elements: &[Element]| elements.iter().filter_map(|e| e.0).collect::<Vec<_>>();
    let secondaries = |elements: &[Element]| elements.iter().map(|e| e.1).collect::<Vec<_>>();
    let cases = |elements: &[Element]| elements.iter().map(|e| e.2).collect::<Vec<_>>();
    primaries(&a_elements)
        .cmp(&primaries(&b_elements))
        .then_with(|| secondaries(&a_elements).cmp(&secondaries(&b_elements)))
        .then_with(|| cases(&a_elements).cmp(&cases(&b_elements)))
        .then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_like_locale_compare() {
        // Orders checked against Node's localeCompare over the whole catalog,
        // and for accented names; "e\u{301}" is the decomposed "é".
        let mut ids = vec![
            "gpt-4o",
            "gpt-4.1",
            "GPT-4",
            "gpt-4",
            "gpt-40",
            "a~b",
            "a:b",
            "a/b",
            "a@b",
            "a-b",
            "Zed.txt",
            "ünï cödé.txt",
            "été",
            "Été",
            "ete",
            "être",
            "côté",
            "cote",
            "coté",
            "côte",
            "e\u{301}tf",
            "Ångström",
            "angle",
        ];
        ids.sort_by(|a, b| locale_compare(a, b));
        assert_eq!(
            ids,
            [
                "a-b",
                "a:b",
                "a@b",
                "a/b",
                "a~b",
                "angle",
                "Ångström",
                "cote",
                "coté",
                "côte",
                "côté",
                "ete",
                "été",
                "Été",
                "e\u{301}tf",
                "être",
                "gpt-4",
                "GPT-4",
                "gpt-4.1",
                "gpt-40",
                "gpt-4o",
                "ünï cödé.txt",
                "Zed.txt"
            ]
        );
    }
}
