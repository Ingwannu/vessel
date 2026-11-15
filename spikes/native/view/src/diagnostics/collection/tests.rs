//! Adversarial run transitions, privacy and combined byte boundaries.
use super::*;
use crate::devices::Tickets;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-15T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}
fn client() -> ReportText {
    ReportText::new("Vessel app diagnostics\n  Client: ready").unwrap()
}
fn ticket(tickets: &mut Tickets) -> Ticket {
    tickets.next_ticket().unwrap()
}
fn facts(node: NodeId) -> NodeFacts {
    NodeFacts {
        node,
        protocol: ProtocolVersion::new(13),
        state: NodeState::Serving,
    }
}
fn result(node: NodeId) -> Result<HostResult, Problem> {
    Ok(HostResult {
        admitted: node,
        facts: Ok(facts(node)),
    })
}

#[test]
fn every_source_outcome_is_independent_and_unavailable_is_not_healthy() {
    let mut tickets = Tickets::default();
    let t = ticket(&mut tickets);
    let good = MachineKey::from_book("secret.example");
    let bad = MachineKey::from_book("offline");
    let mut run = Collection::default();
    run.start(
        t,
        now(),
        client(),
        [(good.clone(), Ok(())), (bad, Err(Problem::Offline))],
    );
    assert_eq!(run.phase(), &Phase::Collecting);
    assert!(run.report().is_none());
    run.running(t, &good);
    run.answer(t, &good, result(NodeId::random()));
    assert_eq!(run.phase(), &Phase::Ready);
    let text = run.report().unwrap().as_str();
    assert!(text.contains("Protocol: 13"));
    assert!(text.contains("host is not connected"));
    assert!(text.contains("Daemon diagnostic: API unavailable"));
    assert!(!text.contains("secret.example"));
    assert!(!text.contains("no errors"));
}

#[test]
fn late_answer_cannot_cross_refresh_close_or_offline_invalidation() {
    for invalidate in [false, true] {
        let mut run = Collection::default();
        let mut tickets = Tickets::default();
        let old = ticket(&mut tickets);
        let new = ticket(&mut tickets);
        let host = MachineKey::from_book("laptop");
        run.start(old, now(), client(), [(host.clone(), Ok(()))]);
        run.running(old, &host);
        run.start(new, now(), client(), [(host.clone(), Ok(()))]);
        run.running(new, &host);
        run.answer(old, &host, result(NodeId::random()));
        assert_eq!(run.phase(), &Phase::Collecting);
        if invalidate {
            run.invalidate(&host);
            run.answer(new, &host, result(NodeId::random()));
            assert_eq!(run.phase(), &Phase::Ready);
            assert!(
                run.report()
                    .unwrap()
                    .as_str()
                    .contains("host is not connected")
            );
        } else {
            run.close();
            run.answer(new, &host, result(NodeId::random()));
            assert_eq!(run.phase(), &Phase::Hidden);
            assert!(run.report().is_none());
        }
    }
}

#[test]
fn duplicate_unrequested_and_mismatched_answers_are_refused() {
    let mut tickets = Tickets::default();
    let t = ticket(&mut tickets);
    let host = MachineKey::from_book("laptop");
    let mut run = Collection::default();
    run.start(t, now(), client(), [(host.clone(), Ok(()))]);
    run.answer(t, &host, result(NodeId::random()));
    assert_eq!(run.phase(), &Phase::Collecting);
    run.running(t, &host);
    let admitted = NodeId::random();
    run.answer(
        t,
        &host,
        Ok(HostResult {
            admitted,
            facts: Ok(facts(NodeId::random())),
        }),
    );
    assert!(matches!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::InvalidReply)
    ));
    let original = run.report().cloned();
    run.answer(t, &host, result(admitted));
    assert_eq!(run.report(), original.as_ref());
}

#[test]
fn byte_and_host_caps_refuse_whole_report_instead_of_truncating() {
    let exact = "a".repeat(MAX_REPORT_BYTES);
    assert!(ReportText::new(&exact).is_ok());
    assert_eq!(
        ReportText::new(&(exact.clone() + "a")),
        Err(Problem::TooLarge)
    );
    assert_eq!(
        ReportText::new(&"가".repeat(MAX_REPORT_BYTES)),
        Err(Problem::TooLarge)
    );
    let mut tickets = Tickets::default();
    let t = ticket(&mut tickets);
    let mut run = Collection::default();
    run.start(
        t,
        now(),
        ReportText::new(&exact).unwrap(),
        [(MachineKey::from_book("a"), Err(Problem::Offline))],
    );
    assert_eq!(run.phase(), &Phase::Failed(Problem::TooLarge));
    assert!(run.report().is_none());
    run.start(
        t,
        now(),
        client(),
        (0..=MAX_REPORT_HOSTS).map(|n| (MachineKey::from_book(&n.to_string()), Ok(()))),
    );
    assert_eq!(run.phase(), &Phase::Failed(Problem::TooLarge));
    assert!(run.rows().is_empty());
}

#[test]
fn all_failure_categories_remain_explicit_and_a_zero_host_report_is_copyable() {
    let failures = [
        Problem::Offline,
        Problem::Busy,
        Problem::Unavailable,
        Problem::Unsupported,
        Problem::Forbidden,
        Problem::TimedOut,
        Problem::InvalidReply,
        Problem::Failed,
        Problem::Exhausted,
    ];
    let mut tickets = Tickets::default();
    for problem in failures {
        let t = ticket(&mut tickets);
        let mut run = Collection::default();
        run.start(
            t,
            now(),
            client(),
            [(MachineKey::from_book("host"), Err(problem))],
        );
        assert_eq!(run.phase(), &Phase::Ready);
        assert!(
            run.report()
                .unwrap()
                .as_str()
                .contains(&problem.to_string())
        );
    }
    let mut run = Collection::default();
    run.start(ticket(&mut tickets), now(), client(), []);
    assert_eq!(run.phase(), &Phase::Ready);
    assert!(run.report().is_some());
}
