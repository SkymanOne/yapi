//! String ordering as JavaScript's `localeCompare` gives it for identifiers and
//! file names.

use std::cmp::Ordering;

/// Primary weight of a character in ICU root collation: whitespace, then
/// ASCII punctuation and symbols in DUCET order, then digits, then letters
/// without case, and then any other character by code point.
fn primary(c: char) -> (u8, u32) {
    const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
    if c.is_whitespace() {
        (0, c as u32)
    } else if let Some(rank) = PUNCTUATION.find(c) {
        (1, rank as u32)
    } else if c.is_ascii_digit() {
        (2, c as u32)
    } else if c.is_alphabetic() {
        letter(c.to_lowercase().next().unwrap_or(c), 0)
    } else {
        (4, c as u32)
    }
}

/// The primary weight of the `after`th letter ICU places after `letter`, or
/// of `letter` itself for 0.
fn letter(letter: char, after: u8) -> (u8, u32) {
    (3, (letter as u32) << 8 | u32::from(after))
}

/// A collation element of a [`LETTERS`] entry, with DUCET's tertiary weight.
enum Weight {
    /// The `.1`th letter after `.0` (0 for `.0` itself).
    Letter(char, u8, u8),
    /// An accent's secondary weight.
    Accent(u32, u8),
}

use Weight::{Accent as A, Letter as L};

/// The Latin letters of U+00C0 to U+017F, and ẞ, that have no canonical
/// decomposition, with their collation elements from DUCET 13: a stroke is
/// an accent, a ligature is its letters, and þ, ı, ŋ and similar letters
/// sort after a base letter. Sorted by letter.
const LETTERS: &[(char, &[Weight])] = &[
    ('Æ', &[L('a', 0, 10), A(0x118, 4), L('e', 0, 10)]),
    ('Ð', &[L('d', 0, 10), A(0x118, 4)]),
    ('Ø', &[L('o', 0, 8), A(0x2F, 2)]),
    ('Þ', &[L('z', 1, 8)]),
    ('ß', &[L('s', 0, 4), A(0x118, 4), L('s', 0, 4)]),
    ('æ', &[L('a', 0, 4), A(0x118, 4), L('e', 0, 4)]),
    ('ð', &[L('d', 0, 4), A(0x118, 4)]),
    ('ø', &[L('o', 0, 2), A(0x2F, 2)]),
    ('þ', &[L('z', 1, 2)]),
    ('Đ', &[L('d', 0, 8), A(0x39, 2)]),
    ('đ', &[L('d', 0, 2), A(0x39, 2)]),
    ('Ħ', &[L('h', 0, 8), A(0x39, 2)]),
    ('ħ', &[L('h', 0, 2), A(0x39, 2)]),
    ('ı', &[L('i', 1, 2)]),
    ('Ĳ', &[L('i', 0, 10), L('j', 0, 10)]),
    ('ĳ', &[L('i', 0, 4), L('j', 0, 4)]),
    ('ĸ', &[L('q', 1, 2)]),
    ('Ŀ', &[L('l', 0, 8), A(0x118, 2)]),
    ('ŀ', &[L('l', 0, 2), A(0x118, 2)]),
    ('Ł', &[L('l', 0, 8), A(0x39, 2)]),
    ('ł', &[L('l', 0, 2), A(0x39, 2)]),
    ('ŉ', &[L('z', 2, 4), L('n', 0, 4)]),
    ('Ŋ', &[L('n', 1, 8)]),
    ('ŋ', &[L('n', 1, 2)]),
    ('Œ', &[L('o', 0, 10), A(0x118, 4), L('e', 0, 10)]),
    ('œ', &[L('o', 0, 4), A(0x118, 4), L('e', 0, 4)]),
    ('Ŧ', &[L('t', 1, 8)]),
    ('ŧ', &[L('t', 1, 2)]),
    ('ſ', &[L('s', 0, 4), A(0x119, 4)]),
    ('ẞ', &[L('s', 0, 10), A(0x118, 4), L('s', 0, 10)]),
];

/// The DUCET secondary weights of the combining marks U+0300 to U+0362; 0
/// for one that is ignored.
const MARKS: [u8; 0x63] = [
    0x25, 0x24, 0x27, 0x2D, 0x32, 0x3A, 0x26, 0x2E, 0x2B, 0x3B, 0x29, 0x2C, 0x28, 0x33, 0x33, 0x3C,
    0x3D, 0x3E, 0x33, 0x22, 0x23, 0x33, 0x34, 0x34, 0x34, 0x34, 0x33, 0x3F, 0x34, 0x34, 0x34, 0x34,
    0x34, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x30, 0x31, 0x34, 0x34, 0x34, 0x34, 0x46, 0x47, 0x34,
    0x48, 0x49, 0x21, 0x34, 0x4A, 0x39, 0x35, 0x35, 0x2F, 0x4B, 0x34, 0x34, 0x34, 0x33, 0x33, 0x33,
    0x25, 0x24, 0x2A, 0x22, 0x2B, 0x4C, 0x33, 0x34, 0x34, 0x34, 0x33, 0x33, 0x33, 0x34, 0x34, 0x0,
    0x33, 0x33, 0x33, 0x34, 0x34, 0x34, 0x34, 0x33, 0x4D, 0x34, 0x34, 0x33, 0x34, 0x33, 0x33, 0x34,
    0x4E, 0x4F, 0x34,
];

/// The secondary and tertiary weights of a letter without accents.
const COMMON: (u32, u8) = (0x20, 2);

/// A collation element: the primary weight, none for an accent; and the
/// secondary and tertiary weights.
type Element = (Option<(u8, u32)>, u32, u8);

/// The collation elements of `text` in canonical decomposition, so an
/// accented letter is its base letter and its marks.
fn elements(text: &str) -> Vec<Element> {
    use unicode_normalization::UnicodeNormalization;
    let mut elements = Vec::new();
    for c in text.nfd() {
        if let Ok(index) = LETTERS.binary_search_by_key(&c, |(letter, _)| *letter) {
            elements.extend(LETTERS[index].1.iter().map(|weight| match *weight {
                L(base, after, tertiary) => (Some(letter(base, after)), COMMON.0, tertiary),
                A(secondary, tertiary) => (None, secondary, tertiary),
            }));
        } else if unicode_normalization::char::is_combining_mark(c) {
            // Marks DUCET does not list here follow its marks by code point.
            let secondary = (c as usize)
                .checked_sub(0x300)
                .and_then(|index| MARKS.get(index))
                .map_or(0x200 + c as u32, |&weight| u32::from(weight));
            if secondary != 0 {
                elements.push((None, secondary, COMMON.1));
            }
        } else {
            let tertiary = if c.is_uppercase() { 8 } else { COMMON.1 };
            elements.push((Some(primary(c)), COMMON.0, tertiary));
        }
    }
    elements
}

/// `a.localeCompare(b)` for identifiers and names: primary weights first,
/// with accented letters next to their base letters, then accents, then case
/// and ligatures. Texts ICU weighs alike, such as `đ` and `d` with a combining
/// stroke, are equal.
pub fn locale_compare(a: &str, b: &str) -> Ordering {
    let (a_elements, b_elements) = (elements(a), elements(b));
    let primaries = |elements: &[Element]| elements.iter().filter_map(|e| e.0).collect::<Vec<_>>();
    let secondaries = |elements: &[Element]| elements.iter().map(|e| e.1).collect::<Vec<_>>();
    let tertiaries = |elements: &[Element]| elements.iter().map(|e| e.2).collect::<Vec<_>>();
    primaries(&a_elements)
        .cmp(&primaries(&b_elements))
        .then_with(|| secondaries(&a_elements).cmp(&secondaries(&b_elements)))
        .then_with(|| tertiaries(&a_elements).cmp(&tertiaries(&b_elements)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_like_locale_compare() {
        // Orders checked against Node's localeCompare over the whole catalog,
        // and for names with accents and Latin letters such as ø, ß and þ;
        // "e\u{301}" is the decomposed "é".
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
            "ørsted",
            "oz",
            "straße",
            "strasse",
            "stras",
            "Łódź",
            "lz",
            "æble",
            "aeble",
            "afar",
            "þorn",
            "zebra",
            "ŋa",
            "nz",
            "đuro",
            "dz",
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
                "aeble",
                "æble",
                "afar",
                "angle",
                "Ångström",
                "cote",
                "coté",
                "côte",
                "côté",
                "đuro",
                "dz",
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
                "Łódź",
                "lz",
                "nz",
                "ŋa",
                "ørsted",
                "oz",
                "stras",
                "strasse",
                "straße",
                "ünï cödé.txt",
                "zebra",
                "Zed.txt",
                "þorn"
            ]
        );
        // Texts ICU weighs alike are equal; the table is sorted for its search.
        assert_eq!(locale_compare("đ", "d\u{335}"), Ordering::Equal);
        assert!(LETTERS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
}
