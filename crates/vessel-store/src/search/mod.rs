//! The index of what was said in every session (protocol 12, ADR 0037):
//! built by the writer as records are appended (`index`), read by
//! `session.search` (`query`).
//!
//! The log holds records; the index holds messages, each a prompt or a run
//! of the agent's reply chunks, written once it ends (`said` decides what a
//! record is to it). A message longer than 8 KiB is several pieces, each
//! pointing at the record where it starts.
//!
//! [Decision Log]
//! - Purpose: `session.search` over every session a node keeps, at a cost
//!   that does not grow with the log on each query.
//! - Alternatives: reading the logs at query time (every record of up to
//!   500 sessions decoded per keystroke); FTS5's `unicode61` tokenizer, as
//!   agent-orchestrator uses (a Korean word with its particle, `세션을`, is
//!   one token, so `세션` is found only as a prefix and never inside a
//!   compound; an identifier inside a longer one is not found); a
//!   contentless index, as GOAT has (no text for snippets or for short
//!   words without reading the log back).
//! - Choice: FTS5 with the trigram tokenizer (GOAT's), external content in
//!   a table of its own, ids that are the messages' times so the index's
//!   order is the page's.
//! - Cost: the text is stored twice (once in the records' JSON, once in
//!   the index's table) plus the trigram index; measured in the plan's
//!   "Track A2: progress". Words under three characters cannot use the
//!   index and are checked by reading messages, bounded per page.

pub(crate) mod idle;
pub(crate) mod index;
mod pieces;
pub(crate) mod query;
mod said;
mod snippet;
