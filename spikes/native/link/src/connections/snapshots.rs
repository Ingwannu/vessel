//! A single Machine snapshot pairs its welcome identity and admission epoch.
//! Session-read refusal is not transport death and cannot erase authentication.
use crate::{Connected, Event, State};
use vessel_app_view::{Card, Machine, NodeVersion, Reach};
impl State {
    pub(crate) async fn read_sessions(&mut self, name: String, connected: Connected) -> Machine {
        let mut machine = match connected.client.sessions().await {
            Ok(sessions) => Machine {
                sessions: sessions
                    .iter()
                    .map(|s| Card::from_session(&name, s))
                    .collect(),
                agents: connected.agents.clone(),
                protocol: Some(connected.client.protocol()),
                version: Some(NodeVersion::new(&connected.client.node().version)),
                name: name.clone(),
                reach: Reach::Online,
                node: Some(connected.client.node().id),
                connection_epoch: Some(connected.context.epoch()),
            },
            Err(error) => {
                let mut machine = Machine::unreachable(name.clone(), &error.to_string());
                machine.node = Some(connected.client.node().id);
                if matches!(error, vessel_client::CallError::Disconnected) {
                    connected.retire();
                    let _old = self.clients.remove(&name);
                    self.emit(Event::DeviceRetired(connected.context.tag()))
                        .await;
                    return machine;
                }
                // A semantic refusal retains the exact authenticated admission.
                machine.connection_epoch = Some(connected.context.epoch());
                machine.protocol = Some(connected.client.protocol());
                machine.version = Some(NodeVersion::new(&connected.client.node().version));
                machine.agents = connected.agents.clone();
                machine
            }
        };
        // Every exported context comes from this one public Machine projection.
        machine.node = Some(connected.client.node().id);
        let _old = self.clients.insert(name, connected);
        machine
    }
}
