//! One independent subscription. Closing it cannot touch `State::follow`.

use super::{Event, Follow};
use tokio::{sync::mpsc, task::JoinHandle};
use vessel_app_view::side_question::Problem;
use vessel_client::{NodeClient, StreamError};
use vessel_proto::EventSeq;

/// Own the side's cancellation handle separately from the main conversation.
#[derive(Debug, Default)]
pub(crate) struct Stream(Option<JoinHandle<()>>);

impl Stream {
    pub(super) fn close(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }

    pub(super) fn watch(
        &mut self,
        follow: Follow,
        client: Option<NodeClient>,
        events: mpsc::Sender<crate::Event>,
    ) {
        self.close();
        self.0 = Some(tokio::spawn(pump(follow, client, events)));
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.close();
    }
}

async fn pump(follow: Follow, client: Option<NodeClient>, events: mpsc::Sender<crate::Event>) {
    let result = read(&follow, client, &events).await;
    if let Err(problem) = result {
        let _closed = events
            .send(crate::Event::SideQuestion(Event::Lost { follow, problem }))
            .await;
    }
}

async fn read(
    follow: &Follow,
    client: Option<NodeClient>,
    events: &mpsc::Sender<crate::Event>,
) -> Result<(), Problem> {
    if follow.side == follow.target.parent {
        return Err(Problem::InvalidReply);
    }
    let client = client.ok_or(Problem::Offline)?;
    let mut stream = client
        .follow(follow.side, EventSeq::new(0))
        .await
        .map_err(|_disconnected| Problem::Offline)?;
    while let Some(record) = stream.next().await {
        let record = match record {
            Ok(record) => record,
            Err(StreamError::Missing { .. }) => {
                let gap = Event::Lost {
                    follow: follow.clone(),
                    problem: Problem::HistoryGap,
                };
                if events.send(crate::Event::SideQuestion(gap)).await.is_err() {
                    return Ok(());
                }
                continue;
            }
            Err(StreamError::Resumed { .. }) => continue,
            Err(
                StreamError::Call(_)
                | StreamError::Unsubscribed
                | StreamError::Reconnecting { .. }
                | StreamError::Unreachable { .. },
            ) => return Err(Problem::Offline),
        };
        let event = Event::Records {
            follow: follow.clone(),
            records: vec![record],
        };
        if events
            .send(crate::Event::SideQuestion(event))
            .await
            .is_err()
        {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
