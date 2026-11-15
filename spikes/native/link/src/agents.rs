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
#[derive(Debug)]
pub enum Ask {
    /// One validated record-only form; use [`crate::Command::add_agent`].
    Add(Addition),
    /// Definitions, discovery and cached status; starts no login check.
    List,
    /// Read this confirmed node's registry through the same bounded hub.
    Catalogue {
        /// Identity of the inventory that opened the catalog.
        node: vessel_proto::NodeId,
        /// An explicit refresh may fetch the public index.
        fetch: CatalogueFetch,
    },
    /// A person explicitly checks one agent and profile.
    Check {
        /// The node whose displayed status is being checked.
        node: vessel_proto::NodeId,
        /// The displayed agent, not a row position.
        agent: vessel_proto::HarnessRef,
    },
    /// The node runs its offered installer, then the link follows output.
    Install {
        /// The node that offered the displayed installer.
        node: vessel_proto::NodeId,
        /// Its exact offered builtin part.
        install: InstallAgent,
    },
    /// One consumed confirmation; use [`crate::Command::remove_agent`].
    Remove(Removal),
}

mod addition;
mod catalogue;
mod removal;
pub use addition::Addition;
#[cfg(not(target_arch = "wasm32"))]
pub use addition::AdditionError;
#[cfg(not(target_arch = "wasm32"))]
pub use catalogue::CatalogueError;
pub use catalogue::Fetch as CatalogueFetch;
pub use removal::Removal;
#[cfg(not(target_arch = "wasm32"))]
pub use removal::RemovalError;

pub use vessel_app_view::agent_status::pages::{Administration, Inventory, MachineKey};

/// Facts, progress or completion from a request.
#[derive(Debug)]
pub enum Answer {
    /// The authoritative definition returned by this record-only mutation.
    Added {
        /// Actual completing client identity.
        node: vessel_proto::NodeId,
        /// Returned definition, without any automatic probe.
        info: vessel_proto::agents::AgentInfo,
    },
    /// The node acknowledged forgetting this definition on this connection.
    Removed {
        /// The node whose response was received.
        node: vessel_proto::NodeId,
        /// The exact confirmed definition.
        name: vessel_proto::agents::AgentName,
    },
    /// The registry belongs to the connection that actually answered.
    Catalogue {
        /// The welcome id, checked against the requested machine before I/O.
        node: vessel_proto::NodeId,
        /// The node's platform-resolved copy, never an app-parsed public index.
        catalogue: vessel_proto::agents::Catalogue,
    },
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
    /// A stale machine name must not redirect a check or installer.
    #[error("the displayed node {expected} is now connected as {actual}")]
    TargetChanged {
        /// Physical identity retained by the displayed action.
        expected: vessel_proto::NodeId,
        /// Identity of the actual client immediately before I/O.
        actual: vessel_proto::NodeId,
    },
    /// A record-only addition refusal or uncertain result.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Addition(#[from] AdditionError),
    /// A registry-specific identity, deadline or typed-client failure.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Catalogue(#[from] CatalogueError),
    /// A removal-specific refusal or an explicitly uncertain outcome.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Removal(#[from] RemovalError),
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
            Self::Add(target) => target.operation(),
            Self::Catalogue { node, .. } => Operation::Catalogue { node: *node },
            Self::Check { node: _node, agent } => Operation::Check(agent.clone()),
            Self::Install {
                node: _node,
                install,
            } => Operation::Install(*install),
            Self::Remove(target) => target.operation(),
        }
    }
}

impl AgentsError {
    /// Only a possibly applied mutation requires a new definition snapshot.
    #[must_use]
    pub fn removal_uncertain(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Self::Removal(error) = self {
            return error.is_uncertain();
        }
        false
    }
    /// Plain-language category, while this error keeps the complete source.
    #[must_use]
    pub const fn problem(&self) -> vessel_app_view::agent_status::problem::Problem {
        use vessel_app_view::agent_status::problem::Problem;
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Removal(error) => error.problem(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Catalogue(_) | Self::Addition(_) => Problem::Call,
            Self::TargetChanged { .. } => Problem::NodeChanged,
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

impl AgentsError {
    /// Registry reads cannot reuse removal-specific permission/version words.
    #[must_use]
    pub fn catalogue_problem(&self) -> vessel_app_view::agent_status::catalogue::Problem {
        use vessel_app_view::agent_status::catalogue::Problem;
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Catalogue(error) => error.problem(),
            Self::TargetChanged { .. } => Problem::NodeChanged,
            Self::Disconnected => Problem::Offline,
            Self::Busy => Problem::Busy,
            Self::NoTransport => Problem::NoTransport,
            Self::Output(_) => Problem::Read,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Removal(_) | Self::Addition(_) => Problem::Read,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(vessel_client::CallError::Disconnected) => Problem::Offline,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Call(_) => Problem::Read,
        }
    }
}

impl AgentsError {
    /// Add failures belong to the form, with their original source kept here.
    #[must_use]
    pub fn addition_problem(&self) -> vessel_app_view::agent_status::addition::Failure {
        use vessel_app_view::agent_status::addition::Failure;
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Addition(error) => error.problem(),
            Self::TargetChanged { .. } => Failure::Changed,
            Self::Disconnected => Failure::Offline,
            Self::Busy => Failure::Busy,
            Self::NoTransport => Failure::NoTransport,
            Self::Output(_) => Failure::Rejected,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Removal(_) | Self::Catalogue(_) | Self::Call(_) => Failure::Rejected,
        }
    }
    /// Do not resend when no conclusive acknowledgement was received.
    #[must_use]
    pub fn addition_uncertain(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Self::Addition(error) = self {
            return error.is_uncertain();
        }
        false
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod testing;

// Keep the real actor's owned Agents forwarding arm cohesive as its command
// union grows. This is the former dispatch block, with no lookup/effect change.
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(super) async fn request_agents(
        &mut self,
        machine: MachineKey,
        request: vessel_proto::RequestId,
        ask: Ask,
    ) {
        let client = self
            .clients
            .get(machine.as_str())
            .map(|connected| connected.client.clone());
        self.agents
            .ask(client, machine, request, ask, self.events.clone())
            .await;
    }
}
