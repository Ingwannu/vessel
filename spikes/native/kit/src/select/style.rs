//! Geometry/paint of the ONE select frame; existing public presentations retain
//! their values. Static form values never enter the interaction renderer.
use super::{FORM_VERTICAL_POINTER, FORM_VERTICAL_TOUCH, Presentation, Tokens, Type, Visibility};
#[derive(Debug, Clone, Copy)]
pub(super) enum FramePresentation {
    Select(Presentation),
    ReadOnlyForm,
}
impl From<Presentation> for FramePresentation {
    fn from(value: Presentation) -> Self {
        Self::Select(value)
    }
}
pub(super) fn opacity(
    presentation: Presentation,
    availability: crate::ButtonState,
    visibility: Visibility,
) -> Option<f32> {
    // Legacy disabled wins even while open; the HostFilter source has no
    // disabled/open opacity. A caller never attaches a disabled handler.
    match (presentation, availability, visibility) {
        (
            Presentation::HostFilter,
            crate::ButtonState::Enabled | crate::ButtonState::Disabled,
            Visibility::Closed | Visibility::Open,
        )
        | (Presentation::Settings, crate::ButtonState::Enabled, Visibility::Closed) => None,
        (
            Presentation::Settings,
            crate::ButtonState::Disabled,
            Visibility::Closed | Visibility::Open,
        ) => Some(DISABLED_OPACITY),
        (
            Presentation::Form,
            crate::ButtonState::Enabled,
            Visibility::Closed | Visibility::Open,
        ) => None,
        (
            Presentation::Form,
            crate::ButtonState::Disabled,
            Visibility::Closed | Visibility::Open,
        ) => Some(DISABLED_OPACITY),
        (Presentation::Settings, crate::ButtonState::Enabled, Visibility::Open) => {
            Some(PRESSED_OPACITY)
        }
    }
}
const POINTER_HEIGHT: f32 = 32.0;
pub(super) const PRESSED_OPACITY: f32 = 0.85;
const DISABLED_OPACITY: f32 = 0.5;
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Radius {
    Full,
    Points(f32),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Geometry {
    pub(super) minimum: Option<f32>,
    pub(super) horizontal: f32,
    pub(super) vertical: Option<f32>,
    pub(super) gap: f32,
    pub(super) radius: Radius,
    pub(super) type_: Type,
}
pub(super) fn geometry(
    presentation: impl Into<FramePresentation>,
    sizing: crate::ControlSizing,
) -> Geometry {
    use crate::{
        ControlSizing,
        theme::{radius, spacing},
    };
    match presentation.into() {
        FramePresentation::Select(Presentation::Settings) => Geometry {
            minimum: Some(match sizing {
                ControlSizing::Pointer => POINTER_HEIGHT,
                ControlSizing::Touch => crate::breakpoint::TOUCH_TARGET,
            }),
            horizontal: match sizing {
                ControlSizing::Pointer => spacing::S4,
                ControlSizing::Touch => spacing::S6,
            },
            vertical: None,
            gap: spacing::S2,
            radius: Radius::Full,
            type_: Type::Body,
        },
        FramePresentation::Select(Presentation::Form) | FramePresentation::ReadOnlyForm => {
            Geometry {
                minimum: Some(match sizing {
                    ControlSizing::Pointer => POINTER_HEIGHT,
                    ControlSizing::Touch => crate::breakpoint::TOUCH_TARGET,
                }),
                horizontal: spacing::S3,
                vertical: Some(match sizing {
                    ControlSizing::Pointer => FORM_VERTICAL_POINTER,
                    ControlSizing::Touch => FORM_VERTICAL_TOUCH,
                }),
                gap: spacing::S2,
                radius: Radius::Points(radius::R2),
                type_: Type::Body,
            }
        }
        FramePresentation::Select(Presentation::HostFilter) => Geometry {
            minimum: None,
            horizontal: spacing::S3,
            vertical: Some(spacing::S1_5),
            gap: spacing::S1_5,
            radius: Radius::Points(radius::MD),
            type_: Type::Label,
        },
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Paint {
    pub(super) background: gpui::Hsla,
    pub(super) border: gpui::Hsla,
    pub(super) hover: Option<gpui::Hsla>,
    pub(super) pressed: Option<gpui::Hsla>,
    pub(super) foreground: gpui::Hsla,
}
pub(super) fn paint(presentation: impl Into<FramePresentation>, tokens: &Tokens) -> Paint {
    match presentation.into() {
        FramePresentation::Select(Presentation::Settings) => Paint {
            background: gpui::transparent_black(),
            border: tokens.neutral.border_accent,
            hover: None,
            pressed: None,
            foreground: tokens.neutral.foreground,
        },
        FramePresentation::Select(Presentation::Form) => Paint {
            background: tokens.neutral.surface2,
            border: gpui::transparent_black(),
            hover: None,
            pressed: None,
            foreground: tokens.neutral.foreground,
        },
        FramePresentation::Select(Presentation::HostFilter) => Paint {
            background: tokens.neutral.surface1,
            border: tokens.neutral.border,
            hover: Some(tokens.neutral.surface2),
            pressed: Some(tokens.neutral.surface3),
            foreground: tokens.neutral.foreground,
        },
        FramePresentation::ReadOnlyForm => Paint {
            background: tokens.neutral.surface2,
            border: tokens.neutral.border,
            hover: None,
            pressed: None,
            foreground: tokens.neutral.foreground,
        },
    }
}
