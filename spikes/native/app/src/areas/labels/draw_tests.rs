//! The manager body as Paseo's: one rail of rows with the pencil at its end,
//! the host filter only with several hosts, the edit view's header back,
//! swatch row and footer, no focus taken by the search on open, the
//! pencil's and swatches' accessible names, and a pending Save.
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
fn open<'a>(
    cx: &'a mut TestAppContext,
    hosts: &[&str],
) -> (gpui::Entity<Shell>, &'a mut VisualTestContext) {
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

/// The ten swatches' selectors, in `Color::ALL`'s order.
const SWATCHES: [&str; 10] = [
    "label-swatch-Violet",
    "label-swatch-Sky",
    "label-swatch-Emerald",
    "label-swatch-Orange",
    "label-swatch-Pink",
    "label-swatch-Indigo",
    "label-swatch-Teal",
    "label-swatch-Red",
    "label-swatch-Amber",
    "label-swatch-Blue",
];

/// The distance from `from` to `to` (`mul_add`: no arithmetic operator).
fn apart(from: gpui::Pixels, to: gpui::Pixels) -> u32 {
    f32::from(from).mul_add(-1.0, f32::from(to)).to_bits()
}

#[gpui::test]
fn rows_hold_one_rail_and_the_search_takes_no_focus_on_open(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx, &["fixture"]);
    // Paseo passes no `autoFocus`: typing before a click reaches no field.
    cx.simulate_input("Rea");
    cx.run_until_parked();
    shell.read_with(cx, |shell, cx| {
        assert!(shell.areas.labels.manager.query().is_empty());
        assert_eq!(shell.areas.labels.search.text(cx), "");
    });
    // One host: no filter (Paseo's `hosts.length > 1`).
    assert!(cx.debug_bounds("labels-host-filter").is_none());
    let list = cx.debug_bounds("labels-list").unwrap();
    for (row, pencil) in [
        ("label-row-Ready", "label-edit-Ready"),
        ("label-row-Blocked", "label-edit-Blocked"),
    ] {
        let row = cx.debug_bounds(row).unwrap();
        let pencil = cx.debug_bounds(pencil).unwrap();
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
        let dismissal = surface::Dismiss::capture(state.opening.unwrap(), state.manager.state());
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
    for swatch in SWATCHES {
        let swatch = cx.debug_bounds(swatch).unwrap();
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

/// GPUI's real accessibility snapshot, read only for the names below; no raw
/// snapshot is printed.
fn snapshot(cx: &mut VisualTestContext) -> String {
    cx.update(|window, _| window.debug_a11y_tree_json().unwrap())
}

#[gpui::test]
fn the_pencil_and_the_swatches_are_named_as_paseos(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx, &["fixture"]);
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    shell.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    // The pencil: a button named "Edit {name}" (built by the same catalog
    // call, so Fluent's isolation marks around the name match).
    let named = |name: &str| {
        let words =
            vessel_app_view::localization::Translator::get(vessel_app_view::localization::ENGLISH)
                .unwrap();
        let text = words.labels().edit_named(name).unwrap();
        assert!(text.starts_with("Edit ") && text.contains(name));
        format!(r#""label": "{text}""#)
    };
    let listing = snapshot(cx);
    assert!(listing.contains(&named("Ready")));
    assert!(listing.contains(&named("Blocked")));
    let pencil = cx.debug_bounds("label-edit-Ready").unwrap();
    cx.simulate_click(pencil.center(), Modifiers::none());
    cx.run_until_parked();
    // Ten radio swatches named by colour; only the chosen one is checked.
    let editing = snapshot(cx);
    assert_eq!(editing.matches(r#""role": "RadioButton""#).count(), 10);
    assert!(editing.contains(r#""label": "Violet""#));
    assert!(editing.contains(r#""label": "Blue""#));
    assert_eq!(editing.matches(r#""toggled": "True""#).count(), 1);
    assert_eq!(editing.matches(r#""toggled": "False""#).count(), 9);
}

#[gpui::test]
fn a_save_under_way_refuses_a_second_save(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx, &["fixture"]);
    let pencil = cx.debug_bounds("label-edit-Ready").unwrap();
    cx.simulate_click(pencil.center(), Modifiers::none());
    cx.run_until_parked();
    let blue = cx.debug_bounds("label-swatch-Blue").unwrap();
    cx.simulate_click(blue.center(), Modifiers::none());
    cx.run_until_parked();
    shell.update(cx, |shell, cx| {
        let state = &mut shell.areas.labels;
        let catalog = state.hosts.first().unwrap().catalog.clone();
        let ticket = vessel_app_view::labels::Tickets::default().issue().unwrap();
        let replacement = state
            .manager
            .save(HostState::Online, &catalog, ticket)
            .unwrap();
        assert_eq!(replacement.color, Color::Blue);
        cx.notify();
    });
    cx.run_until_parked();
    // Save keeps its place at the footer's end while pending; a press on it
    // starts nothing (the kit's loading button attaches no click).
    let footer = cx.debug_bounds("labels-footer").unwrap();
    let end = gpui::point(
        px(f32::from(footer.right()).mul_add(1.0, -20.0)),
        footer.center().y,
    );
    cx.simulate_click(end, Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _cx| {
        assert!(matches!(
            shell.areas.labels.manager.state(),
            ManagerState::Saving { .. }
        ));
        assert!(shell.areas.labels.error.is_none());
    });
}
