//! Paseo's tracker, and what a machine makes of the reports (its 180 s
//! policy, as vessel-node `push/policy.rs:13-55` ports Paseo's
//! `agent-attention-policy.ts:3,42-76`).

use super::*;

const BOX: &str = "box";
const LAB: &str = "lab";

/// Two sessions, the same for every use in a test run.
static SESSIONS: std::sync::LazyLock<[SessionId; 2]> =
    std::sync::LazyLock::new(|| [SessionId::random(), SessionId::random()]);

fn session(n: usize) -> SessionId {
    *SESSIONS.get(n.saturating_sub(1)).unwrap()
}

/// The machine's rule: an `Active` report is stamped with its own time;
/// present means stamped within 180 s; a present window focused on the
/// target suppresses; else the window with the latest input is told; else
/// push.
#[derive(Default)]
struct Machine {
    windows: Vec<(&'static str, PushFocus, Option<Millis>)>,
}

#[derive(Debug, PartialEq, Eq)]
enum Plan {
    Suppressed,
    Told(&'static str),
    Push,
}

impl Machine {
    fn report(&mut self, window: &'static str, report: &ReportPushPresence, now: Millis) {
        let before = self
            .windows
            .iter()
            .position(|(name, _focus, _at)| *name == window);
        let at = match report.activity {
            PushActivity::Active => Some(now),
            PushActivity::Unchanged => before
                .and_then(|index| self.windows.get(index))
                .and_then(|(_name, _focus, at)| *at),
        };
        let entry = (window, report.focus, at);
        match before.and_then(|index| self.windows.get_mut(index)) {
            Some(kept) => *kept = entry,
            None => self.windows.push(entry),
        }
    }

    fn plan(&self, target: PushFocus, now: Millis) -> Plan {
        let mut told: Option<(&'static str, Millis)> = None;
        for (window, focus, at) in &self.windows {
            let Some(at) = at.map(|at| at.min(now)) else {
                continue;
            };
            if now.saturating_sub(at) > 180_000 {
                continue;
            }
            if *focus == target {
                return Plan::Suppressed;
            }
            if told.is_none_or(|(_window, latest)| at > latest) {
                told = Some((window, at));
            }
        }
        told.map_or(Plan::Push, |(window, _at)| Plan::Told(window))
    }
}

/// One window on one machine connection, its clock run by the test, with
/// a heartbeat every 15 s as the app's timer sends it.
struct Window {
    name: &'static str,
    presence: Presence,
    told: Told<&'static str>,
    now: Millis,
    sent: Vec<ReportPushPresence>,
}

impl Window {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            presence: Presence::new(0, true),
            told: Told::default(),
            now: 0,
            sent: Vec::new(),
        }
    }

    fn send(&mut self, machine: &mut Machine) {
        let snapshot = self.presence.snapshot(self.now);
        let report = self.told.report(BOX, BOX, &snapshot, 0);
        machine.report(self.name, &report, self.now);
        self.sent.push(report);
    }

    fn pass(&mut self, ms: Millis, machine: &mut Machine) {
        let end = self.now.saturating_add(ms);
        let beat = millis(HEARTBEAT);
        let mut next = self
            .now
            .checked_div(beat)
            .unwrap()
            .saturating_add(1)
            .saturating_mul(beat);
        while next <= end {
            self.now = next;
            self.send(machine);
            next = next.saturating_add(beat);
        }
        self.now = end;
    }

    fn last(&self) -> &ReportPushPresence {
        self.sent.last().unwrap()
    }
}

#[test]
fn the_first_report_carries_opening_and_later_ones_only_new_input() {
    let mut machine = Machine::default();
    let mut window = Window::new("window");
    window.send(&mut machine);
    assert_eq!(window.last().activity, PushActivity::Active);
    assert_eq!(window.last().focus, PushFocus::Elsewhere);
    assert_eq!(window.last().idle_age, None);
    window.pass(15_000, &mut machine);
    assert_eq!(window.last().activity, PushActivity::Unchanged);
    window.presence.input(window.now);
    window.pass(15_000, &mut machine);
    assert_eq!(window.last().activity, PushActivity::Active);
    window.pass(15_000, &mut machine);
    assert_eq!(window.last().activity, PushActivity::Unchanged);
}

#[test]
fn immediate_heartbeats_go_at_most_every_five_seconds() {
    let mut presence = Presence::new(0, true);
    assert!(presence.immediate(0));
    assert!(!presence.immediate(0));
    assert!(!presence.immediate(4_999));
    assert!(presence.immediate(5_000));
    assert!(!presence.immediate(9_999));
}

#[test]
fn a_new_shown_session_is_input_and_goes_at_once_the_same_one_does_not() {
    let mut presence = Presence::new(0, true);
    assert!(presence.show(Some((BOX, session(1))), 1_000));
    assert!(!presence.show(Some((BOX, session(1))), 2_000));
    let snapshot = presence.snapshot(2_000);
    assert_eq!(snapshot.last_input, 1_000);
    assert_eq!(
        snapshot.focus(BOX),
        PushFocus::Session {
            session: session(1)
        }
    );
    assert_eq!(snapshot.focus(LAB), PushFocus::Elsewhere);
    assert!(presence.show(Some((LAB, session(1))), 3_000));
    assert_eq!(presence.snapshot(3_000).focus(BOX), PushFocus::Elsewhere);
    assert!(presence.show(None, 4_000));
    assert!(!presence.show(None, 5_000));
}

#[test]
fn hidden_reports_background_and_visible_again_is_input() {
    let mut presence = Presence::new(0, true);
    assert!(presence.show(Some((BOX, session(1))), 0));
    assert!(presence.visibility(false, 60_000));
    assert!(!presence.visibility(false, 61_000));
    let hidden = presence.snapshot(61_000);
    assert_eq!(hidden.focus(BOX), PushFocus::Background);
    assert_eq!(hidden.last_input, 0, "hiding is not input");
    assert!(presence.visibility(true, 75_000));
    let back = presence.snapshot(75_000);
    assert_eq!(back.last_input, 75_000);
    assert_eq!(
        back.focus(BOX),
        PushFocus::Session {
            session: session(1)
        }
    );
}

#[test]
fn a_new_connection_hears_active_only_for_input_in_the_last_fifteen_seconds() {
    let presence = Presence::new(0, true);
    let mut told = Told::default();
    let snapshot = presence.snapshot(10_000);
    assert_eq!(
        told.report("first", BOX, &snapshot, 5_000).activity,
        PushActivity::Active
    );
    assert_eq!(
        told.report("second", BOX, &snapshot, 5_001).activity,
        PushActivity::Unchanged,
        "16 s after the input, counting the time since the snapshot"
    );
    assert_eq!(
        told.report("first", BOX, &snapshot, 0).activity,
        PushActivity::Unchanged,
        "already told"
    );
    told.retain(|connection| *connection == "second");
    assert_eq!(
        told.report("first", BOX, &snapshot, 0).activity,
        PushActivity::Active,
        "a connection that went away is told afresh"
    );
}

#[test]
fn a_window_that_goes_stale_lets_push_resume_180_seconds_after_its_last_input() {
    let mut machine = Machine::default();
    let mut window = Window::new("window");
    let shown = PushFocus::Session {
        session: session(1),
    };
    let other = PushFocus::Session {
        session: session(2),
    };
    assert!(window.presence.show(Some((BOX, session(1))), 0));
    window.send(&mut machine);
    assert_eq!(machine.plan(other, 0), Plan::Told("window"));
    // No more input: the heartbeats keep coming, unchanged.
    window.pass(180_000, &mut machine);
    assert_eq!(machine.plan(other, window.now), Plan::Told("window"));
    assert_eq!(machine.plan(shown, window.now), Plan::Suppressed);
    window.pass(1, &mut machine);
    assert_eq!(machine.plan(other, window.now), Plan::Push);
    assert_eq!(machine.plan(shown, window.now), Plan::Push);
}

#[test]
fn input_waiting_for_a_heartbeat_keeps_presence_at_most_fifteen_seconds_past_paseos() {
    let mut machine = Machine::default();
    let mut window = Window::new("window");
    let other = PushFocus::Session {
        session: session(2),
    };
    window.send(&mut machine);
    window.pass(1_000, &mut machine);
    window.presence.input(window.now);
    assert!(window.presence.immediate(window.now));
    window.send(&mut machine);
    window.pass(2_000, &mut machine);
    window.presence.input(window.now);
    assert!(
        !window.presence.immediate(window.now),
        "throttled: goes at 15 s"
    );
    // Paseo's window would stop being present at 183 s; this one at 195 s.
    window.pass(195_000_u64.saturating_sub(window.now), &mut machine);
    assert_eq!(machine.plan(other, window.now), Plan::Told("window"));
    window.pass(1, &mut machine);
    assert_eq!(machine.plan(other, window.now), Plan::Push);
}

#[test]
fn a_hidden_window_is_present_but_never_suppresses_as_paseo_counts_it() {
    let mut machine = Machine::default();
    let mut window = Window::new("window");
    let shown = PushFocus::Session {
        session: session(1),
    };
    assert!(window.presence.show(Some((BOX, session(1))), 0));
    window.send(&mut machine);
    assert_eq!(machine.plan(shown, 0), Plan::Suppressed);
    assert!(window.presence.visibility(false, 0));
    window.pass(15_000, &mut machine);
    assert_eq!(window.last().focus, PushFocus::Background);
    assert_eq!(machine.plan(shown, window.now), Plan::Told("window"));
    window.pass(165_000, &mut machine);
    assert_eq!(machine.plan(shown, window.now), Plan::Told("window"));
    window.pass(1, &mut machine);
    assert_eq!(machine.plan(shown, window.now), Plan::Push);
}

#[test]
fn of_two_windows_the_one_with_the_latest_input_is_told() {
    let mut machine = Machine::default();
    let mut one = Window::new("one");
    let mut two = Window::new("two");
    one.send(&mut machine);
    two.now = 10_000;
    two.presence.input(10_000);
    two.send(&mut machine);
    let other = PushFocus::Session {
        session: session(2),
    };
    assert_eq!(machine.plan(other, 10_000), Plan::Told("two"));
}
