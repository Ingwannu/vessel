//! Durable snapshot metadata remains paged across appends and rejects rewinds.

use vessel_proto::prompt_index::{ListSessionPrompts, PromptIndexLimit};
use vessel_proto::{EventSeq, SessionEvent, SessionId};
use vessel_store::{StoreConfig, StoreError};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(dead_code, reason = "shared store builders; this test uses a subset")]
mod fixtures;
#[cfg(test)]
#[path = "support/messages.rs"]
#[expect(dead_code, reason = "shared search builders; this test uses a subset")]
mod messages;

fn request(session: SessionId) -> ListSessionPrompts {
    ListSessionPrompts {
        session,
        after: None,
        epoch: None,
        limit: PromptIndexLimit::new(1).unwrap(),
    }
}

#[tokio::test]
async fn prompt_index_reads_one_snapshot_and_survives_reopen_without_a_new_schema() {
    let (_temp, dir) = fixtures::data_dir();
    let store = fixtures::open(&dir, StoreConfig::default()).await;
    let session = messages::started(&store, 0).await;
    assert!(
        store
            .prompt_index(request(session))
            .await
            .unwrap()
            .entries()
            .is_empty()
    );
    store
        .append(
            session,
            vec![messages::prompt(1, "first"), messages::prompt(2, "second")],
        )
        .await
        .unwrap();
    let first = store.prompt_index(request(session)).await.unwrap();
    assert_eq!(first.entries().first().unwrap().preview.as_str(), "first");
    assert_eq!(first.next(), Some(EventSeq::new(1)));
    store
        .append(session, vec![messages::prompt(3, "third")])
        .await
        .unwrap();
    let mut next = request(session);
    next.after = first.next();
    next.epoch = Some(first.epoch());
    let second = store.prompt_index(next.clone()).await.unwrap();
    assert_eq!(second.entries().first().unwrap().preview.as_str(), "second");
    assert_eq!(first.epoch(), second.epoch());
    store
        .append(
            session,
            vec![fixtures::record(
                4,
                SessionEvent::Rewound {
                    after: EventSeq::new(1),
                    harness_session: "fake-history".parse().unwrap(),
                    files_changed: 0,
                },
            )],
        )
        .await
        .unwrap();
    assert!(matches!(
        store.prompt_index(next).await,
        Err(StoreError::PromptIndexChanged)
    ));
    let visible = store.prompt_index(request(session)).await.unwrap();
    assert_eq!(visible.entries().len(), 1);
    assert_eq!(visible.next(), None);
    drop(store);
    let reopened = fixtures::open(&dir, StoreConfig::default()).await;
    assert_eq!(
        reopened.prompt_index(request(session)).await.unwrap(),
        visible
    );
    assert!(matches!(
        reopened.prompt_index(request(SessionId::random())).await,
        Err(StoreError::UnknownSession(_))
    ));
}
