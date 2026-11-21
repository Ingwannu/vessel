//! Asking a machine before a notice is shown (ADR 0089
//! `notification.claim`; `vessel_app_view::notices::claims` holds the
//! Decision Log and the rules).
//!
//! The app hands over one machine's notices at a time ([`Ask`]). The link
//! reads that machine's first inbox page once, finds each notice's exact
//! entry, and claims it. It answers every notice with what asking came to
//! ([`Answer`]). When that page cannot be read, every notice takes ADR
//! 0114's path, as one past the first page does. The app shows only those
//! `vessel_app_view::notices::claims::shows` lets through. A claim is never
//! retried.
//!
//! A terminal's notice ([`TerminalAsk`]) needs no page: it names its exact
//! attention revision, so the link claims it at once.

use vessel_app_view::notices::Announce;
use vessel_app_view::notices::claims::Asked;
use vessel_app_view::notices::terminals::TerminalAnnounce;

/// One machine's notices to ask about, with how each would be told.
#[derive(Debug)]
pub struct Ask {
    /// The machine, by its book name.
    pub machine: String,
    /// The notices, as `announce` gave them.
    pub told: Vec<Announce>,
}

/// What asking came to, notice by notice, in the order asked.
#[derive(Debug)]
pub struct Answer {
    /// The machine asked.
    pub machine: String,
    /// Each notice with its outcome.
    pub asked: Vec<(Announce, Asked)>,
}

impl Ask {
    /// Every notice failed: a command never sent, a machine not connected,
    /// or a link that cannot ask. Nothing shows.
    #[must_use]
    pub fn failed(self) -> Answer {
        Answer {
            machine: self.machine,
            asked: self
                .told
                .into_iter()
                .map(|one| (one, Asked::Failed))
                .collect(),
        }
    }
}

/// A session read through a record on a machine that admits ADR 0089's
/// protocol (`inbox.read`, claims row 4). It takes the `Read` scope, so a
/// device given only `read` can mark this way, where `session.mark` asks
/// for more.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadInbox {
    /// The machine, by its book name.
    pub machine: String,
    /// Which session.
    pub session: vessel_proto::SessionId,
    /// The record after the last one shown.
    pub through: vessel_proto::EventSeq,
}

/// One machine's terminal notices to ask about.
#[derive(Debug)]
pub struct TerminalAsk {
    /// The machine, by its book name.
    pub machine: String,
    /// The notices, as `announce_terminals` gave them.
    pub told: Vec<TerminalAnnounce>,
}

/// What asking came to for each terminal notice, in the order asked. Only
/// `Answered(Admitted)` shows: a terminal has no inbox to fall back on.
#[derive(Debug)]
pub struct TerminalAnswer {
    /// The machine asked.
    pub machine: String,
    /// Each notice with its outcome.
    pub asked: Vec<(TerminalAnnounce, Asked)>,
}

impl TerminalAsk {
    /// Every notice failed, and nothing shows.
    #[must_use]
    pub fn failed(self) -> TerminalAnswer {
        TerminalAnswer {
            machine: self.machine,
            asked: self
                .told
                .into_iter()
                .map(|one| (one, Asked::Failed))
                .collect(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Ask `ask.machine` about its terminals' notices, in the background;
    /// the answer comes back as [`crate::Event::TerminalClaims`].
    pub(crate) async fn terminal_claims(&self, ask: TerminalAsk) {
        let Ok(client) = self.client(&ask.machine).cloned() else {
            self.emit(crate::Event::TerminalClaims(ask.failed())).await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let answer = claim_terminals(&client, ask).await;
            // The receiver is gone only when the app is closing.
            let _closed = events.send(crate::Event::TerminalClaims(answer)).await;
        }));
    }

    /// Ask `ask.machine` about its notices, in the background; the answer
    /// comes back as [`crate::Event::Claims`].
    pub(crate) async fn claims(&self, ask: Ask) {
        let Ok(client) = self.client(&ask.machine).cloned() else {
            self.emit(crate::Event::Claims(ask.failed())).await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let answer = ask_machine(&client, ask).await;
            // The receiver is gone only when the app is closing.
            let _closed = events.send(crate::Event::Claims(answer)).await;
        }));
    }
}

/// The first inbox page once, then one claim per notice found on it.
#[cfg(not(target_arch = "wasm32"))]
async fn ask_machine(client: &vessel_client::NodeClient, ask: Ask) -> Answer {
    let page = match client
        .inbox(vessel_proto::inbox::ListInbox { before: None })
        .await
    {
        Ok(page) => page.entries,
        // A failed page tells every notice by ADR 0114's path rather than
        // silencing them (ADR 0089, "Turned on", item 4; the page's
        // `askBeforeTelling`): no claim is made, as for a notice past the
        // first page.
        Err(error) => {
            tracing::debug!(%error, "a machine's inbox was not read for its notices");
            return Answer {
                machine: ask.machine,
                asked: ask
                    .told
                    .into_iter()
                    .map(|one| (one, Asked::NotOnFirstPage))
                    .collect(),
            };
        }
    };
    let mut asked = Vec::with_capacity(ask.told.len());
    for one in ask.told {
        let outcome = match vessel_app_view::notices::claims::source(&page, &one.notice) {
            None => Asked::NotOnFirstPage,
            Some(source) => match client.claim_notification(source).await {
                Ok(claim) => Asked::Answered(claim),
                Err(error) => {
                    tracing::debug!(%error, "a notice's claim was not answered");
                    Asked::Failed
                }
            },
        };
        asked.push((one, outcome));
    }
    Answer {
        machine: ask.machine,
        asked,
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Mark a session read through a record with `inbox.read`, fire and
    /// forget: a lost mark leaves the session unread, and the next view
    /// sends it again.
    pub(crate) fn read_inbox(&self, read: &ReadInbox) {
        let Some(client) = self
            .clients
            .get(&read.machine)
            .map(|connected| connected.client.clone())
        else {
            return;
        };
        let (session, through) = (read.session, read.through);
        drop(tokio::spawn(async move {
            if let Err(error) = client.read_inbox_through(session, through).await {
                tracing::debug!(%error, "a session read was not marked in the inbox");
            }
        }));
    }
}

/// One claim per terminal notice, by its exact revision.
#[cfg(not(target_arch = "wasm32"))]
async fn claim_terminals(client: &vessel_client::NodeClient, ask: TerminalAsk) -> TerminalAnswer {
    let mut asked = Vec::with_capacity(ask.told.len());
    for one in ask.told {
        let outcome = match client.claim_notification(one.notice.source()).await {
            Ok(claim) => Asked::Answered(claim),
            Err(error) => {
                tracing::debug!(%error, "a terminal notice's claim was not answered");
                Asked::Failed
            }
        };
        asked.push((one, outcome));
    }
    TerminalAnswer {
        machine: ask.machine,
        asked,
    }
}

#[cfg(test)]
mod tests;
