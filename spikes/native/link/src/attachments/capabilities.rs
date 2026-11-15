//! A new/background session must not borrow the selected timeline's facts.
//! session.conversation is the existing authority for abilities and limits.
use super::{Answer, CapabilityAsk, CapabilityFailure};
use crate::Event;
use tokio::sync::mpsc;
use vessel_app_view::composer::tray::Target;
use vessel_client::NodeClient;
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
const HANDSHAKE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const ACTIVE: usize = 2;
static READERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(ACTIVE);

pub(crate) fn spawn(
    client: Result<NodeClient, CapabilityFailure>,
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
async fn read(client: &NodeClient, ask: &CapabilityAsk) -> Result<Target, CapabilityFailure> {
    super::target::check(client, ask.expected_node)?;
    let _permit = READERS
        .acquire()
        .await
        .map_err(|_closed| CapabilityFailure::Offline)?;
    loop {
        let facts = client
            .conversation(ask.session)
            .await
            .map_err(CapabilityFailure::Call)?;
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
    async fn an_admitted_creation_never_reads_capabilities_from_a_replacement_node() {
        let (client, peer) = crate::attachments::read::tests::peer::connect(Vec::new()).await;
        let actual = client.node().id;
        let expected = vessel_proto::NodeId::random();
        let ask = CapabilityAsk {
            machine: vessel_app_view::machine_book::Name::parse("fixture").unwrap(),
            expected_node: expected,
            session: vessel_proto::SessionId::random(),
            ticket: vessel_app_view::creation::Ticket::new(1),
        };
        assert_eq!(
            read(&client, &ask).await,
            Err(CapabilityFailure::Target(
                super::super::TargetProblem::Changed { expected, actual }
            ))
        );
        drop(client);
        assert!(peer.await.unwrap().is_empty());
    }
}
