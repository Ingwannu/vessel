//! A running session's option changes, as Paseo's composer makes them on a
//! running agent (`packages/app/src/composer/agent-controls/index.tsx`,
//! `AgentControls`; `mode-control.tsx`, `useLiveAgentModeControl`).
//!
//! What Paseo does, and so what this decides:
//!
//! - A choice is sent at once (`setAgentModel`, `setAgentThinkingOption`,
//!   `setAgentMode`, `setAgentFeature`). Nothing is blocked while the agent
//!   is in a turn: whether a change applies mid-turn is the agent's to say,
//!   and its refusal is what the person sees.
//! - The control keeps showing what the agent reports; the new value shows
//!   when the agent's own report of its options arrives.
//! - Each choice is also kept as that agent's preference for new chats
//!   (`mergeProviderPreferences`): the model once the agent took it, the
//!   effort, the mode and a feature as they are sent ([`KeepAt`]).
//! - A refusal is an error toast ([`Refusal`]): the app's sentence for the
//!   machine's answer, never the agent's own words in its body (the
//!   coordinator's rule for original text: those belong in a tooltip or a
//!   copy, which the toast does not have yet).
//!
//! [Decision Log]
//! - Purpose: the coordinator's row a (change the model, effort or
//!   permissions of a running session), at Paseo's behaviour.
//! - Constraint: the link answers an act with its kind only (`ActKind::
//!   SetOption`), not with the choice it was for, and runs every act on a
//!   task of its own, so two changes in flight on one session can come back
//!   in either order.
//! - Alternatives: one change at a time per session, the next waiting for
//!   the first (Paseo sends at once; the page disables its menus instead,
//!   which Paseo does not); the act's answer carrying its choice (a change
//!   to the link's answer type for every act).
//! - Choice: changes in flight are matched to answers oldest first, per
//!   session ([`InFlight`]), as the board matches its acts.
//! - Cost: with two changes in flight on one session, a refusal of the
//!   later one can be taken for the earlier; only what is kept as the
//!   default after a model change can then be wrong, and the next start of
//!   a chat shows it in its pill before anything is sent.

use std::collections::VecDeque;

use vessel_proto::{
    ErrorCode, HarnessRef, OptionCategory, OptionChoice, OptionId, SessionId, SessionOption,
};

use crate::localization::{ComposerOptionRefusedReason, FormatError, Translator};

/// When a change on a running session becomes its agent's default for new
/// chats (Paseo's `AgentControls`: the model is kept after `setAgentModel`
/// resolves; a thinking option, a mode and a feature before they are sent,
/// whatever the agent answers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAt {
    /// Kept as it is sent.
    Sending,
    /// Kept once the agent took it.
    Taken,
}

/// When a change to option `id` is kept, from what the session reported of
/// its options: the model once taken, anything else as it is sent (an
/// option the session no longer reports too: it is sent as asked, and kept
/// as Paseo keeps a feature).
#[must_use]
pub fn keep_at(reported: &[SessionOption], id: &OptionId) -> KeepAt {
    let category = reported
        .iter()
        .find(|option| &option.id == id)
        .map(|option| option.category);
    match category {
        Some(OptionCategory::Model) => KeepAt::Taken,
        Some(
            OptionCategory::Mode
            | OptionCategory::ModelConfig
            | OptionCategory::ThoughtLevel
            | OptionCategory::Other,
        )
        | None => KeepAt::Sending,
    }
}

/// The agent's kept choices with `choice` in them: an earlier choice of the
/// same option is replaced where it stood, a new one is added at the end
/// (Paseo merges one field into the provider's preferences; the others
/// stay).
#[must_use]
pub fn kept_with(kept: Option<&[OptionChoice]>, choice: &OptionChoice) -> Vec<OptionChoice> {
    let mut merged: Vec<OptionChoice> = kept.map(<[OptionChoice]>::to_vec).unwrap_or_default();
    match merged.iter_mut().find(|earlier| earlier.id == choice.id) {
        Some(earlier) => earlier.clone_from(choice),
        None => merged.push(choice.clone()),
    }
    merged
}

/// One change sent to a running session, waiting for its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The machine the session is on.
    pub machine: String,
    /// The session.
    pub session: SessionId,
    /// The session's agent, whose default the change may become.
    pub agent: HarnessRef,
    /// What was asked.
    pub choice: OptionChoice,
    /// When it becomes the agent's default.
    pub keep: KeepAt,
}

/// The changes sent and not yet answered, oldest first.
///
/// Bounded: past [`InFlight::MAX`] the oldest is forgotten (its answer then
/// still toasts a refusal, it only keeps nothing). Every act is answered
/// (the link settles one it could not send), so the list empties by itself.
#[derive(Debug, Default)]
pub struct InFlight {
    changes: VecDeque<Change>,
}

impl InFlight {
    /// How many unanswered changes are remembered. A person picks one value
    /// at a time from a list; sixteen in flight already means the machine
    /// has stopped answering.
    pub const MAX: usize = 16;

    /// Remember `change` until its answer; the oldest is forgotten when
    /// the list is full.
    pub fn sent(&mut self, change: Change) {
        if self.changes.len() >= Self::MAX {
            let _forgotten = self.changes.pop_front();
        }
        self.changes.push_back(change);
    }

    /// The change an answer for `session` on `machine` settles: the oldest
    /// sent to it.
    pub fn answered(&mut self, machine: &str, session: SessionId) -> Option<Change> {
        let at = self
            .changes
            .iter()
            .position(|change| change.machine == machine && change.session == session)?;
        self.changes.remove(at)
    }

    /// How many changes wait for their answer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Whether no change waits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Why a running session did not take a change, by the node's code, each
/// with a sentence of its own (the toast's words).
///
/// To move into the shared mapping of the node's codes to sentences (the
/// lead's slice, with the conversation bar's words); until then this small
/// table is the composer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The agent refused it in the state it is in (mid-turn, or a change
    /// it does not make now).
    Busy,
    /// The option or its value is no longer offered.
    NotOffered,
    /// This device may only read the machine.
    ReadOnly,
    /// The session is no longer running there.
    Gone,
    /// The session's agent takes no change while it runs.
    Unsupported,
    /// The machine is stopping, or the app could not send it (its queue
    /// full or stopped); again later.
    Unavailable,
    /// Anything else (the connection failed, the machine failed).
    Other,
}

impl Refusal {
    /// The refusal a failed act's code means (`None`: the node never
    /// answered).
    #[must_use]
    pub const fn of(code: Option<ErrorCode>) -> Self {
        let Some(code) = code else {
            return Self::Other;
        };
        match code {
            ErrorCode::Conflict => Self::Busy,
            ErrorCode::InvalidRequest => Self::NotOffered,
            ErrorCode::Forbidden => Self::ReadOnly,
            ErrorCode::NotFound => Self::Gone,
            ErrorCode::Unsupported => Self::Unsupported,
            ErrorCode::Unavailable => Self::Unavailable,
            ErrorCode::Unauthorized
            | ErrorCode::ProtocolMismatch
            | ErrorCode::HarnessNotInstalled
            | ErrorCode::SpawnFailed
            | ErrorCode::Internal => Self::Other,
        }
    }

    /// The toast's whole sentence.
    ///
    /// # Errors
    /// The catalog could not format the message (its English fallback
    /// too).
    pub fn words(self, catalog: &Translator) -> Result<String, FormatError> {
        let reason = match self {
            Self::Busy => ComposerOptionRefusedReason::Busy,
            Self::NotOffered => ComposerOptionRefusedReason::NotOffered,
            Self::ReadOnly => ComposerOptionRefusedReason::ReadOnly,
            Self::Gone => ComposerOptionRefusedReason::Gone,
            Self::Unsupported => ComposerOptionRefusedReason::Unsupported,
            Self::Unavailable => ComposerOptionRefusedReason::Unavailable,
            Self::Other => ComposerOptionRefusedReason::Other,
        };
        catalog.composer().option_refused(reason)
    }
}

#[cfg(test)]
mod tests;
