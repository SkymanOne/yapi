//! yapi's resource resolver against `tests/fixtures/pi/resolve/cases.json`,
//! recorded from pi's `DefaultPackageManager.resolve` by
//! `tests/fixtures/pi/generator/resolve.mjs`.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use yapi_core::packages::resolve::{
    PackageInput, ProjectOnly, ResolveInput, ResolvedPaths, ResourceType, package_inputs,
    package_resources, resolve, settings_lists,
};
use yapi_core::packages::source::{Source, local_path};

fn fixtures() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pi/resolve/cases.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn document(path: &Path) -> Map<String, Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn as_json(resolved: &ResolvedPaths, root: &Path) -> Value {
    let root = root.to_string_lossy().into_owned();
    let relative = |text: &str| text.replace(&root, "<root>");
    let mut out = Map::new();
    for kind in ResourceType::ALL {
        let list: Vec<Value> = resolved
            .of(kind)
            .iter()
            .map(|resource| {
                let info = &resource.info;
                let mut entry = json!({
                    "path": relative(&info.path),
                    "enabled": resource.enabled,
                    "source": relative(&info.source),
                    "scope": info.scope,
                    "origin": info.origin,
                });
                if let Some(base) = &info.base_dir {
                    entry["baseDir"] = json!(relative(base));
                }
                entry
            })
            .collect();
        out.insert(kind.key().to_owned(), Value::Array(list));
    }
    Value::Object(out)
}

#[test]
fn resolves_resources_as_pi() {
    let scratch = std::env::temp_dir().join(format!("yapi-resolve-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    for (name, case) in fixtures().as_object().unwrap() {
        let root = scratch.join(name);
        for (file, text) in case["files"].as_object().unwrap() {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text.as_str().unwrap()).unwrap();
        }
        for dir in ["agent", "project", "home"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let root = std::fs::canonicalize(&root).unwrap();
        let trusted = case["trusted"].as_bool().unwrap();
        let agent_dir = root.join("agent");
        let cwd = root.join("project");
        let project_dir = cwd.join(".pi");
        let global = document(&agent_dir.join("settings.json"));
        let project = if trusted {
            document(&project_dir.join("settings.json"))
        } else {
            Map::new()
        };
        let base = |scope: &str| -> PathBuf {
            if scope == "project" {
                project_dir.clone()
            } else {
                agent_dir.clone()
            }
        };
        let packages: Vec<PackageInput> = package_inputs(
            project
                .get("packages")
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice),
            global
                .get("packages")
                .and_then(Value::as_array)
                .map_or(&[][..], Vec::as_slice),
            base,
            |source, scope| match source {
                Source::Local { path } => local_path(path, &base(scope)),
                _ => PathBuf::from("/nonexistent-package"),
            },
        );
        let user = settings_lists(&global);
        let project_lists = settings_lists(&project);
        let input = ResolveInput {
            cwd: cwd.clone(),
            agent_dir: agent_dir.clone(),
            project_dir: project_dir.clone(),
            home: root.join("home"),
            project_trusted: trusted,
            packages,
            skills: Vec::new(),
            user: [&user[0][..], &user[1][..], &user[2][..], &user[3][..]],
            project: [
                &project_lists[0][..],
                &project_lists[1][..],
                &project_lists[2][..],
                &project_lists[3][..],
            ],
            builtins: &["mcp"],
            project_only: None,
        };
        let actual = as_json(&resolve(&input), &root);
        for kind in ResourceType::ALL {
            assert_eq!(
                actual[kind.key()],
                case[kind.key()],
                "{name}: {}",
                kind.key()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

/// pi's `readPiManifest` ignores a manifest list unless every entry is a
/// string.
#[test]
fn ignores_non_string_lists() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("resolve-mixed-manifest");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("prompts")).unwrap();
    std::fs::write(root.join("a.ts"), "").unwrap();
    std::fs::write(root.join("prompts/p.md"), "").unwrap();
    std::fs::write(
        root.join("package.json"),
        r#"{"pi": {"extensions": ["./a.ts", 1], "prompts": ["./prompts/p.md"]}}"#,
    )
    .unwrap();
    let resources = package_resources(&root, None, true);
    assert!(enabled(&resources, ResourceType::Extensions).is_empty());
    assert_eq!(
        enabled(&resources, ResourceType::Prompts),
        [root.join("prompts/p.md")]
    );
}

/// A `chest.toml` `[package]` loads a package without `package.json`, with
/// the globs and `!` patterns of pi's lists, and each kind falls back on its
/// own: a `yapi` key holding only capabilities hides no `pi` list.
#[test]
fn package_lists_resolve() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let chest = tmp.join("package-chest");
    write(
        &chest,
        &[
            (
                "chest.toml",
                "[package]\nname = \"acme\"\nextensions = [\"ext/*.ts\", \"!ext/skip.ts\"]\nskills = [\"skills\"]\n",
            ),
            ("ext/a.ts", ""),
            ("ext/skip.ts", ""),
            ("skills/x/SKILL.md", ""),
            ("prompts/p.md", ""),
        ],
    );
    let resources = package_resources(&chest, None, false);
    assert_eq!(
        enabled(&resources, ResourceType::Extensions),
        [chest.join("ext/a.ts")]
    );
    assert_eq!(
        enabled(&resources, ResourceType::Skills),
        [chest.join("skills/x/SKILL.md")]
    );
    // As with pi's lists, a kind none names provides nothing.
    assert!(enabled(&resources, ResourceType::Prompts).is_empty());
    assert!(resources.problems.is_empty());
    // A `[package]` that does not parse is reported, and the folders apply.
    std::fs::write(chest.join("chest.toml"), "[package]\nauthor = \"Ada\"\n").unwrap();
    let resources = package_resources(&chest, None, false);
    assert_eq!(
        enabled(&resources, ResourceType::Prompts),
        [chest.join("prompts/p.md")]
    );
    let [(folder, message)] = &resources.problems[..] else {
        panic!("{:?}", resources.problems);
    };
    assert_eq!(folder, &chest.display().to_string());
    assert!(message.contains("unknown field `author`"), "{message}");

    let mixed = tmp.join("package-yapi-capabilities");
    write(
        &mixed,
        &[
            (
                "package.json",
                r#"{"yapi": {"capabilities": ["fs"]}, "pi": {"extensions": ["./pi.ts"]}}"#,
            ),
            ("pi.ts", ""),
            ("native.wasm", ""),
            ("p.md", ""),
            ("extensions/other.ts", ""),
        ],
    );
    let resources = package_resources(&mixed, None, false);
    assert_eq!(
        enabled(&resources, ResourceType::Extensions),
        [mixed.join("pi.ts")]
    );
    // A chest.toml that does not parse is reported and hides no package.json list.
    std::fs::write(mixed.join("chest.toml"), "[package\n").unwrap();
    let resources = package_resources(&mixed, None, false);
    assert_eq!(
        enabled(&resources, ResourceType::Extensions),
        [mixed.join("pi.ts")]
    );
    assert_eq!(resources.problems.len(), 1, "{:?}", resources.problems);
    std::fs::remove_file(mixed.join("chest.toml")).unwrap();
    // A `yapi` list still wins for its kind, and pi's lists fill the rest.
    std::fs::write(
        mixed.join("package.json"),
        r#"{"yapi": {"extensions": ["./native.wasm"]}, "pi": {"extensions": ["./pi.ts"], "prompts": ["./p.md"]}}"#,
    )
    .unwrap();
    let resources = package_resources(&mixed, None, false);
    assert_eq!(
        enabled(&resources, ResourceType::Extensions),
        [mixed.join("native.wasm")]
    );
    assert_eq!(
        enabled(&resources, ResourceType::Prompts),
        [mixed.join("p.md")]
    );

    // A settings entry cannot pass for one of chest's packages.
    std::fs::create_dir_all(tmp.join("chest:package-chest")).unwrap();
    let entries = [json!("chest:package-chest"), json!("package-chest")];
    let inputs = package_inputs(
        &entries,
        &[],
        |_| tmp.to_path_buf(),
        |source, _| match source {
            Source::Local { path } => local_path(path, tmp),
            _ => PathBuf::new(),
        },
    );
    let sources: Vec<&str> = inputs.iter().map(|input| input.source.as_str()).collect();
    assert_eq!(sources, ["package-chest"]);
}

/// The enabled resources of `kind`, as paths.
fn enabled(resolved: &ResolvedPaths, kind: ResourceType) -> Vec<PathBuf> {
    resolved
        .enabled(kind)
        .map(|info| PathBuf::from(&info.path))
        .collect()
}

fn write(root: &Path, files: &[(&str, &str)]) {
    let _ = std::fs::remove_dir_all(root);
    for (name, text) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn package_resource_discovery() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let root = tmp.join("package-manifest");
    write(
        &root,
        &[
            (
                "package.json",
                r#"{"pi": {"extensions": ["./src/*.ts", "!src/skip.ts", "packages/*/extensions"], "skills": ["./skills"]}}"#,
            ),
            ("src/a.ts", ""),
            ("src/skip.ts", ""),
            ("packages/x/extensions/tool/index.ts", ""),
            ("skills/x/SKILL.md", ""),
            ("prompts/p.md", ""),
        ],
    );
    let resources = package_resources(&root, None, false);
    // A glob that matches a folder takes the extensions in it.
    assert_eq!(
        enabled(&resources, ResourceType::Extensions),
        [
            root.join("src/a.ts"),
            root.join("packages/x/extensions/tool/index.ts")
        ]
    );
    assert_eq!(
        enabled(&resources, ResourceType::Skills),
        [root.join("skills/x/SKILL.md")]
    );
    // Only the types the manifest lists.
    assert!(enabled(&resources, ResourceType::Prompts).is_empty());

    let plain = tmp.join("package-conventional");
    write(
        &plain,
        &[
            ("extensions/a.ts", ""),
            ("extensions/b.js", ""),
            ("prompts/p.md", ""),
        ],
    );
    let resources = package_resources(&plain, None, false);
    assert_eq!(enabled(&resources, ResourceType::Extensions).len(), 2);
    assert_eq!(
        enabled(&resources, ResourceType::Prompts),
        [plain.join("prompts/p.md")]
    );
    let filtered = yapi_types::settings::FilteredPackage {
        source: String::new(),
        autoload: None,
        extensions: Some(vec!["extensions/a.ts".into()]),
        skills: None,
        prompts: Some(Vec::new()),
        themes: None,
    };
    let resources = package_resources(&plain, Some(&filtered), false);
    assert_eq!(
        enabled(&resources, ResourceType::Extensions),
        [plain.join("extensions/a.ts")]
    );
    assert!(enabled(&resources, ResourceType::Prompts).is_empty());

    // A folder with neither is one extension only as a local path.
    let bare = tmp.join("package-bare");
    write(&bare, &[("index.ts", "")]);
    assert_eq!(
        enabled(
            &package_resources(&bare, None, true),
            ResourceType::Extensions
        ),
        [bare.join("index.ts")]
    );
    assert!(
        enabled(
            &package_resources(&bare, None, false),
            ResourceType::Extensions
        )
        .is_empty()
    );
}

/// A project-only run loads the project's resources and the built-ins, and
/// of the user's only what `projectOnly` keeps: whole packages by identity
/// or `chest:<name>`, and single resources by pattern, wherever they come
/// from.
#[test]
fn project_only_keeps() {
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let root = tmp.join("project-only");
    let package_json =
        r#"{"name": "x", "pi": {"extensions": ["./ext.ts"], "themes": ["./t.json"]}}"#;
    write(
        &root,
        &[
            ("agent/extensions/global.ts", ""),
            ("agent/themes/dark.json", "{}"),
            ("home/.agents/skills/pdf/SKILL.md", "# pdf\n"),
            ("loose.ts", ""),
            ("pkg/package.json", package_json),
            ("pkg/ext.ts", ""),
            ("pkg/t.json", "{}"),
            (
                "notes/package.json",
                r#"{"name": "notes", "pi": {"extensions": ["./n.ts"]}}"#,
            ),
            ("notes/n.ts", ""),
            ("project/.git/HEAD", ""),
            ("project/.yapi/extensions/p.ts", ""),
            ("project/.agents/skills/ps/SKILL.md", "# ps\n"),
        ],
    );
    let root = std::fs::canonicalize(&root).unwrap();
    let user_lists = [
        vec!["-builtin:mcp".to_owned(), "../loose.ts".to_owned()],
        vec![],
        vec![],
        vec![],
    ];
    let none: [Vec<String>; 4] = Default::default();
    let package = |source: &str, folder: &str| PackageInput {
        source: source.into(),
        scope: "user".into(),
        root: root.join(folder),
        filter: None,
    };
    let names = |project_only: Option<ProjectOnly>| {
        let input = ResolveInput {
            cwd: root.join("project"),
            agent_dir: root.join("agent"),
            project_dir: root.join("project/.yapi"),
            home: root.join("home"),
            project_trusted: true,
            packages: vec![package("npm:x@1.2", "pkg"), package("chest:notes", "notes")],
            skills: Vec::new(),
            user: [
                &user_lists[0][..],
                &user_lists[1][..],
                &user_lists[2][..],
                &user_lists[3][..],
            ],
            project: [&none[0][..], &none[1][..], &none[2][..], &none[3][..]],
            builtins: &["mcp", "codemode"],
            project_only,
        };
        let resolved = resolve(&input);
        let mut names: Vec<String> = ResourceType::ALL
            .into_iter()
            .flat_map(|kind| {
                resolved.enabled(kind).map(|info| {
                    let path = Path::new(&info.path);
                    match path.file_name().and_then(|name| name.to_str()) {
                        Some("SKILL.md") => path
                            .parent()
                            .unwrap()
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        _ => path.file_name().map_or(info.path.clone(), |name| {
                            name.to_string_lossy().into_owned()
                        }),
                    }
                })
            })
            .collect();
        names.sort();
        names
    };
    let only = |packages: &[&str], kinds: [&[&str]; 4]| {
        Some(ProjectOnly {
            packages: packages.iter().map(|entry| (*entry).to_owned()).collect(),
            kinds: kinds.map(|list| list.iter().map(|entry| (*entry).to_owned()).collect()),
            mcp_servers: Vec::new(),
        })
    };
    let project = ["builtin:codemode", "p.ts", "ps"];
    let with = |extra: &[&str]| {
        let mut all: Vec<String> = project
            .iter()
            .chain(extra)
            .map(|name| (*name).to_owned())
            .collect();
        all.sort();
        all
    };
    // Without the mode, everything loads, and the user's `-builtin:mcp` holds.
    assert_eq!(
        names(None),
        with(&[
            "dark.json",
            "ext.ts",
            "global.ts",
            "loose.ts",
            "n.ts",
            "pdf",
            "t.json"
        ])
    );
    for (keep, extra) in [
        (only(&[], [&[], &[], &[], &[]]), &[][..]),
        (
            only(&[], [&[], &[], &[], &["*"]]),
            &["dark.json", "t.json"][..],
        ),
        (
            only(&["npm:x"], [&[], &[], &[], &[]]),
            &["ext.ts", "t.json"][..],
        ),
        (only(&["chest:notes"], [&[], &[], &[], &[]]), &["n.ts"][..]),
        (
            only(&["*"], [&[], &[], &[], &[]]),
            &["ext.ts", "n.ts", "t.json"][..],
        ),
        (
            only(&[], [&[], &["~/.agents/skills/pdf"], &[], &[]]),
            &["pdf"][..],
        ),
        (
            only(&[], [&["extensions/global.ts", "loose.ts"], &[], &[], &[]]),
            &["global.ts", "loose.ts"][..],
        ),
    ] {
        assert_eq!(names(keep.clone()), with(extra), "{keep:?}");
    }
    // A user's copy left out never shadows the project's copy of one file.
    let shared = PackageInput {
        source: "../project/.yapi/extensions".into(),
        ..package("../project/.yapi/extensions", "project/.yapi/extensions")
    };
    let input = ResolveInput {
        cwd: root.join("project"),
        agent_dir: root.join("agent"),
        project_dir: root.join("project/.yapi"),
        home: root.join("home"),
        project_trusted: true,
        packages: vec![shared],
        user: [&none[0][..], &none[1][..], &none[2][..], &none[3][..]],
        project: [&none[0][..], &none[1][..], &none[2][..], &none[3][..]],
        project_only: only(&[], [&[], &[], &[], &[]]),
        ..ResolveInput::default()
    };
    let resolved = resolve(&input);
    let sources: Vec<(&str, &str)> = resolved
        .enabled(ResourceType::Extensions)
        .map(|info| (info.scope.as_str(), info.source.as_str()))
        .collect();
    assert_eq!(sources, [("project", "auto")]);
    assert!(resolved.shadowed.is_empty(), "{:?}", resolved.shadowed);
}

/// The `projectOnly` setting, and how it matches one resource.
#[test]
fn project_only_setting() {
    let document = json!({"projectOnly": {"packages": ["npm:x"], "skills": ["~/s/"], "themes": ["*"], "mcpServers": ["git*"]}});
    let keep = ProjectOnly::from_settings(document.as_object().unwrap());
    assert_eq!(keep.packages, ["npm:x"]);
    // A trailing `/` names the folder all the same.
    assert_eq!(keep.kinds[ResourceType::Skills as usize], ["~/s"]);
    assert!(keep.kinds[ResourceType::Extensions as usize].is_empty());
    assert_eq!(keep.mcp_servers, ["git*"]);
    let info = |path: &str, source: &str, origin: &str| yapi_types::rpc::SourceInfo {
        path: path.into(),
        source: source.into(),
        scope: "user".into(),
        origin: origin.into(),
        base_dir: None,
    };
    let (agent, home) = (Path::new("/a"), Path::new("/h"));
    let skills = ResourceType::Skills;
    assert!(keep.keeps(
        &info("/h/s/SKILL.md", "auto", "top-level"),
        skills,
        agent,
        home
    ));
    assert!(!keep.keeps(
        &info("/h/t/SKILL.md", "auto", "top-level"),
        skills,
        agent,
        home
    ));
    assert!(keep.keeps(
        &info("/a/npm/x/e.ts", "npm:x@^2", "package"),
        ResourceType::Extensions,
        agent,
        home
    ));
    assert!(!keep.keeps(
        &info("/a/npm/y/e.ts", "npm:y", "package"),
        ResourceType::Extensions,
        agent,
        home
    ));
    assert!(
        keep.needs_package("npm:y", agent),
        "a kind list could match inside it"
    );
    let packages_only = ProjectOnly {
        packages: vec!["npm:x".into()],
        ..ProjectOnly::default()
    };
    assert!(
        packages_only.needs_package("npm:x@1", agent)
            && !packages_only.needs_package("npm:y", agent)
    );
    assert!(
        !packages_only.needs_chest()
            && ProjectOnly {
                packages: vec!["chest:a".into()],
                ..ProjectOnly::default()
            }
            .needs_chest()
    );
}
