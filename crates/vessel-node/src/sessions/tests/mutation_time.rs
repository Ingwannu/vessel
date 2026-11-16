//! Actual writer acknowledgements reach live and ended runtime metadata.

use super::{Fixture, ready};
use crate::sessions::Found;
use std::time::Duration;
use vessel_proto::{
    AgentEvent, ContentBlock, SessionEvent, SessionInput,
    inbox::{InboxKey, InboxRead, MarkInbox},
};
use vessel_proto::{Creator, EventSeq, Mark, NodeId, SessionId};
use vessel_session::{LaunchOptions, SessionHandle};

const MUTATION_PUBLICATION_BUDGET: Duration = Duration::from_secs(30);

#[tokio::test]
async fn creation_append_mark_and_handover_keep_acknowledged_clock_data() {
    vessel_testkit::within(
        MUTATION_PUBLICATION_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let created = fixture
                .sessions
                .create(fixture.structured(), LaunchOptions::default(), None)
                .await
                .unwrap();
            let initial = created.updated_at.unwrap();
            let handle = fixture.runtime.get(created.id).unwrap();
            let _running = ready(&handle).await.unwrap();
            let _page = fixture
                .sessions
                .read(created.id, EventSeq::ZERO, 100)
                .await
                .unwrap();
            let before = handle.info().updated_at.unwrap();
            assert!(before >= initial);
            let marked = fixture.sessions.mark(created.id, Mark::Pin).await.unwrap();
            assert!(marked.marks.pinned);
            assert!(marked.updated_at.unwrap() > before);
            assert!(
                fixture
                    .store
                    .session(created.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .updated_at
                    >= marked.updated_at
            );
            assert_eq!(
                handle.watch_info().borrow().updated_at,
                None,
                "the raw actor watch must not present a stale durable clock"
            );
            handle.kill();
            let _ended = handle.ended().await;
            fixture.sessions.wait_stored(created.id).await.unwrap();
            let stored = fixture.store.session(created.id).await.unwrap().unwrap();
            assert_eq!(handle.info().updated_at, stored.updated_at);
            assert!(stored.updated_at > marked.updated_at);
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn live_detach_publishes_its_ack_and_repeating_it_does_not_erase_time() {
    vessel_testkit::within(
        MUTATION_PUBLICATION_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let created = fixture
                .sessions
                .create(
                    fixture.structured(),
                    LaunchOptions {
                        created_by: Creator::Agent {
                            node: NodeId::random(),
                            session: SessionId::random(),
                        },
                        ..LaunchOptions::default()
                    },
                    None,
                )
                .await
                .unwrap();
            let handle = fixture.runtime.get(created.id).unwrap();
            let _running = ready(&handle).await.unwrap();
            let detached = fixture.sessions.detach(created.id).await.unwrap();
            assert!(detached.session.updated_at.unwrap() > created.updated_at.unwrap());
            assert_eq!(handle.info().detach_receipt, Some(detached.receipt));
            assert!(handle.info().updated_at >= detached.session.updated_at);
            let repeated = fixture.sessions.detach(created.id).await.unwrap();
            assert_eq!(repeated.receipt, detached.receipt);
            assert!(repeated.session.updated_at >= detached.session.updated_at);
            fixture.runtime.shutdown().await;
            fixture.settle().await;
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn exact_loaded_attention_marks_preserve_time_and_survive_caller_cancellation() {
    vessel_testkit::within(
        MUTATION_PUBLICATION_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let created = fixture
                .sessions
                .create(fixture.structured(), LaunchOptions::default(), None)
                .await
                .unwrap();
            let handle = fixture.runtime.get(created.id).unwrap();
            let _running = ready(&handle).await.unwrap();
            let key = finished_key(&fixture, &handle).await;
            let before = handle.info().updated_at;
            fixture
                .sessions
                .mark_inbox(MarkInbox {
                    key,
                    read: InboxRead::Read,
                })
                .await
                .unwrap();
            assert_eq!(handle.info().updated_at, before);
            assert_eq!(
                fixture.store.inbox_entry(key).await.unwrap().unwrap().read,
                InboxRead::Read
            );
            let read = fixture
                .sessions
                .mark(
                    created.id,
                    Mark::Read {
                        through: handle.log_end(),
                    },
                )
                .await
                .unwrap();
            assert!(!read.marks.unread);
            assert_eq!(read.updated_at, before);
            let unread = fixture
                .sessions
                .mark(created.id, Mark::Unread)
                .await
                .unwrap();
            assert!(unread.marks.unread);
            assert_eq!(unread.updated_at, before);
            fixture
                .sessions
                .mark_inbox(MarkInbox {
                    key,
                    read: InboxRead::Unread,
                })
                .await
                .unwrap();
            let before_cancel = handle.info().updated_at;
            cancel_before_admission(&fixture, key).await;
            cancel_after_admission(&fixture, key);
            fixture.runtime.shutdown().await;
            fixture.settle().await;
            assert_eq!(
                fixture.store.inbox_entry(key).await.unwrap().unwrap().read,
                InboxRead::Read
            );
            // This later increase belongs to the final Exited append. The
            // cancelled inbox operation is proved by Read above and the
            // retained lease/task checks, not by an attention-clock tick.
            assert!(
                fixture
                    .store
                    .session(created.id)
                    .await
                    .unwrap()
                    .unwrap()
                    .updated_at
                    > before_cancel
            );
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}

// Isolate actual turn/persistence setup from the cancellation-order assertions.
async fn finished_key(fixture: &Fixture, handle: &SessionHandle) -> InboxKey {
    let mut follow = handle.follow(EventSeq::ZERO).unwrap();
    handle
        .send(SessionInput::Prompt {
            content: vec![ContentBlock::Text {
                text: "plain".to_owned(),
            }],
        })
        .await
        .unwrap();
    let seq = loop {
        let record = follow.live.recv().await.unwrap();
        if matches!(
            &record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        ) {
            break record.seq;
        }
    };
    let mut durability = match fixture.sessions.find(handle.id()).await.unwrap() {
        Found::Live { durability, .. } => durability,
        Found::Stored(_) => panic!("the fake must remain live after its turn"),
    };
    // Publication precedes Info update. Wait for THIS known record's commit,
    // not a possibly earlier cursor read immediately after its broadcast.
    crate::persist::wait_until(handle.id(), &mut durability, seq.next().unwrap())
        .await
        .unwrap();
    InboxKey {
        session: handle.id(),
        seq,
    }
}

async fn cancel_before_admission(fixture: &Fixture, key: InboxKey) {
    let held = fixture.sessions.marking.lock().await;
    let task_count = fixture.tasks.len();
    let waker = std::task::Waker::noop();
    let mut waiting = Box::pin(fixture.sessions.mark_inbox(MarkInbox {
        key,
        read: InboxRead::Read,
    }));
    assert!(
        waiting
            .as_mut()
            .poll(&mut std::task::Context::from_waker(waker))
            .is_pending()
    );
    assert_eq!(
        fixture.tasks.len(),
        task_count,
        "no retained task before admission"
    );
    drop(waiting);
    drop(held);
    assert_eq!(
        fixture.store.inbox_entry(key).await.unwrap().unwrap().read,
        InboxRead::Unread
    );
}

fn cancel_after_admission(fixture: &Fixture, key: InboxKey) {
    let task_count = fixture.tasks.len();
    let waker = std::task::Waker::noop();
    let mut call = Box::pin(fixture.sessions.mark_inbox(MarkInbox {
        key,
        read: InboxRead::Read,
    }));
    assert!(
        call.as_mut()
            .poll(&mut std::task::Context::from_waker(waker))
            .is_pending()
    );
    assert!(
        fixture.tasks.len() > task_count,
        "the admitted task exists before caller loss"
    );
    assert!(
        fixture.sessions.marking.try_lock().is_err(),
        "the task owns the same metadata order"
    );
    drop(call);
}

#[tokio::test]
async fn reopen_publishes_its_real_writer_clock_before_opening_the_existing_gate() {
    vessel_testkit::within(
        MUTATION_PUBLICATION_BUDGET,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let created = fixture
                .sessions
                .create(fixture.structured(), LaunchOptions::default(), None)
                .await
                .unwrap();
            let handle = fixture.runtime.get(created.id).unwrap();
            let _running = ready(&handle).await.unwrap();
            handle.kill();
            let _ended = handle.ended().await;
            fixture.sessions.wait_stored(created.id).await.unwrap();
            let before = fixture
                .store
                .session(created.id)
                .await
                .unwrap()
                .unwrap()
                .updated_at;
            let claim = fixture.sessions.claim(created.id).unwrap();
            let ended = fixture.sessions.ended(created.id).await.unwrap();
            let reopened = fixture
                .sessions
                .start_again(ended, vessel_session::Relaunch::default(), claim)
                .await
                .unwrap();
            assert!(reopened.info().updated_at > before);
            assert_eq!(reopened.watch_info().borrow().updated_at, None);
            let _running = ready(&reopened).await.unwrap();
            let _page = fixture
                .sessions
                .read(created.id, EventSeq::ZERO, 100)
                .await
                .unwrap();
            assert!(reopened.info().updated_at >= before);
            fixture.runtime.shutdown().await;
            fixture.settle().await;
            let kept = fixture
                .store
                .session(created.id)
                .await
                .unwrap()
                .unwrap()
                .updated_at;
            assert_eq!(reopened.info().updated_at, kept);
            fixture.stop().await;
        }),
    )
    .await
    .unwrap();
}
