//! The app's plugin catalogs (ADR 0115, catalog and invoke), after the
//! page's `plugin-catalog.test.ts` and Paseo's client registry: which
//! machines are watched and what is asked next, what an offline machine
//! keeps, and which Command Center items and slash commands a place
//! offers.

use vessel_proto::conversation::CommandName;
use vessel_proto::plugins::{
    Action, ClientCatalog, ClientDocument, ClientInstallation, CommandContext, Contribution,
    ContributionContent, InvokeContext, PanelContext, PluginAddress, PluginCatalog,
    PluginGeneration, RuntimeGeneration,
};
use vessel_proto::{ErrorCode, NodeId, ProjectId, RawJson, SessionId, WorktreeId};

use super::fixture::{fleet, machine};
use crate::board::Machine;
use crate::connection::{ConnectionEpoch, ConnectionEpochs};
use crate::devices::MachineKey;
use crate::palette::{PaletteEntry, PaletteGroup, PaletteTarget, palette_groups};
use crate::plugins::{
    CatalogAnswered, Catalogs, Place, PluginCall, PluginCommand, PluginRun, Problem, QUIET_AFTER,
    RETRY_AFTER, Reply, Request, context_at, speaks_catalog,
};

fn key(name: &str) -> MachineKey {
    MachineKey::from_book(name)
}

fn epoch(count: usize) -> ConnectionEpoch {
    let mut epochs = ConnectionEpochs::default();
    let mut last = None;
    for _ in 0..count {
        last = Some(
            epochs
                .admit(key("laptop"), NodeId::random())
                .unwrap()
                .epoch(),
        );
    }
    last.unwrap()
}

fn online(name: &str, at: ConnectionEpoch) -> Machine {
    let mut machine = machine(name, true, &[]);
    machine.connection_epoch = Some(at);
    machine
}

fn item(id: &str, title: &str, content: ContributionContent) -> Contribution {
    Contribution {
        id: id.parse().unwrap(),
        title: title.parse().unwrap(),
        content,
    }
}

fn command(context: CommandContext) -> ContributionContent {
    ContributionContent::Command {
        keywords: vec!["memo".parse().unwrap()],
        context,
        action: Action::Invoke {
            method: "notes.open".parse().unwrap(),
            input: RawJson::from_text(r#"{"view":"today"}"#).unwrap(),
        },
    }
}

/// `plugin`'s catalog: a global command, a workspace command, an agent
/// slash command, at `generation`.
fn catalog(plugin: &str, may_invoke: bool, generation: u64) -> PluginCatalog {
    let node = NodeId::random();
    let document = ClientDocument::new(
        vec!["notes.open".parse().unwrap(), "notes.ask".parse().unwrap()],
        vec![
            item("open", "Open notes", command(CommandContext::Global)),
            item(
                "here",
                "Notes for this project",
                command(CommandContext::Workspace),
            ),
            item(
                "ask",
                "Ask notes",
                ContributionContent::SlashCommand {
                    name: "notes".parse::<CommandName>().unwrap(),
                    description: "Ask your notes".parse().unwrap(),
                    argument_hint: Some("question".parse().unwrap()),
                    context: PanelContext::Agent,
                    method: "notes.ask".parse().unwrap(),
                },
            ),
        ],
        Vec::new(),
    )
    .unwrap();
    let installation = ClientInstallation::new(
        PluginAddress {
            node,
            plugin: plugin.parse().unwrap(),
            generation: RuntimeGeneration::random(),
        },
        crate::plugins::CATALOG_PROTOCOL.unwrap(),
        document,
    )
    .unwrap();
    PluginCatalog {
        generation: PluginGeneration::new(generation),
        catalog: ClientCatalog::new(node, vec![installation]).unwrap(),
        may_invoke,
    }
}

fn session_at(machine: &str, project: Option<ProjectId>) -> Place {
    Place::Session {
        machine: key(machine),
        agent: SessionId::random(),
        project,
        worktree: None,
    }
}

/// `catalogs` holding `held` for "laptop", read through its watcher.
fn holding(held: PluginCatalog) -> Catalogs {
    let mut catalogs = Catalogs::default();
    let calls = catalogs.sync(&fleet(vec![online("laptop", epoch(1))]));
    let (machine, read) = calls.into_iter().next().unwrap();
    let _asked = catalogs.answered(&machine, read.ticket, Ok(Reply::Catalog(held)));
    catalogs
}

#[test]
fn only_a_machine_that_answers_and_speaks_the_catalog_is_asked() {
    assert!(speaks_catalog(Some(vessel_proto::PROTOCOL_VERSION)));
    assert!(!speaks_catalog(None));
    let mut catalogs = Catalogs::default();
    let mut old = online("old", epoch(1));
    old.protocol = Some(vessel_proto::ProtocolVersion::new(3));
    let calls = catalogs.sync(&fleet(vec![
        online("laptop", epoch(1)),
        machine("away", false, &[]),
        old,
    ]));
    let asked: Vec<(&str, &Request)> = calls
        .iter()
        .map(|(machine, call)| (machine.as_str(), &call.request))
        .collect();
    assert_eq!(asked, [("laptop", &Request::Catalog)]);
    // The same connection is not asked again.
    assert!(
        catalogs
            .sync(&fleet(vec![online("laptop", epoch(1))]))
            .is_empty()
    );
}

#[test]
fn the_watch_follows_the_catalogs_generation_and_reads_again_when_it_moves() {
    let mut catalogs = Catalogs::default();
    let laptop = key("laptop");
    let calls = catalogs.sync(&fleet(vec![online("laptop", epoch(1))]));
    let read = calls[0].1.ticket;
    let CatalogAnswered::Asked { calls, redraw } =
        catalogs.answered(&laptop, read, Ok(Reply::Catalog(catalog("notes", true, 4))))
    else {
        panic!("a read asks for the watch");
    };
    assert!(redraw);
    assert_eq!(calls[0].1.request, Request::Watch(PluginGeneration::new(4)));
    let watch = calls[0].1.ticket;
    // A late answer to the read is nothing now.
    assert_eq!(
        catalogs.answered(
            &laptop,
            read,
            Ok(Reply::Generation(PluginGeneration::new(4)))
        ),
        CatalogAnswered::Stale
    );
    // No change: a quiet gap, then watch again (app review note 2).
    let CatalogAnswered::Paused { pause, wait } = catalogs.answered(
        &laptop,
        watch,
        Ok(Reply::Generation(PluginGeneration::new(4))),
    ) else {
        panic!("a quiet answer waits");
    };
    assert_eq!(wait, QUIET_AFTER);
    let calls = catalogs.resume(&pause);
    assert_eq!(calls[0].1.request, Request::Watch(PluginGeneration::new(4)));
    // A wait resumes once.
    assert!(catalogs.resume(&pause).is_empty());
    // A change: read again.
    let CatalogAnswered::Asked { calls, .. } = catalogs.answered(
        &laptop,
        calls[0].1.ticket,
        Ok(Reply::Generation(PluginGeneration::new(5))),
    ) else {
        panic!("a moved generation reads again");
    };
    assert_eq!(calls[0].1.request, Request::Catalog);
}

#[test]
fn a_failure_waits_and_reads_again_within_the_connection() {
    let mut catalogs = Catalogs::default();
    let laptop = key("laptop");
    let calls = catalogs.sync(&fleet(vec![online("laptop", epoch(1))]));
    let CatalogAnswered::Asked { calls, .. } = catalogs.answered(
        &laptop,
        calls[0].1.ticket,
        Ok(Reply::Catalog(catalog("notes", true, 1))),
    ) else {
        panic!("a read asks for the watch");
    };
    // The watch fails: wait, then read again (app review note 1).
    let refused = Problem::Refused {
        code: ErrorCode::Internal,
        message: None,
    };
    let CatalogAnswered::Paused { pause, wait } =
        catalogs.answered(&laptop, calls[0].1.ticket, Err(refused.clone()))
    else {
        panic!("a failure waits");
    };
    assert_eq!(wait, RETRY_AFTER);
    // The same connection is not asked again meanwhile; the catalog is kept.
    assert!(
        catalogs
            .sync(&fleet(vec![online("laptop", epoch(1))]))
            .is_empty()
    );
    assert_eq!(catalogs.commands(&Place::Global).len(), 1);
    let again = catalogs.resume(&pause);
    assert_eq!(again[0].1.request, Request::Catalog);
    // The read fails too: another wait, which a new connection outlives.
    let CatalogAnswered::Paused { pause, .. } =
        catalogs.answered(&laptop, again[0].1.ticket, Err(refused))
    else {
        panic!("a failed read waits");
    };
    let fresh = catalogs.sync(&fleet(vec![online("laptop", epoch(2))]));
    assert_eq!(fresh[0].1.request, Request::Catalog);
    assert!(catalogs.resume(&pause).is_empty());
    // Away: the catalog stays (Paseo keeps its registry entry).
    assert!(
        catalogs
            .sync(&fleet(vec![machine("laptop", false, &[])]))
            .is_empty()
    );
    assert_eq!(catalogs.commands(&Place::Global).len(), 1);
    // Back on a new connection: read again.
    let back = catalogs.sync(&fleet(vec![online("laptop", epoch(3))]));
    assert_eq!(back[0].1.request, Request::Catalog);
    // Out of the book: forgotten.
    assert!(catalogs.sync(&fleet(Vec::new())).is_empty());
    assert!(catalogs.commands(&Place::Global).is_empty());
}

#[test]
fn items_need_may_invoke_and_the_context_they_declare() {
    let catalogs = holding(catalog("notes", true, 1));
    let global = catalogs.commands(&Place::Global);
    let ids: Vec<&str> = global.iter().map(|command| command.id.as_str()).collect();
    assert_eq!(ids, ["plugin:laptop:notes:open"]);
    assert_eq!(global[0].title, "Open notes");
    assert_eq!(global[0].keywords, "memo");
    assert_eq!(invoked(&global[0]).invoke.context, InvokeContext::Global);
    assert_eq!(invoked(&global[0]).invoke.method.as_str(), "notes.open");
    // A session with a project offers the workspace item too; a session
    // the node placed in no project does not.
    let project = ProjectId::random();
    let here = catalogs.commands(&session_at("laptop", Some(project)));
    assert_eq!(here.len(), 2);
    assert_eq!(
        invoked(&here[1]).invoke.context,
        InvokeContext::Workspace {
            project,
            worktree: None
        }
    );
    assert_eq!(catalogs.commands(&session_at("laptop", None)).len(), 1);
    // Another machine's session is no workspace for this machine's plugin.
    assert_eq!(
        catalogs.commands(&session_at("desk", Some(project))).len(),
        1
    );
    // A Read device draws none of them.
    let reader = holding(catalog("notes", false, 1));
    assert!(reader.commands(&Place::Global).is_empty());
    assert!(
        reader
            .slash_commands(&key("laptop"), &session_at("laptop", Some(project)))
            .is_empty()
    );
}

#[test]
fn a_slash_command_runs_its_method_with_the_arguments_in_the_session() {
    let catalogs = holding(catalog("notes", true, 1));
    let laptop = key("laptop");
    let project = ProjectId::random();
    let worktree = WorktreeId::random();
    let agent = SessionId::random();
    let place = Place::Session {
        machine: laptop.clone(),
        agent,
        project: Some(project),
        worktree: Some(worktree),
    };
    assert!(catalogs.slash_commands(&laptop, &Place::Global).is_empty());
    let offered = catalogs.slash_commands(&laptop, &place);
    let [notes] = offered.as_slice() else {
        panic!("one command: {offered:?}");
    };
    assert_eq!(notes.name.as_str(), "notes");
    assert_eq!(notes.hint.as_deref(), Some("question"));
    let call = notes.call("what is due", &place).unwrap();
    assert_eq!(call.machine, laptop);
    assert_eq!(call.invoke.method.as_str(), "notes.ask");
    assert_eq!(
        call.invoke.input.as_text(),
        r#"{"arguments":"what is due"}"#
    );
    assert_eq!(
        call.invoke.context,
        InvokeContext::Agent {
            project,
            worktree: Some(worktree),
            agent
        }
    );
    assert!(context_at(CommandContext::Agent, &Place::Global, &laptop).is_none());
}

#[test]
fn a_method_calls_answer_is_its_own_and_not_the_watchers() {
    let mut catalogs = holding(catalog("notes", true, 1));
    let laptop = key("laptop");
    let call = invoked(&catalogs.commands(&Place::Global)[0]).clone();
    let (machine, asked) = catalogs.invoke(call.clone()).unwrap();
    assert_eq!(machine, laptop);
    assert_eq!(asked.request, Request::Invoke(call.invoke));
    let output = RawJson::from_text(r#"{"ok":true}"#).unwrap();
    assert_eq!(
        catalogs.answered(&laptop, asked.ticket, Ok(Reply::Output(output.clone()))),
        CatalogAnswered::Invoked(Ok(output))
    );
    // Answered once.
    assert_eq!(
        catalogs.answered(&laptop, asked.ticket, Ok(Reply::Done)),
        CatalogAnswered::Stale
    );
}

#[test]
fn plugin_items_are_a_section_per_plugin_after_the_apps_own_groups() {
    let catalogs = holding(catalog("notes", true, 1));
    let commands = catalogs.commands(&Place::Global);
    let mut entries: Vec<PaletteEntry<(), ()>> = vec![PaletteEntry::plugin(&commands[0], 0)];
    entries[0].id = "first".to_owned();
    entries.push(PaletteEntry::plugin(&commands[0], 1));
    let groups = palette_groups(&entries, "");
    let order: Vec<PaletteGroup> = groups.iter().map(|(group, _)| *group).collect();
    assert_eq!(order, [PaletteGroup::Plugin(0), PaletteGroup::Plugin(1)]);
    let PaletteTarget::Plugin { command, plugin } = &groups[0].1[0].target else {
        panic!("a plugin item");
    };
    assert_eq!(command, "plugin:laptop:notes:open");
    assert_eq!(plugin.as_str(), "notes");
    // The keywords are searched.
    assert_eq!(palette_groups(&entries, "memo").len(), 2);
}

/// The call an invoking item runs.
fn invoked(command: &PluginCommand) -> &PluginCall {
    let PluginRun::Invoke(call) = &command.run else {
        panic!("an invoking item: {command:?}");
    };
    call
}

/// A catalog whose plugin declares a screen, a sidebar row opening it, an
/// item opening it and an item calling a method (ADR 0115, "Surfaces").
fn screens(may_invoke: bool) -> PluginCatalog {
    use vessel_proto::plugins::{ControlContent, SidebarPlacement, Surface};

    let node = NodeId::random();
    let text = |id: &str| vessel_proto::plugins::Control {
        id: id.parse().unwrap(),
        content: ControlContent::Text {
            text: "Notes".parse().unwrap(),
        },
    };
    let opening = ContributionContent::Command {
        keywords: Vec::new(),
        context: CommandContext::Global,
        action: Action::OpenScreen {
            screen: "main".parse().unwrap(),
        },
    };
    let document = ClientDocument::new(
        vec!["notes.open".parse().unwrap()],
        vec![
            item(
                "main",
                "Notes",
                ContributionContent::Screen {
                    surface: Surface {
                        controls: vec![text("intro")],
                    },
                },
            ),
            item(
                "main",
                "Notes",
                ContributionContent::Sidebar {
                    placement: SidebarPlacement::Header,
                    screen: "main".parse().unwrap(),
                },
            ),
            item("show", "Show notes", opening),
            item("open", "Open notes", command(CommandContext::Global)),
        ],
        Vec::new(),
    )
    .unwrap();
    let installation = ClientInstallation::new(
        PluginAddress {
            node,
            plugin: "notes".parse().unwrap(),
            generation: RuntimeGeneration::random(),
        },
        document.required_client(),
        document,
    )
    .unwrap();
    PluginCatalog {
        generation: PluginGeneration::new(1),
        catalog: ClientCatalog::new(node, vec![installation]).unwrap(),
        may_invoke,
    }
}

/// Catalogs holding `held` for each of `machines`.
fn holding_on(machines: &[&str], held: &PluginCatalog) -> Catalogs {
    let mut catalogs = Catalogs::default();
    let book = machines.iter().map(|name| online(name, epoch(1))).collect();
    for (machine, read) in catalogs.sync(&fleet(book)) {
        let _asked = catalogs.answered(&machine, read.ticket, Ok(Reply::Catalog(held.clone())));
    }
    catalogs
}

#[test]
fn a_screen_is_found_and_opened_by_any_device_that_reads_the_catalog() {
    use crate::plugins::{ScreenAddress, needs_admin};

    let catalogs = holding_on(&["laptop"], &screens(false));
    let at = ScreenAddress {
        machine: key("laptop"),
        plugin: "notes".parse().unwrap(),
        screen: "main".parse().unwrap(),
    };
    let found = catalogs.screen(&at).unwrap();
    assert_eq!(found.title, "Notes");
    assert!(!found.may_invoke);
    assert!(!needs_admin(found.surface));
    let other = ScreenAddress {
        screen: "elsewhere".parse().unwrap(),
        ..at.clone()
    };
    assert!(catalogs.screen(&other).is_none());
    // Without admin: the item opening the screen, not the one calling a
    // method; with admin both, in declaration order.
    let read = catalogs.commands(&Place::Global);
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].run, PluginRun::Open(at));
    let admin = holding_on(&["laptop"], &screens(true)).commands(&Place::Global);
    let ids: Vec<&str> = admin.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(
        ids,
        ["plugin:laptop:notes:show", "plugin:laptop:notes:open"]
    );
}

#[test]
fn a_sidebar_row_is_drawn_once_across_machines_and_opens_the_right_one() {
    use std::collections::HashMap;

    use vessel_proto::plugins::SidebarPlacement;

    use crate::plugins::{ScreenAddress, SidebarChoice};

    let catalogs = holding_on(&["laptop", "server"], &screens(false));
    let both = [key("laptop"), key("server")];
    let remembered = HashMap::new();
    let choice = SidebarChoice {
        online: &both,
        current: None,
        remembered: &remembered,
    };
    let rows = catalogs.sidebar_rows(SidebarPlacement::Header, &choice);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Notes");
    assert_eq!(rows[0].target.machine, key("laptop"));
    assert!(
        catalogs
            .sidebar_rows(SidebarPlacement::Footer, &choice)
            .is_empty()
    );
    // The screen open now wins, then the machine last shown.
    let open = ScreenAddress {
        machine: key("server"),
        plugin: "notes".parse().unwrap(),
        screen: "main".parse().unwrap(),
    };
    let now = SidebarChoice {
        current: Some(&open),
        ..choice
    };
    assert_eq!(
        catalogs.sidebar_rows(SidebarPlacement::Header, &now)[0]
            .target
            .machine,
        key("server")
    );
    let last = HashMap::from([(
        ("notes".parse().unwrap(), "main".parse().unwrap()),
        key("server"),
    )]);
    let shown = SidebarChoice {
        remembered: &last,
        ..choice
    };
    assert_eq!(
        catalogs.sidebar_rows(SidebarPlacement::Header, &shown)[0]
            .target
            .machine,
        key("server")
    );
    // An away machine's rows are left out.
    let away = [key("server")];
    let only = SidebarChoice {
        online: &away,
        ..choice
    };
    assert_eq!(
        catalogs.sidebar_rows(SidebarPlacement::Header, &only)[0]
            .target
            .machine,
        key("server")
    );
    // The screen header's host filter lists the connected machines that
    // declare the screen, in book order.
    let (notes, main) = (open.plugin.clone(), open.screen.clone());
    assert_eq!(catalogs.screen_machines(&notes, &main, &both), both);
    assert_eq!(catalogs.screen_machines(&notes, &main, &away), away);
    assert!(
        catalogs
            .screen_machines(&notes, &"other".parse().unwrap(), &both)
            .is_empty()
    );
}
