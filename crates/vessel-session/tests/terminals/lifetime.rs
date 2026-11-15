//! Handles carry the actual current run and never revive a replaced run.
use vessel_session::terminals::Scrollback;

use super::support::{Fixture, type_in, wait_for_count, wait_for_output, within};

#[tokio::test]
async fn a_restarted_service_replaces_its_observation_and_stop_clears_it() {
    within(async {
        let fixture = Fixture::new();
        let service = fixture.script("echo service-up; exec sleep 300", true, || true);
        wait_for_output(&service, "service-up").await;
        let first = service.process_lifetime().unwrap();
        assert_eq!(Some(first.pid().get()), service.pid());
        service.restart().await.unwrap();
        wait_for_count(&service, "service-up", 2).await;
        first.ended().await;
        let second = service.process_lifetime().unwrap();
        assert!(!first.same_run(&second));
        assert!(!first.is_live());
        assert!(second.is_live());
        service.kill();
        service.ended().await;
        second.ended().await;
        assert!(service.process_lifetime().is_none());
    })
    .await;
}

#[tokio::test]
async fn a_shell_handle_observes_its_pty_and_clears_after_exit() {
    within(async {
        let fixture = Fixture::new();
        let shell = fixture.shell(Scrollback::DEFAULT).await;
        let observed = shell.process_lifetime().unwrap();
        assert_eq!(Some(observed.pid().get()), shell.pid());
        shell.send(type_in("exit 0\n")).await.unwrap();
        shell.ended().await;
        observed.ended().await;
        assert!(!observed.is_live());
        assert!(shell.process_lifetime().is_none());
    })
    .await;
}
