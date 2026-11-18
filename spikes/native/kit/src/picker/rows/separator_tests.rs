//! A row's separator (TRACKER 807): Paseo's `MenuSeparator` between the
//! labels and Create label, 1 high with 4 above and below, the list's full
//! width; none above the first row shown.
use super::super::*;
use crate::{Scheme, Tokens};
use gpui::{Render, TestAppContext, VisualTestContext, div, px};

struct Probe {
    list: ComboboxState,
    rows: Vec<ComboRow>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
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
            "labels", &self.list, &header, &self.rows, &handlers, 232.0, &tokens, window, cx,
        );
        div().size_full().pt(px(40.0)).children(list)
    }
}

fn create() -> ComboRow {
    ComboRow {
        opens: true,
        separated: true,
        ..ComboRow::new("Create label")
    }
}

fn setup(cx: &mut TestAppContext, rows: Vec<ComboRow>) -> &mut VisualTestContext {
    cx.update(|cx| {
        let initialized = crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark));
        assert!(initialized.is_ok(), "faces and input globals register");
    });
    let (view, cx) = cx.add_window_view(move |window, cx| Probe {
        list: ComboboxState::new(window, cx),
        rows,
    });
    view.update_in(cx, |probe, window, cx| {
        probe.list.open_beside(None, window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    cx
}

#[gpui::test]
fn a_separated_row_has_paseos_rule_between_it_and_the_row_before(cx: &mut TestAppContext) {
    let cx = setup(
        cx,
        vec![ComboRow::new("Docs"), ComboRow::new("Release"), create()],
    );
    let before = cx.debug_bounds("combobox-row-1").unwrap();
    let rule = cx.debug_bounds("combobox-rule-2").unwrap();
    let after = cx.debug_bounds("combobox-row-2").unwrap();
    assert_eq!(rule.size.height, px(1.0));
    // 4 above and below it.
    let gap = |upper: Pixels, lower: Pixels| {
        f32::from(upper).mul_add(-1.0, f32::from(lower)).to_bits()
    };
    assert_eq!(gap(before.bottom(), rule.top()), 4.0_f32.to_bits());
    assert_eq!(gap(rule.bottom(), after.top()), 4.0_f32.to_bits());
    // The list's full width, as the rows.
    assert_eq!(rule.left(), after.left());
    assert_eq!(rule.size.width, after.size.width);
    // No other row has one.
    assert!(cx.debug_bounds("combobox-rule-1").is_none());
}

#[gpui::test]
fn a_separated_row_shown_first_has_none(cx: &mut TestAppContext) {
    let cx = setup(cx, vec![create()]);
    assert!(cx.debug_bounds("combobox-row-0").is_some());
    assert!(cx.debug_bounds("combobox-rule-0").is_none());
}
