//! Composer gestures reuse the existing preference and session-delivery rules.

use vessel_proto::Delivery;

use crate::prefs::{EnterPress, sends, sends_other};
use crate::prefs::{SendKey, TurnDelivery};
use crate::session::{SendFacts, send_delivery};

use super::surface::Layout;

/// A gesture can queue only when the host supplies a queue action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueAction {
    /// The owner exposes an explicit queue action.
    Available,
    /// A one-shot prompt owner cannot queue.
    Unavailable,
}

/// Whether an alternate send has a running turn to distinguish it from Send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// A send starts a new turn.
    Idle,
    /// The gesture can queue or reach the current turn.
    Running,
}

/// An input method owns its committing Enter independently of shortcut preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterAction {
    /// Leave the key to the input method.
    Composition,
    /// Insert a line at the current selection.
    Newline,
    /// Apply the ordinary send preference.
    DefaultSend,
    /// Flip effective queue/send while a turn runs.
    AlternateSend,
}

/// Desktop shortcuts are IME-safe; compact layouts submit through the button only.
#[must_use]
pub fn enter(
    press: EnterPress,
    shortcut: SendKey,
    layout: Layout,
    turn: Turn,
    queue: QueueAction,
) -> EnterAction {
    if press.composing {
        return EnterAction::Composition;
    }
    if layout == Layout::Compact || !sends(press, shortcut) {
        return EnterAction::Newline;
    }
    if turn == Turn::Running && queue == QueueAction::Available && sends_other(press, shortcut) {
        EnterAction::AlternateSend
    } else {
        EnterAction::DefaultSend
    }
}

/// The send button/shortcut uses either its ordinary or alternate action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Ordinary button press or configured shortcut.
    Default,
    /// Long press or eligible modified Enter.
    Alternate,
}

/// Alternate flips effective queue/send while a turn runs; a pending permission
/// first turns default Queue into Interrupt, so an explicit alternate can queue.
#[must_use]
pub fn delivery(action: Action, chosen: TurnDelivery, facts: SendFacts) -> Delivery {
    if action == Action::Default || !facts.turn_running {
        return send_delivery(chosen, facts);
    }
    match chosen {
        TurnDelivery::Queue if !facts.permission_waits => Delivery::Interrupt,
        TurnDelivery::Queue | TurnDelivery::Steer | TurnDelivery::Interrupt => Delivery::Queue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press() -> EnterPress {
        EnterPress {
            enter: true,
            ..EnterPress::default()
        }
    }

    #[test]
    fn shift_and_compact_always_insert_lines_and_composition_never_sends() {
        for shortcut in SendKey::ALL {
            for meta in [false, true] {
                for layout in [Layout::Desktop, Layout::Compact] {
                    let shifted = EnterPress {
                        shift: true,
                        meta,
                        ..press()
                    };
                    assert_eq!(
                        enter(
                            shifted,
                            shortcut,
                            layout,
                            Turn::Running,
                            QueueAction::Available
                        ),
                        EnterAction::Newline
                    );
                    let composing = EnterPress {
                        composing: true,
                        meta,
                        ..press()
                    };
                    assert_eq!(
                        enter(
                            composing,
                            shortcut,
                            layout,
                            Turn::Running,
                            QueueAction::Available
                        ),
                        EnterAction::Composition
                    );
                }
                assert_eq!(
                    enter(
                        EnterPress { meta, ..press() },
                        shortcut,
                        Layout::Compact,
                        Turn::Running,
                        QueueAction::Available
                    ),
                    EnterAction::Newline
                );
            }
        }
    }

    #[test]
    fn modified_enter_alternates_only_in_enter_mode_with_a_running_queue_capable_host() {
        let modified = EnterPress {
            meta: true,
            ..press()
        };
        assert_eq!(
            enter(
                modified,
                SendKey::Enter,
                Layout::Desktop,
                Turn::Running,
                QueueAction::Available
            ),
            EnterAction::AlternateSend
        );
        assert_eq!(
            enter(
                modified,
                SendKey::ModEnter,
                Layout::Desktop,
                Turn::Running,
                QueueAction::Available
            ),
            EnterAction::DefaultSend
        );
        assert_eq!(
            enter(
                press(),
                SendKey::ModEnter,
                Layout::Desktop,
                Turn::Running,
                QueueAction::Available
            ),
            EnterAction::Newline
        );
        assert_eq!(
            enter(
                modified,
                SendKey::Enter,
                Layout::Desktop,
                Turn::Idle,
                QueueAction::Available
            ),
            EnterAction::DefaultSend
        );
        assert_eq!(
            enter(
                modified,
                SendKey::Enter,
                Layout::Desktop,
                Turn::Running,
                QueueAction::Unavailable
            ),
            EnterAction::DefaultSend
        );
    }

    #[test]
    fn alternate_respects_permission_override_and_default_steering_fallback() {
        let running = SendFacts {
            turn_running: true,
            steers: true,
            permission_waits: false,
        };
        assert_eq!(
            delivery(Action::Default, TurnDelivery::Steer, running),
            Delivery::Steer
        );
        assert_eq!(
            delivery(Action::Alternate, TurnDelivery::Steer, running),
            Delivery::Queue
        );
        assert_eq!(
            delivery(Action::Default, TurnDelivery::Queue, running),
            Delivery::Queue
        );
        assert_eq!(
            delivery(Action::Alternate, TurnDelivery::Queue, running),
            Delivery::Interrupt
        );
        let waiting = SendFacts {
            permission_waits: true,
            ..running
        };
        assert_eq!(
            delivery(Action::Default, TurnDelivery::Queue, waiting),
            Delivery::Interrupt
        );
        assert_eq!(
            delivery(Action::Alternate, TurnDelivery::Queue, waiting),
            Delivery::Queue
        );
        let no_steer = SendFacts {
            steers: false,
            ..running
        };
        assert_eq!(
            delivery(Action::Default, TurnDelivery::Steer, no_steer),
            Delivery::Interrupt
        );
        for chosen in TurnDelivery::ALL {
            let idle = SendFacts {
                turn_running: false,
                ..running
            };
            assert_eq!(
                delivery(Action::Default, chosen, idle),
                delivery(Action::Alternate, chosen, idle)
            );
        }
    }
}
