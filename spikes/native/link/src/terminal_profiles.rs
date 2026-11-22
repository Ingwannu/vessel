//! A machine's terminal profiles (ADR 0117, protocol 45): read with
//! `node.settings`, kept whole with `node.set_settings`, behind New tab's
//! profile rows and Settings › Terminals.
//!
//! [Decision Log]
//! - Purpose: the app shows a machine's own list and changes it as Paseo's
//!   host page patches its daemon's `terminalProfiles`.
//! - Choice: one request for both (`Ask::save` absent reads), answered with
//!   the list in force (the kept one, else the shipped four), so a page
//!   always shows what the node keeps. A node below protocol 45 is not
//!   asked; it is `TooOld`. Failures cross as codes, never the node's text
//!   (the Devices page's rule).
//! - Cost: no retry; a page asks again when the person comes back.

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::terminal_profiles::Problem;
use vessel_proto::terminal_profiles::Profiles;

/// A read, or a save of the whole list, of one machine's profiles.
#[derive(Debug, Clone)]
pub struct Ask {
    /// The machine, by its book name.
    pub machine: MachineKey,
    /// The request's identity.
    pub ticket: Ticket,
    /// The whole list to keep; `None` reads.
    pub save: Option<Profiles>,
}

/// Its answer: the list in force, or why not.
#[derive(Debug, Clone)]
pub struct Answer {
    /// The machine asked.
    pub machine: MachineKey,
    /// The request's identity.
    pub ticket: Ticket,
    /// The list in force after the request.
    pub result: Result<Profiles, Problem>,
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
    pub(crate) async fn terminal_profiles(&self, ask: Ask) {
        let Ok(client) = self.client(ask.machine.as_str()).cloned() else {
            self.emit(crate::Event::TerminalProfiles(
                ask.refused(Problem::Offline),
            ))
            .await;
            return;
        };
        let served = vessel_proto::terminal_profiles::TERMINAL_PROFILES_PROTOCOL
            .is_some_and(|since| client.protocol() >= since);
        if !served {
            self.emit(crate::Event::TerminalProfiles(ask.refused(Problem::TooOld)))
                .await;
            return;
        }
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let result =
                tokio::time::timeout(std::time::Duration::from_secs(15), call(&client, ask.save))
                    .await
                    .unwrap_or(Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)));
            let answer = Answer {
                machine: ask.machine,
                ticket: ask.ticket,
                result,
            };
            let _closed = events.send(crate::Event::TerminalProfiles(answer)).await;
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(
    client: &vessel_client::NodeClient,
    save: Option<Profiles>,
) -> Result<Profiles, Problem> {
    use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
    let answer = match save {
        None => client.call(GetNodeSettings::default()).await,
        Some(profiles) => {
            client
                .call(SetNodeSettings {
                    terminal_profiles: Some(profiles),
                    ..SetNodeSettings::default()
                })
                .await
        }
    };
    let settings = answer.map_err(problem)?;
    Ok(settings
        .terminal_profiles
        .unwrap_or_else(|| Profiles::shipped().unwrap_or_default()))
}

/// A call's failure as a code (never the node's text).
#[cfg(not(target_arch = "wasm32"))]
fn problem(error: vessel_client::CallError) -> Problem {
    use vessel_proto::ErrorCode;
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => Problem::TooOld,
        vessel_client::CallError::UnexpectedReply(_)
        | vessel_client::CallError::RequestIdsExhausted => Problem::Refused(ErrorCode::Internal),
    }
}
