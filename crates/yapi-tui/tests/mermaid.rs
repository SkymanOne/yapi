//! yapi's port of grok-mermaid, and pi's Mermaid Markdown transformer,
//! against their output recorded by `tests/fixtures/pi/generator/mermaid.mjs`.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use serde_json::Value;
use yapi_tui::lines::plain;
use yapi_tui::markdown::{MarkdownOptions, MarkdownTheme, render as markdown};
use yapi_tui::mermaid::{Cls, MermaidArt, Span, render, transform};

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/../../tests/fixtures/pi/mermaid/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line.as_str().unwrap().to_owned())
        .collect()
}

fn art(value: &Value) -> Option<MermaidArt> {
    let cls = |name: &str| match name {
        "border" => Cls::Border,
        "text" => Cls::Text,
        "edge" => Cls::Edge,
        "edgeLabel" => Cls::EdgeLabel,
        "title" => Cls::Title,
        _ => Cls::None,
    };
    (!value.is_null()).then(|| MermaidArt {
        plain: strings(&value["plain"]),
        styled: value["styled"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|span| Span {
                        text: span["text"].as_str().unwrap().to_owned(),
                        cls: cls(span["cls"].as_str().unwrap()),
                    })
                    .collect()
            })
            .collect(),
        width: value["width"].as_u64().unwrap() as usize,
        warnings: strings(&value["warnings"]),
    })
}

#[test]
fn diagrams_like_grok_mermaid() {
    let cases = fixture("render.json");
    let failures: Vec<&str> = cases
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| render(case["source"].as_str().unwrap()) != art(&case["art"]))
        .map(|case| case["source"].as_str().unwrap())
        .collect();
    assert!(
        failures.is_empty(),
        "{} failures: {failures:#?}",
        failures.len()
    );
}

#[test]
fn transforms_markdown_like_pi() {
    let mut failures = Vec::new();
    for case in fixture("transform.json").as_array().unwrap() {
        let (name, text) = (
            case["name"].as_str().unwrap(),
            case["text"].as_str().unwrap(),
        );
        let width = case["width"].as_u64().unwrap() as usize;
        let streaming = case["isStreaming"].as_bool().unwrap();
        let transformed = transform(text, width - 2, streaming, None);
        let lines: Vec<String> = markdown(
            &transformed,
            width,
            1,
            0,
            &MarkdownTheme::default(),
            MarkdownOptions::default(),
        )
        .iter()
        .map(plain)
        .collect();
        if transformed != case["transformed"].as_str().unwrap() || lines != strings(&case["lines"])
        {
            failures.push(format!(
                "{name} width {width} streaming {streaming}:\n  pi {:?}\n  yapi {transformed:?}\n  pi {:#?}\n  yapi {lines:#?}",
                case["transformed"], case["lines"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures\n{}",
        failures.len(),
        failures.join("\n")
    );
}
