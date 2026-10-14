//! Port of `src/canvas.ts` of grok-mermaid 0.2.3.
//!
//! The character grid every layout paints on: cells with a semantic class,
//! accumulated line-direction bits that resolve into box-drawing glyphs, and
//! the conversion of a finished grid into plain and styled rows.

use super::width::measured;
use super::{Cls, Span};

/// Sentinel occupying the trailing column of a wide glyph. Never emitted: the
/// line builder skips it so a CJK character claims two cells of layout but
/// contributes one character of output.
pub(super) const CONT: &str = "\0";

/// Connection direction bits, combined into a box-drawing glyph by `mask_char`.
pub(super) const U: u8 = 1;
pub(super) const D: u8 = 2;
pub(super) const L: u8 = 4;
pub(super) const R: u8 = 8;

/// Line styles, tracked per cell so crossing edges keep their own stroke.
pub(super) const STY_DOT: u8 = 1;
pub(super) const STY_THICK: u8 = 2;
pub(super) const STY_SOLID: u8 = 4;

/// A grid of cells. Edges accumulate as direction bits rather than glyphs so
/// that crossings and junctions resolve correctly whatever order they are
/// drawn in; `finalize_mask` turns the accumulated bits into characters at the
/// end.
///
/// `occupied` marks cells claimed by a box, which edge bits must not overwrite.
pub(super) struct Canvas {
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) ch: Vec<String>,
    pub(super) cls: Vec<Cls>,
    pub(super) mask: Vec<u8>,
    pub(super) style: Vec<u8>,
    pub(super) occupied: Vec<u8>,
    pub(super) cur_style: u8,
}

/// The rows of a finished canvas.
pub(super) struct Lines {
    pub(super) plain: Vec<String>,
    pub(super) styled: Vec<Vec<Span>>,
    pub(super) width: usize,
}

impl Canvas {
    pub(super) fn new(w: usize, h: usize) -> Canvas {
        let n = w * h;
        Canvas {
            w,
            h,
            ch: vec![" ".to_string(); n],
            cls: vec![Cls::None; n],
            mask: vec![0; n],
            style: vec![0; n],
            occupied: vec![0; n],
            cur_style: STY_SOLID,
        }
    }

    pub(super) fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    pub(super) fn set(&mut self, x: usize, y: usize, c: &str, cls: Cls) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = self.idx(x, y);
        self.ch[i] = c.to_string();
        self.cls[i] = cls;
    }

    /// Mark the cell at `(x, y)` as claimed by a box. A cell past the
    /// grid is ignored, as a write past the end of a typed array is.
    pub(super) fn occupy(&mut self, x: usize, y: usize) {
        let i = self.idx(x, y);
        if let Some(o) = self.occupied.get_mut(i) {
            *o = 1;
        }
    }

    /// Accumulate direction bits on a free cell.
    ///
    /// `cls` is the class to claim the cell for; `Border` cells are never
    /// reclassified, so a connector meeting a box keeps the box's styling.
    pub(super) fn add_bits(&mut self, x: usize, y: usize, bits: u8, cls: Cls) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = self.idx(x, y);
        if self.occupied[i] != 0 {
            return;
        }
        self.mask[i] |= bits;
        self.style[i] |= self.cur_style;
        if self.cls[i] != Cls::Border {
            self.cls[i] = cls;
        }
    }

    /// Stamp a finished sub-canvas (a subgraph frame's contents) at an offset.
    pub(super) fn blit(&mut self, sub: &Canvas, ox: usize, oy: usize) {
        for sy in 0..sub.h {
            for sx in 0..sub.w {
                let x = ox + sx;
                let y = oy + sy;
                if x >= self.w || y >= self.h {
                    continue;
                }
                let si = sub.idx(sx, sy);
                let di = self.idx(x, y);
                self.ch[di] = sub.ch[si].clone();
                self.cls[di] = sub.cls[si];
                self.style[di] = sub.style[si];
                self.occupied[di] = 1;
            }
        }
    }

    /// Add direction bits even to an occupied cell, so an edge can meet a border.
    pub(super) fn junction(&mut self, x: usize, y: usize, bits: u8) {
        if x >= self.w || y >= self.h {
            return;
        }
        let i = self.idx(x, y);
        self.mask[i] |= bits;
        if self.cls[i] != Cls::Border {
            self.cls[i] = Cls::Edge;
        }
    }

    pub(super) fn seg_v(&mut self, x: usize, y0: usize, y1: usize) {
        let a = y0.min(y1);
        let b = y0.max(y1);
        for y in a..=b {
            let mut bits = 0;
            if y > a {
                bits |= U;
            }
            if y < b {
                bits |= D;
            }
            self.add_bits(x, y, bits, Cls::Edge);
        }
    }

    pub(super) fn seg_h(&mut self, y: usize, x0: usize, x1: usize) {
        let a = x0.min(x1);
        let b = x0.max(x1);
        for x in a..=b {
            let mut bits = 0;
            if x > a {
                bits |= L;
            }
            if x < b {
                bits |= R;
            }
            self.add_bits(x, y, bits, Cls::Edge);
        }
    }

    /// Resolve accumulated direction bits into glyphs, honouring line style.
    pub(super) fn finalize_mask(&mut self) {
        for i in 0..self.ch.len() {
            if self.mask[i] != 0 && self.ch[i] == " " {
                let c = mask_char(self.mask[i]);
                self.ch[i] = match self.style[i] {
                    STY_DOT => dotted_char(c),
                    STY_THICK => thick_char(c),
                    _ => c,
                }
                .to_string();
            }
        }
    }

    /// Mirror top-to-bottom for `BT`. Rows reorder but within-row text does
    /// not, so labels stay readable; box-drawing glyphs flip to match.
    pub(super) fn flip_vertical(&mut self) {
        for y in 0..self.h / 2 {
            let y2 = self.h - 1 - y;
            for x in 0..self.w {
                let i = self.idx(x, y);
                let j = self.idx(x, y2);
                self.ch.swap(i, j);
                self.cls.swap(i, j);
            }
        }
        for c in &mut self.ch {
            if let Some(f) = flip_glyph_v(c) {
                *c = f.to_string();
            }
        }
    }

    /// Mirror left-to-right for `RL`. Mirroring reverses each row, so after
    /// flipping glyphs each text/label run is reversed back to reading order.
    pub(super) fn flip_horizontal(&mut self) {
        for y in 0..self.h {
            for x in 0..self.w / 2 {
                let x2 = self.w - 1 - x;
                let i = self.idx(x, y);
                let j = self.idx(x2, y);
                self.ch.swap(i, j);
                self.cls.swap(i, j);
            }
        }
        for c in &mut self.ch {
            if let Some(f) = flip_glyph_h(c) {
                *c = f.to_string();
            }
        }
        for y in 0..self.h {
            let mut x = 0;
            while x < self.w {
                let cls = self.cls[self.idx(x, y)];
                if cls == Cls::Text || cls == Cls::EdgeLabel {
                    let start = self.idx(x, y);
                    while x < self.w && self.cls[self.idx(x, y)] == cls {
                        x += 1;
                    }
                    let end = self.idx(x, y);
                    self.ch[start..end].reverse();
                } else {
                    x += 1;
                }
            }
        }
    }

    /// Group each row into runs of one class, dropping wide-glyph continuations.
    pub(super) fn to_lines(&self) -> Lines {
        let mut plain: Vec<String> = Vec::new();
        let mut styled: Vec<Vec<Span>> = Vec::new();
        let mut width = 0;
        for y in 0..self.h {
            // A trailing CONT counts as painted: it is the second cell of a
            // wide glyph, so the row really does reach that column.
            let mut last = 0;
            for x in (0..self.w).rev() {
                if self.ch[self.idx(x, y)] != " " {
                    last = x + 1;
                    break;
                }
            }
            width = width.max(last);
            let mut spans: Vec<Span> = Vec::new();
            let mut plain_row = String::new();
            let mut run = String::new();
            let mut run_cls = Cls::None;
            for x in 0..last {
                let i = self.idx(x, y);
                let c = &self.ch[i];
                if c == CONT {
                    continue;
                }
                let cls = self.cls[i];
                plain_row.push_str(c);
                if cls != run_cls && !run.is_empty() {
                    spans.push(Span {
                        text: std::mem::take(&mut run),
                        cls: run_cls,
                    });
                }
                run_cls = cls;
                run.push_str(c);
            }
            if !run.is_empty() {
                spans.push(Span {
                    text: run,
                    cls: run_cls,
                });
            }
            styled.push(spans);
            // Only ASCII spaces, which is all a blank cell ever holds. Trimming
            // `\s` would eat a trailing NBSP that `styled` keeps, desyncing the two.
            plain.push(plain_row.trim_end_matches(' ').to_string());
        }
        let mut first = 0;
        while first < plain.len() && plain[first].is_empty() {
            first += 1;
        }
        let mut end = plain.len();
        while end > first && plain[end - 1].is_empty() {
            end -= 1;
        }
        Lines {
            plain: plain[first..end].to_vec(),
            styled: styled[first..end].to_vec(),
            width,
        }
    }
}

/// Paint `text` at `x, y`, one grapheme cluster per cell.
///
/// A wide cluster claims a second cell, marked with `CONT` so the line builder
/// emits one character for it rather than a stray space.
pub(super) fn draw_text(canvas: &mut Canvas, text: &str, x: usize, y: usize, cls: Cls) {
    let mut cur = x;
    for (cluster, cw) in measured(text) {
        if cw == 0 {
            continue;
        }
        canvas.set(cur, y, cluster, cls);
        for k in 1..cw {
            canvas.set(cur + k, y, CONT, cls);
        }
        cur += cw;
    }
}

/// Paint `text` at `x, y`, clearing any edge bits underneath first.
///
/// Used where text sits on top of a drawn line (sequence messages, dividers,
/// compartment rows) and must win over it.
pub(super) fn draw_text_over_edges(canvas: &mut Canvas, text: &str, x: usize, y: usize, cls: Cls) {
    let mut cur = x;
    for (cluster, cw) in measured(text) {
        if cw == 0 {
            continue;
        }
        for k in 0..cw {
            if cur + k < canvas.w && y < canvas.h {
                let i = canvas.idx(cur + k, y);
                canvas.mask[i] = 0;
            }
            canvas.set(cur + k, y, if k == 0 { cluster } else { CONT }, cls);
        }
        cur += cw;
    }
}

pub(super) fn mask_char(mask: u8) -> &'static str {
    match mask {
        0 => " ",
        m if m == U || m == D || m == U | D => "│",
        m if m == L || m == R || m == L | R => "─",
        m if m == D | R => "┌",
        m if m == D | L => "┐",
        m if m == U | R => "└",
        m if m == U | L => "┘",
        m if m == U | D | R => "├",
        m if m == U | D | L => "┤",
        m if m == D | L | R => "┬",
        m if m == U | L | R => "┴",
        _ => "┼",
    }
}

fn dotted_char(c: &'static str) -> &'static str {
    match c {
        "─" => "╌",
        "│" => "╎",
        _ => c,
    }
}

fn thick_char(c: &'static str) -> &'static str {
    match c {
        "─" => "━",
        "│" => "┃",
        "┌" => "┏",
        "┐" => "┓",
        "└" => "┗",
        "┘" => "┛",
        "├" => "┣",
        "┤" => "┫",
        "┬" => "┳",
        "┴" => "┻",
        "┼" => "╋",
        _ => c,
    }
}

fn flip_glyph_v(c: &str) -> Option<&'static str> {
    Some(match c {
        "┌" => "└",
        "└" => "┌",
        "┐" => "┘",
        "┘" => "┐",
        "┏" => "┗",
        "┗" => "┏",
        "┓" => "┛",
        "┛" => "┓",
        "╭" => "╰",
        "╰" => "╭",
        "╮" => "╯",
        "╯" => "╮",
        "┬" => "┴",
        "┴" => "┬",
        "┳" => "┻",
        "┻" => "┳",
        "▼" => "▲",
        "▲" => "▼",
        "▽" => "△",
        "△" => "▽",
        _ => return None,
    })
}

fn flip_glyph_h(c: &str) -> Option<&'static str> {
    Some(match c {
        "┌" => "┐",
        "┐" => "┌",
        "└" => "┘",
        "┘" => "└",
        "┏" => "┓",
        "┓" => "┏",
        "┗" => "┛",
        "┛" => "┗",
        "╭" => "╮",
        "╮" => "╭",
        "╰" => "╯",
        "╯" => "╰",
        "├" => "┤",
        "┤" => "├",
        "┣" => "┫",
        "┫" => "┣",
        "▶" => "◄",
        "◄" => "▶",
        "▷" => "◁",
        "◁" => "▷",
        _ => return None,
    })
}
