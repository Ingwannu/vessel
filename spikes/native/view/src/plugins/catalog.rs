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
//!   The watcher belongs to the machine's connection; a failed call waits
//!   and reads again within it (`catalog/watch.rs`). The last catalog
//!   stays while a machine is away (Paseo keeps its registry entry) and
//!   goes when the machine leaves the book; no other machine's plugins
//!   take its place.
//! - Invoke-bound items are offered only where the catalog says the caller
//!   may invoke (`may_invoke`, coordinator ruling 2026-11-20). An
//!   `open_screen` item is offered to every device that reads the catalog
//!   (ADR 0115, "Surfaces"); `open_settings` waits for the settings
//!   screens' packet.
//! - Cost: one long poll per connected machine that serves the catalog
//!   (the node answers within 25 s), and one read every 5 s while it fails.

use std::collections::HashMap;

use vessel_proto::conversation::CommandName;
use vessel_proto::plugins::{
    Action, Caption, ClientInstallation, CommandContext, ContributionContent, InvokeContext,
    InvokePlugin, PanelContext, PluginCatalog, PluginId, RpcName, RuntimeGeneration,
};
use vessel_proto::{ProjectId, ProtocolVersion, RawJson, SessionId, WorktreeId};

use crate::devices::{MachineKey, Ticket, Tickets};

use super::{Call, Problem, Reply, Request};

mod surfaces;
mod watch;

pub use surfaces::{
    ControlValue, FoundScreen, ScreenAddress, SidebarChoice, SidebarRow, action_needs_admin,
    control_input, needs_admin, screen_call,
};

use watch::Watcher;
pub use watch::{Answered, Pause, QUIET_AFTER, RETRY_AFTER};

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
    /// What choosing it does.
    pub run: PluginRun,
}

/// What a Command Center item does when chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginRun {
    /// Call a declared method.
    Invoke(PluginCall),
    /// Open one of the plugin's screens (ADR 0115, "Surfaces").
    Open(ScreenAddress),
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
            for installation in catalog.catalog.installations() {
                for item in installation.document().contributions() {
                    let ContributionContent::Command {
                        keywords,
                        context,
                        action,
                    } = &item.content
                    else {
                        continue;
                    };
                    let Some(context) = context_at(*context, place, machine) else {
                        continue;
                    };
                    let plugin = installation.address().plugin.clone();
                    let run = match action {
                        Action::Invoke { method, input } if catalog.may_invoke => {
                            PluginRun::Invoke(call_of(
                                machine,
                                installation,
                                method,
                                input.clone(),
                                context,
                            ))
                        }
                        Action::OpenScreen { screen } => PluginRun::Open(ScreenAddress {
                            machine: machine.clone(),
                            plugin: plugin.clone(),
                            screen: screen.clone(),
                        }),
                        Action::Invoke { .. } | Action::OpenSettings { .. } => continue,
                    };
                    commands.push(PluginCommand {
                        id: format!("plugin:{}:{plugin}:{}", machine.as_str(), item.id),
                        title: item.title.as_str().to_owned(),
                        keywords: keywords
                            .iter()
                            .map(Caption::as_str)
                            .collect::<Vec<_>>()
                            .join(" "),
                        run,
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
