//! [`AgentName`] and [`HarnessRef`]: naming any agent a node can run, and
//! [`TerminalSupport`]: whether it runs in a terminal.

use std::fmt;
use std::str::FromStr;

use serde::de::{IntoDeserializer as _, MapAccess, Visitor};
use serde::ser::SerializeMap as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::harness::HarnessKind;
use crate::name::{InvalidName, ProfileName};
use crate::node::ProtocolVersion;

/// The first protocol that carries an added agent.
const ADDED_AGENTS: ProtocolVersion = ProtocolVersion::new(11);

/// The name of an agent added on a node by its command (protocol 11, ADR
/// 0032): the profile name rule (`[A-Za-z0-9._-]`, 1 to 64), and never a
/// name of one of the four known agents, in any case or spelling
/// (`claude`, `Claude-Code`, `opencode`).
///
/// Why the profile rule: an added agent's default profile is named after
/// it, as the four's are (ADR 0011), so every agent name must be a profile
/// name. Why the four are refused: a word a person types is read as one of
/// the four first ([`HarnessRef::from_str`]), so an added agent called
/// `codex` could never be named.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct AgentName(ProfileName);

impl AgentName {
    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// The name of this agent's default profile: its own.
    #[must_use]
    pub const fn profile(&self) -> &ProfileName {
        &self.0
    }
}

impl fmt::Display for AgentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why a text is not an [`AgentName`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentNameError {
    /// It breaks the profile name rule.
    Invalid(InvalidName),
    /// It names one of the four agents Vessel knows.
    Reserved {
        /// The text given.
        written: String,
        /// The known agent it names.
        harness: HarnessKind,
    },
}

impl fmt::Display for AgentNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(invalid) => fmt::Display::fmt(invalid, f),
            Self::Reserved { written, harness } => write!(
                f,
                "`{written}` names {harness}, an agent Vessel knows; an added agent needs another name"
            ),
        }
    }
}

impl std::error::Error for AgentNameError {}

impl FromStr for AgentName {
    type Err = AgentNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(harness) = s.parse::<HarnessKind>() {
            return Err(AgentNameError::Reserved {
                written: s.to_owned(),
                harness,
            });
        }
        s.parse().map(Self).map_err(AgentNameError::Invalid)
    }
}

crate::key::serde_via_text!(AgentName);

/// Any agent a node can run (protocol 11, ADR 0032): one of the four Vessel
/// knows, or one added on that node by its command.
///
/// On the wire a known agent is the string it was before protocol 11
/// (`"claude-code"`, `"codex"`, `"gemini"`, `"open-code"`), so every spec
/// and list an older node or client wrote reads the same; an added agent is
/// `{"added": "<name>"}`, an object, so it can never be read as one of the
/// four.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(as = "HarnessRefShape"))]
pub enum HarnessRef {
    /// One of the four.
    Known(HarnessKind),
    /// One added on the node by its command.
    Added(AgentName),
}

/// [`HarnessRef`]'s declaration for TypeScript, as its serde form is:
/// `HarnessKind | { added: AgentName }`. A type of its own rather than a
/// written-out string, so the generator also declares `AgentName`.
#[cfg(feature = "ts")]
#[derive(ts_rs::TS)]
#[ts(rename = "HarnessRef", untagged)]
#[expect(dead_code, reason = "only its TypeScript declaration is used")]
enum HarnessRefShape {
    Known(HarnessKind),
    Added { added: AgentName },
}

impl HarnessRef {
    /// The known agent, if this is one.
    #[must_use]
    pub const fn known(&self) -> Option<HarnessKind> {
        match self {
            Self::Known(kind) => Some(*kind),
            Self::Added(_) => None,
        }
    }

    /// The added agent's name, if this is one.
    #[must_use]
    pub const fn added(&self) -> Option<&AgentName> {
        match self {
            Self::Known(_) => None,
            Self::Added(name) => Some(name),
        }
    }

    /// The protocol a peer must speak to read this: an added agent came in
    /// 11.
    #[must_use]
    pub const fn since(&self) -> ProtocolVersion {
        match self {
            Self::Known(_) => ProtocolVersion::new(2),
            Self::Added(_) => ADDED_AGENTS,
        }
    }
}

impl From<HarnessKind> for HarnessRef {
    fn from(kind: HarnessKind) -> Self {
        Self::Known(kind)
    }
}

impl From<AgentName> for HarnessRef {
    fn from(name: AgentName) -> Self {
        Self::Added(name)
    }
}

/// Tests and filters compare a spec's agent with one of the four.
impl PartialEq<HarnessKind> for HarnessRef {
    fn eq(&self, other: &HarnessKind) -> bool {
        self.known() == Some(*other)
    }
}

/// The known agent's name (`claude-code`), or the added agent's.
impl fmt::Display for HarnessRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(kind) => fmt::Display::fmt(kind, f),
            Self::Added(name) => fmt::Display::fmt(name, f),
        }
    }
}

/// A word that names no agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAgent(pub String);

impl fmt::Display for UnknownAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` names no agent: one of claude, codex, gemini, opencode, or an added agent's name",
            self.0
        )
    }
}

impl std::error::Error for UnknownAgent {}

/// One word as a person types it (the CLI, an agent tool's argument,
/// `harness = "..."` in `profiles.toml`): one of the four by any of its
/// names, otherwise an added agent's name. Whether a node has that added
/// agent is the node's to say.
impl FromStr for HarnessRef {
    type Err = UnknownAgent;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(kind) = s.parse::<HarnessKind>() {
            return Ok(Self::Known(kind));
        }
        s.trim()
            .parse::<AgentName>()
            .map(Self::Added)
            .map_err(|_invalid| UnknownAgent(s.to_owned()))
    }
}

impl Serialize for HarnessRef {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Known(kind) => kind.serialize(serializer),
            Self::Added(name) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("added", name)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for HarnessRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RefVisitor;

        impl<'de> Visitor<'de> for RefVisitor {
            type Value = HarnessRef;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(r#"one of "claude-code", "codex", "gemini", "open-code", or {"added": "<name>"}"#)
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<HarnessRef, E> {
                // The four exactly as their wire names, as before protocol
                // 11: an alias (`claude`) was never a wire name.
                HarnessKind::deserialize(text.into_deserializer()).map(HarnessRef::Known)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<HarnessRef, A::Error> {
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                enum Key {
                    Added,
                }
                let (Key::Added, name) = map
                    .next_entry::<Key, AgentName>()?
                    .ok_or_else(|| serde::de::Error::missing_field("added"))?;
                if map.next_key::<Key>()?.is_some() {
                    return Err(serde::de::Error::duplicate_field("added"));
                }
                Ok(HarnessRef::Added(name))
            }
        }

        deserializer.deserialize_any(RefVisitor)
    }
}

/// Whether a harness runs as a terminal session on a node (protocol 11).
/// Absent from older nodes, whose every harness runs in one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TerminalSupport {
    /// It does.
    #[default]
    Available,
    /// It does not; `reason` says why (an added agent: Vessel knows only
    /// the command that speaks ACP).
    Unavailable {
        /// Human-readable reason.
        reason: String,
    },
}

impl TerminalSupport {
    /// Whether it does.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_keep_their_wire_names_and_added_agents_are_objects() {
        for kind in HarnessKind::ALL {
            let known = HarnessRef::Known(kind);
            let json = serde_json::to_string(&known).unwrap();
            assert_eq!(json, serde_json::to_string(&kind).unwrap());
            assert_eq!(serde_json::from_str::<HarnessRef>(&json).unwrap(), known);
        }
        let goose = HarnessRef::Added("goose".parse().unwrap());
        let json = serde_json::to_string(&goose).unwrap();
        assert_eq!(json, r#"{"added":"goose"}"#);
        assert_eq!(serde_json::from_str::<HarnessRef>(&json).unwrap(), goose);
    }

    #[test]
    fn what_does_not_decode() {
        for bad in [
            r#""claude""#,
            r#""goose""#,
            r#"{"added":"codex"}"#,
            r#"{"added":"Claude-Code"}"#,
            r#"{"added":"a b"}"#,
            r#"{"added":""}"#,
            r#"{"added":"goose","more":1}"#,
            r#"{"other":"goose"}"#,
            "{}",
            "3",
        ] {
            assert!(serde_json::from_str::<HarnessRef>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_typed_word_is_one_of_the_four_first() {
        assert_eq!(
            "claude".parse::<HarnessRef>().unwrap(),
            HarnessKind::ClaudeCode
        );
        assert_eq!(
            "goose".parse::<HarnessRef>().unwrap(),
            HarnessRef::Added("goose".parse().unwrap())
        );
        assert!("a b".parse::<HarnessRef>().is_err());
        assert!(matches!(
            "OpenCode".parse::<AgentName>(),
            Err(AgentNameError::Reserved {
                harness: HarnessKind::OpenCode,
                ..
            })
        ));
    }

    #[test]
    fn versions_and_names() {
        let goose = HarnessRef::Added("goose".parse().unwrap());
        assert_eq!(goose.since(), ProtocolVersion::new(11));
        assert_eq!(
            goose.added().map(|name| name.profile().as_str()),
            Some("goose")
        );
        let codex = HarnessRef::Known(HarnessKind::Codex);
        assert_eq!(codex.since(), ProtocolVersion::new(2));
        assert_eq!(goose.to_string(), "goose");
        assert_eq!(codex.to_string(), "codex");
    }

    #[test]
    fn terminal_support_defaults_to_available() {
        assert_eq!(
            serde_json::from_str::<TerminalSupport>(r#"{"state":"available"}"#).unwrap(),
            TerminalSupport::Available
        );
        assert!(TerminalSupport::default().is_available());
    }
}
