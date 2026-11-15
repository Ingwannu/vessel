//! Closed regular/circle configuration families share one body and input guard.
//! Circle has no ignored regular kind/size/layout fields: it always uses44/20,
//! source fixed tones, and no hover/pressed override. It is not a second renderer.
use super::content::{circle_icon, icon_element};
use super::control::Activity;
use super::{
    ButtonConfig, ButtonGeometry, ButtonIcon, ButtonKind, ButtonPaint, ButtonPhase, ButtonSize,
    ButtonState, button_paint,
};
use crate::{
    fonts::Type,
    theme::{Tokens, palette},
};
use gpui::{Hsla, Pixels, px};

// Overlay size is explicit in the source, independent of control density.
const CIRCLE_SIZE: Pixels = px(44.0);
pub(super) const CIRCLE_ICON: Pixels = px(crate::theme::icon::LG);
/// Fixed source tones for the voice overlay's circular actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleTone {
    /// Surface0, neutral border and foreground.
    Surface,
    /// Fixed red600/red800 and white, independent of semantic destructive.
    Danger,
}
/// Source44-point circle. No density/size/kind is accepted and then ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircleConfig {
    tone: CircleTone,
    state: ButtonState,
    activity: Activity,
}
impl CircleConfig {
    /// One of the two source appearances and current input availability.
    #[must_use]
    pub const fn new(tone: CircleTone, state: ButtonState) -> Self {
        Self {
            tone,
            state,
            activity: Activity::Idle,
        }
    }
    /// Replace the icon with Small20 loading and refuse the action.
    #[must_use]
    pub const fn loading(mut self) -> Self {
        self.activity = Activity::Loading;
        self
    }
    /// Loading and explicitly disabled both refuse a supplied click handler.
    #[must_use]
    pub const fn availability(self) -> ButtonState {
        match (self.state, self.activity) {
            (ButtonState::Enabled, Activity::Idle) => ButtonState::Enabled,
            (ButtonState::Disabled, _) | (_, Activity::Loading) => ButtonState::Disabled,
        }
    }
}
/// Two complete configuration families accepted by the one button entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonDesign {
    /// Existing sized filled/outline/ghost button.
    Regular(ButtonConfig),
    /// Source44-point circle with fixed appearance and no pressed fade.
    Circle(CircleConfig),
}
impl From<ButtonConfig> for ButtonDesign {
    fn from(value: ButtonConfig) -> Self {
        Self::Regular(value)
    }
}
impl From<CircleConfig> for ButtonDesign {
    fn from(value: CircleConfig) -> Self {
        Self::Circle(value)
    }
}
impl ButtonDesign {
    pub(super) const fn availability(self) -> ButtonState {
        match self {
            Self::Regular(config) => config.availability(),
            Self::Circle(config) => config.availability(),
        }
    }
    pub(super) const fn activity(self) -> Activity {
        match self {
            Self::Regular(config) => config.activity,
            Self::Circle(config) => config.activity,
        }
    }
    pub(super) fn geometry(self) -> ButtonGeometry {
        match self {
            Self::Regular(config) => config.size.geometry(config.sizing),
            Self::Circle(_) => ButtonGeometry {
                height: CIRCLE_SIZE,
                horizontal: px(0.0),
            },
        }
    }
    pub(super) const fn is_circle(self) -> bool {
        matches!(self, Self::Circle(_))
    }
    pub(super) const fn type_step(self) -> Type {
        match self {
            Self::Regular(config) => match config.size {
                ButtonSize::Small => Type::Meta,
                ButtonSize::Medium => Type::Label,
            },
            Self::Circle(_) => Type::Label,
        }
    }
    pub(super) fn paint(self, phase: ButtonPhase, tokens: &Tokens) -> ButtonPaint {
        match self {
            Self::Regular(config) => button_paint(config.kind, phase, tokens),
            Self::Circle(config) => {
                let (fill, border, text) = match config.tone {
                    CircleTone::Surface => (
                        tokens.neutral.surface0,
                        tokens.neutral.border,
                        tokens.neutral.foreground,
                    ),
                    CircleTone::Danger => (
                        gpui::rgb(palette::RED_600).into(),
                        gpui::rgb(palette::RED_800).into(),
                        gpui::white(),
                    ),
                };
                ButtonPaint {
                    fill,
                    border,
                    text,
                    opacity: match phase {
                        ButtonPhase::Disabled => 0.5,
                        ButtonPhase::Idle | ButtonPhase::Hovered | ButtonPhase::Pressed => 1.0,
                    },
                }
            }
        }
    }
    pub(super) fn hover_ink(self, tokens: &Tokens) -> Option<Hsla> {
        match self {
            Self::Regular(config) if config.kind == ButtonKind::Ghost => {
                Some(self.paint(ButtonPhase::Hovered, tokens).text)
            }
            Self::Regular(_) | Self::Circle(_) => None,
        }
    }
    pub(super) fn pressed_opacity(self, tokens: &Tokens) -> Option<f32> {
        match self {
            Self::Regular(_) => Some(self.paint(ButtonPhase::Pressed, tokens).opacity),
            Self::Circle(_) => None,
        }
    }
    pub(super) fn icon(self, icon: ButtonIcon, tokens: &Tokens) -> gpui::Svg {
        match self {
            Self::Regular(config) => {
                icon_element(icon, config.kind, self.availability(), config.size, tokens)
            }
            Self::Circle(_) => {
                circle_icon(icon, self.paint(ButtonPhase::Idle, tokens).text, tokens)
            }
        }
    }
}
