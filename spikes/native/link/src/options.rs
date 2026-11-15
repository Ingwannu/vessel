//! Existing agent option queries and retained command/option lists.
//!
//! [Decision Log]
//! - Purpose: keep one query/cache path outside the root command dispatcher.
//! - Constraint: the root actor remains its real caller; order and effects
//!   must not change while making room under the file-size ratchet.
//! - Alternatives: grow the dispatcher or duplicate its probe/cache path.
//! - Choice: move the existing method, with parent-only visibility. No
//!   request, persistence rule or dependency changes.
//! - Cost: one private native module, registered by its immediate follow-up.
use super::{Event, State, offers};
use chrono::Utc;
use vessel_proto::{AgentCommand, HarnessRef};

impl State {
    /// Ask `machine` what `harness` offers, beside the other commands (the
    /// node may start the agent to answer). A machine not connected is
    /// answered at once with why.
    pub(super) fn options(&self, machine: String, harness: HarnessRef) {
        let client = self.client(&machine).cloned();
        let events = self.events.clone();
        let store = std::sync::Arc::clone(&self.offers);
        drop(tokio::spawn(async move {
            let outcome = match client {
                Ok(client) => {
                    let answer = client.harness_options(harness.clone(), None).await;
                    // Kept beside the answer, not before it: the disk
                    // never delays what the app shows.
                    if let Ok(offered) = &answer {
                        drop(tokio::spawn(offers::keep_answer(
                            store,
                            client.node().id,
                            harness.clone(),
                            offers::Said::Options(offered.options.clone()),
                            offered.probed_at,
                        )));
                    }
                    answer
                        .map(|offered| offered.options)
                        .map_err(|error| error.to_string())
                }
                Err(error) => Err(error.to_string()),
            };
            let _closed = events
                .send(Event::Options {
                    machine,
                    harness,
                    outcome,
                })
                .await;
        }));
    }
    /// Keep a session's reported commands for `harness` on `machine`'s
    /// node (by its id, as its options are); a machine not connected has no
    /// id to keep them under, and they are dropped (its next session
    /// reports them again).
    pub(super) fn keep_commands(&self, machine: &str, harness: HarnessRef, commands: Vec<AgentCommand>) {
        let Ok(client) = self.client(machine) else {
            return;
        };
        drop(tokio::spawn(offers::keep_answer(
            std::sync::Arc::clone(&self.offers),
            client.node().id,
            harness,
            offers::Said::Commands(commands),
            Utc::now(),
        )));
    }

}
