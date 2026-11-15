//! Drains admitted changes through the existing engine, even without a caller.
use super::{Change, Engine};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    engine: Arc<Engine>,
    mut changes: mpsc::Receiver<Change>,
    stopping: CancellationToken,
) {
    loop {
        let change = tokio::select! {
            biased;
            () = stopping.cancelled() => {
                changes.close();
                break;
            }
            next = changes.recv() => match next {
                Some(change) => change,
                None => return,
            },
        };
        apply(&engine, change).await;
    }
    // A shutdown closes admission, not an operation's ownership. In
    // particular a blocking atomic file replacement must settle before a
    // later change or teardown can pass its runtime apply.
    while let Some(change) = changes.recv().await {
        apply(&engine, change).await;
    }
}
async fn apply(engine: &Engine, change: Change) {
    let result = engine.change(change.request).await;
    if let Err(error) = &result {
        tracing::warn!(%error, "node settings operation failed");
    }
    drop(change.reply.send(result));
}
