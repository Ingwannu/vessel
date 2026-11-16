//! Real RPC cancellation, timeout and disconnect retain queued source leases.
//! The test observer identifies actual guard entry and server-side future drop.
use super::{
    authority,
    deadline::{self, phase},
    wire_observer::{self, Stage},
};
use std::{future::Future as _, task::Poll};
use vessel_proto::diagnostics::{
    DiagnosticRefusal, DiagnosticSection, DiagnosticUnavailable, MAX_DIAGNOSTIC_COLLECTIONS,
};
use vessel_proto::{ErrorCode, ErrorDetail};
const BLOCKING_THREADS: usize = 1;
#[test]
fn diagnostics_wire_partial_deadlines_and_disconnect_do_not_release_queued_sources() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(BLOCKING_THREADS)
        .build()
        .unwrap();
    runtime.block_on(deadline::run(async {
        let (_temp, node) = authority::node().await;
        // Authenticate/handshake while the actual blocking pool is available.
        let mut transport = wire_observer::transport(&node).await;
        let other = authority::transport(&node).await;
        // The timed future owns release: unwinding unblocks runtime teardown.
        let (release, blocker) = occupy().await;
        let caller = tokio::spawn({
            let client = transport.client.clone();
            async move { client.diagnostics().await }
        });
        transport.stage(Stage::Entered).await;
        caller.abort();
        assert!(
            phase("aborted wire caller joins", caller)
                .await
                .unwrap_err()
                .is_cancelled()
        );
        // This is the real server-side future's Drop, after the RPC driver
        // processes the client's Cancel, not merely an aborted client task.
        transport.stage(Stage::Cancelled).await;
        for _ in 1..MAX_DIAGNOSTIC_COLLECTIONS {
            let report = phase("real RPC partial deadline", transport.client.diagnostics())
                .await
                .unwrap();
            assert_eq!(report.fields().node.id, node.info().id);
            partial(&report);
        }
        transport.close().await;
        let error = phase(
            "new connection still sees retained source capacity",
            other.client().diagnostics(),
        )
        .await
        .unwrap_err();
        busy(error);
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

async fn occupy() -> (std::sync::mpsc::Sender<()>, tokio::task::JoinHandle<()>) {
    let (ready, registered) = tokio::sync::oneshot::channel();
    let (release, hold) = std::sync::mpsc::channel();
    let blocker = tokio::task::spawn_blocking(move || {
        ready.send(()).unwrap();
        let _released_or_closed = hold.recv();
    });
    phase("occupy real wire source pool", registered)
        .await
        .unwrap();
    (release, blocker)
}

fn partial(report: &vessel_proto::diagnostics::DiagnosticReport) {
    assert!(matches!(
        report.fields().store,
        DiagnosticSection::Unavailable {
            reason: DiagnosticUnavailable::TimedOut
        }
    ));
    assert!(matches!(
        report.fields().agents,
        DiagnosticSection::Unavailable {
            reason: DiagnosticUnavailable::TimedOut
        }
    ));
    assert!(matches!(
        report.fields().listeners,
        DiagnosticSection::Available { .. }
    ));
}
fn busy(error: vessel_client::DiagnosticError) {
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
}
