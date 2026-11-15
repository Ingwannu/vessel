//! One managed-child index for sidebar order, the child track and keyboard jumps.
//! Relation is hydrated current state, never reconstructed from Creator or cwd.

mod managed;
pub use managed::{
    MAX_MANAGED_SNAPSHOTS, MAX_TREE_DEPTH, ManagedSnapshot, ManagedTree, Relation, Reveal,
    TreeCoverage, TreeDepth, TreeProblem, TreeRow, Visibility,
};
