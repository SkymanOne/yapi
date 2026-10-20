//! Inline images in the terminal.
//!
//! Ports of `terminal-image.ts` and `components/image.ts` in pi-tui `v1.0.0`.
//! An image's first or last row holds the escape sequence that draws it as
//! raw text, which [`is_image_line`] recognizes; code that measures or edits
//! rows leaves those rows as they are, as pi-tui does.

use std::sync::{LazyLock, Mutex, PoisonError};

use indexmap::IndexMap;
use ratatui_core::text::Line;

use crate::lines::{StyledLine, raw};

/// pi-tui's `ImageProtocol`: the graphics protocol a terminal speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    /// The Kitty graphics protocol, which takes PNG data.
    Kitty,
    /// iTerm2's inline images.
    Iterm2,
}

/// A cell's width and height in pixels until the terminal reports its own.
pub const DEFAULT_CELL: (u32, u32) = (9, 18);

/// pi-tui's `deleteAllKittyImages`: deletes every Kitty image and its data.
pub const DELETE_KITTY_IMAGES: &str = "\x1b_Ga=d,d=A,q=2\x1b\\";
/// pi-tui's `deleteAllKittyPlacements`: deletes every Kitty placement and
/// keeps the uploaded data.
pub const DELETE_KITTY_PLACEMENTS: &str = "\x1b_Ga=d,d=a,q=2\x1b\\";

const KITTY_PREFIX: &str = "\x1b_G";
const ITERM2_PREFIX: &str = "\x1b]1337;File=";
const KITTY_CHUNK: usize = 4096;
/// The Kitty controls a placement-only command keeps.
const PLACEMENT_KEYS: [&str; 17] = [
    "i", "p", "x", "y", "w", "h", "X", "Y", "c", "r", "C", "U", "z", "P", "Q", "H", "V",
];

/// pi-tui's `isImageLine` for text with escape sequences.
pub fn is_image(text: &str) -> bool {
    text.contains(KITTY_PREFIX) || text.contains(ITERM2_PREFIX)
}

/// Whether `line` draws an image.
pub fn is_image_line(line: &Line<'_>) -> bool {
    line.spans.iter().any(|span| is_image(&span.content))
}

/// pi-tui's `allocateImageId`: a random Kitty image id, so that ids rarely
/// collide with images other programs drew in the terminal.
pub fn allocate_id() -> u32 {
    use std::hash::{BuildHasher, RandomState};
    (RandomState::new().hash_one(()) as u32).max(1)
}

/// pi-tui's `deleteKittyImage`: deletes Kitty image `id` and its data.
pub fn delete_kitty(id: u32) -> String {
    format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\")
}

/// What a Kitty image was drawn with, kept by id as pi-tui's
/// `registerKittyImageMetadata` does.
#[derive(Clone, Copy, Debug)]
struct Metadata {
    rows: usize,
    width_px: u32,
    height_px: u32,
    generation: u64,
}

#[derive(Default)]
struct Registry {
    images: IndexMap<u32, Metadata>,
    generation: u64,
}

static KITTY_IMAGES: LazyLock<Mutex<Registry>> = LazyLock::new(Mutex::default);

fn register(id: u32, rows: usize, (width_px, height_px): (u32, u32)) {
    let mut registry = KITTY_IMAGES.lock().unwrap_or_else(PoisonError::into_inner);
    registry.generation += 1;
    let generation = registry.generation;
    registry.images.shift_remove(&id);
    registry.images.insert(
        id,
        Metadata {
            rows,
            width_px,
            height_px,
            generation,
        },
    );
    if registry.images.len() > 1000 {
        registry.images.shift_remove_index(0);
    }
}

/// Where the first Kitty command in `text` starts, and its controls.
fn kitty_controls(text: &str) -> Option<(usize, &str)> {
    let start = text.find(KITTY_PREFIX)?;
    let from = start + KITTY_PREFIX.len();
    let end = from + text[from..].find(';')?;
    Some((start, &text[from..end]))
}

fn control<'a>(controls: &'a str, key: &str) -> Option<&'a str> {
    controls
        .split(',')
        .find_map(|control| control.strip_prefix(key)?.strip_prefix('='))
}

fn metadata(text: &str) -> Option<Metadata> {
    let (_, controls) = kitty_controls(text)?;
    let id: u32 = control(controls, "i")?.parse().ok()?;
    let registry = KITTY_IMAGES.lock().unwrap_or_else(PoisonError::into_inner);
    registry.images.get(&id).copied()
}

/// The rows of the Kitty image `text` draws, as it was rendered.
pub fn kitty_rows(text: &str) -> Option<usize> {
    metadata(text).map(|metadata| metadata.rows)
}

/// The ids and `r` rows in the first Kitty command of `text`, as pi-tui's
/// main screen reads them: rows default to one.
pub fn kitty_header(text: &str) -> Option<(Vec<u32>, usize)> {
    let (_, controls) = kitty_controls(text)?;
    let mut ids = Vec::new();
    let mut rows = 1;
    for control in controls.split(',') {
        let Some((key, value)) = control.split_once('=') else {
            continue;
        };
        let Some(value) = value.parse::<u32>().ok().filter(|value| *value > 0) else {
            continue;
        };
        match key {
            "i" => ids.push(value),
            "r" => rows = value as usize,
            _ => {}
        }
    }
    Some((ids, rows))
}

/// pi-tui's `cropKittyImageLine`: the Kitty image `text` draws, cut to
/// `visible` rows after skipping `hidden`.
pub fn crop_kitty(text: &str, hidden: usize, visible: usize) -> String {
    let (Some(metadata), Some((start, controls))) = (metadata(text), kitty_controls(text)) else {
        return text.to_owned();
    };
    if hidden >= metadata.rows || visible == 0 {
        return text.to_owned();
    }
    let cropped = visible.min(metadata.rows - hidden);
    if hidden == 0 && cropped == metadata.rows {
        return text.to_owned();
    }
    let (height, rows) = (u64::from(metadata.height_px), metadata.rows as u64);
    let source_y = height * hidden as u64 / rows;
    let source_end = (height * (hidden + cropped) as u64).div_ceil(rows);
    let source_height = (source_end.min(height).saturating_sub(source_y)).max(1);
    let mut kept: Vec<String> = controls
        .split(',')
        .filter(|control| {
            !["y=", "h=", "r="]
                .iter()
                .any(|key| control.starts_with(key))
        })
        .map(str::to_owned)
        .collect();
    kept.extend([
        format!("y={source_y}"),
        format!("h={source_height}"),
        format!("r={cropped}"),
    ]);
    let rest = &text[start + KITTY_PREFIX.len() + controls.len() + 1..];
    format!("{}{KITTY_PREFIX}{};{rest}", &text[..start], kept.join(","))
}

/// A Kitty image drawn again without its data: pi-tui's
/// `KittyImagePlacement`.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// The image's id.
    pub id: u32,
    /// Which rendering of the image `id` this is; a newer one uploads again.
    pub generation: u64,
    /// The length of the commands that upload the image.
    pub transmission_bytes: usize,
    /// About how much memory the terminal holds for the image.
    pub decoded_bytes: u64,
    /// `text` with the upload replaced by a placement-only command.
    pub line: String,
}

/// pi-tui's `getKittyImagePlacement`: `text` placing the image it uploads
/// without uploading it again.
pub fn kitty_placement(text: &str) -> Option<Placement> {
    let metadata = metadata(text)?;
    let (start, controls) = kitty_controls(text)?;
    let (mut command, mut command_controls) = (start, controls);
    let end = loop {
        let terminator =
            command + KITTY_PREFIX.len() + text[command + KITTY_PREFIX.len()..].find("\x1b\\")?;
        let end = terminator + 2;
        if control(command_controls, "m") != Some("1") {
            break end;
        }
        command = end;
        if !text[command..].starts_with(KITTY_PREFIX) {
            return None;
        }
        let from = command + KITTY_PREFIX.len();
        command_controls = &text[from..from + text[from..].find(';')?];
    };
    let kept: Vec<&str> = controls
        .split(',')
        .filter(|control| PLACEMENT_KEYS.contains(&control.split('=').next().unwrap_or("")))
        .collect();
    let sequence = format!("{KITTY_PREFIX}a=p,q=2,{}\x1b\\", kept.join(","));
    Some(Placement {
        id: control(controls, "i")?.parse().ok()?,
        generation: metadata.generation,
        transmission_bytes: end - start,
        decoded_bytes: u64::from(metadata.width_px) * u64::from(metadata.height_px) * 4,
        line: format!("{}{sequence}{}", &text[..start], &text[end..]),
    })
}

fn less_distorted(upper: usize, ideal: f64) -> usize {
    if upper <= 1 {
        return upper;
    }
    let lower = upper - 1;
    let distortion = |count: usize| (count as f64 / ideal).max(ideal / count as f64);
    if distortion(lower) < distortion(upper) {
        lower
    } else {
        upper
    }
}

/// pi-tui's `calculateImageCellSize`: the columns and rows an image of
/// `image` pixels fills, at most `max_width` by `max_height` cells of
/// `cell` pixels. `optimize` picks the cell count that distorts the image
/// least, as Kitty stretches images to whole cells.
pub fn cell_size(
    image: (u32, u32),
    max_width: usize,
    max_height: Option<usize>,
    cell: (u32, u32),
    optimize: bool,
) -> (usize, usize) {
    let max_width = max_width.max(1);
    let max_height = max_height.map(|rows| rows.max(1));
    let (cell_width, cell_height) = (f64::from(cell.0), f64::from(cell.1));
    let (image_width, image_height) = (f64::from(image.0.max(1)), f64::from(image.1.max(1)));
    let width_scale = max_width as f64 * cell_width / image_width;
    let height_scale =
        max_height.map_or(width_scale, |rows| rows as f64 * cell_height / image_height);
    let scale = width_scale.min(height_scale);
    let (scaled_width, scaled_height) = (image_width * scale, image_height * scale);
    let mut columns = ((scaled_width / cell_width).ceil() as usize).clamp(1, max_width);
    let mut rows = ((scaled_height / cell_height).ceil() as usize).max(1);
    if let Some(max_height) = max_height {
        rows = rows.min(max_height);
    }
    if optimize {
        if width_scale <= height_scale {
            let ideal = columns as f64 * cell_width * image_height / (image_width * cell_height);
            rows = less_distorted(rows, ideal);
        } else {
            let ideal = rows as f64 * cell_height * image_width / (image_height * cell_width);
            columns = less_distorted(columns, ideal);
        }
    }
    (columns, rows)
}

/// pi-tui's `encodeKitty` with the `Image` component's options: uploads
/// and places image `id` over `columns` by `rows` cells, leaving the cursor
/// where it is, in commands of at most 4096 bytes of data.
fn encode_kitty(data: &str, columns: usize, rows: usize, id: u32) -> String {
    let params = format!("a=T,f=100,q=2,C=1,c={columns},r={rows},i={id}");
    if data.len() <= KITTY_CHUNK {
        return format!("{KITTY_PREFIX}{params};{data}\x1b\\");
    }
    // `data` is base64, so every byte is a character boundary.
    let chunks: Vec<&str> = (0..data.len())
        .step_by(KITTY_CHUNK)
        .map(|at| &data[at..(at + KITTY_CHUNK).min(data.len())])
        .collect();
    let mut out = String::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let controls = match index {
            0 => format!("{params},m=1"),
            _ if index + 1 == chunks.len() => "m=0".to_owned(),
            _ => "m=1".to_owned(),
        };
        out.push_str(&format!("{KITTY_PREFIX}{controls};{chunk}\x1b\\"));
    }
    out
}

/// The length of base64 `data` decoded, as Node's `Buffer.byteLength`.
fn decoded_len(data: &str) -> usize {
    let bytes = data.as_bytes();
    let mut len = bytes.len();
    for _ in 0..2 {
        if len > 0 && bytes[len - 1] == b'=' {
            len -= 1;
        }
    }
    len * 3 / 4
}

/// pi-tui's `Image` component at `width`, with its `maxWidthCells` and
/// `imageId` options: the rows that draw base64 `data`, an image of `size`
/// pixels, in cells of `cell` pixels. Kitty takes only PNG data.
///
/// `None` when `data` holds anything but base64, since it reaches the
/// terminal raw and could end the sequence to send one of its own.
pub fn render(
    data: &str,
    size: (u32, u32),
    protocol: Protocol,
    width: usize,
    max_width_cells: usize,
    cell: (u32, u32),
    id: u32,
) -> Option<Vec<StyledLine>> {
    let base64 = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=');
    if data.is_empty() || !data.bytes().all(base64) {
        return None;
    }
    let max_width = width.saturating_sub(2).min(max_width_cells).max(1);
    let max_height =
        ((max_width as f64 * f64::from(cell.0) / f64::from(cell.1)).ceil() as usize).max(1);
    let (columns, rows) = cell_size(
        size,
        max_width,
        Some(max_height),
        cell,
        protocol == Protocol::Kitty,
    );
    let blank = || std::iter::repeat_n(StyledLine::default(), rows - 1);
    Some(match protocol {
        Protocol::Kitty => {
            register(id, rows, size);
            std::iter::once(raw(encode_kitty(data, columns, rows, id)))
                .chain(blank())
                .collect()
        }
        Protocol::Iterm2 => {
            // The last row moves the cursor up to the first and draws there,
            // so the rows above are reserved before the image covers them.
            let up = if rows > 1 {
                format!("\x1b[{}A", rows - 1)
            } else {
                String::new()
            };
            let sequence = format!(
                "{up}{ITERM2_PREFIX}inline=1;size={};width={columns};height=auto:{data}\x07",
                decoded_len(data)
            );
            blank().chain(std::iter::once(raw(sequence))).collect()
        }
    })
}
