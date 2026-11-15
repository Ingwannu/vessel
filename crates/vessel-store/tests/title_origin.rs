//! Durable origin prevents an old or a person's title becoming eligible
//! merely because a node restarted. No model is involved in these tests.

use fixtures::{data_dir, new_session, open, record};
use vessel_proto::{EventSeq, SessionEvent, SessionStatus, SessionTitle, TitleOrigin};
use vessel_store::StoreConfig;

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "only the shared store fixture's session and record builders are needed"
)]
mod fixtures;

const BEFORE_TITLE_ORIGIN_SCHEMA: i64 = 17;

#[tokio::test]
async fn origins_follow_renames_and_survive_reopening() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut info = new_session(0);
    info.title = Some("Mine".parse().unwrap());
    info.title_origin = TitleOrigin::Typed;
    store.create_session(info.clone()).await.unwrap();
    assert_eq!(
        store.session(info.id).await.unwrap().unwrap().title_origin,
        TitleOrigin::Typed
    );
    let origins = [
        TitleOrigin::Given,
        TitleOrigin::Generated,
        TitleOrigin::Typed,
        TitleOrigin::Unknown,
    ];
    for (seq, origin) in origins.into_iter().enumerate() {
        let title: SessionTitle = format!("Title {seq}").parse().unwrap();
        store
            .append(
                info.id,
                vec![record(
                    u64::try_from(seq).unwrap(),
                    SessionEvent::Renamed {
                        title: title.clone(),
                        origin,
                    },
                )],
            )
            .await
            .unwrap();
        let stored = store.session(info.id).await.unwrap().unwrap();
        assert_eq!((stored.title, stored.title_origin), (Some(title), origin));
    }
    let mut saved = Vec::new();
    for (index, origin) in origins.into_iter().enumerate() {
        let mut another = new_session(i64::try_from(index).unwrap());
        another.title = Some("Persisted".parse().unwrap());
        another.title_origin = origin;
        store.create_session(another.clone()).await.unwrap();
        saved.push(another);
    }
    store.close().await;
    drop(store);
    let store = open(&dir, StoreConfig::default()).await;
    for another in saved {
        let stored = store.session(another.id).await.unwrap().unwrap();
        assert_eq!(stored.title_origin, another.title_origin);
        assert_eq!(stored.title, another.title);
    }
    let stored = store.session(info.id).await.unwrap().unwrap();
    assert_eq!(stored.title_origin, TitleOrigin::Unknown);
    assert_eq!(stored.title.unwrap().as_str(), "Title 3");
    let events = store
        .read_events(info.id, EventSeq::ZERO, 10)
        .await
        .unwrap();
    let (recovery, renames) = events.records.split_last().unwrap();
    assert_eq!(recovery.seq, EventSeq::new(4));
    assert_eq!(
        recovery.event,
        SessionEvent::Status {
            status: SessionStatus::Interrupted
        }
    );
    assert_eq!(renames.len(), origins.len());
    let read: Vec<_> = renames
        .iter()
        .map(|record| {
            let SessionEvent::Renamed { origin, .. } = record.event else {
                panic!("rename expected")
            };
            origin
        })
        .collect();
    assert_eq!(read, origins);
}

#[tokio::test]
async fn pre_origin_schema_titles_are_unknown_including_explicit_ones() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let mut info = new_session(0);
    info.spec.title = Some("Explicit old title".parse().unwrap());
    info.title = info.spec.title.clone();
    info.title_origin = TitleOrigin::Typed;
    store.create_session(info.clone()).await.unwrap();
    store.close().await;
    drop(store);
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    connection.execute_batch("DROP TRIGGER timeline_admission_insert; DROP TRIGGER timeline_admission_delete; DROP TABLE timeline_admissions; DROP TABLE timeline_admission_usage; DROP TABLE timeline_admission_meta;").unwrap();
    // Gate40's existing title-origin append is schema18. Later appends must not
    // turn this into a fixture for the newest migration instead of that one.
    fixtures::undo_registry_schema(&connection);
    connection
        .execute_batch(
            "ALTER TABLE sessions DROP COLUMN detach_receipt;
     ALTER TABLE sessions DROP COLUMN title_origin;",
        )
        .unwrap();
    connection
        .pragma_update(None, "user_version", BEFORE_TITLE_ORIGIN_SCHEMA)
        .unwrap();
    drop(connection);
    let store = open(&dir, StoreConfig::default()).await;
    let stored = store.session(info.id).await.unwrap().unwrap();
    assert_eq!(stored.title, info.title);
    assert_eq!(stored.title_origin, TitleOrigin::Unknown);
}
