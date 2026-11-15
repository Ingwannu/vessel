//! New form profile values; old Settings/HostFilter test bodies stay intact.
use super::*;
use crate::{ButtonState, ControlSizing, Scheme};
#[test]
fn form_geometry_is_source_md_with_distinct_leading_chevron_and_loading_slots() {
    for (sizing, height, vertical) in [
        (ControlSizing::Pointer, 32.0, 5.0),
        (ControlSizing::Touch, 44.0, 11.0),
    ] {
        assert_eq!(
            geometry(Presentation::Form, sizing),
            Geometry {
                minimum: Some(height),
                horizontal: 12.0,
                vertical: Some(vertical),
                gap: 8.0,
                radius: Radius::Points(8.0),
                type_: Type::Body,
                line_height: Some(20.0),
            }
        );
    }
    assert_eq!(FORM_LEADING.to_bits(), 18.0_f32.to_bits());
    assert_eq!(crate::theme::icon::MD.to_bits(), 16.0_f32.to_bits());
    assert_eq!(crate::loading::Points::FOURTEEN.pixels(), px(14.0));
}
#[test]
fn actual_form_frames_keep_source_rest_open_disabled_and_placeholder_values() {
    for (scheme, surface, accent, foreground, muted) in [
        (
            Scheme::Light,
            0x00f4_f4f5,
            0x00ec_ecf1,
            0x001a_1a1e,
            0x0071_717a,
        ),
        (
            Scheme::Dark,
            0x0027_2a29,
            0x002f_3534,
            0x00fa_fafa,
            0x00a1_a5a4,
        ),
    ] {
        let tokens = Tokens::of(scheme);
        assert_eq!(
            paint(Presentation::Form, &tokens).background,
            gpui::rgb(surface).into()
        );
        for (sizing, height, vertical) in [
            (ControlSizing::Pointer, 32.0, 5.0),
            (ControlSizing::Touch, 44.0, 11.0),
        ] {
            for availability in [ButtonState::Enabled, ButtonState::Disabled] {
                for visibility in [Visibility::Closed, Visibility::Open] {
                    let mut frame = form_trigger(
                        "form",
                        FormContent {
                            value: FormValue::Placeholder("Choose one".into()),
                            leading: None,
                            sizing,
                            availability,
                            visibility,
                            activity: FormActivity::Loading,
                        },
                        &Anchor::default(),
                        &tokens,
                    );
                    let style = frame.style();
                    assert_eq!(style.min_size.height, Some(px(height).into()));
                    assert_eq!(style.padding.left, Some(px(12.0).into()));
                    assert_eq!(style.padding.top, Some(px(vertical).into()));
                    assert_eq!(
                        style.opacity,
                        if availability == ButtonState::Disabled {
                            Some(0.5)
                        } else {
                            None
                        }
                    );
                    assert_eq!(
                        style.border_color,
                        Some(
                            if availability == ButtonState::Enabled
                                && visibility == Visibility::Open
                            {
                                gpui::rgb(accent).into()
                            } else {
                                gpui::transparent_black()
                            }
                        )
                    );
                }
            }
        }
        for (placeholder, expected) in [(false, foreground), (true, muted)] {
            assert_eq!(
                form_foreground(
                    Some(form::Look {
                        placeholder,
                        activity: FormActivity::Idle
                    }),
                    &tokens
                ),
                gpui::rgb(expected).into()
            );
        }
    }
}
