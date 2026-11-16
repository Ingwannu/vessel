//! A new/background session must not borrow the selected timeline's facts.
//! session.conversation is the existing authority for abilities and limits.
use super::{Answer, CapabilityAsk, CapabilityFailure};
use crate::Event;
use crate::connections::SelectedConnection;
use tokio::sync::mpsc;
use vessel_app_view::composer::tray::Target;
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
const HANDSHAKE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const ACTIVE: usize = 2;
static READERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(ACTIVE);

pub(crate) fn spawn(
    client: Result<SelectedConnection, CapabilityFailure>,
    ask: CapabilityAsk,
    events: mpsc::Sender<Event>,
) {
    drop(tokio::spawn(async move {
        let result = match client {
            Ok(client) => tokio::time::timeout(DEADLINE, read(&client, &ask))
                .await
                .unwrap_or(Err(CapabilityFailure::TimedOut)),
            Err(why) => Err(why),
        };
        let _closed = events
            .send(Event::Files(Answer::Capabilities { ask, result }))
            .await;
    }));
}
async fn read(
    client: &SelectedConnection,
    ask: &CapabilityAsk,
) -> Result<Target, CapabilityFailure> {
    let _permit = READERS
        .acquire()
        .await
        .map_err(|_closed| CapabilityFailure::Offline)?;
    loop {
        let facts = client
            .call(vessel_proto::conversation::GetConversation {
                session: ask.session,
            })
            .await
            .map_err(|error| CapabilityFailure::Call(super::selected::problem(&error)))?;
        if facts.abilities.is_some() {
            return Ok(Target {
                abilities: facts.abilities,
                limits: facts.limits,
            });
        }
        tokio::time::sleep(HANDSHAKE_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn a_retired_creation_never_reads_capabilities_on_a_stale_admission() {
        let (client, peer) = crate::attachments::read::tests::peer::connect(Vec::new()).await;
        let (_home, state, connection, selected) = super::super::testing::selected(client);
        let ask = CapabilityAsk {
            machine: vessel_app_view::machine_book::Name::parse("fixture").unwrap(),
            expected_node: connection.node(),
            connection,
            session: vessel_proto::SessionId::random(),
            ticket: vessel_app_view::creation::Ticket::new(1),
        };
        state.clients["fixture"].retire();
        assert!(matches!(
            super::super::read::tests::peer::bounded(read(&selected, &ask)).await,
            Err(CapabilityFailure::Call(
                super::super::RequestProblem::Retired(_)
            ))
        ));
        drop(selected);
        drop(state);
        assert!(peer.finish().await.is_empty());
    }
}
