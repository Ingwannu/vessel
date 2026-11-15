//! A composer's file sent to its session (track P's row c): begun, written
//! a piece at a time from what the node says it has, finished, and each
//! step told to the app (`Event::Files`, `Answer::Upload`). Runs beside the
//! link's other commands, as a 50 MiB file takes a while. A sent image is
//! read back the same way, a piece at a time (`read`, `Answer::Read`).
//!
//! [Decision Log]
//! - Purpose: the composer's attached files reach the node before the
//!   prompt that names them (ADR 0025).
//! - Followed: the CLI's upload (`vessel attach`: a buffer per piece, the
//!   file read as it is sent, never whole) and the page's
//!   (`web/src/data/upload.ts`: the next offset is the node's `received`,
//!   a retry goes on from what the node has, only an upload the node
//!   dropped is begun again).
//! - Constraint: the node takes at most its `chunk` a piece and answers
//!   each with what it has; protocol 10 has no call to abandon an upload
//!   (the node drops one idle for ten minutes).
//! - Choice: one task per file; an image's first bytes are checked against
//!   its type before anything is begun (the node would refuse it after the
//!   whole file was sent); a failure says whether, and from where, it may
//!   be tried again (`Again`).
//! - Cost: one round trip a piece.

use std::sync::Arc;

pub use vessel_app_view::composer::attach::TargetProblem;
use vessel_app_view::composer::sent::Sent;
use vessel_app_view::composer::tray::{Again, Begun, ItemKey, Origin, Problem};
use vessel_app_view::machine_book::Name;
use vessel_proto::conversation::MimeType;
use vessel_proto::{AttachmentName, AttachmentRef, Delivery, HarnessRef, OptionChoice, SessionId};

// The tasks need tokio's files and the node's connection, which a browser
// has not.
#[cfg(not(target_arch = "wasm32"))]
mod capabilities;
#[cfg(not(target_arch = "wasm32"))]
pub mod local;
#[cfg(not(target_arch = "wasm32"))]
mod read;
#[cfg(not(target_arch = "wasm32"))]
mod submit;
#[cfg(not(target_arch = "wasm32"))]
mod target;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use capabilities::spawn as spawn_capabilities;
#[cfg(not(target_arch = "wasm32"))]
mod send;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use read::spawn as spawn_read;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use send::{content, spawn};

/// What a composer's files ask of the link (one command, so the link's
/// command loop stays one screen of dispatch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Files {
    /// File/root suggestions share this feature's single transport registration.
    Suggest(crate::mentions::Ask),
    /// Read this created session's own abilities and limits without selecting it.
    Capabilities(CapabilityAsk),
    /// Send a file to its session beside the other commands; each step
    /// answered `Answer::Upload`.
    Send(Attach),
    /// Start a conversation without its first prompt, which follows once
    /// its files are in; answered `Event::Started` (or `Event::Failed`).
    Create(Create),
    /// A prompt with the files already sent for it.
    Prompt(Arc<WithFiles>),
    /// Read a sent image back from its node beside the other commands;
    /// answered `Answer::Read`.
    Read(Sent),
    /// Resume an ended session without sending its pending attachment prompt yet.
    Wake(Wake),
}

/// What the link says of the files (`Event::Files`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Suggestions for a captured composer/token.
    Suggestions(crate::mentions::Answer),
    /// An existing session's registered root, with captured identity.
    Root(crate::mentions::RootAnswer),
    /// Capabilities belong to the captured creation ticket and session.
    Capabilities {
        /// Original destination.
        ask: CapabilityAsk,
        /// Authoritative conversation facts or a typed refusal.
        result: Result<vessel_app_view::composer::tray::Target, CapabilityFailure>,
    },
    /// A final immutable payload remains retained until this exact completion.
    Submitted {
        /// The same local ownership handle, not a wire idempotency identity.
        prompt: Arc<WithFiles>,
        /// The existing prompt result; uncertainty never authorizes replay.
        result: Result<vessel_proto::Delivered, SubmitFailure>,
    },
    /// A session resumed before its files upload; failure is translated by the area.
    Woke {
        /// Original immutable submission owner, before asynchronous resume.
        message: vessel_proto::timeline::ClientMessageId,
        /// The session whose composer waits.
        session: SessionId,
        /// The node accepted the resume or the files remain in the tray.
        outcome: WakeOutcome,
    },
    /// A step of a composer's file on its way to its session.
    Upload {
        /// An old tray operation cannot settle a newer submission.
        message: vessel_proto::timeline::ClientMessageId,
        /// The session.
        session: SessionId,
        /// The file's place in the composer's tray.
        key: ItemKey,
        /// What happened.
        step: Step,
    },
    /// A sent image read back: its bytes, or `None` when it could not be
    /// read (the log says why).
    Read {
        /// Which image.
        sent: Sent,
        /// Its bytes.
        bytes: Option<Arc<[u8]>>,
    },
}

/// A conversation to start whose first prompt waits for its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Create {
    /// The machine.
    pub machine: String,
    /// The agent.
    pub harness: HarnessRef,
    /// The folder on the machine; `None` for its home.
    pub folder: Option<String>,
    /// The agent's options to start with.
    pub options: Vec<OptionChoice>,
}

/// One file to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attach {
    /// Same local submission identity retained through upload and retry.
    pub message: vessel_proto::timeline::ClientMessageId,
    /// The machine.
    pub machine: Name,
    /// The immutable welcome identity admitted before preparation.
    pub expected_node: vessel_proto::NodeId,
    /// The session it goes to.
    pub session: SessionId,
    /// Its place in the composer's tray.
    pub key: ItemKey,
    /// Its name.
    pub name: AttachmentName,
    /// Its type.
    pub mime: MimeType,
    /// Its size in bytes, as it was attached.
    pub size: u64,
    /// Where its bytes are.
    pub origin: Origin,
    /// The upload the node already holds, to go on with.
    pub resume: Option<Begun>,
}

/// A step of a file's upload, told to the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// The node has this many bytes.
    Progress(u64),
    /// In: the reference a prompt names it by.
    Done(AttachmentRef),
    /// It failed.
    Failed {
        /// Why.
        problem: Problem,
        /// Whether, and from where, it may be tried again.
        again: Again,
    },
}

/// A prompt with the files already sent for it (queued when a turn runs):
/// its text, then each file, as the CLI sends them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithFiles {
    /// Allocated once before preparation; retained through every upload/result.
    /// This becomes the canonical wire key only after identified RPC activation.
    pub message: vessel_proto::timeline::ClientMessageId,
    /// The machine.
    pub machine: Name,
    /// The immutable welcome identity admitted before preparation.
    pub expected_node: vessel_proto::NodeId,
    /// The session.
    pub session: SessionId,
    /// What was typed (may be empty when files go alone).
    pub text: String,
    /// The immutable context captured before asynchronous preparation.
    pub context: Option<vessel_app_view::composer::context::Submission>,
    /// The files, as `attachment.finish` returned them.
    pub attachments: Vec<AttachmentRef>,
    /// What it does while a turn runs.
    pub delivery: Delivery,
}

/// Why an image download was refused before its bytes reached the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadProblem {
    /// The connected peer did not finish within the named operation deadline.
    TimedOut,
    /// Zero bytes cannot decode as an image.
    Empty,
    /// The source reports an image above the node's bound.
    TooLarge,
    /// The total changed between chunks.
    Changed,
    /// A chunk crosses the reported total.
    Overrun,
    /// No progress before the end of the image.
    Stopped,
}

/// An existing session must be live before the node will accept files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wake {
    /// Same local submission identity retained while resume is asynchronous.
    pub message: vessel_proto::timeline::ClientMessageId,
    /// Validated book identity, rather than a display label.
    pub machine: Name,
    /// The immutable welcome identity admitted before preparation.
    pub expected_node: vessel_proto::NodeId,
    /// The original conversation, whose log continues after resume.
    pub session: SessionId,
}

/// No prompt is sent during resume; a refusal leaves the tray recoverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeOutcome {
    /// Resume succeeded; follow the session before validating file capabilities.
    Ready,
    /// The node or link refused the resume.
    Refused,
}

/// Creation correlation remains independent of the selected conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAsk {
    /// The validated node book key.
    pub machine: Name,
    /// The immutable welcome identity admitted before preparation.
    pub expected_node: vessel_proto::NodeId,
    /// The actual created session, even when it is not selected.
    pub session: SessionId,
    /// The local creation which retained the files.
    pub ticket: vessel_app_view::creation::Ticket,
}
/// Read failures do not fabricate an agent capability or erase retained files.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityFailure {
    /// The captured address no longer reaches the admitted peer.
    #[error(transparent)]
    Target(#[from] TargetProblem),
    /// All bounded preparation slots are retained by other submissions.
    #[error("attachment preparation capacity reached")]
    Capacity,
    /// No connected peer accepted the read.
    #[error("attachment capability peer is offline")]
    Offline,
    /// The handshake/read did not finish within its bound.
    #[error("attachment capability read timed out")]
    TimedOut,
    /// Preserve the node/transport cause; no message-string dispatch.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Call(#[from] vessel_rpc::CallError),
}

/// A refused local dispatch is distinct from a call with an unknown outcome.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitFailure {
    /// No request was admitted by the local link.
    #[error("the attachment prompt was not sent")]
    NotSent,
    /// The captured peer changed before the call; no request was sent.
    #[error(transparent)]
    Target(#[from] TargetProblem),
    /// The call reached transport but its final result is not known.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    Uncertain(#[from] vessel_rpc::CallError),
    /// A transport deadline cannot prove whether the node accepted the input.
    #[error("the attachment prompt outcome is unknown after its deadline")]
    TimedOut,
}
