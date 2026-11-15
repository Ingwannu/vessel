//! The link task's command dispatch (`State::handle`), moved unchanged out of the crate root.
//!
//! [Decision Log]
//! - Purpose: the link task's command dispatch sits beside the crate root, which then
//!   fits its 500-line budget and leaves the size list, so packages can
//!   register a module and a hub without growing a listed file.
//! - Constraint: the bodies are exactly what they were in `lib.rs`; only
//!   their visibility widens to `pub(super)` so the root actor still calls
//!   them. Order and effects do not change.
//! - Alternatives: raise the root's recorded size (the list only shrinks).
//! - Choice: a child module of the crate root, as `options.rs` is.
//! - Cost: one more file; no API, behaviour or dependency change.

use super::{Command, Event, LinkError, State, drafts, nodes, pairing};
use vessel_proto::{PermissionOutcome, SessionInput};

impl State {
    pub(super) async fn handle(&mut self, command: Command) {
        match command {
            Command::Reload => self.reload().await,
            Command::RetireConnection(tag) => self.closed_connection(&tag).await,
            Command::Agents {
                machine,
                request,
                ask,
            } => {
                let client = self
                    .clients
                    .get(machine.as_str())
                    .map(|connected| connected.client.clone());
                self.agents
                    .ask(client, machine, request, ask, self.events.clone())
                    .await;
            }

            Command::Machines(ask) => self.machine_book(ask).await,
            Command::Sync { machine, ticket } => self.sync(&machine, ticket).await,
            Command::NodeSettings(ask) => self.node_settings(ask).await,
            Command::Devices {
                context,
                ticket,
                ask,
            } => self.devices(&context, ticket, ask).await,
            Command::Open { machine, session } => self.open(&machine, session),
            Command::Prompt {
                machine,
                session,
                text,
                delivery,
            } => self.prompt(&machine, session, text, delivery).await,
            Command::Start {
                machine,
                harness,
                folder,
                prompt,
                options,
            } => {
                self.start(&machine, harness, folder, Some(prompt), options)
                    .await;
            }
            Command::Options { machine, harness } => self.options(machine, harness),
            Command::KeepCommands {
                machine,
                harness,
                commands,
            } => self.keep_commands(&machine, harness, commands),
            Command::SaveDrafts(drafts) => drafts::save_in_background(drafts),
            Command::Files(files) => self.files(files).await,
            Command::Resume {
                machine,
                session,
                text,
            } => self.resume(&machine, session, text).await,
            Command::Stop { machine, session } => {
                let input = SessionInput::CancelTurn { turn: None };
                self.input(&machine, session, input).await;
            }
            Command::Answer {
                machine,
                session,
                id,
                option,
            } => {
                let input = SessionInput::AnswerPermission {
                    id,
                    outcome: PermissionOutcome::Selected { option },
                };
                self.input(&machine, session, input).await;
            }
            Command::Pair {
                offer,
                name,
                prefer_node_name,
            } => match nodes::pair(&offer, &name, prefer_node_name).await {
                Ok((node, via, address)) => {
                    self.emit(Event::Paired { node, via, address }).await;
                    self.reload().await;
                }
                Err(LinkError::BookFault(fault)) => self.emit(Event::BookUnreadable(fault)).await,
                Err(error) => self.emit(Event::PairFailed(pairing::problem(&error))).await,
            },
            Command::Act {
                machine,
                session,
                act,
            } => self.act(&machine, session, act),
            Command::Workspace { machine, ask } => self.workspace(machine, ask),
            Command::Projects(request) => self.projects(request).await,
            Command::Search(ask) => self.search(ask),
            Command::SavePrefs(prefs) => {
                let saved = tokio::task::spawn_blocking(move || prefs.save()).await;
                match saved {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => tracing::warn!(%error, "could not save the app's choices"),
                    Err(error) => tracing::warn!(%error, "could not save the app's choices"),
                }
            }
        }
    }
}
