//! One canonical complete safe report fixture, available only to unit tests.
use super::DaemonSnapshot;
use chrono::DateTime;
use std::sync::Arc;
use vessel_proto::{
    DurationMs, NodeId, NodeState,
    diagnostics::{
        DiagnosticArch, DiagnosticBinding, DiagnosticCount, DiagnosticErrorCoverage,
        DiagnosticFailureWindow, DiagnosticHome, DiagnosticListeners, DiagnosticLocalKind,
        DiagnosticNode, DiagnosticOs, DiagnosticRelays, DiagnosticReport, DiagnosticReportFields,
        DiagnosticRuntimeSessions, DiagnosticSection, DiagnosticStore, DiagnosticStoredSessions,
    },
};
fn available<T>(value: T) -> DiagnosticSection<T> {
    DiagnosticSection::Available {
        at: DateTime::UNIX_EPOCH,
        value,
    }
}
pub(super) fn complete(node: NodeId) -> DaemonSnapshot {
    let sessions = DiagnosticStoredSessions {
        total: DiagnosticCount::new(0),
        live: DiagnosticCount::new(0),
        ended: DiagnosticCount::new(0),
    };
    DaemonSnapshot::Report(Arc::new(
        DiagnosticReport::try_from(DiagnosticReportFields {
            collected_at: DateTime::UNIX_EPOCH,
            node: DiagnosticNode {
                id: node,
                version: None,
                os: DiagnosticOs::Linux,
                arch: DiagnosticArch::X86_64,
                state: NodeState::Serving,
                uptime: DurationMs::from_duration(std::time::Duration::ZERO),
                home: DiagnosticHome::Redacted,
            },
            process: available(vessel_proto::diagnostics::DiagnosticProcess {
                pid: 42.try_into().unwrap(),
                parent: None,
                memory: available(vessel_proto::diagnostics::DiagnosticMemory {
                    resident_bytes: DiagnosticCount::new(4096),
                    virtual_bytes: DiagnosticCount::new(8192),
                }),
            }),
            store: available(DiagnosticStore {
                expected_schema: 1_u32.try_into().unwrap(),
                observed_schema: 1_u32.try_into().unwrap(),
                sessions,
            }),
            agents: available(Vec::new().try_into().unwrap()),
            sessions: available(DiagnosticRuntimeSessions {
                live: DiagnosticCount::new(0),
                parked: DiagnosticCount::new(0),
            }),
            listeners: available(DiagnosticListeners {
                local_kind: DiagnosticLocalKind::Socket,
                local: DiagnosticBinding::Bound,
                direct: DiagnosticBinding::Disabled,
            }),
            relays: available(DiagnosticRelays::default()),
            recent_failures: available(DiagnosticFailureWindow {
                coverage: DiagnosticErrorCoverage::LocalRpcCompletions,
                observed_since: DateTime::UNIX_EPOCH,
                entries: Vec::new().try_into().unwrap(),
                omitted: DiagnosticCount::new(0),
            }),
        })
        .unwrap(),
    ))
}
