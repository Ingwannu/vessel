//! C3 git panel meaning, independent of the pane manager and transport.
//!
//! [Decision Log]
//! - Purpose: one interpretation for Changes, working diff and commit diff.
//! - Constraints: the old Base wire scope includes working changes, whereas
//!   Paseo distinguishes Committed and All; unavailable support stays explicit.
//! - Alternatives: another session-bound panel or decisions in drawing code.
//! - Choice: typed preferences, read projection and demand; existing diff rows
//!   and git action preparation remain the shared lower-level implementation.
//! - Cost: registry/layout binding arrives through their owners' contracts.

mod binding;
mod comparison;
mod document;
mod failure;
mod history;
mod options;
mod projection;
mod requests;
mod tree;
mod words;

pub use binding::{
    AnswerAdmission, LocationRefusal, LocationTicket, PanelAddress, PanelBinding, answer_admission,
};
pub use comparison::{Comparison, ComparisonKind, ComparisonMode, RepositoryKey, Whitespace};
pub use document::{
    BufferColumn, BufferColumns, BufferError, BufferKind, BufferLine, DiffCell, DiffDocument,
    DisplayRow, ReviewKey, ReviewSide, SourceLine, SourceRow,
};
pub use failure::{PanelRead, ReadFailure};
pub use history::{
    COMMIT_FILE_RETAIN_FOR, CommitLocation, CommitPublication, HISTORY_FRESH_FOR, HistoryCache,
    HistoryCapability, HistoryCommit, HistoryData, HistoryState, history_state,
};
pub use options::{
    DisplayPreferences, Fold, FormFactor, InlineDiff, Layout, PanelState, PanelTarget,
    Presentation, TreeVisibility, Wrap,
};
pub use projection::{Body, Demand, DiffRead, EmptyAction, RepositoryRead, body, empty_action};
pub use requests::{ComparisonTicket, FileReadTarget, FileTicket, PanelReads, StatusTicket};
pub use tree::{
    ChangeIndex, ChangeTotals, ChangeTree, ChangeTreeRow, LineCount, TreeDepth, TreeError,
};
pub use words::ReadIssue;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tree_tests;

#[cfg(test)]
mod requests_tests;

#[cfg(test)]
mod file_target_tests;
