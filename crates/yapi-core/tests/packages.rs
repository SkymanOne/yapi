//! Packages against a mock npm registry: installs with hoisted dependencies,
//! settings entries, resources, removal, native addons and integrity.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Write as _};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use aws_lc_rs::digest::{SHA512, digest};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use indexmap::IndexMap;
use serde_json::{Value, json};
use yapi_ai::auth::lock::FileLock;
use yapi_core::packages::PackageManager;
use yapi_core::packages::npm::{self, Npm, Pinned};
use yapi_core::settings::{Scope, SettingsManager};
use yapi_mock::MockServer;
use yapi_mock::{Cassette, Interaction, RequestMatch, Response};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yapi-packages-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An npm tarball with `files` under `package/`.
fn tarball(files: &[(&str, &str)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, text) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(text.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("package/{name}"), text.as_bytes())
            .unwrap();
    }
    let tar = builder.into_inner().unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&tar).unwrap();
    gz.finish().unwrap()
}

fn json_response(body: &Value) -> Response {
    let mut headers = IndexMap::new();
    headers.insert("content-type".into(), "application/json".into());
    Response {
        status: 200,
        headers,
        chunks: vec![body.to_string()],
        body_base64: None,
        chunk_delay_ms: 0,
        close: None,
    }
}

fn get(path: &str, response: Response) -> Interaction {
    Interaction {
        request: RequestMatch {
            method: "GET".into(),
            path: path.into(),
        },
        response,
    }
}

/// The packument and tarball interactions for `name@version`.
fn publish(
    base: &str,
    name: &str,
    versions: &[(&str, Value, Vec<u8>)],
    served: &str,
) -> Vec<Interaction> {
    let mut manifests = serde_json::Map::new();
    let mut tarball_bytes = Vec::new();
    let file = name.rsplit('/').next().unwrap();
    for (version, manifest, bytes) in versions {
        let mut manifest = manifest.clone();
        manifest["name"] = json!(name);
        manifest["version"] = json!(version);
        manifest["dist"] = json!({
            "tarball": format!("{base}/{name}/-/{file}-{version}.tgz"),
            "integrity": format!("sha512-{}", STANDARD.encode(digest(&SHA512, bytes))),
        });
        manifests.insert((*version).to_owned(), manifest);
        if *version == served {
            tarball_bytes = bytes.clone();
        }
    }
    let latest = versions.last().unwrap().0;
    let packument = json!({"name": name, "dist-tags": {"latest": latest}, "versions": manifests});
    vec![
        get(&format!("/{name}"), json_response(&packument)),
        get(
            &format!("/{name}/-/{file}-{served}.tgz"),
            Response {
                status: 200,
                headers: IndexMap::new(),
                chunks: Vec::new(),
                body_base64: Some(STANDARD.encode(tarball_bytes)),
                chunk_delay_ms: 0,
                close: None,
            },
        ),
    ]
}

async fn registry(interactions: impl Fn(&str) -> Vec<Interaction>) -> MockServer {
    MockServer::local_with(|base| Cassette {
        interactions: interactions(base),
    })
    .await
    .unwrap()
}

/// A manager for the trusted project `<dir>/project` and the agent directory
/// `<dir>/agent`, with `url` as the registry.
fn manager(dir: &Path, url: &str) -> PackageManager {
    let cwd = dir.join("project");
    let agent = dir.join("agent");
    std::fs::create_dir_all(&cwd).unwrap();
    let settings = SettingsManager::load(&agent, &cwd, true);
    let config = [("registry".to_owned(), format!("{url}/"))];
    PackageManager::new(cwd, agent, settings, config.into())
}

#[tokio::test(flavor = "multi_thread")]
async fn installs_npm_dependencies() {
    let dir = scratch("npm");
    let extension = tarball(&[
        (
            "package.json",
            r#"{"name": "ext-pkg", "pi": {"extensions": ["./index.ts"]}}"#,
        ),
        ("index.ts", "export default function () {}\n"),
    ]);
    let pad = |version: &str| {
        tarball(&[
            (
                "package.json",
                &format!(r#"{{"name": "left-pad", "version": "{version}"}}"#),
            ),
            ("index.js", "module.exports = 1;\n"),
        ])
    };
    let server = registry(|base| {
        let mut all = publish(
            base,
            "ext-pkg",
            &[(
                "1.0.0",
                json!({"dependencies": {"left-pad": "^1.1.0", "@earendil-works/pi-coding-agent": "*"}}),
                extension.clone(),
            )],
            "1.0.0",
        );
        all.extend(publish(
            base,
            "left-pad",
            &[("1.0.0", json!({}), pad("1.0.0")), ("1.3.0", json!({}), pad("1.3.0")), ("2.0.0", json!({}), pad("2.0.0"))],
            "1.3.0",
        ));
        all
    })
    .await;
    let mut packages = manager(&dir, &server.url());
    let progress = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = progress.clone();
    packages.on_progress(move |line| log.lock().unwrap().push(line.to_owned()));
    packages.install("npm:ext-pkg", false).await.unwrap();
    assert_eq!(*progress.lock().unwrap(), ["Installing npm:ext-pkg..."]);

    let root = dir.join("agent/npm");
    assert!(root.join("node_modules/ext-pkg/index.ts").exists());
    let pad_manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("node_modules/left-pad/package.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(pad_manifest["version"], "1.3.0");
    assert!(!root.join("node_modules/@earendil-works").exists());
    assert_eq!(
        std::fs::read_to_string(root.join(".gitignore")).unwrap(),
        "*\n!.gitignore\n"
    );
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
    assert_eq!(
        manifest,
        json!({"name": "pi-extensions", "private": true, "dependencies": {"ext-pkg": "^1.0.0"}})
    );
    assert_eq!(
        packages.settings().document(Scope::Global)["packages"],
        json!(["npm:ext-pkg"])
    );

    let listed = packages.list();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].installed_path.as_deref(),
        Some(root.join("node_modules/ext-pkg").as_path())
    );
    assert_eq!(
        listed[0].extensions,
        [root.join("node_modules/ext-pkg/index.ts")]
    );

    assert!(packages.remove("npm:ext-pkg", false).await.unwrap());
    assert!(!root.join("node_modules/ext-pkg").exists());
    assert!(
        !root.join("node_modules/left-pad").exists(),
        "unreferenced dependencies are pruned"
    );
    assert_eq!(
        packages.settings().document(Scope::Global)["packages"],
        json!([])
    );
    assert!(!packages.remove("npm:ext-pkg", false).await.unwrap());
    server.finish().unwrap();
}

/// A dependency already resolvable through any `node_modules` up from its
/// dependent is checked there, as Node resolves it: a different version in
/// between gets a nested copy, and a cycle ends at its ancestor's copy.
#[tokio::test(flavor = "multi_thread")]
async fn ancestor_node_modules() {
    let dir = scratch("npm-ancestors");
    let versions = |name: &str, versions: &[(&str, Value)]| -> Vec<(String, Value, Vec<u8>)> {
        versions
            .iter()
            .map(|(version, dependencies)| {
                let manifest = json!({"name": name, "version": version});
                let bytes = tarball(&[("package.json", &manifest.to_string())]);
                let manifest = json!({ "dependencies": dependencies });
                ((*version).to_owned(), manifest, bytes)
            })
            .collect()
    };
    let packages = [
        (
            "ext",
            versions(
                "ext",
                &[(
                    "1.0.0",
                    json!({"x": "^1", "d": "^2", "b": "^1", "p": "^1", "q": "^1", "c": "^1"}),
                )],
            ),
        ),
        (
            "x",
            versions("x", &[("1.0.0", json!({})), ("2.0.0", json!({}))]),
        ),
        (
            "d",
            versions("d", &[("1.0.0", json!({"x": "^1"})), ("2.0.0", json!({}))]),
        ),
        (
            "b",
            versions("b", &[("1.0.0", json!({"x": "^2", "d": "^1"}))]),
        ),
        (
            "p",
            versions("p", &[("1.0.0", json!({})), ("2.0.0", json!({"q": "^2"}))]),
        ),
        (
            "q",
            versions("q", &[("1.0.0", json!({})), ("2.0.0", json!({"p": "^2"}))]),
        ),
        ("c", versions("c", &[("1.0.0", json!({"p": "^2"}))])),
    ];
    let server = registry(|base| {
        let fetch = |name: &str, version: &str| {
            let (_, versions) = packages.iter().find(|(n, _)| *n == name).unwrap();
            let versions: Vec<(&str, Value, Vec<u8>)> = versions
                .iter()
                .map(|(v, manifest, bytes)| (v.as_str(), manifest.clone(), bytes.clone()))
                .collect();
            publish(base, name, &versions, version)
        };
        // The packument is fetched once per name.
        let cached = |name: &str, version: &str| fetch(name, version).split_off(1);
        [
            fetch("ext", "1.0.0"),
            fetch("x", "1.0.0"),
            fetch("d", "2.0.0"),
            fetch("b", "1.0.0"),
            cached("x", "2.0.0"),
            cached("d", "1.0.0"),
            // d@1 resolves `x` to b's x@2, so it gets its own x@1.
            cached("x", "1.0.0"),
            fetch("p", "1.0.0"),
            fetch("q", "1.0.0"),
            fetch("c", "1.0.0"),
            cached("p", "2.0.0"),
            // q@2 finds p@2 two levels up and the cycle ends.
            cached("q", "2.0.0"),
        ]
        .concat()
    })
    .await;
    let mut packages = manager(&dir, &server.url());
    packages.install("npm:ext", false).await.unwrap();
    server.finish().unwrap();

    let modules = dir.join("agent/npm/node_modules");
    let version = |path: &str| {
        let text = std::fs::read_to_string(modules.join(path).join("package.json")).unwrap();
        serde_json::from_str::<Value>(&text).unwrap()["version"].clone()
    };
    for (path, expected) in [
        ("x", "1.0.0"),
        ("d", "2.0.0"),
        ("b/node_modules/x", "2.0.0"),
        ("b/node_modules/d", "1.0.0"),
        ("b/node_modules/d/node_modules/x", "1.0.0"),
        ("p", "1.0.0"),
        ("q", "1.0.0"),
        ("c/node_modules/p", "2.0.0"),
        ("c/node_modules/p/node_modules/q", "2.0.0"),
    ] {
        assert_eq!(version(path), expected, "{path}");
    }
    assert!(
        !modules
            .join("c/node_modules/p/node_modules/q/node_modules")
            .exists()
    );
}

/// Writes the agent's `settings.json` with `packages`.
fn configure(dir: &Path, packages: Value) {
    let agent = dir.join("agent");
    std::fs::create_dir_all(&agent).unwrap();
    std::fs::write(
        agent.join("settings.json"),
        json!({ "packages": packages }).to_string(),
    )
    .unwrap();
}

/// `npm/node_modules/<name>` in the agent directory, as installed at
/// `version`.
fn installed(dir: &Path, name: &str, version: &str) -> PathBuf {
    let root = dir.join("agent/npm/node_modules").join(name);
    std::fs::create_dir_all(&root).unwrap();
    let manifest = json!({"name": name, "version": version});
    std::fs::write(root.join("package.json"), manifest.to_string()).unwrap();
    root
}

/// pi compares the installed version with the newest one in the source's
/// range, and `update` installs that one.
#[tokio::test(flavor = "multi_thread")]
async fn npm_updates_in_range() {
    let dir = scratch("npm-updates");
    configure(
        &dir,
        json!([
            "npm:ext-pkg@^1.0.0",
            "npm:fixed@1.0.0",
            "npm:missing",
            "./local"
        ]),
    );
    installed(&dir, "fixed", "0.9.0");
    let root = installed(&dir, "ext-pkg", "1.0.0");
    let release = |version: &str| {
        let manifest = format!(r#"{{"name": "ext-pkg", "version": "{version}"}}"#);
        tarball(&[("package.json", &manifest)])
    };
    // Each check reads the packument afresh, like pi's `npm view`: the
    // check and the update's own check. The install reads it once more and
    // keeps it for the check after.
    let server = registry(|base| {
        let versions =
            ["1.0.0", "1.2.0", "2.0.0"].map(|version| (version, json!({}), release(version)));
        let [packument, tarball] =
            <[Interaction; 2]>::try_from(publish(base, "ext-pkg", &versions, "1.2.0")).unwrap();
        vec![packument.clone(), packument.clone(), packument, tarball]
    })
    .await;
    let mut packages = manager(&dir, &server.url());
    let progress = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = progress.clone();
    packages.on_progress(move |line| log.lock().unwrap().push(line.to_owned()));
    assert_eq!(packages.available_updates().await, ["ext-pkg"]);
    packages.update(Some("npm:ext-pkg")).await.unwrap();
    assert_eq!(
        *progress.lock().unwrap(),
        ["Updating npm:ext-pkg@^1.0.0..."]
    );
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
    assert_eq!(manifest["version"], "1.2.0");
    assert!(packages.available_updates().await.is_empty());
    // A current package is neither reported nor reinstalled.
    packages.update(Some("npm:ext-pkg")).await.unwrap();
    assert_eq!(progress.lock().unwrap().len(), 1);
    assert_eq!(server.finish().unwrap().len(), 4);
}

/// With `npmCommand`, pi asks that command's `npm view` for the versions.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn npm_command_updates() {
    let dir = scratch("npm-command-updates");
    let script = dir.join("npm.sh");
    std::fs::write(
        &script,
        "[ \"$1 $3 $4\" = 'view version --json' ] && echo '[\"1.0.0\", \"1.5.0\", \"2.0.0\"]'\n\
         [ \"$1\" != install ] || sleep 1\n",
    )
    .unwrap();
    installed(&dir, "behind", "1.0.0");
    installed(&dir, "current", "2.0.0");
    std::fs::write(
        dir.join("agent/settings.json"),
        json!({
            "npmCommand": ["sh", script],
            "packages": ["npm:behind@^1.0.0", "npm:current@^2.0.0"],
        })
        .to_string(),
    )
    .unwrap();
    let mut packages = manager(&dir, "http://127.0.0.1:9");
    assert_eq!(packages.available_updates().await, ["behind"]);
    // The install waits on the command without blocking the runtime, so a
    // timeout ends it.
    let install = packages.install("npm:behind@^1.0.0", false);
    let waited = tokio::time::timeout(std::time::Duration::from_millis(100), install).await;
    assert!(waited.is_err(), "{waited:?}");
}

fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(["-c", "user.name=yapi", "-c", "user.email=yapi@example.com"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// pi compares a git package's commit with its remote branch's. Pinned
/// sources are left alone.
#[tokio::test(flavor = "multi_thread")]
async fn git_remote_updates() {
    let dir = scratch("git-updates");
    let origin = dir.join("origin.git");
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    git(&dir, &["init", "--bare", "-b", "main", "origin.git"]);
    git(&work, &["init", "-b", "main"]);
    git(&work, &["commit", "--allow-empty", "-m", "first"]);
    git(
        &work,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    git(&work, &["push", "origin", "main"]);
    for path in ["user/repo", "user/pinned"] {
        let checkout = dir.join("agent/git/github.com").join(path);
        std::fs::create_dir_all(checkout.parent().unwrap()).unwrap();
        git(
            &dir,
            &[
                "clone",
                origin.to_str().unwrap(),
                checkout.to_str().unwrap(),
            ],
        );
    }
    configure(
        &dir,
        json!([
            "git:github.com/user/repo",
            "git:github.com/user/pinned@main"
        ]),
    );
    let packages = manager(&dir, "http://127.0.0.1:9");
    assert!(packages.available_updates().await.is_empty());

    git(&work, &["commit", "--allow-empty", "-m", "second"]);
    git(&work, &["push", "origin", "main"]);
    assert_eq!(packages.available_updates().await, ["github.com/user/repo"]);

    // `update` moves the checkouts and, as pi's `updateGit`, installs a
    // missing one into the git folder, which gets pi's `.gitignore` first.
    configure(
        &dir,
        json!([
            "git:github.com/user/repo",
            "git:github.com/user/pinned@main",
            "git:http://127.0.0.1:9/user/missing"
        ]),
    );
    let mut packages = manager(&dir, "http://127.0.0.1:9");
    let error = packages.update(None).await.unwrap_err().to_string();
    assert!(error.starts_with("git clone failed"), "{error}");
    assert!(packages.available_updates().await.is_empty());
    assert_eq!(
        std::fs::read_to_string(dir.join("agent/git/.gitignore")).unwrap(),
        "*\n!.gitignore\n"
    );

    // A package folder that is no checkout of its own, as a clone of a project
    // that committed it leaves, is refused rather than resetting the project.
    let project = dir.join("project");
    git(&dir, &["clone", origin.to_str().unwrap(), "project"]);
    std::fs::write(project.join("notes.txt"), "committed").unwrap();
    git(&project, &["add", "notes.txt"]);
    git(&project, &["commit", "-m", "notes"]);
    git(&project, &["push", "origin", "main"]);
    std::fs::write(project.join("notes.txt"), "uncommitted").unwrap();
    std::fs::create_dir_all(project.join(".yapi/git/github.com/user/repo")).unwrap();
    let settings = json!({"packages": ["git:github.com/user/repo"]});
    std::fs::write(project.join(".yapi/settings.json"), settings.to_string()).unwrap();
    configure(&dir, json!([]));
    let mut packages = manager(&dir, "http://127.0.0.1:9");
    let error = packages.update(None).await.unwrap_err().to_string();
    assert!(error.starts_with("Refusing to update "), "{error}");
    assert_eq!(
        std::fs::read_to_string(project.join("notes.txt")).unwrap(),
        "uncommitted"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn native_addons_and_integrity() {
    let dir = scratch("native");
    let native = tarball(&[
        (
            "package.json",
            r#"{"name": "native-pkg", "scripts": {"install": "node-gyp rebuild"}}"#,
        ),
        ("binding.gyp", "{}"),
    ]);
    let server = registry(|base| {
        publish(
            base,
            "native-pkg",
            &[("1.0.0", json!({}), native.clone())],
            "1.0.0",
        )
    })
    .await;
    let mut packages = manager(&dir, &server.url());
    packages
        .install("npm:native-pkg@1.0.0", false)
        .await
        .unwrap();
    let installed = dir.join("agent/npm/node_modules/native-pkg");
    assert!(installed.join("binding.gyp").exists());
    // No install script ran, so nothing was built.
    assert!(!installed.join("build").exists());

    let dir = scratch("integrity");
    let good = tarball(&[("package.json", r#"{"name": "pkg"}"#)]);
    let server = registry(|base| {
        let mut all = publish(base, "pkg", &[("1.0.0", json!({}), good.clone())], "1.0.0");
        all[1].response.body_base64 = Some(STANDARD.encode(b"tampered"));
        all
    })
    .await;
    let mut packages = manager(&dir, &server.url());
    let error = packages.install("npm:pkg", false).await.unwrap_err();
    assert_eq!(error.to_string(), "Integrity check failed for pkg@1.0.0");
}

/// The files below `dir` with their contents, by relative path.
fn files(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let contents = std::fs::read(&path).unwrap();
                found.push((path.strip_prefix(dir).unwrap().to_owned(), contents));
            }
        }
    }
    found.sort();
    found
}

/// An install pins each package it unpacks, nested and aliased ones
/// included, with a sha512 even where the registry sent sha1, and leaves out
/// an optional dependency that failed. Replay rebuilds the tree from the
/// tarballs alone, and refuses tampered tarballs and paths outside
/// `node_modules`.
#[tokio::test(flavor = "multi_thread")]
async fn npm_tree_replays() {
    let dir = scratch("replay");
    let package = |name: &str, version: &str, dependencies: Value| {
        let bytes = tarball(&[
            (
                "package.json",
                &json!({"name": name, "version": version}).to_string(),
            ),
            (
                "index.js",
                &format!("module.exports = '{name}@{version}';\n"),
            ),
        ]);
        (name.to_owned(), version.to_owned(), dependencies, bytes)
    };
    // `@acme/lib`'s `util` is an alias that nests beside app's `util@1`.
    // `opt` nests one too, then fails on `@acme/lib@^2`.
    let packages = [
        package(
            "app",
            "1.0.0",
            json!({"dependencies": {"util": "^1", "@acme/lib": "^1"}, "optionalDependencies": {"opt": "^1"}}),
        ),
        package("util", "1.0.0", json!({})),
        package(
            "@acme/lib",
            "1.0.0",
            json!({"dependencies": {"util": "npm:@acme/fork@^2"}}),
        ),
        package("@acme/fork", "2.0.0", json!({})),
        package(
            "opt",
            "1.0.0",
            json!({"dependencies": {"util": "npm:@acme/fork@^2", "@acme/lib": "^2"}}),
        ),
    ];
    let published = |base: &str, name: &str| {
        let (_, version, manifest, bytes) = packages.iter().find(|(n, ..)| n == name).unwrap();
        let mut all = publish(
            base,
            name,
            &[(version.as_str(), manifest.clone(), bytes.clone())],
            version,
        );
        all[0].request.path = format!("/{}", name.replace('/', "%2f"));
        if name == "util" {
            let mut packument: Value = serde_json::from_str(&all[0].response.chunks[0]).unwrap();
            let sha1 = digest(&aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY, bytes);
            packument["versions"][version]["dist"]["integrity"] =
                json!(format!("sha1-{}", STANDARD.encode(sha1)));
            all[0].response.chunks = vec![packument.to_string()];
        }
        all
    };
    let server = registry(|base| {
        let mut all: Vec<Interaction> = packages
            .iter()
            .flat_map(|(name, ..)| published(base, name))
            .collect();
        // opt's nested `@acme/fork`, then `fetch` of `app`.
        all.push(published(base, "@acme/fork").remove(1));
        all.push(published(base, "app").remove(1));
        all
    })
    .await;
    let mut npm = Npm::new([("registry".to_owned(), format!("{}/", server.url()))].into());
    let root = dir.join("installed");
    let installed = npm::install(&mut npm, &root, "app", None).await.unwrap();
    assert_eq!(installed.version, "1.0.0");
    let mut tree = installed.tree;
    tree.sort_by(|a, b| a.path.cmp(&b.path));
    let summary: Vec<_> = tree
        .iter()
        .map(|pin| (pin.path.as_str(), pin.name.as_str(), pin.version.as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            ("node_modules/@acme/lib", "@acme/lib", "1.0.0"),
            (
                "node_modules/@acme/lib/node_modules/util",
                "@acme/fork",
                "2.0.0"
            ),
            ("node_modules/app", "app", "1.0.0"),
            ("node_modules/util", "util", "1.0.0"),
        ]
    );
    for pin in &tree {
        let (.., bytes) = packages.iter().find(|(n, ..)| *n == pin.name).unwrap();
        let sha512 = STANDARD.encode(digest(&SHA512, bytes));
        assert_eq!(pin.integrity, format!("sha512-{sha512}"), "{pin:?}");
        let file = pin.name.rsplit('/').next().unwrap();
        let tarball = format!("{}/{}/-/{file}-{}.tgz", server.url(), pin.name, pin.version);
        assert_eq!(pin.tarball, tarball);
    }

    let fetched = dir.join("fetched");
    let pin = npm::fetch(&mut npm, "app", None, &fetched).await.unwrap();
    assert_eq!(
        pin,
        Pinned {
            path: String::new(),
            ..tree[2].clone()
        }
    );
    assert_eq!(files(&fetched), files(&root.join("node_modules/app")));
    server.finish().unwrap();

    // A mirror of the tarballs: one replay, whose downloads overlap, a
    // replay with a tampered download, then `fetch_pinned`.
    let mirror = Mirror::default();
    for (name, version, _, bytes) in &packages {
        let file = name.rsplit('/').next().unwrap();
        let path = format!("/{name}/-/{file}-{version}.tgz");
        mirror.files.lock().unwrap().insert(path, bytes.clone());
    }
    let mirror_url = mirror.serve();
    for pin in &mut tree {
        pin.tarball = pin.tarball.replace(&server.url(), &mirror_url);
    }
    // The registry is the http mirror, so its http tarballs pass, and a
    // packument request would break the mirror's cassette.
    let offline = Npm::new([("registry".to_owned(), format!("{mirror_url}/"))].into());
    let replayed = dir.join("replayed");
    let stale = replayed.join("node_modules/stale");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("package.json"), "{}").unwrap();
    let reversed: Vec<Pinned> = tree.iter().rev().cloned().collect();
    *mirror.gather.lock().unwrap() = true;
    npm::replay(&offline, &replayed, &reversed).await.unwrap();
    *mirror.gather.lock().unwrap() = false;
    let most = mirror.load.0.lock().unwrap().1;
    assert!((2..=8).contains(&most), "{most} downloads at once");
    let modules = files(&root.join("node_modules"));
    assert_eq!(modules.len(), 8);
    // The unlisted package is gone, and nothing is left beside the tree.
    assert_eq!(files(&replayed.join("node_modules")), modules);
    let entries = |dir: &Path| -> Vec<String> {
        let names = std::fs::read_dir(dir).unwrap();
        names
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    };
    assert_eq!(entries(&replayed), ["node_modules"]);

    // A download that fails partway leaves the root as it was.
    let fork = "/@acme/fork/-/fork-2.0.0.tgz".to_owned();
    let original = mirror
        .files
        .lock()
        .unwrap()
        .insert(fork.clone(), b"tampered".to_vec());
    let before = files(&replayed);
    let error = npm::replay(&offline, &replayed, &tree).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Integrity check failed for @acme/fork@2.0.0"
    );
    assert_eq!(files(&replayed), before);
    assert_eq!(entries(&replayed), ["node_modules"]);
    mirror.files.lock().unwrap().insert(fork, original.unwrap());

    // Refused before any request: the tree plus one more pin, last so that a
    // lazy check would download the others first.
    let with = |extra: Pinned| [tree.clone(), vec![extra]].concat();
    let app = |path: &str| Pinned {
        path: path.into(),
        ..tree[2].clone()
    };
    let integrity = |integrity: String| Pinned {
        integrity,
        ..app("node_modules/extra")
    };
    let secure = Npm::new([("registry".to_owned(), "https://registry.test/".to_owned())].into());
    // An http registry for the scope never lets plain http reach another
    // host that holds a token.
    let local = Npm::new(
        [
            (
                "@local:registry".to_owned(),
                "http://localhost:4873/".to_owned(),
            ),
            ("//npm.corp.test/:_authToken".to_owned(), "T".to_owned()),
        ]
        .into(),
    );
    let corp = Pinned {
        path: "node_modules/@local/x".into(),
        name: "@local/x".into(),
        tarball: "http://npm.corp.test/@local/x/-/x-1.0.0.tgz".into(),
        ..tree[2].clone()
    };
    let refused = [
        (
            &offline,
            with(app("node_modules/..")),
            "Invalid package path: node_modules/..".to_owned(),
        ),
        (
            &offline,
            with(app("../evil")),
            "Invalid package path: ../evil".into(),
        ),
        (
            &offline,
            with(app("/evil")),
            "Invalid package path: /evil".into(),
        ),
        (
            &offline,
            with(app("node_modules/\u{e9}")),
            "Invalid package path: node_modules/\u{e9}".into(),
        ),
        (
            &offline,
            // Its parent differs from the pinned `node_modules/app` by case alone.
            with(app("node_modules/APP/node_modules/app")),
            "Package path node_modules/APP/node_modules/app has no pinned parent node_modules/APP"
                .into(),
        ),
        (
            &offline,
            with(app("node_modules/APP")),
            "Package paths collide: node_modules/app and node_modules/APP".into(),
        ),
        (
            &offline,
            with(integrity(format!("sha1-{}", STANDARD.encode([0u8; 20])))),
            "Invalid integrity for app@1.0.0: expected one sha512 digest".into(),
        ),
        (
            &offline,
            with(integrity(format!("{0} {0}", tree[2].integrity))),
            "Invalid integrity for app@1.0.0: expected one sha512 digest".into(),
        ),
        (
            &offline,
            with(integrity(String::new())),
            "Invalid integrity for app@1.0.0: expected one sha512 digest".into(),
        ),
        (
            &secure,
            tree.clone(),
            format!(
                "Refusing to download @acme/lib@1.0.0 from {}: a pinned tarball must come from https://registry.test/, the registry for @acme/lib, over https unless that registry uses http",
                tree[0].tarball
            ),
        ),
        (
            &local,
            vec![corp],
            "Refusing to download @local/x@1.0.0 from http://npm.corp.test/@local/x/-/x-1.0.0.tgz: a pinned tarball must come from http://localhost:4873/, the registry for @local/x, over https unless that registry uses http".into(),
        ),
    ];
    for (npm, pins, message) in refused {
        let target = dir.join("refused");
        let error = npm::replay(npm, &target, &pins).await.unwrap_err();
        assert_eq!(error.to_string(), message);
        assert!(!target.exists());
    }
    #[cfg(unix)]
    {
        let elsewhere = dir.join("elsewhere");
        let linked = dir.join("linked");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::create_dir_all(&linked).unwrap();
        std::os::unix::fs::symlink(&elsewhere, linked.join("node_modules")).unwrap();
        let error = npm::replay(&offline, &linked, &tree).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "Refusing to install through the link {}",
                linked.join("node_modules").display()
            )
        );
        assert!(entries(&elsewhere).is_empty());
    }

    // `fetch_pinned` reinstalls what `fetch` pinned, dropping stray files.
    std::fs::write(fetched.join("stray.js"), "").unwrap();
    let pinned = Pinned {
        tarball: pin.tarball.replace(&server.url(), &mirror_url),
        ..pin
    };
    npm::fetch_pinned(&offline, &pinned, &fetched)
        .await
        .unwrap();
    assert_eq!(files(&fetched), files(&root.join("node_modules/app")));
}

/// Files served over HTTP by path, in any order and many at once, which
/// remembers the most requests it held at the same time. While `gather` is
/// set, each request waits up to 5 s for another before it is answered, so
/// that downloads made one at a time show as one.
#[derive(Clone, Default)]
struct Mirror {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    gather: Arc<Mutex<bool>>,
    /// The requests held now and the most held at once.
    load: Arc<(Mutex<(usize, usize)>, Condvar)>,
}

impl Mirror {
    /// Serves until the process exits; the base URL.
    fn serve(&self) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let mirror = self.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mirror = mirror.clone();
                std::thread::spawn(move || mirror.answer(stream.unwrap()));
            }
        });
        base
    }

    fn answer(&self, mut stream: std::net::TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        reader.read_line(&mut request).unwrap();
        let mut header = String::new();
        while reader.read_line(&mut header).unwrap() > 2 {
            header.clear();
        }
        let path = request.split(' ').nth(1).unwrap_or("/").replace("%2f", "/");
        let (lock, wake) = &*self.load;
        let mut load = lock.lock().unwrap();
        load.0 += 1;
        load.1 = load.1.max(load.0);
        wake.notify_all();
        if *self.gather.lock().unwrap() {
            // Only until two requests have overlapped once.
            let alone = |load: &mut (usize, usize)| load.1 < 2;
            load = wake
                .wait_timeout_while(load, Duration::from_secs(5), alone)
                .unwrap()
                .0;
        }
        load.0 -= 1;
        drop(load);
        let body = self.files.lock().unwrap().get(&path).cloned();
        let status = if body.is_some() {
            "200 OK"
        } else {
            "404 Not Found"
        };
        let body = body.unwrap_or_default();
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
    }
}

/// An `npm:` alias installs the package it names under the alias, and
/// packuments are asked for in npm's abbreviated form.
#[tokio::test(flavor = "multi_thread")]
async fn aliases_from_abbreviated_packuments() {
    let dir = scratch("alias");
    let app = tarball(&[("package.json", r#"{"name": "app-pkg"}"#)]);
    let fork = tarball(&[(
        "package.json",
        r#"{"name": "@forks/pty", "version": "0.13.2"}"#,
    )]);
    let server = registry(|base| {
        let mut all = publish(
            base,
            "app-pkg",
            &[(
                "1.0.0",
                json!({"dependencies": {"node-pty": "npm:@forks/pty@^0.13.1"}}),
                app.clone(),
            )],
            "1.0.0",
        );
        let mut forked = publish(
            base,
            "@forks/pty",
            &[
                ("0.13.2", json!({}), fork.clone()),
                ("1.0.0", json!({}), fork.clone()),
            ],
            "0.13.2",
        );
        forked[0].request.path = "/@forks%2fpty".into();
        all.extend(forked);
        all
    })
    .await;
    let mut packages = manager(&dir, &server.url());
    packages.install("npm:app-pkg", false).await.unwrap();

    let modules = dir.join("agent/npm/node_modules");
    let installed: Value = serde_json::from_str(
        &std::fs::read_to_string(modules.join("node-pty/package.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(installed["name"], "@forks/pty");
    assert_eq!(installed["version"], "0.13.2");
    assert!(!modules.join("@forks").exists());
    let requests = server.finish().unwrap();
    let packuments: Vec<_> = requests
        .iter()
        .filter(|request| !request.path.ends_with(".tgz"))
        .collect();
    assert_eq!(packuments.len(), 2);
    for request in packuments {
        assert!(
            request.headers["accept"].starts_with("application/vnd.npm.install-v1+json"),
            "{request:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn local_packages_scope_relative() {
    let dir = scratch("local");
    let mut packages = manager(&dir, "http://127.0.0.1:9");
    std::fs::create_dir_all(dir.join("project/tools")).unwrap();
    std::fs::write(
        dir.join("project/tools/index.ts"),
        "export default () => {};\n",
    )
    .unwrap();
    let previous = std::env::current_dir().unwrap();
    packages
        .install(&dir.join("project/tools").to_string_lossy(), false)
        .await
        .unwrap();
    assert_eq!(
        packages.settings().document(Scope::Global)["packages"],
        json!(["../project/tools"])
    );
    let error = packages.install("./missing", false).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "Path does not exist: {}",
            dir.join("project/missing").display()
        )
    );
    assert_eq!(
        packages.list()[0].extensions,
        [dir.join("project/tools/index.ts")]
    );
    assert_eq!(std::env::current_dir().unwrap(), previous);
}

/// Native extensions install like any package: a built `.wasm` file by path,
/// a folder with an `extensions` directory, or an npm package whose `yapi` key
/// names its `.wasm` build ahead of the `pi` key's JavaScript.
#[tokio::test(flavor = "multi_thread")]
async fn installs_native_extensions() {
    const WASM: &str = "\0asm\u{1}\0\0\0";
    let dir = scratch("native-extensions");
    let shout = tarball(&[
        (
            "package.json",
            r#"{"name": "shout", "pi": {"extensions": ["./dist/shout.js"]}, "yapi": {"extensions": ["./dist/shout.wasm"]}}"#,
        ),
        ("dist/shout.js", "export default function () {}\n"),
        ("dist/shout.wasm", WASM),
    ]);
    let server = registry(|base| {
        publish(
            base,
            "shout",
            &[("1.0.0", json!({}), shout.clone())],
            "1.0.0",
        )
    })
    .await;
    let mut packages = manager(&dir, &server.url());

    let file = dir.join("project/target/shout.wasm");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, WASM).unwrap();
    packages
        .install(&file.to_string_lossy(), false)
        .await
        .unwrap();

    let folder = dir.join("project/shout-package");
    std::fs::create_dir_all(folder.join("extensions")).unwrap();
    std::fs::write(folder.join("extensions/shout.wasm"), WASM).unwrap();
    packages
        .install(&folder.to_string_lossy(), false)
        .await
        .unwrap();

    packages.install("npm:shout", false).await.unwrap();
    let installed = dir.join("agent/npm/node_modules/shout/dist/shout.wasm");
    assert_eq!(std::fs::read(&installed).unwrap(), WASM.as_bytes());

    let extensions: Vec<PathBuf> = packages
        .list()
        .into_iter()
        .flat_map(|package| package.extensions)
        .collect();
    assert_eq!(
        extensions,
        [file, folder.join("extensions/shout.wasm"), installed]
    );
    server.finish().unwrap();
}

/// Credentials from npm's configuration go to the registry their key names:
/// a scoped package's packument and tarball carry them, and the public
/// registry on another host never sees them.
#[tokio::test(flavor = "multi_thread")]
async fn npmrc_credentials_per_registry() {
    let dir = scratch("npmrc");
    let private = tarball(&[("package.json", r#"{"name": "@acme/ext"}"#)]);
    let public = tarball(&[("package.json", r#"{"name": "left-pad"}"#)]);
    let scoped = registry(|base| {
        let mut all = publish(
            base,
            "@acme/ext",
            &[(
                "1.0.0",
                json!({"dependencies": {"left-pad": "^1.0.0"}}),
                private.clone(),
            )],
            "1.0.0",
        );
        all[0].request.path = "/@acme%2fext".into();
        all
    })
    .await;
    let npmjs = registry(|base| {
        publish(
            base,
            "left-pad",
            &[("1.0.0", json!({}), public.clone())],
            "1.0.0",
        )
    })
    .await;
    let host = scoped.url().trim_start_matches("http:").to_owned();
    let config = [
        ("registry".to_owned(), npmjs.url()),
        ("@acme:registry".to_owned(), scoped.url()),
        (format!("{host}/:_authToken"), "secret".to_owned()),
    ];
    let cwd = dir.join("project");
    std::fs::create_dir_all(&cwd).unwrap();
    let settings = SettingsManager::load(&dir.join("agent"), &cwd, false);
    let mut packages = PackageManager::new(cwd, dir.join("agent"), settings, config.into());
    packages.install("npm:@acme/ext", false).await.unwrap();

    let scoped_requests = scoped.finish().unwrap();
    assert_eq!(scoped_requests.len(), 2, "packument and tarball");
    for request in &scoped_requests {
        assert_eq!(
            request.headers.get("authorization").map(String::as_str),
            Some(yapi_mock::REDACTED),
            "{request:?}"
        );
    }
    let public_requests = npmjs.finish().unwrap();
    assert_eq!(public_requests.len(), 2, "packument and tarball");
    for request in &public_requests {
        assert!(
            !request.headers.contains_key("authorization"),
            "{request:?}"
        );
    }
}

/// `-e npm:` installs into pi's temporary folder for one run without
/// touching settings, reuses an install whose version is in range, and
/// offline installs nothing.
#[tokio::test(flavor = "multi_thread")]
async fn temporary_packages_outside_settings() {
    let dir = scratch("temporary");
    let package = tarball(&[
        (
            "package.json",
            r#"{"name": "temp-pkg", "version": "1.2.0"}"#,
        ),
        ("extensions/index.ts", "export default function () {}\n"),
    ]);
    let server = registry(|base| {
        publish(
            base,
            "temp-pkg",
            &[("1.2.0", json!({}), package.clone())],
            "1.2.0",
        )
    })
    .await;
    let mut packages = manager(&dir, &server.url());

    let offline = packages.install_temporary("npm:temp-pkg", true).await;
    assert_eq!(offline.unwrap(), None);
    let git = packages
        .install_temporary("git:github.com/user/repo", true)
        .await;
    assert_eq!(git.unwrap(), None);
    // A failed clone leaves no checkout and keeps the temporary folder.
    let failed = packages
        .install_temporary("git:http://127.0.0.1:9/user/repo", false)
        .await;
    assert!(failed.is_err());
    assert!(!dir.join("agent/tmp/extensions/git-127.0.0.1").exists());
    assert!(dir.join("agent/tmp/extensions").exists());

    // pi's `getTemporaryDir("npm")`: the hash is SHA-256 of `npm-`.
    let installed = dir.join("agent/tmp/extensions/npm/f35b2129/node_modules/temp-pkg");
    let path = packages.install_temporary("npm:temp-pkg", false).await;
    assert_eq!(path.unwrap().as_deref(), Some(installed.as_path()));
    assert!(installed.join("extensions/index.ts").exists());
    for (source, offline) in [
        ("npm:temp-pkg@^1.1", false),
        ("npm:temp-pkg@latest", false),
        ("npm:temp-pkg", true),
    ] {
        let path = packages.install_temporary(source, offline).await;
        assert_eq!(
            path.unwrap().as_deref(),
            Some(installed.as_path()),
            "{source}"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(dir.join("agent/tmp/extensions"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700);
    }
    assert!(packages.list().is_empty());
    assert!(!dir.join("agent/settings.json").exists());
    assert_eq!(
        server.finish().unwrap().len(),
        2,
        "one packument and one tarball"
    );
}

/// pi's startup install: a missing configured package is installed, one
/// whose installed version is outside the configured range is replaced, and
/// one in range is left alone (the registry serves one install only).
#[tokio::test(flavor = "multi_thread")]
async fn installs_missing_or_outdated() {
    let dir = scratch("missing");
    let version = |version: &str| {
        tarball(&[
            (
                "package.json",
                &format!(r#"{{"name": "ext-pkg", "version": "{version}"}}"#),
            ),
            ("index.ts", version),
        ])
    };
    let server = registry(|base| {
        publish(
            base,
            "ext-pkg",
            &[("2.0.0", json!({}), version("2.0.0"))],
            "2.0.0",
        )
    })
    .await;
    let installed = dir.join("agent/npm/node_modules/ext-pkg");
    let configure = |source: &str| {
        std::fs::create_dir_all(dir.join("agent")).unwrap();
        std::fs::write(
            dir.join("agent/settings.json"),
            json!({"packages": [source]}).to_string(),
        )
        .unwrap();
    };

    // Missing: installed from the registry.
    configure("npm:ext-pkg@^2.0.0");
    let outcome = manager(&dir, &server.url())
        .install_missing(|_, message| ControlFlow::Break(message))
        .await;
    assert_eq!(outcome, ControlFlow::Continue(()));
    assert_eq!(
        std::fs::read_to_string(installed.join("index.ts")).unwrap(),
        "2.0.0"
    );

    // In range, a different range that 2.0.0 satisfies, and a dist-tag: no
    // request reaches the registry.
    for source in [
        "npm:ext-pkg@^2.0.0",
        "npm:ext-pkg@>=1",
        "npm:ext-pkg@latest",
        "npm:ext-pkg",
    ] {
        configure(source);
        let outcome = manager(&dir, &server.url())
            .install_missing(|_, message| ControlFlow::Break(message))
            .await;
        assert_eq!(outcome, ControlFlow::Continue(()));
    }

    // Out of range: replaced by the 1.x install put in its place.
    std::fs::write(
        installed.join("package.json"),
        r#"{"name": "ext-pkg", "version": "1.0.0"}"#,
    )
    .unwrap();
    std::fs::write(installed.join("index.ts"), "1.0.0").unwrap();
    assert_eq!(
        server.finish().unwrap().len(),
        2,
        "one packument and one tarball"
    );
    let server = registry(|base| {
        publish(
            base,
            "ext-pkg",
            &[("2.0.0", json!({}), version("2.0.0"))],
            "2.0.0",
        )
    })
    .await;
    configure("npm:ext-pkg@^2.0.0");
    let outcome = manager(&dir, &server.url())
        .install_missing(|_, message| ControlFlow::Break(message))
        .await;
    assert_eq!(outcome, ControlFlow::Continue(()));
    assert_eq!(
        std::fs::read_to_string(installed.join("index.ts")).unwrap(),
        "2.0.0"
    );
    assert_eq!(server.finish().unwrap().len(), 2);

    // An install that never finished, its dependencies missing, is redone.
    std::fs::write(installed.join(".yapi-incomplete"), "").unwrap();
    std::fs::write(installed.join("index.ts"), "partial").unwrap();
    let server = registry(|base| {
        publish(
            base,
            "ext-pkg",
            &[("2.0.0", json!({}), version("2.0.0"))],
            "2.0.0",
        )
    })
    .await;
    let outcome = manager(&dir, &server.url())
        .install_missing(|_, message| ControlFlow::Break(message))
        .await;
    assert_eq!(outcome, ControlFlow::Continue(()));
    assert_eq!(
        std::fs::read_to_string(installed.join("index.ts")).unwrap(),
        "2.0.0"
    );
    assert!(!installed.join(".yapi-incomplete").exists());
    assert_eq!(server.finish().unwrap().len(), 2);

    // One that another process installs while this one waits for the npm
    // root's lock is kept, and no request reaches the registry.
    std::fs::write(installed.join(".yapi-incomplete"), "").unwrap();
    let root = dir.join("agent/npm/package.json");
    let lock = FileLock::acquire(&root, &Default::default()).await.unwrap();
    let mut packages = manager(&dir, "http://127.0.0.1:9");
    let other = async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::fs::remove_file(installed.join(".yapi-incomplete")).unwrap();
        drop(lock);
    };
    let install = packages.install_missing(|_, message| ControlFlow::Break(message));
    let (outcome, ()) = tokio::join!(install, other);
    assert_eq!(outcome, ControlFlow::Continue(()));
}

/// pi's `findAutoloadDeltaBase`: a project entry with `autoload: false` over
/// the user's entry for the same package installs the user's copy, which
/// both entries resolve to, and no project copy.
#[tokio::test(flavor = "multi_thread")]
async fn autoload_delta_user_package() {
    let dir = scratch("delta");
    let package = tarball(&[("package.json", r#"{"name": "ext-pkg", "version": "2.0.0"}"#)]);
    let server = registry(|base| {
        publish(
            base,
            "ext-pkg",
            &[("2.0.0", json!({}), package.clone())],
            "2.0.0",
        )
    })
    .await;
    std::fs::create_dir_all(dir.join("agent")).unwrap();
    std::fs::create_dir_all(dir.join("project/.yapi")).unwrap();
    std::fs::write(
        dir.join("agent/settings.json"),
        json!({"packages": ["npm:ext-pkg@^2"]}).to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("project/.yapi/settings.json"),
        json!({"packages": [{"source": "npm:ext-pkg@^2", "autoload": false}]}).to_string(),
    )
    .unwrap();
    let outcome = manager(&dir, &server.url())
        .install_missing(|_, message| ControlFlow::Break(message))
        .await;
    assert_eq!(outcome, ControlFlow::Continue(()));
    assert!(dir.join("agent/npm/node_modules/ext-pkg").exists());
    assert!(!dir.join("project/.yapi/npm").exists());
    assert_eq!(server.finish().unwrap().len(), 2);
}

/// A dependency named by a path, absolute or climbing out of `node_modules`,
/// fails the install before anything is written there, even when the
/// registry serves a package for that name.
#[tokio::test(flavor = "multi_thread")]
async fn path_dependency_names_refused() {
    let dir = scratch("dependency-paths");
    let outside = dir.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("keep.txt"), "mine").unwrap();
    let package = tarball(&[("package.json", r#"{"name": "evil-pkg"}"#)]);
    let payload = tarball(&[("package.json", r#"{"name": "evil", "version": "1.0.0"}"#)]);
    for name in [outside.to_str().unwrap(), "../../../outside"] {
        let server = registry(|base| {
            let manifest = json!({"dependencies": {name: "1.0.0"}});
            let versions = [("1.0.0", manifest, package.clone())];
            let mut all = publish(base, "evil-pkg", &versions, "1.0.0");
            let mut evil = publish(
                base,
                "evil",
                &[("1.0.0", json!({}), payload.clone())],
                "1.0.0",
            );
            evil[0].request.path = format!("/{}", name.replace('/', "%2f"));
            all.extend(evil);
            all
        })
        .await;
        let mut packages = manager(&dir, &server.url());
        let error = packages.install("npm:evil-pkg", false).await.unwrap_err();
        assert_eq!(error.to_string(), format!("Invalid package name: {name}"));
        assert_eq!(
            std::fs::read_to_string(outside.join("keep.txt")).unwrap(),
            "mine"
        );
        assert!(!dir.join("agent/npm/node_modules/evil-pkg").exists());
    }
}

/// Removing a package another installed package depends on keeps it, as
/// npm does, until nothing needs it.
#[tokio::test(flavor = "multi_thread")]
async fn remove_keeps_shared_dependency() {
    let dir = scratch("shared");
    configure(&dir, json!(["npm:pi-a", "npm:pi-shared"]));
    let a = installed(&dir, "pi-a", "1.0.0");
    let manifest =
        json!({"name": "pi-a", "version": "1.0.0", "dependencies": {"pi-shared": "^1.0.0"}});
    std::fs::write(a.join("package.json"), manifest.to_string()).unwrap();
    let shared = installed(&dir, "pi-shared", "1.0.0");
    let root = json!({"dependencies": {"pi-a": "^1.0.0", "pi-shared": "^1.0.0"}});
    std::fs::write(dir.join("agent/npm/package.json"), root.to_string()).unwrap();
    let mut packages = manager(&dir, "http://127.0.0.1:9");

    // Removal waits for another process's hold on the npm root.
    let root = dir.join("agent/npm/package.json");
    let lock = FileLock::acquire(&root, &Default::default()).await.unwrap();
    let remove = packages.remove("npm:pi-shared", false);
    let waited = tokio::time::timeout(std::time::Duration::from_millis(100), remove).await;
    assert!(waited.is_err(), "{waited:?}");
    drop(lock);
    assert!(packages.remove("npm:pi-shared", false).await.unwrap());
    assert!(shared.join("package.json").exists());
    assert!(packages.remove("npm:pi-a", false).await.unwrap());
    assert!(!a.exists());
    assert!(!shared.exists());

    // Dependencies resolve through every `node_modules` up from their
    // dependent, as in trees npm makes: b reaches a's c@2, whose d is hoisted.
    let modules = dir.join("agent/npm/node_modules");
    for (path, version, dependencies) in [
        ("a", "1.0.0", json!({"b": "^1"})),
        ("a/node_modules/b", "1.0.0", json!({"c": "^2"})),
        ("a/node_modules/c", "2.0.0", json!({"d": "^1"})),
        ("c", "1.0.0", json!({})),
        ("d", "1.0.0", json!({})),
        ("x", "1.0.0", json!({})),
    ] {
        let name = path.rsplit('/').next().unwrap();
        let manifest = json!({"name": name, "version": version, "dependencies": dependencies});
        std::fs::create_dir_all(modules.join(path)).unwrap();
        std::fs::write(
            modules.join(path).join("package.json"),
            manifest.to_string(),
        )
        .unwrap();
    }
    configure(&dir, json!(["npm:a", "npm:c", "npm:x"]));
    let root = json!({"dependencies": {"a": "^1", "c": "^1", "x": "^1"}});
    std::fs::write(dir.join("agent/npm/package.json"), root.to_string()).unwrap();
    let mut packages = manager(&dir, "http://127.0.0.1:9");
    assert!(packages.remove("npm:x", false).await.unwrap());
    assert!(!modules.join("x").exists());
    assert!(modules.join("d").exists());
}

/// An update whose new version's dependency fails leaves the working
/// version in place, still out of range, so the next startup tries again.
#[tokio::test(flavor = "multi_thread")]
async fn failed_update_keeps_version() {
    let dir = scratch("failed-update");
    configure(&dir, json!(["npm:ext-pkg@^2.0.0"]));
    let old = installed(&dir, "ext-pkg", "1.0.0");
    std::fs::write(old.join("index.ts"), "1.0.0").unwrap();
    let new = tarball(&[
        ("package.json", r#"{"name": "ext-pkg", "version": "2.0.0"}"#),
        ("index.ts", "2.0.0"),
    ]);
    let server = registry(|base| {
        let manifest = json!({"dependencies": {"missing-dep": "^1.0.0"}});
        let mut all = publish(
            base,
            "ext-pkg",
            &[("2.0.0", manifest, new.clone())],
            "2.0.0",
        );
        let mut missing = json_response(&json!({"error": "Not found"}));
        missing.status = 404;
        all.push(get("/missing-dep", missing));
        all
    })
    .await;
    let outcome = manager(&dir, &server.url())
        .install_missing(|_, message| ControlFlow::Break(message))
        .await;
    let ControlFlow::Break(message) = outcome else {
        panic!("the update succeeded");
    };
    assert!(
        message.starts_with("Failed to install npm:ext-pkg@^2.0.0: "),
        "{message}"
    );
    assert_eq!(
        std::fs::read_to_string(old.join("index.ts")).unwrap(),
        "1.0.0"
    );
    let modules = std::fs::read_dir(dir.join("agent/npm/node_modules")).unwrap();
    assert_eq!(modules.count(), 1, "no copy is left beside it");
    server.finish().unwrap();
}
