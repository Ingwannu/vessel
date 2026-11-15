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

use vessel_app_view::composer::sent::Sent;
use vessel_app_view::composer::tray::{Again, Begun, ItemKey, Origin, Problem};
use vessel_proto::conversation::MimeType;
use vessel_proto::{AttachmentName, AttachmentRef, Delivery, HarnessRef, OptionChoice, SessionId};

// The tasks need tokio's files and the node's connection, which a browser
// has not.
#[cfg(not(target_arch = "wasm32"))]
mod read;
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
    /// Send a file to its session beside the other commands; each step
    /// answered `Answer::Upload`.
    Send(Attach),
    /// Start a conversation without its first prompt, which follows once
    /// its files are in; answered `Event::Started` (or `Event::Failed`).
    Create(Create),
    /// A prompt with the files already sent for it.
    Prompt(WithFiles),
    /// Read a sent image back from its node beside the other commands;
    /// answered `Answer::Read`.
    Read(Sent),
    /// Resume an ended session without sending its pending attachment prompt yet.
    Wake(Wake),
}

/// What the link says of the files (`Event::Files`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// A session resumed before its files upload; failure is translated by the area.
    Woke {
        /// The session whose composer waits.
        session: SessionId,
        /// The node accepted the resume or the files remain in the tray.
        outcome: WakeOutcome,
    },
    /// A step of a composer's file on its way to its session.
    Upload {
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
    /// The machine.
    pub machine: String,
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
    /// The machine.
    pub machine: String,
    /// The session.
    pub session: SessionId,
    /// What was typed (may be empty when files go alone).
    pub text: String,
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
    /// Validated book identity, rather than a display label.
    pub machine: vessel_app_view::machine_book::Name,
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
