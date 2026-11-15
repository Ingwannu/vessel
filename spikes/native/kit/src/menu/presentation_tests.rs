//! Exact idle presentation and glyph geometry; feedback/input remain independent.
use super::*;
use super::{
    draw,
    item::{self, Leading, Paint},
};
use crate::{Scheme, Tokens, assets::Glyph, popup::Surfaces};
use gpui::{Styled as _, point, px};

#[test]
fn leading_size_is_explicit_without_changing_feedback_or_input() {
    let legacy = MenuItem::new("Pause").with_leading(Glyph::Pause);
    assert_eq!(legacy.leading_size, LeadingSize::Small);
    assert_eq!(legacy.leading(), Leading::Glyph(Glyph::Pause));
    for size in [LeadingSize::Small, LeadingSize::Regular] {
        let item = MenuItem::new("Push")
            .with_leading_at(Glyph::Upload, size)
            .with_dismissal(Dismissal::KeepOpen);
        assert_eq!(item.leading_size, size);
        let mut vector = draw::ordinary_glyph(&item, Glyph::Upload, gpui::rgb(0x00ff_ffff).into());
        assert_eq!(vector.style().size.width, Some(px(size.points()).into()));
        assert_eq!(vector.style().size.height, Some(px(size.points()).into()));
        assert_eq!(item.leading(), Leading::Glyph(Glyph::Upload));
        assert!(item.selectable());
        assert_eq!(item.dismissal, Dismissal::KeepOpen);
        for feedback in [item.clone().pending("Pushing"), item.success("Pushed")] {
            assert_eq!(feedback.leading_size, size);
            assert!(!feedback.selectable());
        }
    }
    assert_eq!(draw::LEADING_SLOT, px(16.0));
    assert_eq!(draw::SUCCESS_GLYPH, px(16.0));
    assert_eq!(crate::loading::Points::SIXTEEN.pixels(), px(16.0));
}

#[test]
fn muted_is_selectable_and_feedback_overrides_its_colour_and_opacity() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let surfaces = Surfaces::of(&Tokens::of(scheme));
        let muted = MenuItem::new("Push")
            .with_presentation(Presentation::Muted)
            .with_dismissal(Dismissal::KeepOpen);
        assert!(muted.selectable());
        assert_eq!(
            selected_index(std::slice::from_ref(&muted), Some(0)),
            Some(0)
        );
        for highlighted in [false, true] {
            assert_eq!(
                item::paint(&muted, highlighted, &surfaces),
                Paint {
                    ink: surfaces.foreground_muted,
                    glyph: surfaces.foreground_muted,
                    fill: if highlighted {
                        surfaces.surface2
                    } else {
                        gpui::transparent_black()
                    },
                    opacity: 0.72,
                }
            );
        }
        let mut disabled = muted.clone();
        disabled.disabled = true;
        for (item, ink) in [
            (disabled, surfaces.foreground),
            (muted.clone().pending("Pushing"), surfaces.foreground),
            (muted.success("Pushed"), gpui::rgb(0x0022_c55e).into()),
        ] {
            assert!(!item.selectable());
            let paint = item::paint(&item, true, &surfaces);
            assert_eq!(paint.ink, ink);
            assert_eq!(paint.opacity.to_bits(), 0.5_f32.to_bits());
            assert_eq!(paint.fill, gpui::transparent_black());
        }
    }
}
#[gpui::test]
fn muted_activation_uses_both_input_paths_and_preserves_dismissal(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        for dismissal in [Dismissal::Close, Dismissal::KeepOpen] {
            let item = MenuItem::new("Push")
                .with_presentation(Presentation::Muted)
                .with_dismissal(dismissal);
            for keyboard in [false, true] {
                let mut state = MenuState::new(cx);
                state.at = Some(Placement::Pointer(point(px(10.0), px(20.0))));
                state.highlighted = Some(0);
                let selected = if keyboard {
                    keyboard_choice(&mut state, std::slice::from_ref(&item))
                } else {
                    pointer_choice(&mut state, &item, 0)
                };
                assert_eq!(selected, Some(0));
                assert_eq!(state.is_open(), dismissal == Dismissal::KeepOpen);
            }
        }
    });
}
