//! Packages against a mock npm registry: installs with hoisted dependencies,
//! settings entries, resources, removal, native addons and integrity.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::io::Write as _;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use indexmap::IndexMap;
use serde_json::{Value, json};
use sha2::Digest as _;
use yapi_core::packages::PackageManager;
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
    for (version, manifest, bytes) in versions {
        let mut manifest = manifest.clone();
        manifest["name"] = json!(name);
        manifest["version"] = json!(version);
        manifest["dist"] = json!({
            "tarball": format!("{base}/{name}/-/{name}-{version}.tgz"),
            "integrity": format!("sha512-{}", STANDARD.encode(sha2::Sha512::digest(bytes))),
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
            &format!("/{name}/-/{name}-{served}.tgz"),
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
async fn installs_npm_packages_with_dependencies() {
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
async fn finds_and_applies_npm_updates_in_range() {
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
async fn checks_npm_updates_through_npm_command() {
    let dir = scratch("npm-command-updates");
    let script = dir.join("npm.sh");
    std::fs::write(
        &script,
        "[ \"$1 $3 $4\" = 'view version --json' ] && echo '[\"1.0.0\", \"1.5.0\", \"2.0.0\"]'\n",
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
    let packages = manager(&dir, "http://127.0.0.1:9");
    assert_eq!(packages.available_updates().await, ["behind"]);
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
async fn finds_git_updates_on_the_remote() {
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
}

#[tokio::test(flavor = "multi_thread")]
async fn installs_native_addons_unbuilt_and_rejects_bad_integrity() {
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

/// An `npm:` alias installs the package it names under the alias, and
/// packuments are asked for in npm's abbreviated form.
#[tokio::test(flavor = "multi_thread")]
async fn installs_aliased_dependencies_from_abbreviated_packuments() {
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
async fn local_packages_are_stored_relative_to_the_scope() {
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
async fn sends_npmrc_credentials_to_their_registry_only() {
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
async fn installs_temporary_packages_outside_settings() {
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
async fn installs_missing_and_out_of_range_configured_packages() {
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
}

/// pi's `findAutoloadDeltaBase`: a project entry with `autoload: false` over
/// the user's entry for the same package installs the user's copy, which
/// both entries resolve to, and no project copy.
#[tokio::test(flavor = "multi_thread")]
async fn installs_an_autoload_delta_as_the_users_package() {
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
