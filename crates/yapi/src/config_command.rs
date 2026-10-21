//! `yapi config`: the resource configuration selector, globally or for the
//! project (`handleConfigCommand` in `package-manager-cli.ts` in pi `v1.0.0`).

use std::collections::HashMap;
use std::io::Write as _;
use std::ops::ControlFlow;
use std::path::Path;

use yapi_core::config::{PROJECT_DIR, agent_dir};
use yapi_core::packages::{PackageManager, resolve_resources};
use yapi_core::settings::SettingsManager;

use crate::interactive::config_selector::ConfigSelector;
use crate::interactive::picker;

const USAGE: &str = "yapi config [-l] [--approve|--no-approve]";

fn help() -> String {
    format!(
        "Usage:\n  {USAGE}\n\nOpen the resource configuration TUI to enable or disable package resources.\nWithout -l, starts in global settings (~/{PROJECT_DIR}/agent/settings.json).\nPress Tab in the TUI to switch between global and project-local modes.\n\nOptions:\n  -l, --local       Edit project overrides ({PROJECT_DIR}/settings.json)\n  -a, --approve     Trust project-local files for this command with -l\n  -na, --no-approve Ignore project-local files for this command with -l\n"
    )
}

/// Runs `yapi config` with the arguments after `config`; its exit code.
pub async fn run(args: &[String]) -> u8 {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        // pi's console.log adds a newline after the text's own.
        let _ = writeln!(std::io::stdout(), "{}", help());
        return 0;
    }
    let mut local = false;
    let mut trust = None;
    for arg in args {
        match arg.as_str() {
            "-l" | "--local" => local = true,
            "-a" | "--approve" => trust = Some(true),
            "-na" | "--no-approve" => trust = Some(false),
            option if option.starts_with('-') => {
                eprintln!("Unknown option {option} for \"config\".");
                eprintln!("Use \"yapi --help\" or \"{USAGE}\".");
                return 1;
            }
            argument => {
                eprintln!("Unexpected argument {argument}.");
                eprintln!("Usage: {USAGE}");
                return 1;
            }
        }
    }
    match configure(local, trust).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

async fn configure(local: bool, trust: Option<bool>) -> anyhow::Result<u8> {
    let cwd = std::env::current_dir()?;
    let agent_dir = agent_dir();
    let (settings, trusted) = crate::startup::load_settings(&cwd, &agent_dir, trust, true)?;
    if local && !trusted {
        eprintln!("Project is not trusted. Use --approve to modify local resource config.");
        return Ok(1);
    }
    for error in settings.errors() {
        eprintln!("Warning: {error}");
    }
    // Global mode shows what the user's settings alone resolve to.
    let global_settings = SettingsManager::load(&agent_dir, &cwd, false);
    if !yapi_core::tools::external::offline() {
        let npm_config = yapi_core::packages::npm::config();
        install_packages(&cwd, &agent_dir, &global_settings, &settings, npm_config)
            .await
            .map_err(anyhow::Error::msg)?;
    }
    let builtins = crate::startup::BUILTINS;
    let global = resolve_resources(&cwd, &agent_dir, &global_settings, &builtins);
    let project = if trusted {
        resolve_resources(&cwd, &agent_dir, &settings, &builtins)
    } else {
        global.clone()
    };
    let theme = settings.settings().theme.clone();
    let (_, rows) = yapi_tui::terminal::size();
    let selector = ConfigSelector::new(
        &global,
        &project,
        settings,
        cwd,
        agent_dir.clone(),
        rows,
        local,
        trusted,
    );
    picker::configure(&agent_dir, theme.as_deref(), selector)?;
    Ok(0)
}

/// pi's resolves before the selector opens, which install the configured
/// packages that are missing or outside their range: with the user's
/// settings alone, then, in a trusted project, with `settings`, so a package
/// in both scopes gets both copies. The first failure stops them and is the
/// error.
async fn install_packages(
    cwd: &Path,
    agent_dir: &Path,
    global: &SettingsManager,
    settings: &SettingsManager,
    npm_config: HashMap<String, String>,
) -> Result<(), String> {
    for settings in [Some(global), settings.project_trusted().then_some(settings)]
        .into_iter()
        .flatten()
    {
        let mut packages = PackageManager::new(
            cwd.to_path_buf(),
            agent_dir.to_path_buf(),
            settings.clone(),
            npm_config.clone(),
        );
        if let ControlFlow::Break(message) = packages
            .install_missing(|_, message| ControlFlow::Break(message))
            .await
        {
            return Err(message);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "test helpers; a panic is a test failure"
    )]

    use std::path::PathBuf;

    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use indexmap::IndexMap;
    use sha2::Digest as _;
    use yapi_mock::{Cassette, Interaction, MockServer, RequestMatch, Response};

    use super::*;

    fn get(path: String, status: u16, body: Option<String>, bytes: Option<&[u8]>) -> Interaction {
        Interaction {
            request: RequestMatch {
                method: "GET".into(),
                path,
            },
            response: Response {
                status,
                headers: IndexMap::new(),
                chunks: body.into_iter().collect(),
                body_base64: bytes.map(|bytes| STANDARD.encode(bytes)),
                chunk_delay_ms: 0,
                close: None,
            },
        }
    }

    /// The packument and tarball of `ext-pkg@2.0.0` on the registry at `base`.
    fn publish(base: &str) -> Vec<Interaction> {
        let manifest = br#"{"name": "ext-pkg", "version": "2.0.0"}"#;
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "package/package.json", &manifest[..])
            .unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&builder.into_inner().unwrap()).unwrap();
        let tgz = gz.finish().unwrap();
        let integrity = STANDARD.encode(sha2::Sha512::digest(&tgz));
        let tarball = "/ext-pkg/-/ext-pkg-2.0.0.tgz";
        let packument = serde_json::json!({
            "name": "ext-pkg",
            "dist-tags": {"latest": "2.0.0"},
            "versions": {"2.0.0": {
                "name": "ext-pkg",
                "version": "2.0.0",
                "dist": {
                    "tarball": format!("{base}{tarball}"),
                    "integrity": format!("sha512-{integrity}"),
                },
            }},
        });
        vec![
            get("/ext-pkg".into(), 200, Some(packument.to_string()), None),
            get(tarball.into(), 200, None, Some(&tgz)),
        ]
    }

    /// `<dir>/project`, trusted, and `<dir>/agent`, whose settings list
    /// `user` and `project` packages, with the user's settings alone and with
    /// both.
    fn settings(
        name: &str,
        user: &[&str],
        project: &[&str],
    ) -> (PathBuf, PathBuf, [SettingsManager; 2]) {
        let dir = std::env::temp_dir().join(format!("yapi-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (cwd, agent) = (dir.join("project"), dir.join("agent"));
        std::fs::create_dir_all(cwd.join(PROJECT_DIR)).unwrap();
        std::fs::create_dir_all(&agent).unwrap();
        let packages = |packages: &[&str]| serde_json::json!({"packages": packages}).to_string();
        std::fs::write(agent.join("settings.json"), packages(user)).unwrap();
        std::fs::write(
            cwd.join(PROJECT_DIR).join("settings.json"),
            packages(project),
        )
        .unwrap();
        let managers = [
            SettingsManager::load(&agent, &cwd, false),
            SettingsManager::load(&agent, &cwd, true),
        ];
        (cwd, agent, managers)
    }

    fn registry(server: &MockServer) -> HashMap<String, String> {
        HashMap::from([("registry".to_owned(), format!("{}/", server.url()))])
    }

    /// pi's two resolves: a package in the user's and the trusted project's
    /// settings is installed in both scopes.
    #[tokio::test(flavor = "multi_thread")]
    async fn installs_user_then_projects() {
        let source = "npm:ext-pkg@^2";
        let (cwd, agent, [global, settings]) = settings("scopes", &[source], &[source]);
        let server = MockServer::local_with(|base| Cassette {
            interactions: [publish(base), publish(base)].concat(),
        })
        .await
        .unwrap();
        install_packages(&cwd, &agent, &global, &settings, registry(&server))
            .await
            .unwrap();
        assert!(agent.join("npm/node_modules/ext-pkg/package.json").exists());
        let project = cwd.join(PROJECT_DIR);
        assert!(
            project
                .join("npm/node_modules/ext-pkg/package.json")
                .exists()
        );
        assert_eq!(server.finish().unwrap().len(), 4);
    }

    /// As pi's resolve throws, the first failure ends the install.
    #[tokio::test(flavor = "multi_thread")]
    async fn stops_at_first_failure() {
        let (cwd, agent, [global, settings]) =
            settings("failure", &["npm:missing-pkg", "npm:ext-pkg"], &[]);
        let server = MockServer::local_with(|_| Cassette {
            interactions: vec![get("/missing-pkg".into(), 404, Some("{}".into()), None)],
        })
        .await
        .unwrap();
        let error = install_packages(&cwd, &agent, &global, &settings, registry(&server))
            .await
            .unwrap_err();
        assert!(
            error.starts_with("Failed to install npm:missing-pkg: "),
            "{error}"
        );
        assert_eq!(server.finish().unwrap().len(), 1);
    }
}
