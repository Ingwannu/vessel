//! The agent-tool contract shared by the node and its stdio adapter.
//! Values only: authentication, transport and filesystem work live elsewhere.

mod id;

pub use id::{AddedId, ToolId};
