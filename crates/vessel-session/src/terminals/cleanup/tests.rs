use super::*;
use std::{future::Future, task::Poll, time::Duration};
const SAFETY_NET: Duration = Duration::from_secs(60);
const PHASE: Duration = Duration::from_secs(5);
async fn phase<T>(name: &'static str, wait: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, wait)
        .await
        .unwrap_or_else(|_elapsed| panic!("cleanup unit phase timed out: {name}"))
}

#[tokio::test]
async fn losing_the_actual_runner_never_fabricates_completion() {
    vessel_testkit::within(SAFETY_NET, async {
        let terminal = TerminalId::random();
        let (completion, progress) = begin(terminal);
        let waiting = progress.observe();
        tokio::pin!(waiting);
        phase(
            "original report is pending",
            std::future::poll_fn(|cx| {
                assert!(waiting.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            }),
        )
        .await;
        drop(completion);
        assert_eq!(
            phase("sender loss becomes Abandoned", waiting).await,
            CleanupObservation::Abandoned { terminal }
        );
    })
    .await
    .expect("cleanup unit exceeded canonical safety net");
}

#[tokio::test]
async fn actual_completion_preserves_each_unproved_fact() {
    vessel_testkit::within(SAFETY_NET, async {
        let terminal = TerminalId::random();
        for facts in [Facts::unconfirmed(), Facts::not_started()] {
            let (completion, progress) = begin(terminal);
            completion.finish(facts);
            let CleanupObservation::Finished(report) =
                phase("observe original partial report", progress.observe()).await
            else {
                panic!("lost actual report")
            };
            assert_eq!(report.terminal(), terminal);
            assert_eq!(report.process(), facts.process);
            assert_eq!(report.descendants(), facts.descendants);
            assert_eq!(report.io(), facts.io);
            assert_eq!(report.auxiliary(), facts.auxiliary);
            assert!(!report.is_confirmed());
        }
        let (completion, progress) = begin(terminal);
        let mut facts = Facts::not_started();
        facts.auxiliary = CleanupFact::Confirmed;
        completion.finish(facts);
        let CleanupObservation::Finished(report) =
            phase("observe original complete report", progress.observe()).await
        else {
            panic!("lost actual report")
        };
        assert!(report.is_confirmed());
    })
    .await
    .expect("cleanup unit exceeded canonical safety net");
}
