//! Which `pi.events` messages cross between extensions with different grants.

use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Grants;

/// The `pi.events` channels an extension may send to, and hear from,
/// extensions whose grants differ from its own. Each pattern is an exact
/// channel, a prefix ending in `*`, or `*` for every channel.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventPolicy {
    /// The channels it may send on.
    pub send: Vec<String>,
    /// The channels it may hear.
    pub receive: Vec<String>,
}

impl Default for EventPolicy {
    fn default() -> EventPolicy {
        EventPolicy::shared()
    }
}

impl EventPolicy {
    /// Every channel both ways, as pi shares its bus: the default.
    pub fn shared() -> EventPolicy {
        EventPolicy {
            send: vec!["*".into()],
            receive: vec!["*".into()],
        }
    }

    /// No channel either way, so only extensions with identical grants
    /// hear it and are heard.
    pub fn isolated() -> EventPolicy {
        EventPolicy {
            send: Vec::new(),
            receive: Vec::new(),
        }
    }

    /// The policy with patterns `send` and `receive`. `Err` names the first
    /// pattern that is empty or uses glob syntax other than a final `*`.
    pub fn new(send: Vec<String>, receive: Vec<String>) -> Result<EventPolicy, String> {
        for pattern in send.iter().chain(&receive) {
            let body = pattern.strip_suffix('*').unwrap_or(pattern);
            if pattern.is_empty() || body.contains(['*', '?', '[', ']', '{', '}']) {
                return Err(format!(
                    "invalid event channel pattern \"{pattern}\": use a channel name, a prefix ending in `*`, or `*`"
                ));
            }
        }
        Ok(EventPolicy { send, receive })
    }

    /// Whether it may send on `channel` across grants.
    pub fn sends(&self, channel: &str) -> bool {
        matches_any(&self.send, channel)
    }

    /// Whether it may hear `channel` across grants.
    pub fn receives(&self, channel: &str) -> bool {
        matches_any(&self.receive, channel)
    }

    /// The channels both `self` and `other` send on and hear.
    pub fn intersection(&self, other: &EventPolicy) -> EventPolicy {
        EventPolicy {
            send: both(&self.send, &other.send),
            receive: both(&self.receive, &other.receive),
        }
    }
}

/// The patterns of `left` and `right` that the other side covers whole. Two
/// patterns that match one channel are nested, so these match exactly the
/// channels both sides match.
fn both(left: &[String], right: &[String]) -> Vec<String> {
    let covers = |patterns: &[String], pattern: &str| match pattern.strip_suffix('*') {
        Some(prefix) => patterns.iter().any(|other| {
            other
                .strip_suffix('*')
                .is_some_and(|wider| prefix.starts_with(wider))
        }),
        None => matches_any(patterns, pattern),
    };
    let mut found: Vec<String> = Vec::new();
    for pattern in left.iter().filter(|pattern| covers(right, pattern)) {
        found.push(pattern.clone());
    }
    for pattern in right.iter().filter(|pattern| covers(left, pattern)) {
        if !found.contains(pattern) {
            found.push(pattern.clone());
        }
    }
    found
}

fn matches_any(patterns: &[String], channel: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| match pattern.strip_suffix('*') {
            Some(prefix) => channel.starts_with(prefix),
            None => pattern == channel,
        })
}

/// Whether an event on `channel` from an extension with `sender`'s grants
/// and policy reaches one with `receiver`'s: always between identical
/// grants, since neither gains anything, and otherwise when the sender may
/// send the channel and the receiver may hear it.
pub fn delivers(
    sender: (&Grants, &EventPolicy),
    receiver: (&Grants, &EventPolicy),
    channel: &str,
) -> bool {
    // shortcut: identical grants ignore filesystem roots, which only codemode narrows,
    // compare roots too once file grants are path-scoped.
    sender.0 == receiver.0 || sender.1.sends(channel) && receiver.1.receives(channel)
}

/// Written as a manifest writes it: `"shared"`, `"isolated"`, or
/// `{ send, receive }`.
impl Serialize for EventPolicy {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if *self == EventPolicy::shared() {
            return serializer.serialize_str("shared");
        }
        if *self == EventPolicy::isolated() {
            return serializer.serialize_str("isolated");
        }
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("send", &self.send)?;
        map.serialize_entry("receive", &self.receive)?;
        map.end()
    }
}

/// Read from `"shared"`, `"isolated"`, or `{ send, receive }`, where a
/// missing list is empty.
impl<'de> Deserialize<'de> for EventPolicy {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<EventPolicy, D::Error> {
        struct PolicyVisitor;

        impl<'de> Visitor<'de> for PolicyVisitor {
            type Value = EventPolicy;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("\"shared\", \"isolated\" or { send, receive }")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<EventPolicy, E> {
                match value {
                    "shared" => Ok(EventPolicy::shared()),
                    "isolated" => Ok(EventPolicy::isolated()),
                    _ => Err(E::invalid_value(de::Unexpected::Str(value), &self)),
                }
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<EventPolicy, A::Error> {
                let (mut send, mut receive) = (Vec::new(), Vec::new());
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "send" => send = map.next_value()?,
                        "receive" => receive = map.next_value()?,
                        _ => return Err(de::Error::unknown_field(&key, &["send", "receive"])),
                    }
                }
                EventPolicy::new(send, receive).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(PolicyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_policy_routing() {
        let policy = |send: &[&str], receive: &[&str]| {
            let owned = |patterns: &[&str]| patterns.iter().map(|p| (*p).to_owned()).collect();
            EventPolicy::new(owned(send), owned(receive)).unwrap()
        };
        let todo = policy(&["todo:*"], &["subagent:done"]);
        assert!(todo.sends("todo:changed") && todo.sends("todo:"));
        assert!(!todo.sends("todos") && !todo.sends("subagent:done"));
        assert!(todo.receives("subagent:done") && !todo.receives("subagent:done:x"));
        assert!(EventPolicy::default().sends("any") && EventPolicy::shared().receives(""));
        assert!(!EventPolicy::isolated().sends("any"));
        // An intersection matches exactly the channels both match.
        let wide = policy(&["todo:*", "x"], &["*"]);
        let both = todo.intersection(&wide);
        assert_eq!(both, policy(&["todo:*"], &["subagent:done"]));
        let narrow = policy(&["todo:a", "todo:b*", "y"], &[]);
        assert_eq!(
            narrow.intersection(&todo),
            policy(&["todo:a", "todo:b*"], &[])
        );
        assert_eq!(
            EventPolicy::shared().intersection(&EventPolicy::isolated()),
            EventPolicy::isolated()
        );

        let fs = Grants {
            filesystem: true,
            ..Grants::none()
        };
        let all = Grants::default();
        let isolated = EventPolicy::isolated();
        let shared = EventPolicy::shared();
        let cases = [
            // Identical grants always share, whatever the policies.
            ((&fs, &isolated), (&fs, &isolated), "x", true),
            ((&all, &shared), (&fs, &shared), "x", true),
            ((&fs, &isolated), (&all, &shared), "x", false),
            ((&all, &shared), (&fs, &isolated), "x", false),
            ((&fs, &todo), (&all, &shared), "todo:changed", true),
            ((&fs, &todo), (&all, &shared), "other", false),
            ((&all, &shared), (&fs, &todo), "subagent:done", true),
            ((&all, &shared), (&fs, &todo), "todo:changed", false),
        ];
        for (sender, receiver, channel, expected) in cases {
            assert_eq!(
                delivers(sender, receiver, channel),
                expected,
                "{sender:?} to {receiver:?} on {channel}"
            );
        }

        for bad in ["", "a*b", "**", "todo:?", "[ab]", "{a,b}"] {
            assert!(
                EventPolicy::new(vec![bad.into()], Vec::new()).is_err(),
                "{bad}"
            );
        }
        let parse =
            |json: &str| serde_json::from_str::<EventPolicy>(json).map_err(|e| e.to_string());
        assert_eq!(parse("\"shared\""), Ok(EventPolicy::shared()));
        assert_eq!(parse("\"isolated\""), Ok(EventPolicy::isolated()));
        assert_eq!(
            parse(r#"{"send": ["todo:*"]}"#),
            Ok(policy(&["todo:*"], &[]))
        );
        for bad in [r#""open""#, r#"{"sent": []}"#, r#"{"send": ["a*b"]}"#, "[]"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        for policy in [EventPolicy::shared(), EventPolicy::isolated(), todo] {
            let json = yapi_types::json::to_string(&policy).unwrap();
            assert_eq!(parse(&json), Ok(policy));
        }
    }
}
