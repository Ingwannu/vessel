//! Desktop-owned node orchestration; service files and arbitrary PID signals stay elsewhere.
//!
//! The caller supplies every path and owns the async runtime. Only an exact
//! launched child grants cleanup authority; authenticated reuse remains Attached.

mod node_files;
pub use node_files::{FilesError, NodeHome};

mod launch;
pub use launch::{KeptNode, Launch, LaunchError, ProcessExit};
mod logs;
pub use logs::{LogError, LogReader, LogText};
mod controller;
pub use controller::{LocalNode, LocalNodeError, LocalStatus, Operation, QuitPolicy, StopPolicy};
