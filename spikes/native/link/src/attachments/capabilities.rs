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

pub(crate) fn spawn(client: Option<NodeClient>, ask: CapabilityAsk, events: mpsc::Sender<Event>) {
    drop(tokio::spawn(async move {
        let result = match client {
            Some(client) => tokio::time::timeout(DEADLINE, read(&client, &ask))
                .await
                .unwrap_or(Err(CapabilityFailure::TimedOut)),
            None => Err(CapabilityFailure::Offline),
        };
        let _closed = events
            .send(Event::Files(Answer::Capabilities { ask, result }))
            .await;
    }));
}
async fn read(client: &NodeClient, ask: &CapabilityAsk) -> Result<Target, CapabilityFailure> {
    let _permit = READERS
        .acquire()
        .await
        .map_err(|_| CapabilityFailure::Offline)?;
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
