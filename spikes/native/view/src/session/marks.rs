//! A session's marks and the actions on them (protocol 10, ADR 0026; the
//! page's `data/marks.ts`): pinned, archived, unread; pin, archive, bring
//! back, start again, delete, mark read or unread; and when an open
//! session marks itself read.
//!
//! What is offered is what the node takes: marks need protocol 10;
//! start again needs an ended structured session whose agent had a
//! session to load and did not say it cannot resume; delete needs an
//! ended session. `read` names the record after the last one shown; the
//! node counts unread by records asking for attention.

use vessel_proto::{EventSeq, ProtocolVersion, SessionInfo, SessionMode};

/// The first protocol with marks, the queue and the rest of a
/// conversation's events.
pub const CONVERSATION_PROTOCOL: ProtocolVersion = ProtocolVersion::new(10);

/// The first protocol on which archiving also ends the terminals and
/// services beside a session.
pub const ENDS_BESIDE_PROTOCOL: ProtocolVersion = ProtocolVersion::new(11);

/// An action on a session's marks or life, in the menu's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkAction {
    /// Pin it.
    Pin,
    /// Unpin it.
    Unpin,
    /// Mark it unread.
    MarkUnread,
    /// Mark it read.
    MarkRead,
    /// Start an ended session again.
    StartAgain,
    /// Archive it (a live one is stopped first; the confirm says so).
    Archive,
    /// Bring it back from the archive.
    Unarchive,
    /// Delete an ended session for good.
    Delete,
}

/// Whether the node keeps `session` archived.
#[must_use]
pub const fn is_archived(session: &SessionInfo) -> bool {
    session.marks.archived_at.is_some()
}

/// Whether `session` is live (starting or running).
#[must_use]
pub const fn is_live(session: &SessionInfo) -> bool {
    session.status.is_live()
}

/// Whether `session` can be started again: ended, structured, with an
/// agent session to load, whose agent did not say it cannot resume. A
/// session stored before protocol 10 has no abilities recorded: the
/// node's answer decides.
#[must_use]
pub fn can_start_again(session: &SessionInfo) -> bool {
    if is_live(session) || !matches!(session.spec.mode, SessionMode::Structured { .. }) {
        return false;
    }
    session.harness_session.is_some()
        && session
            .abilities
            .as_ref()
            .is_none_or(|abilities| abilities.resume)
}

/// The actions `session` offers on a connection speaking `protocol`
/// (`None`: not connected), in the menu's order.
#[must_use]
pub fn mark_actions(session: &SessionInfo, protocol: Option<ProtocolVersion>) -> Vec<MarkAction> {
    if protocol.is_none_or(|protocol| protocol < CONVERSATION_PROTOCOL) {
        return Vec::new();
    }
    let mut actions = vec![
        if session.marks.pinned {
            MarkAction::Unpin
        } else {
            MarkAction::Pin
        },
        if session.marks.unread {
            MarkAction::MarkRead
        } else {
            MarkAction::MarkUnread
        },
    ];
    if can_start_again(session) {
        actions.push(MarkAction::StartAgain);
    }
    actions.push(if is_archived(session) {
        MarkAction::Unarchive
    } else {
        MarkAction::Archive
    });
    if !is_live(session) {
        actions.push(MarkAction::Delete);
    }
    actions
}

/// What an open session knows when it decides whether to mark itself
/// read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadView {
    /// The session is in front: its screen shown, the window focused.
    pub front: bool,
    /// The list says it is unread.
    pub unread: bool,
    /// The last record that asks for attention (`Timeline::attention`).
    pub attention: Option<EventSeq>,
    /// The record after the last one shown (`Timeline::next`).
    pub next: EventSeq,
    /// The last `through` sent; `None` before one.
    pub sent: Option<EventSeq>,
    /// The `next` when the person marked it unread, if they did: no read
    /// until a record comes after it (else the open screen would undo the
    /// person's mark at once).
    pub held_at: Option<EventSeq>,
}

/// The `through` to send now, or `None` for none. The screen sends it at
/// most once a second.
#[must_use]
pub fn read_through(view: &ReadView) -> Option<EventSeq> {
    if !view.front || view.next == EventSeq::ZERO || view.sent.is_some_and(|sent| view.next <= sent)
    {
        return None;
    }
    if view.held_at.is_some_and(|held| view.next <= held) {
        return None;
    }
    let asked_now = view
        .attention
        .is_some_and(|attention| view.sent.is_none_or(|sent| attention >= sent));
    let asked = (view.unread && view.held_at.is_none()) || asked_now;
    asked.then_some(view.next)
}
