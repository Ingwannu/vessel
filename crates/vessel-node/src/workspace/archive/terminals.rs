//! Staged strict member stop: retain the original handle on any uncertain cleanup.
//! The registry adapter must resolve exact admitted membership before calling it.
use vessel_proto::TerminalId;
use vessel_session::terminals::{CleanupObservation, CleanupReport, TerminalHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum StopFailure {
    #[error("the resolved terminal does not match the admitted member")]
    WrongTerminal {
        expected: TerminalId,
        actual: TerminalId,
    },
    #[error("terminal {0} cleanup did not finish before the deadline")]
    Deadline(TerminalId),
    #[error("terminal {0} lost its runner without a cleanup acknowledgement")]
    Abandoned(TerminalId),
    #[error("terminal cleanup has unconfirmed postconditions: {0:?}")]
    Unconfirmed(CleanupReport),
}

/// The actual workflow's deadline includes this bounded member wait. Dropping
/// only the observation never cancels or forgets an admitted runner's cleanup.
pub(super) async fn stop_terminal(
    handle: &TerminalHandle,
    expected: TerminalId,
) -> Result<(), StopFailure> {
    if handle.id() != expected {
        return Err(StopFailure::WrongTerminal {
            expected,
            actual: handle.id(),
        });
    }
    handle.kill();
    let observed = tokio::time::timeout(crate::terminals::CLOSE_WAIT, handle.cleanup())
        .await
        .map_err(|_elapsed| StopFailure::Deadline(expected))?;
    match observed {
        CleanupObservation::Finished(report) if report.terminal() != expected => {
            Err(StopFailure::WrongTerminal {
                expected,
                actual: report.terminal(),
            })
        }
        CleanupObservation::Finished(report) if !report.is_confirmed() => {
            Err(StopFailure::Unconfirmed(report))
        }
        CleanupObservation::Finished(_) => Ok(()),
        CleanupObservation::Abandoned { terminal } => Err(StopFailure::Abandoned(terminal)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proc::CommandSpec;
    use vessel_proto::{
        SessionId,
        terminals::{TerminalInfo, TerminalKind, TerminalPlace},
    };
    use vessel_session::terminals::{Leftover, ScriptStart, Scrollback, start_script};
    fn blocked(
        folder: &std::path::Path,
        leftover: Leftover,
    ) -> (TerminalHandle, tokio_util::task::TaskTracker) {
        let tasks = tokio_util::task::TaskTracker::new();
        let info = TerminalInfo::new(
            TerminalId::random(),
            TerminalPlace::Session {
                session: SessionId::random(),
            },
            folder.to_path_buf(),
            TerminalKind::Script {
                script: "fixture".parse().unwrap(),
                command: "never-run".parse().unwrap(),
            },
            chrono::DateTime::UNIX_EPOCH,
        );
        let run = start_script(ScriptStart {
            info,
            spec: CommandSpec::new("never-run"),
            leftover,
            service: false,
            approved: || false,
            scrollback: Scrollback::DEFAULT,
            shutdown: tokio_util::sync::CancellationToken::new(),
            tasks: tasks.clone(),
        });
        (run, tasks)
    }
    #[tokio::test]
    async fn strict_stop_accepts_actual_no_spawn_cleanup_without_forgetting_the_handle() {
        let folder = tempfile::tempdir().unwrap();
        let (handle, tasks) = blocked(folder.path(), Leftover::default());
        assert_eq!(stop_terminal(&handle, handle.id()).await, Ok(()));
        assert!(handle.is_ended());
        assert!(
            matches!(handle.cleanup().await, CleanupObservation::Finished(report) if report.is_confirmed())
        );
        tasks.close();
        tasks.wait().await;
    }
    #[tokio::test]
    async fn strict_stop_rejects_wrong_member_and_retains_unconfirmed_auxiliary_cleanup() {
        let folder = tempfile::tempdir().unwrap();
        let directory = folder.path().join("not-a-file");
        tokio::fs::create_dir(&directory).await.unwrap();
        let (handle, tasks) = blocked(folder.path(), Leftover(Some(directory.clone())));
        let wrong = TerminalId::random();
        assert_eq!(
            stop_terminal(&handle, wrong).await,
            Err(StopFailure::WrongTerminal {
                expected: wrong,
                actual: handle.id()
            })
        );
        assert!(
            matches!(stop_terminal(&handle, handle.id()).await, Err(StopFailure::Unconfirmed(report)) if report.terminal() == handle.id())
        );
        assert!(directory.is_dir());
        assert!(handle.is_ended());
        tasks.close();
        tasks.wait().await;
    }
}
