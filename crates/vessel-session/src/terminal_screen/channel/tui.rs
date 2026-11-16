//! An actual fake TUI feeds the common screen; its final projection survives ring removal.
use super::*;
use crate::handle::Input;
use crate::terminal_screen::ScreenAuthority;
use crate::{DEFAULT_DURABLE_CAPACITY, DEFAULT_HANDSHAKE_TIMEOUT, RuntimeConfig, SessionRuntime};
use bytes::Bytes;
use std::{future::Future, num::NonZeroUsize, path::Path, sync::Arc};
use tokio_util::sync::{CancellationToken, DropGuard};
use vessel_harness::fakes::{Role, install};
use vessel_proto::{
    EventRecord, EventSeq, HarnessKind, SessionEvent, SessionInput, SessionMode, SessionSpec,
    TerminalBytes, TerminalId, TerminalInput,
};

const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE_WAIT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn tui_screen_has_the_session_identity_and_final_cells_after_the_ring_is_gone() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let (runtime, _cancel) = runtime(root.path());
        let created = phase("TUI start", runtime.create(spec(root.path())))
            .await
            .unwrap();
        let handle = created.handle;
        let id = handle.id();
        let endpoint = match &handle.0.input {
            Input::Terminal { screen, .. } => screen.clone(),
            Input::Structured(_) => panic!("fake TUI used a structured route"),
        };
        let mut durable = created.durable;
        until_text(&mut durable, "ready").await;
        phase(
            "TUI resize admission",
            handle.send(SessionInput::Terminal {
                input: TerminalInput::Resize {
                    size: TerminalSize { rows: 2, cols: 32 },
                },
            }),
        )
        .await
        .unwrap();
        keys(&handle, "shared-screen-proof\r").await;
        until_text(&mut durable, "echo: shared-screen-proof").await;
        keys(&handle, "exit 0\r").await;
        phase("TUI durable close", async {
            while durable.recv().await.is_some() {}
        })
        .await;
        assert!(matches!(
            handle.follow(EventSeq::ZERO),
            Err(crate::FollowError::Gap { oldest_cached }) if oldest_cached > EventSeq::ZERO
        ));
        assert_eq!(
            handle.info().status,
            vessel_proto::SessionStatus::Exited { code: Some(0) }
        );
        phase("TUI runtime drain", runtime.shutdown()).await;
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

fn runtime(root: &Path) -> (SessionRuntime, DropGuard) {
    with_capacity(root, DEFAULT_DURABLE_CAPACITY)
}
fn with_capacity(root: &Path, durable_capacity: NonZeroUsize) -> (SessionRuntime, DropGuard) {
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
    // Match the runtime diagnostics fixture: assertion failure must signal the
    // real actor before this fixture yields; dropping a handle is not cleanup.
    let stop = CancellationToken::new();
    let cancel = stop.clone().drop_guard();
    let runtime = SessionRuntime::new(
        RuntimeConfig {
            harness_lookup: vessel_harness::Lookup::new([bin]),
            harness_env: vessel_harness::HarnessEnv::new(root.join("user")),
            ring_capacity: NonZeroUsize::new(2).unwrap(),
            durable_capacity,
            agent_handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            park_after: None,
            private_dir: None,
        },
        stop,
    );
    (runtime, cancel)
}

async fn keys(handle: &crate::SessionHandle, text: &'static str) {
    phase(
        "TUI key admission",
        handle.send(SessionInput::Terminal {
            input: TerminalInput::Data {
                data: TerminalBytes::new(Bytes::from_static(text.as_bytes())),
            },
        }),
    )
    .await
    .unwrap();
}

async fn until_text(durable: &mut mpsc::Receiver<Arc<EventRecord>>, expected: &str) {
    phase(expected, async {
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
    .await;
}

async fn phase<F: Future>(name: &str, action: F) -> F::Output {
    tokio::time::timeout(PHASE_WAIT, action)
        .await
        .unwrap_or_else(|_| panic!("TUI phase timed out: {name}"))
}

#[cfg(unix)]
#[tokio::test]
async fn a_full_screen_inbox_cannot_run_ahead_of_already_ready_tui_output() {
    vessel_testkit::within(
        SAFETY_NET,
        Box::pin(async {
            use tokio::io::AsyncReadExt as _;
            let root = tempfile::tempdir().unwrap();
            let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .await
                .unwrap();
            let port = listener.local_addr().unwrap().port();
            let (runtime, _cancel) = with_capacity(root.path(), NonZeroUsize::new(1).unwrap());
            let mut spec = spec(root.path());
            spec.mode = SessionMode::Terminal {
                args: vec!["--held-input".to_owned(), port.to_string()],
                size: TerminalSize { rows: 3, cols: 40 },
            };
            let created = phase("held TUI start", runtime.create(spec)).await.unwrap();
            let handle = created.handle;
            let endpoint = match &handle.0.input {
                Input::Terminal { screen, .. } => screen.clone(),
                Input::Structured(_) => panic!("fake TUI used structured routing"),
            };
            // The actor cannot leave startup until its one-slot durable channel
            // is drained. The child flushes real output before this separate ack.
            let (mut ready, _) = phase("held TUI acknowledgement", listener.accept())
                .await
                .unwrap();
            let mut control_port = [0_u8; 2];
            assert_eq!(
                phase("held TUI control port", ready.read_exact(&mut control_port))
                    .await
                    .unwrap(),
                control_port.len()
            );
            assert_ne!(u16::from_be_bytes(control_port), 0);
            for _ in 1..crate::terminal_screen::CAPTURE_QUEUE {
                endpoint.resize(TerminalSize { rows: 3, cols: 40 }).unwrap();
            }
            let (reply, mut barrier) = oneshot::channel();
            // A private last-command acknowledgement observes scheduling only;
            // text capture still rejects the session target when eventually handled.
            endpoint
                .commands
                .try_send(CaptureCommand::Capture {
                    options: CaptureOptions::default(),
                    reply,
                })
                .unwrap();
            let mut durable = created.durable;
            phase("held TUI first output", async {
                loop {
                    let record = durable.recv().await.expect("TUI ended before ready output");
                    if matches!(record.event, SessionEvent::Terminal(_)) {
                        break;
                    }
                }
            })
            .await;
            assert!(matches!(
                barrier.try_recv(),
                Err(oneshot::error::TryRecvError::Empty)
            ));
            handle.kill();
            phase("held TUI durable close", async {
                while durable.recv().await.is_some() {}
            })
            .await;
            phase("held TUI runtime drain", runtime.shutdown()).await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn final_owner_is_sealed_and_second_finish_does_not_replace_the_retained_cut() {
    vessel_testkit::within(SAFETY_NET, async {
        let (endpoint, mut authority) = ScreenAuthority::channel(
            TerminalAddress::Standalone(TerminalId::random()),
            TerminalSize { rows: 2, cols: 8 },
        );
        authority.output(b"final");
        authority.finish();
        let first = endpoint.final_screen.borrow().clone().unwrap();
        authority.output(b"must not feed after EOF");
        authority.resize(TerminalSize { rows: 3, cols: 12 });
        authority.finish();
        let second = endpoint.final_screen.borrow().clone().unwrap();
        assert!(Arc::ptr_eq(&first.owner, &second.owner));
        assert_eq!(
            phase(
                "sealed final capture",
                endpoint.capture(CaptureOptions::default())
            )
            .await
            .unwrap()
            .lines,
            ["final", ""]
        );
        assert_eq!(second.owner.snapshot().unwrap().size().cols(), 8);
        assert_eq!(second.owner.snapshot().unwrap().size().rows(), 2);
    })
    .await
    .unwrap();
}
