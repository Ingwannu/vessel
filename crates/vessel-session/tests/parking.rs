//! Parking an idle agent and waking it (protocol 12, ADR 0038), with
//! `fake-acp` as Claude's adapter: the session stays live and `running`
//! while its agent's process is stopped, and the next prompt starts it
//! again with its conversation and its current options.
//!
//! Tests wait on records and on the session's info, never on time; the
//! idle period is short so the idle timer is what fires.

use std::time::Duration;

use vessel_proto::parking::ParkReason;
use vessel_proto::{AgentEvent, ContentBlock, SessionEvent, SessionInput, SessionStatus};
use vessel_session::{ParkRefusal, SessionError, SessionHandle};

use support::{Fixture, Log, mode, prompt, select, structured, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

/// Wait until the session's info says its agent is parked, or woken.
async fn parked(handle: &SessionHandle, parked: bool) {
    let mut info = handle.watch_info();
    let seen = info.wait_for(|info| info.parked.is_some() == parked).await;
    assert!(seen.is_ok(), "the session ended first");
}

/// The text of the reply of the turn that starts after record `from`.
async fn reply(log: &mut Log, from: usize) -> String {
    log.since(from, |event| {
        matches!(
            event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        )
    })
    .await;
    log.records
        .get(from..)
        .unwrap_or_default()
        .iter()
        .filter_map(|record| {
            if let SessionEvent::Agent {
                event:
                    AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    },
            } = &record.event
            {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect()
}

/// Idle: parked by the timer, its process gone and the session still
/// running; the next prompt starts the agent again with
/// `session/load` and the options it had, and the turn runs.
#[tokio::test]
async fn an_idle_agent_is_parked_and_the_next_prompt_wakes_it() {
    within(Box::pin(async {
        let fixture = Fixture::parking_after(Some(Duration::from_millis(200)));
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        // A person picks a mode: a wake must start the agent with it.
        let _options = handle.set_option(select("mode", "plan")).await.unwrap();
        let pid = handle.pid();
        assert!(pid.is_some());
        // The idle timer parks it.
        drop(log);
        parked(&handle, true).await;
        let info = handle.info();
        assert_eq!(info.status, SessionStatus::Running);
        assert_eq!(
            info.parked.map(|parked| parked.reason),
            Some(ParkReason::Idle)
        );
        assert_eq!(handle.pid(), None, "no process while parked");
        // What a client reads does not wake it.
        assert_eq!(
            handle
                .options()
                .await
                .unwrap()
                .iter()
                .find_map(|option| mode(std::slice::from_ref(option))),
            Some("Plan")
        );
        assert!(handle.info().parked.is_some());

        let mut log = Log::of(&handle);
        let from = log.records.len();
        handle.send(prompt("loaded")).await.unwrap();
        let said = reply(&mut log, from).await;
        assert!(said.starts_with("loaded "), "{said}");
        assert!(!said.ends_with("none"), "resumed with session/load: {said}");
        assert!(handle.info().parked.is_none());
        assert!(handle.pid().is_some());
        let mut woken_mode = None;
        for option in handle.options().await.unwrap() {
            if let Some(value) = mode(std::slice::from_ref(&option)) {
                woken_mode = Some(value.to_owned());
            }
        }
        assert_eq!(woken_mode.as_deref(), Some("Plan"), "its options came back");
        let kinds: Vec<&str> = log
            .records
            .iter()
            .filter_map(|record| {
                if matches!(record.event, SessionEvent::Parked { .. }) {
                    Some("parked")
                } else if matches!(record.event, SessionEvent::Woken) {
                    Some("woken")
                } else if matches!(record.event, SessionEvent::Status { .. }) {
                    Some("status")
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            kinds.iter().filter(|kind| **kind == "status").count(),
            2,
            "only starting and running: parking changes no status ({kinds:?})"
        );
        assert!(kinds.ends_with(&["parked", "woken"]), "{kinds:?}");
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// `park` asked by hand: refused while a turn runs, done between turns;
/// answering a permission meets `Parked`; a kill ends a parked session at
/// once; `wake` on a running session does nothing.
#[tokio::test]
async fn parking_by_hand_waits_for_a_quiet_session_and_a_kill_ends_it() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        // Before its handshake it cannot be parked.
        let early = handle.park().await;
        assert!(
            matches!(
                early,
                Err(SessionError::NotParked(
                    ParkRefusal::NotReady | ParkRefusal::CannotResume
                ))
            ),
            "{early:?}"
        );
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let woken = handle.wake().await.unwrap();
        assert!(
            woken.parked.is_none(),
            "waking a running agent does nothing"
        );

        // `wait` runs until cancelled: a turn running refuses a park.
        handle.send(prompt("wait")).await.unwrap();
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnStarted { .. }
                }
            )
        })
        .await;
        let busy = handle.park().await;
        assert!(
            matches!(busy, Err(SessionError::NotParked(ParkRefusal::TurnRunning))),
            "{busy:?}"
        );
        let from = log.records.len();
        handle
            .send(SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        log.since(from, |event| {
            matches!(
                event,
                SessionEvent::Agent {
                    event: AgentEvent::TurnEnded { .. }
                }
            )
        })
        .await;

        let info = handle.park().await.unwrap();
        assert_eq!(
            info.parked.map(|parked| parked.reason),
            Some(ParkReason::Asked)
        );
        // Parked twice is parked.
        assert!(handle.park().await.unwrap().parked.is_some());
        // A cancel holds the queue and wakes nothing.
        handle
            .send(SessionInput::CancelTurn { turn: None })
            .await
            .unwrap();
        assert!(handle.info().parked.is_some());
        handle.kill();
        let end = handle.ended().await;
        assert!(
            matches!(end.status, SessionStatus::Exited { code: None }),
            "{:?}",
            end.status
        );
        assert!(end.parked.is_none(), "an ended session is not parked");
    }))
    .await;
}

/// Someone following the session live does not keep its agent awake
/// (Paseo's rule, ADR 0038's amendment of 2026-11-14): the follower sees
/// `parked` on its own subscription, and its next prompt wakes the agent
/// and is answered on that same subscription.
#[tokio::test]
async fn a_followed_session_is_parked_and_its_follower_follows_on() {
    within(Box::pin(async {
        let fixture = Fixture::parking_after(Some(Duration::from_millis(100)));
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let from = log.records.len();
        log.since(from, |event| matches!(event, SessionEvent::Parked { .. }))
            .await;
        assert_eq!(
            handle.info().parked.map(|parked| parked.reason),
            Some(ParkReason::Idle)
        );
        let from = log.records.len();
        handle.send(prompt("plain")).await.unwrap();
        assert_eq!(reply(&mut log, from).await, "echo: plain");
        assert!(
            log.records
                .get(from..)
                .unwrap_or_default()
                .iter()
                .any(|record| matches!(record.event, SessionEvent::Woken))
        );
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// The node's setting changes while sessions run (protocol 12,
/// `node.settings`): a runtime that never parked starts parking, and a
/// session idle since its turn ended is parked without a new period.
#[tokio::test]
async fn a_changed_setting_applies_to_live_sessions() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        assert_eq!(fixture.runtime.park_after(), None);
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        drop(log);
        fixture
            .runtime
            .set_park_after(Some(Duration::from_millis(100)));
        parked(&handle, true).await;
        assert_eq!(
            handle.info().parked.map(|parked| parked.reason),
            Some(ParkReason::Idle)
        );
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

#[tokio::test]
async fn parking_revokes_the_old_process_and_waking_issues_a_new_observation() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        let before = handle.process_lifetime().unwrap();
        assert_eq!(Some(before.pid().get()), handle.pid());
        let _parked = handle.park().await.unwrap();
        before.ended().await;
        assert!(handle.process_lifetime().is_none());
        let _woken = handle.wake().await.unwrap();
        let after = handle.process_lifetime().unwrap();
        assert!(!before.same_run(&after));
        assert!(!before.is_live());
        assert!(after.is_live());
        handle.kill();
        let _end = handle.ended().await;
        after.ended().await;
        assert!(handle.process_lifetime().is_none());
    }))
    .await;
}
