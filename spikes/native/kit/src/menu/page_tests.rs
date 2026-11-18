//! The form page (TRACKER 792): its field in a row's box on the rows'
//! rail, its action row under the separator, the hint under that, and a
//! press that acts only while the row can be chosen.
use super::*;
use crate::menu::ItemStatus;
use crate::{Scheme, SchemeChoice};
use gpui::{Entity, Modifiers, Render, TestAppContext, VisualTestContext};

struct Probe {
    field: TextField,
    pressed: usize,
    action: MenuItem,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let page = FormPage::new(&self.field, self.action.clone())
            .with_body(
                div()
                    .debug_selector(|| "probe-body".to_owned())
                    .h(px(20.0)),
            )
            .with_hint(Some("Unable to update label".into()));
        let act: Rc<dyn Fn(&mut Self, &mut Window, &mut Context<'_, Self>)> =
            Rc::new(|probe: &mut Self, _window, _cx| probe.pressed = probe.pressed.saturating_add(1));
        let rows = form_rows(page, act, false, &tokens, cx);
        div()
            .size_full()
            .child(form_flyout("probe-page", rows, px(220.0), &tokens))
    }
}

fn setup(cx: &mut TestAppContext, action: MenuItem) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(move |window, cx| Probe {
        field: TextField::new("Label name", |_, _, _, _| {}, window, cx),
        pressed: 0,
        action,
    });
    cx.run_until_parked();
    (view, cx)
}

fn press_the_action(view: &Entity<Probe>, cx: &mut VisualTestContext) -> usize {
    let row = cx.debug_bounds("menu-row-0").unwrap();
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |probe, _| probe.pressed)
}

#[gpui::test]
fn the_field_is_a_row_box_on_the_rows_rail(cx: &mut TestAppContext) {
    let (_view, cx) = setup(cx, MenuItem::new("Create"));
    let field = cx.debug_bounds("menu-text-field").unwrap();
    let row = cx.debug_bounds("menu-row-0").unwrap();
    // A row's height and inset, so its words start where a row's do.
    assert_eq!(field.size.height, px(28.0));
    assert_eq!(field.left(), row.left());
    assert_eq!(field.size.width, row.size.width);
    let body = cx.debug_bounds("probe-body").unwrap();
    assert!(body.top() >= field.bottom() && row.top() > body.bottom());
    let hint = cx.debug_bounds("menu-page-hint").unwrap();
    assert!(hint.top() >= row.bottom());
}

#[gpui::test]
fn a_press_on_the_action_acts(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, MenuItem::new("Create"));
    assert_eq!(press_the_action(&view, cx), 1);
}

#[gpui::test]
fn a_disabled_or_pending_action_does_not_act(cx: &mut TestAppContext) {
    let mut disabled = MenuItem::new("Create");
    disabled.disabled = true;
    let (view, cx) = setup(cx, disabled);
    assert_eq!(press_the_action(&view, cx), 0);
    view.update(cx, |probe, cx| {
        probe.action = MenuItem::new("Create");
        probe.action.status = ItemStatus::Pending("Creating…".into());
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(press_the_action(&view, cx), 0);
}
