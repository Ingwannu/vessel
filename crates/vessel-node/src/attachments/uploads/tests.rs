//! Tests of the uploads' bounds and their idle drop, through a real store.

use super::*;

#[test]
fn idle_uploads_expire_and_their_files_go() {
    let temp = tempfile::tempdir().unwrap();
    let uploads = Uploads::new(temp.path().join("uploads"), IDLE);
    let path = temp.path().join("uploads").join("u");
    let file = stage(&temp.path().join("uploads"), &path).unwrap();
    let id = UploadId::random();
    let open = Open {
        holder: Holder::Owner,
        declared: Declared {
            session: SessionId::random(),
            name: "a.txt".parse().unwrap(),
            mime: "text/plain".parse().unwrap(),
            kind: AttachmentKind::File,
            size: 1,
        },
        path: path.clone(),
        used: Mutex::new(Instant::now()),
        progress: tokio::sync::Mutex::new(Some(Progress::new(file))),
    };
    drop(uploads.open().insert(id, Arc::new(open)));
    uploads.expire(Instant::now());
    assert!(uploads.held(Holder::Owner, id).is_ok(), "not idle yet");
    assert!(
        matches!(
            uploads.held(Holder::Device(vessel_proto::DeviceId::random()), id),
            Err(UploadError::Unknown(_))
        ),
        "another holder never sees it"
    );
    let later = Instant::now()
        .checked_add(IDLE.saturating_add(Duration::from_secs(1)))
        .unwrap();
    uploads.expire(later);
    assert!(matches!(
        uploads.held(Holder::Owner, id),
        Err(UploadError::Unknown(_))
    ));
    assert!(!path.exists(), "its staging file went with it");
}

fn declared(session: SessionId, size: u64) -> Declared {
    Declared {
        session,
        name: "a.bin".parse().unwrap(),
        mime: "application/octet-stream".parse().unwrap(),
        kind: AttachmentKind::File,
        size,
    }
}

/// Four places a holder, sixteen a node; a session's bytes and the
/// node's count what is kept and what is declared.
#[tokio::test]
async fn begins_are_bounded_by_places_and_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let dir = vessel_store::DataDir::new(temp.path());
    let store = Store::open(&dir, vessel_store::StoreConfig::default())
        .await
        .unwrap()
        .store;
    let uploads = Uploads::new(temp.path().join("uploads"), IDLE);
    let session = SessionId::random();
    let me = Holder::Device(vessel_proto::DeviceId::random());
    for _ in 0..AttachmentLimits::OPEN_UPLOADS_PER_CALLER {
        let _id = uploads
            .begin(&store, me, declared(session, 1))
            .await
            .unwrap();
    }
    assert!(matches!(
        uploads.begin(&store, me, declared(session, 1)).await,
        Err(UploadError::Busy { yours: true, .. })
    ));
    for _ in 0..3 {
        let other = Holder::Device(vessel_proto::DeviceId::random());
        for _ in 0..AttachmentLimits::OPEN_UPLOADS_PER_CALLER {
            let _id = uploads
                .begin(&store, other, declared(SessionId::random(), 1))
                .await
                .unwrap();
        }
    }
    assert!(matches!(
        uploads
            .begin(&store, Holder::Owner, declared(SessionId::random(), 1))
            .await,
        Err(UploadError::Busy { yours: false, .. })
    ));

    // Bytes: a session's 100 MiB with what it has open.
    let uploads = Uploads::new(temp.path().join("uploads-2"), IDLE);
    let per_session = AttachmentLimits::DEFAULT.per_session;
    let _most = uploads
        .begin(&store, Holder::Owner, declared(session, per_session - 1))
        .await
        .unwrap();
    assert!(matches!(
        uploads.begin(&store, Holder::Owner, declared(session, 2)).await,
        Err(UploadError::OverLimit { size, .. }) if size == per_session + 1
    ));
    assert!(
        uploads
            .begin(&store, Holder::Owner, declared(session, 1))
            .await
            .is_ok(),
        "exactly the limit"
    );

    // The node's 4 GiB with what it keeps.
    let kept = SessionId::random();
    let spec = vessel_proto::SessionSpec::new(
        vessel_proto::HarnessKind::ClaudeCode,
        vessel_proto::SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    store
        .create_session(vessel_proto::SessionInfo::new(
            kept,
            spec,
            chrono::Utc::now(),
        ))
        .await
        .unwrap();
    let connection = rusqlite::Connection::open(dir.database_path()).unwrap();
    let _rows = connection
        .execute(
            "INSERT INTO attachments (id, session_id, name, kind, mime, size, sha256, created_at) \
             VALUES (?1, ?2, 'big.bin', 'file', 'application/octet-stream', ?3, zeroblob(32), 0)",
            rusqlite::params![
                vessel_proto::AttachmentId::random().to_string(),
                kept.to_string(),
                i64::try_from(AttachmentLimits::PER_NODE - (1 << 20)).unwrap(),
            ],
        )
        .unwrap();
    let elsewhere = SessionId::random();
    assert!(matches!(
        uploads
            .begin(&store, Holder::Owner, declared(elsewhere, 1 << 20))
            .await,
        Err(UploadError::OverLimit { what, .. }) if what.contains("node")
    ));
    store.close().await;
}
