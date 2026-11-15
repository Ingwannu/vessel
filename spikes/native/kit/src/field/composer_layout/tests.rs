//! Geometry, same engine retention, and the existing Enter listener's compact policy.

use super::*;
use crate::{Editability, Scheme};
use gpui::{Context, Render, TestAppContext, Window};

#[test]
fn viewport_caps_and_fractional_row_fits_follow_reference_boundaries() {
    for (height, expected) in [
        (0.0, 160.0),
        (200.0, 160.0),
        (900.0, 450.0),
        (901.0, 450.0),
        (f32::NAN, 160.0),
    ] {
        let layout = ComposerLayout::inline(
            ComposerViewport::Desktop(px(height)),
            ComposerFont::Chat,
            SendOn::Enter,
        );
        assert_eq!(layout.height, px(expected));
    }
    let compact = ComposerLayout::inline(
        ComposerViewport::Compact,
        ComposerFont::Terminal,
        SendOn::ModEnter,
    );
    assert_eq!(compact.height, px(160.0));
    assert_eq!(compact.send, SendOn::ButtonOnly);
    assert_eq!(fitted_rows(px(160.0), px(21.0)), 7);
    assert_eq!(fitted_rows(px(450.0), px(19.5)), 23);
    assert_eq!(fitted_rows(px(0.0), px(21.0)), 1);
    assert_eq!(fitted_rows(px(100.0), px(0.0)), 1);
}

struct Probe {
    field: TextField,
    layout: ComposerLayout,
    submitted: usize,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.field
            .composer_element(self.layout, &Tokens::of(Scheme::Dark), cx)
    }
}

#[gpui::test]
fn compact_preference_reapply_cannot_send_and_fullscreen_retains_the_same_editor(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_base::init);
    let (probe, window) = cx.add_window_view(|window, cx| {
        let field = TextField::composer(
            "Message",
            |owner: &mut Probe, _, _, _| owner.submitted = owner.submitted.saturating_add(1),
            window,
            cx,
        );
        field.focus(window, cx);
        Probe {
            field,
            layout: ComposerLayout::inline(
                ComposerViewport::Compact,
                ComposerFont::Chat,
                SendOn::Enter,
            ),
            submitted: 0,
        }
    });
    window.run_until_parked();
    window.update(|window, cx| {
        probe.update(cx, |probe, cx| {
            probe.field.set_text("한글", window, cx);
            let Kind::Lines(state, _) = &probe.field.kind else {
                panic!("composer");
            };
            state.update(cx, |state, cx| {
                state.set_selected_range("한글".len().."한글".len(), cx);
            });
            probe.field.set_send_on(SendOn::Enter);
            assert_eq!(probe.field.send.get(), SendOn::ButtonOnly);
        });
    });
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    let identity = probe.read_with(window, |probe, cx| {
        assert_eq!(probe.submitted, 0);
        assert_eq!(probe.field.text(cx), "한글\n");
        let Kind::Lines(state, _) = &probe.field.kind else {
            panic!("composer");
        };
        state.entity_id()
    });
    window.update(|_, cx| {
        probe.update(cx, |probe, cx| {
            probe.field.set_editability(Editability::ReadOnly, cx);
            probe.layout = ComposerLayout::fullscreen(px(400.0), ComposerFont::Terminal);
            cx.notify();
        });
    });
    window.run_until_parked();
    probe.read_with(window, |probe, cx| {
        let Kind::Lines(state, _) = &probe.field.kind else {
            panic!("composer");
        };
        assert_eq!(state.entity_id(), identity);
        assert!(state.read(cx).presentation().is_readonly());
        assert_eq!(probe.field.text(cx), "한글\n");
        assert_eq!(
            state.read(cx).selected_range(),
            "한글\n".len().."한글\n".len()
        );
    });
}
