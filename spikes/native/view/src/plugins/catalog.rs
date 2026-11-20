//! The plugin catalogs the app holds, one per machine (ADR 0115, "Catalog
//! and invoke"), as Paseo's client registry (`plugins/registry.ts`,
//! `plugins/catalog-sync.tsx`) and the page's `data/plugin-catalog.ts`; and
//! what they offer where an action is taken: Command Center items
//! (Paseo's `plugins/command-center/contributions.ts`) and slash commands
//! (`plugins/client-slash-commands/`).
//!
//! [Decision Log]
//! - Purpose: a machine's plugins add Command Center items and composer
//!   commands; the palette and the composer need them without asking the
//!   machine each time they open.
//! - Existing assets: the Settings page's call type and its link task
//!   ([`Request`], `link::plugins`), and [`Tickets`] for answer identity.
//! - Choice: this model decides what to ask (read the catalog, long-poll
//!   `plugin.watch` from its generation, read again when it moves) and the
//!   app sends it; the page does the same with one watcher per machine.
//!   The watcher belongs to the machine's connection: a failed call stops
//!   it until the machine connects again, so a node that keeps refusing is
//!   not asked again every fleet read. The last catalog stays while a
//!   machine is away (Paseo keeps its registry entry) and goes when the
//!   machine leaves the book; no other machine's plugins take its place.
//! - Invoke-bound items are offered only where the catalog says the caller
//!   may invoke (`may_invoke`, coordinator ruling 2026-11-20). An
//!   `open_screen` action waits for screens (P3b) and is left out until
//!   then.
//! - Cost: one long poll per connected machine that serves the catalog
//!   (the node answers within 25 s).

use std::collections::HashMap;

use vessel_proto::conversation::CommandName;
use vessel_proto::plugins::{
    Action, Caption, ClientInstallation, CommandContext, ContributionContent, InvokeContext,
    InvokePlugin, PanelContext, PluginCatalog, PluginGeneration, PluginId, RpcName,
    RuntimeGeneration,
};
use vessel_proto::{ProjectId, ProtocolVersion, RawJson, SessionId, WorktreeId};

use crate::board::{Fleet, Reach};
use crate::connection::ConnectionEpoch;
use crate::devices::{MachineKey, Ticket, Tickets};

use super::{Call, Problem, Reply, Request};

/// The protocol that has the catalog and method calls
/// (`PLUGIN_CATALOG_PROTOCOL`).
pub const CATALOG_PROTOCOL: Option<ProtocolVersion> =
    vessel_proto::plugins::PLUGIN_CATALOG_PROTOCOL;

/// Whether a machine speaking `protocol` serves the catalog (the page's
/// `speaksCatalog`). An unknown protocol is not asked.
#[must_use]
pub fn speaks_catalog(protocol: Option<ProtocolVersion>) -> bool {
    protocol
        .zip(CATALOG_PROTOCOL)
        .is_some_and(|(spoken, since)| spoken >= since)
}

/// Where an action is taken: no session, or the open session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// No session is open.
    Global,
    /// A session is open.
    Session {
        /// Its machine.
        machine: MachineKey,
        /// The session.
        agent: SessionId,
        /// The project it runs in, when its node placed it in one.
        project: Option<ProjectId>,
        /// The worktree it runs in, when it runs in one.
        worktree: Option<WorktreeId>,
    },
}

/// The invoke context a declared context needs at `place` for `machine`'s
/// plugin, or `None` when it is not available there (Paseo: a workspace
/// command needs a workspace, an agent command an agent).
#[must_use]
pub fn context_at(
    wanted: CommandContext,
    place: &Place,
    machine: &MachineKey,
) -> Option<InvokeContext> {
    let session = || match place {
        Place::Session {
            machine: at,
            agent,
            project: Some(project),
            worktree,
        } if at == machine => Some((*project, *worktree, *agent)),
        Place::Global | Place::Session { .. } => None,
    };
    match wanted {
        CommandContext::Global => Some(InvokeContext::Global),
        CommandContext::Workspace => session()
            .map(|(project, worktree, _agent)| InvokeContext::Workspace { project, worktree }),
        CommandContext::Agent => session().map(|(project, worktree, agent)| InvokeContext::Agent {
            project,
            worktree,
            agent,
        }),
    }
}

/// A slash command's context as a command's (they share the session
/// cases).
const fn panel_context(context: PanelContext) -> CommandContext {
    match context {
        PanelContext::Workspace => CommandContext::Workspace,
        PanelContext::Agent => CommandContext::Agent,
    }
}

/// A plugin method call as an item captured it (ADR 0079: the run and the
/// context ride along and are checked again at the node).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCall {
    /// The machine whose plugin it is.
    pub machine: MachineKey,
    /// The call.
    pub invoke: InvokePlugin,
}

/// `installation`'s `method` with `input`, taken at `context`.
fn call_of(
    machine: &MachineKey,
    installation: &ClientInstallation,
    method: &RpcName,
    input: RawJson,
    context: InvokeContext,
) -> PluginCall {
    PluginCall {
        machine: machine.clone(),
        invoke: InvokePlugin {
            plugin: installation.address().plugin.clone(),
            generation: installation.address().generation,
            method: method.clone(),
            input,
            context,
        },
    }
}

/// A Command Center item a plugin declared, runnable where it is listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCommand {
    /// Unique across machines and plugins.
    pub id: String,
    /// What it says.
    pub title: String,
    /// Searched with the title, not shown.
    pub keywords: String,
    /// The plugin's id: the item's section, as Paseo's `sectionTitle`.
    pub plugin: PluginId,
    /// What choosing it runs.
    pub call: PluginCall,
}

/// A slash command a plugin declared, kept with its machine's catalog so
/// the composer's menu can borrow it while it is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSlash {
    /// `/name`.
    pub name: CommandName,
    /// Its help.
    pub description: String,
    /// Its argument help, when it has some.
    pub hint: Option<String>,
    /// The plugin's id.
    pub plugin: PluginId,
    /// Its machine.
    machine: MachineKey,
    /// The run that declared it.
    generation: RuntimeGeneration,
    /// The method it runs.
    method: RpcName,
    /// Where it may be submitted.
    context: PanelContext,
}

impl PluginSlash {
    /// Every slash command `catalog` declares, for `machine`.
    fn declared(machine: &MachineKey, catalog: &PluginCatalog) -> Vec<Self> {
        let mut commands = Vec::new();
        for installation in catalog.catalog.installations() {
            for item in installation.document().contributions() {
                let ContributionContent::SlashCommand {
                    name,
                    description,
                    argument_hint,
                    context,
                    method,
                } = &item.content
                else {
                    continue;
                };
                commands.push(Self {
                    name: name.clone(),
                    description: description.as_str().to_owned(),
                    hint: argument_hint.as_ref().map(|hint| hint.as_str().to_owned()),
                    plugin: installation.address().plugin.clone(),
                    machine: machine.clone(),
                    generation: installation.address().generation,
                    method: method.clone(),
                    context: *context,
                });
            }
        }
        commands
    }

    /// Whether it can be submitted at `place`.
    fn offered_at(&self, place: &Place) -> bool {
        context_at(panel_context(self.context), place, &self.machine).is_some()
    }

    /// Its call at `place` with `arguments` (the text after the name) as
    /// Paseo's `{arguments}` input; `None` when `place` has not the
    /// context it needs.
    #[must_use]
    pub fn call(&self, arguments: &str, place: &Place) -> Option<PluginCall> {
        let context = context_at(panel_context(self.context), place, &self.machine)?;
        let input = serde_json::json!({ "arguments": arguments }).to_string();
        Some(PluginCall {
            machine: self.machine.clone(),
            invoke: InvokePlugin {
                plugin: self.plugin.clone(),
                generation: self.generation,
                method: self.method.clone(),
                input: RawJson::from_text(input).ok()?,
                context,
            },
        })
    }
}

/// A machine's last catalog with the slash commands it declares.
#[derive(Debug)]
struct Held {
    /// The catalog as read.
    catalog: PluginCatalog,
    /// Its slash commands, whatever the place.
    slash: Vec<PluginSlash>,
}

/// A machine's catalog watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Watch {
    /// The catalog is being read.
    Reading(Ticket),
    /// The watch is open after the generation the held catalog was read at.
    Watching(Ticket, PluginGeneration),
    /// A call failed; it waits for the machine's next connection.
    Stopped,
}

/// The watcher of one connection to a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Watcher {
    /// The connection it belongs to.
    epoch: Option<ConnectionEpoch>,
    /// Where it is.
    watch: Watch,
}

/// What an answer came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answered {
    /// Not one of these calls, or a late one: nothing changed.
    Stale,
    /// The watcher moved on: send these calls. `redraw` when a catalog
    /// changed.
    Asked {
        /// What to send.
        calls: Vec<(MachineKey, Call)>,
        /// Whether what the catalogs offer changed.
        redraw: bool,
    },
    /// A plugin method call came back: its output, or why not.
    Invoked(Result<RawJson, Problem>),
}

/// Every machine's last catalog and its watcher.
#[derive(Debug, Default)]
pub struct Catalogs {
    /// The book's machines, in its order, as last synced.
    book: Vec<MachineKey>,
    /// The last catalog read, by machine.
    held: HashMap<MachineKey, Held>,
    /// Each serving machine's watcher.
    watchers: HashMap<MachineKey, Watcher>,
    /// The method calls waiting for their answers.
    invoking: Vec<Ticket>,
    /// Answer identities.
    tickets: Tickets,
}

impl Catalogs {
    /// Follow the fleet: forget the machines out of the book, end the
    /// watchers of machines that stopped serving, and start one for each
    /// connection that serves the catalog and has none. Returns the reads
    /// to send.
    pub fn sync(&mut self, fleet: &Fleet) -> Vec<(MachineKey, Call)> {
        self.book = fleet
            .machines
            .iter()
            .map(|machine| MachineKey::from_book(&machine.name))
            .collect();
        let book = &self.book;
        self.held.retain(|machine, _held| book.contains(machine));
        let mut calls = Vec::new();
        for machine in &fleet.machines {
            let key = MachineKey::from_book(&machine.name);
            let serving = machine.reach == Reach::Online && speaks_catalog(machine.protocol);
            let current = self
                .watchers
                .get(&key)
                .is_some_and(|watcher| watcher.epoch == machine.connection_epoch);
            if !serving {
                let _ended = self.watchers.remove(&key);
            } else if !current {
                let Ok(ticket) = self.tickets.next_ticket() else {
                    continue;
                };
                let watcher = Watcher {
                    epoch: machine.connection_epoch,
                    watch: Watch::Reading(ticket),
                };
                let _replaced = self.watchers.insert(key.clone(), watcher);
                calls.push((
                    key,
                    Call {
                        ticket,
                        request: Request::Catalog,
                    },
                ));
            }
        }
        let book = &self.book;
        self.watchers
            .retain(|machine, _watcher| book.contains(machine));
        calls
    }

    /// Ask `call`: its answer comes back as [`Answered::Invoked`]. `None`
    /// when every answer identity was used.
    pub fn invoke(&mut self, call: PluginCall) -> Option<(MachineKey, Call)> {
        let ticket = self.tickets.next_ticket().ok()?;
        self.invoking.push(ticket);
        Some((
            call.machine,
            Call {
                ticket,
                request: Request::Invoke(call.invoke),
            },
        ))
    }

    /// Take `machine`'s answer to `ticket`.
    pub fn answered(
        &mut self,
        machine: &MachineKey,
        ticket: Ticket,
        result: Result<Reply, Problem>,
    ) -> Answered {
        if let Some(at) = self.invoking.iter().position(|held| *held == ticket) {
            let _done = self.invoking.swap_remove(at);
            return Answered::Invoked(result.and_then(Reply::into_output));
        }
        let Some(watcher) = self.watchers.get(machine).copied() else {
            return Answered::Stale;
        };
        let (next, redraw) = match (watcher.watch, result) {
            (Watch::Reading(asked), Ok(Reply::Catalog(catalog))) if asked == ticket => {
                let generation = catalog.generation;
                let slash = PluginSlash::declared(machine, &catalog);
                let _earlier = self.held.insert(machine.clone(), Held { catalog, slash });
                (Some(Request::Watch(generation)), true)
            }
            (Watch::Watching(asked, seen), Ok(Reply::Generation(now))) if asked == ticket => {
                let next = if now == seen {
                    Request::Watch(seen)
                } else {
                    Request::Catalog
                };
                (Some(next), false)
            }
            (Watch::Reading(asked) | Watch::Watching(asked, _), _) if asked == ticket => {
                (None, false)
            }
            (Watch::Reading(_) | Watch::Watching(..) | Watch::Stopped, _) => {
                return Answered::Stale;
            }
        };
        let ticket = next.as_ref().and_then(|_| self.tickets.next_ticket().ok());
        let watch = match (&next, ticket) {
            (Some(Request::Watch(after)), Some(ticket)) => Watch::Watching(ticket, *after),
            (Some(_), Some(ticket)) => Watch::Reading(ticket),
            (Some(_) | None, None) | (None, Some(_)) => Watch::Stopped,
        };
        let _moved = self.watchers.insert(
            machine.clone(),
            Watcher {
                epoch: watcher.epoch,
                watch,
            },
        );
        let calls = next
            .zip(ticket)
            .map(|(request, ticket)| (machine.clone(), Call { ticket, request }))
            .into_iter()
            .collect();
        Answered::Asked { calls, redraw }
    }

    /// The machines' catalogs, in the book's order.
    fn in_order(&self) -> impl Iterator<Item = (&MachineKey, &PluginCatalog)> {
        self.book
            .iter()
            .filter_map(|machine| self.held.get(machine).map(|held| (machine, &held.catalog)))
    }

    /// Every machine's runnable Command Center items at `place`, in book,
    /// plugin and declaration order (Paseo's rank).
    #[must_use]
    pub fn commands(&self, place: &Place) -> Vec<PluginCommand> {
        let mut commands = Vec::new();
        for (machine, catalog) in self.in_order() {
            if !catalog.may_invoke {
                continue;
            }
            for installation in catalog.catalog.installations() {
                for item in installation.document().contributions() {
                    let ContributionContent::Command {
                        keywords,
                        context,
                        action: Action::Invoke { method, input },
                    } = &item.content
                    else {
                        continue;
                    };
                    let Some(context) = context_at(*context, place, machine) else {
                        continue;
                    };
                    let plugin = installation.address().plugin.clone();
                    commands.push(PluginCommand {
                        id: format!("plugin:{}:{plugin}:{}", machine.as_str(), item.id),
                        title: item.title.as_str().to_owned(),
                        keywords: keywords
                            .iter()
                            .map(Caption::as_str)
                            .collect::<Vec<_>>()
                            .join(" "),
                        call: call_of(machine, installation, method, input.clone(), context),
                        plugin,
                    });
                }
            }
        }
        commands
    }

    /// The slash commands `machine`'s plugins offer at `place` (Paseo's
    /// `usePluginClientSlashCommands`: the open session's machine only), in
    /// plugin and declaration order.
    #[must_use]
    pub fn slash_commands(&self, machine: &MachineKey, place: &Place) -> Vec<&PluginSlash> {
        self.held
            .get(machine)
            .filter(|held| held.catalog.may_invoke)
            .map(|held| {
                held.slash
                    .iter()
                    .filter(|command| command.offered_at(place))
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Reply {
    fn into_output(self) -> Result<RawJson, Problem> {
        if let Self::Output(output) = self {
            Ok(output)
        } else {
            Err(Problem::Refused {
                code: vessel_proto::ErrorCode::Internal,
                message: None,
            })
        }
    }
}
