//! The foreign trusted-pointer vocabulary. Coordinates are checked again by the target owner.
use super::{Acknowledged, Method, Request, sealed};
use serde::Serialize;
use vessel_proto::browser::{
    CssDelta, CssPoint, KeyModifiers, ModifierState, MouseButton, PointerAction,
};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum Kind {
    #[serde(rename = "mouseMoved")]
    Move,
    #[serde(rename = "mousePressed")]
    Press,
    #[serde(rename = "mouseReleased")]
    Release,
    #[serde(rename = "mouseWheel")]
    Wheel,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum Button {
    None,
    Left,
    Right,
    Middle,
}
impl From<MouseButton> for Button {
    fn from(value: MouseButton) -> Self {
        match value {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
        }
    }
}
/// CDP masks have separate units even though both occupy one byte.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
struct Modifiers(u8);
impl From<KeyModifiers> for Modifiers {
    fn from(value: KeyModifiers) -> Self {
        const ALT: u8 = 1;
        const CONTROL: u8 = 2;
        const META: u8 = 4;
        const SHIFT: u8 = 8;
        let mut bits = 0;
        for (state, bit) in [
            (value.alt, ALT),
            (value.control, CONTROL),
            (value.meta, META),
            (value.shift, SHIFT),
        ] {
            if state == ModifierState::Held {
                bits |= bit;
            }
        }
        Self(bits)
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
struct Buttons(u8);
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(transparent)]
struct Clicks(u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Held {
    #[default]
    Released,
    Pressed,
}

/// This is target-owned state, never the native/global mouse's state.
#[derive(Debug, Default)]
pub(crate) struct PointerState {
    left: Held,
    right: Held,
    middle: Held,
}
impl PointerState {
    fn set(&mut self, button: MouseButton, value: Held) {
        match button {
            MouseButton::Left => self.left = value,
            MouseButton::Right => self.right = value,
            MouseButton::Middle => self.middle = value,
        }
    }
    fn buttons(&self) -> Buttons {
        const LEFT: u8 = 1;
        const RIGHT: u8 = 2;
        const MIDDLE: u8 = 4;
        let mut bits = 0;
        for (held, bit) in [
            (self.left, LEFT),
            (self.right, RIGHT),
            (self.middle, MIDDLE),
        ] {
            if held == Held::Pressed {
                bits |= bit;
            }
        }
        Buttons(bits)
    }
    /// Release only this target's held buttons; no input is sent for unheld buttons.
    pub(crate) fn release_all(&mut self, point: CssPoint) -> [Option<MouseEvent>; 3] {
        let held = [
            (MouseButton::Left, self.left),
            (MouseButton::Right, self.right),
            (MouseButton::Middle, self.middle),
        ];
        held.map(|(button, state)| {
            (state == Held::Pressed).then(|| {
                MouseEvent::pointer(
                    point,
                    PointerAction::Release(button),
                    KeyModifiers::default(),
                    self,
                )
            })
        })
    }
}

/// Only this fixed shape can invoke `Input.dispatchMouseEvent`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MouseEvent {
    #[serde(rename = "type")]
    kind: Kind,
    x: vessel_proto::browser::CssCoordinate,
    y: vessel_proto::browser::CssCoordinate,
    button: Button,
    buttons: Buttons,
    modifiers: Modifiers,
    click_count: Clicks,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta_x: Option<CssDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta_y: Option<CssDelta>,
}
impl MouseEvent {
    /// The actor must quarantine the target if delivery fails after updating held state.
    pub(crate) fn pointer(
        point: CssPoint,
        action: PointerAction,
        modifiers: KeyModifiers,
        state: &mut PointerState,
    ) -> Self {
        let (kind, button, clicks) = match action {
            PointerAction::Move => (Kind::Move, Button::None, Clicks(0)),
            PointerAction::Press(button) => {
                state.set(button, Held::Pressed);
                (Kind::Press, Button::from(button), Clicks(1))
            }
            PointerAction::Release(button) => {
                state.set(button, Held::Released);
                (Kind::Release, Button::from(button), Clicks(1))
            }
        };
        Self {
            kind,
            x: point.x,
            y: point.y,
            button,
            buttons: state.buttons(),
            modifiers: Modifiers::from(modifiers),
            click_count: clicks,
            delta_x: None,
            delta_y: None,
        }
    }
    pub(crate) fn wheel(
        point: CssPoint,
        delta_x: CssDelta,
        delta_y: CssDelta,
        modifiers: KeyModifiers,
        state: &PointerState,
    ) -> Self {
        Self {
            kind: Kind::Wheel,
            x: point.x,
            y: point.y,
            button: Button::None,
            buttons: state.buttons(),
            modifiers: Modifiers::from(modifiers),
            click_count: Clicks(0),
            delta_x: Some(delta_x),
            delta_y: Some(delta_y),
        }
    }
}
request!(MouseEvent, Acknowledged, MouseEvent);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn button_and_modifier_masks_follow_the_actual_target_sequence() {
        let point: CssPoint = serde_json::from_value(json!({"x":10.5,"y":20})).unwrap();
        let mut state = PointerState::default();
        let mut modifiers = KeyModifiers::default();
        for modifier in [
            vessel_proto::browser::KeyModifier::Alt,
            vessel_proto::browser::KeyModifier::Control,
            vessel_proto::browser::KeyModifier::Meta,
            vessel_proto::browser::KeyModifier::Shift,
        ] {
            modifiers.include(modifier);
        }
        let pressed = MouseEvent::pointer(
            point,
            PointerAction::Press(MouseButton::Left),
            modifiers,
            &mut state,
        );
        assert_eq!(
            serde_json::to_value(pressed).unwrap(),
            json!({"type":"mousePressed","x":10.5,"y":20.0,"button":"left","buttons":1,"modifiers":15,"clickCount":1})
        );
        let moved = MouseEvent::pointer(point, PointerAction::Move, modifiers, &mut state);
        assert_eq!(serde_json::to_value(moved).unwrap()["buttons"], json!(1));
        let _right = MouseEvent::pointer(
            point,
            PointerAction::Press(MouseButton::Right),
            modifiers,
            &mut state,
        );
        let _middle = MouseEvent::pointer(
            point,
            PointerAction::Press(MouseButton::Middle),
            modifiers,
            &mut state,
        );
        let delta = CssDelta(vessel_proto::conversation::FiniteNumber::new(-0.5).unwrap());
        let wheel = MouseEvent::wheel(point, delta, delta, modifiers, &state);
        assert_eq!(serde_json::to_value(wheel).unwrap()["buttons"], json!(7));
        assert_eq!(state.release_all(point).into_iter().flatten().count(), 3);
        assert_eq!(state.release_all(point).into_iter().flatten().count(), 0);
        assert_eq!(state.buttons().0, 0);
    }
}
