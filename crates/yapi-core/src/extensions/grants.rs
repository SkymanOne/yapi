//! What an extension may reach outside its sandbox.

use crate::tools::RegisteredTool;

/// What an extension may reach. The default grants everything, as pi does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grants {
    /// Read and write files.
    pub filesystem: bool,
    /// Run processes, which run as the user outside the sandbox.
    pub process: bool,
    /// Send HTTP requests and resolve host names.
    pub network: bool,
    /// Read environment variables.
    pub environment: bool,
    /// Start turns and change the session and its files.
    pub session: bool,
}

impl Default for Grants {
    fn default() -> Grants {
        Grants {
            filesystem: true,
            process: true,
            network: true,
            environment: true,
            session: true,
        }
    }
}

impl Grants {
    /// Nothing outside the instance.
    pub fn none() -> Grants {
        Grants {
            filesystem: false,
            process: false,
            network: false,
            environment: false,
            session: false,
        }
    }

    /// The grants of manifest capabilities `values`, each `fs`, `net`,
    /// `env`, `session`, `all` or `none`. An empty list grants nothing, and
    /// only `all` grants running processes. `Err` names an unknown value,
    /// `process`, or `none` combined with another value.
    pub fn parse(values: &[String]) -> Result<Grants, String> {
        let mut grants = Grants::none();
        for value in values {
            match value.as_str() {
                "fs" => grants.filesystem = true,
                "net" => grants.network = true,
                "env" => grants.environment = true,
                "session" => grants.session = true,
                "all" => grants = Grants::default(),
                "none" if values.len() == 1 => {}
                "none" => return Err("\"none\" cannot be combined with other capabilities".into()),
                "process" => {
                    return Err(
                        "\"process\" is not a capability: a process runs as you, outside the sandbox, so only \"all\" allows it".into(),
                    );
                }
                other => return Err(format!("unknown capability \"{other}\"")),
            }
        }
        Ok(grants)
    }

    /// What either `self` or `other` grants.
    pub fn union(self, other: Grants) -> Grants {
        Grants {
            filesystem: self.filesystem || other.filesystem,
            process: self.process || other.process,
            network: self.network || other.network,
            environment: self.environment || other.environment,
            session: self.session || other.session,
        }
    }
}

/// Whether `grants` allow running `tool`. The built-in `bash`, `grep` and
/// `find` run processes, and the other built-in tools need file access. Any
/// other tool, including one an extension registers under a built-in name,
/// may reach anything, so it needs every grant.
pub fn tool_granted(grants: &Grants, tool: &RegisteredTool) -> bool {
    let all = *grants == Grants::default();
    match tool.name() {
        _ if tool.source.is_some() => all,
        "bash" => grants.process,
        // They run ripgrep and fd, which they download when missing.
        "grep" | "find" => grants.filesystem && grants.process && grants.network,
        "read" | "write" | "edit" | "ls" => grants.filesystem,
        _ => all,
    }
}

/// Whether an extension with `grants` hears pi event `kind`. The provider
/// request events carry the request's body and headers, credentials
/// included, which only the network grant may read.
pub fn receives(kind: &str, grants: &Grants) -> bool {
    match kind {
        "before_provider_request" | "before_provider_headers" => grants.network,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_parse() {
        let fs = Grants {
            filesystem: true,
            ..Grants::none()
        };
        let all_but_process = Grants {
            process: false,
            ..Grants::default()
        };
        let process = "\"process\" is not a capability: a process runs as you, outside the sandbox, so only \"all\" allows it";
        let cases: &[(&[&str], Result<Grants, &str>)] = &[
            (&[], Ok(Grants::none())),
            (&["none"], Ok(Grants::none())),
            (&["all"], Ok(Grants::default())),
            (&["fs", "all"], Ok(Grants::default())),
            (&["fs"], Ok(fs)),
            (&["fs", "net", "env", "session"], Ok(all_but_process)),
            (&["process"], Err(process)),
            (
                &["none", "fs"],
                Err("\"none\" cannot be combined with other capabilities"),
            ),
            (&["exec"], Err("unknown capability \"exec\"")),
        ];
        for (values, expected) in cases {
            let values: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
            let expected = expected.map_err(str::to_owned);
            assert_eq!(Grants::parse(&values), expected, "{values:?}");
        }
        let net = Grants {
            network: true,
            ..Grants::none()
        };
        assert_eq!(
            fs.union(net),
            Grants {
                filesystem: true,
                network: true,
                ..Grants::none()
            }
        );
    }

    /// Built-in tools need the grants of what they reach, and any other
    /// tool, even one registered under a built-in name, needs every grant.
    #[test]
    fn grants_gate_tools() {
        let env = crate::tools::ToolEnv {
            cwd: std::path::PathBuf::new(),
            runtime: std::sync::Arc::default(),
            bin_dir: std::path::PathBuf::new(),
        };
        let fs = Grants {
            filesystem: true,
            ..Grants::none()
        };
        let all_but_process = Grants {
            process: false,
            ..Grants::default()
        };
        // Each tool, extension or built-in, and whether none, fs, all but
        // process, and all may run it.
        let cases = [
            ("read", false, [false, true, true, true]),
            ("write", false, [false, true, true, true]),
            ("edit", false, [false, true, true, true]),
            ("ls", false, [false, true, true, true]),
            ("bash", false, [false, false, false, true]),
            ("grep", false, [false, false, false, true]),
            ("find", false, [false, false, false, true]),
            ("read", true, [false, false, false, true]),
        ];
        for (name, extension, expected) in cases {
            let mut tool = crate::tools::builtin(name, &env).unwrap();
            if extension {
                tool.source = Some(yapi_types::rpc::SourceInfo::default());
            }
            let granted = [Grants::none(), fs, all_but_process, Grants::default()]
                .map(|grants| tool_granted(&grants, &tool));
            assert_eq!(granted, expected, "{name}, extension {extension}");
        }
    }

    #[test]
    fn provider_events_need_network() {
        let net = Grants {
            network: true,
            ..Grants::none()
        };
        let no_net = Grants {
            network: false,
            ..Grants::default()
        };
        for (kind, heard_without_network) in [
            ("before_provider_request", false),
            ("before_provider_headers", false),
            ("after_provider_response", true),
            ("provider_stream_event", true),
            ("tool_call", true),
        ] {
            assert!(receives(kind, &net), "{kind}");
            assert_eq!(receives(kind, &no_net), heard_without_network, "{kind}");
        }
    }
}
