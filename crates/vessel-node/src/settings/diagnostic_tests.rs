//! The real startup recovery logger must not re-expose a private serde cause.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use vessel_store::DataDir;

use super::set_aside;

const MARKER: &str = "startup-private-marker-fb4a1d20";

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

#[test]
fn startup_recovery_logs_safe_parse_diagnostics_on_move_success_and_failure() {
    for move_fails in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path());
        let original = format!(r#"{{"drivers":"{MARKER}"}}"#);
        std::fs::write(data.settings_path(), &original).unwrap();
        let error = data.read_settings().unwrap_err();
        if move_fails {
            // A deterministic missing source exercises the real failure log;
            // permissions would behave differently for privileged test users.
            std::fs::remove_file(data.settings_path()).unwrap();
        }
        let captured = Captured(Arc::default());
        let output = Arc::clone(&captured.0);
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || captured.clone())
            .finish();
        let kept = tracing::subscriber::with_default(subscriber, || set_aside(&data, &error));
        assert_eq!(kept.is_some(), move_fails);
        let text = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(text.contains("settings.json could not be used"));
        assert!(text.contains("invalid field at $.<field>"));
        assert!(!text.contains(MARKER));
        if !move_fails {
            let aside = std::fs::read_dir(root.path())
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(std::fs::read(aside).unwrap(), original.as_bytes());
        }
    }
}
