//! Real RPC timeout/disconnect leaves queued source jobs owning their leases.
//! The separate direct-future fixture covers abort before the response deadline.
use super::{
    authority,
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
fn diagnostics_wire_partial_deadlines_and_disconnect_do_not_release_queued_sources() {
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
        let node = phase("start wire lease fixture", Node::start(config))
            .await
            .unwrap();
        // Authenticate/handshake while the actual blocking pool is available.
        let transport = authority::transport(&node).await;
        let other = authority::transport(&node).await;
        let (ready, registered) = tokio::sync::oneshot::channel();
        let (release, hold) = std::sync::mpsc::channel();
        // The timed future owns release: unwinding must unblock runtime teardown.
        let blocker = tokio::task::spawn_blocking(move || {
            ready.send(()).unwrap();
            let _released_or_closed = hold.recv();
        });
        phase("occupy real wire source pool", registered)
            .await
            .unwrap();
        for _ in 0..MAX_DIAGNOSTIC_COLLECTIONS {
            let report = phase(
                "real RPC partial deadline",
                transport.client().diagnostics(),
            )
            .await
            .unwrap();
            assert_eq!(report.fields().node.id, node.info().id);
            for timed_out in [
                matches!(
                    report.fields().store,
                    DiagnosticSection::Unavailable {
                        reason: DiagnosticUnavailable::TimedOut
                    }
                ),
                matches!(
                    report.fields().agents,
                    DiagnosticSection::Unavailable {
                        reason: DiagnosticUnavailable::TimedOut
                    }
                ),
            ] {
                assert!(timed_out);
            }
            assert!(matches!(
                report.fields().listeners,
                DiagnosticSection::Available { .. }
            ));
        }
        transport.close().await;
        let error = phase(
            "new connection still sees retained source capacity",
            other.client().diagnostics(),
        )
        .await
        .unwrap_err();
        let vessel_client::DiagnosticError::Call(vessel_client::CallError::Remote(error)) = error
        else {
            panic!("retained source admission must return a typed remote refusal")
        };
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert_eq!(
            error.detail,
            Some(ErrorDetail::Diagnostics {
                reason: DiagnosticRefusal::Busy
            })
        );
        other.close().await;
        let mut stopping = Box::pin(node.stop());
        assert!(
            std::future::poll_fn(|cx| Poll::Ready(stopping.as_mut().poll(cx).is_pending())).await
        );
        release.send(()).unwrap();
        phase("release source pool", blocker).await.unwrap();
        phase("shutdown joins actual queued jobs", stopping)
            .await
            .unwrap();
    }));
}
