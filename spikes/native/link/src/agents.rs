//! Settings' agent discovery, explicit checks and bounded install output.
//!
//! [Decision Log]
//! - Purpose: slow checks and installs must not block fleet traffic.
//! - Constraints: calls need the selected machine's existing connection;
//!   changing machines must not apply an old answer to a new screen.
//! - Alternatives: await inside `State::handle`; spawn without a bound.
//! - Choice: four task permits, typed asks/answers and echoed request ids.
//!   Output long-polls with backpressure on the bounded event channel and
//!   drains terminal pages before announcing that the job is complete.
//! - Cost: busy requests fail visibly; an install continues on its node
//!   if the client closes, as one started by the CLI does.

use vessel_app_view::agent_status::install::{Completion, OutputProblem};
use vessel_proto::agents::{AgentReadiness, InstallAgent, InstallJob, InstallOutput};

/// What the Agents page asks of its selected machine.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Definitions, discovery and cached status; starts no login check.
    List,
    /// A person explicitly checks one agent and profile.
    Check(vessel_proto::HarnessRef),
    /// The node runs its offered installer, then the link follows output.
    Install(InstallAgent),
}

pub use vessel_app_view::agent_status::pages::{Administration, Inventory, MachineKey};

/// Facts, progress or completion from a request.
#[derive(Debug)]
pub enum Answer {
    /// A fresh snapshot.
    List(Inventory),
    /// The explicit status check's results.
    Checked(Vec<AgentReadiness>),
    /// The node accepted an installer.
    Installing(InstallJob),
    /// A validated page, including pages after the process exited.
    Output(InstallOutput),
    /// All retained output was drained, not merely the first terminal page.
    Finished(Completion),
}

/// Failures stay typed until the screen presents them beside the action.
#[derive(Debug, thiserror::Error)]
pub enum AgentsError {
    /// The selected node disconnected.
    #[error("the machine is not connected")]
    Disconnected,
    /// Four requests are already in progress.
    #[error("Settings is busy; try again when the current request finishes")]
    Busy,
    /// A browser has no node transport in this build.
    #[error("this browser build has no connection to a node")]
    NoTransport,
    /// A node output page would repeat, skip or miscount bytes.
    #[error(transparent)]
    Output(#[from] OutputProblem),
    /// Keep the node or transport error, including its code and source.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Call(#[from] vessel_client::CallError),
}

#[cfg(not(target_arch = "wasm32"))]
mod calls;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use calls::Hub;

impl Ask {
    /// The error/progress context, derived before the call starts.
    #[must_use]
    pub fn operation(&self) -> vessel_app_view::agent_status::pages::Operation {
        use vessel_app_view::agent_status::pages::Operation;
        match self {
            Self::List => Operation::Discovery,
            Self::Check(agent) => Operation::Check(agent.clone()),
            Self::Install(install) => Operation::Install(*install),
        }
    }
}

impl AgentsError {
    /// Plain-language category, while this error keeps the complete source.
    #[must_use]
    pub const fn problem(&self) -> vessel_app_view::agent_status::problem::Problem {
        use vessel_app_view::agent_status::problem::Problem;
        match self {
            Self::Disconnected => Problem::Disconnected,
            Self::Busy => Problem::Busy,
            Self::NoTransport => Problem::NoTransport,
            Self::Output(_) => Problem::InvalidOutput,
            // A call whose connection closed is the machine's lost
            // connection, said as such; the transport's words stay out of
            // the screen (the session's rule).
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(vessel_client::CallError::Disconnected) => Problem::Disconnected,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(_) => Problem::Call,
        }
    }
}
