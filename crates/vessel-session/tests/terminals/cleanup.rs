//! The actual actor's report never upgrades bounded cleanup to physical proof.
use super::*;
use std::{future::Future, time::Duration};
use vessel_session::terminals::{CleanupFact, CleanupObservation};
mod safety;
use safety::case;
const PHASE: Duration = Duration::from_secs(10);
async fn phase<T>(name: &'static str, wait: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, wait)
        .await
        .unwrap_or_else(|_elapsed| panic!("terminal cleanup phase timed out: {name}"))
}

#[tokio::test]
async fn attempted_spawn_failure_is_not_a_no_child_cleanup_certificate() {
    case(|fixture| async move {
        let run = start_script(ScriptStart {
            info: info(script_kind(false), fixture.folder.path()),
            spec: CommandSpec::new(fixture.folder.path().join("missing-program")),
            leftover: Leftover::default(),
            service: false,
            approved: || true,
            scrollback: Scrollback::DEFAULT,
            shutdown: fixture.shutdown.clone(),
            tasks: fixture.tasks.clone(),
        });
        let CleanupObservation::Finished(report) =
            phase("actual runner final cleanup report", run.cleanup()).await
        else {
            panic!("runner lost")
        };
        assert!(matches!(run.info().status, SessionStatus::Failed { .. }));
        assert_eq!(report.process(), CleanupFact::Unconfirmed);
        assert_eq!(report.descendants(), CleanupFact::Unconfirmed);
        assert_eq!(report.io(), CleanupFact::Unconfirmed);
        assert!(!report.is_confirmed());
    })
    .await;
}

#[tokio::test]
async fn no_spawn_completion_certifies_no_child_and_waits_for_auxiliary_cleanup() {
    case(|fixture| async move {
        let output = fixture.folder.path().join("must-not-run");
        let leftover = fixture.folder.path().join("leftover");
        phase(
            "prepare auxiliary command file",
            tokio::fs::write(&leftover, b"temporary command"),
        )
        .await
        .unwrap();
        let run = start_script(ScriptStart {
            info: info(script_kind(false), fixture.folder.path()),
            spec: CommandSpec::new("/bin/sh")
                .arg("-c")
                .arg("touch must-not-run")
                .cwd(fixture.folder.path()),
            leftover: Leftover(Some(leftover.clone())),
            service: false,
            approved: || false,
            scrollback: Scrollback::DEFAULT,
            shutdown: fixture.shutdown.clone(),
            tasks: fixture.tasks.clone(),
        });
        phase("logical terminal ended status", run.ended()).await;
        let CleanupObservation::Finished(report) =
            phase("actual runner final cleanup report", run.cleanup()).await
        else {
            panic!("runner lost")
        };
        assert_eq!(report.terminal(), run.id());
        assert!(report.is_confirmed());
        assert!(!output.exists());
        assert!(!leftover.exists());
    })
    .await;
}

#[tokio::test]
async fn ended_script_joins_io_but_does_not_claim_unproved_descendant_cleanup() {
    case(|fixture| async move {
        let run = fixture.script("exit 0", false, || true);
        phase("logical terminal ended status", run.ended()).await;
        let CleanupObservation::Finished(report) =
            phase("actual runner final cleanup report", run.cleanup()).await
        else {
            panic!("runner lost")
        };
        assert_eq!(report.terminal(), run.id());
        assert!(!report.is_confirmed());
        assert_eq!(report.descendants(), CleanupFact::Unconfirmed);
        assert_eq!(report.process(), CleanupFact::Confirmed);
        assert_eq!(report.io(), CleanupFact::Confirmed);
    })
    .await;
}

#[tokio::test]
async fn failed_auxiliary_cleanup_cannot_certify_a_terminal_that_never_spawned() {
    case(|fixture| async move {
        let directory = fixture.folder.path().join("not-a-command-file");
        phase(
            "prepare auxiliary cleanup refusal",
            tokio::fs::create_dir(&directory),
        )
        .await
        .unwrap();
        let run = start_script(ScriptStart {
            info: info(script_kind(false), fixture.folder.path()),
            spec: CommandSpec::new("/bin/sh"),
            leftover: Leftover(Some(directory.clone())),
            service: false,
            approved: || false,
            scrollback: Scrollback::DEFAULT,
            shutdown: fixture.shutdown.clone(),
            tasks: fixture.tasks.clone(),
        });
        let CleanupObservation::Finished(report) =
            phase("actual runner final cleanup report", run.cleanup()).await
        else {
            panic!("runner lost")
        };
        assert_eq!(report.process(), CleanupFact::Confirmed);
        assert_eq!(report.auxiliary(), CleanupFact::Unconfirmed);
        assert!(!report.is_confirmed());
        assert!(directory.is_dir());
    })
    .await;
}
