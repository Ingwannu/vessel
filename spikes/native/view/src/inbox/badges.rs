//! Inbox consumes the one shared pure attention policy. Workspace consumers
//! can adopt `view::attention` without importing Inbox or pending node contracts.
pub use crate::attention::{
    AgentContribution, Icon, IconActivity, IconAttention, IconFacts, LatestRoot, TerminalState,
    WorkspaceCount, dock_count, icon, workspace,
};
