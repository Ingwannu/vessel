//! Rename validation and correlation are independent from modal widgets and RPCs.
use super::LayoutError;
use vessel_proto::{ErrorCode, SessionId, SessionTitle, TerminalId};
/// Rename forms cannot accept an answer to a cancelled/replaced form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenameTicket(u64);
impl RenameTicket {
    /// Before the first form is opened.
    pub const ZERO: Self = Self(0);
    /// Exhaustion is a refusal, never reuse of an in-flight identity.
    ///
    /// # Errors
    /// Refuses exhausted identity space instead of reusing an outstanding ticket.
    pub fn next(self) -> Result<Self, LayoutError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(LayoutError::Exhausted)
    }
}
/// Renaming a tab changes its backing entity, never its local identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameTarget {
    /// Agent's title on the node.
    Agent(SessionId),
    /// Standalone terminal's title on the node.
    Terminal(TerminalId),
}
/// Only safe structured failure data crosses from the node call into the modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameProblem {
    /// Alias now resolves to a different authenticated node.
    NodeChanged,
    /// No connected node for this target.
    Offline,
    /// The outcome may have changed remotely; a fresh entity read settles it.
    TimedOut,
    /// UI queue is saturated; nothing was sent.
    Busy,
    /// Node or protocol refusal, kept without arbitrary peer text.
    Node(ErrorCode),
    /// An acknowledgement omitted the authoritative title.
    MalformedResponse,
}
/// A submitted title carries the server's validated text type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameSubmission {
    /// Exact form, independent of tab and current navigation.
    pub ticket: RenameTicket,
    /// Entity mutation, not layout metadata.
    pub target: RenameTarget,
    /// Trimmed title validated before any call.
    pub title: SessionTitle,
}
/// Pending form stays open on failure and ignores earlier submissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenamePhase {
    /// User can change and submit text.
    Editing,
    /// One request is outstanding for this form.
    Sending(RenameSubmission),
    /// Retains entered text while the user fixes the refusal.
    Refused(RenameProblem),
    /// Successful entity acknowledgement closes the form.
    Complete,
}
impl RenamePhase {
    /// Only the matching form's reply may change its state.
    pub fn answered(
        &mut self,
        ticket: RenameTicket,
        target: RenameTarget,
        result: Result<(), RenameProblem>,
    ) -> bool {
        let matches = matches!(self,Self::Sending(submission) if submission.ticket==ticket&&submission.target==target);
        if !matches {
            return false;
        }
        *self = match result {
            Ok(()) => Self::Complete,
            Err(problem) => Self::Refused(problem),
        };
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_forms_do_not_accept_a_previous_form_reply_and_failure_stays_editable() {
        let old = RenameTicket::ZERO.next().unwrap();
        let current = old.next().unwrap();
        let target = RenameTarget::Agent(SessionId::random());
        let mut phase = RenamePhase::Sending(RenameSubmission {
            ticket: current,
            target,
            title: "new title".parse().unwrap(),
        });
        assert!(!phase.answered(old, target, Ok(())));
        assert!(matches!(phase, RenamePhase::Sending(_)));
        assert!(phase.answered(current, target, Err(RenameProblem::Offline)));
        assert_eq!(phase, RenamePhase::Refused(RenameProblem::Offline));
    }
}
