//! Paseo's `ProviderRow`/`StatusIndicator` rules. The row has one status,
//! and only a known sign-out changes its installation/chat status.
//!
//! [Decision Log]
//! - Purpose: reproduce the provider title line without repeated verdict
//!   or authentication paragraphs.
//! - Constraints: installation and authentication remain separate node
//!   facts; a model count is known only after options were read.
//! - Alternatives: an installation badge plus sign-in and summary lines.
//! - Choice: one typed row state, a count only on Available, a short error
//!   only on an expanded error row, and a dot/spinner alone when compact.
//! - Why: this is `ProviderRow` and `StatusIndicator`, not the catalogue badge.
//! - Cost: all other sign-in facts are read in the detail page.
use super::{pages::Operation, readiness::AgentBadge};
use crate::badge::Variant;
use crate::{Lang, agent_name, controls_of};
use std::num::NonZeroUsize;
use vessel_proto::agents::{Installed, SignedIn};
use vessel_proto::{HarnessRef, SessionOption};

/// The provider row's responsive presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// Title-line words and known model count.
    Expanded,
    /// The marker alone beside the agent name.
    Compact,
}
/// A pending request concerning this particular agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// No request currently concerns this row.
    Idle,
    /// A list refresh, check or installer concerns this row.
    /// A relevant operation is in flight.
    Loading,
}
impl Activity {
    /// A list refresh concerns every row; a check/install only its agent.
    #[must_use]
    pub fn for_agent(agent: &HarnessRef, operation: Option<&Operation>) -> Self {
        match operation {
            // Provider removal disables its action, not its readiness verdict.
            None | Some(Operation::Remove { .. }) => Self::Idle,
            Some(Operation::Discovery) => Self::Loading,
            Some(Operation::Check(target)) => {
                if target == agent {
                    Self::Loading
                } else {
                    Self::Idle
                }
            }
            Some(Operation::Install(install)) => {
                if agent == &HarnessRef::Known(install.agent) {
                    Self::Loading
                } else {
                    Self::Idle
                }
            }
        }
    }
}
/// A positive count; the reference omits zero rather than saying “0 models”.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCount(NonZeroUsize);
impl ModelCount {
    /// The same model control the composer and defaults use.
    #[must_use]
    pub fn of(lang: Lang, agent: &HarnessRef, options: &[SessionOption]) -> Option<Self> {
        let model = controls_of(options, &agent_name(agent), lang).model?;
        NonZeroUsize::new(model.values.len()).map(Self)
    }
    /// Its numerical value, passed to the shared count translation.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0.get()
    }
}
/// Raw facts at the edge of a row projection; no UI or transport state.
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    /// Identity of the agent the row opens.
    pub agent: &'a HarnessRef,
    /// Installation/chat facts, including unknown checks.
    pub installed: &'a Installed,
    /// Only a known No changes the row; other help belongs in details.
    pub signed_in: Option<&'a SignedIn>,
    /// A previously read options snapshot; None is not a zero count.
    pub options: Option<&'a [SessionOption]>,
    /// Whether an in-flight request concerns this row.
    pub activity: Activity,
}
/// A model count can only be attached to an available row; an error always
/// carries its short, translated explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// Its program can chat; authentication does not claim credentials valid.
    Available {
        /// Its positive known count, omitted until options were read.
        models: Option<ModelCount>,
    },
    /// The program is absent; warning as in the reference provider row.
    NotInstalled,
    /// Chat needs an adapter or a compatible agent version.
    NeedsAttention,
    /// The installation/chat check failed.
    Error {
        /// A short whole sentence, without an instruction hidden in details.
        message: String,
    },
    /// No installation/chat verdict exists.
    Unknown,
    /// The machine explicitly reported that sign-in is needed.
    SignedOut,
    /// A relevant operation is in flight.
    Loading,
}
/// The reference's status marker, independently of translated words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// A stable semantic state.
    Dot,
    /// Neutral activity indicator during a relevant operation.
    Spinner,
}
/// Compact indicators cannot accidentally expose a label or model count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Indicator {
    /// A phone row has only its marker.
    Compact {
        /// Semantic marker colour.
        tone: Variant,
        /// Dot or neutral activity indicator.
        marker: Marker,
    },
    /// A pointer-width row also names the state and known model count.
    Expanded {
        /// Semantic marker colour.
        tone: Variant,
        /// Dot or neutral activity indicator.
        marker: Marker,
        /// Localized short state on the title line.
        label: &'static str,
        /// Whole localized model-count phrase, when known and positive.
        models: Option<String>,
    },
}
/// Derive the one row status from the same readiness used by details.
#[must_use]
pub fn row(lang: Lang, facts: Facts<'_>) -> Row {
    if facts.activity == Activity::Loading {
        return Row::Loading;
    }
    match AgentBadge::of(facts.installed) {
        AgentBadge::Available => match facts.signed_in {
            Some(SignedIn::No { .. }) => Row::SignedOut,
            Some(SignedIn::Yes | SignedIn::NotRequired | SignedIn::Unknown { .. }) | None => {
                Row::Available {
                    models: facts
                        .options
                        .and_then(|options| ModelCount::of(lang, facts.agent, options)),
                }
            }
        },
        AgentBadge::NotInstalled => Row::NotInstalled,
        AgentBadge::NeedsAttention => Row::NeedsAttention,
        AgentBadge::Error => Row::Error {
            message: lang.agents().row_check_failed().to_owned(),
        },
        AgentBadge::Unknown => Row::Unknown,
    }
}
impl Row {
    /// Semantic dot colour: the reference's absent install needs attention.
    #[must_use]
    pub const fn tone(&self) -> Variant {
        match self {
            Self::Available { .. } => Variant::Success,
            Self::NotInstalled | Self::NeedsAttention | Self::SignedOut => Variant::Warning,
            Self::Error { .. } => Variant::Error,
            Self::Unknown | Self::Loading => Variant::Muted,
        }
    }
    /// `StatusIndicator` drops words and the count on compact rows.
    #[must_use]
    pub fn indicator(&self, lang: Lang, density: Density) -> Indicator {
        let marker = if matches!(self, Self::Loading) {
            Marker::Spinner
        } else {
            Marker::Dot
        };
        let tone = self.tone();
        match density {
            Density::Compact => Indicator::Compact { tone, marker },
            Density::Expanded => Indicator::Expanded {
                tone,
                marker,
                label: lang.agents().row_status(self),
                models: match self {
                    Self::Available { models } => {
                        models.map(|count| lang.options().model_count(count.get()))
                    }
                    Self::NotInstalled
                    | Self::NeedsAttention
                    | Self::Error { .. }
                    | Self::Unknown
                    | Self::SignedOut
                    | Self::Loading => None,
                },
            },
        }
    }
    /// `ProviderRow` hides the error under compact rows, and never repeats
    /// a normal state or an unknown sign-in as a paragraph beneath a name.
    #[must_use]
    pub fn error(&self, density: Density) -> Option<&str> {
        match (self, density) {
            (Self::Error { message }, Density::Expanded) => Some(message),
            (
                Self::Available { .. }
                | Self::NotInstalled
                | Self::NeedsAttention
                | Self::Unknown
                | Self::SignedOut
                | Self::Loading,
                Density::Expanded,
            )
            | (_, Density::Compact) => None,
        }
    }
}
#[cfg(test)]
mod tests;
