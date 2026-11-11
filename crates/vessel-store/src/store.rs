//! The public handle to the event log.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};
use vessel_proto::{
    AttachmentId, Conversation, DeviceId, DeviceInfo, EventPage, EventRecord, EventSeq, KeptMarks,
    Mark, OptionChoice, SessionId, SessionInfo,
};

use crate::again::EndedSession;
use crate::attachments::files::{FileIdentity, ReadBack};
use crate::attachments::{AttachmentUsage, KeptAttachment};
use crate::config::StoreConfig;
use crate::data_dir::DataDir;
use crate::devices::{CredentialHash, StoredDevice};
use crate::error::StoreError;
use crate::log;
use crate::reader::Reader;
use crate::writer::{self, Done, Setup, Write, WriteOp};
use crate::{attachments, files, recovery, schema};

/// The session event log. Cheap to clone; every clone talks to the same
/// writer thread and read pool.
#[derive(Debug, Clone)]
pub struct Store {
    writes: mpsc::Sender<WriteOp>,
    reader: Arc<Reader>,
    /// The data directory's `attachments/` (protocol 10).
    attachments: Arc<Path>,
}

/// A freshly opened store and what recovery found.
#[derive(Debug)]
pub struct Opened {
    /// The store.
    pub store: Store,
    /// Sessions that were live when the node last stopped, now marked
    /// `Interrupted` (their logs end with that status). Newest first.
    pub interrupted: Vec<SessionInfo>,
}

impl Store {
    /// Open (or create) the event log in `dir`, migrate it, and recover from
    /// the previous run: every session that was still live is closed with a
    /// final `Status{Interrupted}` record, then retention runs (removing
    /// the attachment folders of the sessions it prunes), and every
    /// attachment folder no row names is removed.
    ///
    /// # Errors
    /// The database cannot be opened or migrated (for example it was written
    /// by a newer Vessel), or the writer thread cannot start.
    pub async fn open(dir: &DataDir, config: StoreConfig) -> Result<Opened, StoreError> {
        let path = dir.database_path();
        let retention = config.retention;
        let opening = path.clone();
        let folders = dir.attachments_dir();
        let (connection, interrupted) = tokio::task::spawn_blocking(move || {
            files::private_database(&opening).map_err(StoreError::io(&opening))?;
            let mut connection = Connection::open(&opening)?;
            schema::configure_writer(&connection)?;
            schema::migrate(&mut connection)?;
            let interrupted = recovery::interrupt_live(&mut connection, Utc::now())?;
            // Outside a transaction: each deletion is committed as it runs.
            for session in log::prune_ended(&connection, &retention)? {
                attachments::remove_folder(&folders, session);
            }
            attachments::sweep(&connection, &folders)?;
            Ok::<_, StoreError>((connection, interrupted))
        })
        .await
        .map_err(StoreError::Task)??;
        if !interrupted.is_empty() {
            tracing::info!(
                sessions = interrupted.len(),
                "marked sessions from the previous run as interrupted"
            );
        }

        let (writes, ops) = mpsc::channel(config.write_queue.get());
        let setup = Setup {
            retention,
            attachments: dir.attachments_dir(),
        };
        writer::spawn(connection, ops, setup)?;
        let store = Self {
            writes,
            reader: Arc::new(Reader::new(path, config)),
            attachments: Arc::from(dir.attachments_dir()),
        };
        Ok(Opened { store, interrupted })
    }

    /// Store a new session with an empty log.
    ///
    /// # Errors
    /// The id is already stored, or the write failed.
    pub async fn create_session(&self, info: SessionInfo) -> Result<(), StoreError> {
        self.write(|done| Write::Create {
            info: Box::new(info),
            done,
        })
        .await
    }

    /// Append records to a session's log; returns once they are committed.
    /// They must continue the log exactly (next sequence number, no gaps).
    /// Records are taken as `Arc`s, the form the session runtime shares them
    /// in, so persisting them clones nothing.
    ///
    /// Status, end time and harness session id of the stored session follow
    /// from the records (`Status` and `Agent{Ready}` events).
    ///
    /// # Errors
    /// The session is unknown, the records do not continue its log, or the
    /// write failed. A failed append leaves the log unchanged.
    pub async fn append(
        &self,
        session: SessionId,
        records: Vec<Arc<EventRecord>>,
    ) -> Result<(), StoreError> {
        self.write(|done| Write::Append {
            session,
            records,
            done,
        })
        .await
    }

    /// A page of a session's log; see `session.read` in the protocol.
    ///
    /// # Errors
    /// The session is unknown, or the read failed.
    pub async fn read_events(
        &self,
        session: SessionId,
        from: EventSeq,
        limit: u32,
    ) -> Result<EventPage, StoreError> {
        self.read(move |reader| reader.read_events(session, from, limit))
            .await
    }

    /// One stored session.
    ///
    /// # Errors
    /// The read failed.
    pub async fn session(&self, id: SessionId) -> Result<Option<SessionInfo>, StoreError> {
        self.read(move |reader| reader.session(id)).await
    }

    /// The agent options a stored session last ran with, as choices
    /// (protocol 9, schema 6): what an export carries on. `None` before
    /// its first options report, and for an unknown session.
    ///
    /// # Errors
    /// The read failed, or the stored value is corrupt.
    pub async fn choices(&self, id: SessionId) -> Result<Option<Vec<OptionChoice>>, StoreError> {
        self.read(move |reader| reader.choices(id)).await
    }

    /// A stored session's conversation as its log left it (protocol 10's
    /// `session.conversation` for an ended session); `None` for an unknown
    /// session.
    ///
    /// # Errors
    /// The read failed, or a stored value is corrupt.
    pub async fn conversation(&self, id: SessionId) -> Result<Option<Conversation>, StoreError> {
        self.read(move |reader| reader.conversation(id)).await
    }

    /// Set a mark on a session (protocol 10's `session.mark`, ADR 0026),
    /// made at `at`; returns its marks after. `read` keeps the furthest
    /// `through` and clears `unread`; `archive` keeps the first time.
    /// Nothing checks that an archived session has ended: the node stops
    /// a session before archiving it.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`], or the write failed.
    pub async fn mark(
        &self,
        session: SessionId,
        mark: Mark,
        at: DateTime<Utc>,
    ) -> Result<KeptMarks, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Mark {
            session,
            mark,
            at,
            done,
        })
        .await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }

    /// Delete an ended session (protocol 10's `session.delete`): its log,
    /// its marks, its queue, its attachment rows, and its folder of
    /// attachment files, which is gone when this returns.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`], [`StoreError::SessionLive`] while
    /// the store still shows it live, or the write failed.
    pub async fn delete_session(&self, session: SessionId) -> Result<(), StoreError> {
        self.write(|done| Write::Delete { session, done }).await
    }

    /// A stored session with what a new run of it continues from (protocol
    /// 10's `session.resume`, ADR 0026): its info, choices, queue, last
    /// context report and attention point, read as one state of its row.
    /// `None` for an unknown session.
    ///
    /// # Errors
    /// The read failed, or a stored value is corrupt.
    pub async fn ended_session(&self, id: SessionId) -> Result<Option<EndedSession>, StoreError> {
        self.read(move |reader| reader.ended_session(id)).await
    }

    /// Reopen an ended session's row for a new run whose first record is
    /// `next` (the `next_seq` the caller read): live, no end, out of the
    /// archive, before the run publishes anything, so retention never
    /// prunes a session that is starting again and its log continues after
    /// its final status. Returns its marks after.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`] once deleted or pruned,
    /// [`StoreError::SessionLive`] while the row is still live,
    /// [`StoreError::OutOfOrder`] when its log no longer ends at `next`,
    /// or the write failed. A refusal changes nothing.
    pub async fn reopen(
        &self,
        session: SessionId,
        next: EventSeq,
    ) -> Result<KeptMarks, StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(Write::Reopen {
            session,
            next,
            done,
        })
        .await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }

    /// Keep a finished upload as `kept` (protocol 10, ADR 0025): `staged`,
    /// the upload's received bytes, is moved to the attachment's place
    /// under the data directory and checked against `kept`'s size and
    /// SHA-256, then its row is recorded at `at`. A refused row takes the
    /// file back; `staged` is gone either way once the move was made.
    ///
    /// # Errors
    /// [`StoreError::UnknownSession`], [`StoreError::SessionEnded`] once
    /// its session ended (or was removed), [`StoreError::Attachment`] when
    /// the file could not be put in place or is not what was received, or
    /// the write failed.
    pub async fn keep_attachment(
        &self,
        kept: KeptAttachment,
        staged: PathBuf,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let root = Arc::clone(&self.attachments);
        let placing = kept.clone();
        let placed = tokio::task::spawn_blocking(move || {
            attachments::files::place(&root, &placing, &staged)
        })
        .await
        .map_err(StoreError::Task)??;
        let recorded = self
            .write(|done| Write::Attach {
                kept: Box::new(kept),
                at,
                done,
            })
            .await;
        if recorded.is_err() {
            let removed = tokio::task::spawn_blocking(move || placed.remove()).await;
            if let Err(error) = removed {
                tracing::warn!(%error, "could not take back an attachment not kept");
            }
        }
        recorded
    }

    /// `session`'s attachment `id`, if the store keeps it (protocol 10).
    ///
    /// # Errors
    /// The read failed, or the row is corrupt.
    pub async fn attachment(
        &self,
        session: SessionId,
        id: AttachmentId,
    ) -> Result<Option<KeptAttachment>, StoreError> {
        self.read(move |reader| reader.attachment(session, id))
            .await
    }

    /// How many bytes of attachments the node keeps, and `session` (protocol
    /// 10's limits).
    ///
    /// # Errors
    /// The read failed.
    pub async fn attachment_usage(
        &self,
        session: SessionId,
    ) -> Result<AttachmentUsage, StoreError> {
        self.read(move |reader| reader.attachment_usage(session))
            .await
    }

    /// Up to `max` bytes of `kept` from `from`, read back only from the
    /// regular file of its recorded size and SHA-256, never through a link
    /// (ADR 0025); `max` 0 only checks it. A file still of identity
    /// `verified` is not hashed again.
    ///
    /// # Errors
    /// [`StoreError::Attachment`]: the file is missing, was changed, or
    /// could not be read.
    pub async fn read_attachment(
        &self,
        kept: KeptAttachment,
        from: u64,
        max: usize,
        verified: Option<FileIdentity>,
    ) -> Result<ReadBack, StoreError> {
        let root = Arc::clone(&self.attachments);
        tokio::task::spawn_blocking(move || {
            attachments::files::read(&root, &kept, from, max, verified)
        })
        .await
        .map_err(StoreError::Task)?
        .map_err(StoreError::from)
    }

    /// Where `kept`'s bytes are: what an agent is given to read a file
    /// attachment with its own tools.
    #[must_use]
    pub fn attachment_path(&self, kept: &KeptAttachment) -> PathBuf {
        attachments::files::path(&self.attachments, kept)
    }

    /// Every stored session, newest first.
    ///
    /// # Errors
    /// The read failed.
    pub async fn sessions(&self) -> Result<Vec<SessionInfo>, StoreError> {
        self.read(Reader::sessions).await
    }

    /// Store a newly paired device with its credential's hash.
    ///
    /// # Errors
    /// The id is already stored, or the write failed.
    pub async fn add_device(
        &self,
        device: DeviceInfo,
        credential: CredentialHash,
    ) -> Result<(), StoreError> {
        self.write(|done| Write::AddDevice {
            device,
            credential,
            done,
        })
        .await
    }

    /// Note that `device` authenticated at `at` (`last_seen`).
    ///
    /// # Errors
    /// The write failed.
    pub async fn device_seen(&self, device: DeviceId, at: DateTime<Utc>) -> Result<(), StoreError> {
        self.write(|done| Write::DeviceSeen { device, at, done })
            .await
    }

    /// Revoke `device` at `at`. It stays listed, with `revoked_at` set;
    /// revoking it again keeps the first time.
    ///
    /// # Errors
    /// [`StoreError::UnknownDevice`] if there is no such device, or the
    /// write failed.
    pub async fn revoke_device(
        &self,
        device: DeviceId,
        at: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.write(|done| Write::RevokeDevice { device, at, done })
            .await
    }

    /// Every paired device, revoked ones included, oldest first.
    ///
    /// # Errors
    /// The read failed.
    pub async fn devices(&self) -> Result<Vec<DeviceInfo>, StoreError> {
        self.read(Reader::devices).await
    }

    /// One device with its credential hash, for authentication.
    ///
    /// # Errors
    /// The read failed.
    pub async fn device(&self, id: DeviceId) -> Result<Option<StoredDevice>, StoreError> {
        self.read(move |reader| reader.device(id)).await
    }

    /// Commit everything queued so far, fold the WAL into the database file
    /// and stop the writer. Later writes through any clone fail with
    /// [`StoreError::Closed`]; reads keep working.
    pub async fn close(&self) {
        let (done, closed) = oneshot::channel();
        if self.writes.send(WriteOp::Close { done }).await.is_ok() && closed.await.is_err() {
            tracing::debug!("event log writer stopped before acknowledging close");
        }
    }

    async fn write(&self, write: impl FnOnce(Done) -> Write) -> Result<(), StoreError> {
        let (done, committed) = oneshot::channel();
        self.send(write(done)).await?;
        committed.await.map_err(|_dropped| StoreError::Closed)?
    }

    /// Queue `write` for the writer; its answer comes on its own channel.
    async fn send(&self, write: Write) -> Result<(), StoreError> {
        self.writes
            .send(WriteOp::Write(write))
            .await
            .map_err(|_closed| StoreError::Closed)
    }

    async fn read<T: Send + 'static>(
        &self,
        read: impl FnOnce(&Reader) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let reader = Arc::clone(&self.reader);
        tokio::task::spawn_blocking(move || read(&reader))
            .await
            .map_err(StoreError::Task)?
    }
}
