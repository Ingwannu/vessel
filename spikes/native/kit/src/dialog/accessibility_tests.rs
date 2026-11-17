//! The dialog's wordless header controls are buttons with names in the app's
//! words (TRACKER 782): close and back, and an owner's own back label.
use super::*;
use crate::a11y_snapshot::{activate, button_names};
use crate::{ControlNames, Scheme, SchemeChoice, set_control_names};
use gpui::{Render, TestAppContext, VisualTestContext};

struct Probe {
    state: DialogState,
    back_label: Option<&'static str>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let back = HeaderBack::new(|_event, _window, _cx| {});
        let back = match self.back_label {
            Some(label) => back.with_label(label),
            None => back,
        };
        let content = DialogContent {
            title: "Edit label".into(),
            body: div().h(px(120.0)).into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        }
        .with_header_back(back);
        let modal = dialog(
            &self.state,
            content,
            &tokens,
            window,
            false,
            |_probe: &mut Self, _, _| {},
            cx,
        );
        div().size_full().child(modal)
    }
}

fn names(cx: &mut VisualTestContext) -> Vec<Option<String>> {
    cx.update(|window, _| button_names(window).unwrap())
}

fn redraw(view: &gpui::Entity<Probe>, cx: &mut VisualTestContext) {
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
}

#[gpui::test]
fn close_and_back_are_buttons_named_in_the_apps_words(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) = cx.add_window_view(|_window, cx| Probe {
        state: DialogState::new(cx),
        back_label: None,
    });
    cx.run_until_parked();
    activate(cx);
    redraw(&view, cx);
    // Before the app names them: English, as Paseo's `common.actions`.
    let before = names(cx);
    assert!(before.contains(&Some("Close".to_owned())));
    assert!(before.contains(&Some("Back".to_owned())));
    // The app's words, from the next frame on.
    cx.update(|_, cx| {
        set_control_names(
            ControlNames {
                close: "닫기".into(),
                back: "뒤로".into(),
            },
            cx,
        );
    });
    redraw(&view, cx);
    let korean = names(cx);
    assert!(korean.contains(&Some("닫기".to_owned())));
    assert!(korean.contains(&Some("뒤로".to_owned())));
    assert!(!korean.contains(&Some("Close".to_owned())));
    // An owner's own label wins over the kit's "Back".
    view.update(cx, |probe, cx| {
        probe.back_label = Some("Labels");
        cx.notify();
    });
    cx.run_until_parked();
    let owned = names(cx);
    assert!(owned.contains(&Some("Labels".to_owned())));
    assert!(!owned.contains(&Some("뒤로".to_owned())));
}
