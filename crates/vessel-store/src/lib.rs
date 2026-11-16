//! A node's durable state: its data directory and the session event log.
//!
//! - [`DataDir`] is the directory layout: `node.id` (stable identity),
//!   `token` (the bearer token clients present, ADR 0005), `node.key` (the
//!   node's secret key, ADR 0009), `vessel.db`, and the paths milestone 2
//!   adds (local socket, `bin/`, `logs/`).
//! - [`NodeLock`] keeps one node per data directory and records which
//!   process that is ([`NodeOwner`]).
//! - [`Store`] is the event log (ADR 0004): one `SQLite` database holding each
//!   session's metadata and every [`EventRecord`] it produced, so a client
//!   can resume from any sequence number even after the node restarts. It
//!   also keeps the paired devices (ADR 0010), each with only the hash of
//!   its credential ([`CredentialHash`]).
//!
//! This crate is the only place that knows the on-disk formats. It decodes
//! rows into `vessel-proto` types once, here, and nothing outside sees SQL or
//! stored JSON.
//!
//! [Decision Log]
//! - Purpose: history that survives restarts, paged reads by sequence
//!   number, bounded disk use, and no blocking I/O on async threads.
//! - Choice: `SQLite` in WAL mode (ADR 0004 compares it with JSONL files). One
//!   writer thread owns the only write connection and is fed by a bounded
//!   queue; it commits each batch of queued operations in one transaction
//!   (no `fsync` per event: WAL with `synchronous=NORMAL` syncs at
//!   checkpoints), isolating each operation in a savepoint so one bad append
//!   cannot fail its neighbours. Reads run on the blocking pool with their
//!   own connections, which WAL lets proceed while the writer commits.
//! - The `sessions` row is a projection of the log: status, end time and
//!   the harness session id are derived from the records as they are
//!   appended, so the summary can never disagree with the history.
//! - A stored session's place on the board (`SessionInfo::board`, protocol
//!   8, ADR 0019) is derived when its row is read, never stored: an ended
//!   session is in the archive once archived and in building as `exited`
//!   until then (ADR 0026), its last activity its end; a live one has none
//!   from the store (the node serves it from its runtime).
//! - A person's marks (protocol 10, ADR 0026) are columns of the row, and
//!   `unread` is derived when it is read, from the last record asking for
//!   attention (a projection of the log) and the last `read`. Deleting a
//!   session, or retention pruning it, removes its attachment folder once
//!   the deletion is committed.
//! - Attachments (protocol 10, ADR 0025): a row each and the bytes under
//!   `attachments/<session>/<id>/<name>`, put there and read back only as
//!   the regular file of the recorded size and SHA-256, never through a
//!   link (`attachments::files`); folders no row names are swept at open.
//! - Cost: a C build step (bundled `SQLite`), and writes are acknowledged per
//!   batch commit rather than immediately.
//!
//! [`EventRecord`]: vessel_proto::EventRecord

mod again;
mod agents_file;
mod attachments;
mod automation;
mod codec;
mod config;
mod data_dir;
mod devices;
mod error;
mod files;
mod forge;
mod inbox;
mod lock;
mod log;
mod marks;
mod node_key;
mod push;
mod push_key;
mod reader;
mod recovery;
mod registry;
mod schema;
mod search;
mod settings_file;
mod side_context;
mod store;
pub use store::{DiagnosticReadAdmission, DiagnosticSnapshotError};
mod subagents;
mod token;
mod usage;
mod usage_lookup;
mod workspace;
mod writer;

pub use again::EndedSession;
pub use agents_file::AgentsFileError;
pub use attachments::files::{AttachmentFileError, FileIdentity, ReadBack};
pub use attachments::{AttachmentUsage, KeptAttachment};
pub use automation::{DELIVERIES_KEPT, RUNS_KEPT, StoredAutomation};
pub use config::{ByteCount, Retention, StoreConfig};
pub use data_dir::{DataDir, ProfilesFile};
pub use devices::{CredentialHash, DeviceWindow, StoredDevice};
pub use error::{Column, Quantity, StoreError};
pub use forge::{MAX_SENT as MAX_FEEDBACK_SENT, StoredFeedback};
pub use lock::{KernelLock, KernelProbe, NodeLock, NodeOwner, StartLock};
pub use node_key::{NODE_KEY_BYTES, NodeKeySecret};
pub use push::{PushAttention, StoredPush};
pub use push_key::{PushKeyError, PushPrivateKey};
pub use settings_file::{SetAside, SettingsDecodeFailure, SettingsFileError};
pub use side_context::{LatestPlan, SideContext};
pub use store::{Opened, Store};
pub use token::{InvalidToken, NodeToken};
pub use workspace::{ApprovalChange, MAX_APPROVED_SETS, StoredWorkspace};

/// Explicit workspace registry records shared by create/lifecycle services.
pub use registry::{
    RegistryRecord, RegistryReservation, ReservationStage, StoredRegistry, WorkspaceProvision,
};

/// Lifetime logical registry reservation bounds; `SQLite` physical growth is separately managed.
pub use registry::{
    MAX_DESCRIPTOR_BYTES, MAX_INTENT_BYTES, MAX_RECOVERY_BYTES, MAX_RESOURCES, MAX_RETAINED_BYTES,
    MAX_WORKSPACES, RESERVED_BYTES_PER_RESOURCE, RESERVED_BYTES_PER_WORKSPACE, WORKSPACE_KEY_BYTES,
};
/// Pre-spawn resource admission shared by creation, scripts and lifecycle drains.
pub use registry::{
    PendingRegistryStart, RegistryResourceReadiness, RegistryResourceRun, RegistryStartState,
    WorkspaceStartLease,
};
