//! Actual runner revocation, independent of numeric PID reuse.
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use vessel_proc::{Signal, Spawned, spawn};

use super::support::{drain, fake_tui, read_until, within};

#[tokio::test]
async fn observing_a_waited_exit_revokes_current_and_late_observers() {
    within(async {
        let Spawned {
            mut child,
            mut stdin,
            stdout,
            ..
        } = spawn(&fake_tui()).unwrap();
        let observed = child.lifetime();
        assert_eq!(observed.pid().get(), child.pid());
        let mut lines = BufReader::new(stdout).lines();
        let _ready = read_until(&mut lines, "ready").await;
        assert!(observed.is_live());
        stdin.write_all(b"exit 0\n").await.unwrap();
        assert_eq!(child.wait().await.unwrap(), Some(0));
        observed.ended().await;
        assert!(!observed.is_live());
        assert!(!child.lifetime().is_live());
        let _remaining = drain(lines).await;
    })
    .await;
}

#[tokio::test]
async fn a_signal_revokes_before_wait_and_a_cancelled_wait_does_not() {
    within(async {
        let Spawned {
            mut child,
            stdin,
            stdout,
            ..
        } = spawn(&fake_tui()).unwrap();
        let observed = child.lifetime();
        let mut lines = BufReader::new(stdout).lines();
        let _ready = read_until(&mut lines, "ready").await;
        let mut waiting = Box::pin(child.wait());
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                waiting.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        drop(waiting);
        assert!(observed.is_live());
        child.signal(Signal::Terminate).unwrap();
        assert!(!observed.is_live());
        observed.ended().await;
        let _status = child.wait().await.unwrap();
        let _remaining = drain(lines).await;
        drop(stdin);
    })
    .await;
}

#[tokio::test]
async fn dropping_the_owner_revokes_without_an_exit_waiter() {
    within(async {
        let Spawned {
            child,
            stdin,
            stdout,
            ..
        } = spawn(&fake_tui()).unwrap();
        let observed = child.lifetime();
        let mut lines = BufReader::new(stdout).lines();
        let _ready = read_until(&mut lines, "ready").await;
        drop(child);
        assert!(!observed.is_live());
        observed.ended().await;
        let _remaining = drain(lines).await;
        drop(stdin);
    })
    .await;
}
