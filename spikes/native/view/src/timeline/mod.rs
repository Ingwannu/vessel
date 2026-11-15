//! A structured session's timeline, ported from the page's
//! `web/src/data` (timeline, tool runs, rows, the window, find, the
//! outline, approvals, the context ring) with the page's test cases as
//! its tests.
//!
//! [Decision Log]
//! - Purpose: the app draws a session as the page does (ADR 0036: one
//!   experience on every surface; behaviour has one specification,
//!   checked on both sides). The first build's fold (`conversation.rs`)
//!   kept only what its screen drew; the page's timeline has fifteen
//!   kinds of entry and the state the composer, inspector and title bar
//!   read.
//! - Constraint: records arrive one at a time, by the thousand on a
//!   replay; a 20,000-entry session must open and scroll smoothly. No UI
//!   toolkit's types here (ADR 0035), and this crate's lints deny floats,
//!   `as` and indexing.
//! - Alternatives: the page's immutable batches (a new entry list per
//!   batch, new objects for what changed): React needs them to know what
//!   to draw again; a native list asks per row, so copying 20,000 entries
//!   per batch would buy nothing. Keeping raw records and folding while
//!   drawing: the screens would then hold protocol types and fold every
//!   frame.
//! - Choice: one `Timeline`, changed in place, entries only appended or
//!   changed where they stand; each change bumps the entry's revision
//!   and `apply` says which entries a batch added or changed, so the
//!   screen measures only those again. Indices by id (tool call,
//!   permission, question) as the page has them, so a replay is linear.
//!   What the page passes around by entry key, the app passes by entry
//!   index: an index never changes, and costs no hashing.
//! - Cost: an entry is the page's shape, not the first build's `Entry`;
//!   the session screen moves to this one when its area is drawn.

mod agent;
mod agent_message;
mod append;
mod approval;
mod compaction;
mod diff;
mod disclosures;
mod entry;
mod failure;
mod find;
mod fold;
mod fork;
mod items;
mod live;
mod logs;
mod meter;
mod origin;
mod outline;
mod pending;
mod permission;
mod plan;
mod plan_card;
mod plan_types;
mod plan_words;
mod question;
mod question_step;
mod rewind;
mod rows;
mod runs;
mod searchable;
mod source;
mod text_reveal;
mod tool_display;
mod tool_sections;
mod tool_source;
mod tool_status;
mod tool_words;
mod turn_footer;
mod turns;
mod usage;
mod user_stream;
mod window;

#[cfg(test)]
mod bounds_tests;

pub use agent_message::agent_message_display;
pub use approval::{AnswerWords, Answers, Settled, allows, answer_words, answers_of, settled};
pub use compaction::{CompactionCaption, CompactionThousands, merge_compaction};
pub use diff::{
    DiffLine, MAX_LINES, Side, WordHighlights, diff_lines, json_text, unified_lines,
    word_highlights,
};
pub use disclosures::Disclosures;
pub use entry::{
    Entry, EntryKey, EntryKind, ParkingChange, Permission, Prompt, ToolView, block_text,
    first_reply, plan_progress, prompt_parts,
};
pub use failure::{FailureKind, TurnFailure, repeats_failure, turn_failure};
pub use find::{
    FindCache, Finder, MAX_QUERY, Match, SLICE, Stepped, find_all, fold, is_space, occurrences,
    query_of, readable_text, searchable, spans_of, start_at, step,
};
pub use fold::{Change, Facts, Timeline};
pub use fork::{EmptyFork, empty_fork, fork_point};
pub use live::Live;
pub use logs::{LogRun, Logs};
pub use meter::{Fullness, Meter, meter_of};
pub use origin::{Until, origin_of};
pub use outline::{OUTLINE_MIN, OUTLINE_WIDTH, OutlineItem, PREVIEW, REPLY, outline, preview};
pub use pending::{MAX_PENDING_MESSAGES, PendingFull, PendingMessage, PendingMessages};
pub use permission::{AnswerMark, CardAnswer, CardTitle, PermissionCard, permission_card};
pub use plan::{PlanActivity, all_pending, plan_activities};
pub use plan_card::{PlanDisclosures, plan_outcome, plan_text};
pub use plan_types::{PlanExpansion, PlanOutcome};
pub use question::{
    AnsweredLine, Draft, Drafts, Problem, Read, Step, Wrong, answer_of, answerable, answered_lines,
    read_field, refused_field, starting_drafts, step_answered, step_problem, step_ready, steps_of,
};
pub use question_step::{
    Choice, Pick, box_is_other, choices, pick, picked, step_header, step_prompt, text_target, typed,
};
pub use rewind::rewind_point;
pub use rows::Rows;
pub use runs::{Folds, RunSummary, Tally, ToolRun};
pub use source::{ExtraSources, entry_at_source};
pub use text_reveal::{RevealClock, TextPhase, TextReveal};
pub use tool_display::{DetailContents, ToolDisplay, ToolLabel, ToolLabelKind, tool_display};
pub use tool_sections::{ToolSection, ToolSectionLabel, tool_sections};
pub use tool_source::ToolDetailSource;
pub use turn_footer::{TurnCompletion, TurnFooter, duration_label};
pub use turns::{TurnActivity, TurnFold, TurnFolds};
pub use usage::ModelUsage;
pub use window::{
    Align, FOLLOW, INSET, MARGIN, Span, at_end, covers, guess, off_by, offsets, scroll_for, span,
};
