//! Report output is bounded, and hostile error descriptions stay internal.
use super::*;
use chrono::DateTime;
use vessel_proto::diagnostics::{
    DiagnosticArch, DiagnosticHome, DiagnosticNode, DiagnosticOs, DiagnosticReportFields,
    DiagnosticUnavailable,
};
use vessel_proto::{DurationMs, NodeId, NodeState};
const PRIVATE: &str = "PRIVATE-DIAGNOSTIC-OUTPUT-CANARY";
fn missing<T>() -> DiagnosticSection<T> {
    DiagnosticSection::Unavailable {
        reason: DiagnosticUnavailable::NotChecked,
    }
}
fn report() -> DiagnosticReport {
    DiagnosticReport::try_from(DiagnosticReportFields {
        collected_at: DateTime::UNIX_EPOCH,
        node: DiagnosticNode {
            id: NodeId::random(),
            version: Some(format!("1.2.3-{PRIVATE}").parse().unwrap()),
            os: DiagnosticOs::Linux,
            arch: DiagnosticArch::X86_64,
            state: NodeState::Serving,
            uptime: DurationMs::from_duration(std::time::Duration::ZERO),
            home: DiagnosticHome::Redacted,
        },
        process: missing(),
        store: missing(),
        agents: missing(),
        sessions: missing(),
        listeners: missing(),
        relays: missing(),
        recent_failures: missing(),
    })
    .unwrap()
}
#[test]
fn text_and_exact_json_render_the_safe_report_without_free_version_suffixes() {
    let report = report();
    let text = render(&report, Output::Text).unwrap();
    assert!(text.starts_with("Vessel diagnostics\n"));
    assert!(text.contains("Home: ~"));
    for name in [
        "Daemon process",
        "Store",
        "Agents",
        "Runtime sessions",
        "Listeners",
        "Relays",
        "Recent failures",
    ] {
        assert!(text.contains(name));
    }
    assert!(text.len() <= MAX_EXPORT_BYTES);
    assert!(!text.contains(PRIVATE));
    let json = render(&report, Output::Json).unwrap();
    assert_eq!(json, serde_json::to_string(&report).unwrap());
    assert_eq!(
        serde_json::from_str::<DiagnosticReport>(&json).unwrap(),
        report
    );
    assert!(!json.contains(PRIVATE));
}
#[test]
fn output_cap_refuses_before_appending_the_excess() {
    let mut out = Text::default();
    out.write_str(&"x".repeat(MAX_EXPORT_BYTES)).unwrap();
    assert!(out.write_str("x").is_err());
    assert_eq!(out.0.len(), MAX_EXPORT_BYTES);
    assert!(
        OutputError::Format(fmt::Error)
            .to_string()
            .contains(&MAX_EXPORT_BYTES.to_string())
    );
}
#[test]
fn private_rpc_and_connection_causes_remain_internal_at_the_output_boundary() {
    use vessel_client::{CallError, DiagnosticError};
    use vessel_proto::{ErrorCode, WireError};
    let error = CommandError::Request(DiagnosticError::Call(CallError::Remote(WireError::new(
        ErrorCode::Internal,
        PRIVATE,
    ))));
    assert!(!error.to_string().contains(PRIVATE));
    assert!(
        format!("{error:?}").contains(PRIVATE),
        "typed internal evidence remains available"
    );
    let error = CommandError::Connection(Box::new(crate::error::CliError::Usage(PRIVATE)));
    assert!(!error.to_string().contains(PRIVATE));
    assert!(format!("{error:?}").contains(PRIVATE));
}

#[test]
fn workspace_label_detail_does_not_reclassify_a_diagnostic_failure_or_echo_peer_text() {
    use vessel_client::{CallError, DiagnosticError};
    use vessel_proto::{
        ErrorCode, ErrorDetail, WireError, workspace_labels::WorkspaceLabelFailure,
    };
    for reason in [
        WorkspaceLabelFailure::NameTaken,
        WorkspaceLabelFailure::LabelNotFound,
        WorkspaceLabelFailure::WorkspaceNotFound,
        WorkspaceLabelFailure::WorkspaceInactive,
        WorkspaceLabelFailure::Unknown,
    ] {
        for (code, message) in [
            (
                ErrorCode::Unavailable,
                "diagnostics unavailable: node is overloaded or stopping",
            ),
            (
                ErrorCode::Conflict,
                "diagnostics unavailable: node refused the request",
            ),
        ] {
            let error =
                CommandError::Request(DiagnosticError::Call(CallError::Remote(WireError {
                    code,
                    message: PRIVATE.to_owned(),
                    detail: Some(ErrorDetail::WorkspaceLabel { reason }),
                })));
            assert_eq!(error.to_string(), message);
            assert!(!error.to_string().contains(PRIVATE));
        }
    }
}
