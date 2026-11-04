//! `install.sh` against releases packaged by `scripts/package-release.sh`,
//! served the way GitHub serves them. The installed "binary" is a script that
//! prints its version, and the docs archive holds `.version` and `index.md`.
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use sha2::Digest as _;

const TARGETS: [&str; 4] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

fn scratch(name: &str) -> PathBuf {
    common::scratch(&format!("install-{name}"))
}

/// [`release_with`] a `chest` that prints `chest <version>`.
fn release(site: &Path, version: &str) {
    release_with(site, version, Some(&format!("echo chest {version}")));
}

/// Packages a fake `yapi` that prints `version` for every target, with a
/// fake `chest` that runs the shell commands `chest`, or none as releases
/// before chest had, fake third-party notices, and a docs archive naming
/// `version`, under `<site>/releases/download/v<version>/`.
fn release_with(site: &Path, version: &str, chest: Option<&str>) {
    std::fs::create_dir_all(site).unwrap();
    let binary = site.join(format!("yapi-{version}"));
    std::fs::write(&binary, format!("#!/bin/sh\necho {version}\n")).unwrap();
    let chest_binary = site.join(format!("chest-{version}"));
    if let Some(chest) = chest {
        std::fs::write(&chest_binary, format!("#!/bin/sh\n{chest}\n")).unwrap();
    }
    let notices = site.join("THIRD-PARTY-NOTICES");
    std::fs::write(&notices, "notices\n").unwrap();
    let out = site.join(format!("releases/download/v{version}"));
    for target in TARGETS {
        let output = Command::new("sh")
            .arg(common::repo().join("scripts/package-release.sh"))
            .arg(&binary)
            .arg(target)
            .arg(&out)
            .arg(&notices)
            .args(chest.map(|_| &chest_binary))
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    let docs = site.join(format!("docs-{version}"));
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join(".version"), format!("{version}\n")).unwrap();
    std::fs::write(docs.join("index.md"), format!("# yapi {version}\n")).unwrap();
    let archive = out.join("yapi-docs.tar.gz");
    let output = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&docs)
        .args([".version", "index.md"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let digest = sha2::Sha256::digest(std::fs::read(&archive).unwrap());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::write(
        out.join("yapi-docs.tar.gz.sha256"),
        format!("{hex}  yapi-docs.tar.gz\n"),
    )
    .unwrap();
}

/// Serves `site` over HTTP until the process exits, redirecting the latest
/// release's downloads to release `latest` as GitHub does, or answering them
/// with 404 when there is no release.
fn serve(site: PathBuf, latest: Option<&'static str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let host = base.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request).unwrap();
            let path = request.split(' ').nth(1).unwrap_or("/").to_owned();
            let latest_file = path.strip_prefix("/releases/latest/download/");
            let (status, headers, body) = match (latest_file, latest) {
                (Some(file), Some(tag)) => (
                    "302 Found",
                    format!("Location: {host}/releases/download/v{tag}/{file}\r\n"),
                    Vec::new(),
                ),
                _ => match std::fs::read(site.join(path.trim_start_matches('/'))) {
                    Ok(body) => ("200 OK", String::new(), body),
                    Err(_) => ("404 Not Found", String::new(), Vec::new()),
                },
            };
            let head = format!(
                "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("{base}/releases")
}

/// `install.sh` with `args` against `releases`, with `home` as its home and
/// none of its own variables set.
fn script(releases: &str, home: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("sh");
    command
        .arg(common::repo().join("install.sh"))
        .args(args)
        .env("YAPI_RELEASES_URL", releases)
        .env("HOME", home)
        .env_remove("YAPI_VERSION")
        .env_remove("YAPI_INSTALL_DIR")
        .env_remove("YAPI_INSTALL_ONLY")
        .env_remove("YAPI_NO_DOCS")
        .env_remove("YAPI_CODING_AGENT_DIR");
    command
}

fn install(releases: &str, home: &Path, args: &[&str]) -> Output {
    script(releases, home, args).output().unwrap()
}

/// The names in `dir`, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// The `index.md` of the docs installed in `agent`, if any.
fn docs_index(agent: &Path) -> Option<String> {
    std::fs::read_to_string(agent.join("docs/index.md")).ok()
}

fn installed_version(binary: &Path) -> String {
    let output = Command::new(binary).output().unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[test]
fn installs_latest_release() {
    let dir = scratch("latest");
    let site = dir.join("site");
    release(&site, "9.9.8");
    release(&site, "9.9.9");
    // A trailing slash on the releases page is harmless.
    let releases = format!("{}/", serve(site, Some("9.9.9")));
    let output = install(&releases, &dir, &[]);
    assert!(output.status.success(), "{output:?}");
    let binary = dir.join(".local/bin/yapi");
    assert_eq!(installed_version(&binary), "9.9.9");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("Installed yapi 9.9.9 to {}", binary.display())));
    // The release's docs for the model, in the default agent directory.
    let agent = dir.join(".yapi/agent");
    assert_eq!(docs_index(&agent).as_deref(), Some("# yapi 9.9.9\n"));
    assert!(stdout.contains(&format!(
        "Installed the docs for the model to {}",
        agent.join("docs").display()
    )));
}

#[test]
fn replaces_docs_unless_skipped() {
    let dir = scratch("docs");
    let site = dir.join("site");
    release(&site, "9.9.9");
    let releases = serve(site, Some("9.9.9"));
    let agent = dir.join("agent");
    std::fs::create_dir_all(agent.join("docs")).unwrap();
    std::fs::write(agent.join("docs/stale.md"), "old").unwrap();
    let with_agent = |args: &[&str], no_docs: Option<&str>| {
        let mut command = script(&releases, &dir, args);
        command.env("YAPI_CODING_AGENT_DIR", &agent);
        if let Some(value) = no_docs {
            command.env("YAPI_NO_DOCS", value);
        }
        command.output().unwrap()
    };
    let output = with_agent(&[], None);
    assert!(output.status.success(), "{output:?}");
    // The new copy replaces the old one whole, and nothing is left beside it.
    assert_eq!(docs_index(&agent).as_deref(), Some("# yapi 9.9.9\n"));
    assert!(!agent.join("docs/stale.md").exists());
    let names: Vec<_> = std::fs::read_dir(&agent)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["docs"]);

    // `--no-docs` and `YAPI_NO_DOCS` keep the docs as they are.
    std::fs::write(agent.join("docs/index.md"), "edited").unwrap();
    for (args, no_docs) in [(&["--no-docs"][..], None), (&[][..], Some("1"))] {
        let output = with_agent(args, no_docs);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(docs_index(&agent).as_deref(), Some("edited"));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(!stdout.contains("Installed the docs"), "{stdout}");
    }
}

#[test]
fn skips_docs_failing_checksum() {
    let dir = scratch("docs-checksum");
    let site = dir.join("site");
    release(&site, "9.9.9");
    let sums = site.join("releases/download/v9.9.9/yapi-docs.tar.gz.sha256");
    let text = std::fs::read_to_string(&sums).unwrap();
    std::fs::write(&sums, format!("{}{}", "0".repeat(64), &text[64..])).unwrap();
    let output = install(&serve(site, Some("9.9.9")), &dir, &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(installed_version(&dir.join(".local/bin/yapi")), "9.9.9");
    assert_eq!(docs_index(&dir.join(".yapi/agent")), None);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("skipped the docs for the model: yapi-docs.tar.gz does not match its SHA-256 checksum. yapi downloads them when it first runs."),
        "{stderr}"
    );
}

#[test]
fn installs_chosen_version() {
    let dir = scratch("pinned");
    let site = dir.join("site");
    // An archive from before chest shipped installs yapi alone.
    release_with(&site, "9.9.8", None);
    release(&site, "9.9.9");
    let releases = serve(site, Some("9.9.9"));
    let to = dir.join("bin");
    let output = install(
        &releases,
        &dir,
        &["--version", "9.9.8", "--to", to.to_str().unwrap()],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(installed_version(&to.join("yapi")), "9.9.8");
    assert_eq!(std::fs::read_dir(&to).unwrap().count(), 1, "only yapi");

    // Such an archive can't install chest alone, and the script says why.
    let other = dir.join("chest-only");
    let to_other = other.to_str().unwrap();
    let args = ["--version", "9.9.8", "--only", "chest", "--to", to_other];
    let output = install(&releases, &dir, &args);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.ends_with(" holds no chest, because releases before chest shipped hold yapi alone. Build chest from source: cargo install --locked --git https://github.com/SkymanOne/yapi chest-cli\n"),
        "{stderr}"
    );
    assert!(!other.exists());

    // The environment variables do the same, and a newer install replaces it.
    let output = script(&releases, &dir, &[])
        .env("YAPI_VERSION", "v9.9.9")
        .env("YAPI_INSTALL_DIR", &to)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(installed_version(&to.join("yapi")), "9.9.9");
    assert_eq!(installed_version(&to.join("chest")), "chest 9.9.9");
    assert_eq!(std::fs::read_dir(&to).unwrap().count(), 2);
}

/// Without a terminal or a choice, the install script installs chest next
/// to yapi, and installs neither when the archive's chest does not run.
/// `--only` and `YAPI_INSTALL_ONLY` install one of them.
#[test]
fn installs_chest_too() {
    let dir = scratch("chest");
    let site = dir.join("site");
    release(&site, "9.9.9");
    release_with(&site, "9.9.8", Some("exit 1"));
    let releases = serve(site, Some("9.9.9"));
    let output = install(&releases, &dir, &[]);
    assert!(output.status.success(), "{output:?}");
    let chest = dir.join(".local/bin/chest");
    assert_eq!(installed_version(&chest), "chest 9.9.9");
    assert_eq!(listing(&dir.join(".local/bin")), ["chest", "yapi"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&format!("Installed chest 9.9.9 to {}", chest.display())),
        "{stdout}"
    );
    assert!(
        stdout.contains("to PATH to run yapi and chest, for"),
        "{stdout}"
    );

    // chest alone skips yapi's docs, and yapi alone never runs the
    // archive's chest, which here fails.
    for (case, (args, only, binary, version)) in [
        (&["--only", "chest"][..], None, "chest", "9.9.9"),
        (&[][..], Some("chest"), "chest", "9.9.9"),
        (
            &["--version", "9.9.8", "--only", "yapi"],
            None,
            "yapi",
            "9.9.8",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let home = dir.join(format!("only-{case}"));
        let mut command = script(&releases, &home, args);
        if let Some(only) = only {
            command.env("YAPI_INSTALL_ONLY", only);
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let bin = home.join(".local/bin");
        assert_eq!(listing(&bin), [binary], "{case}");
        // chest prints its name before its version.
        let printed = installed_version(&bin.join(binary));
        assert_eq!(printed.trim_start_matches("chest "), version, "{case}");
        let docs = docs_index(&home.join(".yapi/agent"));
        assert_eq!(docs.is_some(), binary == "yapi", "{case}");
        // The messages name only what was installed.
        let stdout = String::from_utf8(output.stdout).unwrap();
        let path = bin.join(binary);
        let installed = format!("Installed {binary} {version} to {}\n", path.display());
        assert!(stdout.contains(&installed), "{stdout}");
        assert!(
            stdout.contains(&format!("to PATH to run {binary}, for")),
            "{stdout}"
        );
    }
    let output = install(&releases, &dir, &["--only", "all"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("--only and YAPI_INSTALL_ONLY take yapi, chest or both, not 'all'"),
        "{stderr}"
    );

    let to = dir.join("broken");
    let output = install(
        &releases,
        &dir,
        &["--version", "9.9.8", "--to", to.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("the downloaded chest does not run on this system"),
        "{stderr}"
    );
    assert!(!to.exists());
}

/// In a terminal, the install script asks what to install. It reads the
/// answer from the terminal, as `curl … | sh` feeds it the script on stdin,
/// and asks again after an invalid answer, three times at most. With
/// `--only`, or with its output going elsewhere, it doesn't ask.
#[test]
fn asks_in_terminal() {
    let dir = scratch("ask");
    let site = dir.join("site");
    release(&site, "9.9.9");
    let releases = serve(site, Some("9.9.9"));
    let script = common::repo().join("install.sh");
    let limit = Duration::from_secs(30);
    let prompts = |rows: &[String]| {
        let prompt = "Choose 1, 2 or 3 [1]: ";
        rows.iter().filter(|row| row.starts_with(prompt)).count()
    };
    let exit = |rows: &[String]| {
        rows.iter()
            .find_map(|row| row.trim_end().strip_prefix("exit ").map(str::to_owned))
    };
    for (case, rest, answers, installed) in [
        ("enter", "", &[""][..], &["chest", "yapi"][..]),
        ("chest", "", &["4", "3"], &["chest"]),
        ("invalid", "", &["x", "x", "x"], &[]),
        ("flag", " -s -- --only both", &[], &["chest", "yapi"]),
        ("redirected", " >out", &[], &["chest", "yapi"]),
    ] {
        let home = dir.join(case);
        std::fs::create_dir_all(&home).unwrap();
        let env = [
            ("PATH", std::env::var_os("PATH").unwrap()),
            ("HOME", home.clone().into_os_string()),
            ("YAPI_RELEASES_URL", releases.clone().into()),
        ];
        // sh reads the script from a pipe, as from curl.
        let args = [
            "-c".to_owned(),
            format!("cat \"$0\" | sh{rest}; echo \"exit $?\""),
            script.display().to_string(),
        ];
        let shell = Path::new("/bin/sh");
        let mut pty =
            yapi_mock::pty::Pty::spawn(shell, &args, &home, &env, (120, 30), false).unwrap();
        for (asked, answer) in answers.iter().enumerate() {
            let shown = pty.wait_for(limit, |rows| prompts(rows) > asked);
            assert!(shown.is_some(), "{case}: {:#?}", pty.rows());
            pty.write(&format!("{answer}\r")).unwrap();
        }
        let done = pty.wait_for(limit, |rows| exit(rows).is_some());
        assert!(done.is_some(), "{case}: {:#?}", pty.rows());
        let rows = pty.rows();
        assert_eq!(prompts(&rows), answers.len(), "{case}: {rows:#?}");
        let bin = home.join(".local/bin");
        if installed.is_empty() {
            assert_eq!(exit(&rows).as_deref(), Some("1"), "{rows:#?}");
            let failed = rows.iter().any(|row| row.contains("no valid choice"));
            assert!(failed, "{rows:#?}");
            assert!(!bin.exists());
        } else {
            assert_eq!(exit(&rows).as_deref(), Some("0"), "{case}: {rows:#?}");
            assert_eq!(listing(&bin), installed, "{case}");
        }
        let _ = pty.finish();
    }
}

#[test]
fn refuses_bad_checksum() {
    let dir = scratch("checksum");
    let site = dir.join("site");
    release(&site, "9.9.9");
    let download = site.join("releases/download/v9.9.9");
    for entry in std::fs::read_dir(&download).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "sha256")
        {
            let text = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, format!("{}{}", "0".repeat(64), &text[64..])).unwrap();
        }
    }
    let output = install(&serve(site, Some("9.9.9")), &dir, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("does not match its SHA-256 checksum"),
        "{stderr}"
    );
    assert!(!dir.join(".local/bin/yapi").exists());
}

#[test]
fn suggests_source_build() {
    let dir = scratch("none");
    let releases = serve(dir.join("site"), None);
    // The build names the crates of the binaries chosen.
    for (args, crates) in [
        (&[][..], "yapi chest-cli"),
        (&["--only", "yapi"], "yapi"),
        (&["--only", "chest"], "chest-cli"),
    ] {
        let output = install(&releases, &dir, args);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("in the latest release at"), "{stderr}");
        let build = "cargo install --locked --git https://github.com/SkymanOne/yapi";
        assert!(stderr.ends_with(&format!("{build} {crates}\n")), "{stderr}");
    }
}

/// Each archive holds the binaries and their license files, and extracting
/// `yapi` and `chest` from it into a directory on PATH installs only the
/// binaries, as the install guide shows.
#[test]
fn archives_hold_binary_licenses() {
    let dir = scratch("archive");
    let site = dir.join("site");
    release(&site, "9.9.9");
    let download = site.join("releases/download/v9.9.9");
    for target in TARGETS {
        let listing = Command::new("tar")
            .arg("-tzf")
            .arg(download.join(format!("yapi-{target}.tar.gz")))
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(listing.stdout).unwrap(),
            "yapi\nchest\nLICENSE-MIT\nLICENSE-APACHE\nTHIRD-PARTY-NOTICES\n",
            "{target}"
        );
    }
    let releases = serve(site, Some("9.9.9"));
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "curl -fsSL {releases}/latest/download/yapi-{}.tar.gz | tar xzf - -C {} yapi chest",
            TARGETS[0],
            bin.display()
        ))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(installed_version(&bin.join("yapi")), "9.9.9");
    assert_eq!(installed_version(&bin.join("chest")), "chest 9.9.9");
    assert_eq!(
        std::fs::read_dir(&bin).unwrap().count(),
        2,
        "only the binaries"
    );
}
