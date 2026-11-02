//! What an extension may reach outside its sandbox.

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
}
