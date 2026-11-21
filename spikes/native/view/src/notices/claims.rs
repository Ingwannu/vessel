//! A notice shown only once its machine admits this window as the one
//! recipient (ADR 0089 `notification.claim`; ADR 0114 "Claims"). This is
//! the claims follow-up's second row.
//!
//! [Decision Log]
//! - Purpose: two open windows both notified, which is ADR 0114's stated
//!   difference. Paseo elects one client (`websocket-server.ts:2838-3010`).
//!   With ADR 0089's protocol, a window asks its machine before it shows,
//!   and only the admitted window tells the person.
//! - Constraint: the protocol is unassigned. [`claims`] is false for every
//!   machine until it is, and until then a notice takes ADR 0114's path
//!   unchanged.
//! - Source: a claim names the exact inbox entry, by key and cursor. The
//!   only way to learn them is the inbox listing: newest first, sixteen a
//!   page, no session filter. The window reads the machine's first page
//!   once per change. For each notice it takes the newest unread entry of
//!   that session whose item matches the notice's kind ([`source`]).
//! - Not on the first page (sixteen newer entries came first): the notice
//!   falls back to ADR 0114's path and shows. This is the coordinator's
//!   rule (2026-11-20), and ADR 0089 states the limit. A first page that
//!   cannot be read falls back the same way for every notice, rather than
//!   silencing them (ADR 0089, "Turned on", item 4).
//! - Otherwise only `Admitted` shows ([`shows`]). Every other outcome, a
//!   failed claim and a lost reply show nothing, and a claim is never
//!   retried (ADR 0089: the inbox still holds it).
//! - Failed sessions never reach here: the diff drops them, as Paseo's
//!   `session-context.tsx:267-269` does.
//! - Alternatives: a session-scoped claim form on the node line before
//!   assignment, which is the integrator's call and a wire change; or
//!   reading pages until the entry turns up, which costs unbounded reads per
//!   change.

use vessel_proto::ProtocolVersion;
use vessel_proto::inbox::{InboxEntry, InboxItem, InboxRead};
use vessel_proto::notifications::{
    ATTENTION_NOTIFICATIONS_PROTOCOL, NotificationClaim, NotificationSource,
};

use super::{Notice, NoticeKind};

/// Whether a machine at `protocol` admits claims: never while ADR 0089's
/// protocol is unassigned.
#[must_use]
pub fn claims(protocol: Option<ProtocolVersion>) -> bool {
    match (ATTENTION_NOTIFICATIONS_PROTOCOL, protocol) {
        (Some(needed), Some(have)) => have >= needed,
        _ => false,
    }
}

/// How a session read on screen is marked on a machine (claims row 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadCall {
    /// `inbox.read` (`read_inbox_through`). It takes the `Read` scope, so a
    /// device given only `read` can mark this way.
    Inbox,
    /// `session.mark` with `read { through }`, as before ADR 0089's
    /// protocol.
    Mark,
}

/// The read call for a machine at `protocol`: `inbox.read` once it admits
/// ADR 0089's protocol, the read mark before (and always while the
/// protocol is unassigned).
#[must_use]
pub fn read_call(protocol: Option<ProtocolVersion>) -> ReadCall {
    if claims(protocol) {
        ReadCall::Inbox
    } else {
        ReadCall::Mark
    }
}

/// Whether an inbox item is what `kind` told of: a request for "needs
/// you", a turn's end for "finished".
const fn told_of(kind: NoticeKind, item: &InboxItem) -> bool {
    match kind {
        NoticeKind::NeedsYou => matches!(
            item,
            InboxItem::Permission { .. } | InboxItem::Question { .. } | InboxItem::InSession { .. }
        ),
        NoticeKind::Finished => matches!(item, InboxItem::Finished | InboxItem::Limit),
    }
}

/// The exact source to claim for `notice`: the newest unread entry of its
/// session that matches its kind, in `entries` (a first page, newest first).
/// `None` when the first page does not hold it.
#[must_use]
pub fn source(entries: &[InboxEntry], notice: &Notice) -> Option<NotificationSource> {
    entries
        .iter()
        .find(|entry| {
            entry.key.session == notice.session
                && entry.read == InboxRead::Unread
                && told_of(notice.kind, &entry.item)
        })
        .map(|entry| NotificationSource::Session {
            key: entry.key,
            cursor: entry.cursor,
        })
}

/// What asking came to for one notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The first page did not hold its entry, or could not be read: ADR
    /// 0114's path.
    NotOnFirstPage,
    /// The machine answered the claim.
    Answered(NotificationClaim),
    /// The claim failed or its reply was lost, or the machine could not be
    /// asked at all.
    Failed,
}

/// Whether the window shows the notice after asking.
#[must_use]
pub const fn shows(asked: Asked) -> bool {
    matches!(
        asked,
        Asked::NotOnFirstPage | Asked::Answered(NotificationClaim::Admitted)
    )
}

#[cfg(test)]
mod tests;
