//! Real rendered `KeepOpen` activation and refusal after feedback replaces a row.
//! Pure index helpers cannot prove that an old pointer/keyboard handler is gone.
use super::*;
use crate::{Scheme, Tokens, assets::Glyph};
use gpui::prelude::*;
use gpui::{Modifiers, Render, TestAppContext, div, point, px};

struct Probe {
    menu: MenuState,
    items: Vec<MenuItem>,
    chosen: usize,
    scheme: Scheme,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let handlers = MenuHandlers {
            state: |this: &mut Self| &mut this.menu,
            chose: Rc::new(|this, index, _, _| {
                this.chosen = this.chosen.saturating_add(1);
                this.items[index] = this.items[index].clone().pending("Sending / 보내는 중");
            }),
        };
        div().size_full().children(menu_with(
            Content::new("feedback", &self.items).with_width(px(220.0)),
            &self.menu,
            &handlers,
            &Tokens::of(self.scheme),
            false,
            cx,
        ))
    }
}
fn item() -> MenuItem {
    MenuItem::new("Push long branch / 긴 브랜치 보내기")
        .with_leading_at(Glyph::Upload, LeadingSize::Regular)
        .with_dismissal(Dismissal::KeepOpen)
}
fn click(cx: &mut gpui::VisualTestContext) {
    cx.simulate_click(point(px(120.0), px(62.0)), Modifiers::none());
    cx.run_until_parked();
}
fn enter(cx: &mut gpui::VisualTestContext) {
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
}
#[gpui::test]
fn kept_open_feedback_refuses_real_pointer_and_keyboard_then_reenables(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    for scheme in [Scheme::Light, Scheme::Dark] {
        let (view, cx) = cx.add_window_view(move |window, cx| {
            let mut menu = MenuState::new(cx);
            menu.open(point(px(100.0), px(40.0)), window, cx);
            Probe {
                menu,
                items: vec![item()],
                chosen: 0,
                scheme,
            }
        });
        cx.run_until_parked();
        click(cx);
        view.read_with(cx, |this, _| {
            assert!(this.menu.is_open());
            assert_eq!(this.chosen, 1);
            assert!(matches!(this.items[0].status, ItemStatus::Pending(_)));
        });
        click(cx);
        enter(cx);
        view.read_with(cx, |this, _| assert_eq!(this.chosen, 1));
        view.update(cx, |this, cx| {
            this.items[0] = item().success("Pushed / 보냄");
            cx.notify();
        });
        cx.run_until_parked();
        click(cx);
        enter(cx);
        view.read_with(cx, |this, _| assert_eq!(this.chosen, 1));
        view.update(cx, |this, cx| {
            this.items[0] = item();
            this.items[0].disabled = true;
            cx.notify();
        });
        cx.run_until_parked();
        click(cx);
        enter(cx);
        view.read_with(cx, |this, _| assert_eq!(this.chosen, 1));
        view.update(cx, |this, cx| {
            this.items[0] = item();
            cx.notify();
        });
        cx.run_until_parked();
        enter(cx);
        view.read_with(cx, |this, _| {
            assert_eq!(this.chosen, 2);
            assert!(this.menu.is_open());
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        view.read_with(cx, |this, _| assert!(!this.menu.is_open()));
    }
}
