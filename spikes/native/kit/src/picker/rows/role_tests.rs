//! A list's rows in GPUI's real accessibility tree (TRACKER 808): a
//! combobox's option is a button named by its words, with no checked state
//! (Paseo's `ComboboxItem`); a menu's rows are menu items, and one that
//! draws a check says whether it is checked (Paseo's `MenuItem`).
use super::super::*;
use crate::a11y_snapshot::{A11yNode, activate, nodes};
use crate::{Scheme, Tokens};
use gpui::{Render, TestAppContext, VisualTestContext, div, px};

struct Probe {
    list: ComboboxState,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let row = |label: &str, role: RowRole, checked: bool| ComboRow {
            checked,
            role,
            ..ComboRow::new(label.to_owned())
        };
        let rows = vec![
            ComboRow {
                detail: Some("Fast".into()),
                ..ComboRow::new("Opus")
            },
            row("Docs", RowRole::CheckedItem, true),
            row("Release", RowRole::CheckedItem, false),
            row("Create label", RowRole::MenuItem, false),
        ];
        let header = ComboHeader {
            title: None,
            back: false,
            search: false,
            empty: "Empty".into(),
            note: None,
            side: Side::Below,
        };
        let handlers = ComboHandlers {
            state: |this: &mut Self| &mut this.list,
            chose: Rc::new(|_, _, _, _| {}),
            back: None,
        };
        let list = combobox_inline(
            "rows", &self.list, &header, &rows, &handlers, 232.0, &tokens, window, cx,
        );
        div().size_full().pt(px(40.0)).children(list)
    }
}

fn tree(cx: &mut TestAppContext) -> (Vec<A11yNode>, Vec<A11yNode>) {
    cx.update(|cx| {
        let initialized = crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok(), "faces and input globals register");
    });
    let (view, cx): (_, &mut VisualTestContext) = cx.add_window_view(|window, cx| Probe {
        list: ComboboxState::new(window, cx),
    });
    view.update_in(cx, |probe, window, cx| {
        probe.list.open_beside(None, window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    activate(cx);
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    cx.update(|window, _| {
        (
            nodes(window, "Button").unwrap(),
            nodes(window, "MenuItem").unwrap(),
        )
    })
}

#[gpui::test]
fn options_are_buttons_and_a_menus_rows_say_their_check(cx: &mut TestAppContext) {
    let (buttons, items) = tree(cx);
    // Paseo's ComboboxItem: a button named by its words, no checked state.
    let option = buttons
        .iter()
        .find(|node| node.label.as_deref() == Some("Opus Fast"))
        .unwrap();
    assert_eq!(option.toggled, None);
    assert!(option.actions.iter().any(|action| action == "Click"));
    // Paseo's MenuItem: checked only where the row draws a check.
    let said: Vec<(Option<&str>, Option<&str>)> = items
        .iter()
        .map(|node| (node.label.as_deref(), node.toggled.as_deref()))
        .collect();
    assert_eq!(
        said,
        [
            (Some("Create label"), None),
            (Some("Docs"), Some("True")),
            (Some("Release"), Some("False")),
        ]
    );
}
