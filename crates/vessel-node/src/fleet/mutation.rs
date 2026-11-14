//! Non-queueing membership admission shared with the offline CLI. Its
//! lock is separate from materialization, so network pairing never holds
//! the apply lock. Dropping the reservation releases both admissions.
use super::files::{self, FileError, FleetFiles};
use std::fs::{File, OpenOptions, TryLockError};
use tokio::sync::OwnedSemaphorePermit;

#[derive(Debug)]
pub(super) struct Reservation {
    _lock: File,
    _permit: Option<OwnedSemaphorePermit>,
}

impl Reservation {
    pub(super) fn take(
        files: &FleetFiles,
        permit: Option<OwnedSemaphorePermit>,
    ) -> Result<Self, FileError> {
        files::create_private_dir(files.dir())
            .map_err(|error| FileError::io(files.dir(), error))?;
        let path = files.dir().join("membership.lock");
        let mut options = OpenOptions::new();
        let _same = options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            let _same = options.mode(0o600);
        }
        let lock = options
            .open(&path)
            .map_err(|error| FileError::io(&path, error))?;
        lock.try_lock().map_err(|error| {
            FileError::io(
                &path,
                match error {
                    TryLockError::WouldBlock => {
                        std::io::Error::from(std::io::ErrorKind::WouldBlock)
                    }
                    TryLockError::Error(error) => error,
                },
            )
        })?;
        Ok(Self {
            _lock: lock,
            _permit: permit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn cancellation_keeps_membership_admission_until_blocking_work_ends() {
        let temp = tempfile::tempdir().unwrap();
        let files = FleetFiles::new(&vessel_store::DataDir::new(temp.path().join("data")));
        let slots = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = Arc::clone(&slots).try_acquire_owned().unwrap();
        let reservation = Reservation::take(&files, Some(permit)).unwrap();
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::sync_channel(1);
        let (ended, finished) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                let _reservation = reservation;
                entered.send(()).unwrap();
                held.recv().unwrap();
                ended.send(()).unwrap();
            })
            .await
            .unwrap();
        });
        started.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(slots.available_permits(), 0);
        let refused = super::super::leave(&files).unwrap_err();
        assert!(
            matches!(refused, FileError::Io { source, .. } if source.kind() == std::io::ErrorKind::WouldBlock)
        );
        let refused = super::super::init(
            &files,
            &super::super::FleetInit {
                instructions: Some(temp.path().join("instructions")),
                skills: None,
                skip: Vec::new(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(refused, super::super::InitError::File(FileError::Io { source, .. }) if source.kind() == std::io::ErrorKind::WouldBlock)
        );
        release.send(()).unwrap();
        finished.await.unwrap();
        let permit = slots.acquire().await.unwrap();
        assert!(!super::super::leave(&files).unwrap());
        drop(permit);
    }
}
