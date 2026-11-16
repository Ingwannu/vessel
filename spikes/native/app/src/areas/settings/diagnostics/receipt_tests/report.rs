//! The real SDK reply type, with explicit partial self-process memory.
use vessel_proto::{
    DurationMs, NodeId, NodeState,
    diagnostics::{
        DiagnosticArch, DiagnosticHome, DiagnosticNode, DiagnosticOs, DiagnosticProcess,
        DiagnosticReport, DiagnosticReportFields, DiagnosticSection, DiagnosticUnavailable,
    },
};
fn missing<T>() -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable {
        reason: DiagnosticUnavailable::NotChecked,
    }
}
pub(super) fn partial(id: NodeId) -> DiagnosticReport {
    DiagnosticReport::try_from(DiagnosticReportFields {
        collected_at: chrono::DateTime::UNIX_EPOCH,
        node: DiagnosticNode {
            id,
            version: None,
            os: DiagnosticOs::Linux,
            arch: DiagnosticArch::X86_64,
            state: NodeState::Serving,
            uptime: DurationMs::from_duration(std::time::Duration::ZERO),
            home: DiagnosticHome::Redacted,
        },
        process: DiagnosticSection::Available {
            at: chrono::DateTime::UNIX_EPOCH,
            value: DiagnosticProcess {
                pid: 42_u32.try_into().unwrap(),
                parent: None,
                memory: DiagnosticSection::Unavailable {
                    reason: DiagnosticUnavailable::Unsupported,
                },
            },
        },
        store: missing(),
        agents: missing(),
        sessions: missing(),
        listeners: missing(),
        relays: missing(),
        recent_failures: missing(),
    })
    .unwrap()
}
