//! Port of `src/width.ts` of grok-mermaid 0.2.3.
//!
//! Display width, measured in grapheme clusters.
//!
//! A cluster is the unit both of measuring and of painting, so a box is always
//! sized for exactly what gets drawn into it. Splitting those two (sizing by
//! cluster but painting by code point) makes `👨‍👩‍👧` overflow its border.
//!
//! Clustering follows UAX #29 through `unicode-segmentation`, which handles ZWJ
//! sequences, skin-tone modifiers, variation selectors, keycaps, flags and
//! Hangul. Per-code-point widths come from the table in `width_data.rs`.

use super::width_data::WIDTHS;
use unicode_segmentation::UnicodeSegmentation;

const VS16: u32 = 0xfe0f;

fn is_regional_indicator(cp: u32) -> bool {
    (0x1f1e6..=0x1f1ff).contains(&cp)
}

/// Width of one code point; the table covers the whole code point space.
fn code_point_width(cp: u32) -> usize {
    let (mut lo, mut hi) = (0usize, WIDTHS.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        let (a, b, w) = WIDTHS[mid];
        if cp < a {
            hi = mid;
        } else if cp > b {
            lo = mid + 1;
        } else {
            return usize::from(w);
        }
    }
    1
}

/// Columns occupied by one grapheme cluster.
///
/// The widest code point wins, so a base plus its combining marks measures as
/// the base. Two adjustments: a variation selector requesting emoji
/// presentation forces two columns, as does a regional indicator pair (a flag).
///
/// Zero is a real answer: a soft hyphen or zero-width space occupies nothing,
/// and callers skip painting such a cluster rather than reserving a cell.
pub(super) fn cluster_width(cluster: &str) -> usize {
    let mut w = 0;
    let mut vs16 = false;
    let mut regional = 0;
    for ch in cluster.chars() {
        let cp = u32::from(ch);
        if cp == VS16 {
            vs16 = true;
        }
        if is_regional_indicator(cp) {
            regional += 1;
        }
        w = w.max(code_point_width(cp));
    }
    if vs16 || regional >= 2 { 2 } else { w }
}

/// Iterate clusters paired with their display width.
pub(super) fn measured(s: &str) -> impl Iterator<Item = (&str, usize)> {
    s.graphemes(true).map(|g| (g, cluster_width(g)))
}

/// Display columns of a string.
pub(super) fn string_width(s: &str) -> usize {
    measured(s).map(|(_, w)| w).sum()
}
