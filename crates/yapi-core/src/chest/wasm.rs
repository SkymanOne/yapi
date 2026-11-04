//! The capabilities a native extension uses, read from its WebAssembly
//! component: the `yapi:capabilities` section that `yapi-extension-api`
//! writes, and the WASI interfaces it imports.

use std::collections::HashSet;

use wasmparser::{Encoding, Parser, Payload};

use super::{ChestError, invalid};
use crate::extensions::CAPABILITIES;

/// The custom section that lists the SDK features of a build.
const SECTION: &str = "yapi:capabilities";
/// The section's line that says the SDK wrote it.
const MARKER: &str = "yapi-extension-api";

/// A capability a component uses, and what shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Use {
    /// One of [`CAPABILITIES`].
    pub capability: &'static str,
    /// What shows it, such as ``SDK feature `session` `` or
    /// ``WASI import `wasi:filesystem/types@0.2.6` ``.
    pub source: String,
}

/// What a component uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// Whether it carries a `yapi:capabilities` section. Without one it was
    /// built without capability metadata, so only its WASI imports show
    /// what it uses.
    pub metadata: bool,
    /// Each capability it uses, once, with the first thing that shows it.
    /// A build with the SDK's `all` feature lists `all` alone from the
    /// section.
    pub used: Vec<Use>,
    /// The WASI network interfaces it imports, which yapi does not provide:
    /// it links no `wasi:http`, and refuses every `wasi:sockets` call. An
    /// extension reaches the network through the SDK's `fetch`.
    pub unsupported: Vec<String>,
    /// The lines of its section that name no capability yapi knows, each
    /// once, such as one a newer SDK writes.
    pub unknown: Vec<String>,
}

impl Capabilities {
    fn add(&mut self, capability: &'static str, source: String) {
        if !self.used.iter().any(|used| used.capability == capability) {
            self.used.push(Use { capability, source });
        }
    }
}

/// The capability a section line names.
fn feature(line: &str) -> Option<&'static str> {
    CAPABILITIES.into_iter().find(|name| *name == line)
}

/// The capability a WASI import needs. `wasi:cli/environment` counts for
/// none: every Rust component imports it, and yapi gives WASI no
/// environment variables, so an extension reads them through the SDK.
fn import(name: &str) -> Option<&'static str> {
    name.starts_with("wasi:filesystem/").then_some("fs")
}

/// Whether yapi leaves WASI import `name` out, as [`Capabilities::unsupported`]
/// describes.
fn unsupported(name: &str) -> bool {
    name.starts_with("wasi:sockets/") || name.starts_with("wasi:http/")
}

/// What the component `bytes` uses: the features its `yapi:capabilities`
/// section lists, in any of its modules, then the WASI interfaces the
/// component itself imports. `Err` for bytes that are not a WebAssembly
/// component.
pub fn capabilities(bytes: &[u8]) -> Result<Capabilities, ChestError> {
    let mut found = Capabilities::default();
    let mut features = Vec::new();
    let mut imports = Vec::new();
    let mut unknown = HashSet::new();
    let mut depth = 0usize;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload =
            payload.map_err(|err| invalid(format!("not a WebAssembly component: {err}")))?;
        match payload {
            Payload::Version { encoding, .. } => {
                if depth == 0 && encoding != Encoding::Component {
                    return Err(invalid(
                        "a WebAssembly module, not a component: build it for wasm32-wasip2",
                    ));
                }
                depth += 1;
            }
            Payload::End(_) => depth = depth.saturating_sub(1),
            Payload::CustomSection(section) if section.name() == SECTION => {
                found.metadata = true;
                let text = String::from_utf8_lossy(section.data());
                for line in text
                    .lines()
                    .filter(|line| !line.is_empty() && *line != MARKER)
                {
                    match feature(line) {
                        Some(capability) => features.push(capability),
                        None if unknown.insert(line.to_owned()) => {
                            found.unknown.push(line.to_owned());
                        }
                        None => {}
                    }
                }
            }
            // A nested component's imports are met inside the component.
            Payload::ComponentImportSection(reader) if depth == 1 => {
                for entry in reader {
                    let entry = entry
                        .map_err(|err| invalid(format!("not a WebAssembly component: {err}")))?;
                    imports.push(entry.name.name.to_owned());
                }
            }
            _ => {}
        }
    }
    // The SDK's `all` feature turns on the others.
    if features.contains(&"all") {
        features = vec!["all"];
    }
    for capability in features {
        found.add(capability, format!("SDK feature `{capability}`"));
    }
    for name in imports {
        if let Some(capability) = import(&name) {
            found.add(capability, format!("WASI import `{name}`"));
        } else if unsupported(&name) {
            found.unsupported.push(name);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../yapi-ext/tests/fixtures");
        std::fs::read(format!("{dir}/{name}.wasm")).unwrap()
    }

    /// A component section: id, LEB128 size, contents.
    fn section(id: u8, contents: &[u8]) -> Vec<u8> {
        let (mut bytes, mut size) = (vec![id], contents.len());
        while size >= 0x80 {
            bytes.push((size as u8) | 0x80);
            size >>= 7;
        }
        bytes.push(size as u8);
        [&bytes[..], contents].concat()
    }

    fn name(text: &str) -> Vec<u8> {
        [&[text.len() as u8], text.as_bytes()].concat()
    }

    /// An empty component that imports each of `interfaces` as an instance
    /// with no exports.
    fn importing(interfaces: &[&str]) -> Vec<u8> {
        let mut imports = vec![interfaces.len() as u8];
        for interface in interfaces {
            imports.extend([&[0x00][..], &name(interface), &[0x05, 0x00]].concat());
        }
        let types = section(0x07, &[0x01, 0x42, 0x00]);
        [&EMPTY[..], &types, &section(0x0a, &imports)].concat()
    }

    const EMPTY: [u8; 8] = *b"\0asm\x0d\x00\x01\x00";

    /// yapi-extension-api cannot depend on yapi-core, so its capability
    /// features are checked against [`CAPABILITIES`] here: one each, and
    /// `all` turns on the others.
    #[test]
    fn sdk_features_match() {
        let manifest = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../guest/yapi-extension-api/Cargo.toml"
        );
        let manifest: toml_edit::DocumentMut =
            std::fs::read_to_string(manifest).unwrap().parse().unwrap();
        let features = manifest["features"].as_table().unwrap();
        let mut names: Vec<&str> = features
            .iter()
            .map(|(name, _)| name)
            .filter(|name| !["default", "widgets"].contains(name))
            .collect();
        let mut expected = CAPABILITIES.to_vec();
        names.sort_unstable();
        expected.sort_unstable();
        assert_eq!(names, expected);
        let all: Vec<&str> = features["all"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|value| value.as_str())
            .collect();
        assert_eq!(all, CAPABILITIES[..CAPABILITIES.len() - 1]);
    }

    fn used(found: &Capabilities) -> Vec<(&str, &str)> {
        let pairs = found.used.iter();
        pairs
            .map(|used| (used.capability, used.source.as_str()))
            .collect()
    }

    #[test]
    fn capabilities_from_component() {
        let hello = capabilities(&fixture("hello")).unwrap();
        assert!(hello.metadata && hello.unknown.is_empty());
        assert_eq!(used(&hello), [("session", "SDK feature `session`")]);
        let probe = capabilities(&fixture("capability-probe")).unwrap();
        assert_eq!(used(&probe), [("all", "SDK feature `all`")]);
        let todo = capabilities(&fixture("todo")).unwrap();
        assert!(todo.metadata);
        assert_eq!(used(&todo), []);

        let wasi = importing(&[
            "wasi:cli/environment@0.2.6",
            "wasi:filesystem/types@0.2.6",
            "wasi:http/outgoing-handler@0.2.6",
        ]);
        let wasi = capabilities(&wasi).unwrap();
        assert!(!wasi.metadata);
        assert_eq!(
            used(&wasi),
            [("fs", "WASI import `wasi:filesystem/types@0.2.6`")]
        );
        assert_eq!(wasi.unsupported, ["wasi:http/outgoing-handler@0.2.6"]);
        let lines = b"yapi-extension-api\nnet\nexec:git\nfs\nexec:git\n";
        let custom = section(0x00, &[&name(SECTION)[..], lines].concat());
        let declared =
            capabilities(&[&importing(&["wasi:sockets/tcp@0.2.6"])[..], &custom].concat());
        let declared = declared.unwrap();
        assert!(declared.metadata);
        assert_eq!(
            used(&declared),
            [("net", "SDK feature `net`"), ("fs", "SDK feature `fs`")]
        );
        assert_eq!(declared.unsupported, ["wasi:sockets/tcp@0.2.6"]);
        assert_eq!(declared.unknown, ["exec:git"]);
        // 100,000 unknown lines, each given twice, are kept once each, in
        // order, in linear time.
        let lines: String = (0..100_000).map(|n| format!("x{n}\nx{n}\n")).collect();
        let custom = section(0x00, &[&name(SECTION)[..], lines.as_bytes()].concat());
        let many = capabilities(&[&EMPTY[..], &custom].concat()).unwrap();
        assert_eq!(many.unknown.len(), 100_000);
        assert_eq!(many.unknown[..2], ["x0", "x1"]);

        assert_eq!(capabilities(&EMPTY).unwrap(), Capabilities::default());
        let module = capabilities(b"\0asm\x01\x00\x00\x00").unwrap_err();
        assert!(module.to_string().contains("not a component"), "{module}");
        assert!(capabilities(b"plain text").is_err());
    }
}
