//! Fixtures exercise the real allowlisted root, never a release-build fake producer.
use super::*;
use crate::devices::{MachineKey, Tickets};
use crate::diagnostics::collection::{Collection, HostResult, NodeFacts, Phase, Progress};
use vessel_proto::{
    DurationMs, NodeState, ProtocolVersion,
    diagnostics::{
        DiagnosticArch, DiagnosticCount, DiagnosticHome, DiagnosticNode, DiagnosticOs,
        DiagnosticReport, DiagnosticReportFields, DiagnosticRuntimeSessions, DiagnosticSection,
        DiagnosticUnavailable,
    },
};
fn missing<T>() -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable {
        reason: DiagnosticUnavailable::NotChecked,
    }
}
fn report(id: NodeId) -> DiagnosticReport {
    DiagnosticReport::try_from(DiagnosticReportFields {
        collected_at: DateTime::UNIX_EPOCH,
        node: DiagnosticNode {
            id,
            version: Some("1.2.3-private-suffix".parse().unwrap()),
            os: DiagnosticOs::Linux,
            arch: DiagnosticArch::X86_64,
            state: NodeState::Serving,
            uptime: DurationMs::from_duration(std::time::Duration::ZERO),
            home: DiagnosticHome::Redacted,
        },
        process: missing(),
        store: DiagnosticSection::Unavailable {
            reason: DiagnosticUnavailable::Stale,
        },
        agents: missing(),
        sessions: DiagnosticSection::Available {
            at: DateTime::UNIX_EPOCH,
            value: DiagnosticRuntimeSessions {
                live: DiagnosticCount::new(7),
                parked: DiagnosticCount::new(2),
            },
        },
        listeners: missing(),
        relays: missing(),
        recent_failures: missing(),
    })
    .unwrap()
}
#[test]
fn a_partial_typed_report_keeps_successful_sources_and_their_observation_times() {
    let id = NodeId::random();
    let snapshot = DaemonSnapshot::Report(Arc::new(report(id)));
    assert!(!snapshot.complete());
    let text = snapshot.render().unwrap();
    let text = text.as_str();
    assert!(text.contains("Home: ~"));
    assert!(text.contains("Unavailable: Stale"));
    assert!(!text.contains("expected_schema"));
    assert!(text.contains("Observed at: 1970-01-01"));
    assert!(text.contains(r#""live":{"value":7"#));
    assert!(text.contains("Failed local node calls (not a daemon log)"));
    assert!(!text.contains("private-suffix"));
    assert!(!text.contains("no errors"));
}
#[test]
fn safe_daemon_results_survive_an_independently_failed_basic_read() {
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let machine = MachineKey::from_book("private-label-not-for-copy");
    let id = NodeId::random();
    let mut collection = Collection::default();
    collection.start(
        ticket,
        ReportText::new("Client").unwrap(),
        [(machine.clone(), Ok(()))],
    );
    collection.running(ticket, &machine);
    collection.answer(
        ticket,
        &machine,
        Ok(HostResult {
            admitted: id,
            facts: Err(Problem::TimedOut),
            daemon: DaemonSnapshot::Report(Arc::new(report(id))),
        }),
    );
    assert_eq!(collection.phase(), &Phase::Ready);
    assert!(matches!(
        collection.rows().first().unwrap().progress,
        Progress::Partial(_)
    ));
    let text = collection.report().unwrap().as_str();
    assert!(text.contains("Node info: diagnostic collection timed out"));
    assert!(text.contains("Runtime sessions"));
    assert!(!text.contains("private-label-not-for-copy"));
}
#[test]
fn a_report_for_another_admitted_node_cannot_enter_the_copyable_result() {
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let machine = MachineKey::from_book("host");
    let id = NodeId::random();
    let mut collection = Collection::default();
    collection.start(
        ticket,
        ReportText::new("Client").unwrap(),
        [(machine.clone(), Ok(()))],
    );
    collection.running(ticket, &machine);
    collection.answer(
        ticket,
        &machine,
        Ok(HostResult {
            admitted: id,
            facts: Ok(NodeFacts {
                node: id,
                protocol: ProtocolVersion::new(13),
                state: NodeState::Serving,
            }),
            daemon: DaemonSnapshot::Report(Arc::new(report(NodeId::random()))),
        }),
    );
    assert!(matches!(
        collection.rows().first().unwrap().progress,
        Progress::Failed(Problem::InvalidReply)
    ));
    assert!(
        !collection
            .report()
            .unwrap()
            .as_str()
            .contains("Runtime sessions")
    );
}

#[test]
fn a_complete_typed_report_is_distinct_from_partial_coverage() {
    let id = NodeId::random();
    let report = crate::diagnostics::fixture::complete(id);
    assert!(report.complete());
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    let machine = MachineKey::from_book("host");
    let mut collection = Collection::default();
    collection.start(
        ticket,
        ReportText::new("Client").unwrap(),
        [(machine.clone(), Ok(()))],
    );
    collection.running(ticket, &machine);
    collection.answer(
        ticket,
        &machine,
        Ok(HostResult {
            admitted: id,
            facts: Ok(NodeFacts {
                node: id,
                protocol: ProtocolVersion::new(13),
                state: NodeState::Serving,
            }),
            daemon: report,
        }),
    );
    assert!(matches!(
        collection.rows().first().unwrap().progress,
        Progress::Done(_)
    ));
    assert!(
        !collection
            .report()
            .unwrap()
            .as_str()
            .contains("Unavailable:")
    );
}

#[test]
fn numeric_process_data_is_rendered_and_missing_memory_stays_partial() {
    let id = NodeId::random();
    let DaemonSnapshot::Report(complete) = crate::diagnostics::fixture::complete(id) else {
        panic!("fixture");
    };
    let mut fields = complete.fields().clone();
    let DiagnosticSection::Available { value, .. } = &mut fields.process else {
        panic!("process");
    };
    value.memory = DiagnosticSection::Unavailable {
        reason: DiagnosticUnavailable::Unsupported,
    };
    let snapshot = DaemonSnapshot::Report(Arc::new(DiagnosticReport::try_from(fields).unwrap()));
    assert!(!snapshot.complete());
    let text = snapshot.render().unwrap();
    assert!(
        text.as_str()
            .contains("Daemon self process (numeric metadata only)")
    );
    assert!(text.as_str().contains(r#""pid":42"#));
    assert!(text.as_str().contains("unsupported"));
    assert!(!text.as_str().contains("resident_bytes"));
}
