//! Real store reads, explicit blocking gates, and synthetic values only.
use super::*;
use std::{
    sync::{Mutex, mpsc},
    time::Duration,
};
use tokio::sync::Notify;
use vessel_store::DataDir;

const TEST_LIMIT: Duration = Duration::from_secs(60);
pub(super) struct Hook {
    entered: Notify,
    release: Mutex<mpsc::Receiver<()>>,
}
impl Hook {
    pub(super) fn wait(&self) -> Result<(), ReadError> {
        self.entered.notify_one();
        self.release
            .lock()
            .map_err(|_poison| ReadError::HookPoison)?
            .recv_timeout(TEST_LIMIT)
            .map_err(ReadError::Hook)
    }
}
fn fixture() -> (tempfile::TempDir, Reader, SecretName) {
    let temp = tempfile::tempdir().unwrap();
    let files = FleetFiles::new(&DataDir::new(temp.path().join("data")));
    let reader = Reader::new(files);
    (temp, reader, "TRANSCRIPTION".parse().unwrap())
}

#[tokio::test]
async fn values_are_fresh_and_presence_never_returns_the_selected_name_or_value() {
    tokio::time::timeout(TEST_LIMIT, async {
        let (_temp, reader, name) = fixture();
        assert_eq!(reader.presence(None).await.unwrap(), KeyPresence::Absent);
        assert_eq!(
            reader.presence(Some(&name)).await.unwrap(),
            KeyPresence::Absent
        );
        crate::fleet::set_secret(&reader.files, &name, "synthetic-first").unwrap();
        let first = reader
            .resolve(&name, Admission::Presence)
            .await
            .unwrap()
            .unwrap();
        assert!(first.expose() == "synthetic-first");
        assert_eq!(
            reader.presence(Some(&name)).await.unwrap(),
            KeyPresence::Present
        );
        crate::fleet::set_secret(&reader.files, &name, "synthetic-second").unwrap();
        let second = reader
            .resolve(&name, Admission::Presence)
            .await
            .unwrap()
            .unwrap();
        assert!(second.expose() == "synthetic-second");
        assert!(crate::fleet::delete_secret(&reader.files, &name).unwrap());
        assert_eq!(
            reader.presence(Some(&name)).await.unwrap(),
            KeyPresence::Absent
        );
        crate::fleet::set_secret(&reader.files, &name, "   ").unwrap();
        assert_eq!(
            reader.presence(Some(&name)).await.unwrap(),
            KeyPresence::Absent
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_preparation_neither_waits_for_nor_enters_the_read_gate() {
    let (_temp, reader, name) = fixture();
    let held = Arc::clone(&reader.slots)
        .acquire_many_owned(u32::try_from(NODE_STREAMS).unwrap())
        .await
        .unwrap();
    let stopped = CancellationToken::new();
    let lease = crate::dictation::quota::tests::lease();
    let weak = Arc::downgrade(&lease);
    let mut reading = Box::pin(reader.resolve(
        &name,
        Admission::Preparing {
            lease,
            stopped: &stopped,
        },
    ));
    assert!(futures_util::poll!(reading.as_mut()).is_pending());
    stopped.cancel();
    // Every permit is still held: success here cannot depend on a read ending.
    assert!(matches!(
        futures_util::poll!(reading.as_mut()),
        std::task::Poll::Ready(Err(ReadError::Cancelled))
    ));
    drop(reading);
    assert!(weak.upgrade().is_none());
    assert_eq!(reader.slots.available_permits(), 0);
    drop(held);
    // Already cancelled admission also wins when capacity is immediately ready.
    assert!(matches!(
        reader
            .resolve(
                &name,
                Admission::Preparing {
                    lease: crate::dictation::quota::tests::lease(),
                    stopped: &stopped,
                },
            )
            .await,
        Err(ReadError::Cancelled)
    ));
    assert_eq!(reader.slots.available_permits(), NODE_STREAMS);
}

#[tokio::test]
async fn dropping_a_waiter_retains_read_capacity_and_stream_lease_until_actual_io_ends() {
    tokio::time::timeout(TEST_LIMIT, async {
        let (_temp, mut reader, name) = fixture();
        let (release, held) = mpsc::sync_channel(1);
        let hook = Arc::new(Hook {
            entered: Notify::new(),
            release: Mutex::new(held),
        });
        reader.hook = Some(Arc::clone(&hook));
        let lease = crate::dictation::quota::tests::lease();
        let weak = Arc::downgrade(&lease);
        let running = reader.clone();
        let task = tokio::spawn(async move {
            running
                .resolve(
                    &name,
                    Admission::Preparing {
                        lease,
                        stopped: &CancellationToken::new(),
                    },
                )
                .await
        });
        hook.entered.notified().await;
        assert_eq!(reader.slots.available_permits(), NODE_STREAMS - 1);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(weak.upgrade().is_some());
        // Presence is independent of stream quota and does no I/O for no name.
        assert_eq!(reader.presence(None).await.unwrap(), KeyPresence::Absent);
        assert_eq!(reader.slots.available_permits(), NODE_STREAMS - 1);
        release.send(()).unwrap();
        let _all_reads = Arc::clone(&reader.slots)
            .acquire_many_owned(u32::try_from(NODE_STREAMS).unwrap())
            .await
            .unwrap();
        assert!(weak.upgrade().is_none());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn corrupt_store_diagnostics_keep_the_cause_without_names_paths_or_payload() {
    tokio::time::timeout(TEST_LIMIT, async {
        let (_temp, reader, name) = fixture();
        std::fs::create_dir_all(reader.files.secrets().parent().unwrap()).unwrap();
        std::fs::write(reader.files.secrets(), "TRANSCRIPTION = [synthetic-hidden").unwrap();
        let error = reader
            .resolve(&name, Admission::Presence)
            .await
            .unwrap_err();
        assert!(matches!(error, ReadError::Store(_)));
        assert!(std::error::Error::source(&error).is_some());
        for rendered in [format!("{error}"), format!("{error:?}")] {
            assert!(!rendered.contains("synthetic-hidden"));
            assert!(!rendered.contains(name.as_str()));
            assert!(!rendered.contains(&reader.files.secrets().display().to_string()));
        }
    })
    .await
    .unwrap();
}
