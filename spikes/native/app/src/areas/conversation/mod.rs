//! A session's conversation and its frame: the timeline, the title bar
//! with its Session actions, the confirms, and (next) find, the outline,
//! the inspector's Summary and a terminal session's view. The page's
//! `features/session` less the composer and its docks, which are the
//! session area's (the lead's); drawn after GOAT's conversation and
//! Paseo's controls (the owner's rule after build 0.0.0.0.3).
//!
//! What it means is `view::timeline`, `view::session` and
//! `view::terminal`; what it asks of the node is `link::Act`; its words
//! are `lang.conversation()`.
//!
//! [Decision Log]
//! - Purpose: the session screen shows the whole conversation as the
//!   page does (every entry kind, a long log scrolling smoothly) and
//!   offers the page's Session actions.
//! - Constraint: the session area (`session.rs`) is the lead's: it keeps
//!   its own fold for the composer and the permission dock, and its frame.
//! - Choice: this area keeps a `Timeline` of its own, fed from the same
//!   records (the lead's `records` hands them over in one line), and
//!   draws the frame's title bar and body for the lead's `render` to
//!   place. Two folds of one log cost a second pass over each record; in
//!   exchange neither area reaches into the other's state, and the lead's
//!   fold goes once the composer reads this one.
//! - Cost: a record is folded twice until then.
//!
//! [Decision Log] Another area's conversation ([`Seat`]).
//! - Purpose: a side question (track N5) is a conversation beside the
//!   session's, drawn as GOAT's `SideChatSurface` embeds its normal chat
//!   view: the same rows, tool cards, thoughts and toggles, not a copy.
//! - Constraint: every row and toggle read and changed
//!   `shell.areas.conversation`, the session area's one conversation.
//! - Alternatives: a state of its own for the rows only (a second model
//!   beside this one, and every fold and row rule kept twice); the rows
//!   drawn from a borrowed state with closures for their toggles (a
//!   closure per row, per frame).
//! - Choice: the state stays one type, [`ConversationArea`]; another area
//!   keeps one of its own, opens it ([`ConversationArea::open`]), feeds it
//!   its session's records ([`ConversationArea::take`]) and draws it with
//!   [`timeline::rows`]. A [`Seat`] says where the state is: two plain functions
//!   from the shell to it, so the rows' toggles reach the same state
//!   they were drawn from. The session area's is [`Seat::SESSION`].
//! - Cost: an embedded conversation carries the frame's parts it does not
//!   use (the menu, the confirm, find), which stay empty.

mod actions;
mod area;
mod bar;
mod bench;
mod bubbles;
/// The slot over the composer, and the requests' cards for other areas
/// ([`dock::requests`]).
pub(crate) mod dock;
mod ended;
mod entries;
mod find;
mod follow;
mod footer;
mod fork;
mod intake;
mod look;
mod meter;
mod motion;
mod origin;
mod outline;
mod overlays;
mod permission;
mod place;
mod plan;
pub(crate) mod plan_card;
mod question;
mod queue;
mod read;
mod rename;
#[cfg(test)]
mod resident_requests;
mod responses;
mod rewind;
pub(crate) mod search;
mod tail;
/// Its rows, for another area's conversation too ([`timeline::rows`]).
pub(crate) mod timeline;
mod tool;

pub(crate) use actions::{AgentLog, Confirm, MenuAction};
pub(crate) use area::ConversationArea;
pub(crate) use bar::bar;
pub(crate) use dock::dock;
pub(crate) use meter::meter;
pub(crate) use overlays::overlays;
pub(crate) use place::{Answer, Place, Seat, SessionPlace};
pub(crate) use timeline::timeline;

#[cfg(test)]
mod connection_tests;

#[cfg(test)]
mod late_answers;
