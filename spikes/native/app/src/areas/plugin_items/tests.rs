//! The app's plugin items over a detached link (ADR 0115, catalog and
//! invoke): a serving machine's catalog is read and then watched from its
//! generation, its Command Center item runs `plugin.invoke`, and a refused
//! call is a toast in the node's words.

use gpui::TestAppContext;
use vessel_app_link::plugins::Answer;
use vessel_app_link::{Command, Link};
use vessel_app_view::plugins::{Problem, Reply, Request};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::{Fleet, Machine, Reach};
use vessel_proto::plugins::{
    Action, ClientCatalog, ClientDocument, ClientInstallation, CommandContext, Contribution,
    ContributionContent, PluginAddress, PluginCatalog, PluginGeneration, RuntimeGeneration,
};
use vessel_proto::{ErrorCode, NodeId, RawJson};

use crate::shell::Shell;

fn fleet() -> Fleet {
    Fleet {
        machines: vec![Machine {
            connection_epoch: None,
            node: None,
            name: "lab".to_owned(),
            reach: Reach::Online,
            sessions: Vec::new(),
            agents: Vec::new(),
            protocol: Some(vessel_proto::PROTOCOL_VERSION),
            version: None,
        }],
        read_at: None,
    }
}

/// "notes" with one global Command Center item, at generation 2.
fn catalog() -> PluginCatalog {
    let node = NodeId::random();
    let open = Contribution {
        id: "open".parse().unwrap(),
        title: "Open notes".parse().unwrap(),
        content: ContributionContent::Command {
            keywords: Vec::new(),
            context: CommandContext::Global,
            action: Action::Invoke {
                method: "notes.open".parse().unwrap(),
                input: RawJson::empty_object(),
            },
        },
    };
    let document =
        ClientDocument::new(vec!["notes.open".parse().unwrap()], vec![open], Vec::new()).unwrap();
    let installation = ClientInstallation::new(
        PluginAddress {
            node,
            plugin: "notes".parse().unwrap(),
            generation: RuntimeGeneration::random(),
        },
        vessel_app_view::plugins::CATALOG_PROTOCOL.unwrap(),
        document,
    )
    .unwrap();
    PluginCatalog {
        generation: PluginGeneration::new(2),
        catalog: ClientCatalog::new(node, vec![installation]).unwrap(),
        may_invoke: true,
    }
}

#[gpui::test]
fn a_serving_machines_item_runs_its_method_and_a_refusal_is_a_toast(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, mut commands) = Link::detached(16).unwrap();
    let (shell, cx) = cx.add_window_view(move |window, cx| Shell::with_test_link(window, cx, link));
    shell.update_in(cx, |shell, _window, _cx| {
        shell.fleet = fleet();
        shell.plugin_catalogs_fleet();
    });
    let Ok(Command::PluginCatalog(read)) = commands.try_recv() else {
        panic!("the catalog is read");
    };
    assert_eq!(read.request, Request::Catalog);
    shell.update_in(cx, |shell, _window, cx| {
        shell.plugin_catalog_answer(
            Answer {
                machine: read.machine.clone(),
                ticket: read.ticket,
                result: Ok(Reply::Catalog(catalog())),
            },
            cx,
        );
        // The same connection is not read twice.
        shell.plugin_catalogs_fleet();
    });
    let Ok(Command::PluginCatalog(watch)) = commands.try_recv() else {
        panic!("then watched");
    };
    assert_eq!(watch.request, Request::Watch(PluginGeneration::new(2)));
    assert!(commands.try_recv().is_err());
    shell.update_in(cx, |shell, _window, _cx| {
        let items = shell.areas.plugin_catalogs.commands(&shell.plugin_place());
        assert_eq!(items.len(), 1);
        shell.run_plugin_command(&items[0].id);
    });
    let Ok(Command::PluginCatalog(invoke)) = commands.try_recv() else {
        panic!("the item runs");
    };
    let Request::Invoke(call) = &invoke.request else {
        panic!("plugin.invoke");
    };
    assert_eq!(call.method.as_str(), "notes.open");
    shell.update_in(cx, |shell, _window, cx| {
        shell.plugin_catalog_answer(
            Answer {
                machine: invoke.machine.clone(),
                ticket: invoke.ticket,
                result: Err(Problem::Refused {
                    code: ErrorCode::Unavailable,
                    message: Some("plugin notes is restarting".to_owned()),
                }),
            },
            cx,
        );
        let toast = shell.areas.toasts.shown().unwrap();
        assert_eq!(toast.tone, ToastTone::Error);
        assert_eq!(toast.text, "plugin notes is restarting");
    });
}
