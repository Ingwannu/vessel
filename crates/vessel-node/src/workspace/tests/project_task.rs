//! Actual task failures through the enclosing error, full chain and workspace warning.
use super::WorkspaceError;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_poisoned| std::io::Error::other("test warning capture was poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn assert_sealed(error: &WorkspaceError, canary: &str, class: &str) {
    let outer = format!("{error:?} {error}");
    assert!(outer.contains(class));
    assert!(!outer.contains(canary));
    let mut source: Option<&dyn std::error::Error> = Some(error);
    let mut depth = 0usize;
    while let Some(part) = source {
        assert!(!format!("{part:?} {part}").contains(canary));
        source = part.source();
        depth = depth.checked_add(1).unwrap();
    }
    assert_eq!(
        depth, 2,
        "the concrete panic payload stays behind the sealed cause"
    );
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_ansi(false)
        .without_time()
        .with_writer(move || writer.clone())
        .finish();
    let wire = tracing::subscriber::with_default(subscriber, || error.wire());
    assert_eq!(wire.code, vessel_proto::ErrorCode::Internal);
    assert!(!format!("{wire:?} {wire}").contains(canary));
    let warning = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(warning.contains("workspace call failed"));
    assert!(warning.contains(class));
    assert!(!warning.contains(canary));
}

#[tokio::test]
async fn project_admission_static_panic_is_sealed_through_warning_and_complete_chain() {
    let cause = tokio::spawn(async {
        panic!("STATIC_PROJECT_PANIC_CANARY");
    })
    .await
    .unwrap_err();
    assert_sealed(
        &WorkspaceError::project_task(cause),
        "STATIC_PROJECT_PANIC_CANARY",
        "Panicked",
    );
}

#[tokio::test]
async fn project_admission_owned_panic_is_sealed_through_warning_and_complete_chain() {
    let cause = tokio::spawn(async {
        std::panic::panic_any(String::from("OWNED_PROJECT_PANIC_CANARY"));
    })
    .await
    .unwrap_err();
    assert_sealed(
        &WorkspaceError::project_task(cause),
        "OWNED_PROJECT_PANIC_CANARY",
        "Panicked",
    );
}

#[tokio::test]
async fn project_admission_cancelled_task_has_only_its_input_free_class() {
    let task = tokio::spawn(std::future::pending::<()>());
    task.abort();
    let cause = task.await.unwrap_err();
    assert!(cause.is_cancelled());
    assert_sealed(
        &WorkspaceError::project_task(cause),
        "CANCELLED_PROJECT_PANIC_CANARY",
        "Cancelled",
    );
}
