//! The window's presence to every connected machine (`push.presence`,
//! protocol 22; `vessel_app_view::presence`, which holds the Decision Log
//! and Paseo's references).
//!
//! The app sends a snapshot on input, on a change of shown session or
//! visibility, and every 15 s, through [`crate::Link::presence`]: a
//! latest-value channel beside the command queue, not a command. Only the
//! latest state matters, a heartbeat must never fill the queue that carries
//! a person's actions, and a screen's tests that read that queue see only
//! actions. This module hands each connected machine its own report: its session focus or
//! elsewhere, and `Active` only for input that connection has not heard
//! of. A machine that connects hears the last snapshot at once, as Paseo's
//! client sends a heartbeat on connecting (`use-client-activity.ts:129-132`).
//!
//! Reports are fire and forget, as Paseo's heartbeats are. A machine older
//! than protocol 22 is refused by the client before the wire; a machine
//! that refuses the device answers each report, once every 15 s.

use std::time::Instant;

use tokio::sync::watch;
use vessel_app_view::connection::ConnectionEpoch;
use vessel_app_view::notices::claims::claims;
use vessel_app_view::presence::{Millis, Snapshot, Told};
use vessel_client::NodeClient;
use vessel_proto::push::ReportPushPresence;

use crate::State;

/// The app's end of the presence channel.
pub(crate) type Sender = watch::Sender<Option<Snapshot>>;

/// The link task's end.
pub(crate) type Receiver = watch::Receiver<Option<Snapshot>>;

/// A presence channel holding no snapshot yet.
pub(crate) fn channel() -> (Sender, Receiver) {
    watch::channel(None)
}

/// The last snapshot and what each connection was told.
#[derive(Debug, Default)]
pub(crate) struct Hub {
    last: Option<(Snapshot, Instant)>,
    told: Told<(String, ConnectionEpoch)>,
}

impl Hub {
    /// The report for one connection, `later` counted from the snapshot.
    fn report(
        &mut self,
        machine: &str,
        epoch: ConnectionEpoch,
        terminals: bool,
    ) -> Option<ReportPushPresence> {
        let (snapshot, taken) = self.last.as_ref()?;
        let later = Millis::try_from(taken.elapsed().as_millis()).unwrap_or(Millis::MAX);
        Some(self.told.report(
            (machine.to_owned(), epoch),
            machine,
            snapshot,
            later,
            terminals,
        ))
    }
}

impl State {
    /// The app sent a snapshot: report the latest.
    pub(crate) fn presence_changed(&mut self, receiver: &mut Receiver) {
        let latest = receiver.borrow_and_update().clone();
        if let Some(snapshot) = latest {
            self.presence(snapshot);
        }
    }

    /// A new snapshot: every connected machine hears its report.
    pub(crate) fn presence(&mut self, snapshot: Snapshot) {
        self.presence_hub.last = Some((snapshot, Instant::now()));
        let current: Vec<(String, ConnectionEpoch)> = self
            .clients
            .iter()
            .map(|(machine, connected)| (machine.clone(), connected.context.epoch()))
            .collect();
        self.presence_hub
            .told
            .retain(|connection| current.contains(connection));
        for (machine, epoch) in current {
            self.presence_to(&machine, epoch);
        }
    }

    /// A machine just connected: it hears the last snapshot at once.
    pub(crate) fn presence_connected(&mut self, machine: &str) {
        if let Some(epoch) = self
            .clients
            .get(machine)
            .map(|connected| connected.context.epoch())
        {
            self.presence_to(machine, epoch);
        }
    }

    fn presence_to(&mut self, machine: &str, epoch: ConnectionEpoch) {
        let Some(client) = self
            .clients
            .get(machine)
            .map(|connected| connected.client.clone())
        else {
            return;
        };
        // Only a machine that admits claims may hear a shown terminal
        // (`vessel_app_view::presence`'s Decision Log).
        let terminals = claims(Some(client.protocol()));
        let Some(report) = self.presence_hub.report(machine, epoch, terminals) else {
            return;
        };
        drop(tokio::spawn(report_presence(client, report)));
    }
}

async fn report_presence(client: NodeClient, report: ReportPushPresence) {
    if let Err(error) = client.report_push_presence(report).await {
        tracing::debug!(%error, "a machine did not take this window's presence");
    }
}

#[cfg(test)]
mod tests;
