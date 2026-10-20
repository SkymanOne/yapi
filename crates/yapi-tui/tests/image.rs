//! Inline images against pi-tui's, recorded by `tests/fixtures/pi/generator/image.mjs`.

#![allow(clippy::unwrap_used, reason = "test fixture access")]

use serde_json::Value;
use yapi_tui::image::{Protocol, cell_size, crop_kitty, kitty_placement, render};
use yapi_tui::lines::plain;

fn fixture() -> Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/pi/image/image.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn pair(value: &Value) -> (u32, u32) {
    (
        value[0].as_u64().unwrap() as u32,
        value[1].as_u64().unwrap() as u32,
    )
}

fn usize_of(value: &Value) -> usize {
    value.as_u64().unwrap() as usize
}

#[test]
fn sizes_images_like_pi() {
    for case in fixture()["cellSizes"].as_array().unwrap() {
        let size = cell_size(
            pair(&case["image"]),
            usize_of(&case["maxWidth"]),
            case["maxHeight"].as_u64().map(|rows| rows as usize),
            pair(&case["cell"]),
            case["optimize"].as_bool().unwrap(),
        );
        let expected = (
            usize_of(&case["size"]["columns"]),
            usize_of(&case["size"]["rows"]),
        );
        assert_eq!(size, expected, "{case}");
    }
}

/// Renders the recorded images, which also registers the Kitty ones'
/// metadata for the crops and placements.
fn rendered(fixture: &Value, key: &str, protocol: Protocol) {
    for case in fixture[key].as_array().unwrap() {
        let lines = render(
            case["data"].as_str().unwrap(),
            pair(&case["size"]),
            protocol,
            usize_of(&case["width"]),
            usize_of(&case["maxWidthCells"]),
            pair(&case["cell"]),
            case["imageId"].as_u64().unwrap() as u32,
        )
        .unwrap();
        let actual: Vec<String> = lines.iter().map(plain).collect();
        let expected: Vec<&str> = case["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line.as_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "{key} {}", case["width"]);
    }
}

#[test]
fn renders_kitty_and_iterm2_rows_crops_and_placements_like_pi() {
    let fixture = fixture();
    rendered(&fixture, "iterm2", Protocol::Iterm2);
    rendered(&fixture, "kitty", Protocol::Kitty);
    for case in fixture["crops"].as_array().unwrap() {
        let cropped = crop_kitty(
            case["line"].as_str().unwrap(),
            usize_of(&case["hidden"]),
            usize_of(&case["visible"]),
        );
        assert_eq!(
            cropped,
            case["cropped"].as_str().unwrap(),
            "hidden {} visible {}",
            case["hidden"],
            case["visible"]
        );
    }
    for case in fixture["placements"].as_array().unwrap() {
        let placement = kitty_placement(case["line"].as_str().unwrap()).unwrap();
        assert_eq!(placement.line, case["replacement"].as_str().unwrap());
        assert!(placement.line.contains(case["sequence"].as_str().unwrap()));
    }
}

#[test]
fn refuses_data_that_is_not_base64() {
    // A tool result's data could otherwise end the sequence and send its own.
    let injected = "AAAA\x1b\\\x1b]52;c;aGk=\x07";
    for protocol in [Protocol::Kitty, Protocol::Iterm2] {
        assert!(render(injected, (1, 1), protocol, 80, 60, (9, 18), 1).is_none());
        assert!(render("", (1, 1), protocol, 80, 60, (9, 18), 1).is_none());
    }
}
