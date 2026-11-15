//! The independently tracked setup owner survives caller loss but stays finite.
use super::*;
use crate::automation::notices::{Notices, PER_SESSION};
use std::future::pending;
use tokio::sync::oneshot;
use vessel_proto::SessionId;

const TEST_DEADLINE: Duration = Duration::from_secs(150);
#[derive(Clone, Copy)]
enum Stage {
    Lookup,
    TrackPrompt,
}
#[derive(Clone, Copy)]
enum Ending {
    CallerDroppedThenDeadline,
    Stop,
    ParentEnded,
}

#[cfg(test)]
async fn held_setup(stage: Stage, ending: Ending) {
    let notices = Arc::new(Notices::default());
    let parent = SessionId::random();
    let mut slots: Vec<_> = (0..PER_SESSION)
        .map(|_slot| notices.follow(parent).unwrap())
        .collect();
    let following = slots.pop().unwrap();
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let worker_stop = stop.clone();
    let (lookup_started, lookup_entered) = oneshot::channel();
    let (prompt_started, prompt_entered) = oneshot::channel();
    let mut prompt_entered = Some(prompt_entered);
    let (ended, parent_ended) = oneshot::channel();
    let (settled, outcome) = oneshot::channel();
    let worker = tasks.spawn(async move {
        let _following = following;
        let lookup = async move {
            lookup_started.send(()).unwrap();
            if matches!(stage, Stage::Lookup) {
                pending::<Result<(), WireError>>().await
            } else {
                Ok(())
            }
        };
        let result = supervision::setup(
            lookup,
            || async move {
                prompt_started.send(()).unwrap();
                // An already dispatched prompt may have effects while its
                // response is withheld. Setup must not retry it or invent an id.
                pending::<Result<(), WireError>>().await
            },
            async {
                parent_ended.await.unwrap();
            },
            &worker_stop,
        )
        .await;
        settled.send(result).unwrap();
    });
    let caller = tokio::spawn(async move {
        worker.await.unwrap();
    });
    lookup_entered.await.unwrap();
    if matches!(stage, Stage::TrackPrompt) {
        prompt_entered.take().unwrap().await.unwrap();
    }
    assert!(notices.follow(parent).is_err());
    match ending {
        Ending::CallerDroppedThenDeadline => {
            caller.abort();
            assert!(caller.await.unwrap_err().is_cancelled());
            assert!(
                !tasks.is_empty(),
                "caller loss must not detach ownership tracking"
            );
            assert!(notices.follow(parent).is_err());
            tokio::time::advance(supervision::SETUP_BUDGET).await;
        }
        Ending::Stop => {
            stop.cancel();
            drop(caller);
        }
        Ending::ParentEnded => {
            ended.send(()).unwrap();
            drop(caller);
        }
    }
    let error = outcome.await.unwrap().unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert!(error.message.contains("could not be confirmed"));
    if let Some(not_started) = prompt_entered {
        assert!(
            not_started.await.is_err(),
            "held lookup must never dispatch TrackPrompt"
        );
    }
    let _closed = tasks.close();
    tasks.wait().await;
    let _released = notices.follow(parent).unwrap();
}

#[tokio::test(start_paused = true)]
async fn held_lookup_and_prompt_survive_caller_drop_but_release_on_stop_end_or_deadline() {
    for stage in [Stage::Lookup, Stage::TrackPrompt] {
        for ending in [
            Ending::CallerDroppedThenDeadline,
            Ending::Stop,
            Ending::ParentEnded,
        ] {
            tokio::time::timeout(TEST_DEADLINE, held_setup(stage, ending))
                .await
                .unwrap();
        }
    }
}
