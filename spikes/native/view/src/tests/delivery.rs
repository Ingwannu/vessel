//! What Send does with a prompt (`session::delivery`): Paseo's
//! `composer/input/state.test.ts` "composer send behavior" cases, its
//! server's steer-or-replace, and the default.

use vessel_proto::Delivery;

use crate::prefs::{Preferences, TurnDelivery};
use crate::session::{SendFacts, send_delivery, shows_at_once};

const fn facts(turn_running: bool, steers: bool, permission_waits: bool) -> SendFacts {
    SendFacts {
        turn_running,
        steers,
        permission_waits,
    }
}

#[test]
fn the_default_is_paseos_steer() {
    assert_eq!(
        Preferences::default().composer.delivery,
        TurnDelivery::Steer
    );
}

#[test]
fn with_no_turn_running_every_choice_starts_one() {
    for chosen in TurnDelivery::ALL {
        for steers in [false, true] {
            assert_eq!(
                send_delivery(chosen, facts(false, steers, false)),
                Delivery::Queue,
                "{chosen:?}"
            );
        }
    }
}

// Paseo: "sends immediately when queue mode cannot advance past a
// permission".
#[test]
fn queue_interrupts_while_a_permission_waits_and_only_queue_changes() {
    assert_eq!(
        send_delivery(TurnDelivery::Queue, facts(true, true, true)),
        Delivery::Interrupt
    );
    assert_eq!(
        send_delivery(TurnDelivery::Queue, facts(true, true, false)),
        Delivery::Queue
    );
    assert_eq!(
        send_delivery(TurnDelivery::Steer, facts(true, true, true)),
        Delivery::Steer
    );
    assert_eq!(
        send_delivery(TurnDelivery::Interrupt, facts(true, true, true)),
        Delivery::Interrupt
    );
}

// Paseo's server: steer joins the turn, or replaces it when the agent
// cannot take a message mid-turn.
#[test]
fn steer_joins_the_turn_or_replaces_it() {
    assert_eq!(
        send_delivery(TurnDelivery::Steer, facts(true, true, false)),
        Delivery::Steer
    );
    assert_eq!(
        send_delivery(TurnDelivery::Steer, facts(true, false, false)),
        Delivery::Interrupt
    );
}

#[test]
fn a_send_shows_at_once_unless_it_waits_in_the_queue() {
    assert!(shows_at_once(Delivery::Queue, false), "it starts a turn");
    assert!(
        !shows_at_once(Delivery::Queue, true),
        "the queue's rows show it"
    );
    assert!(shows_at_once(Delivery::Steer, true));
    assert!(shows_at_once(Delivery::Interrupt, true));
}
