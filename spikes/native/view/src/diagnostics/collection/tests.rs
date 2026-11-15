//! Adversarial run transitions, privacy and combined byte boundaries.
use super::*;
use crate::devices::Tickets;

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
fn result(node: NodeId) -> HostResult {
    HostResult {
        admitted: node,
        facts: Ok(facts(node)),
        daemon: DaemonSnapshot::Unavailable(Problem::Unavailable),
    }
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
        client(),
        [(good.clone(), Ok(())), (bad, Err(Problem::Offline))],
    );
    assert_eq!(run.phase(), &Phase::Collecting);
    assert!(run.report().is_none());
    run.running(t, &good);
    run.answer(t, &good, Ok(result(NodeId::random())));
    assert_eq!(run.phase(), &Phase::Ready);
    let text = run.report().unwrap().as_str();
    assert!(text.contains("Protocol: 13"));
    assert!(text.contains("host is not connected"));
    assert!(
        text.contains("Daemon diagnostic\n  Status: diagnostic API is not available in this build")
    );
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
        run.start(old, client(), [(host.clone(), Ok(()))]);
        run.running(old, &host);
        run.start(new, client(), [(host.clone(), Ok(()))]);
        run.running(new, &host);
        run.answer(old, &host, Ok(result(NodeId::random())));
        assert_eq!(run.phase(), &Phase::Collecting);
        if invalidate {
            run.invalidate(&host);
            run.answer(new, &host, Ok(result(NodeId::random())));
            assert_eq!(run.phase(), &Phase::Ready);
            assert!(
                run.report()
                    .unwrap()
                    .as_str()
                    .contains("host is not connected")
            );
        } else {
            run.close();
            run.answer(new, &host, Ok(result(NodeId::random())));
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
    run.start(t, client(), [(host.clone(), Ok(()))]);
    run.answer(t, &host, Ok(result(NodeId::random())));
    assert_eq!(run.phase(), &Phase::Collecting);
    run.running(t, &host);
    let admitted = NodeId::random();
    run.answer(
        t,
        &host,
        Ok(HostResult {
            admitted,
            facts: Ok(facts(NodeId::random())),
            daemon: DaemonSnapshot::Unavailable(Problem::Unavailable),
        }),
    );
    assert!(matches!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::InvalidReply)
    ));
    let original = run.report().cloned();
    run.answer(t, &host, Ok(result(admitted)));
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
        ReportText::new(&exact).unwrap(),
        [(MachineKey::from_book("a"), Err(Problem::Offline))],
    );
    assert_eq!(run.phase(), &Phase::Failed(Problem::TooLarge));
    assert!(run.report().is_none());
    run.start(
        t,
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
        Problem::ConnectionChanged,
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
        run.start(t, client(), [(MachineKey::from_book("host"), Err(problem))]);
        assert_eq!(run.phase(), &Phase::Ready);
        assert!(
            run.report()
                .unwrap()
                .as_str()
                .contains(&problem.to_string())
        );
    }
    let mut run = Collection::default();
    run.start(ticket(&mut tickets), client(), []);
    assert_eq!(run.phase(), &Phase::Ready);
    assert!(run.report().is_some());
}

#[test]
fn duplicate_host_keys_are_an_explicit_refusal_not_a_permanent_spinner() {
    let mut tickets = Tickets::default();
    let mut run = Collection::default();
    let host = MachineKey::from_book("same");
    run.start(
        ticket(&mut tickets),
        client(),
        [(host.clone(), Ok(())), (host, Ok(()))],
    );
    assert_eq!(run.phase(), &Phase::Failed(Problem::InvalidReply));
    assert!(run.report().is_none());
}

#[test]
fn client_section_keeps_the_real_saved_count_without_host_names_or_errors() {
    let facts = crate::diagnostics::AppFacts {
        collected_at: "2026-11-15T00:00:00Z".to_owned(),
        label: "dev".to_owned(),
        platform: "linux x86_64".to_owned(),
        renderer: crate::diagnostics::Renderer::Unknown,
        preferences: "file".to_owned(),
        log: None,
        home: None,
    };
    let report =
        crate::diagnostics::client_report(&facts, HostCount::from_book(MAX_REPORT_HOSTS)).unwrap();
    assert!(
        report
            .as_str()
            .contains(&format!("Saved machines: {MAX_REPORT_HOSTS}"))
    );
}

#[test]
fn a_failed_refresh_invalidates_its_predecessor_and_cannot_copy_the_old_report() {
    let mut tickets = Tickets::default();
    let t = ticket(&mut tickets);
    let host = MachineKey::from_book("host");
    let mut run = Collection::default();
    run.start(t, client(), [(host.clone(), Ok(()))]);
    run.running(t, &host);
    run.refuse(Problem::Exhausted);
    run.answer(t, &host, Ok(result(NodeId::random())));
    assert_eq!(run.phase(), &Phase::Failed(Problem::Exhausted));
    assert!(!run.has_client_report());
    assert!(run.report().is_none());
}

#[test]
fn equal_node_reconnect_invalidates_old_results_before_and_after_completion() {
    for settled in [false, true] {
        let mut tickets = Tickets::default();
        let ticket = tickets.next_ticket().unwrap();
        let host = MachineKey::from_book("same book entry");
        let node = NodeId::random();
        let mut run = Collection::default();
        run.start(ticket, client(), [(host.clone(), Ok(()))]);
        run.running(ticket, &host);
        if settled {
            run.answer(ticket, &host, Ok(result(node)));
        }
        run.connected(&host, node);
        run.answer(ticket, &host, Ok(result(node)));
        assert!(matches!(
            run.rows().first().unwrap().progress,
            Progress::Failed(Problem::ConnectionChanged)
        ));
        assert!(!run.report().unwrap().as_str().contains("Protocol:"));
        let fresh = tickets.next_ticket().unwrap();
        run.start(fresh, client(), [(host.clone(), Ok(()))]);
        run.running(fresh, &host);
        run.answer(fresh, &host, Ok(result(node)));
        assert!(run.report().unwrap().as_str().contains("Protocol:"));
    }
}

#[test]
fn report_scope_counts_requested_hosts_independently_of_saved_host_count() {
    let mut tickets = Tickets::default();
    let mut run = Collection::default();
    let first = MachineKey::from_book("first-private-label");
    let second = MachineKey::from_book("second-private-label");
    run.start(
        tickets.next_ticket().unwrap(),
        client(),
        [
            (first.clone(), Err(Problem::Offline)),
            (second, Err(Problem::Offline)),
        ],
    );
    assert!(
        run.report()
            .unwrap()
            .as_str()
            .contains("Requested host reports: 2")
    );
    run.start(
        tickets.next_ticket().unwrap(),
        client(),
        [(first, Err(Problem::Offline))],
    );
    assert!(
        run.report()
            .unwrap()
            .as_str()
            .contains("Requested host reports: 1")
    );
    assert!(!run.report().unwrap().as_str().contains("private-label"));
}

#[test]
fn a_shared_welcome_identity_cannot_be_retargeted_to_another_client_reply() {
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let host = MachineKey::from_book("host");
    let expected = NodeId::random();
    let other = NodeId::random();
    let mut run = Collection::default();
    run.start(ticket, client(), [(host.clone(), Ok(()))]);
    run.expect_node(&host, expected);
    run.running(ticket, &host);
    run.expect_node(&host, other);
    run.answer(ticket, &host, Ok(result(other)));
    assert!(matches!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::InvalidReply)
    ));
    assert!(!run.report().unwrap().as_str().contains(&other.to_string()));
}

#[test]
fn the_shared_fleet_retires_changed_removed_and_offline_identity_without_raw_errors() {
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let host = MachineKey::from_book("host");
    let first = NodeId::random();
    let replacement = NodeId::random();
    let mut run = Collection::default();
    run.start(ticket, client(), [(host.clone(), Ok(()))]);
    run.expect_node(&host, first);
    run.running(ticket, &host);
    run.answer(ticket, &host, Ok(result(first)));
    let mut machine = Machine::unreachable("host".to_owned(), "PRIVATE-FAILURE-MUST-NOT-COPY");
    machine.reach = Reach::Online;
    machine.node = Some(replacement);
    run.reconcile(&[machine.clone()]);
    assert!(matches!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::ConnectionChanged)
    ));
    machine.reach = Reach::Unreachable("PRIVATE-FAILURE-MUST-NOT-COPY".to_owned());
    run.reconcile(&[machine]);
    assert!(matches!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::Offline)
    ));
    assert!(!run.report().unwrap().as_str().contains("PRIVATE-FAILURE"));
    run.reconcile(&[]);
    assert!(matches!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::Offline)
    ));
}

#[test]
fn exact_admission_epochs_fence_same_node_answers_retirement_and_fleet_reconnect() {
    use crate::connection::ConnectionEpochs;
    let id = NodeId::random();
    let machine = MachineKey::from_book("epoch-host");
    let mut epochs = ConnectionEpochs::default();
    let old = epochs.admit(machine.clone(), id).unwrap();
    let next = epochs.admit(machine.clone(), id).unwrap();
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let mut run = Collection::default();
    run.start(
        ticket,
        ReportText::new("Client").unwrap(),
        [(machine.clone(), Ok(()))],
    );
    run.expect_connection(old.clone());
    run.running(ticket, &machine);
    run.answer_connection(ticket, &next, Err(Problem::Failed));
    assert_eq!(run.rows().first().unwrap().progress, Progress::Running);
    // A stale retirement for another epoch cannot retire the selected one.
    run.retired(&next.tag());
    assert_eq!(run.rows().first().unwrap().progress, Progress::Running);
    let mut snapshot = Machine::unreachable("epoch-host".into(), "private test reason");
    snapshot.node = Some(id);
    snapshot.connection_epoch = Some(old.epoch());
    run.reconcile(&[snapshot.clone()]);
    assert_eq!(run.rows().first().unwrap().progress, Progress::Running);
    snapshot.reach = Reach::Online;
    snapshot.connection_epoch = Some(next.epoch());
    run.reconcile(&[snapshot]);
    run.answer_connection(ticket, &old, Err(Problem::Failed));
    assert_eq!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::Offline)
    );
    let next_ticket = tickets.next_ticket().unwrap();
    run.start(
        next_ticket,
        ReportText::new("Client").unwrap(),
        [(machine.clone(), Ok(()))],
    );
    run.expect_connection(next.clone());
    run.running(next_ticket, &machine);
    run.answer_connection(ticket, &old, Err(Problem::Failed));
    assert_eq!(run.rows().first().unwrap().progress, Progress::Running);
    run.retired(&next.tag());
    run.answer_connection(next_ticket, &next, Err(Problem::Failed));
    assert_eq!(
        run.rows().first().unwrap().progress,
        Progress::Failed(Problem::Offline)
    );
}
