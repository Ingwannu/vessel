//! Protocol 11 in the session runtime, with `fake-acp` as Claude's
//! adapter: a prompt that asked learns which turn it started or joined
//! (`SessionHandle::prompt_tracked`), a queued one when its entry starts
//! one, and a removed one never.
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use vessel_proto::{
    AgentEvent, ContentBlock, Creator, Delivered, Delivery, EventSeq, QueueChange, SessionEvent,
};

use support::{Fixture, Log, structured, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

fn text(text: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: text.to_owned(),
    }]
}

/// The seq of the `turn_started` record of `prompt`'s turn, once read.
async fn turn_of(log: &mut Log, prompt: &str) -> Option<EventSeq> {
    let started = |event: &SessionEvent| {
        matches!(event, SessionEvent::Agent { event: AgentEvent::TurnStarted { provenance: None, content ,
client_message_id: None,
} }
            if *content == text(prompt))
    };
    log.until(started).await;
    log.records
        .iter()
        .find(|record| started(&record.event))
        .map(|record| record.seq)
}

#[tokio::test]
async fn a_prompt_learns_the_turn_it_started_joined_or_later_started() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        let owner = || Creator::Owner;

        let (delivered, waits) = handle
            .prompt_tracked(text("wait"), Delivery::Queue, owner())
            .await
            .unwrap();
        assert_eq!(delivered, Delivered::Started);
        let running = turn_of(&mut log, "wait").await.unwrap();
        assert_eq!(waits.await.unwrap(), running);

        // Queued behind it: told when its entry starts a turn.
        let (delivered, queued) = handle
            .prompt_tracked(text("plain"), Delivery::Queue, owner())
            .await
            .unwrap();
        assert!(matches!(delivered, Delivered::Queued { .. }));
        // Queued, then removed: never told.
        let (removed, never) = handle
            .prompt_tracked(text("never sent"), Delivery::Queue, owner())
            .await
            .unwrap();
        let Delivered::Queued { entry } = removed else {
            panic!("a turn runs: it waits");
        };
        let _changed = handle
            .change_queue(QueueChange::Remove { entry })
            .await
            .unwrap();
        assert!(never.await.is_err(), "a removed entry starts no turn");

        // A steer joins the running turn.
        let (delivered, joined) = handle
            .prompt_tracked(text("now"), Delivery::Steer, owner())
            .await
            .unwrap();
        assert_eq!(delivered, Delivered::Steered);
        assert_eq!(joined.await.unwrap(), running);

        // The fake ends its `wait` turn on `finish`; the queue moves.
        let _finished = handle
            .prompt(text("finish"), Delivery::Steer, owner())
            .await
            .unwrap();
        let next = turn_of(&mut log, "plain").await.unwrap();
        assert_eq!(queued.await.unwrap(), next);
        assert!(next > running);

        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}
