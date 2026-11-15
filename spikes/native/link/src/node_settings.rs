//! A machine's own settings (`node.settings`, `node.set_settings`,
//! protocol 12): for now how long an idle agent runs before it is parked,
//! behind Settings › Agents' "Release idle agents".
//!
//! [Decision Log]
//! - Purpose: the app reads and changes what `vessel node settings
//!   --park-after` does, as Paseo's host card patches its daemon's config.
//! - Choice: one request type for both (`Ask::change` absent reads),
//!   answered with the settings in force either way, so the page always
//!   shows what the node keeps. Failures cross as codes only (the Devices
//!   page's rule: a node's own error text is not carried to the screen).
//! - Cost: no retry; the page asks again when the person comes back.

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::idle_release::Problem;
use vessel_proto::parking::ParkAfter;
use vessel_proto::settings::NodeSettings;

/// A read, or a change, of one machine's settings.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine, by its book name.
    pub machine: MachineKey,
    /// The page's identity for this request.
    pub ticket: Ticket,
    /// What to change; `None` reads.
    pub change: Option<ParkAfter>,
}

/// Its answer: the settings in force, or why not.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine asked.
    pub machine: MachineKey,
    /// The request's identity.
    pub ticket: Ticket,
    /// The settings in force after the request.
    pub result: Result<NodeSettings, Problem>,
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
    pub(crate) async fn node_settings(&self, ask: Ask) {
        let Ok(client) = self.client(ask.machine.as_str()).cloned() else {
            self.emit(crate::Event::NodeSettings(ask.refused(Problem::Offline)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                call(&client, ask.change),
            )
            .await
            .unwrap_or(Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)));
            let answer = Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result,
            };
            let _closed = events.send(crate::Event::NodeSettings(answer)).await;
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(
    client: &vessel_client::NodeClient,
    change: Option<ParkAfter>,
) -> Result<NodeSettings, Problem> {
    use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
    let answer = match change {
        None => client.call(GetNodeSettings {}).await,
        Some(park_after) => {
            client
                .call(SetNodeSettings {
                    park_after: Some(park_after),
                    ..SetNodeSettings::default()
                })
                .await
        }
    };
    answer.map_err(problem)
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
    use vessel_app_view::idle_release::Problem;
    use vessel_proto::ErrorCode;

    #[test]
    fn an_unassigned_call_is_unsupported() {
        assert_eq!(
            problem(vessel_client::CallError::UnassignedProtocol {
                method: vessel_proto::Method::NodeInfo,
            }),
            Problem::Refused(ErrorCode::Unsupported)
        );
    }
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
    }
}
