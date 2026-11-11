//! Uploads in progress (protocol 10's `attachment.begin` and
//! `attachment.write`): who holds each, how much has come, and their
//! bytes staged under the data directory's `uploads/` until `finish`
//! hands them to the store.
//!
//! Must not: let anyone but the upload's holder write to it or see it
//! (another caller gets `not_found`, as for an unknown id), take a piece
//! out of order or past the declared size, or hold more uploads, or more
//! declared bytes, than the bounds allow.
//!
//! [Decision Log]
//! - Purpose: a file of up to 50 MiB arrives in pieces of at most 1 MiB,
//!   from any transport, possibly with a lost reply in between.
//! - Choice: the node keeps one open staging file per upload and writes
//!   each piece at the end of it, hashing as it goes, so `finish` needs no
//!   second read to know the SHA-256 (the store then checks the file it
//!   put in place against it). A piece must start where the received
//!   bytes end; one that ends at or before it (sent again after a lost
//!   reply) is answered with how much there is and not written again (ADR
//!   0025); one that starts elsewhere is refused.
//! - Bounds: 16 uploads open on a node and 4 per holder (`unavailable`
//!   beyond, so one device cannot hold every place for the ten idle
//!   minutes an upload is kept); a declared size counts against its
//!   session's 100 MiB and the node's 4 GiB from `begin` until its row is
//!   in the store, so uploads begun together cannot pass a limit that
//!   each passes alone. Begins take one lock while they compare and add,
//!   so two cannot both take the last room.
//! - Lifetime: dropped after [`IDLE`] without a piece, at a failed
//!   `finish` that cannot succeed later (not an image), and at a restart
//!   (`clear`, before the node serves anything).
//! - Alternatives: the bytes in memory until `finish` (16 uploads of 50
//!   MiB); the staging file inside the session's attachment folder (a
//!   half-sent file would be the session's, and its folder's sweep would
//!   race the upload).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::conversation::MimeType;
use vessel_proto::{
    AttachmentKind, AttachmentLimits, AttachmentName, BlobChunk, SessionId, UploadId,
};
use vessel_store::Store;

use super::UploadError;
use super::staging::{Progress, discard, stage};
use crate::transfers::Holder;

/// How long an upload stays open without a piece (protocol 10).
pub(crate) const IDLE: Duration = Duration::from_secs(10 * 60);

/// How often idle uploads are looked for.
const SWEEP: Duration = Duration::from_secs(30);

/// What `attachment.begin` declares.
#[derive(Debug, Clone)]
pub(crate) struct Declared {
    pub(crate) session: SessionId,
    pub(crate) name: AttachmentName,
    pub(crate) mime: MimeType,
    pub(crate) kind: AttachmentKind,
    pub(crate) size: u64,
}

/// One open upload.
#[derive(Debug)]
pub(crate) struct Open {
    pub(crate) holder: Holder,
    pub(crate) declared: Declared,
    /// Its staging file.
    pub(crate) path: PathBuf,
    used: Mutex<Instant>,
    /// What has come; `None` once it was finished or dropped. A tokio
    /// lock: a piece is written while it is held (on the blocking pool),
    /// so pieces and `finish` take their turns in order.
    pub(crate) progress: tokio::sync::Mutex<Option<Progress>>,
}

impl Open {
    fn touch(&self) {
        *self.used.lock().unwrap_or_else(PoisonError::into_inner) = Instant::now();
    }

    fn idle_for(&self, now: Instant) -> Duration {
        now.saturating_duration_since(*self.used.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// The node's open uploads.
#[derive(Debug)]
pub(crate) struct Uploads {
    root: PathBuf,
    /// A `std` lock: held for a lookup, an insert or a sum, never across
    /// `.await`.
    open: Mutex<HashMap<UploadId, Arc<Open>>>,
    /// Held by a `begin` from its limits' check to its insert (it reads
    /// the store meanwhile, hence tokio's).
    beginning: tokio::sync::Mutex<()>,
    idle: Duration,
}

impl Uploads {
    pub(crate) fn new(root: PathBuf, idle: Duration) -> Self {
        Self {
            root,
            open: Mutex::new(HashMap::new()),
            beginning: tokio::sync::Mutex::new(()),
            idle,
        }
    }

    fn open(&self) -> MutexGuard<'_, HashMap<UploadId, Arc<Open>>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Open an upload of `declared` for `holder`, within the bounds: the
    /// node's places and the holder's, then the session's and the node's
    /// bytes, kept ones (`store`) and declared ones together.
    pub(crate) async fn begin(
        &self,
        store: &Store,
        holder: Holder,
        declared: Declared,
    ) -> Result<UploadId, UploadError> {
        let _one_at_a_time = self.beginning.lock().await;
        let kept = store.attachment_usage(declared.session).await?;
        let (open_all, open_session) = {
            let open = self.open();
            if open.len() >= AttachmentLimits::OPEN_UPLOADS {
                return Err(UploadError::Busy {
                    limit: AttachmentLimits::OPEN_UPLOADS,
                    yours: false,
                });
            }
            let yours = open
                .values()
                .filter(|upload| upload.holder == holder)
                .count();
            if yours >= AttachmentLimits::OPEN_UPLOADS_PER_CALLER {
                return Err(UploadError::Busy {
                    limit: AttachmentLimits::OPEN_UPLOADS_PER_CALLER,
                    yours: true,
                });
            }
            let declared_by = |same: &dyn Fn(&Open) -> bool| {
                open.values()
                    .filter(|upload| same(upload))
                    .fold(0_u64, |sum, upload| {
                        sum.saturating_add(upload.declared.size)
                    })
            };
            (
                declared_by(&|_| true),
                declared_by(&|upload| upload.declared.session == declared.session),
            )
        };
        // Saturating: a sum past `u64` is past every limit anyway.
        let session = kept
            .session
            .saturating_add(open_session)
            .saturating_add(declared.size);
        let limit = AttachmentLimits::DEFAULT.per_session;
        if session > limit {
            return Err(UploadError::OverLimit {
                what: "the session's attachments",
                size: session,
                limit,
            });
        }
        let node = kept
            .node
            .saturating_add(open_all)
            .saturating_add(declared.size);
        if node > AttachmentLimits::PER_NODE {
            return Err(UploadError::OverLimit {
                what: "this node's attachments",
                size: node,
                limit: AttachmentLimits::PER_NODE,
            });
        }
        let id = UploadId::random();
        let path = self.root.join(id.to_string());
        let (root, staging) = (self.root.clone(), path.clone());
        let file = tokio::task::spawn_blocking(move || stage(&root, &staging))
            .await
            .map_err(UploadError::Task)?
            .map_err(UploadError::Staging)?;
        let upload = Open {
            holder,
            declared,
            path,
            used: Mutex::new(Instant::now()),
            progress: tokio::sync::Mutex::new(Some(Progress::new(file))),
        };
        drop(self.open().insert(id, Arc::new(upload)));
        Ok(id)
    }

    /// `holder`'s upload `id`.
    pub(crate) fn held(&self, holder: Holder, id: UploadId) -> Result<Arc<Open>, UploadError> {
        self.open()
            .get(&id)
            .filter(|upload| upload.holder == holder)
            .cloned()
            .ok_or(UploadError::Unknown(id))
    }

    /// `attachment.write`: the piece at `offset`; how much has come after.
    pub(crate) async fn write(
        &self,
        holder: Holder,
        id: UploadId,
        offset: u64,
        data: bytes::Bytes,
    ) -> Result<u64, UploadError> {
        let upload = self.held(holder, id)?;
        upload.touch();
        if data.len() > BlobChunk::MAX_LEN {
            return Err(UploadError::PieceTooLarge {
                len: data.len(),
                limit: BlobChunk::MAX_LEN,
            });
        }
        // Held across the write, so the next piece and `finish` wait for
        // this one: their order is the file's.
        let mut progress = upload.progress.lock().await;
        let Some(received) = progress.as_ref().map(|progress| progress.received) else {
            return Err(UploadError::Unknown(id));
        };
        let len = u64::try_from(data.len()).unwrap_or(u64::MAX);
        let end = offset.saturating_add(len);
        if end <= received {
            // Sent again after a lost reply: it is in already.
            return Ok(received);
        }
        if offset != received {
            return Err(UploadError::OutOfOrder { offset, received });
        }
        if end > upload.declared.size {
            return Err(UploadError::PastSize {
                end,
                size: upload.declared.size,
            });
        }
        let Some(taken) = progress.take() else {
            return Err(UploadError::Unknown(id));
        };
        let written = tokio::task::spawn_blocking(move || taken.write(&data))
            .await
            .map_err(UploadError::Task)?;
        match written {
            Ok(written) => {
                let received = written.received;
                *progress = Some(written);
                upload.touch();
                Ok(received)
            }
            Err(error) => {
                // A staging file that cannot take a piece will not take
                // the next: the upload is dropped.
                drop(progress);
                self.drop_upload(id).await;
                Err(UploadError::Staging(error))
            }
        }
    }

    /// Remove upload `id` and its staging file (if it is still there).
    pub(crate) async fn drop_upload(&self, id: UploadId) {
        let removed = self.open().remove(&id);
        if let Some(upload) = removed {
            let path = upload.path.clone();
            if let Err(error) = tokio::task::spawn_blocking(move || discard(&path)).await {
                tracing::warn!(%error, "removing an upload's staging file failed");
            }
        }
    }

    /// Forget upload `id` once it is finished (its file moved away).
    pub(crate) fn forget(&self, id: UploadId) {
        drop(self.open().remove(&id));
    }

    /// Drop every upload idle for longer than allowed at `now`. Blocks.
    fn expire(&self, now: Instant) {
        let expired: Vec<Arc<Open>> = {
            let mut open = self.open();
            let ids: Vec<UploadId> = open
                .iter()
                .filter(|(_, upload)| upload.idle_for(now) > self.idle)
                .map(|(id, _)| *id)
                .collect();
            ids.iter().filter_map(|id| open.remove(id)).collect()
        };
        for upload in expired {
            tracing::info!(session = %upload.declared.session, "an idle upload was dropped");
            discard(&upload.path);
        }
    }

    /// Drop idle uploads every little while until `stop`.
    pub(crate) fn sweep(self: &Arc<Self>, tasks: &TaskTracker, stop: &CancellationToken) {
        let uploads = Arc::clone(self);
        let stop = stop.clone();
        let every = SWEEP.min(self.idle);
        drop(tasks.spawn(async move {
            loop {
                tokio::select! {
                    () = stop.cancelled() => return,
                    () = tokio::time::sleep(every) => {}
                }
                let uploads = Arc::clone(&uploads);
                if let Err(error) =
                    tokio::task::spawn_blocking(move || uploads.expire(Instant::now())).await
                {
                    tracing::warn!(%error, "dropping idle uploads failed");
                }
            }
        }));
    }
}

#[cfg(test)]
mod tests {
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
}
