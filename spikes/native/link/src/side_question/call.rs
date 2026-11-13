//! Each call has a fixed task budget and deadline. The permit covers the
//! result send too, so an undrained UI cannot accumulate completed tasks.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::side_question::{Target, Ticket};
use vessel_client::NodeClient;
use vessel_proto::{Delivered, SessionInput};

use super::{Answer, Ask, Discovered, Failure, Notice, Outcome};

pub(super) const CALLS: usize = 4;

pub(super) async fn dispatch(
    permits: &Arc<Semaphore>,
    client: Option<NodeClient>,
    events: mpsc::Sender<crate::Event>,
    target: Target,
    ticket: Ticket,
    ask: Ask,
) {
    let kind = ask.kind();
    let permit = Arc::clone(permits).try_acquire_owned();
    let failure = if client.is_none() {
        Some(Failure::Offline)
    } else if permit.is_err() {
        Some(Failure::Busy)
    } else {
        None
    };
    if let Some(why) = failure {
        let answer = Answer {
            target,
            ticket,
            kind,
            result: Err(why),
        };
        let _closed = events
            .send(crate::Event::SideQuestion(Notice::Answer(answer)))
            .await;
        return;
    }
    let (Some(client), Ok(permit)) = (client, permit) else {
        return;
    };
    drop(tokio::spawn(async move {
        let result =
            tokio::time::timeout(Duration::from_secs(30), run(&client, target.parent, ask))
                .await
                .unwrap_or(Err(Failure::TimedOut));
        let answer = Answer {
            target,
            ticket,
            kind,
            result,
        };
        let _closed = events
            .send(crate::Event::SideQuestion(Notice::Answer(answer)))
            .await;
        drop(permit);
    }));
}

async fn run(
    client: &NodeClient,
    parent: vessel_proto::SessionId,
    ask: Ask,
) -> Result<Outcome, Failure> {
    match ask {
        Ask::Discover => {
            let parent_info = client.session(parent).await.map_err(failure)?;
            let sessions = client.sessions().await.map_err(failure)?;
            let side = vessel_app_view::side_question::newest(&sessions, parent)
                .cloned()
                .map(Box::new);
            Ok(Outcome::Discovered(Discovered {
                parent: Box::new(parent_info),
                side,
            }))
        }
        Ask::Open => client
            .open_side_question(parent)
            .await
            .map(|side| Outcome::Session(Box::new(side)))
            .map_err(failure),
        Ask::Reset(side) => client
            .reset_side_question(parent, side)
            .await
            .map(|side| Outcome::Session(Box::new(side)))
            .map_err(failure),
        Ask::Resume(side) => client
            .resume_session(side)
            .await
            .map(|side| Outcome::Session(Box::new(side)))
            .map_err(failure),
        Ask::Prompt(side, question) => match client
            .prompt_side_question(side, question)
            .await
            .map_err(failure)?
        {
            Delivered::Started => Ok(Outcome::Done),
            Delivered::Queued { .. } | Delivered::Steered => Err(Failure::InvalidReply),
        },
        Ask::Stop(side) => client
            .send(side, SessionInput::CancelTurn { turn: None })
            .await
            .map(|()| Outcome::Done)
            .map_err(failure),
    }
}

pub(super) fn failure(error: vessel_client::CallError) -> Failure {
    match error {
        vessel_client::CallError::Disconnected => Failure::Offline,
        vessel_client::CallError::Remote(error) => Failure::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. } => {
            Failure::Refused(vessel_proto::ErrorCode::Unsupported)
        }
        vessel_client::CallError::UnexpectedReply(_) => Failure::InvalidReply,
        vessel_client::CallError::RequestIdsExhausted => Failure::Exhausted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_error_text_cannot_become_panel_text_or_diagnostics() {
        let error = vessel_client::CallError::Remote(vessel_proto::WireError::new(
            vessel_proto::ErrorCode::Forbidden,
            "a-fake-sensitive-value-echoed-by-a-peer",
        ));
        assert_eq!(
            failure(error),
            Failure::Refused(vessel_proto::ErrorCode::Forbidden)
        );
        assert_eq!(
            failure(vessel_client::CallError::Disconnected),
            Failure::Offline
        );
        assert_eq!(
            failure(vessel_client::CallError::RequestIdsExhausted),
            Failure::Exhausted
        );
    }
}
