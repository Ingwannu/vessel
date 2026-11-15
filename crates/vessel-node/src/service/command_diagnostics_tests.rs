//! Real session/service error projection must not reopen a private cause.
use super::{ServiceError, wire};
use std::{error::Error as _, io};
use vessel_proc::{CommandDiagnostics, CommandSpec};
use vessel_session::SessionError;

const SENTINEL: &str = "PRIVATE-WIRE-SPAWN-ERROR-CANARY";

fn assert_public_edges(error: ServiceError) {
    assert!(!format!("{error:?} {error}").contains(SENTINEL));
    let mut cause = error.source();
    while let Some(next) = cause {
        assert!(!format!("{next:?} {next}").contains(SENTINEL));
        cause = next.source();
    }
    let projected = wire(&error);
    assert_eq!(projected.code, vessel_proto::ErrorCode::SpawnFailed);
    assert!(!format!("{projected:?} {projected}").contains(SENTINEL));
    assert!(
        !serde_json::to_string(&projected)
            .unwrap()
            .contains(SENTINEL)
    );
}

#[tokio::test]
async fn private_proc_and_pty_causes_stay_sealed_through_actual_wire_projection() {
    let spec = CommandSpec::new(std::env::current_exe().unwrap().join(SENTINEL))
        .arg(SENTINEL)
        .with_diagnostics(CommandDiagnostics::Redacted);
    let proc = vessel_proc::spawn(&spec).unwrap_err();
    assert_public_edges(ServiceError::Session(SessionError::StartAgent(
        vessel_agent::AgentError::Spawn(proc),
    )));
    let pty = vessel_pty::PtyError::Thread(spec.diagnostic_io_error(io::Error::other(SENTINEL)));
    assert_public_edges(ServiceError::Session(SessionError::Spawn(pty)));
}
