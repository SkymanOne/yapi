//! `chest init`: a new manifest with folders for the agents found, or a
//! `[package]` table for a package.

use std::path::Path;

use anyhow::bail;
use clap::Args;
use yapi_core::chest::manifest::{ManifestDocument, Package};
use yapi_core::settings::Scope;
use yapi_core::tools::path::home_dir;

use crate::prompt::{Choice, Prompt};
use crate::{Target, say, write};

/// `chest init` options.
#[derive(Args)]
pub struct InitArgs {
    /// Skill folders, comma-separated, the canonical one first.
    #[arg(long, value_delimiter = ',')]
    skills: Option<Vec<String>>,
    /// Subagent folders, comma-separated, the canonical one first.
    #[arg(long, value_delimiter = ',')]
    agents: Option<Vec<String>>,
    /// The extension store.
    #[arg(long)]
    store: Option<String>,
    /// Add a `[package]` table for publishing, to a new or existing manifest.
    #[arg(long)]
    package: bool,
    /// Take the defaults without asking.
    #[arg(long)]
    pub yes: bool,
}

/// One agent's folders. A project marker is relative to the project, a
/// home marker to the home folder, as are the global folders.
struct Agent {
    name: &'static str,
    markers: &'static [&'static str],
    skills: &'static str,
    agents: Option<&'static str>,
    home_marker: &'static str,
    home_skills: &'static str,
    home_agents: Option<&'static str>,
}

/// The agents chest knows. Skill folders follow the Agent Skills
/// locations that skills.sh lists for each agent, subagent folders each
/// agent's own documentation.
const AGENTS: [Agent; 7] = [
    // Agent Skills (Codex, Cursor, Copilot, OpenCode, pi)
    Agent {
        name: "Agent Skills",
        markers: &[".agents"],
        skills: ".agents/skills",
        agents: None,
        home_marker: ".agents",
        home_skills: ".agents/skills",
        home_agents: None,
    },
    // Claude Code
    Agent {
        name: "Claude Code",
        markers: &[".claude"],
        skills: ".claude/skills",
        agents: Some(".claude/agents"),
        home_marker: ".claude",
        home_skills: ".claude/skills",
        home_agents: Some(".claude/agents"),
    },
    // Codex
    Agent {
        name: "Codex",
        markers: &[".codex"],
        skills: ".agents/skills",
        agents: Some(".codex/agents"),
        home_marker: ".codex",
        home_skills: ".agents/skills",
        home_agents: Some(".codex/agents"),
    },
    // Cursor
    Agent {
        name: "Cursor",
        markers: &[".cursor"],
        skills: ".agents/skills",
        agents: Some(".cursor/agents"),
        home_marker: ".cursor",
        home_skills: ".cursor/skills",
        home_agents: Some(".cursor/agents"),
    },
    // GitHub Copilot
    Agent {
        name: "GitHub Copilot",
        markers: &[".github/agents", ".github/copilot-instructions.md"],
        skills: ".agents/skills",
        agents: Some(".github/agents"),
        home_marker: ".copilot",
        home_skills: ".copilot/skills",
        home_agents: Some(".copilot/agents"),
    },
    // OpenCode
    Agent {
        name: "OpenCode",
        markers: &[".opencode"],
        skills: ".agents/skills",
        agents: Some(".opencode/agent"),
        home_marker: ".config/opencode",
        home_skills: ".config/opencode/skills",
        home_agents: Some(".config/opencode/agent"),
    },
    // yapi
    Agent {
        name: "yapi",
        markers: &[".yapi"],
        skills: ".agents/skills",
        agents: Some(".yapi/agents"),
        home_marker: ".yapi",
        home_skills: ".agents/skills",
        home_agents: Some(YAPI_HOME_AGENTS),
    },
];

/// The folders on offer for one kind, in table order without repeats, each
/// with the agents that read it, and which of them belong to a detected
/// agent.
fn offered(
    scope: Scope,
    base: &Path,
    pick: impl Fn(&Agent, Scope) -> Option<&'static str>,
) -> (Vec<(String, Vec<&'static str>)>, Vec<usize>) {
    let mut folders: Vec<(String, Vec<&'static str>)> = Vec::new();
    let mut detected = Vec::new();
    for agent in &AGENTS {
        let Some(folder) = pick(agent, scope) else {
            continue;
        };
        let folder = match scope {
            Scope::Project => folder.to_owned(),
            Scope::Global if folder == YAPI_HOME_AGENTS => match yapi_home_agents() {
                Ok(folder) => folder,
                Err(_) => continue,
            },
            Scope::Global => format!("~/{folder}"),
        };
        let index = folders
            .iter()
            .position(|(known, _)| *known == folder)
            .unwrap_or_else(|| {
                folders.push((folder, Vec::new()));
                folders.len() - 1
            });
        folders[index].1.push(agent.name);
        let found = match scope {
            Scope::Project => agent
                .markers
                .iter()
                .any(|marker| base.join(marker).exists()),
            Scope::Global => home_dir().join(agent.home_marker).exists(),
        };
        if found && !detected.contains(&index) {
            detected.push(index);
        }
    }
    detected.sort_unstable();
    (folders, detected)
}

/// The folders for `kind` the person ticks, `found` or else `fallback`
/// unless asked, with the one that holds the copy first. Asked, they pick
/// that one among two or more, from the found folder when only one was
/// found, else the first.
fn choose(
    prompt: &Prompt,
    kind: &str,
    folders: &[(String, Vec<&str>)],
    (found, fallback): (Vec<usize>, Vec<usize>),
) -> anyhow::Result<Vec<String>> {
    let options: Vec<Choice<usize>> = folders
        .iter()
        .enumerate()
        .map(|(index, (folder, agents))| (index, folder.clone(), agents.join(", ")))
        .collect();
    let question = format!("Where should {kind} go?\nOne holds the copy, the others link to it.");
    let defaults = if found.is_empty() {
        fallback
    } else {
        found.clone()
    };
    let mut chosen = prompt.multiselect(&question, &options, defaults, false)?;
    if let [first, _, ..] = chosen[..] {
        let default = match found[..] {
            [only] if chosen.contains(&only) => only,
            _ => first,
        };
        let options: Vec<Choice<usize>> = chosen
            .iter()
            .map(|&index| (index, folders[index].0.clone(), String::new()))
            .collect();
        let copy = prompt.select("Which folder holds the copy?", &options, default)?;
        chosen.retain(|&index| index != copy);
        chosen.insert(0, copy);
    }
    Ok(chosen
        .into_iter()
        .map(|index| folders[index].0.clone())
        .collect())
}

/// A TOML string list.
fn list(values: &[String]) -> String {
    let quoted: Vec<String> = values
        .iter()
        .map(|value| yapi_types::json::to_string(value).unwrap_or_default())
        .collect();
    format!("[{}]", quoted.join(", "))
}

/// yapi's global subagent folder in [`AGENTS`], relative to the home folder.
const YAPI_HOME_AGENTS: &str = ".yapi/agent/agents";

/// yapi's global subagent folder, in the agent folder that
/// `YAPI_CODING_AGENT_DIR` may move, from `~`. The global manifest installs
/// only inside the home folder, so a folder outside it is the error.
fn yapi_home_agents() -> Result<String, std::path::PathBuf> {
    let agents = yapi_core::config::agent_dir().join("agents");
    match agents.strip_prefix(home_dir()) {
        Ok(path) => Ok(format!("~/{}", path.display())),
        Err(_) => Err(agents),
    }
}

/// Runs `chest init`.
pub fn run(target: &Target, args: &InitArgs, prompt: &Prompt) -> anyhow::Result<()> {
    let exists = target.path.exists();
    if exists && !args.package {
        bail!("{} already exists", target.path.display());
    }
    let base = target.base();
    let mut document = if exists {
        ManifestDocument::parse(&crate::read(target)?, target.scope)?
    } else {
        let skills = match args.skills.clone() {
            Some(skills) => skills,
            None => {
                let (folders, found) = offered(target.scope, &base, |agent, scope| match scope {
                    Scope::Project => Some(agent.skills),
                    Scope::Global => Some(agent.home_skills),
                });
                choose(prompt, "skills", &folders, (found, vec![0]))?
            }
        };
        let agents = match args.agents.clone() {
            Some(agents) => agents,
            None => {
                let (folders, found) = offered(target.scope, &base, |agent, scope| match scope {
                    Scope::Project => agent.agents,
                    Scope::Global => agent.home_agents,
                });
                // Without an agent found, subagents go where yapi reads them.
                let yapi = match target.scope {
                    Scope::Project => Ok(".yapi/agents".to_owned()),
                    Scope::Global => yapi_home_agents(),
                };
                if let Err(folder) = &yapi {
                    prompt.info(&[format!(
                    "Left out yapi's subagent folder {}: the global manifest installs only inside your home folder",
                    folder.display()
                )]);
                }
                let fallback = folders
                    .iter()
                    .position(|(folder, _)| yapi.as_ref().is_ok_and(|yapi| folder == yapi))
                    .into_iter()
                    .collect();
                choose(prompt, "subagents", &folders, (found, fallback))?
            }
        };
        let store = args.store.clone().unwrap_or_else(|| match target.scope {
            Scope::Project => ".yapi/chest".into(),
            Scope::Global => "~/.chest/store".into(),
        });
        let install = yapi_core::chest::install_command(&target.path, target.scope);
        let mut text = format!(
            "# Installed with `{install}`, pinned in the lock beside this file.\n\n[folders]\n"
        );
        if !skills.is_empty() {
            text.push_str(&format!("skills = {}\n", list(&skills)));
        }
        if !agents.is_empty() {
            text.push_str(&format!("agents = {}\n", list(&agents)));
        }
        let summary = [
            ("skills", skills.join(", ")),
            ("subagents", agents.join(", ")),
        ];
        let mut lines: Vec<String> = summary
            .iter()
            .filter(|(_, folders)| !folders.is_empty())
            .map(|(kind, folders)| format!("{kind}: {folders}"))
            .collect();
        lines.push(format!("extensions: {store}"));
        // With both folder flags, chest asked nothing, so it writes at once.
        if args.skills.is_none() || args.agents.is_none() {
            prompt.summary(&target.name(), &lines);
            prompt.confirm(&format!("Write {}?", target.name()))?;
        }
        let store = yapi_types::json::to_string(&store).unwrap_or_default();
        text.push_str(&format!("extensions = {store}\n"));
        ManifestDocument::parse(&text, target.scope)?
    };
    if args.package {
        if document.manifest().package.is_some() {
            bail!("{} already has a [package] table", target.path.display());
        }
        document.set_package(&scaffold(&base))?;
    }
    write(target, &document.to_string())?;
    match prompt.styled() {
        true => prompt.outro(&format!("Wrote {}", target.label())),
        false => say(&format!("Wrote {}", target.path.display())),
    }
    Ok(())
}

/// A `[package]` table for the package at `root`, listing the conventional
/// folders it has and requesting no capabilities.
fn scaffold(root: &Path) -> Package {
    let folder = root.file_name().unwrap_or_default().to_string_lossy();
    let name: String = folder
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let listed =
        |folder: &str, entry: &str| root.join(folder).is_dir().then(|| vec![entry.to_owned()]);
    Package {
        name: Some(name),
        version: Some("0.1.0".into()),
        capabilities: Some(Vec::new()),
        skills: listed("skills", "skills/*"),
        agents: listed("agents", "agents/*.md"),
        extensions: listed("extensions", "extensions"),
        prompts: listed("prompts", "prompts"),
        themes: listed("themes", "themes"),
        ..Package::default()
    }
}
