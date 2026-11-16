//! The manager body as Paseo's: one rail of rows with the pencil at its end,
//! the host filter only with several hosts, the edit view's header back,
//! swatch row and footer, and no focus taken by the search on open.
use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, px};
use vessel_app_view::{
    connection::{ConnectionEpochs, MachineKey},
    labels::{Catalog, Color, Label, ManagerState},
    machine_book::Name,
};
use vessel_proto::NodeId;

fn label(name: &str, color: Color) -> Label {
    Label {
        name: name.parse().unwrap(),
        color,
    }
}

/// An open manager on `hosts` online hosts, the first one shown.
fn open(cx: &mut TestAppContext, hosts: &[&str]) -> (gpui::Entity<Shell>, &mut VisualTestContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    cx.update(|window, _| window.activate_window());
    let hosts: Vec<String> = hosts.iter().map(|host| (*host).to_owned()).collect();
    shell.update_in(cx, move |shell, window, cx| {
        shell.labels_open(window, cx);
        let mut epochs = ConnectionEpochs::default();
        let state = &mut shell.areas.labels;
        state.hosts = hosts
            .iter()
            .map(|route| {
                let context = epochs
                    .admit(MachineKey::from_book(route), NodeId::random())
                    .unwrap();
                Host {
                    key: context.node(),
                    route: Name::parse(route).unwrap(),
                    connection: Some(context),
                    state: HostState::Online,
                    catalog: Catalog::snapshot([
                        label("Ready", Color::Violet),
                        label("Blocked", Color::Orange),
                    ]),
                }
            })
            .collect();
        let first = state.hosts.first().unwrap();
        state.route = Some(first.route.clone());
        let key = first.key;
        state.manager.select(key).unwrap();
        cx.notify();
    });
    cx.run_until_parked();
    (shell, cx)
}

/// The distance from `from` to `to` (`mul_add`: no arithmetic operator).
fn apart(from: gpui::Pixels, to: gpui::Pixels) -> u32 {
    f32::from(from).mul_add(-1.0, f32::from(to)).to_bits()
}

#[gpui::test]
fn rows_hold_one_rail_and_the_search_takes_no_focus_on_open(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx, &["fixture"]);
    shell.update_in(cx, |shell, window, cx| {
        assert!(
            !shell
                .areas
                .labels
                .search
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    // One host: no filter (Paseo's `hosts.length > 1`).
    assert!(cx.debug_bounds("labels-host-filter").is_none());
    let list = cx.debug_bounds("labels-list").unwrap();
    for name in ["Ready", "Blocked"] {
        let row = cx.debug_bounds(&format!("label-row-{name}")).unwrap();
        let pencil = cx.debug_bounds(&format!("label-edit-{name}")).unwrap();
        // The dot leads at the list's edge and the pencil ends on its other edge.
        assert_eq!(row.left(), list.left());
        assert_eq!(row.size.width, list.size.width);
        assert!(row.size.height >= px(32.0));
        assert_eq!(pencil.right(), row.right());
        assert_eq!(pencil.size.width, px(14.0));
    }
    // Rows 4 apart.
    let first = cx.debug_bounds("label-row-Blocked").unwrap();
    let second = cx.debug_bounds("label-row-Ready").unwrap();
    assert_eq!(apart(first.bottom(), second.top()), 4.0_f32.to_bits());
}

#[gpui::test]
fn the_host_filter_shows_with_several_hosts_and_opens_the_host_list(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx, &["fixture", "laptop"]);
    let filter = cx.debug_bounds("labels-host-filter").unwrap();
    let list = cx.debug_bounds("labels-list").unwrap();
    // Above the rows, from the same edge, sized to its content.
    assert!(filter.bottom() <= list.top());
    assert_eq!(filter.left(), list.left());
    assert!(filter.size.width < list.size.width);
    cx.simulate_click(filter.center(), Modifiers::none());
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.areas.labels.host_list.is_open());
        let state = &shell.areas.labels;
        let dismissal =
            super::surface::Dismiss::capture(state.opening.unwrap(), state.manager.state());
        shell.labels_close(dismissal, window, cx);
        assert!(!shell.areas.labels.host_list.is_open());
    });
}

#[gpui::test]
fn the_pencil_opens_the_edit_view_with_header_back_swatches_and_footer(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx, &["fixture"]);
    let pencil = cx.debug_bounds("label-edit-Ready").unwrap();
    cx.simulate_click(pencil.center(), Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _cx| {
        assert!(matches!(
            shell.areas.labels.manager.state(),
            ManagerState::Editing(_)
        ));
    });
    assert!(cx.debug_bounds("dialog-header-search").is_none());
    assert!(cx.debug_bounds("dialog-header-back").is_some());
    assert!(cx.debug_bounds("labels-footer").is_some());
    assert!(cx.debug_bounds("labels-host-filter").is_none());
    // Ten 20-point swatches; choosing one recolours the draft.
    for color in Color::ALL {
        let swatch = cx.debug_bounds(&format!("label-swatch-{color:?}")).unwrap();
        assert_eq!(swatch.size.width, px(20.0));
        assert_eq!(swatch.size.height, px(20.0));
    }
    let blue = cx.debug_bounds("label-swatch-Blue").unwrap();
    cx.simulate_click(blue.center(), Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _cx| {
        let draft = shell.areas.labels.manager.state().draft().unwrap();
        assert_eq!(draft.color(), Color::Blue);
    });
    // The header's way back returns to the list.
    let back = cx.debug_bounds("dialog-header-back").unwrap();
    cx.simulate_click(back.center(), Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _cx| {
        assert!(matches!(
            shell.areas.labels.manager.state(),
            ManagerState::Listing
        ));
    });
    assert!(cx.debug_bounds("dialog-header-search").is_some());
}
