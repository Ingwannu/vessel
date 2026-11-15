//! Public chains and the actual startup/owner log callsites, with fake canaries.
use super::*;
use crate::providers::configuration::{ConfigurationFailure, Prepared};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use vessel_store::DataDir;

const MARKER: &str = "provider-private-task-canary-ea4917";

fn safe_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut result = String::new();
    let mut current = Some(error);
    while let Some(error) = current {
        let rendered = format!("{error}\n{error:?}\n");
        assert!(!rendered.contains(MARKER));
        result.push_str(&rendered);
        current = error.source();
    }
    result
}

#[derive(Clone)]
struct Captured(Arc<Mutex<Vec<u8>>>);
impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn logs(operation: impl FnOnce()) -> String {
    let captured = Captured(Arc::default());
    let output = Arc::clone(&captured.0);
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || captured.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, operation);
    String::from_utf8(output.lock().unwrap().clone()).unwrap()
}

#[tokio::test]
async fn static_and_owned_task_panics_keep_the_actual_public_chain_and_owner_log_safe() {
    for owned in [false, true] {
        let raw = tokio::task::spawn_blocking::<_, ()>(move || {
            if owned {
                std::panic::panic_any(MARKER.to_owned());
            }
            panic!("provider-private-task-canary-ea4917");
        })
        .await
        .unwrap_err();
        assert!(raw.is_panic());
        assert!(format!("{raw:?}").contains(MARKER));
        let error = ConfigurationFailure::from(raw);
        assert!(safe_chain(&error).contains("panicked"));
        let ConfigurationFailure::Task(failure) = &error else {
            panic!("expected task failure");
        };
        let output = logs(|| report_owner(failure));
        assert!(output.contains("provider owner ended unexpectedly"));
        assert!(output.contains("panicked"));
        assert!(!output.contains(MARKER));
        let _rendered = safe_chain(&crate::NodeError::Providers(error));
    }
    // The process-wide panic hook is outside this error/log boundary contract.
}

#[tokio::test]
async fn actual_task_cancellation_retains_the_closed_outcome_without_a_payload_source() {
    let task = tokio::spawn(std::future::pending::<()>());
    task.abort();
    let raw = task.await.unwrap_err();
    assert!(raw.is_cancelled());
    let error = ConfigurationFailure::from(raw);
    assert!(safe_chain(&error).contains("cancelled"));
}

#[test]
fn real_provider_file_error_and_startup_recovery_log_do_not_quote_the_canary() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path());
    let bytes = format!(r#"{{"providers":{{"codex":{{"enabled":"{MARKER}"}}}}}}"#);
    std::fs::write(data.providers_path(), &bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            data.providers_path(),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
    let error = data.read_provider_overrides().unwrap_err();
    let error = ConfigurationFailure::from(error);
    assert!(safe_chain(&error).contains("Data at line 1 column"));
    let _rendered = safe_chain(&crate::NodeError::Providers(error));
    let output = logs(|| {
        let _prepared = Prepared::read(data.clone(), vec![]).unwrap();
    });
    assert!(output.contains("unreadable provider configuration preserved"));
    assert!(!output.contains(MARKER));
    assert!(!data.providers_path().exists());
    let copy = std::fs::read_dir(root.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(std::fs::read(copy).unwrap(), bytes.as_bytes());
}
