//! A run asks each connected machine that speaks `node.diagnostics`, shows a
//! progress row per step while any is asked, and only then the whole text
//! (Paseo's app-diagnostic sheet; ADR 0111).

use vessel_proto::diagnostics::DiagnosticReport;
use vessel_proto::{ErrorCode, ProtocolVersion};

use super::{Run, Step, StepOf, StepState};
use crate::board::{Machine, NodeVersion, Reach};
use crate::devices::{MachineKey, Tickets};
use crate::diagnostics::{AppFacts, Problem, Renderer};

fn app() -> AppFacts {
    AppFacts {
        collected_at: "2026-11-19T08:00:00Z".to_owned(),
        label: "0.0.0.0.30".to_owned(),
        platform: "linux x86_64".to_owned(),
        renderer: Renderer::Unknown,
        preferences: "~/.config/vessel/app-prefs.json".to_owned(),
        log: None,
        home: None,
    }
}

fn machine(name: &str, reach: Reach, protocol: Option<u32>) -> Machine {
    Machine {
        node: None,
        connection_epoch: None,
        name: name.to_owned(),
        reach,
        sessions: Vec::new(),
        agents: Vec::new(),
        protocol: protocol.map(ProtocolVersion::new),
        version: Some(NodeVersion::new("0.1.0")),
    }
}

/// The node's report the page and the Rust renderer share as a fixture.
fn report() -> DiagnosticReport {
    serde_json::from_str(include_str!(
        "../../../../../../web/src/wire/fixtures/node-diagnostics.json"
    ))
    .unwrap()
}

fn machines() -> Vec<Machine> {
    vec![
        machine("laptop", Reach::Online, Some(33)),
        machine("old", Reach::Online, Some(20)),
        machine("away", Reach::Unreachable("no answer".to_owned()), None),
    ]
}

fn step(of: StepOf, state: StepState) -> Step {
    Step { of, state }
}

#[test]
fn only_a_connected_machine_that_speaks_it_is_asked_and_the_text_waits_for_it() {
    let mut tickets = Tickets::default();
    let (mut run, asks) = Run::start(app(), &machines(), &mut tickets);
    assert_eq!(asks.len(), 1);
    let (key, ticket) = asks[0].clone();
    assert_eq!(key, MachineKey::from_book("laptop"));
    assert!(run.running());
    assert_eq!(run.text(), None, "no text while a machine is asked");
    assert_eq!(
        run.steps(),
        vec![
            step(StepOf::App, StepState::Done),
            step(StepOf::Machine("laptop".to_owned()), StepState::Running),
            step(StepOf::Machine("old".to_owned()), StepState::Done),
            step(StepOf::Machine("away".to_owned()), StepState::Done),
        ]
    );
    // An earlier run's ticket, or another machine, is not this run's.
    let (_earlier, earlier_asks) = Run::start(app(), &machines(), &mut tickets);
    assert!(!run.answered(&key, earlier_asks[0].1, Ok(report())));
    assert!(!run.answered(&MachineKey::from_book("old"), ticket, Ok(report())));
    assert!(run.answered(&key, ticket, Ok(report())));
    assert!(!run.running());
    let text = run.text().unwrap();
    let node = vessel_proto::diagnostics::text::report(&report());
    assert!(
        text.contains(&format!("Machine: laptop\n  Status: online\n  Version: 0.1.0\n  Protocol: 33\n  Sessions: 0\n\n{}", node.trim_end())),
        "{text}"
    );
    assert!(text.contains("Machine: old\n  Status: online"), "{text}");
    assert!(
        text.contains("\n\nNode diagnostics: old\n  Status: unsupported by this node\n\n"),
        "{text}"
    );
    assert!(
        text.contains("Machine: away\n  Status: unreachable: no answer"),
        "{text}"
    );
    assert!(
        text.ends_with(
            "Machine: away\n  Status: unreachable: no answer\n  Version: 0.1.0\n  Protocol: unknown\n  Sessions: 0"
        ),
        "nothing below an unreachable machine: {text}"
    );
}

#[test]
fn a_failed_call_is_its_step_and_its_reason_in_the_text() {
    let mut tickets = Tickets::default();
    let (mut run, asks) = Run::start(app(), &machines(), &mut tickets);
    let (key, ticket) = asks[0].clone();
    assert!(run.answered(&key, ticket, Err(Problem::Refused(ErrorCode::Forbidden))));
    assert_eq!(
        run.steps()[1],
        step(StepOf::Machine("laptop".to_owned()), StepState::Failed)
    );
    let text = run.text().unwrap();
    assert!(
        text.contains("\n\nMachine diagnostics: laptop\n  Error: refused (forbidden)\n\n"),
        "{text}"
    );
    let (mut timed, asks) = Run::start(app(), &machines(), &mut tickets);
    assert!(timed.answered(&asks[0].0, asks[0].1, Err(Problem::TimedOut)));
    assert!(timed.text().unwrap().contains("  Error: no answer in time"));
}
