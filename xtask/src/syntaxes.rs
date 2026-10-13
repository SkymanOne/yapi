//! `cargo xtask syntaxes`: generate the syntax definitions yapi highlights
//! code with, `crates/yapi-core/assets/syntaxes/syntaxes.packdump`, and their
//! notices beside it in `LICENSES.md`.
//!
//! The definitions are a subset of those two-face packs from bat: the
//! languages below and those they embed, such as JavaScript in HTML.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, bail};
use serde_json::{Value, json};
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};

const OUT_DIR: &str = "crates/yapi-core/assets/syntaxes";
/// The two-face release in `Cargo.lock`, which pins it.
const TWO_FACE: &str = "two-face 0.5.2 (bat 0.26.1)";

/// The syntaxes kept, by name: the languages pi's `getLanguageFromPath`
/// names, those pi's highlighter loads at startup, and common code block
/// languages.
const KEEP: &[&str] = &[
    "Plain Text",
    "Bourne Again Shell (bash)",
    "C",
    "C#",
    "C++",
    "CMake",
    "CSS",
    "Clojure",
    "Dart",
    "Diff",
    "Dockerfile",
    "Elixir",
    "Erlang",
    "Fish",
    "Go",
    "GraphQL",
    "Groovy",
    "HTML",
    "Haskell",
    "INI",
    "JSON",
    "Java",
    "JavaScript",
    "Kotlin",
    "Less",
    "Lua",
    "Makefile",
    "Markdown",
    "Nix",
    "OCaml",
    "PHP",
    "Perl",
    "Protocol Buffer",
    "Python",
    "R",
    "Ruby",
    "Rust",
    "SCSS",
    "SQL",
    "Sass",
    "Scala",
    "Swift",
    "TOML",
    "Terraform",
    "TypeScript",
    "TypeScriptReact",
    "VimL",
    "XML",
    "YAML",
];

/// The packages of kept syntaxes whose licenses ask for a notice, as
/// two-face's listing names them. Syntaxes from Sublime Text's default
/// packages need none, except Rust's. Update this with `KEEP`.
const NOTICES: &[&str] = &[
    "syntaxes/01_Packages/Rust",
    "syntaxes/02_Extra/CMake",
    "syntaxes/02_Extra/Dart",
    "syntaxes/02_Extra/Docker",
    "syntaxes/02_Extra/Elixir",
    "syntaxes/02_Extra/Fish",
    "syntaxes/02_Extra/JavaScript_(Babel)",
    "syntaxes/02_Extra/Kotlin",
    "syntaxes/02_Extra/Nix",
    "syntaxes/02_Extra/Protobuf",
    "syntaxes/02_Extra/Puppet",
    "syntaxes/02_Extra/SCSS_Sass",
    "syntaxes/02_Extra/Swift",
    "syntaxes/02_Extra/TOML",
    "syntaxes/02_Extra/Terraform",
    "syntaxes/02_Extra/TypeScript",
    "syntaxes/02_Extra/VimL",
];

/// Sublime Text's license for its default packages, which two-face does not
/// list because it asks for no notice.
const SUBLIME_LICENSE: &str = "Permission to copy, use, modify, sell and distribute this software is \
granted. This software is provided \"as is\" without express or implied warranty, and with no \
claim as to its suitability for any purpose.";

/// Regenerate the syntax definitions and their notices.
#[derive(clap::Args)]
pub struct Args {}

pub fn run(_args: Args) -> anyhow::Result<()> {
    let full = two_face::syntax::extra_newlines();
    // A linked set refers to other contexts by index, which removing syntaxes
    // would shift. The definitions are rewritten as JSON to refer by name, so
    // that linking the subset resolves them again.
    let definitions: Vec<Value> = full
        .into_builder()
        .syntaxes()
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;
    // A syntax's contexts are linked in the order of their names.
    let context_names: Vec<Vec<String>> = definitions
        .iter()
        .map(|definition| {
            let mut names: Vec<String> = definition["contexts"]
                .as_object()
                .map(|contexts| contexts.keys().cloned().collect())
                .unwrap_or_default();
            names.sort();
            names
        })
        .collect();
    let name = |index: usize| definitions[index]["name"].as_str().unwrap_or_default();

    let mut keep: BTreeSet<usize> = (0..definitions.len())
        .filter(|&index| KEEP.contains(&name(index)))
        .collect();
    if keep.len() != KEEP.len() {
        let found: Vec<&str> = keep.iter().map(|&index| name(index)).collect();
        let missing: Vec<&&str> = KEEP.iter().filter(|kept| !found.contains(kept)).collect();
        bail!("two-face has no syntax named {missing:?}");
    }
    loop {
        let mut referenced = BTreeSet::new();
        for &index in &keep {
            let mut definition = definitions[index].clone();
            visit(&mut definition, &mut |value| {
                if let Some(target) = direct(value) {
                    referenced.insert(target.0);
                }
            });
        }
        let before = keep.len();
        keep.extend(referenced);
        if keep.len() == before {
            break;
        }
    }

    let mut builder = SyntaxSetBuilder::new();
    for &index in &keep {
        let mut definition = definitions[index].clone();
        visit(&mut definition, &mut |value| {
            // Linking sets each context's prototype again.
            if let Some(context) = value
                .as_object_mut()
                .filter(|object| object.contains_key("meta_include_prototype"))
            {
                context.insert("prototype".into(), Value::Null);
            }
            if let Some((syntax, context)) = direct(value) {
                let context = &context_names[syntax][context];
                *value = if syntax == index {
                    json!({ "Named": context })
                } else {
                    json!({ "ByScope": {
                        "scope": definitions[syntax]["scope"],
                        "sub_context": context,
                        "with_escape": false,
                    }})
                };
            }
        });
        builder.add(
            serde_json::from_value::<SyntaxDefinition>(definition)
                .with_context(|| format!("rebuilding {}", name(index)))?,
        );
    }
    let set = builder.build();
    let out = Path::new(OUT_DIR);
    fs::create_dir_all(out)?;
    fs::write(
        out.join("syntaxes.packdump"),
        syntect::dumps::dump_binary(&set),
    )?;

    let mut names: Vec<&str> = set
        .syntaxes()
        .iter()
        .filter(|syntax| !syntax.hidden)
        .map(|syntax| syntax.name.as_str())
        .collect();
    names.sort_unstable();
    let mut notices = format!(
        "# Syntax definitions\n\n\
         `syntaxes.packdump` holds the syntax definitions yapi highlights code with. \
         `cargo xtask syntaxes` generates it, and this file, from the definitions that \
         {TWO_FACE} packs: {}.\n\n\
         The definitions from Sublime Text's default packages are under Sublime's license:\n\n\
         > {SUBLIME_LICENSE}\n\n\
         The licenses of the others follow.\n\n",
        names.join(", ")
    );
    let listing = two_face::acknowledgement::listing();
    for package in NOTICES {
        let licenses: Vec<_> = listing
            .for_syntaxes()
            .iter()
            .filter(|license| license.rel_path.starts_with(package))
            .collect();
        if licenses.is_empty() {
            bail!("two-face lists no license for {package}");
        }
        for license in licenses {
            let _ = write!(
                notices,
                "## `{}`\n\n```text\n{}\n```\n\n",
                license.rel_path.display(),
                license.text.trim_end()
            );
        }
    }
    fs::write(
        out.join("LICENSES.md"),
        notices.trim_end().to_owned() + "\n",
    )?;
    println!(
        "wrote {} syntaxes to {}",
        set.syntaxes().len(),
        out.display()
    );
    Ok(())
}

/// Calls `f` on `value` and every value inside it.
fn visit(value: &mut Value, f: &mut dyn FnMut(&mut Value)) {
    f(value);
    match value {
        Value::Array(items) => items.iter_mut().for_each(|item| visit(item, f)),
        Value::Object(fields) => fields.values_mut().for_each(|field| visit(field, f)),
        _ => {}
    }
}

/// The syntax and context indices of a linked context reference.
fn direct(value: &Value) -> Option<(usize, usize)> {
    let id = value.get("Direct")?;
    let index = |key: &str| id[key].as_u64().map(|index| index as usize);
    Some((index("syntax_index")?, index("context_index")?))
}
