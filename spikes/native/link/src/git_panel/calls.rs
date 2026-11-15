//! Every path, including a timeout/preflight/send rejection, retains its ticket.
use super::{Answer, Ask, Outcome};
use crate::connections::{RequestFailure, SelectedConnection};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use vessel_app_view::connection::Refusal;
use vessel_app_view::layout::LayoutKey;
use vessel_app_view::workspace::git::panel::ReadFailure;
use vessel_proto::workspace::{CompareGitFile, GetChanges, GetFileDiff, GetStatus, GitCompared};

/// Bound outstanding work including a response held by UI backpressure.
const MAX_CALLS: usize = 8;
/// A failed read must offer retry rather than wait indefinitely for a peer.
const DEADLINE: Duration = Duration::from_secs(15);

#[derive(Debug)]
pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(MAX_CALLS)))
    }
}
impl crate::State {
    pub(crate) async fn git_panel(&self, ask: Ask) {
        if let Some(why) = admission_failure(&ask) {
            self.emit(crate::Event::GitPanel(ask.refused(why))).await;
            return;
        }
        let selected = match self.selected_connection(ask.context()) {
            Ok(selected) => selected,
            Err(why) => {
                self.emit(crate::Event::GitPanel(ask.refused(local(why))))
                    .await;
                return;
            }
        };
        let Ok(permit) = Arc::clone(&self.git_panel_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::GitPanel(ask.refused(ReadFailure::Capacity)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let timeout = ask.clone().refused(ReadFailure::TimedOut);
            let answer = tokio::time::timeout(DEADLINE, run(&selected, ask))
                .await
                .unwrap_or(timeout);
            let _closed = events.send(crate::Event::GitPanel(answer)).await;
            drop(permit);
        }));
    }
}

fn admission_failure(ask: &Ask) -> Option<ReadFailure> {
    let context = ask.context();
    let matches = match &ask.address().workspace {
        LayoutKey::Workspace { node, .. } => *node == context.node(),
        LayoutKey::Session { machine, .. } => machine.as_str() == context.machine().as_str(),
    };
    if !matches {
        return Some(ReadFailure::DestinationChanged);
    }
    ask.repository()
        .filter(|repo| repo.node != context.node())
        .map(|repo| ReadFailure::UnexpectedNode {
            expected: repo.node,
            received: context.node(),
        })
}

async fn run(client: &SelectedConnection, ask: Ask) -> Answer {
    let context = ask.context().clone();
    let address = ask.address().clone();
    let outcome = match ask {
        Ask::Locate(ticket) => {
            let result = client
                .call(vessel_proto::GetSession {
                    session: ticket.session(),
                })
                .await
                .map(Box::new)
                .map_err(failure);
            Outcome::Located(ticket, result)
        }
        Ask::Status { ticket, .. } => {
            let result = client
                .call(GetStatus {
                    root: ticket.repository().root,
                })
                .await
                .map_err(failure);
            Outcome::Status(ticket, result)
        }
        Ask::Summary { ticket, .. } => {
            let result = summary(client, &ticket).await;
            Outcome::Summary(ticket, result)
        }
        Ask::File { ticket, .. } => {
            let result = file(client, &ticket).await;
            Outcome::File(ticket, result)
        }
    };
    Answer {
        context,
        address,
        outcome,
    }
}
async fn summary(
    client: &SelectedConnection,
    ticket: &vessel_app_view::workspace::git::panel::ComparisonTicket,
) -> Result<vessel_proto::workspace::DiffSummary, ReadFailure> {
    let comparison = ticket.comparison();
    if let Some(scope) = comparison.legacy_scope() {
        client
            .call(GetChanges {
                root: ticket.repository().root,
                scope,
                ignore_whitespace: comparison.whitespace().ignored(),
            })
            .await
            .map_err(failure)
    } else {
        client
            .call(comparison.query(ticket.repository().root, ticket.reason()))
            .await
            .map_err(failure)
            .and_then(compared)
    }
}
async fn file(
    client: &SelectedConnection,
    ticket: &vessel_app_view::workspace::git::panel::FileTicket,
) -> Result<vessel_proto::workspace::FileDiff, ReadFailure> {
    let comparison = ticket.comparison().comparison();
    if let Some(scope) = comparison.legacy_scope() {
        client
            .call(GetFileDiff {
                root: ticket.comparison().repository().root,
                scope,
                path: ticket.path().clone(),
                ignore_whitespace: comparison.whitespace().ignored(),
                from_hunk: Some(ticket.from_hunk().get()),
            })
            .await
            .map_err(failure)
    } else {
        client
            .call(CompareGitFile {
                query: comparison.query(
                    ticket.comparison().repository().root,
                    ticket.comparison().reason(),
                ),
                path: ticket.path().clone(),
                from_hunk: ticket.from_hunk(),
            })
            .await
            .map_err(failure)
            .and_then(compared)
    }
}

fn compared<T>(reply: GitCompared<T>) -> Result<T, ReadFailure> {
    match reply {
        GitCompared::Ready { data } => Ok(data),
        GitCompared::NoBase => Err(ReadFailure::NoBase),
        GitCompared::TooLarge => Err(ReadFailure::TooLarge),
        GitCompared::BaseMismatch { stored, requested } => {
            Err(ReadFailure::BaseMismatch { stored, requested })
        }
    }
}

fn local(why: Refusal) -> ReadFailure {
    match why {
        Refusal::Changed => ReadFailure::DestinationChanged,
        Refusal::Closed => ReadFailure::DestinationClosed,
    }
}
fn failure(error: RequestFailure) -> ReadFailure {
    match error {
        RequestFailure::Refused(why) => local(why),
        // Git's own already-redacted diagnostic is retained as detail. Only
        // typed machine codes choose the translated recovery; text is not parsed.
        RequestFailure::Call(error) => crate::workspace::call_problem(&error).into(),
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;
    use crate::git_panel::tests::{answer, install, status};
    use crate::testing::{Home, peer, state, within};

    #[tokio::test]
    async fn full_admission_refuses_before_any_rpc_and_releases_no_fake_result() {
        within(async {
            let home = Home::new();
            let (mut state, mut events) = state(&home);
            let mut peer = peer::connect("fixture").await;
            let context = install(&mut state, peer.client.take().unwrap());
            let held = Arc::clone(&state.git_panel_calls.0)
                .try_acquire_many_owned(u32::try_from(MAX_CALLS).unwrap())
                .unwrap();
            state
                .git_panel(status(context.clone(), context.node(), "fixture"))
                .await;
            let result = answer(&mut events).await;
            assert_eq!(result.context, context);
            assert!(matches!(
                result.outcome,
                Outcome::Status(_, Err(ReadFailure::Capacity))
            ));
            assert!(peer.calls.lock().unwrap().is_empty());
            drop(held);
            assert_eq!(state.git_panel_calls.0.available_permits(), MAX_CALLS);
        })
        .await;
    }
}

#[cfg(test)]
mod compared_tests {
    use super::*;
    use vessel_proto::workspace::{ChangeStatus, DiffContent, FileDiff, WorktreeBase};
    #[test]
    fn canonical_outcomes_preserve_all_payload_fields_and_distinguish_nondata() {
        let file = FileDiff {
            path: "new/file.rs".parse().unwrap(),
            old_path: Some("old/file.rs".parse().unwrap()),
            status: ChangeStatus::Renamed,
            mode: None,
            content: DiffContent::Unknown,
            more_from: Some(42),
        };
        assert_eq!(
            compared(GitCompared::Ready { data: file.clone() }),
            Ok(file)
        );
        assert_eq!(
            compared::<FileDiff>(GitCompared::NoBase),
            Err(ReadFailure::NoBase)
        );
        assert_eq!(
            compared::<FileDiff>(GitCompared::TooLarge),
            Err(ReadFailure::TooLarge)
        );
        let stored: vessel_proto::workspace::RefName = "refs/remotes/origin/main".parse().unwrap();
        let requested = WorktreeBase::Commit {
            id: "a".repeat(40).parse().unwrap(),
        };
        assert_eq!(
            compared::<FileDiff>(GitCompared::BaseMismatch {
                stored: stored.clone(),
                requested: requested.clone()
            }),
            Err(ReadFailure::BaseMismatch { stored, requested })
        );
    }
}
