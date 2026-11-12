//! Attachments kept by the store (protocol 10, ADR 0025): a finished
//! upload put in place and recorded, read back only as the file that was
//! kept, refused for an ended session, and swept at open when no row
//! names it.

use std::sync::Arc;

use fixtures::{at, data_dir, new_session, open, status};
use sha2::{Digest as _, Sha256};
use vessel_proto::{
    AttachmentId, AttachmentKind, AttachmentRef, BlobHash, EventRecord, EventSeq, SessionEvent,
    SessionId, SessionStatus,
};
use vessel_store::{
    AttachmentFileError, AttachmentUsage, DataDir, KeptAttachment, Store, StoreConfig, StoreError,
};

#[cfg(test)]
#[path = "support/fixtures.rs"]
#[expect(
    dead_code,
    reason = "builders shared with the log's tests; this file uses some"
)]
mod fixtures;

/// Builders for this file. A `cfg(test)` module so the lints' test
/// allowances apply.
#[cfg(test)]
mod support {
    use super::*;

    /// A stored session, started and running.
    pub(crate) async fn running(store: &Store) -> SessionId {
        let session = new_session(0);
        store.create_session(session.clone()).await.unwrap();
        store
            .append(
                session.id,
                vec![
                    status(0, SessionStatus::Starting),
                    status(1, SessionStatus::Running),
                ],
            )
            .await
            .unwrap();
        session.id
    }

    pub(crate) async fn end(store: &Store, session: SessionId) {
        let end = Arc::new(EventRecord {
            seq: EventSeq::new(2),
            at: at(5),
            event: SessionEvent::Status {
                status: SessionStatus::Exited { code: Some(0) },
            },
        });
        store.append(session, vec![end]).await.unwrap();
    }

    /// `bytes` as an upload staged in `dir`'s `uploads/`, and the record
    /// that keeps it for `session`.
    pub(crate) fn staged(
        dir: &DataDir,
        session: SessionId,
        name: &str,
        bytes: &[u8],
    ) -> (std::path::PathBuf, KeptAttachment) {
        let uploads = dir.root().join("uploads");
        std::fs::create_dir_all(&uploads).unwrap();
        let path = uploads.join(AttachmentId::random().to_string());
        std::fs::write(&path, bytes).unwrap();
        let kept = KeptAttachment {
            session,
            reference: AttachmentRef {
                id: AttachmentId::random(),
                name: name.parse().unwrap(),
                kind: AttachmentKind::File,
                mime: "text/plain".parse().unwrap(),
                size: u64::try_from(bytes.len()).unwrap(),
            },
            sha256: BlobHash::from_bytes(Sha256::digest(bytes).into()),
        };
        (path, kept)
    }

    /// Keep `bytes` for `session`.
    pub(crate) async fn keep(
        store: &Store,
        dir: &DataDir,
        session: SessionId,
        bytes: &[u8],
    ) -> KeptAttachment {
        let (path, kept) = staged(dir, session, "notes.txt", bytes);
        store
            .keep_attachment(kept.clone(), path, at(3))
            .await
            .unwrap();
        kept
    }

    pub(crate) fn changed(result: Result<vessel_store::ReadBack, StoreError>) -> &'static str {
        match result {
            Err(StoreError::Attachment(AttachmentFileError::Changed { what, .. })) => what,
            other => panic!("expected a changed file, got {other:?}"),
        }
    }
}

use support::{changed, end, keep, running, staged};

/// A kept attachment is recorded, counted, and read back whole or by
/// pieces from where it was put.
#[tokio::test]
async fn an_upload_is_kept_and_read_back() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = running(&store).await;
    let other = running(&store).await;
    let kept = keep(&store, &dir, session, b"hello, attachment").await;
    let _theirs = keep(&store, &dir, other, b"1234").await;

    assert_eq!(
        store.attachment(session, kept.reference.id).await.unwrap(),
        Some(kept.clone())
    );
    assert_eq!(
        store.attachment(other, kept.reference.id).await.unwrap(),
        None,
        "another session's"
    );
    assert_eq!(
        store.attachment_usage(session).await.unwrap(),
        AttachmentUsage {
            node: 21,
            session: 17
        }
    );
    let path = store.attachment_path(&kept);
    assert!(path.ends_with(format!(
        "attachments/{session}/{}/notes.txt",
        kept.reference.id
    )));

    let whole = store
        .read_attachment(kept.clone(), 0, 1 << 20, None)
        .await
        .unwrap();
    assert_eq!(whole.data, b"hello, attachment");
    let piece = store
        .read_attachment(kept.clone(), 7, 4, whole.identity)
        .await
        .unwrap();
    assert_eq!(piece.data, b"atta");
    let end_of_it = store
        .read_attachment(kept.clone(), 17, 4, None)
        .await
        .unwrap();
    assert!(end_of_it.data.is_empty());
}

/// An ended (or unknown) session keeps nothing new, and the file put in
/// place for it is taken back.
#[tokio::test]
async fn an_ended_session_keeps_no_new_attachment() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = running(&store).await;
    end(&store, session).await;
    let (path, kept) = staged(&dir, session, "late.txt", b"late");
    assert!(matches!(
        store.keep_attachment(kept.clone(), path, at(9)).await,
        Err(StoreError::SessionEnded(id)) if id == session
    ));
    assert!(
        !dir.attachments_dir()
            .join(session.to_string())
            .join(kept.reference.id.to_string())
            .exists()
    );

    let unknown = SessionId::random();
    let (path, kept) = staged(&dir, unknown, "lost.txt", b"lost");
    assert!(matches!(
        store.keep_attachment(kept, path, at(9)).await,
        Err(StoreError::UnknownSession(_))
    ));
}

/// Bytes that are not what was received are never kept.
#[tokio::test]
async fn a_staged_file_that_differs_is_not_kept() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = running(&store).await;
    let (path, kept) = staged(&dir, session, "notes.txt", b"what was received");
    std::fs::write(&path, b"what was there instead").unwrap();
    assert!(matches!(
        store.keep_attachment(kept.clone(), path, at(3)).await,
        Err(StoreError::Attachment(AttachmentFileError::Changed { .. }))
    ));
    assert_eq!(
        store.attachment(session, kept.reference.id).await.unwrap(),
        None
    );
    assert!(!store.attachment_path(&kept).exists(), "taken back");
}

/// A file changed after it was kept is refused: other content of the same
/// size, another size, or gone; a verified identity does not hide a
/// change.
#[tokio::test]
async fn a_changed_file_is_never_read_back() {
    let (_temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = running(&store).await;
    let kept = keep(&store, &dir, session, b"original").await;
    let path = store.attachment_path(&kept);
    let verified = store
        .read_attachment(kept.clone(), 0, 0, None)
        .await
        .unwrap()
        .identity;

    // Replaced by another file of the same size: a new inode.
    let replacement = path.with_file_name("replacement");
    std::fs::write(&replacement, b"replaced").unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    assert_eq!(
        changed(store.read_attachment(kept.clone(), 0, 8, verified).await),
        "its content"
    );
    std::fs::write(&path, b"longer than it was").unwrap();
    assert_eq!(
        changed(store.read_attachment(kept.clone(), 0, 8, None).await),
        "its size"
    );
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        store.read_attachment(kept, 0, 8, None).await,
        Err(StoreError::Attachment(AttachmentFileError::Missing(_)))
    ));
}

/// Links planted where the file or a folder on its way was are never
/// followed, whatever they point at; a hard link to a secret is found out
/// by its content.
#[cfg(unix)]
#[tokio::test]
async fn links_planted_in_the_folder_are_never_followed() {
    use std::os::unix::fs::symlink;

    let (temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = running(&store).await;
    let secret = temp.path().join("secret");
    std::fs::write(&secret, b"node-token-0123456789").unwrap();

    // The file itself, as a link to the secret.
    let kept = keep(&store, &dir, session, b"node-token-0123456789").await;
    let path = store.attachment_path(&kept);
    std::fs::remove_file(&path).unwrap();
    symlink(&secret, &path).unwrap();
    assert_eq!(
        changed(store.read_attachment(kept.clone(), 0, 64, None).await),
        "a link or another kind of file"
    );

    // A hard link to the secret is a regular file: its content gives it
    // away (the secret differs from what was sent).
    let other = keep(&store, &dir, session, b"other-content-0123456").await;
    let other_path = store.attachment_path(&other);
    std::fs::remove_file(&other_path).unwrap();
    std::fs::hard_link(&secret, &other_path).unwrap();
    assert_eq!(
        changed(store.read_attachment(other, 0, 64, None).await),
        "its content"
    );

    // The attachment's folder, as a link to a folder holding a file of
    // the same name and content.
    let third = keep(&store, &dir, session, b"same bytes").await;
    let folder = store.attachment_path(&third).parent().unwrap().to_owned();
    let elsewhere = temp.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("notes.txt"), b"same bytes").unwrap();
    std::fs::remove_dir_all(&folder).unwrap();
    symlink(&elsewhere, &folder).unwrap();
    assert_eq!(
        changed(store.read_attachment(third.clone(), 0, 64, None).await),
        "a link or another kind of file"
    );

    // The session's folder, as a link.
    let session_folder = dir.attachments_dir().join(session.to_string());
    let moved = temp.path().join("moved");
    std::fs::rename(&session_folder, &moved).unwrap();
    symlink(&moved, &session_folder).unwrap();
    assert_eq!(
        changed(store.read_attachment(third, 0, 64, None).await),
        "a link or another kind of file"
    );
}

/// At open, every folder no row names goes; kept ones stay; a link in
/// the folder is removed, never followed.
#[tokio::test]
async fn folders_no_row_names_are_swept_at_open() {
    let (temp, dir) = data_dir();
    let store = open(&dir, StoreConfig::default()).await;
    let session = running(&store).await;
    let kept = keep(&store, &dir, session, b"kept").await;
    store.close().await;
    drop(store);

    let root = dir.attachments_dir();
    let gone_session = root.join(SessionId::random().to_string());
    std::fs::create_dir_all(gone_session.join("x")).unwrap();
    let gone_attachment = root
        .join(session.to_string())
        .join(AttachmentId::random().to_string());
    std::fs::create_dir_all(&gone_attachment).unwrap();
    std::fs::write(gone_attachment.join("half.bin"), b"half").unwrap();
    std::fs::write(root.join("stray"), b"stray").unwrap();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("keep-me"), b"mine").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join(SessionId::random().to_string())).unwrap();

    let store = open(&dir, StoreConfig::default()).await;
    let left: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, vec![std::ffi::OsString::from(session.to_string())]);
    assert!(!gone_attachment.exists());
    assert!(outside.join("keep-me").exists(), "a link is not followed");
    assert_eq!(
        store.read_attachment(kept, 0, 16, None).await.unwrap().data,
        b"kept"
    );
}
