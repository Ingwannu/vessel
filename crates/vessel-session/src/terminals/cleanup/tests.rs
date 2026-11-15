use super::*;
use std::{task::Poll, time::Duration};
const SAFETY_NET: Duration = Duration::from_secs(5);

#[tokio::test]
async fn losing_the_actual_runner_never_fabricates_completion() {
    let terminal = TerminalId::random();
    let (completion, progress) = begin(terminal);
    let waiting = progress.observe();
    tokio::pin!(waiting);
    std::future::poll_fn(|cx| {
        assert!(waiting.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(completion);
    assert_eq!(
        tokio::time::timeout(SAFETY_NET, waiting).await.unwrap(),
        CleanupObservation::Abandoned { terminal }
    );
}

#[tokio::test]
async fn actual_completion_preserves_each_unproved_fact() {
    let terminal = TerminalId::random();
    for facts in [Facts::unconfirmed(), Facts::not_started()] {
        let (completion, progress) = begin(terminal);
        completion.finish(facts);
        let CleanupObservation::Finished(report) = progress.observe().await else {
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
    let CleanupObservation::Finished(report) = progress.observe().await else {
        panic!("lost actual report")
    };
    assert!(report.is_confirmed());
}
