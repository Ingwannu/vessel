//! The public handle to the event log. Protocol 10's session calls are
//! `conversation`'s, attachments `attached`'s.

mod attached;
mod automation;
#[cfg(test)]
mod checkpoints;
mod conversation;
mod forge;
mod inbox;
mod push;
mod search;
mod usage;

use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot, watch};
use vessel_proto::workspace::{CommandsDigest, ProjectInfo, WorktreeInfo};
use vessel_proto::{
    DeviceId, DeviceInfo, EventPage, EventRecord, EventSeq, OptionChoice, ProjectId, SessionId,
    SessionInfo, WorktreeId,
};

use crate::config::StoreConfig;
use crate::data_dir::DataDir;
use crate::devices::{CredentialHash, StoredDevice};
use crate::error::StoreError;
use crate::log;
use crate::reader::Reader;
use crate::workspace::{ApprovalChange, StoredWorkspace, WorkspaceWrite};
use crate::writer::{self, Done, Setup, Write, WriteOp};
use crate::{attachments, files, recovery, schema};

/// The session event log. Cheap to clone; every clone talks to the same
/// writer thread and read pool.
#[derive(Debug, Clone)]
pub struct Store {
    writes: mpsc::Sender<WriteOp>,
    reader: Arc<Reader>,
    /// Shared admission for expensive usage scans, not the idle reader cache.
    usage_reads: Arc<tokio::sync::Semaphore>,
    /// The data directory's `attachments/` (protocol 10).
    attachments: Arc<Path>,
    /// How many sessions retention has removed since the store opened
    /// (protocol 10, [`Store::pruned`]).
    pruned: watch::Receiver<u64>,
    inbox: watch::Receiver<()>,
    /// Whether some session's messages are not in the search index yet
    /// (protocol 12, [`Store::indexed`]).
    indexing: watch::Receiver<bool>,
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
            #[cfg(test)]
            crate::checkpoint::retention::startup(&mut connection)?;
            let interrupted = recovery::interrupt_live(&mut connection, Utc::now())?;
            // Checkpoint garbage transfer and its parent session deletion must
            // commit together, including this startup path before the writer.
            let pruned = {
                let transaction = connection.transaction()?;
                let pruned = log::prune_ended(&transaction, &retention)?;
                transaction.commit()?;
                pruned
            };
            for session in pruned {
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
        let (pruned, pruned_count) = watch::channel(0);
        let (inbox, inbox_changes) = watch::channel(());
        let (indexing_sender, indexing) = watch::channel(true);
        let setup = Setup {
            retention,
            attachments: dir.attachments_dir(),
            pruned,
            inbox,
            indexing: indexing_sender,
        };
        writer::spawn(connection, ops, setup)?;
        let store = Self {
            writes,
            reader: Arc::new(Reader::new(path, config)),
            usage_reads: Arc::new(tokio::sync::Semaphore::new(usage::MAX_READS)),
            attachments: Arc::from(dir.attachments_dir()),
            pruned: pruned_count,
            inbox: inbox_changes,
            indexing,
        };
        Ok(Opened { store, interrupted })
    }

    /// How many sessions retention has removed since the store opened: it
    /// changes after each commit that removed some (protocol 10), so the
    /// node can tidy what they left outside the store (their worktrees).
    /// The ones the opening's own retention removed are not counted; the
    /// node looks for those when it starts.
    #[must_use]
    pub fn pruned(&self) -> watch::Receiver<u64> {
        self.pruned.clone()
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

    /// At most `DeviceWindow::MAX_ITEMS` records after an id, in id order.
    /// `more` explicitly distinguishes an incomplete window from a full list.
    ///
    /// # Errors
    /// The read failed or a record exceeds pairing's metadata bounds.
    pub async fn device_window(
        &self,
        after: Option<DeviceId>,
    ) -> Result<crate::DeviceWindow, StoreError> {
        self.guarded_device_window(after, ())
            .await
            .map(|(window, ())| window)
    }

    /// Read a device window while retaining the caller's admission guard
    /// inside the blocking operation, then hand it back. Dropping the async
    /// request never releases that guard while queued/running I/O remains.
    /// The node uses this for its status permit; no node type enters storage.
    ///
    /// # Errors
    /// The same failures as [`Self::device_window`].
    pub async fn guarded_device_window<G: Send + 'static>(
        &self,
        after: Option<DeviceId>,
        guard: G,
    ) -> Result<(crate::DeviceWindow, G), StoreError> {
        self.read(move |reader| Ok((reader.device_window(after)?, guard)))
            .await
    }

    /// One device with its credential hash, for authentication.
    ///
    /// # Errors
    /// The read failed.
    pub async fn device(&self, id: DeviceId) -> Result<Option<StoredDevice>, StoreError> {
        self.read(move |reader| reader.device(id)).await
    }

    /// Projects, worktrees and approved command sets (protocol 10).
    ///
    /// # Errors
    /// The read failed.
    pub async fn workspace(&self) -> Result<StoredWorkspace, StoreError> {
        self.read(Reader::workspace).await
    }

    /// Insert or replace a project.
    ///
    /// # Errors
    /// The write failed.
    pub async fn put_project(&self, project: ProjectInfo) -> Result<(), StoreError> {
        self.workspace_write(WorkspaceWrite::Project(Box::new(project)))
            .await
    }

    /// Delete a project and its approvals (never one with worktrees: the
    /// node refuses first, and the database refuses too).
    ///
    /// # Errors
    /// The write failed.
    pub async fn forget_project(&self, project: ProjectId) -> Result<(), StoreError> {
        self.workspace_write(WorkspaceWrite::ForgetProject(project))
            .await
    }

    /// Insert or replace a worktree.
    ///
    /// # Errors
    /// The write failed.
    pub async fn put_worktree(&self, worktree: WorktreeInfo) -> Result<(), StoreError> {
        self.workspace_write(WorkspaceWrite::Worktree(Box::new(worktree)))
            .await
    }

    /// Delete a worktree.
    ///
    /// # Errors
    /// The write failed.
    pub async fn forget_worktree(&self, worktree: WorktreeId) -> Result<(), StoreError> {
        self.workspace_write(WorkspaceWrite::ForgetWorktree(worktree))
            .await
    }

    /// Approve the command set `digest` for `project` (with the set, from
    /// schema 11), or withdraw it.
    ///
    /// # Errors
    /// The write failed.
    pub async fn set_approval(
        &self,
        project: ProjectId,
        digest: CommandsDigest,
        change: ApprovalChange,
    ) -> Result<(), StoreError> {
        self.workspace_write(WorkspaceWrite::Approval {
            project,
            digest,
            change,
        })
        .await
    }

    async fn workspace_write(&self, write: WorkspaceWrite) -> Result<(), StoreError> {
        self.write(|done| Write::Workspace { write, done }).await
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

    pub(crate) async fn read<T: Send + 'static>(
        &self,
        read: impl FnOnce(&Reader) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let reader = Arc::clone(&self.reader);
        tokio::task::spawn_blocking(move || read(&reader))
            .await
            .map_err(StoreError::Task)?
    }
}
