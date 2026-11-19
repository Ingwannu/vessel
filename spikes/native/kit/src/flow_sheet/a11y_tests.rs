//! The flow sheet in GPUI's accessibility tree (app review R1 of TRACKER
//! 234): the panel is a dialog named by its owner, each row a button named
//! by its title, a row that cannot be chosen offers no Click, and a later
//! page's back arrow is a button named in the app's words.
use super::*;
use crate::a11y_snapshot::{activate, nodes};
use crate::{Scheme, SchemeChoice};
use gpui::{Render, TestAppContext, VisualTestContext};

struct Probe;

fn row(title: &str, disabled: bool) -> FlowRow {
    FlowRow {
        mark: div().into_any_element(),
        title: title.to_owned().into(),
        subtitle: None,
        active: false,
        disabled,
    }
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let open = flow_row("row-open", row("Search for directory", false), &tokens)
            .on_click(|_event, _window, _cx| {});
        let closed = flow_row("row-closed", row("New directory", true), &tokens);
        let frame = FlowSheetFrame {
            label: "Add project: Search for directory".into(),
            title: "Search for directory".into(),
            context: Some("laptop".into()),
            back: true,
            field: div().into_any_element(),
            field_shown: true,
            results: vec![open.into_any_element(), closed.into_any_element()],
            scroll: gpui::ScrollHandle::new(),
            hints: Vec::new(),
        };
        let handlers = FlowHandlers {
            on_key: |_probe: &mut Self, _key, _window, _cx| {},
            back: |_probe: &mut Self, _window, _cx| {},
            close: |_probe: &mut Self, _window, _cx| {},
        };
        div()
            .size_full()
            .child(flow_sheet(frame, handlers, &tokens, window, cx))
    }
}

fn redraw(view: &gpui::Entity<Probe>, cx: &mut VisualTestContext) {
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
}

#[gpui::test]
fn the_sheet_its_rows_and_back_are_named(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|_window, _cx| Probe);
    cx.run_until_parked();
    activate(cx);
    redraw(&view, cx);
    let (dialogs, buttons) = cx.update(|window, _| {
        (nodes(window, "Dialog").unwrap(), nodes(window, "Button").unwrap())
    });
    assert_eq!(
        dialogs.iter().map(|node| node.label.as_deref()).collect::<Vec<_>>(),
        [Some("Add project: Search for directory")]
    );
    let named = |name: &str| buttons.iter().find(|node| node.label.as_deref() == Some(name));
    let open = named("Search for directory").expect("a selectable row is a named button");
    assert!(open.actions.iter().any(|action| action == "Click"));
    let closed = named("New directory").expect("a disabled row is still a named button");
    assert!(!closed.actions.iter().any(|action| action == "Click"));
    assert!(named("Back").is_some(), "the back arrow is a button named Back");
}
