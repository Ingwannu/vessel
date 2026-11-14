//! Which agent harnesses exist and what a node found installed.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::agents::{HarnessRef, READS_TEN_AGENTS, TerminalSupport};
use crate::node::ProtocolVersion;

/// An agent harness Vessel knows how to drive.
///
/// Closed on purpose: everything harness-specific (argv, environment,
/// structured-mode launch) lives in one module per variant in
/// `vessel-harness`, and adding a variant must fail to compile until that
/// module exists. The first four came with protocol 2; the other seven
/// with ADR 0058, which a peer reads only from the version
/// [`HarnessKind::since`] gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum HarnessKind {
    /// Claude Code (`claude`).
    ClaudeCode,
    /// The `OpenAI` Codex CLI (`codex`).
    Codex,
    /// Google Gemini CLI (`gemini`).
    Gemini,
    /// The `OpenCode` CLI (`opencode`).
    OpenCode,
    /// xAI's Grok Build CLI (`grok`), over ACP.
    Grok,
    /// Pi (`pi`), over its own JSON lines protocol.
    Pi,
    /// Oh My Pi (`omp`), Pi's fork, over its own JSON lines protocol.
    Omp,
    /// Google Antigravity's CLI (`agy`), over its stream.
    Antigravity,
    /// Meta's Muse Code (`muse`), over ACP first.
    Muse,
    /// Amp (`amp`), through the third-party `amp-acp` adapter.
    Amp,
    /// Cursor's agent CLI (`cursor-agent`), over ACP.
    Cursor,
}

impl HarnessKind {
    /// Every harness kind, in display order: the four, then the seven in
    /// the order the owner named them.
    pub const ALL: [Self; 11] = [
        Self::ClaudeCode,
        Self::Codex,
        Self::Gemini,
        Self::OpenCode,
        Self::Grok,
        Self::Pi,
        Self::Omp,
        Self::Antigravity,
        Self::Muse,
        Self::Amp,
        Self::Cursor,
    ];

    /// Canonical kebab-case name used on the wire and in the CLI.
    #[must_use]
    #[cfg_attr(
        feature = "ts",
        expect(
            clippy::same_name_method,
            reason = "`ts_rs::TS::name` is the type's TypeScript name, an associated function; callers of this method have a value"
        )
    )]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::OpenCode => "opencode",
            Self::Grok => "grok",
            Self::Pi => "pi",
            Self::Omp => "omp",
            Self::Antigravity => "antigravity",
            Self::Muse => "muse",
            Self::Amp => "amp",
            Self::Cursor => "cursor",
        }
    }

    /// The protocol a peer must speak to read this kind: 2 for the four,
    /// [`READS_TEN_AGENTS`] for the seven of ADR 0058 (the one place their
    /// number is written). An older peer decodes a kind as one of
    /// the four strings it knows, so it is handed the seven as ADR 0032
    /// hands it an added agent (`agents::older`).
    #[must_use]
    pub const fn since(self) -> ProtocolVersion {
        match self {
            Self::ClaudeCode | Self::Codex | Self::Gemini | Self::OpenCode => {
                ProtocolVersion::new(2)
            }
            Self::Grok
            | Self::Pi
            | Self::Omp
            | Self::Antigravity
            | Self::Muse
            | Self::Amp
            | Self::Cursor => READS_TEN_AGENTS,
        }
    }
}

impl fmt::Display for HarnessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A harness name that is not recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownHarness(pub String);

impl fmt::Display for UnknownHarness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown harness `{}`", self.0)
    }
}

impl std::error::Error for UnknownHarness {}

impl FromStr for HarnessKind {
    type Err = UnknownHarness;

    /// Accepts the canonical name, the binary name (`claude`, `agy`,
    /// `cursor-agent`) and the names the agents' own projects use
    /// (`amp-acp`, `muse-code`, `oh-my-pi`).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "claude-code" | "claude" => Ok(Self::ClaudeCode),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            "opencode" | "open-code" => Ok(Self::OpenCode),
            "grok" => Ok(Self::Grok),
            "pi" => Ok(Self::Pi),
            "omp" | "oh-my-pi" => Ok(Self::Omp),
            "antigravity" | "agy" => Ok(Self::Antigravity),
            "muse" | "muse-code" => Ok(Self::Muse),
            "amp" | "amp-acp" => Ok(Self::Amp),
            "cursor" | "cursor-agent" => Ok(Self::Cursor),
            other => Err(UnknownHarness(other.to_owned())),
        }
    }
}

/// Whether a harness can run as a structured (ACP) session on this node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StructuredSupport {
    /// Ready: the harness speaks ACP itself, or its adapter is installed.
    Available,
    /// Not usable; `reason` says what is missing (for example the adapter).
    Unavailable {
        /// Human-readable reason.
        reason: String,
    },
}

/// A harness found on a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Harness {
    /// Which harness: one of the four, or (protocol 11) an agent added on
    /// the node, which a node leaves out for an older client.
    pub kind: HarnessRef,
    /// Resolved executable path.
    pub path: PathBuf,
    /// First line of `<binary> --version`, if it answered in time.
    pub version: Option<String>,
    /// Structured-mode availability.
    pub structured: StructuredSupport,
    /// Terminal-mode availability (protocol 11): every one of the four
    /// installed runs in a terminal; an added agent does not. Left out
    /// when available.
    #[serde(default, skip_serializing_if = "TerminalSupport::is_available")]
    pub terminal: TerminalSupport,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for kind in HarnessKind::ALL {
            assert_eq!(kind.name().parse::<HarnessKind>().unwrap(), kind);
        }
        assert_eq!(
            "claude".parse::<HarnessKind>().unwrap(),
            HarnessKind::ClaudeCode
        );
        for (alias, kind) in [
            ("agy", HarnessKind::Antigravity),
            ("amp-acp", HarnessKind::Amp),
            ("muse-code", HarnessKind::Muse),
            ("oh-my-pi", HarnessKind::Omp),
            ("cursor-agent", HarnessKind::Cursor),
        ] {
            assert_eq!(alias.parse::<HarnessKind>().unwrap(), kind);
        }
    }

    #[test]
    fn the_wire_names_are_the_canonical_names_but_open_code() {
        for kind in HarnessKind::ALL {
            let wire = serde_json::to_value(kind).unwrap();
            let expected = if kind == HarnessKind::OpenCode {
                "open-code"
            } else {
                kind.name()
            };
            assert_eq!(wire, serde_json::json!(expected));
            assert_eq!(serde_json::from_value::<HarnessKind>(wire).unwrap(), kind);
        }
    }

    #[test]
    fn the_seven_need_their_protocol_and_the_four_protocol_2() {
        let (four, seven) = HarnessKind::ALL.split_at(4);
        assert!(
            four.iter()
                .all(|kind| kind.since() == ProtocolVersion::new(2))
        );
        assert!(seven.iter().all(|kind| kind.since() == READS_TEN_AGENTS));
    }
}
