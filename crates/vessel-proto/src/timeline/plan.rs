//! Typed provider proposal/decision seam. These values never grant execution authority.
use super::{ExternalThreadId, ExternalTurnId, PlanProposalId};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

/// A complete proposal stays below retained presentation and frame budgets.
pub const MAX_PLAN_TEXT_BYTES: usize = 65_536;

/// Normalized provider plan Markdown, independent of any approval outcome.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PlanText(Arc<str>);

/// Invalid plans cannot become actionable approvals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("plan must contain text and fit the plan byte budget")]
pub struct InvalidPlanText;

impl PlanText {
    /// Normalize only outer ECMAScript whitespace, preserving the Markdown body.
    /// # Errors
    /// Empty normalized content or more than `MAX_PLAN_TEXT_BYTES` UTF-8 bytes.
    pub fn new(text: &str) -> Result<Self, InvalidPlanText> {
        let text = crate::trim_ecmascript(text);
        if text.is_empty() || text.len() > MAX_PLAN_TEXT_BYTES {
            return Err(InvalidPlanText);
        }
        Ok(Self(Arc::from(text)))
    }
    /// The normalized Markdown, for provider-owned follow-up construction and display.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for PlanText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PlanText([redacted])")
    }
}
impl TryFrom<String> for PlanText {
    type Error = InvalidPlanText;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::new(&text)
    }
}
impl From<PlanText> for String {
    fn from(text: PlanText) -> Self {
        text.0.to_string()
    }
}

/// Legacy turn plans have no native item identity; no id is fabricated for them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanProposalSource {
    /// A completed native Plan item.
    CompletedItem {
        /// Its actual provider-assigned item spelling.
        item: PlanProposalId,
    },
    /// The actual root turn's legacy `turn/plan/updated` report.
    LegacyTurnPlan,
}

/// Correlation alone; the session actor owns the human-facing permission id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlanProposalKey {
    /// Actual provider conversation.
    pub thread: ExternalThreadId,
    /// Actual successful root turn.
    pub turn: ExternalTurnId,
    /// Native item identity or explicitly legacy per-turn source.
    pub source: PlanProposalSource,
}

/// Resume is offered only when the owning provider can execute that transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlanResumeAvailability {
    /// No supported resume transition in the captured provider state.
    Unavailable,
    /// The provider explicitly supports resuming this plan.
    Available,
}

/// Dedicated producer output; clients never infer this from tool labels or Markdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PlanProposal {
    /// Stable source identity.
    pub key: PlanProposalKey,
    /// Complete bounded proposal.
    pub text: PlanText,
    /// Actual provider capability for this proposal.
    pub resume: PlanResumeAvailability,
}

/// Exact provider-owned decision, independent of option label spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum PlanDecision {
    /// Leave the plan unimplemented.
    Dismiss,
    /// Change native collaboration mode and implement the approved plan.
    Implement,
    /// Use the provider's supported resume transition, then implement the plan.
    ImplementResume,
}

/// The actor maps its exact owned offered option id to this typed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecidePlan {
    /// Proposal whose ownership and state the provider must validate again.
    pub proposal: PlanProposalKey,
    /// Explicit selected decision.
    pub decision: PlanDecision,
}

/// Refusal before an implementation prompt or mode transition takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanDecisionRefusal {
    /// Superseded, absent or already settled proposal.
    StaleProposal,
    /// Resume is unavailable for this exact proposal.
    ResumeUnavailable,
    /// A different foreground turn owns the provider now.
    TurnChanged,
    /// This provider cannot perform plan decisions.
    Unsupported,
    /// The native provider state cannot accept the requested transition.
    InvalidState,
}

/// Never acknowledge a settled human action before native follow-up acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanDecisionReply {
    /// The actual transition and provider-built follow-up were accepted (or dismissed).
    Accepted {
        /// Exact accepted proposal.
        proposal: PlanProposalKey,
        /// Exact accepted decision.
        decision: PlanDecision,
    },
    /// No external effect was attempted. StaleProposal/TurnChanged withdraw the expired
    /// capability; other refusals leave the pending proposal available for a new decision.
    Refused {
        /// Exact proposal refused.
        proposal: PlanProposalKey,
        /// Typed reason.
        reason: PlanDecisionRefusal,
    },
    /// A transition/follow-up may have happened. Retain uncertainty and never resend.
    OutcomeUnknown {
        /// Exact protected proposal.
        proposal: PlanProposalKey,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plans_normalize_only_edges_and_enforce_the_serialized_identity_boundaries() {
        assert_eq!(
            PlanText::new("\u{feff} # Plan\n\nbody \n")
                .unwrap()
                .as_str(),
            "# Plan\n\nbody"
        );
        assert!(PlanText::new("\u{feff} \n").is_err());
        assert!(PlanText::new(&"a".repeat(MAX_PLAN_TEXT_BYTES)).is_ok());
        assert!(PlanText::new(&"a".repeat(MAX_PLAN_TEXT_BYTES.saturating_add(1))).is_err());
        assert!(serde_json::from_str::<PlanText>("\"\"").is_err());
        assert!(!format!("{:?}", PlanText::new("private plan").unwrap()).contains("private plan"));
        assert!("".parse::<PlanProposalId>().is_err());
    }
}
