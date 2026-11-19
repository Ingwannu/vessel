//! The diagnostics calls (ADR 0111; TRACKER 567, 568, 569, 67, 87) for
//! Settings' dialogs:
//! - a machine's own report (`node.diagnostics`, protocol 24; its added
//!   sections from 33);
//! - one agent's diagnostic with a new probe (`agent.diagnostic`, 33);
//! - the node's log (`node.log`, 33, for those who may administer it).
//!
//! [Decision Log]
//! - Purpose: Paseo's app diagnostic asks each host, its provider sheet one
//!   provider, and its logs modal the daemon's log. The app asks a machine
//!   the same through the node's calls.
//! - Existing assets: `node_settings`, which runs one task per ask and sends
//!   its answer as an event carrying the machine and the page's ticket, with
//!   failures as codes only (the Devices page's rule: a node's own words do
//!   not reach the screen).
//! - Choice: the same shape, one request enum for the three calls. Each call
//!   is bounded. The agent's probe has its own bound on the node (the
//!   handshake's), so its timeout is longer.
//! - The screens decide from the machine's protocol before asking: a call is
//!   never sent below its version.
//! - Cost: no retry; Refresh asks again.

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::diagnostics::Problem;
use vessel_proto::agents::AgentDiagnostic;
use vessel_proto::diagnostics::{DiagnosticReport, NodeLog};
use vessel_proto::{HarnessRef, ProfileName};

/// Which call.
#[derive(Debug, Clone)]
pub enum Request {
    /// The machine's own report (`node.diagnostics`).
    Report,
    /// One agent's diagnostic under a profile (`agent.diagnostic`).
    Agent {
        /// Which agent.
        agent: HarnessRef,
        /// Which of its profiles; `None` is its default.
        profile: Option<ProfileName>,
    },
    /// The last lines of the node's log (`node.log`); `None` is its default.
    Log {
        /// How many lines.
        lines: Option<u32>,
    },
}

/// One call to one machine.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine, by its book name.
    pub machine: MachineKey,
    /// The page's identity for this request.
    pub ticket: Ticket,
    /// The call.
    pub request: Request,
}

/// What a call answered.
#[derive(Debug, Clone)]
pub enum Reply {
    /// The machine's report.
    Report(Box<DiagnosticReport>),
    /// The agent's diagnostic.
    Agent(Box<AgentDiagnostic>),
    /// The node's log.
    Log(NodeLog),
}

/// Its answer, or why there is none.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine asked.
    pub machine: MachineKey,
    /// The request's identity.
    pub ticket: Ticket,
    /// The reply.
    pub result: Result<Reply, Problem>,
}

impl Ask {
    /// Its answer when it was not asked at all.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Answer {
        Answer {
            machine: self.machine,
            ticket: self.ticket,
            result: Err(problem),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Ask the machine on a task of its own; the answer comes back as an
    /// event.
    pub(crate) async fn diagnostics(&self, ask: Ask) {
        let Ok(client) = self.client(ask.machine.as_str()).cloned() else {
            self.emit(crate::Event::Diagnostics(ask.refused(Problem::Offline)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let bound = match ask.request {
                // The node bounds its probe by the handshake (60 s) and
                // Paseo its provider refresh by 120 s; this covers both.
                Request::Agent { .. } => std::time::Duration::from_secs(180),
                Request::Report | Request::Log { .. } => std::time::Duration::from_secs(15),
            };
            let result = tokio::time::timeout(bound, call(&client, ask.request))
                .await
                .unwrap_or(Err(Problem::TimedOut));
            let answer = Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result,
            };
            let _closed = events.send(crate::Event::Diagnostics(answer)).await;
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(client: &vessel_client::NodeClient, request: Request) -> Result<Reply, Problem> {
    use vessel_proto::agents::GetAgentDiagnostic;
    use vessel_proto::diagnostics::{GetDiagnostics, ReadNodeLog};
    match request {
        Request::Report => client
            .call(GetDiagnostics {})
            .await
            .map(|report| Reply::Report(Box::new(report))),
        Request::Agent { agent, profile } => client
            .call(GetAgentDiagnostic { agent, profile })
            .await
            .map(|diagnostic| Reply::Agent(Box::new(diagnostic))),
        Request::Log { lines } => client.call(ReadNodeLog { lines }).await.map(Reply::Log),
    }
    .map_err(problem)
}

/// A call's failure as a code (never the node's text).
#[cfg(not(target_arch = "wasm32"))]
fn problem(error: vessel_client::CallError) -> Problem {
    use vessel_proto::ErrorCode;
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => {
            Problem::Refused(ErrorCode::Unsupported)
        }
        vessel_client::CallError::UnexpectedReply(_)
        | vessel_client::CallError::RequestIdsExhausted => Problem::Refused(ErrorCode::Internal),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::problem;
    use vessel_app_view::diagnostics::Problem;
    use vessel_proto::ErrorCode;

    #[test]
    fn a_refusal_crosses_as_its_code_only() {
        let raw = "a node's own words that may not reach the screen";
        let error = vessel_client::CallError::Remote(vessel_proto::WireError::new(
            ErrorCode::Forbidden,
            raw,
        ));
        let safe = problem(error);
        assert_eq!(safe, Problem::Refused(ErrorCode::Forbidden));
        assert!(!format!("{safe:?}").contains(raw));
        assert_eq!(
            problem(vessel_client::CallError::Disconnected),
            Problem::Offline
        );
        assert_eq!(
            problem(vessel_client::CallError::UnassignedProtocol {
                method: vessel_proto::Method::ReadNodeLog,
            }),
            Problem::Refused(ErrorCode::Unsupported)
        );
    }
}
