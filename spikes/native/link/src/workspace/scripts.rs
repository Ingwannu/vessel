//! Existing protocol-11 calls for the native project script surface.
//! No local shell or automatic write retry lives in the client. Every call
//! has a deadline; a timed-out write is uncertain and requires a fresh read.
use crate::connections::{RequestFailure, SelectedConnection};
use std::time::Duration;
use vessel_app_view::workspace::git::RequestKey;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::scripts::{Action, Snapshot, Target};
use vessel_client::{CallError, Unary};
use vessel_proto::ports::ListPorts;
use vessel_proto::scripts::{ListScripts, RestartScript, RunScript, ScriptRunOutcome};
use vessel_proto::terminals::KillTerminal;

/// A complete discovery read or one checked lifecycle intention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptAsk {
    /// Configuration, supervised runs and observed listeners.
    Read {
        /// Screen identity.
        target: Target,
        /// App correlation; never sent on wire.
        key: RequestKey,
    },
    /// One run/restart/stop; its outcome is never retried automatically.
    Write {
        /// Screen identity.
        target: Target,
        /// App correlation.
        key: RequestKey,
        /// Prepared lifecycle action.
        action: Action,
    },
}
/// Echo the owner and correlation even on unsupported/offline/refused paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptAnswer {
    /// Read receipt; port failure stays separate from script configuration.
    Read {
        /// Original owner.
        target: Target,
        /// Original key.
        key: RequestKey,
        /// Fresh snapshot.
        result: Result<Snapshot, Problem>,
    },
    /// Mutation receipt; None is a successful stop acknowledgement.
    Written {
        /// Original owner.
        target: Target,
        /// Original key.
        key: RequestKey,
        /// Node outcome, if this was run/restart.
        result: Result<Option<ScriptRunOutcome>, Problem>,
    },
}
/// Bound a hung peer without repeating a possibly-applied mutation.
const CALL_DEADLINE: Duration = Duration::from_secs(15);

pub(super) async fn run(client: &SelectedConnection, ask: ScriptAsk) -> ScriptAnswer {
    match ask {
        ScriptAsk::Read { target, key } => {
            let (scripts, ports) = tokio::join!(
                bounded(
                    client,
                    ListScripts {
                        place: target.place
                    }
                ),
                bounded(client, ListPorts { changed_from: None })
            );
            let result = scripts.map(|scripts| Snapshot {
                scripts,
                ports,
                host: None,
            });
            ScriptAnswer::Read {
                target,
                key,
                result,
            }
        }
        ScriptAsk::Write {
            target,
            key,
            action,
        } => {
            let result = match action {
                Action::Run(script) => bounded(
                    client,
                    RunScript {
                        place: target.place,
                        script,
                    },
                )
                .await
                .map(Some),
                Action::Restart(terminal) => {
                    bounded(client, RestartScript { terminal }).await.map(Some)
                }
                Action::Stop(terminal) => bounded(client, KillTerminal { terminal })
                    .await
                    .map(|_ack| None),
            };
            ScriptAnswer::Written {
                target,
                key,
                result,
            }
        }
    }
}

async fn bounded<R: Unary>(
    client: &SelectedConnection,
    request: R,
) -> Result<R::Response, Problem> {
    match tokio::time::timeout(CALL_DEADLINE, client.call(request)).await {
        Ok(result) => result.map_err(problem),
        Err(_deadline) => Err(Problem::Node {
            code: vessel_proto::ErrorCode::Unavailable,
            message: String::new(),
        }),
    }
}

fn problem(failure: RequestFailure) -> Problem {
    match failure {
        RequestFailure::Refused(_) => Problem::Gone(String::new()),
        RequestFailure::Call(CallError::Remote(error)) => Problem::Node {
            code: error.code,
            message: String::new(),
        },
        RequestFailure::Call(
            CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. },
        ) => Problem::Older,
        RequestFailure::Call(
            CallError::Disconnected
            | CallError::UnexpectedReply(_)
            | CallError::RequestIdsExhausted,
        ) => Problem::Gone(String::new()),
    }
}

impl ScriptAsk {
    pub(super) fn target(&self) -> &Target {
        match self {
            Self::Read { target, .. } | Self::Write { target, .. } => target,
        }
    }
}

pub(super) fn refused(ask: ScriptAsk, problem: &Problem) -> ScriptAnswer {
    match ask {
        ScriptAsk::Read { target, key } => ScriptAnswer::Read {
            target,
            key,
            result: Err(problem.clone()),
        },
        ScriptAsk::Write {
            target,
            key,
            action: _,
        } => ScriptAnswer::Written {
            target,
            key,
            result: Err(problem.clone()),
        },
    }
}

#[cfg(test)]
#[path = "scripts/peer.rs"]
pub(super) mod peer;
#[cfg(test)]
#[path = "scripts/tests.rs"]
mod tests;
