//! An actual fake TUI feeds the common screen; its final projection survives ring removal.
use super::*;
use crate::handle::Input;
use crate::{DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, RuntimeConfig, SessionRuntime};
use bytes::Bytes;
use std::{num::NonZeroUsize, path::Path, sync::Arc};
use tokio_util::sync::CancellationToken;
use vessel_harness::fakes::{Role, install};
use vessel_proto::{
    EventRecord, EventSeq, HarnessKind, SessionEvent, SessionInput, SessionMode, SessionSpec,
    TerminalBytes, TerminalInput,
};

#[tokio::test]
async fn tui_screen_has_the_session_identity_and_final_cells_after_the_ring_is_gone() {
    vessel_testkit::within(Duration::from_secs(60), async {
        let root = tempfile::tempdir().unwrap();
        let runtime = runtime(root.path());
        let created = runtime.create(spec(root.path())).await.unwrap();
        let handle = created.handle;
        let id = handle.id();
        let endpoint = match &handle.0.input {
            Input::Terminal { screen, .. } => screen.clone(),
            Input::Structured(_) => panic!("fake TUI used a structured route"),
        };
        let mut durable = created.durable;
        until_text(&mut durable, "ready").await;
        handle
            .send(SessionInput::Terminal {
                input: TerminalInput::Resize {
                    size: TerminalSize { rows: 2, cols: 32 },
                },
            })
            .await
            .unwrap();
        keys(&handle, "shared-screen-proof\r").await;
        until_text(&mut durable, "echo: shared-screen-proof").await;
        keys(&handle, "exit 0\r").await;
        tokio::time::timeout(Duration::from_secs(10), async {
            while durable.recv().await.is_some() {}
        })
        .await
        .expect("fake TUI did not close its durable output");
        assert!(matches!(
            handle.follow(EventSeq::ZERO),
            Err(crate::FollowError::Gap { oldest_cached }) if oldest_cached > EventSeq::ZERO
        ));
        assert_eq!(
            handle.info().status,
            vessel_proto::SessionStatus::Exited { code: Some(0) }
        );
        runtime.shutdown().await;
        let removed = runtime.remove(id).unwrap();
        assert_eq!(removed.id, id);
        drop(handle);
        let retained = endpoint.final_screen.borrow().clone().unwrap();
        assert_eq!(retained.target, TerminalAddress::Session(id));
        let snapshot = retained.owner.snapshot().unwrap();
        assert_eq!(snapshot.size().cols(), 32);
        assert_eq!(snapshot.size().rows(), 2);
        let rows = snapshot
            .capture(vessel_terminal_screen::CaptureRange::default())
            .unwrap();
        assert!(
            rows.lines
                .iter()
                .any(|line| line.contains("echo: shared-screen-proof"))
        );
    })
    .await
    .unwrap();
}

fn spec(root: &Path) -> SessionSpec {
    SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Terminal {
            args: Vec::new(),
            size: TerminalSize { rows: 3, cols: 40 },
        },
    )
    .with_cwd(root.to_owned())
}

fn runtime(root: &Path) -> SessionRuntime {
    let bin = root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    drop(
        install(
            &bin,
            HarnessKind::ClaudeCode,
            Role::Terminal,
            &vessel_testkit::Fake::Tui.path().unwrap(),
        )
        .unwrap(),
    );
    SessionRuntime::new(
        RuntimeConfig {
            harness_lookup: vessel_harness::Lookup::new([bin]),
            harness_env: vessel_harness::HarnessEnv::new(root.join("user")),
            ring_capacity: NonZeroUsize::new(2).unwrap(),
            durable_capacity: DEFAULT_DURABLE_CAPACITY,
            agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: None,
            private_dir: None,
        },
        CancellationToken::new(),
    )
}

async fn keys(handle: &crate::SessionHandle, text: &'static str) {
    handle
        .send(SessionInput::Terminal {
            input: TerminalInput::Data {
                data: TerminalBytes::new(Bytes::from_static(text.as_bytes())),
            },
        })
        .await
        .unwrap();
}

async fn until_text(durable: &mut mpsc::Receiver<Arc<EventRecord>>, expected: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut text = String::new();
        while let Some(record) = durable.recv().await {
            if let SessionEvent::Terminal(output) = &record.event {
                text.push_str(std::str::from_utf8(output.data.bytes()).unwrap());
                if text.contains(expected) {
                    return;
                }
            }
        }
        panic!("fake TUI ended without expected text: {expected}");
    })
    .await
    .expect("fake TUI did not emit expected text");
}
