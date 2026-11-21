//! Asking a machine before a notice is shown (ADR 0089
//! `notification.claim`; `vessel_app_view::notices::claims` holds the
//! Decision Log and the rules).
//!
//! The app hands over one machine's notices at a time ([`Ask`]). The link
//! reads that machine's first inbox page once, finds each notice's exact
//! entry, and claims it. It answers every notice with what asking came to
//! ([`Answer`]). The app shows only those
//! `vessel_app_view::notices::claims::shows` lets through. A claim is never
//! retried.

use vessel_app_view::notices::Announce;
use vessel_app_view::notices::claims::Asked;

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

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
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
        Err(error) => {
            tracing::debug!(%error, "a machine's inbox was not read for its notices");
            return ask.failed();
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

#[cfg(test)]
mod tests;
