//! Reference profile paint and layout: passive slots use visual, never hit extent.
use super::*;
use crate::TypeExt as _;
use gpui::{Bounds, Context, Render, TestAppContext, Window};
use std::{cell::RefCell, rc::Rc};

#[test]
fn provider_geometry_paint_and_svg_mapping_match_the_source() {
    for (scheme, fg, muted, surface2, surface3) in [
        (
            crate::Scheme::Light,
            0x001a_1a1e,
            0x0071_717a,
            0x00f4_f4f5,
            0x00e4_e4e7,
        ),
        (
            crate::Scheme::Dark,
            0x00fa_fafa,
            0x00a1_a5a4,
            0x0027_2a29,
            0x0043_4645,
        ),
    ] {
        let tokens = Tokens::of(scheme);
        assert_eq!(tokens.neutral.foreground, gpui::rgb(fg).into());
        assert_eq!(tokens.neutral.foreground_muted, gpui::rgb(muted).into());
        assert_eq!(tokens.neutral.surface2, gpui::rgb(surface2).into());
        assert_eq!(tokens.neutral.surface3, gpui::rgb(surface3).into());
        for sizing in [TriggerSizing::Pointer, TriggerSizing::Touch] {
            let g = frame(TriggerProfile::ProviderRow, sizing);
            assert_eq!(g.visual, px(32.0));
            assert_eq!(g.hit, px(48.0));
            assert_eq!(g.glyph, px(14.0));
            assert_eq!(g.radius, px(8.0));
            assert_eq!(g.asset, Glyph::More);
            assert_eq!(
                pressed_fill(TriggerProfile::ProviderRow, &tokens),
                Some(tokens.neutral.surface3)
            );
            assert_eq!(pressed_fill(TriggerProfile::Schedule, &tokens), None);
            assert_eq!(pressed_fill(TriggerProfile::GitHeader, &tokens), None);
            for availability in [crate::ButtonState::Enabled, crate::ButtonState::Disabled] {
                for (presentation, active) in [
                    (TriggerPresentation::Closed(availability), false),
                    (TriggerPresentation::Open(availability), true),
                ] {
                    let idle =
                        profile_paint(TriggerProfile::ProviderRow, presentation, false, &tokens);
                    assert_eq!(
                        idle.ink,
                        if active {
                            tokens.neutral.foreground
                        } else {
                            tokens.neutral.foreground_muted
                        }
                    );
                    assert_eq!(
                        idle.fill,
                        if active {
                            tokens.neutral.surface2
                        } else {
                            gpui::transparent_black()
                        }
                    );
                    let hover =
                        profile_paint(TriggerProfile::ProviderRow, presentation, true, &tokens);
                    assert_eq!(hover.ink, tokens.neutral.foreground);
                    assert_eq!(hover.fill, tokens.neutral.surface2);
                    let mut outer = trigger_presented(
                        "provider",
                        &TriggerAnchor::default(),
                        TriggerProfile::ProviderRow,
                        sizing,
                        presentation,
                        &tokens,
                    );
                    assert_eq!(outer.style().opacity, None);
                }
            }
            assert_eq!(
                trigger_visual_size(TriggerProfile::GitHeader, sizing),
                match sizing {
                    TriggerSizing::Pointer => px(26.0),
                    TriggerSizing::Touch => px(32.0),
                }
            );
            assert_eq!(
                trigger_visual_size(TriggerProfile::Schedule, sizing),
                match sizing {
                    TriggerSizing::Pointer => px(22.0),
                    TriggerSizing::Touch => px(44.0),
                }
            );
            let mut passive = trigger_space(TriggerProfile::ProviderRow, sizing);
            assert_eq!(passive.style().size.width, Some(px(32.0).into()));
            assert_eq!(passive.style().size.height, Some(px(32.0).into()));
            assert_eq!(passive.style().flex_grow, Some(0.0));
            assert_eq!(passive.style().flex_shrink, Some(0.0));
        }
    }
}

#[derive(Default)]
struct Seen {
    rows: Vec<Bounds<Pixels>>,
    counts: [Option<Bounds<Pixels>>; 2],
}
struct Probe {
    profile: TriggerProfile,
    sizing: TriggerSizing,
    scheme: crate::Scheme,
    seen: Rc<RefCell<Seen>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let mut rows = div().flex().flex_col().w(px(400.0));
        for (index, present) in [true, false].into_iter().enumerate() {
            let slot = if present {
                trigger_with(
                    "some",
                    &TriggerAnchor::default(),
                    self.profile,
                    self.sizing,
                    crate::ButtonState::Enabled,
                    &tokens,
                )
                .into_any_element()
            } else {
                trigger_space(self.profile, self.sizing).into_any_element()
            };
            let seen = Rc::clone(&self.seen);
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .w_full()
                    .child(div().flex_1().type_(crate::Type::Label).child("file.rs"))
                    .child(div().type_(crate::Type::Label).child("12"))
                    .child(slot)
                    .on_children_prepainted(move |bounds, _, _| {
                        seen.borrow_mut().counts[index] = Some(bounds[1]);
                    }),
            );
        }
        let seen = Rc::clone(&self.seen);
        rows.on_children_prepainted(move |bounds, _, _| seen.borrow_mut().rows = bounds)
    }
}
#[gpui::test]
fn passive_slots_keep_counts_x_and_row_height_for_some_and_none(cx: &mut TestAppContext) {
    for scheme in [crate::Scheme::Light, crate::Scheme::Dark] {
        for profile in [
            TriggerProfile::Schedule,
            TriggerProfile::GitHeader,
            TriggerProfile::ProviderRow,
        ] {
            for sizing in [TriggerSizing::Pointer, TriggerSizing::Touch] {
                let seen = Rc::new(RefCell::new(Seen::default()));
                let (_, cx) = cx.add_window_view({
                    let seen = Rc::clone(&seen);
                    move |_, _| Probe {
                        profile,
                        sizing,
                        scheme,
                        seen,
                    }
                });
                cx.run_until_parked();
                let seen = seen.borrow();
                let [some, none] = seen.rows.as_slice() else {
                    panic!("two rows: {:?}", seen.rows)
                };
                assert_eq!(some.size.height, none.size.height);
                let [Some(some), Some(none)] = seen.counts else {
                    panic!("two counts: {:?}", seen.counts)
                };
                assert_eq!(some.origin.x, none.origin.x);
            }
        }
    }
}
