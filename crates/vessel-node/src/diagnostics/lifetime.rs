//! Real product source jobs retain admission after caller abort and deadline.
use super::{
    deadline::{self, phase},
    *,
};
use crate::{Config, Node};
use std::{future::Future as _, task::Poll};
use vessel_harness::HarnessEnv;
use vessel_proto::diagnostics::{
    DiagnosticRefusal, DiagnosticSection, DiagnosticUnavailable, MAX_DIAGNOSTIC_COLLECTIONS,
};
use vessel_proto::{ErrorCode, ErrorDetail};
use vessel_store::DataDir;
const BLOCKING_THREADS: usize = 1;
#[test]
fn actual_queued_sources_survive_abort_and_deadline_without_releasing_capacity() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(BLOCKING_THREADS)
        .build()
        .unwrap();
    runtime.block_on(deadline::run(async {
        let temp = tempfile::tempdir().unwrap();
        let mut config = Config::new(
            DataDir::new(temp.path().join("data")),
            HarnessEnv::new(temp.path().join("user")),
        );
        config.bind = None;
        config.harness_lookup = Lookup::new(Vec::<std::path::PathBuf>::new());
        let node = phase("start queued source fixture", Node::start(config))
            .await
            .unwrap();
        let state = &node.secure.service.0.diagnostics;
        let lifecycle = &node.secure.service.0.lifecycle;
        let (ready, registered) = tokio::sync::oneshot::channel();
        let (release, hold) = std::sync::mpsc::channel();
        // Release sender belongs to this timed future: timeout/panic closes it
        // before custom runtime destruction, so the blocking recv unwinds too.
        let blocker = tokio::task::spawn_blocking(move || {
            ready.send(()).unwrap();
            let _released_or_closed = hold.recv();
        });
        phase("occupy the actual blocking pool", registered)
            .await
            .unwrap();
        let mut cancelled = Box::pin(state.collect(lifecycle));
        assert!(
            std::future::poll_fn(|cx| Poll::Ready(cancelled.as_mut().poll(cx).is_pending())).await
        );
        drop(cancelled);
        for _ in 1..MAX_DIAGNOSTIC_COLLECTIONS {
            let report = phase(
                "actual collector response deadline",
                state.collect(lifecycle),
            )
            .await
            .unwrap();
            assert!(matches!(
                report.fields().agents,
                DiagnosticSection::Unavailable {
                    reason: DiagnosticUnavailable::TimedOut
                }
            ));
            assert!(matches!(
                report.fields().store,
                DiagnosticSection::Unavailable {
                    reason: DiagnosticUnavailable::TimedOut
                }
            ));
            assert!(matches!(
                report.fields().recent_failures,
                DiagnosticSection::Available { .. }
            ));
        }
        let refused = phase(
            "capacity retained by actual source jobs",
            state.collect(lifecycle),
        )
        .await
        .unwrap_err();
        assert_eq!(refused.code, ErrorCode::Unavailable);
        assert_eq!(
            refused.detail,
            Some(ErrorDetail::Diagnostics {
                reason: DiagnosticRefusal::Busy
            })
        );
        release.send(()).unwrap();
        phase("release blocking pool", blocker).await.unwrap();
        phase("shutdown waits for queued real source leases", node.stop())
            .await
            .unwrap();
    }));
}
