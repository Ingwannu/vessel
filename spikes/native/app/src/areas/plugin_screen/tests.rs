//! A plugin screen's menu button as assistive technology meets it (app
//! review R1 on P3b-1 N1, as the Settings › Plugins row menu's P2 R1): a
//! Button that takes a Click and says whether its menu is open, and a click
//! that opens the menu once rather than toggling it shut.

use gpui::{AppContext as _, Entity, IntoElement, Render, TestAppContext, point};
use vessel_app_kit::ui::{Context, ParentElement as _, Styled as _, Window, div, px};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_link::Link;
use vessel_app_view::devices::MachineKey;
use vessel_app_view::plugins::{Reply, ScreenAddress};
use vessel_app_view::{Fleet, Machine, Reach};
use vessel_proto::NodeId;
use vessel_proto::RawJson;
use vessel_proto::plugins::{
    Action, Availability, Button, ButtonBehavior, ClientCatalog, ClientDocument,
    ClientInstallation, Contribution, ContributionContent, ContributionId, Control, ControlContent,
    MenuEntry, PluginAddress, PluginCatalog, PluginGeneration, RuntimeGeneration, Surface,
};

use crate::shell::Shell;

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            div()
                .size_full()
                .child(super::render(shell, &Tokens::of(Scheme::Dark), window, cx))
        })
    }
}

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

fn more() -> ContributionId {
    "more".parse().unwrap()
}

/// "notes" declaring the screen "main" with one menu button, "More".
fn catalog() -> PluginCatalog {
    let node = NodeId::random();
    let ping = MenuEntry::Item {
        id: "ping".parse().unwrap(),
        title: "Ping".parse().unwrap(),
        availability: Availability::Enabled,
        behavior: ButtonBehavior::Action {
            action: Action::Invoke {
                method: "notes.ping".parse().unwrap(),
                input: RawJson::empty_object(),
            },
        },
    };
    let button = Control {
        id: more(),
        content: ControlContent::Button {
            button: Button {
                title: "More".parse().unwrap(),
                availability: Availability::Enabled,
                behavior: ButtonBehavior::Menu { items: vec![ping] },
            },
        },
    };
    let screen = Contribution {
        id: "main".parse().unwrap(),
        title: "Notes".parse().unwrap(),
        content: ContributionContent::Screen {
            surface: Surface {
                controls: vec![button],
            },
        },
    };
    let document = ClientDocument::new(
        vec!["notes.ping".parse().unwrap()],
        vec![screen],
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
        may_invoke: true,
    }
}

/// The menu button's accessibility node after a fresh frame.
fn button_node(cx: &mut gpui::VisualTestContext) -> String {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    let nodes: Vec<String> = json
        .split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .filter(|aria| aria.contains("\"label\": \"More\""))
        .collect();
    assert_eq!(nodes.len(), 1, "one menu button: {json}");
    nodes.into_iter().next().unwrap()
}

#[gpui::test]
fn a_menu_button_takes_a_click_says_expanded_and_opens_its_menu_once(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, _commands) = Link::detached(16).unwrap();
    let (probe, cx) = cx.add_window_view(move |window, cx| Probe {
        shell: cx.new(|cx| Shell::with_test_link(window, cx, link)),
    });
    let shell = probe.update(cx, |probe, _cx| probe.shell.clone());
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    cx.update(|window, cx| {
        shell.update(cx, |shell, cx| {
            shell.fleet = fleet();
            let reads = shell.areas.plugin_catalogs.sync(&shell.fleet);
            for (machine, read) in reads {
                let _read = shell.areas.plugin_catalogs.answered(
                    &machine,
                    read.ticket,
                    Ok(Reply::Catalog(catalog())),
                );
            }
            let at = ScreenAddress {
                machine: MachineKey::from_book("lab"),
                plugin: "notes".parse().unwrap(),
                screen: "main".parse().unwrap(),
            };
            shell.open_plugin_screen(at, window, cx);
        });
    });
    let closed = button_node(cx);
    assert!(closed.contains(r#""Button""#), "{closed}");
    assert!(closed.contains(r#""Click""#), "{closed}");
    assert!(closed.contains(r#""expanded": false"#), "{closed}");
    cx.update(|window, cx| {
        shell.update(cx, |shell, cx| {
            let at = point(px(200.0), px(120.0));
            // No press went down first: assistive technology's Click.
            shell.plugin_screen_menu_activate(&more(), at, window, cx);
            let area = &shell.areas.plugin_screen;
            assert!(area.menu.is_open());
            assert_eq!(area.menu_for, Some(more()));
            // The click that ends a pointer's press leaves it open.
            shell.plugin_screen_menu_activate(&more(), at, window, cx);
            let area = &shell.areas.plugin_screen;
            assert!(area.menu.is_open());
            assert_eq!(area.menu_for, Some(more()));
        });
    });
    let open = button_node(cx);
    assert!(open.contains(r#""expanded": true"#), "{open}");
}
