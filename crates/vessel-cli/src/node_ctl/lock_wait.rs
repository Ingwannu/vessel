//! Wait on the OS lock's release; never poll it or mutate the owner record.
//! A dedicated thread is intentional: Tokio cannot cancel a blocking lock
//! acquisition and waits for blocking-pool threads during runtime shutdown.
//! This CLI's detached thread instead ends with the process on a deadline.

use std::fs::File;

use super::local_read::{Kind, Pending};
use vessel_node::DataDir;

use crate::error::CliError;

#[cfg(test)]
const WAITER_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

/// One OS-lock completion survives timeout cancellation between exit phases.
pub(super) struct Released {
    state: State,
}

enum State {
    Waiting(Pending<Result<(), std::io::Error>>),
    Released,
}

impl Released {
    /// Start one waiter; it never writes the lock or the owner record.
    ///
    /// # Errors
    /// The operating system cannot create the waiter thread.
    pub(super) fn subscribe(dir: &DataDir) -> Result<Self, CliError> {
        let path = dir.lock_path();
        let pending = Pending::spawn(Kind::LockRelease, move || {
            File::options()
                .read(true)
                .write(true)
                .open(path)
                .and_then(|file| file.lock().and_then(|()| file.unlock()))
        })?;
        Ok(Self {
            state: State::Waiting(pending),
        })
    }

    /// Borrowing the receiver makes a timed-out wait cancellation-safe.
    ///
    /// # Errors
    /// The lock could not be opened/acquired, or the waiter exited unexpectedly.
    pub(super) async fn wait(&mut self) -> Result<(), CliError> {
        match &mut self.state {
            State::Released => return Ok(()),
            State::Waiting(pending) => pending.wait().await.map_err(CliError::LockWaiter)??,
        }
        self.state = State::Released;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_store::{NodeLock, NodeOwner};

    async fn within_budget<F: Future>(future: F) -> F::Output {
        vessel_testkit::within(WAITER_TEST_BUDGET, future)
            .await
            .unwrap_or_else(|source| {
                panic!("waiter budget {WAITER_TEST_BUDGET:?} expired: {source}")
            })
    }

    #[tokio::test]
    async fn the_waiter_is_pending_until_the_existing_lock_releases() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let owner = NodeOwner {
            pid: std::process::id(),
            executable: std::env::current_exe().unwrap(),
            version: "test".to_owned(),
            started_at: chrono::Utc::now(),
        };
        let lock = NodeLock::acquire(&dir, &owner).unwrap();
        let before = std::fs::read(dir.owner_path()).unwrap();
        let mut event = Released::subscribe(&dir).unwrap();
        // This is the command's cancellation deadline, not synchronization
        // by sleep: the parent deliberately retains the lock throughout it.
        assert!(
            tokio::time::timeout(WAITER_TEST_BUDGET, event.wait())
                .await
                .is_err(),
            "waiter completed during its {WAITER_TEST_BUDGET:?} cancellation budget while the lock was held"
        );
        drop(lock);
        within_budget(event.wait()).await.unwrap();
        within_budget(event.wait()).await.unwrap();
        assert!(NodeLock::holder(&dir).unwrap().is_none());
        assert!(
            before == std::fs::read(dir.owner_path()).unwrap(),
            "owner record changed"
        );
    }
    #[tokio::test]
    async fn a_missing_lock_keeps_the_io_error_as_its_source() {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path().join("missing"));
        let mut event = Released::subscribe(&dir).unwrap();
        let error = within_budget(event.wait()).await.unwrap_err();
        assert!(
            matches!(error, CliError::Io(source) if source.kind() == std::io::ErrorKind::NotFound)
        );
    }
    #[test]
    fn an_expired_wait_does_not_keep_the_command_process_alive() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        if let Some(path) = std::env::var_os("VESSEL_LOCK_EXIT_TEST_DIR") {
            runtime.block_on(async {
                let mut event = Released::subscribe(&DataDir::new(path)).unwrap();
                assert!(
                    tokio::time::timeout(std::time::Duration::ZERO, event.wait())
                        .await
                        .is_err()
                );
            });
            drop(runtime);
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let owner = NodeOwner {
            pid: std::process::id(),
            executable: std::env::current_exe().unwrap(),
            version: "test".to_owned(),
            started_at: chrono::Utc::now(),
        };
        let lock = NodeLock::acquire(&dir, &owner).unwrap();
        runtime.block_on(async {
            let output = within_budget(async {
                tokio::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "node_ctl::lock_wait::tests::an_expired_wait_does_not_keep_the_command_process_alive"])
                    .env("VESSEL_LOCK_EXIT_TEST_DIR", dir.root())
                    .stdin(std::process::Stdio::null()).kill_on_drop(true).output().await.unwrap()
            }).await;
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        });
        // The child exited even though this process still held the lock.
        drop(lock);
    }
}
