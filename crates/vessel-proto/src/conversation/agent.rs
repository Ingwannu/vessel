//! What an agent says about itself and its session (protocol 10): what it
//! can do, the commands it offers, and the tokens it reports.
//!
//! Numbers are passed on as the agent reports them over ACP
//! (`vessel-agent` reads them). The runtime only adds whether it observed
//! context during conversation loading; it never changes the numbers.

use serde::{Deserialize, Serialize};

use super::text::{CommandName, CostUnit};
use crate::model::TokenCount;

/// What an agent can do, from its ACP `initialize` answer.
///
/// [Decision Log]
/// - Purpose: steering, starting again and attaching images are offered
///   only where the agent takes them, and a client must know before it
///   offers the control, not from a failure after.
/// - Choice: read once per agent start (`initialize`), logged with the
///   session's `ready` and kept by the store, so an ended session still
///   says whether it can start again. Files are not here: every
///   structured agent takes a resource link, ACP's baseline content.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent facts from the agent's handshake, each read on its own; \
              no combination is invalid, so there is no state machine to make an enum of"
)]
pub struct AgentAbilities {
    /// A message can be added to a running turn: the agent's answer
    /// carries `_meta.steering.supported: true` (the ACP steering
    /// extension, `_session/steering`).
    #[serde(default)]
    pub steer: bool,
    /// An ended conversation can be loaded again (`loadSession`).
    #[serde(default)]
    pub resume: bool,
    /// A prompt may hold images (`promptCapabilities.image`).
    #[serde(default)]
    pub image: bool,
    /// A prompt may hold audio (`promptCapabilities.audio`).
    #[serde(default)]
    pub audio: bool,
    /// The conversation can be forked (`sessionCapabilities.fork`, ACP's
    /// unstable `session/fork`; protocol 14, ADR 0052). Absent from older
    /// nodes and sessions started before 14: a fork's own handshake
    /// decides, not this.
    #[serde(default)]
    pub fork: bool,
}

/// One command the agent offers (ACP `available_commands_update`). A
/// client uses it by sending a prompt that starts with `/name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgentCommand {
    /// Its name, without the `/`.
    pub name: CommandName,
    /// What it does, in the agent's words (cut at
    /// [`AgentCommand::MAX_DESCRIPTION`] bytes by the node).
    pub description: String,
    /// What to type after it, when it takes input (cut at
    /// [`AgentCommand::MAX_HINT`] bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Explicit command classification (ADR 0085); absent from older reports.
    /// Only a Skill may be completed inline; unknown and absent remain root-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<crate::composer_commands::CommandKind>,
}

impl AgentCommand {
    /// The most commands a session keeps; an agent offering more has the
    /// rest left out (and logged).
    pub const MAX_COMMANDS: usize = 512;
    /// The longest description kept, in bytes.
    pub const MAX_DESCRIPTION: usize = 1024;
    /// The longest hint kept, in bytes.
    pub const MAX_HINT: usize = 256;
}

/// A JSON number that is finite: no NaN or infinity, so the types holding
/// it compare as equal or not (`Eq`), and it always encodes as JSON.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(transparent)]
pub struct FiniteNumber(f64);

impl Eq for FiniteNumber {}

impl FiniteNumber {
    /// `value`, unless it is NaN or infinite.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FiniteNumber {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| serde::de::Error::custom("a number must be finite"))
    }
}

/// A cost, as the agent counts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Cost {
    /// How much.
    pub amount: FiniteNumber,
    /// In what.
    pub unit: CostUnit,
}

/// How full the session's context is, from the agent's last
/// `usage_update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContextUsage {
    /// Whether the runtime observed this while loading an existing
    /// conversation (protocol 21). Context occupancy remains useful;
    /// a replayed cumulative cost is not new spending.
    #[serde(default, skip_serializing_if = "UsageObservation::is_current")]
    pub observation: UsageObservation,
    /// Tokens in the context now.
    pub used: TokenCount,
    /// The context window's size.
    pub size: TokenCount,
    /// What the session has cost so far, when the agent says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,
}

/// The runtime's observation boundary, independent of the vendor's numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum UsageObservation {
    /// Observed outside conversation loading; the default for older records.
    #[default]
    Current,
    /// Observed while loading, before the runtime's replay boundary ended.
    Replayed,
}

impl UsageObservation {
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's skip_serializing_if predicate receives a reference to the field"
    )]
    fn is_current(&self) -> bool {
        *self == Self::Current
    }
}

impl ContextUsage {
    pub(crate) fn for_protocol(&self, protocol: crate::ProtocolVersion) -> Option<Self> {
        (protocol < crate::usage::USAGE_PROTOCOL_VERSION
            && self.observation == UsageObservation::Replayed)
            .then(|| Self {
                observation: UsageObservation::Current,
                ..self.clone()
            })
    }
}

/// One turn's tokens, as the agent reports them with the turn's end (the
/// prompt's answer `usage`).
///
/// Not [`TokenUsage`](crate::TokenUsage): that is the node proxy's own
/// count, with defined cache rules; agents disagree on whether `input`
/// counts cached tokens, so this keeps their words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TurnUsage {
    /// Every token of the turn.
    pub total: TokenCount,
    /// Input tokens.
    pub input: TokenCount,
    /// Output tokens.
    pub output: TokenCount,
    /// Reasoning tokens, when counted apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought: Option<TokenCount>,
    /// Tokens read from a cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_read: Option<TokenCount>,
    /// Tokens written to a cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_write: Option<TokenCount>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_finite_numbers_decode() {
        assert_eq!(
            serde_json::from_str::<FiniteNumber>("1.25").unwrap(),
            FiniteNumber::new(1.25).unwrap()
        );
        assert!(FiniteNumber::new(f64::NAN).is_none());
        assert!(FiniteNumber::new(f64::INFINITY).is_none());
        assert!(serde_json::from_str::<FiniteNumber>("1e999").is_err());
    }

    #[test]
    fn usage_cost_numbers_survive_json_round_trips() {
        for amount in [1e-29, 2e-29, 1e-28, 0.1, 10_000_000_000_000_002.0] {
            let usage = ContextUsage {
                observation: UsageObservation::Current,
                used: TokenCount::new(1200),
                size: TokenCount::new(200_000),
                cost: Some(Cost {
                    amount: FiniteNumber::new(amount).unwrap(),
                    unit: "USD".parse().unwrap(),
                }),
            };
            let encoded = serde_json::to_string(&usage).unwrap();
            assert_eq!(
                serde_json::from_str::<ContextUsage>(&encoded).unwrap(),
                usage
            );
        }
    }

    #[test]
    fn abilities_default_to_nothing() {
        let abilities: AgentAbilities = serde_json::from_str("{}").unwrap();
        assert_eq!(abilities, AgentAbilities::default());
        let usage = ContextUsage {
            observation: UsageObservation::Current,
            used: TokenCount::new(41_200),
            size: TokenCount::new(200_000),
            cost: Some(Cost {
                amount: FiniteNumber::new(0.5).unwrap(),
                unit: "USD".parse().unwrap(),
            }),
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert_eq!(
            json,
            r#"{"used":41200,"size":200000,"cost":{"amount":0.5,"unit":"USD"}}"#
        );
        assert_eq!(serde_json::from_str::<ContextUsage>(&json).unwrap(), usage);
    }

    #[test]
    fn usage_observation_is_additive_and_replayed_reports_are_projected_for_older_peers() {
        let current: ContextUsage = serde_json::from_str(r#"{"used":1200,"size":200000}"#).unwrap();
        assert_eq!(current.observation, UsageObservation::Current);
        assert_eq!(
            serde_json::to_string(&current).unwrap(),
            r#"{"used":1200,"size":200000}"#
        );
        let replayed = ContextUsage {
            observation: UsageObservation::Replayed,
            ..current.clone()
        };
        let json = r#"{"observation":"replayed","used":1200,"size":200000}"#;
        assert_eq!(serde_json::to_string(&replayed).unwrap(), json);
        assert_eq!(
            serde_json::from_str::<ContextUsage>(json).unwrap(),
            replayed
        );
        assert_eq!(
            replayed.for_protocol(crate::ProtocolVersion::new(18)),
            Some(current.clone())
        );
        assert_eq!(replayed.for_protocol(crate::PROTOCOL_VERSION), None);
        let event = crate::SessionEvent::UsageChanged {
            usage: replayed.clone(),
        };
        assert_eq!(
            event.for_protocol(crate::ProtocolVersion::new(18)),
            Some(crate::SessionEvent::UsageChanged {
                usage: current.clone()
            })
        );
        assert_eq!(event.for_protocol(crate::PROTOCOL_VERSION), None);
        assert_eq!(
            event.for_protocol(crate::ProtocolVersion::new(9)),
            Some(crate::SessionEvent::Agent {
                event: crate::AgentEvent::HarnessLog {
                    line: "vessel: context: 1200 of 200000 tokens".to_owned()
                }
            })
        );
        let conversation = crate::Conversation {
            queue: crate::PromptQueue::default(),
            commands: Vec::new(),
            usage: Some(replayed),
            permissions: Vec::new(),
            questions: Vec::new(),
            abilities: None,
            limits: crate::conversation::AttachmentLimits::DEFAULT,
        };
        let mut older = conversation.clone();
        older.usage = Some(current);
        let reply = crate::Reply::Deliver(crate::forward::Forwarded {
            reply: Box::new(crate::Reply::GetConversation(conversation)),
        });
        let expected = crate::Reply::Deliver(crate::forward::Forwarded {
            reply: Box::new(crate::Reply::GetConversation(older)),
        });
        assert_eq!(
            reply.for_protocol(crate::ProtocolVersion::new(18)),
            Some(expected)
        );
        assert_eq!(reply.for_protocol(crate::PROTOCOL_VERSION), None);
    }
}
