//! The five protocol-28 calls (ADR 0086) on each request's exact admission.
//!
//! [Decision Log]
//! - Purpose: the label manager, the picker and the rows act on the host's
//!   real catalog and assignments; until now every command answered
//!   `Unsupported` without reaching the node.
//! - Constraint: an answer must land on the admission that asked (an alias
//!   moved to another node is a different host); raw node words never
//!   reach the form; a write is one call (create-and-assign, rename-and-
//!   recolour are each atomic on the node).
//! - Choice: one task per request over the selected connection's typed
//!   call, with a deadline (an observation waits up to the node's
//!   heartbeat, so it gets the long one). The node's structured
//!   `WorkspaceLabelFailure` detail maps to the view's safe failures; an
//!   older peer is `Unsupported`, a closed or replaced admission `Offline`
//!   or `Changed`; anything else is the generic update failure.
//! - Cost: a late answer is a failure the area shows; the node may still
//!   have committed, which the next catalog read shows.

use std::time::Duration;

use vessel_app_view::connection::Refusal;
use vessel_app_view::labels::{Failure, Label, Name};
use vessel_client::CallError;
use vessel_proto::workspace_labels::{
    AssignWorkspaceLabel, CatalogRead, DeleteWorkspaceLabel, InspectWorkspaceLabelDelete,
    ListWorkspaceLabels, UpdateWorkspaceLabel, WorkspaceLabelFailure,
};
use vessel_proto::registry::GetRegistry;
use vessel_proto::{ErrorCode, ErrorDetail, WireError};

use super::{Answer, Command, Event, Request};
use crate::connections::{RequestFailure, SelectedConnection};

/// A read now or a write: the node answers at once.
const CALL_DEADLINE: Duration = Duration::from_secs(15);
/// An observation: the node answers on a change or at its heartbeat.
const OBSERVE_DEADLINE: Duration = Duration::from_secs(70);

impl crate::State {
    pub(crate) async fn labels(&self, request: Request) {
        let client = match self.selected_connection(&request.context) {
            Ok(client) => client,
            Err(refusal) => {
                self.emit(crate::Event::Labels(request.refused(refused(refusal))))
                    .await;
                return;
            }
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let observing = matches!(
                request.ask,
                Command::List {
                    read: CatalogRead::Observe,
                    ..
                } | Command::Registry {
                    changed_from: Some(_)
                }
            );
            let deadline = if observing {
                OBSERVE_DEADLINE
            } else {
                CALL_DEADLINE
            };
            let answer = tokio::time::timeout(deadline, answer(&client, &request.ask))
                .await
                .unwrap_or_else(|_late| request.ask.refused(late(&request.ask)));
            let event = Event {
                context: request.context,
                ticket: request.ticket,
                answer,
            };
            let _closed = events.send(crate::Event::Labels(event)).await;
        }));
    }
}

/// `ask` on the node, its answer typed.
async fn answer(client: &SelectedConnection, ask: &Command) -> Answer {
    match ask {
        Command::List { cursor, read } => Answer::Listed(
            client
                .call(ListWorkspaceLabels {
                    cursor: *cursor,
                    read: *read,
                })
                .await
                .map_err(failure),
        ),
        Command::Registry { changed_from } => Answer::Registry(
            client
                .call(GetRegistry {
                    changed_from: *changed_from,
                })
                .await
                .map(Box::new)
                .map_err(failure),
        ),
        Command::Assign {
            workspace,
            label,
            assignment,
        } => Answer::Assigned {
            workspace: *workspace,
            name: label.name.clone(),
            result: client
                .call(AssignWorkspaceLabel {
                    workspace: *workspace,
                    label: label.clone(),
                    assignment: *assignment,
                })
                .await
                .map(|assigned| assigned.labels)
                .map_err(failure),
        },
        Command::Update {
            original,
            replacement,
        } => Answer::Updated(
            client
                .call(update(original, replacement))
                .await
                .map(|updated| updated.label)
                .map_err(failure),
        ),
        Command::InspectDelete(name) => Answer::Inspected(
            client
                .call(InspectWorkspaceLabelDelete { name: name.clone() })
                .await
                .map(|deleted| deleted.affected)
                .map_err(failure),
        ),
        Command::Delete(name) => Answer::Deleted(
            client
                .call(DeleteWorkspaceLabel { name: name.clone() })
                .await
                .map(|deleted| deleted.affected)
                .map_err(failure),
        ),
    }
}

/// The atomic edit: the new name only when its spelling differs, the
/// colour always (the node keeps an unchanged one as it is).
fn update(original: &Name, replacement: &Label) -> UpdateWorkspaceLabel {
    UpdateWorkspaceLabel {
        name: original.clone(),
        new_name: (replacement.name.as_str() != original.as_str())
            .then(|| replacement.name.clone()),
        color: Some(replacement.color),
    }
}

/// No answer in time: a list could not load, a write could not finish.
const fn late(ask: &Command) -> Failure {
    match ask {
        Command::List { .. } | Command::Registry { .. } => Failure::Load,
        Command::Assign { .. }
        | Command::Update { .. }
        | Command::InspectDelete(_)
        | Command::Delete(_) => Failure::Update,
    }
}

/// The admission the request named is gone or replaced.
const fn refused(refusal: Refusal) -> Failure {
    match refusal {
        Refusal::Changed => Failure::Changed,
        Refusal::Closed => Failure::Offline,
    }
}

/// A call's failure as the view's safe failure; the node's words stay here.
fn failure(error: RequestFailure) -> Failure {
    match error {
        RequestFailure::Refused(refusal) => refused(refusal),
        RequestFailure::Call(CallError::Disconnected) => Failure::Offline,
        RequestFailure::Call(
            CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. },
        ) => Failure::Unsupported,
        RequestFailure::Call(CallError::Remote(error)) => remote(&error),
        RequestFailure::Call(CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted) => {
            Failure::Update
        }
    }
}

/// The node's refusal, by its structured reason where it gives one, else
/// by its code (an unsupported call, a missing label; any other is the
/// generic update failure).
fn remote(error: &WireError) -> Failure {
    if let Some(ErrorDetail::WorkspaceLabel { reason }) = &error.detail {
        return match reason {
            WorkspaceLabelFailure::NameTaken => Failure::NameTaken,
            WorkspaceLabelFailure::LabelNotFound => Failure::NotFound,
            WorkspaceLabelFailure::WorkspaceNotFound
            | WorkspaceLabelFailure::WorkspaceInactive
            | WorkspaceLabelFailure::Unknown => Failure::Update,
        };
    }
    if error.code == ErrorCode::Unsupported {
        Failure::Unsupported
    } else if error.code == ErrorCode::NotFound {
        Failure::NotFound
    } else {
        Failure::Update
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
