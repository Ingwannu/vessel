//! Paseo's `SheetHeader.subtitle`: the owner's line under the title in the
//! title group (gap 4), beside the header actions and the close button.
use super::*;
use crate::{Scheme, SchemeChoice};
use gpui::{Render, TestAppContext};

struct Probe {
    state: DialogState,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let content = DialogContent {
            title: "Import session".into(),
            body: div().h(px(120.0)).into_any_element(),
            footer: Vec::new(),
            size: DialogSize::Normal,
        }
        .with_header_actions(
            div()
                .debug_selector(|| "probe-action".to_owned())
                .size(px(32.0)),
        )
        .with_header_subtitle(
            div()
                .debug_selector(|| "probe-subtitle".to_owned())
                .h(px(16.0))
                .child("This workspace"),
        );
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

#[gpui::test]
fn the_subtitle_sits_under_the_title_and_before_the_actions(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (_view, cx) = cx.add_window_view(|_window, cx| Probe {
        state: DialogState::new(cx),
    });
    cx.run_until_parked();
    let title = cx.debug_bounds("dialog-title").unwrap();
    let subtitle = cx.debug_bounds("probe-subtitle").unwrap();
    let action = cx.debug_bounds("probe-action").unwrap();
    assert_eq!(subtitle.left(), title.left());
    let gap = f32::from(title.bottom()).mul_add(-1.0, f32::from(subtitle.top()));
    assert_eq!(gap.to_bits(), 4.0_f32.to_bits());
    assert!(subtitle.right() <= action.left());
}
