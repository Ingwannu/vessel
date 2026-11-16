//! The initial row snapshot cannot include actor publications from a later cursor.

use support::{Fixture, structured, title, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "creation needs only the runtime fixture and title constructor"
)]
mod support;

#[tokio::test]
async fn the_empty_logs_seed_precedes_actor_metadata_publication() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let original = title("Creation title");
        let created = fixture
            .runtime
            .create(structured(Vec::new()).with_title(original.clone()))
            .await
            .unwrap();
        let handle = &created.handle;
        let renamed = handle.rename(title("Already published")).await.unwrap();
        // Hold the durable receiver: publication is forced ahead of the row's
        // creation, without depending on a thread winning a startup race.
        assert!(renamed.next_seq > vessel_proto::EventSeq::ZERO);
        assert_eq!(renamed.title, Some(title("Already published")));
        assert_eq!(created.initial.next_seq, vessel_proto::EventSeq::ZERO);
        assert_eq!(created.initial.title, Some(original));
        assert_eq!(created.initial.spec, renamed.spec);
        assert_eq!(created.initial.id, renamed.id);
        assert_eq!(created.initial.created_at, renamed.created_at);
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}
