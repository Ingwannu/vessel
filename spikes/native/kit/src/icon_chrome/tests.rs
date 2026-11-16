//! Footer square hitbox and the existing Toolbar's shared source frame.
use super::*;
use crate::Scheme;
use gpui::{Context, Modifiers, Render, TestAppContext, Window, point};

#[test]
fn footer_profile_keeps_square_hit_area_separate_from_round_pressed_layer() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        for (sizing, edge, icon) in [
            (ControlSizing::Pointer, 28.0, 16.0),
            (ControlSizing::Touch, 44.0, 20.0),
        ] {
            assert_eq!(footer_geometry(sizing), (px(edge), px(icon)));
            let mut outer = render(
                "footer",
                "Usage",
                Profile::SidebarFooter {
                    glyph: Glyph::Plus,
                    sizing,
                },
                &tokens,
            );
            assert_eq!(outer.style().size.width, Some(px(edge).into()));
            assert_eq!(outer.style().size.height, Some(px(edge).into()));
            assert_eq!(outer.style().padding.left, Some(px(4.0).into()));
            assert!(outer.style().corner_radii.top_left.is_none());
            assert!(outer.style().background.is_none());
            let mut layer = footer_highlight("layer".into(), &tokens);
            assert!(layer.style().corner_radii.top_left.is_some());
            let mut vector = footer_icon(Glyph::Plus, sizing, &tokens);
            assert_eq!(vector.style().size.width, Some(px(icon).into()));
            assert_eq!(
                vector.style().text.color,
                Some(tokens.neutral.foreground_muted)
            );
            assert_eq!(
                footer_ink(Phase::Hovered, &tokens),
                tokens.neutral.foreground
            );
            assert_eq!(
                footer_ink(Phase::Pressed, &tokens),
                tokens.neutral.foreground_muted
            );
        }
    }
}

struct Probe {
    sizing: ControlSizing,
    clicks: usize,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        div().size_full().child(
            render(
                "footer",
                "Usage",
                Profile::SidebarFooter {
                    glyph: Glyph::Plus,
                    sizing: self.sizing,
                },
                &tokens,
            )
            .debug_selector(|| "footer-chrome".to_owned())
            .on_click(cx.listener(|this, _, _, _| this.clicks = this.clicks.saturating_add(1))),
        )
    }
}
#[gpui::test]
fn actual_footer_corner_click_reaches_same_square_owner_hitbox(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    for (sizing, edge, far_corner) in [
        (ControlSizing::Pointer, 28.0, 27.0),
        (ControlSizing::Touch, 44.0, 43.0),
    ] {
        let (view, cx) = cx.add_window_view(move |_, _| Probe { sizing, clicks: 0 });
        cx.run_until_parked();
        let bounds = cx.debug_bounds("footer-chrome").unwrap();
        assert_eq!(bounds.size.width, px(edge));
        assert_eq!(bounds.size.height, px(edge));
        cx.simulate_click(point(px(1.0), px(1.0)), Modifiers::none());
        cx.simulate_click(point(px(far_corner), px(far_corner)), Modifiers::none());
        cx.run_until_parked();
        view.read_with(cx, |probe, _| {
            assert_eq!(probe.clicks, 2);
        });
    }
}
