//! Why a transfer could not be opened, read, pulled or placed.

use std::io;
use std::path::PathBuf;

use vessel_harness::{LinkError, MemoryFilesError, RebaseError, SessionFilesError};
use vessel_proto::{BlobHash, ErrorCode, TransferId};

/// Why a transfer could not be opened, read, pulled or placed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TransferError {
    /// A terminal session: its harness keeps the conversation in a form
    /// Vessel does not resume.
    #[error("terminal sessions cannot be copied; only structured ones")]
    Terminal,
    /// The session's agent has not reported its conversation yet.
    #[error("the session's agent is still starting; there is no conversation to copy yet")]
    NotReady,
    /// A turn is running and the export was not forced.
    #[error(
        "a turn is running; wait for it to end, or force the export (it then copies the \
         conversation as far as it is written)"
    )]
    TurnInProgress,
    /// The session's files could not be found.
    #[error(transparent)]
    Files(#[from] SessionFilesError),
    /// The session's files are not what a copy may take.
    #[error(transparent)]
    Link(#[from] LinkError),
    /// The profile's memory could not be listed.
    #[error(transparent)]
    Memory(#[from] MemoryFilesError),
    /// A path could not be renamed for the target's working directory.
    #[error(transparent)]
    Rebase(#[from] RebaseError),
    /// The files could not be snapshot (over a limit, a name that cannot
    /// travel, a read failure).
    #[error(transparent)]
    Snapshot(#[from] vessel_fleet::SnapshotError),
    /// The files could not be placed.
    #[error(transparent)]
    Place(#[from] vessel_fleet::PlaceError),
    /// A manifest received from a source is over a limit.
    #[error("{what} is {size} bytes, over the limit of {limit} bytes")]
    Oversized {
        /// What: a file's path, or "the transfer".
        what: String,
        /// Its size.
        size: u64,
        /// The limit.
        limit: u64,
    },
    /// No open transfer of that id for this caller (or none at all).
    #[error("no open transfer {0}")]
    Unknown(TransferId),
    /// The transfer holds no file with that hash.
    #[error("transfer {transfer} has no file with hash {hash}")]
    NoFile {
        /// The transfer.
        transfer: TransferId,
        /// The hash asked for.
        hash: BlobHash,
    },
    /// [`MAX_OPEN`] transfers are open.
    #[error("{0} transfers are open on this node; close one, or wait for one to expire")]
    TooMany(usize),
    /// A working directory that is not absolute.
    #[error("{} is not an absolute directory", .0.display())]
    RelativeDirectory(PathBuf),
    /// A session cannot start in a directory this node does not have.
    #[error("{} is not a directory on this node", .0.display())]
    NoDirectory(PathBuf),
    /// A project to import into, with all of a profile's memory to copy.
    #[error("a project to import memory into needs a project to copy it from")]
    ProjectWithoutSource,
    /// The import would put a session where it already is.
    #[error("that conversation is already in this profile and directory")]
    AlreadyThere,
    /// The target profile's harness home does not exist: the harness has
    /// never run there, and an import does not make one.
    #[error(
        "profile `{profile}` has no harness home on this node yet ({}); run or log in to the \
         harness under it once first",
        home.display()
    )]
    NoHome {
        /// The profile.
        profile: vessel_proto::ProfileName,
        /// Where its home would be.
        home: PathBuf,
    },
    /// The source answered with another kind of transfer.
    #[error("the source answered with a {0} transfer")]
    WrongKind(&'static str),
    /// The staging directory could not be used.
    #[error("the transfer's staging directory: {0}")]
    Staging(#[source] io::Error),
    /// A fetched file could not be written, or is not the file (its hash).
    #[error("fetching a file")]
    Fetch(#[source] io::Error),
    /// The source sent more than the manifest or the limit allows.
    #[error("a fetched file is {0} bytes, over the manifest or the limit")]
    FetchedTooLarge(u64),
    /// A blocking task failed.
    #[error("a node task failed")]
    Task(#[source] tokio::task::JoinError),
}

impl TransferError {
    /// The wire code.
    pub(crate) const fn code(&self) -> ErrorCode {
        use vessel_fleet::{PlaceError, SnapshotError};
        match self {
            Self::Terminal
            | Self::Files(SessionFilesError::Unsupported { .. })
            | Self::Memory(MemoryFilesError::Unsupported { .. }) => ErrorCode::Unsupported,
            Self::NotReady
            | Self::TurnInProgress
            | Self::AlreadyThere
            | Self::NoHome { .. }
            | Self::Files(
                SessionFilesError::NotFound { .. } | SessionFilesError::Ambiguous { .. },
            )
            | Self::Link(
                LinkError::Refused { .. }
                | LinkError::SymbolicLink { .. }
                | LinkError::Conflict { .. },
            )
            | Self::Snapshot(SnapshotError::Path { .. })
            | Self::Place(
                PlaceError::Conflict { .. }
                | PlaceError::Blocked { .. }
                | PlaceError::Changed { .. },
            ) => ErrorCode::Conflict,
            Self::Snapshot(
                SnapshotError::FileTooLarge { .. } | SnapshotError::TotalTooLarge { .. },
            )
            | Self::Oversized { .. }
            | Self::RelativeDirectory(_)
            | Self::NoDirectory(_)
            | Self::ProjectWithoutSource
            | Self::Rebase(_)
            | Self::Files(
                SessionFilesError::InvalidId { .. } | SessionFilesError::InvalidDirectory { .. },
            )
            | Self::Memory(MemoryFilesError::InvalidDirectory { .. })
            | Self::Place(PlaceError::Path { .. }) => ErrorCode::InvalidRequest,
            Self::Unknown(_) | Self::NoFile { .. } => ErrorCode::NotFound,
            Self::TooMany(_) => ErrorCode::Unavailable,
            Self::Files(SessionFilesError::Io { .. })
            | Self::Link(LinkError::Io { .. })
            | Self::Memory(MemoryFilesError::Io { .. })
            | Self::Snapshot(SnapshotError::Io { .. })
            | Self::Place(PlaceError::Io { .. } | PlaceError::DamagedBlob { .. })
            | Self::WrongKind(_)
            | Self::Staging(_)
            | Self::Fetch(_)
            | Self::FetchedTooLarge(_)
            | Self::Task(_) => ErrorCode::Internal,
        }
    }
}
