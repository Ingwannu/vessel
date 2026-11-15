//! The blocking waiter ends observations even without polling `PtyProcess::wait`.
use bytes::Bytes;
use vessel_pty::PtyProcess;

use super::SIZE;
use super::support::{Screen, fake_tui, within};

#[tokio::test]
async fn a_natural_exit_revokes_before_the_async_wait_is_polled() {
    within(async {
        let (mut process, output) = PtyProcess::spawn(&fake_tui(), SIZE).unwrap();
        let observed = process.lifetime();
        let mut screen = Screen::new(output);
        let _ready = screen.expect(0, "ready").await;
        assert!(observed.is_live());
        assert_eq!(observed.pid().get(), process.pid());
        process
            .write(Bytes::from_static(b"exit 0\r"))
            .await
            .unwrap();
        observed.ended().await;
        assert!(!process.lifetime().is_live());
        assert_eq!(process.wait().await, Some(0));
        screen.drain().await;
    })
    .await;
}

#[tokio::test]
async fn stopping_a_pty_revokes_before_waiting_for_its_exit() {
    within(async {
        let (mut process, output) = PtyProcess::spawn(&fake_tui(), SIZE).unwrap();
        let observed = process.lifetime();
        let mut screen = Screen::new(output);
        let _ready = screen.expect(0, "ready").await;
        process.kill().unwrap();
        assert!(!observed.is_live());
        observed.ended().await;
        let _status = process.wait().await;
        screen.drain().await;
    })
    .await;
}

#[tokio::test]
async fn a_dropped_pty_revokes_its_retained_observers() {
    within(async {
        let (process, output) = PtyProcess::spawn(&fake_tui(), SIZE).unwrap();
        let observed = process.lifetime();
        let mut screen = Screen::new(output);
        let _ready = screen.expect(0, "ready").await;
        drop(process);
        assert!(!observed.is_live());
        observed.ended().await;
        screen.drain().await;
    })
    .await;
}
