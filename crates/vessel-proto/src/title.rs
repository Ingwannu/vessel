//! A session's title (protocol 9, ADR 0022): parsed once, valid wherever
//! it is held.
//!
//! [Decision Log]
//! - Purpose: a board of cards named after folders is not readable, so
//!   every session gets a name a person recognizes: one they typed
//!   (`SessionSpec.title`, `session.rename`) or the first line of its
//!   first prompt.
//! - Constraint: a title is shown on one line of a card, a tab and a
//!   terminal; a newline or a control character in one breaks all three,
//!   and a title of a whole pasted file fills them. AGENTS.md section 2:
//!   a value with a shape is a type whose constructor checks it.
//! - Alternatives: a `String` checked where the node receives it (every
//!   other holder, the store and every client, would have to trust or
//!   repeat the check; a stored row could hold what the check refuses).
//! - Choice: [`SessionTitle`], built only by [`FromStr`] (a typed title:
//!   trimmed, not empty, at most [`SessionTitle::MAX_CHARS`] characters,
//!   no control characters; anything else is refused, not cut, so a
//!   client never shows another title than the one it sent) and by
//!   [`SessionTitle::from_prompt`] (the first non-empty line, its runs of
//!   blanks and control characters made one space, cut at
//!   [`SessionTitle::FROM_PROMPT_CHARS`] with no ellipsis: a prompt was
//!   never meant as a title, so it is cut rather than refused; Paseo's
//!   `deriveInitialAgentTitle`, TRACKER 801). Decoding goes through the
//!   parse, so a bad title on the wire or in the store is a decode error,
//!   never a value.
//! - Longer typed titles (TRACKER 801, amending ADR 0022): Paseo allows
//!   200 (`MAX_EXPLICIT_AGENT_TITLE_CHARS`) where Vessel allowed 120. A
//!   peer decodes a title through this parse, so an older one refuses any
//!   frame holding a longer title. From [`LONG_TITLES_PROTOCOL`] the
//!   limit is [`SessionTitle::MAX_CHARS`]. Below it a client sends at most
//!   [`SessionTitle::LEGACY_MAX_CHARS`] (`Call::since`), and every reply
//!   or event holding a longer title reaches it cut to that many characters
//!   ([`SessionTitle::for_protocol`]). The store keeps the whole title.
//!   Clients read their limit from [`SessionTitle::max_chars_for`].
//! - Characters, not bytes: a Korean title is cut where a person counts.
//!   `char`s, not grapheme clusters, so no dependency; a cut may split a
//!   combining sequence, which only the last character of a long line
//!   risks.

mod generated;
pub use generated::{GeneratedTitle, InvalidGeneratedTitle, MAX_GENERATED_TITLE_UNITS};

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::node::ProtocolVersion;

/// The protocol from which a title may be longer than
/// [`SessionTitle::LEGACY_MAX_CHARS`] (TRACKER 801). Assigned 30 at its
/// node-line merge, the next free number in merge order.
pub const LONG_TITLES_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(30));

/// A session's title: one line of text a person reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct SessionTitle(Arc<str>);

/// Why a typed title was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidTitle {
    /// Nothing but white space.
    Empty,
    /// A newline, a tab or another control character.
    ControlCharacter,
    /// More than [`SessionTitle::MAX_CHARS`] characters (after trimming).
    TooLong {
        /// How many it has.
        chars: usize,
    },
}

impl fmt::Display for InvalidTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a title cannot be empty"),
            Self::ControlCharacter => {
                f.write_str("a title is one line, without tabs or other control characters")
            }
            Self::TooLong { chars } => write!(
                f,
                "a title is at most {} characters, not {chars}",
                SessionTitle::MAX_CHARS
            ),
        }
    }
}

impl std::error::Error for InvalidTitle {}

impl SessionTitle {
    /// The longest title a person may give, in characters (Paseo's
    /// `MAX_EXPLICIT_AGENT_TITLE_CHARS`; 120 before TRACKER 801).
    pub const MAX_CHARS: usize = 200;

    /// The longest title a peer before [`LONG_TITLES_PROTOCOL`] decodes.
    pub const LEGACY_MAX_CHARS: usize = 120;

    /// The longest title a client may send to, and reads from, a peer
    /// speaking `protocol`: what a rename or a create field allows.
    #[must_use]
    pub const fn max_chars_for(protocol: ProtocolVersion) -> usize {
        match LONG_TITLES_PROTOCOL {
            Some(since) if protocol.get() >= since.get() => Self::MAX_CHARS,
            Some(_) | None => Self::LEGACY_MAX_CHARS,
        }
    }

    /// Whether a peer before [`LONG_TITLES_PROTOCOL`] could not decode it.
    #[must_use]
    pub fn is_long(&self) -> bool {
        self.0.chars().count() > Self::LEGACY_MAX_CHARS
    }

    /// This title as a peer speaking `protocol` decodes it: for one before
    /// [`LONG_TITLES_PROTOCOL`], its first
    /// [`LEGACY_MAX_CHARS`](Self::LEGACY_MAX_CHARS) characters, trimmed;
    /// `None` when it reads the title as it is.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        let most = Self::max_chars_for(protocol);
        if self.0.chars().count() <= most {
            return None;
        }
        let cut: String = self.0.chars().take(most).collect();
        Some(Self(Arc::from(cut.trim_end())))
    }

    /// [`for_protocol`](Self::for_protocol) of an optional title: the
    /// field's new value, or `None` when it stays as it is.
    #[must_use]
    pub fn optional_for_protocol(
        title: Option<&Self>,
        protocol: ProtocolVersion,
    ) -> Option<Option<Self>> {
        title
            .and_then(|title| title.for_protocol(protocol))
            .map(Some)
    }

    /// Where a title taken from a prompt is cut, in characters (Paseo's
    /// `MAX_INITIAL_AGENT_TITLE_CHARS`, 60; it was 80 before TRACKER 801).
    pub const FROM_PROMPT_CHARS: usize = 60;

    /// The title a prompt gives an untitled session (Paseo's
    /// `deriveInitialAgentTitle`): its first non-empty line, each run of
    /// white space or control characters (tabs) made one space, cut at
    /// [`FROM_PROMPT_CHARS`](Self::FROM_PROMPT_CHARS) characters with no
    /// ellipsis, trimmed. `None` when the text has no such line.
    #[must_use]
    pub fn from_prompt(text: &str) -> Option<Self> {
        let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
        let folded = line
            .split(|character: char| character.is_whitespace() || character.is_control())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let cut: String = folded.chars().take(Self::FROM_PROMPT_CHARS).collect();
        let title = cut.trim_end();
        (!title.is_empty()).then(|| Self(Arc::from(title)))
    }

    /// The title.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for SessionTitle {
    type Err = InvalidTitle;

    /// A typed title, trimmed; refused rather than changed otherwise.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let title = text.trim();
        if title.is_empty() {
            return Err(InvalidTitle::Empty);
        }
        if title.chars().any(char::is_control) {
            return Err(InvalidTitle::ControlCharacter);
        }
        let chars = title.chars().count();
        if chars > Self::MAX_CHARS {
            return Err(InvalidTitle::TooLong { chars });
        }
        Ok(Self(Arc::from(title)))
    }
}

impl fmt::Display for SessionTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

crate::key::serde_via_text!(SessionTitle);

/// Who produced the session's current title (ADR 0062). Absent on the wire
/// decodes as [`Unknown`](Self::Unknown): a record from before generated
/// names, or an origin this build does not know. Generation replaces only
/// [`Given`](Self::Given).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum TitleOrigin {
    /// A person wrote it (`SessionSpec.title` or `session.rename`).
    Typed,
    /// The node cut it from text (`SessionTitle::from_prompt`), including a
    /// fork's derived title.
    Given,
    /// The generator's title, after it parsed.
    Generated,
    /// Not recorded, or an origin added later. Not replaced.
    #[default]
    #[serde(other)]
    Unknown,
}

impl TitleOrigin {
    /// Whether no reliable producer was recorded for an older title.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Hide provenance only at a negotiated wire edge. Stored JSON keeps
    /// it even while the merge-assigned threshold is absent.
    #[must_use]
    pub(crate) const fn for_protocol(
        self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Self {
        match assigned {
            Some(since) if protocol.get() >= since.get() => self,
            Some(_) | None => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_title_is_one_trimmed_line() {
        let title: SessionTitle = "  Fix the build ".parse().unwrap();
        assert_eq!(title.as_str(), "Fix the build");
        assert_eq!(" \t ".parse::<SessionTitle>(), Err(InvalidTitle::Empty));
        assert_eq!(
            "two\nlines".parse::<SessionTitle>(),
            Err(InvalidTitle::ControlCharacter)
        );
        assert_eq!(
            "a\tb".parse::<SessionTitle>(),
            Err(InvalidTitle::ControlCharacter)
        );
        let longest = "가".repeat(SessionTitle::MAX_CHARS);
        assert!(
            longest.parse::<SessionTitle>().is_ok(),
            "characters, not bytes"
        );
        assert_eq!(
            "a".repeat(201).parse::<SessionTitle>(),
            Err(InvalidTitle::TooLong { chars: 201 })
        );
        assert!(
            "a".repeat(150).parse::<SessionTitle>().is_ok(),
            "Paseo's 200, not the old 120"
        );
    }

    #[test]
    fn a_client_reads_its_limit_from_the_peers_protocol() {
        let long_titles = LONG_TITLES_PROTOCOL.expect("drafted");
        assert_eq!(
            SessionTitle::max_chars_for(long_titles),
            SessionTitle::MAX_CHARS
        );
        assert_eq!(
            SessionTitle::max_chars_for(crate::PROTOCOL_VERSION),
            SessionTitle::MAX_CHARS
        );
        let older = ProtocolVersion::new(long_titles.get().saturating_sub(1));
        assert_eq!(
            SessionTitle::max_chars_for(older),
            SessionTitle::LEGACY_MAX_CHARS
        );
        assert!(long_titles <= crate::PROTOCOL_VERSION);
    }

    #[test]
    fn an_older_peer_gets_a_long_title_cut_to_what_it_decodes() {
        let long_titles = LONG_TITLES_PROTOCOL.expect("drafted");
        let older = ProtocolVersion::new(long_titles.get().saturating_sub(1));
        let long: SessionTitle = format!("{} tail", "가".repeat(147)).parse().unwrap();
        assert_eq!(long.as_str().chars().count(), 152);
        assert!(long.is_long());
        assert_eq!(
            long.for_protocol(long_titles),
            None,
            "a current peer reads it whole"
        );
        let cut = long.for_protocol(older).unwrap();
        assert_eq!(cut.as_str().chars().count(), SessionTitle::LEGACY_MAX_CHARS);
        assert!(!cut.is_long());
        // A cut ending in a blank drops it, so the title stays trimmed.
        let spaced: SessionTitle = format!("{} {}", "a".repeat(119), "b".repeat(30))
            .parse()
            .unwrap();
        assert_eq!(
            spaced.for_protocol(older).unwrap().as_str(),
            "a".repeat(119)
        );
        let short: SessionTitle = "Fix the build".parse().unwrap();
        assert_eq!(short.for_protocol(older), None);
        assert_eq!(
            SessionTitle::optional_for_protocol(Some(&long), older),
            Some(Some(cut))
        );
        assert_eq!(SessionTitle::optional_for_protocol(None, older), None);
    }

    #[test]
    fn a_bad_title_does_not_decode() {
        let title: SessionTitle = serde_json::from_str(r#""Release notes""#).unwrap();
        assert_eq!(serde_json::to_string(&title).unwrap(), r#""Release notes""#);
        assert!(serde_json::from_str::<SessionTitle>(r#""""#).is_err());
        assert!(serde_json::from_str::<SessionTitle>(r#""a\nb""#).is_err());
    }

    #[test]
    fn a_prompt_gives_its_first_line_folded_and_cut_at_sixty() {
        let title = SessionTitle::from_prompt("\n\n  Fix the failing test\tin ci  \nthen push");
        assert_eq!(
            title.as_ref().map(SessionTitle::as_str),
            Some("Fix the failing test in ci")
        );
        // Runs of blanks and tabs fold to one space, as Paseo's `\s+`.
        let folded = SessionTitle::from_prompt("Fix   the\t\t build\u{2003}now").unwrap();
        assert_eq!(folded.as_str(), "Fix the build now");
        // Cut at 60 with no ellipsis, the space before the cut dropped.
        let words = SessionTitle::from_prompt(&"word ".repeat(20)).unwrap();
        assert_eq!(words.as_str(), "word ".repeat(12).trim_end());
        assert!(!words.as_str().ends_with('…'));
        assert_eq!(SessionTitle::from_prompt("  \n \r\n"), None);
        let long = SessionTitle::from_prompt(&"한".repeat(100)).unwrap();
        assert_eq!(
            long.as_str().chars().count(),
            SessionTitle::FROM_PROMPT_CHARS
        );
        // What a prompt gives is always a title a person could type.
        assert_eq!(long.as_str().parse::<SessionTitle>(), Ok(long));
    }

    /// An origin this build does not name is unknown, and unknown is the
    /// field's absence. A known origin round-trips.
    #[test]
    fn an_absent_origin_is_unknown_and_a_known_one_round_trips() {
        assert_eq!(
            serde_json::from_str::<TitleOrigin>("\"later\"").unwrap(),
            TitleOrigin::Unknown
        );
        assert_eq!(
            serde_json::from_str::<TitleOrigin>("\"given\"").unwrap(),
            TitleOrigin::Given
        );
        assert_eq!(
            serde_json::to_string(&TitleOrigin::Generated).unwrap(),
            "\"generated\""
        );
        assert!(TitleOrigin::is_unknown(&TitleOrigin::Unknown));
        assert!(!TitleOrigin::is_unknown(&TitleOrigin::Typed));
    }

    #[test]
    fn renames_from_older_nodes_and_to_older_clients_keep_the_title() {
        use crate::{ProtocolVersion, SessionEvent};
        #[derive(serde::Deserialize, serde::Serialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum OldRename {
            Renamed { title: SessionTitle },
        }
        let old = r#"{"type":"renamed","title":"My title"}"#;
        let read: SessionEvent = serde_json::from_str(old).unwrap();
        assert!(matches!(
            read,
            SessionEvent::Renamed {
                origin: TitleOrigin::Unknown,
                ..
            }
        ));
        let event = SessionEvent::Renamed {
            title: "My title".parse().unwrap(),
            origin: TitleOrigin::Generated,
        };
        assert_eq!(event.required_protocol(), Some(ProtocolVersion::new(9)));
        assert_eq!(
            event.origin_for_protocol(ProtocolVersion::new(9), Some(ProtocolVersion::new(10))),
            Some(read.clone())
        );
        let encoded = serde_json::to_string(&event).unwrap();
        let older: OldRename = serde_json::from_str(&encoded).unwrap();
        let OldRename::Renamed { title } = &older;
        assert_eq!(title.as_str(), "My title");
        // A typed older hop drops an additive reply field when it encodes
        // again. Unknown origin keeps the surviving title ineligible.
        let forwarded = serde_json::to_string(&older).unwrap();
        let after_hop: SessionEvent = serde_json::from_str(&forwarded).unwrap();
        assert_eq!(after_hop, read);
        assert_eq!(
            serde_json::from_str::<SessionEvent>(&encoded).unwrap(),
            event
        );
        assert!(!serde_json::to_string(&read).unwrap().contains("origin"));
    }

    fn titled_info() -> crate::SessionInfo {
        let spec = crate::SessionSpec::new(
            crate::HarnessKind::Codex,
            crate::SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_title("My title".parse().unwrap());
        crate::SessionInfo::new(crate::SessionId::random(), spec, chrono::Utc::now())
    }

    #[test]
    fn negotiated_origins_are_hidden_until_assigned_but_storage_keeps_them() {
        use crate::SessionEvent;
        let event = SessionEvent::Renamed {
            title: "My title".parse().unwrap(),
            origin: TitleOrigin::Generated,
        };
        let info = titled_info();
        // Durable JSON is independent of wire negotiation.
        assert!(serde_json::to_string(&event).unwrap().contains("origin"));
        assert!(
            serde_json::to_string(&info)
                .unwrap()
                .contains("title_origin")
        );
        let protocol = crate::PROTOCOL_VERSION;
        let old_event = event
            .for_protocol(protocol)
            .unwrap_or_else(|| event.clone());
        let old_info = info.for_protocol(protocol).unwrap_or_else(|| info.clone());
        assert_eq!(
            serde_json::to_string(&old_event)
                .unwrap()
                .contains("origin"),
            crate::speaks_generated_names()
        );
        assert_eq!(
            serde_json::to_string(&old_info)
                .unwrap()
                .contains("title_origin"),
            crate::speaks_generated_names()
        );
        let assigned = Some(protocol);
        assert_eq!(event.origin_for_protocol(protocol, assigned), None);
        assert_eq!(info.for_protocol_with_names(protocol, assigned), None);
        for threshold in [None, assigned] {
            let old = crate::MIN_PROTOCOL_VERSION;
            let event = event.origin_for_protocol(old, threshold).unwrap();
            let info = info.for_protocol_with_names(old, threshold).unwrap();
            assert!(!serde_json::to_string(&event).unwrap().contains("origin"));
            assert!(
                !serde_json::to_string(&info)
                    .unwrap()
                    .contains("title_origin")
            );
        }
    }

    #[test]
    fn an_old_session_info_is_unknown_even_when_its_spec_has_a_title() {
        use crate::SessionInfo;
        let mut info = titled_info();
        assert_eq!(info.title_origin, TitleOrigin::Typed);
        // Removing the field models a pre-origin node without inferring
        // who authored a title from its text or the create specification.
        info.title_origin = TitleOrigin::Unknown;
        let encoded = serde_json::to_string(&info).unwrap();
        assert!(!encoded.contains("title_origin"));
        let read: SessionInfo = serde_json::from_str(&encoded).unwrap();
        assert_eq!(read.title_origin, TitleOrigin::Unknown);
        assert_eq!(read.title, info.title);
    }
}
