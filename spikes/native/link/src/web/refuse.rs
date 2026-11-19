//! What the browser's demo answers to a command it cannot do: the same
//! refusal a node gives for what it does not support, on the control the
//! command is about. One function per kind of request, so
//! [`super::Link::answer`] stays one line per command.

use vessel_app_view::devices::{ConnectionContext, Ticket};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::{ErrorCode, RequestId, SessionId};

use crate::attachments::{
    Answer as FileAnswer, Attach, CapabilityAsk, CapabilityFailure, Step, Wake,
};
use crate::{ActAnswer, ActFailed, Event};

pub(super) fn code_source(request: crate::workspace::code_source::Request) -> Event {
    Event::CodeSource(Box::new(request.refused(Problem::Older)))
}

pub(super) fn usage(ask: crate::usage::Ask) -> Event {
    Event::Usage(ask.refused(vessel_app_view::usage::RequestFailure::Unsupported))
}

pub(super) fn session_access(access: crate::sessions::Access) -> Event {
    access.refused(ActFailed {
        code: Some(ErrorCode::Unsupported),
        message: "Authenticated session navigation requires a native connection".to_owned(),
    })
}

pub(super) fn session_lifecycle(request: &vessel_app_view::sessions::PendingLifecycle) -> Event {
    Event::SessionLifecycle(crate::sessions::lifecycle::Answer {
        ticket: request.ticket,
        result: Err(crate::sessions::lifecycle::Failure::Unsent),
    })
}

pub(super) fn sessions(ask: crate::sessions::Ask) -> Event {
    Event::Sessions(ask.refused(crate::sessions::Failure::Offline))
}

pub(super) fn create(ask: crate::creation::Ask) -> Event {
    Event::Creation(ask.refused(vessel_app_view::creation::Failure::Unsupported))
}

pub(super) fn history(ask: crate::history::Ask) -> Event {
    Event::History(crate::history::Answer::refused(
        ask,
        vessel_app_view::history::Problem::Unsupported,
    ))
}

pub(super) fn labels(request: crate::labels::Request) -> Event {
    Event::Labels(request.refused(vessel_app_view::labels::Failure::Unsupported))
}

pub(super) fn agents(machine: crate::agents::MachineKey, request: RequestId) -> Event {
    Event::Agents {
        machine,
        request,
        answer: Err(crate::agents::AgentsError::NoTransport),
    }
}

pub(super) fn devices(
    context: ConnectionContext,
    ticket: Ticket,
    ask: crate::devices::Ask,
) -> Event {
    Event::Devices(ask.rejected(
        context,
        ticket,
        vessel_app_view::devices::Problem::Refused(ErrorCode::Unsupported),
    ))
}

pub(super) fn machines(ask: crate::machines::Ask) -> Event {
    Event::Machines(ask.refused(vessel_app_view::machine_book::Failure::Unavailable))
}

pub(super) fn sync(ticket: Ticket) -> Event {
    Event::Sync(crate::sync::Answer {
        ticket,
        result: Err(vessel_app_view::sync::Problem::Refused(
            ErrorCode::Unsupported,
        )),
    })
}

pub(super) fn node_settings(ask: crate::node_settings::Ask) -> Event {
    Event::NodeSettings(ask.refused(vessel_app_view::idle_release::Problem::Refused(
        ErrorCode::Unsupported,
    )))
}

pub(super) fn diagnostics(ask: crate::diagnostics::Ask) -> Event {
    Event::Diagnostics(ask.refused(vessel_app_view::diagnostics::Problem::Refused(
        ErrorCode::Unsupported,
    )))
}

pub(super) fn plugins(ask: crate::plugins::Ask) -> Event {
    Event::Plugins(ask.refused(vessel_app_view::plugins::Problem::Unsupported))
}

pub(super) const fn pair() -> Event {
    Event::PairFailed(vessel_app_view::pairing::PairProblem::Browser)
}

// The demo's sessions are its own records, not a node's: an act on one,
// starting one again or a workspace ask is refused as a node refuses what it
// does not support, on the control it is about.

pub(super) fn act(machine: String, session: SessionId, act: &crate::Act) -> Event {
    Event::Acted(ActAnswer {
        terminal: act.terminal_opening().cloned(),
        machine,
        session,
        kind: act.kind(),
        result: Err(ActFailed {
            code: Some(ErrorCode::Unsupported),
            message: "The browser's demo takes no session actions".to_owned(),
        }),
    })
}

pub(super) fn resume(session: SessionId) -> Event {
    Event::ResumeFailed {
        session,
        why: "The browser's demo cannot start a session again".to_owned(),
    }
}

pub(super) fn workspace(machine: String, ask: crate::workspace::Ask) -> Event {
    Event::Workspace {
        machine,
        answer: ask.refused(&Problem::Node {
            code: ErrorCode::Unsupported,
            message: "The browser's demo has no projects".to_owned(),
        }),
    }
}

/// The demo fleet keeps no search index: every machine finds nothing.
pub(super) fn search(ask: crate::search::Ask) -> Event {
    Event::Searched(crate::search::Answer {
        machine: ask.machine,
        ticket: ask.ticket,
        result: Ok(vessel_proto::search::SearchPage {
            hits: Vec::new(),
            next: None,
            reached: None,
        }),
    })
}

/// A browser has no files to send yet (track P's row c).
pub(super) fn send(attach: Attach) -> Event {
    Event::Files(FileAnswer::Upload {
        message: attach.message,
        session: attach.session,
        key: attach.key,
        step: Step::Failed {
            problem: vessel_app_view::composer::tray::Problem::Node(
                "The browser's demo takes no files".to_owned(),
            ),
            again: vessel_app_view::composer::tray::Again::Never,
        },
    })
}

/// Nor any sent file to read back.
pub(super) fn capabilities(ask: CapabilityAsk) -> Event {
    Event::Files(FileAnswer::Capabilities {
        ask,
        result: Err(CapabilityFailure::Offline),
    })
}

pub(super) fn wake(wake: Wake) -> Event {
    Event::Files(FileAnswer::Woke {
        message: wake.message,
        session: wake.session,
        outcome: crate::attachments::WakeOutcome::Refused,
    })
}

pub(super) fn read(sent: vessel_app_view::composer::sent::Sent) -> Event {
    Event::Files(FileAnswer::Read { sent, bytes: None })
}

pub(super) fn create_files() -> Event {
    Event::Failed("The browser's demo takes no files".to_owned())
}

/// The demo fleet keeps no search index: no root, no suggestion.
pub(super) fn suggest(ask: crate::mentions::Ask) -> Event {
    let unsupported = vessel_app_view::composer::mentions::Failure::Unsupported;
    match ask {
        crate::mentions::Ask::Root(ask) => {
            Event::Files(FileAnswer::Root(crate::mentions::RootAnswer {
                ask,
                result: Err(unsupported),
            }))
        }
        crate::mentions::Ask::Suggestions(request) => {
            Event::Files(FileAnswer::Suggestions(crate::mentions::Answer {
                request,
                result: Err(unsupported),
            }))
        }
    }
}
