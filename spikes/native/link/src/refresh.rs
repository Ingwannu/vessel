//! The link task's refresh and connection (`State::refresh`, `connect`, `hand_kept`, `keep_commands`), moved unchanged out of the crate root.
//!
//! [Decision Log]
//! - Purpose: the link task's refresh and connection sit beside the crate root, which then
//!   fits its 500-line budget and leaves the size list, so packages can
//!   register a module and a hub without growing a listed file.
//! - Constraint: connection admission and retirement are hooks into the one
//!   connection owner. Its module owns identity and read-refusal policy;
//!   this module retains the existing refresh and connection path.
//! - Alternatives: raise the root's recorded size (the list only shrinks).
//! - Choice: a child module of the crate root, as `options.rs` is.
//! - Cost: one more file; no API, behaviour or dependency change.

use super::{Connected, Event, KeptLists, LinkError, State, machines, nodes, offers};
use chrono::Utc;
use vessel_app_view::{Agent, Fleet, Machine};
use vessel_client::NodeClient;
use vessel_proto::{AgentCommand, HarnessRef, ProtocolVersion};

impl State {
    pub(crate) async fn reload(&mut self) {
        self.retire_connections().await;
        self.refresh().await;
    }
    /// List every machine's sessions, connecting to those not connected.
    pub(super) async fn refresh(&mut self) {
        let book = match self.saved_entries().await {
            Ok(book) => book,
            Err(vessel_app_view::machine_book::Failure::File(fault)) => {
                self.emit(Event::BookUnreadable(fault)).await;
                return;
            }
            Err(failure) => {
                self.emit(Event::Machines(machines::Answer::Read(Err(failure))))
                    .await;
                return;
            }
        };
        self.reconcile_connections(&book).await;
        let mut machines = Vec::with_capacity(book.len());
        for (book_name, entry) in book {
            let name = book_name.as_str().to_owned();
            let reach = if let Some(connected) = self.clients.get(&name) {
                Ok(connected.clone())
            } else {
                let reach = self.connect(&name, &entry).await;
                if let Ok(connected) = &reach {
                    self.connection_ready(&book_name, &connected.client).await;
                }
                reach
            };
            let machine = match reach {
                Ok(connected) => self.read_sessions(name, connected).await,
                Err(error) => Machine::unreachable(name, &error.to_string()),
            };
            machines.push(machine);
        }
        self.session_connections_changed();
        self.emit(Event::Fleet(Fleet {
            machines,
            read_at: Some(Utc::now()),
        }))
        .await;
    }

    pub(super) async fn connect(
        &mut self,
        name: &str,
        entry: &vessel_client::book::BookEntry,
    ) -> Result<Connected, LinkError> {
        let client = nodes::connect(entry, nodes::client_info(&self.label)).await?;
        let harnesses = match client.harnesses().await {
            Ok(harnesses) => harnesses,
            Err(error) => {
                tracing::warn!(%error, "could not list the machine's agents");
                Vec::new()
            }
        };
        // `agent.list` (protocol 11) names every agent the node knows, the
        // ones not installed too; `harness.list` only the installed.
        let agents = if client.protocol() >= ProtocolVersion::new(11) {
            match client.call(vessel_proto::agents::ListAgents {}).await {
                Ok(known) => Agent::known(&harnesses, &known),
                Err(error) => {
                    tracing::warn!(%error, "could not read the node's agent list");
                    Agent::all(&harnesses)
                }
            }
        } else {
            Agent::all(&harnesses)
        };
        self.admit_connection(name, entry, client, agents)
    }

    /// A machine just connected: the lists its agents gave last time, if
    /// any are kept ([`Event::OptionsKept`]).
    pub(super) async fn hand_kept(&self, machine: &str, client: &NodeClient) {
        let node = &client.node().id;
        let lists: Vec<KeptLists> = match self.offers.lock() {
            Ok(store) => store
                .for_node(node)
                .into_iter()
                .filter(|kept| kept.profile.is_none())
                .map(|kept| KeptLists {
                    harness: kept.agent.clone(),
                    options: kept.options.clone(),
                    commands: kept.commands.clone(),
                })
                .collect(),
            Err(_poisoned) => Vec::new(),
        };
        if !lists.is_empty() {
            self.emit(Event::OptionsKept {
                machine: machine.to_owned(),
                lists,
            })
            .await;
        }
    }

    /// Keep a session's reported commands for `harness` on `machine`'s
    /// node (by its id, as its options are); a machine not connected has no
    /// id to keep them under, and they are dropped (its next session
    /// reports them again).
    pub(super) fn keep_commands(
        &self,
        machine: &str,
        harness: HarnessRef,
        commands: Vec<AgentCommand>,
    ) {
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
