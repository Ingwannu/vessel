//! One managed-child index for session-tree order, the child track and session keyboard jumps.
//! Relation is hydrated current state, never reconstructed from Creator or cwd.

mod archive_finished;
pub use archive_finished::{ArchiveFinished, FinishRead, FinishStatus};
mod managed;
mod pane;
mod requests;
mod words;
pub use requests::{SubagentAsk, SubagentReply};
mod track;
pub use managed::{
    MAX_MANAGED_SNAPSHOTS, MAX_TREE_DEPTH, ManagedSnapshot, ManagedTree, Relation, Reveal,
    TreeCoverage, TreeDepth, TreeProblem, TreeRow, Visibility,
};
pub use pane::{ChildPane, ChildQuery, ChildRead};
pub use track::{
    DetachChoice, Host, ManagedChild, PillCount, PillKind, Target, Track, TrackCounts, TrackLoad,
    TrackProblem, TrackRead,
};
