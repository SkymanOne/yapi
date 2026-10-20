//! Writing frames of styled lines to the terminal.
//!
//! [`MainScreen`] is pi's regular mode (`packages/tui/src/tui-main-screen.ts`
//! in pi `v1.0.0`): the document flows into the terminal's scrollback and only
//! changed lines are rewritten. [`AltScreen`] is the fullscreen mode
//! (`tui-alt-screen.ts`): a scrolling transcript above a dock pinned to the
//! bottom of the alternate screen.

use std::time::{Duration, Instant};

use indexmap::IndexMap;
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::{Line, Span};

use crate::ansi::{line_to_ansi, parse_line};
use crate::image::{self, is_image, is_image_line};
use crate::lines::{StyledLine, composite, plain, raw, truncate, width as line_width};
pub use selection::{MouseAction, MouseEvent, MouseKind, WheelScroll};
/// pi's `fullscreenScrollbar` setting.
pub use yapi_types::settings::Scrollbar;

mod selection;

const SYNC_START: &str = "\x1b[?2026h";
const SYNC_END: &str = "\x1b[?2026l";

fn clip(line: &StyledLine, width: usize) -> String {
    if line_width(line) > width && !is_image_line(line) {
        line_to_ansi(&truncate(line, width, ""))
    } else {
        line_to_ansi(line)
    }
}

/// A frame cursor position: row in the rendered lines and column.
pub(crate) type Cursor = Option<(usize, usize)>;

/// A component drawn over the screen, as pi-tui's overlays are: its rows from
/// screen row `row` and column `col`, `width` columns wide.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    /// The first screen row.
    pub row: usize,
    /// The first column.
    pub col: usize,
    /// Its width in columns.
    pub width: usize,
    /// Its rows.
    pub lines: Vec<StyledLine>,
}

/// Draws `overlays` over `rows`, whose first row is screen row 0.
fn composite_overlays(rows: &mut [StyledLine], overlays: &[Overlay], width: usize) {
    for overlay in overlays {
        for (index, line) in overlay.lines.iter().enumerate() {
            if let Some(row) = rows.get_mut(overlay.row + index) {
                *row = composite(row, line, overlay.col, overlay.width, width);
            }
        }
    }
}

/// Regular mode: the document is written into the main screen and scrollback.
#[derive(Debug, Default)]
pub struct MainScreen {
    previous: Vec<String>,
    /// The Kitty image ids in `previous`.
    previous_kitty: Vec<u32>,
    previous_width: usize,
    previous_height: usize,
    hardware_cursor_row: usize,
    max_lines_rendered: usize,
    previous_viewport_top: usize,
    /// Redraw everything when the document shrinks (`terminal.clearOnShrink`).
    pub clear_on_shrink: bool,
    /// Show the terminal cursor at the frame cursor.
    pub show_hardware_cursor: bool,
    /// Components drawn over the visible rows.
    pub overlays: Vec<Overlay>,
}

/// Moves the cursor `diff` rows down, or up when negative.
fn move_rows(out: &mut String, diff: isize) {
    if diff > 0 {
        out.push_str(&format!("\x1b[{diff}B"));
    } else if diff < 0 {
        out.push_str(&format!("\x1b[{}A", -diff));
    }
}

/// The Kitty image ids `lines` draw, each once.
fn kitty_ids<'a>(lines: impl IntoIterator<Item = &'a String>) -> Vec<u32> {
    let mut ids = Vec::new();
    for line in lines {
        for id in image::kitty_header(line)
            .map(|(ids, _)| ids)
            .unwrap_or_default()
        {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

fn delete_kitty(ids: &[u32]) -> String {
    ids.iter().map(|id| image::delete_kitty(*id)).collect()
}

/// pi's `getKittyImageReservedRows`: the rows from `index` that a Kitty
/// image there covers, up to its `r` rows and `max_index`, while they stay
/// blank.
fn reserved_rows(lines: &[String], index: usize, max_index: usize) -> usize {
    let rows = image::kitty_header(&lines[index]).map_or(1, |(_, rows)| rows);
    let max = rows
        .min((max_index + 1).saturating_sub(index))
        .min(lines.len() - index);
    let mut reserved = 1;
    while reserved < max {
        let line = &lines[index + reserved];
        if is_image(line) || line_width(&parse_line(line).0) > 0 {
            break;
        }
        reserved += 1;
    }
    reserved
}

/// pi's `expandChangedRangeForKittyImages`: rows `first..=last` grown to
/// cover every Kitty image in `old` or `new` that they touch or that
/// follows them, so its placement is deleted and drawn again whole.
fn expand_for_images(first: usize, last: usize, old: &[String], new: &[String]) -> (usize, usize) {
    let (mut from, mut to) = (first, last);
    for lines in [old, new] {
        for (index, line) in lines.iter().enumerate() {
            if image::kitty_header(line).is_none_or(|(ids, _)| ids.is_empty()) {
                continue;
            }
            let end = index + reserved_rows(lines, index, lines.len() - 1) - 1;
            if index >= first || (index <= last && end >= first) {
                from = from.min(index);
                to = to.max(end);
            }
        }
    }
    (from, to)
}

/// Writes `line` and the blank rows below it that the image it draws
/// covers, `reserved` in all, clearing them first when `clear`, so the
/// image is placed once the rows exist.
fn push_image_block(out: &mut String, line: &str, reserved: usize, clear: bool) {
    let row = if clear { "\r\n\x1b[2K" } else { "\r\n" };
    if clear {
        out.push_str("\x1b[2K");
    }
    out.push_str(&row.repeat(reserved - 1));
    out.push_str(&format!(
        "\x1b[{}A{line}\x1b[{}B",
        reserved - 1,
        reserved - 1
    ));
}

impl MainScreen {
    /// A renderer that has drawn nothing yet.
    pub fn new() -> MainScreen {
        MainScreen::default()
    }

    /// Forgets what was drawn, so the next frame is drawn in full.
    pub fn reset(&mut self) {
        let keep = (self.clear_on_shrink, self.show_hardware_cursor);
        *self = MainScreen::default();
        (self.clear_on_shrink, self.show_hardware_cursor) = keep;
    }

    fn position_cursor(&mut self, out: &mut String, cursor: Cursor, total: usize) {
        let Some((row, col)) = cursor.filter(|_| total > 0) else {
            out.push_str("\x1b[?25l");
            return;
        };
        let target = row.min(total - 1);
        move_rows(out, target as isize - self.hardware_cursor_row as isize);
        out.push_str(&format!("\x1b[{}G", col + 1));
        self.hardware_cursor_row = target;
        out.push_str(if self.show_hardware_cursor {
            "\x1b[?25h"
        } else {
            "\x1b[?25l"
        });
    }

    fn full_render(
        &mut self,
        out: &mut String,
        lines: Vec<String>,
        clear: bool,
        cursor: Cursor,
        width: usize,
        height: usize,
    ) {
        out.push_str(SYNC_START);
        if clear {
            out.push_str(&delete_kitty(&self.previous_kitty));
            out.push_str("\x1b[2J\x1b[H\x1b[3J");
        }
        let mut index = 0;
        while index < lines.len() {
            if index > 0 {
                out.push_str("\r\n");
            }
            let line = &lines[index];
            let reserved = if is_image(line) {
                reserved_rows(&lines, index, lines.len() - 1)
            } else {
                1
            };
            if reserved > 1 && reserved <= height {
                push_image_block(out, line, reserved, false);
                index += reserved;
                continue;
            }
            out.push_str(line);
            index += 1;
        }
        out.push_str(SYNC_END);
        self.hardware_cursor_row = lines.len().saturating_sub(1);
        self.max_lines_rendered = if clear {
            lines.len()
        } else {
            self.max_lines_rendered.max(lines.len())
        };
        self.previous_viewport_top = height.max(lines.len()) - height;
        self.position_cursor(out, cursor, lines.len());
        self.previous_kitty = kitty_ids(&lines);
        self.previous = lines;
        self.previous_width = width;
        self.previous_height = height;
    }

    /// pi's `deleteChangedKittyImages`: deletes the Kitty images drawn on
    /// rows `first..=last` of the previous frame.
    fn delete_changed(&self, first: usize, last: usize) -> String {
        let end = (last + 1).min(self.previous.len());
        delete_kitty(&kitty_ids(
            self.previous.get(first..end).unwrap_or_default(),
        ))
    }

    /// Draws a frame of `width` by `height` and returns the bytes to write.
    pub fn frame(
        &mut self,
        document: &[StyledLine],
        cursor: Cursor,
        width: usize,
        height: usize,
    ) -> String {
        let mut out = String::new();
        let lines: Vec<String> = if self.overlays.is_empty() {
            document.iter().map(|line| clip(line, width)).collect()
        } else {
            // As pi-tui, the document grows to the screen's height so
            // overlays sit at screen positions.
            let needed = self
                .overlays
                .iter()
                .map(|overlay| overlay.row + overlay.lines.len())
                .fold(document.len().max(height), usize::max);
            let mut rows = document.to_vec();
            rows.resize(needed, Line::default());
            let start = needed - height;
            composite_overlays(&mut rows[start..], &self.overlays, width);
            rows.iter().map(|line| clip(line, width)).collect()
        };
        // Only a cursor within the visible viewport counts.
        let viewport_start = lines.len().saturating_sub(height);
        let cursor = cursor.filter(|(row, _)| *row >= viewport_start && *row < lines.len());
        let width_changed = self.previous_width != 0 && self.previous_width != width;
        let height_changed = self.previous_height != 0 && self.previous_height != height;
        let previous_buffer = if self.previous_height > 0 {
            self.previous_viewport_top + self.previous_height
        } else {
            height
        };
        let mut prev_top = if height_changed {
            previous_buffer.saturating_sub(height)
        } else {
            self.previous_viewport_top
        };
        let mut top = prev_top;
        let mut hardware_row = self.hardware_cursor_row;

        // The first frame draws without clearing; a resize, or a shrink with
        // `clearOnShrink`, redraws everything.
        let first_frame = self.previous.is_empty() && !width_changed && !height_changed;
        if first_frame
            || width_changed
            || height_changed
            || (self.clear_on_shrink && lines.len() < self.max_lines_rendered)
        {
            self.full_render(&mut out, lines, !first_frame, cursor, width, height);
            return out;
        }

        let longest = lines.len().max(self.previous.len());
        let mut first = None;
        let mut last = 0;
        for index in 0..longest {
            let old = self.previous.get(index).map_or("", String::as_str);
            let new = lines.get(index).map_or("", String::as_str);
            if old != new {
                first.get_or_insert(index);
                last = index;
            }
        }
        let appended = lines.len() > self.previous.len();
        if appended {
            first.get_or_insert(self.previous.len());
            last = lines.len() - 1;
        }
        if let Some(changed) = first {
            let (from, to) = expand_for_images(changed, last, &self.previous, &lines);
            first = Some(from);
            last = to;
        }
        let Some(first) = first else {
            self.position_cursor(&mut out, cursor, lines.len());
            self.previous_viewport_top = prev_top;
            self.previous_height = height;
            return out;
        };
        let append_start = appended && first == self.previous.len() && first > 0;
        let line_diff =
            |target: usize, hardware_row: usize, prev_top: usize, top: usize| -> isize {
                (target as isize - top as isize) - (hardware_row as isize - prev_top as isize)
            };

        if first >= lines.len() {
            // Only deletions at the end.
            if self.previous.len() > lines.len() {
                let target = lines.len().saturating_sub(1);
                if target < prev_top {
                    self.full_render(&mut out, lines, true, cursor, width, height);
                    return out;
                }
                let extra = self.previous.len() - lines.len();
                if extra > height {
                    self.full_render(&mut out, lines, true, cursor, width, height);
                    return out;
                }
                out.push_str(SYNC_START);
                out.push_str(&self.delete_changed(first, last));
                move_rows(&mut out, line_diff(target, hardware_row, prev_top, top));
                out.push('\r');
                let start_offset = usize::from(!lines.is_empty());
                if start_offset > 0 {
                    out.push_str(&format!("\x1b[{start_offset}B"));
                }
                for index in 0..extra {
                    out.push_str("\r\x1b[2K");
                    if index + 1 < extra {
                        out.push_str("\x1b[1B");
                    }
                }
                let back = extra - 1 + start_offset;
                if back > 0 {
                    out.push_str(&format!("\x1b[{back}A"));
                }
                out.push_str(SYNC_END);
                self.hardware_cursor_row = target;
            }
            self.position_cursor(&mut out, cursor, lines.len());
            self.previous_kitty = kitty_ids(&lines);
            self.previous = lines;
            self.previous_width = width;
            self.previous_height = height;
            self.previous_viewport_top = prev_top;
            return out;
        }

        if first < prev_top {
            // The change scrolled out of view: redraw the whole document.
            self.full_render(&mut out, lines, true, cursor, width, height);
            return out;
        }

        out.push_str(SYNC_START);
        out.push_str(&self.delete_changed(first, last));
        let prev_bottom = prev_top + height - 1;
        let move_target = if append_start { first - 1 } else { first };
        if move_target > prev_bottom {
            let screen_row = hardware_row.saturating_sub(prev_top).min(height - 1);
            let to_bottom = height - 1 - screen_row;
            if to_bottom > 0 {
                out.push_str(&format!("\x1b[{to_bottom}B"));
            }
            let scroll = move_target - prev_bottom;
            out.push_str(&"\r\n".repeat(scroll));
            prev_top += scroll;
            top += scroll;
            hardware_row = move_target;
        }
        move_rows(
            &mut out,
            line_diff(move_target, hardware_row, prev_top, top),
        );
        out.push_str(if append_start { "\r\n" } else { "\r" });
        let render_end = last.min(lines.len() - 1);
        let mut index = first;
        while index <= render_end {
            if index > first {
                out.push_str("\r\n");
            }
            let line = &lines[index];
            let reserved = if is_image(line) {
                reserved_rows(&lines, index, render_end)
            } else {
                1
            };
            if reserved > 1 {
                // Clearing the image's rows must not scroll the screen.
                let row = index
                    .checked_sub(top)
                    .filter(|row| row + reserved <= height);
                if row.is_none() {
                    out.clear();
                    self.full_render(&mut out, lines, true, cursor, width, height);
                    return out;
                }
                push_image_block(&mut out, line, reserved, true);
                index += reserved;
                continue;
            }
            out.push_str("\x1b[2K");
            out.push_str(line);
            index += 1;
        }
        let mut final_row = render_end;
        if self.previous.len() > lines.len() {
            if render_end < lines.len() - 1 {
                out.push_str(&format!("\x1b[{}B", lines.len() - 1 - render_end));
                final_row = lines.len() - 1;
            }
            let extra = self.previous.len() - lines.len();
            for _ in 0..extra {
                out.push_str("\r\n\x1b[2K");
            }
            out.push_str(&format!("\x1b[{extra}A"));
        }
        out.push_str(SYNC_END);
        self.hardware_cursor_row = final_row;
        self.max_lines_rendered = self.max_lines_rendered.max(lines.len());
        self.previous_viewport_top = prev_top.max((final_row + 1).saturating_sub(height));
        self.position_cursor(&mut out, cursor, lines.len());
        self.previous_kitty = kitty_ids(&lines);
        self.previous = lines;
        self.previous_width = width;
        self.previous_height = height;
        out
    }

    /// Moves below the document before the program exits or hands over the
    /// terminal.
    pub fn stop(&mut self) -> String {
        if self.previous.is_empty() {
            return String::new();
        }
        let mut out = String::from(" ");
        let target = self.previous.len();
        move_rows(
            &mut out,
            target as isize - self.hardware_cursor_row as isize,
        );
        out.push_str("\r\n");
        out
    }
}

/// pi's `replaceScrollbarCell` for the last column: the line up to it, a
/// wide character crossing it given up for spaces, then `glyph`, over the
/// cell's own background when `keep_background`.
fn replace_last_cell(
    line: &StyledLine,
    width: usize,
    glyph: &str,
    style: Style,
    keep_background: bool,
) -> StyledLine {
    if is_image_line(line) {
        return line.clone();
    }
    let column = width - 1;
    let mut background = None;
    let mut at = 0;
    for span in &line.spans {
        let span_width = line_width(&Line::from(span.clone()));
        if at + span_width > column {
            background = span.style.bg;
            break;
        }
        at += span_width;
    }
    let mut out = truncate(line, column, "");
    let short = column.saturating_sub(line_width(&out));
    if short > 0 {
        out.spans.push(Span::raw(" ".repeat(short)));
    }
    let mut style = style;
    if keep_background && let Some(background) = background {
        style = style.bg(background);
    }
    out.spans.push(Span::styled(glyph.to_owned(), style));
    out
}

/// pi-tui's `allocateStackSizes` for a stack that only shrinks: the heights
/// of parts with natural heights `sizes` and minimums `minimums` in
/// `available` rows. Each pass takes from every part above its minimum in
/// proportion to its height, at least one row each, until the stack fits.
pub(crate) fn shrink_stack(sizes: &[usize], minimums: &[usize], available: usize) -> Vec<usize> {
    let mut sizes: Vec<usize> = sizes
        .iter()
        .zip(minimums)
        .map(|(size, min)| (*size).max(*min))
        .collect();
    let total: usize = sizes.iter().sum();
    let mut remaining = total.saturating_sub(available);
    while remaining > 0 {
        let candidates: Vec<usize> = (0..sizes.len())
            .filter(|index| sizes[*index] > minimums[*index])
            .collect();
        if candidates.is_empty() {
            break;
        }
        let total_weight: usize = candidates.iter().map(|index| sizes[*index].max(1)).sum();
        let mut distributed = 0;
        for index in candidates {
            if remaining == 0 {
                break;
            }
            let weight = sizes[index].max(1);
            let proposed = (remaining * weight / total_weight).max(1);
            let delta = remaining.min(proposed).min(sizes[index] - minimums[index]);
            if delta == 0 {
                continue;
            }
            sizes[index] -= delta;
            remaining -= delta;
            distributed += delta;
        }
        if distributed == 0 {
            break;
        }
    }
    sizes
}

/// One part of a stack: its rows and its minimum height.
pub type StackPart = (Vec<StyledLine>, usize);

/// Where a stack part landed: its first row in the stack, its first row
/// shown and how many of its rows show.
pub type Placement = (usize, usize, usize);

/// Lays out a vertical stack of `parts`, each its rows and minimum height,
/// in `available` rows, as pi-tui's layout does: a part shorter than its
/// minimum gets blank rows below it, parts shrink by [`shrink_stack`], and
/// a cut part keeps its top rows unless the cursor,
/// given as part, row and column, would fall below them. Returns the rows,
/// the cursor within them and where each part landed.
pub fn fit_stack(
    parts: Vec<StackPart>,
    cursor: Option<(usize, usize, usize)>,
    available: usize,
) -> (Vec<StyledLine>, Option<(usize, usize)>, Vec<Placement>) {
    let natural: Vec<usize> = parts
        .iter()
        .map(|(rows, min)| rows.len().max(*min))
        .collect();
    let minimums: Vec<usize> = parts.iter().map(|(_, min)| *min).collect();
    let sizes = if natural.iter().sum::<usize>() > available {
        shrink_stack(&natural, &minimums, available)
    } else {
        natural
    };
    let mut out = Vec::new();
    let mut frame_cursor = None;
    let mut placements = Vec::new();
    for (index, ((mut rows, _), size)) in parts.into_iter().zip(sizes).enumerate() {
        let cursor_row = cursor.filter(|(part, _, _)| *part == index);
        let offset = match cursor_row {
            Some((_, row, _)) if rows.len() > size && size > 0 && row >= size => row + 1 - size,
            _ => 0,
        };
        if let Some((_, row, col)) = cursor_row
            && row >= offset
            && row < offset + size
        {
            frame_cursor = Some((out.len() + row - offset, col));
        }
        rows.resize(rows.len().max(offset + size), Line::default());
        placements.push((out.len(), offset, size));
        out.extend(rows.drain(offset..offset + size));
    }
    // Minimums can exceed the room; the stack's rectangle clips its bottom.
    out.truncate(available);
    (
        out,
        frame_cursor.filter(|(row, _)| *row < available),
        placements,
    )
}

/// Fullscreen mode: the transcript scrolls above a dock pinned to the bottom.
#[derive(Debug, Default)]
pub struct AltScreen {
    previous: Vec<String>,
    previous_size: (usize, usize),
    /// The cursor bytes of the last frame.
    previous_cursor: String,
    /// First transcript row shown; `None` follows the end.
    scroll_top: Option<usize>,
    viewport: usize,
    transcript_len: usize,
    /// Show the terminal cursor at the frame cursor.
    pub show_hardware_cursor: bool,
    /// Style of the "jump to latest" label.
    pub jump_label_style: Style,
    /// Key shown in the "jump to latest" label.
    pub bottom_key: String,
    /// Components drawn over the screen.
    pub overlays: Vec<Overlay>,
    /// The terminal shows Kitty images, so the screen deletes and reuses
    /// their uploads.
    pub kitty_images: bool,
    /// Kitty images the terminal holds, the least recently shown first.
    uploaded: IndexMap<u32, Upload>,
    /// When the transcript's scrollbar shows; pi's `fullscreenScrollbar`.
    pub scrollbar: Scrollbar,
    /// Style of the scrollbar's track.
    pub scrollbar_track: Style,
    /// Style of the scrollbar's thumb.
    pub scrollbar_thumb: Style,
    /// Until when an `auto` scrollbar shows after scrolling.
    scrollbar_until: Option<Instant>,
    /// Copy a selection when the mouse button is released; pi's
    /// `fullscreenCopyOnSelect`.
    pub copy_on_select: bool,
    /// Rows a wheel event scrolls, or `None` for `auto`; pi's
    /// `fullscreenWheelScrollLines`.
    pub wheel_lines: Option<usize>,
    /// The wheel's acceleration in `auto` mode.
    pub wheel: WheelScroll,
    /// The pointer is over the scrollbar, which shows its thumb solid.
    scrollbar_hover: bool,
    /// The "jump to latest" label's row, first column and width.
    jump_label: Option<(usize, usize, usize)>,
    /// The rows of the last frame.
    screen: Vec<StyledLine>,
    /// The width and height of the last frame.
    size: (usize, usize),
    selection: selection::Selection,
    /// Messages at the top right, and when each goes.
    flashes: Vec<(String, Instant)>,
}

/// A Kitty image the terminal holds: pi's `CachedKittyImage`.
#[derive(Clone, Copy, Debug)]
struct Upload {
    generation: u64,
    bytes: usize,
    decoded_bytes: u64,
}

/// How many Kitty images out of view, and how much of their data, the
/// terminal keeps for placing again without an upload.
const MAX_OFFSCREEN_IMAGES: usize = 16;
const MAX_OFFSCREEN_BYTES: usize = 32 * 1024 * 1024;
const MAX_OFFSCREEN_DECODED_BYTES: u64 = 64 * 1024 * 1024;

/// The rows the Kitty image `line` draws was rendered with.
fn kitty_rows(line: &StyledLine) -> Option<usize> {
    is_image_line(line)
        .then(|| image::kitty_rows(&plain(line)))
        .flatten()
}

fn crop(line: &StyledLine, hidden: usize, visible: usize) -> StyledLine {
    raw(image::crop_kitty(&plain(line), hidden, visible))
}

/// WezTerm erases the cells of a Kitty image that a later line clear
/// covers.
fn is_wezterm() -> bool {
    std::env::var_os("WEZTERM_PANE").is_some()
        || std::env::var("TERM_PROGRAM").is_ok_and(|name| name.eq_ignore_ascii_case("wezterm"))
}

/// How long pi's `auto` scrollbar stays after the view scrolls.
const SCROLLBAR_HIDE_DELAY: Duration = Duration::from_millis(1000);

/// How long pi's flash messages show.
pub const FLASH_DURATION: Duration = Duration::from_millis(1000);
/// How long pi shows a failed copy's message.
pub const COPY_ERROR_FLASH_DURATION: Duration = Duration::from_millis(5000);

/// Bytes that enter the alternate screen with mouse and focus reports. As in
/// pi, the terminal also reports pointer moves without a button held, for
/// the scrollbar's hover, except in terminal multiplexers, which can lag
/// forwarding them.
pub fn alt_screen_enter() -> String {
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    let multiplexer = ["TMUX", "ZELLIJ", "STY"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
        || term.starts_with("tmux")
        || term.starts_with("screen");
    let moves = if multiplexer { "" } else { "\x1b[?1003h" };
    format!(
        "\x1b[?1049h\x1b[?7l\x1b[?1000h\x1b[?1002h{moves}\x1b[?1004h\x1b[?1006h\x1b[2J\x1b[H\x1b[?25l"
    )
}
/// Bytes that leave the alternate screen.
pub const ALT_SCREEN_LEAVE: &str =
    "\x1b[?1006l\x1b[?1004l\x1b[?1003l\x1b[?1002l\x1b[?1000l\x1b[?7h\x1b[?1049l\x1b[?25h";

impl AltScreen {
    /// A renderer that has drawn nothing yet.
    pub fn new() -> AltScreen {
        AltScreen {
            bottom_key: "End".to_owned(),
            copy_on_select: true,
            wheel_lines: Some(1),
            wheel: WheelScroll::new(),
            ..AltScreen::default()
        }
    }

    /// Forgets what was drawn, the selection and the flashes, as pi does on
    /// entering the screen, so the next frame repaints everything.
    pub fn invalidate(&mut self) {
        self.previous.clear();
        self.previous_size = (0, 0);
        self.previous_cursor.clear();
        self.uploaded.clear();
        self.selection = selection::Selection::default();
        self.flashes.clear();
    }

    /// The bytes that leave the alternate screen, deleting the Kitty images
    /// drawn on it first, as pi does.
    pub fn leave(&mut self) -> String {
        self.uploaded.clear();
        let images = if self.kitty_images {
            image::DELETE_KITTY_IMAGES
        } else {
            ""
        };
        format!("{images}{ALT_SCREEN_LEAVE}")
    }

    /// pi's `prepareKittyScreen`: `lines` with images the terminal holds
    /// placed again without their data, and the deletions of images out of
    /// view beyond what it keeps, the least recently shown first.
    fn reuse_kitty_uploads(&mut self, lines: &[String]) -> (Vec<String>, String) {
        let mut visible = Vec::new();
        let drawn = lines
            .iter()
            .map(|line| {
                let Some(placement) = image::kitty_placement(line) else {
                    return line.clone();
                };
                visible.push(placement.id);
                let held = self.uploaded.shift_remove(&placement.id);
                self.uploaded.insert(
                    placement.id,
                    Upload {
                        generation: placement.generation,
                        bytes: placement.transmission_bytes,
                        decoded_bytes: placement.decoded_bytes,
                    },
                );
                if held.is_some_and(|held| held.generation == placement.generation) {
                    placement.line
                } else {
                    line.clone()
                }
            })
            .collect();
        let offscreen: Vec<(u32, Upload)> = self
            .uploaded
            .iter()
            .filter(|(id, _)| !visible.contains(id))
            .map(|(id, upload)| (*id, *upload))
            .collect();
        let mut count = offscreen.len();
        let mut bytes: usize = offscreen.iter().map(|(_, upload)| upload.bytes).sum();
        let mut decoded: u64 = offscreen
            .iter()
            .map(|(_, upload)| upload.decoded_bytes)
            .sum();
        let mut deletions = String::new();
        for (id, upload) in offscreen {
            if count <= MAX_OFFSCREEN_IMAGES
                && bytes <= MAX_OFFSCREEN_BYTES
                && decoded <= MAX_OFFSCREEN_DECODED_BYTES
            {
                break;
            }
            deletions.push_str(&image::delete_kitty(id));
            self.uploaded.shift_remove(&id);
            count -= 1;
            bytes -= upload.bytes;
            decoded -= upload.decoded_bytes;
        }
        (drawn, deletions)
    }

    /// pi's `getScreenLines`: the rows of the last frame.
    pub fn screen_lines(&self) -> &[StyledLine] {
        &self.screen
    }

    /// pi's `flash`: shows `message` at the top right of the screen for
    /// `duration`, below the messages already there.
    pub fn flash(&mut self, message: impl Into<String>, duration: Duration) {
        self.flashes
            .push((message.into(), Instant::now() + duration));
    }

    /// When the screen changes on its own: an `auto` scrollbar hides, a
    /// flash ends or a drag held at the transcript's edge scrolls it. A frame
    /// drawn then shows the change.
    pub fn deadline(&self) -> Option<Instant> {
        let now = Instant::now();
        self.flashes
            .iter()
            .map(|(_, until)| *until)
            .chain(self.scrollbar_until)
            .chain(self.autoscroll_deadline())
            .filter(|at| *at > now)
            .min()
    }

    fn max_scroll(&self) -> usize {
        self.transcript_len.saturating_sub(self.viewport)
    }

    /// Scrolls by `rows` (negative is up). Reaching the end resumes following.
    pub fn scroll_by(&mut self, rows: isize) {
        let max = self.max_scroll();
        let current = self.scroll_top.unwrap_or(max) as isize;
        let next = (current + rows).clamp(0, max as isize) as usize;
        if next as isize != current {
            self.scrolled();
        }
        self.scroll_top = (next < max).then_some(next);
    }

    /// pi's `markScrollbarActivity`: an `auto` scrollbar shows for a moment.
    fn scrolled(&mut self) {
        if self.scrollbar == Scrollbar::Auto && self.transcript_len > self.viewport {
            self.scrollbar_until = Some(Instant::now() + SCROLLBAR_HIDE_DELAY);
        }
    }

    /// When a shown `auto` scrollbar hides.
    fn scrollbar_deadline(&self) -> Option<Instant> {
        self.scrollbar_until.filter(|until| *until > Instant::now())
    }

    /// The transcript's width on a screen `width` wide: an `always`
    /// scrollbar keeps the last column, as pi's `ScrollView` does.
    pub fn content_width(&self, width: usize) -> usize {
        if self.scrollbar == Scrollbar::Always && width > 1 {
            width - 1
        } else {
            width
        }
    }

    /// Scrolls by a page: the viewport less four rows.
    pub fn page(&mut self, direction: isize) {
        let rows = self.viewport.saturating_sub(4).max(1) as isize;
        self.scroll_by(direction * rows);
    }

    /// Scrolls by half the viewport.
    pub fn half_page(&mut self, direction: isize) {
        let rows = (self.viewport / 2).max(1) as isize;
        self.scroll_by(direction * rows);
    }

    /// The first transcript row shown.
    pub fn scroll_position(&self) -> usize {
        self.scroll_top.unwrap_or(self.max_scroll())
    }

    /// Shows transcript row `row` at the top, as far as the transcript
    /// allows; the end resumes following.
    pub fn scroll_to(&mut self, row: usize) {
        let max = self.max_scroll();
        let next = row.min(max);
        if next != self.scroll_position() {
            self.scrolled();
        }
        self.scroll_top = (next < max).then_some(next);
    }

    /// Scrolls to the top.
    pub fn top(&mut self) {
        let top = (self.max_scroll() > 0).then_some(0);
        if top != self.scroll_top {
            self.scrolled();
        }
        self.scroll_top = top;
    }

    /// Scrolls to the end and follows new output.
    pub fn bottom(&mut self) {
        if self.scroll_top.is_some() {
            self.scrolled();
        }
        self.scroll_top = None;
    }

    /// Draws a frame and returns the bytes to write. `cursor` is in dock
    /// coordinates.
    pub fn frame(
        &mut self,
        transcript: &[StyledLine],
        dock: &[StyledLine],
        cursor: Cursor,
        width: usize,
        height: usize,
    ) -> String {
        let mut out = String::new();
        let full = self.previous.is_empty() || self.previous_size != (width, height);
        let height = height.max(1);
        let dock_rows = dock.len().min(height.saturating_sub(1));
        let dock_skip = dock.len() - dock_rows;
        let viewport = height - dock_rows;
        self.viewport = viewport;
        self.transcript_len = transcript.len();
        self.size = (width, height);
        self.step_autoscroll(transcript);
        let max = self.max_scroll();
        let top = self.scroll_top.map_or(max, |top| top.min(max));
        if top >= max {
            self.scroll_top = None;
        }
        let mut rows: Vec<StyledLine> = transcript
            .iter()
            .skip(top)
            .take(viewport)
            .cloned()
            .collect();
        rows.resize(viewport, Line::default());
        // pi's `paintBox`: a Kitty image cut by the viewport shows its
        // visible rows, including one that starts above it.
        for (row, line) in rows.iter_mut().enumerate() {
            if let Some(image_rows) = kitty_rows(line)
                && viewport - row < image_rows
            {
                *line = crop(line, 0, viewport - row);
            }
        }
        if viewport > 0 {
            for (index, line) in transcript[..top.min(transcript.len())]
                .iter()
                .enumerate()
                .rev()
            {
                if let Some(image_rows) = kitty_rows(line) {
                    let hidden = top - index;
                    if hidden < image_rows {
                        rows[0] = crop(line, hidden, viewport.min(image_rows - hidden));
                    }
                    break;
                }
                if !plain(line).is_empty() {
                    break;
                }
            }
        }
        let scrollbar = self.paint_scrollbar(&mut rows, width);
        // pi's `compositeScrollToEndIndicator`: the label centered over the
        // last transcript row, stopping at the scrollbar.
        self.jump_label = None;
        if self.scroll_top.is_some() && viewport > 0 && !is_image_line(&rows[viewport - 1]) {
            let label = format!(" ↓ Jump to latest message · {} ", self.bottom_key);
            let label = truncate(&Line::from(label), width, "");
            let left = width.saturating_sub(line_width(&label)) / 2;
            let right_edge = if scrollbar { width - 1 } else { width };
            let label = truncate(&label, right_edge.saturating_sub(left), "");
            let label_width = line_width(&label);
            if label_width > 0 {
                let label = Line::from(crate::lines::under(label.spans, self.jump_label_style));
                rows[viewport - 1] =
                    composite(&rows[viewport - 1], &label, left, label_width, width);
                self.jump_label = Some((viewport - 1, left, label_width));
            }
        }
        rows.extend(dock.iter().skip(dock_skip).cloned());
        composite_overlays(&mut rows, &self.overlays, width);
        self.highlight(&mut rows, top, width);
        self.composite_flashes(&mut rows, width);
        for row in &mut rows {
            if line_width(row) > width && !is_image_line(row) {
                *row = truncate(row, width, "");
            }
        }

        let lines: Vec<String> = rows.iter().map(line_to_ansi).collect();
        self.screen = rows;
        // A changed image row redraws every row, since an image covers rows
        // that keep their text.
        let images_changed = lines.iter().enumerate().any(|(row, line)| {
            let old = self.previous.get(row);
            old != Some(line) && (is_image(line) || old.is_some_and(|old| is_image(old)))
        });
        let redraw = full || images_changed;
        let had_uploads = !self.uploaded.is_empty();
        let (drawn, deletions) = if redraw && self.kitty_images {
            self.reuse_kitty_uploads(&lines)
        } else {
            (lines.clone(), String::new())
        };
        let mut changes = String::new();
        if full {
            if self.kitty_images {
                changes.push_str(if had_uploads {
                    image::DELETE_KITTY_PLACEMENTS
                } else {
                    image::DELETE_KITTY_IMAGES
                });
            }
            changes.push_str("\x1b[2J");
        } else if images_changed && self.kitty_images {
            changes.push_str(image::DELETE_KITTY_PLACEMENTS);
        }
        changes.push_str(&deletions);
        let clear_first =
            redraw && self.kitty_images && lines.iter().any(|line| is_image(line)) && is_wezterm();
        if clear_first {
            for row in 0..lines.len() {
                changes.push_str(&format!("\x1b[{};1H\x1b[2K", row + 1));
            }
        }
        for (index, line) in drawn.iter().enumerate() {
            if redraw || self.previous.get(index) != Some(&lines[index]) {
                let clear = if clear_first { "" } else { "\x1b[2K" };
                changes.push_str(&format!("\x1b[{};1H{clear}{line}", index + 1));
            }
        }
        // pi parks the cursor at the frame cursor even when it stays hidden,
        // so input methods place their candidate windows there.
        let cursor = match cursor
            .and_then(|(row, col)| row.checked_sub(dock_skip).map(|row| (viewport + row, col)))
        {
            Some((row, col)) if row < height => format!(
                "\x1b[{};{}H{}",
                row + 1,
                col.min(width) + 1,
                if self.show_hardware_cursor {
                    "\x1b[?25h"
                } else {
                    "\x1b[?25l"
                }
            ),
            _ => "\x1b[?25l".to_owned(),
        };
        // An unchanged frame writes nothing.
        if out.is_empty() && changes.is_empty() && cursor == self.previous_cursor {
            return out;
        }
        out.push_str(SYNC_START);
        out.push_str(&changes);
        out.push_str(&cursor);
        out.push_str(SYNC_END);
        self.previous = lines;
        self.previous_size = (width, height);
        self.previous_cursor = cursor;
        out
    }

    /// pi's `compositeFlashes`: each message reversed at the right end of a
    /// row, from the top.
    fn composite_flashes(&mut self, rows: &mut [StyledLine], width: usize) {
        let now = Instant::now();
        self.flashes.retain(|(_, until)| *until > now);
        let skip = self.flashes.len().saturating_sub(rows.len());
        let reversed = Style::new().add_modifier(Modifier::REVERSED);
        for (row, (message, _)) in rows.iter_mut().zip(self.flashes.iter().skip(skip)) {
            let flash = Line::from(Span::styled(format!(" {message} "), reversed));
            let flash = truncate(&flash, width, "");
            let flash_width = line_width(&flash);
            if flash_width > 0 {
                *row = composite(row, &flash, width - flash_width, flash_width, width);
            }
        }
    }

    /// pi's `getScrollbarGeometry` for the transcript: the thumb's first row
    /// and height while the scrollbar shows, or when `revealable`, while an
    /// `auto` one would show for the pointer over it.
    fn thumb(&self, revealable: bool) -> Option<(usize, usize)> {
        let (track, content) = (self.viewport, self.transcript_len);
        let visible = match self.scrollbar {
            Scrollbar::Always => track > 0,
            Scrollbar::Auto => {
                content > track
                    && (revealable || self.scrollbar_hover || self.scrollbar_deadline().is_some())
            }
            Scrollbar::Hidden => false,
        };
        if !visible || self.size.0 == 0 || content == 0 {
            return None;
        }
        let round = |value: f64| value.round() as usize;
        let thumb = round((track * track) as f64 / content as f64)
            .min(track)
            .max(2.min(track));
        let max_top = content.saturating_sub(track);
        let offset = if max_top == 0 {
            0
        } else {
            let top = self.scroll_position().min(max_top);
            round(top as f64 / max_top as f64 * (track - thumb) as f64)
        };
        Some((offset, thumb))
    }

    /// pi's `paintScrollbar`: the track and thumb in the transcript rows'
    /// last column while the scrollbar shows.
    fn paint_scrollbar(&mut self, rows: &mut [StyledLine], width: usize) -> bool {
        let Some((offset, thumb)) = self.thumb(false) else {
            if self.scrollbar_deadline().is_none() {
                self.scrollbar_until = None;
            }
            return false;
        };
        let keep_background = self.scrollbar != Scrollbar::Always;
        let glyph = if self.scrollbar_hover { "█" } else { "┃" };
        for (row, line) in rows.iter_mut().take(self.viewport).enumerate() {
            let (glyph, style) = if row >= offset && row < offset + thumb {
                (glyph, self.scrollbar_thumb)
            } else {
                ("│", self.scrollbar_track)
            };
            *line = replace_last_cell(line, width, glyph, style, keep_background);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(rows: &[&str]) -> Vec<StyledLine> {
        rows.iter().map(|row| Line::from(row.to_string())).collect()
    }

    #[test]
    fn main_screen_rewrites_only_changes() {
        let mut screen = MainScreen::new();
        let first = screen.frame(&doc(&["a", "b", "c"]), None, 10, 5);
        assert!(first.contains("a\x1b[0m\r\nb\x1b[0m\r\nc\x1b[0m"));
        let second = screen.frame(&doc(&["a", "B", "c"]), None, 10, 5);
        assert!(second.contains("\x1b[1A\r\x1b[2KB\x1b[0m"), "{second:?}");
        assert!(!second.contains("a\x1b[0m"));
        let appended = screen.frame(&doc(&["a", "B", "c", "d"]), None, 10, 5);
        assert!(appended.contains("\r\n\x1b[2Kd\x1b[0m"), "{appended:?}");
        let resized = screen.frame(&doc(&["a"]), None, 8, 5);
        assert!(resized.contains("\x1b[2J\x1b[H\x1b[3J"));
    }

    #[test]
    fn alt_screen_scrolls_and_follows() {
        let mut screen = AltScreen::new();
        let transcript = doc(&["1", "2", "3", "4", "5", "6"]);
        let dock = doc(&["editor"]);
        let frame = screen.frame(&transcript, &dock, None, 40, 4);
        assert!(frame.contains("\x1b[1;1H\x1b[2K4"));
        assert!(frame.contains("\x1b[4;1H\x1b[2Keditor"));
        screen.scroll_by(-2);
        let frame = screen.frame(&transcript, &dock, None, 40, 4);
        assert!(frame.contains("\x1b[1;1H\x1b[2K2"));
        assert!(frame.contains("Jump to latest"));
        screen.bottom();
        assert!(screen.scroll_top.is_none());
    }

    #[test]
    fn alt_screen_parks_hidden_cursor_at_frame_cursor() {
        let mut screen = AltScreen::new();
        let transcript = doc(&["1"]);
        let dock = doc(&["> hi", "footer"]);
        let frame = screen.frame(&transcript, &dock, Some((0, 4)), 40, 4);
        assert!(
            frame.ends_with("\x1b[3;5H\x1b[?25l\x1b[?2026l"),
            "{frame:?}"
        );
        screen.show_hardware_cursor = true;
        let frame = screen.frame(&transcript, &dock, Some((0, 99)), 40, 4);
        assert!(
            frame.ends_with("\x1b[3;41H\x1b[?25h\x1b[?2026l"),
            "{frame:?}"
        );
        let frame = screen.frame(&transcript, &dock, None, 40, 4);
        assert!(frame.ends_with("\x1b[?25l\x1b[?2026l"), "{frame:?}");
    }

    #[test]
    fn shrinks_stacks_as_pi_tui() {
        // Values computed by pi-tui's `allocateStackSizes`: every part above
        // its minimum gives at least a row per pass.
        assert_eq!(
            shrink_stack(&[0, 0, 1, 40, 0, 2], &[0, 0, 0, 3, 0, 0], 23),
            [0, 0, 0, 22, 0, 1]
        );
        assert_eq!(
            shrink_stack(&[0, 0, 1, 44, 0, 2], &[0, 0, 0, 3, 0, 0], 29),
            [0, 0, 0, 28, 0, 1]
        );
        assert_eq!(shrink_stack(&[2, 5], &[0, 3], 4), [0, 4]);
        assert_eq!(shrink_stack(&[2, 5], &[2, 5], 4), [2, 5]);
        assert_eq!(shrink_stack(&[4, 1], &[0, 0], 3), [3, 0]);
    }

    #[test]
    fn fit_stack_keeps_top_rows_or_the_cursor() {
        let parts = vec![(doc(&["a", "b", "c", "d"]), 0), (doc(&["f"]), 0)];
        let (rows, _, _) = fit_stack(parts.clone(), None, 3);
        let text: Vec<String> = rows.iter().map(|line| line.to_string()).collect();
        assert_eq!(text, ["a", "b", "c"]);
        let (rows, cursor, _) = fit_stack(parts.clone(), Some((0, 3, 1)), 3);
        let text: Vec<String> = rows.iter().map(|line| line.to_string()).collect();
        assert_eq!(text, ["b", "c", "d"]);
        assert_eq!(cursor, Some((2, 1)));
        let (rows, cursor, _) = fit_stack(parts, Some((0, 1, 1)), 9);
        assert_eq!(rows.len(), 5);
        assert_eq!(cursor, Some((1, 1)));
        // A minimum taller than the room is clipped at the bottom.
        let editor = vec![(doc(&["─", "text", "─"]), 3), (doc(&["footer"]), 0)];
        let (rows, cursor, _) = fit_stack(editor, Some((0, 1, 4)), 2);
        let text: Vec<String> = rows.iter().map(|line| line.to_string()).collect();
        assert_eq!(text, ["─", "text"]);
        assert_eq!(cursor, Some((1, 4)));
        // A part shorter than its minimum keeps its rows at the top.
        let component = vec![(doc(&["one line"]), 3), (doc(&["footer"]), 0)];
        let (rows, _, _) = fit_stack(component, None, 9);
        let text: Vec<String> = rows.iter().map(|line| line.to_string()).collect();
        assert_eq!(text, ["one line", "", "", "footer"]);
    }

    #[test]
    fn alt_screen_draws_pi_scrollbars() {
        let transcript = doc(&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]);
        let dock = doc(&["editor"]);
        // The text a frame writes to `row`, up to its next escape.
        let last_column = |frame: &str, row: usize| {
            let prefix = format!("\x1b[{row};1H\x1b[2K");
            let start = frame
                .find(&prefix)
                .unwrap_or_else(|| panic!("row {row} in {frame:?}"))
                + prefix.len();
            let rest = &frame[start..];
            rest[..rest.find("\x1b[").unwrap_or(rest.len())].to_owned()
        };
        // `always`: four rows over ten; thumb of round(16 / 10) = 2 rows at
        // the end, since the view follows the end.
        let mut screen = AltScreen::new();
        screen.scrollbar = Scrollbar::Always;
        let frame = screen.frame(&transcript, &dock, None, 10, 5);
        assert!(last_column(&frame, 1).contains('│'), "{frame:?}");
        assert!(last_column(&frame, 3).contains('┃'), "{frame:?}");
        assert!(last_column(&frame, 4).contains('┃'), "{frame:?}");
        // `auto` shows only after scrolling, and `hidden` never.
        let mut screen = AltScreen::new();
        let frame = screen.frame(&transcript, &dock, None, 10, 5);
        assert!(!frame.contains('│') && !frame.contains('┃'), "{frame:?}");
        screen.top();
        let frame = screen.frame(&transcript, &dock, None, 10, 5);
        assert!(last_column(&frame, 1).contains('┃'), "{frame:?}");
        assert!(screen.scrollbar_deadline().is_some());
        let mut screen = AltScreen::new();
        screen.scrollbar = Scrollbar::Hidden;
        screen.frame(&transcript, &dock, None, 10, 5);
        screen.top();
        let frame = screen.frame(&transcript, &dock, None, 10, 5);
        assert!(!frame.contains('┃'), "{frame:?}");
    }

    /// pi-tui's `encodeKitty` with the `Image` component's options.
    fn kitty(data: &str, rows: usize, id: u32) -> String {
        format!("\x1b_Ga=T,f=100,q=2,C=1,c=2,r={rows},i={id};{data}\x1b\\")
    }

    /// The rows of an image `size` pixels drawn in Kitty, as pi's tests make
    /// them: in 10 pixel cells, `columns` wide.
    fn kitty_image(size: (u32, u32), columns: usize, id: u32) -> Vec<StyledLine> {
        image::render(
            "AAAA",
            size,
            image::Protocol::Kitty,
            40,
            columns,
            (10, 10),
            id,
        )
        .unwrap()
    }

    fn with(rows: &[&str], image: &[StyledLine], after: &[&str]) -> Vec<StyledLine> {
        let mut out = doc(rows);
        out.extend(image.iter().cloned());
        out.extend(doc(after));
        out
    }

    // Ports of pi-tui's "TUI Kitty image cleanup" tests.
    #[test]
    fn main_screen_clears_reserved_image_rows_before_placing_the_image() {
        let mut screen = MainScreen::new();
        screen.frame(&doc(&["before"]), None, 40, 10);
        let image = kitty_image((20, 20), 2, 9001);
        let sequence = line_to_ansi(&image[0]);
        let frame = screen.frame(&with(&["before"], &image, &["after"]), None, 40, 10);
        assert!(
            frame.contains(&format!("\x1b[2K\r\n\x1b[2K\x1b[1A{sequence}\x1b[1B")),
            "{frame:?}"
        );
        assert!(!frame.contains(&format!("{sequence}\r\n\x1b[2K")));
    }

    #[test]
    fn main_screen_redraws_when_clearing_image_rows_would_scroll() {
        let mut screen = MainScreen::new();
        screen.frame(&doc(&["before"]), None, 40, 2);
        let image = kitty_image((30, 30), 3, 9002);
        let frame = screen.frame(&with(&["before"], &image, &["after"]), None, 40, 2);
        assert!(frame.contains("\x1b[2J"), "{frame:?}");
    }

    #[test]
    fn main_screen_reserves_image_rows_in_a_full_redraw() {
        let mut screen = MainScreen::new();
        let text = ["l0", "l1", "l2", "l3", "l4"];
        screen.frame(&doc(&text), None, 40, 5);
        let image = kitty_image((30, 30), 3, 9003);
        let sequence = line_to_ansi(&image[0]);
        let frame = screen.frame(&with(&text, &image, &["after"]), None, 40, 5);
        assert!(frame.contains("\x1b[2J"), "{frame:?}");
        assert!(frame.contains(&format!("\r\n\r\n\x1b[2A{sequence}\x1b[2B")));
        assert!(!frame.contains(&format!("{sequence}\r\n\x1b[0m")));
    }

    #[test]
    fn main_screen_places_an_image_taller_than_the_screen_from_its_first_row() {
        let image = kitty_image((60, 60), 6, 9004);
        let sequence = line_to_ansi(&image[0]);
        let frame = MainScreen::new().frame(&with(&["before"], &image, &["after"]), None, 40, 5);
        assert!(frame.contains(&sequence));
        assert!(!frame.contains(&format!("\x1b[{}A{sequence}", image.len() - 1)));
    }

    #[test]
    fn main_screen_deletes_changed_images_before_drawing_them() {
        let mut screen = MainScreen::new();
        let old = kitty("AAAA", 2, 42);
        screen.frame(&doc(&["top", &old]), None, 40, 10);
        let new = kitty("BBBB", 1, 42);
        let frame = screen.frame(&doc(&[&new, ""]), None, 40, 10);
        let deleted = frame.find(&image::delete_kitty(42)).unwrap();
        assert!(deleted < frame.find(&new).unwrap(), "{frame:?}");

        // A change on a row the image covers redraws it, after deleting it.
        let mut screen = MainScreen::new();
        let image = kitty("AAAA", 2, 88);
        screen.frame(&doc(&["", &image]), None, 40, 10);
        let frame = screen.frame(&doc(&["covered", &image]), None, 40, 10);
        let deleted = frame.find(&image::delete_kitty(88)).unwrap();
        assert!(deleted < frame.find(&image).unwrap(), "{frame:?}");
        assert!(!frame.contains("\x1b[2J"));

        // A full redraw deletes the images drawn before.
        let mut screen = MainScreen::new();
        screen.frame(&doc(&[&kitty("AAAA", 2, 77)]), None, 40, 10);
        let frame = screen.frame(&doc(&["plain text"]), None, 30, 10);
        let deleted = frame.find(&image::delete_kitty(77)).unwrap();
        assert!(deleted < frame.find("\x1b[2J").unwrap(), "{frame:?}");
    }

    // Ports of pi-tui's fullscreen Kitty tests.
    #[test]
    fn alt_screen_crops_an_image_that_starts_above_the_viewport() {
        let image = kitty_image((100, 100), 3, 123);
        assert_eq!(image.len(), 3);
        let transcript = with(&["before"], &image, &["after", "end"]);
        let frame = AltScreen::new().frame(&transcript, &[], None, 20, 3);
        assert!(
            frame.contains("i=123") && frame.contains("y=66,h=34,r=1"),
            "{frame:?}"
        );
    }

    #[test]
    fn alt_screen_places_held_images_again_and_evicts_the_oldest() {
        let mut screen = AltScreen::new();
        screen.kitty_images = true;
        let transcript = with(&[], &kitty_image((100, 50), 2, 321), &["after"]);
        screen.frame(&transcript, &[], None, 20, 1);
        screen.top();
        let shown = screen.frame(&transcript, &[], None, 20, 1);
        assert!(shown.contains("\x1b_Ga=T"), "{shown:?}");
        screen.scroll_by(1);
        screen.frame(&transcript, &[], None, 20, 1);
        screen.scroll_by(-1);
        let again = screen.frame(&transcript, &[], None, 20, 1);
        assert!(again.contains("\x1b_Ga=p,q=2"), "{again:?}");
        assert!(!again.contains("\x1b_Ga=T") && !again.contains(&image::delete_kitty(321)));

        let mut screen = AltScreen::new();
        screen.kitty_images = true;
        let transcript: Vec<StyledLine> = (500..518)
            .flat_map(|id| kitty_image((100, 50), 2, id))
            .collect();
        let mut frames = screen.frame(&transcript, &[], None, 20, 1);
        screen.top();
        for _ in 0..transcript.len() {
            frames.push_str(&screen.frame(&transcript, &[], None, 20, 1));
            screen.scroll_by(1);
        }
        assert!(frames.contains(&image::delete_kitty(500)), "{frames:?}");
        screen.top();
        assert!(
            screen
                .frame(&transcript, &[], None, 20, 1)
                .contains("\x1b_Ga=T")
        );
        assert!(screen.leave().starts_with(image::DELETE_KITTY_IMAGES));
    }

    #[test]
    fn alt_screen_writes_nothing_for_an_unchanged_frame() {
        let mut screen = AltScreen::new();
        let transcript = doc(&["1"]);
        let dock = doc(&["> hi", "footer"]);
        assert!(
            !screen
                .frame(&transcript, &dock, Some((0, 4)), 40, 4)
                .is_empty()
        );
        assert_eq!(screen.frame(&transcript, &dock, Some((0, 4)), 40, 4), "");
        let moved = screen.frame(&transcript, &dock, Some((0, 3)), 40, 4);
        assert!(moved.contains("\x1b[3;4H"), "{moved:?}");
        screen.invalidate();
        assert!(
            !screen
                .frame(&transcript, &dock, Some((0, 3)), 40, 4)
                .is_empty()
        );
    }
}
