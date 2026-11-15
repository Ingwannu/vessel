//! "Release idle agents" over the node's `park_after` (Paseo's
//! `IdleRuntimeCleanupCard`): what the switch shows, what pressing it
//! sends, and that a late or foreign answer changes nothing.

use vessel_proto::parking::{IdleMinutes, ParkAfter};
use vessel_proto::restart::RestartRecovery;
use vessel_proto::settings::{AgentDriver, AgentDrivers, NodeSettings};
use vessel_proto::{ErrorCode, ProtocolVersion};

use crate::devices::{MachineKey, Tickets};
use crate::idle_release::{Card, Page, Problem, Switch};

fn idle(minutes: u16) -> NodeSettings {
    NodeSettings {
        restart_recovery: RestartRecovery::default(),
        drivers: AgentDrivers {
            claude_code: AgentDriver::Acp,
            codex: AgentDriver::Acp,
            open_code: AgentDriver::Acp,
        },
        park_after: ParkAfter::Idle {
            minutes: IdleMinutes::new(minutes).unwrap(),
        },
    }
}

fn never() -> NodeSettings {
    NodeSettings {
        restart_recovery: RestartRecovery::default(),
        drivers: AgentDrivers::default(),
        park_after: ParkAfter::Never,
    }
}

const NEW: Option<ProtocolVersion> = Some(ProtocolVersion::new(15));

#[test]
fn shows_on_with_the_minutes_in_force_and_off_for_never() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let first = tickets.next_ticket().unwrap();
    let mut page = Page::reading(laptop.clone(), first);
    assert_eq!(page.card(NEW), Card::Reading);
    assert!(page.answered(&laptop, first, Ok(idle(30))));
    assert_eq!(
        page.card(NEW),
        Card::Switch(Switch {
            on: true,
            minutes: Some(30),
            pending: false,
            refused: None
        })
    );
    let mut off = Page::reading(laptop.clone(), first);
    assert!(off.answered(&laptop, first, Ok(never())));
    assert!(matches!(
        off.card(NEW),
        Card::Switch(Switch {
            on: false,
            minutes: None,
            ..
        })
    ));
}

#[test]
fn turning_off_sends_never_and_on_restores_paseos_hour() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let read = tickets.next_ticket().unwrap();
    let mut page = Page::reading(laptop.clone(), read);
    assert!(page.answered(&laptop, read, Ok(idle(30))));
    let off = tickets.next_ticket().unwrap();
    assert_eq!(page.toggle(off), Some(ParkAfter::Never));
    // A second press waits for the first change.
    assert_eq!(page.toggle(tickets.next_ticket().unwrap()), None);
    assert!(matches!(
        page.card(NEW),
        Card::Switch(Switch { pending: true, .. })
    ));
    assert!(page.answered(&laptop, off, Ok(never())));
    let on = tickets.next_ticket().unwrap();
    assert_eq!(page.toggle(on), Some(ParkAfter::DEFAULT));
    assert!(page.answered(&laptop, on, Ok(idle(60))));
    assert!(matches!(
        page.card(NEW),
        Card::Switch(Switch {
            on: true,
            minutes: Some(60),
            pending: false,
            refused: None
        })
    ));
}

#[test]
fn a_refused_change_keeps_the_value_in_force_and_says_why() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let read = tickets.next_ticket().unwrap();
    let mut page = Page::reading(laptop.clone(), read);
    assert!(page.answered(&laptop, read, Ok(idle(60))));
    let change = tickets.next_ticket().unwrap();
    assert_eq!(page.toggle(change), Some(ParkAfter::Never));
    let forbidden = Problem::Refused(ErrorCode::Forbidden);
    assert!(page.answered(&laptop, change, Err(forbidden)));
    assert_eq!(
        page.card(NEW),
        Card::Switch(Switch {
            on: true,
            minutes: Some(60),
            pending: false,
            refused: Some(forbidden)
        })
    );
}

#[test]
fn ignores_an_answer_for_another_machine_or_an_older_request() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let old = tickets.next_ticket().unwrap();
    let new = tickets.next_ticket().unwrap();
    let mut page = Page::reading(laptop.clone(), new);
    assert!(!page.answered(&laptop, old, Ok(never())));
    assert!(!page.answered(&MachineKey::from_book("build-server"), new, Ok(never())));
    assert_eq!(page.card(NEW), Card::Reading);
}

#[test]
fn has_nothing_to_switch_on_an_older_node_or_a_value_it_does_not_know() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let read = tickets.next_ticket().unwrap();
    let mut page = Page::reading(laptop.clone(), read);
    assert_eq!(page.card(Some(ProtocolVersion::new(11))), Card::TooOld);
    assert!(page.answered(
        &laptop,
        read,
        Ok(NodeSettings {
            restart_recovery: RestartRecovery::default(),
            drivers: AgentDrivers {
                claude_code: AgentDriver::Acp,
                codex: AgentDriver::Acp,
                open_code: AgentDriver::Acp
            },
            park_after: ParkAfter::Unknown
        })
    ));
    assert_eq!(page.toggle(tickets.next_ticket().unwrap()), None);
    assert!(matches!(page.card(NEW), Card::Failed(_)));
    let mut failed = Page::reading(laptop.clone(), read);
    assert!(failed.answered(&laptop, read, Err(Problem::Offline)));
    assert_eq!(failed.card(NEW), Card::Failed(Problem::Offline));
    assert_eq!(failed.toggle(tickets.next_ticket().unwrap()), None);
}
