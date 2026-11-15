//! The command keeps concrete causes private because main renders whole source chains.
use super::file::FileError;
use std::{fmt, io};
use vessel_client::CallError;
use vessel_proto::{
    Method,
    dictation::{ChunkSeq, DictationReason},
};

#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("dictation input was refused: {0}")]
    File(#[from] FileError),
    #[error("dictation call `{}` did not complete", method.as_str())]
    Call {
        method: Method,
        #[source]
        source: CallError,
    },
    #[error("dictation stopped without a final transcript")]
    Stream(DictationReason),
    #[error("the node returned an unknown dictation update")]
    UnknownUpdate,
    #[error("the node returned Final before finish was sent")]
    EarlyFinal,
    #[error("the node did not acknowledge contiguous chunk {expected} (returned {got:?})", expected = expected.get())]
    Ack {
        expected: ChunkSeq,
        got: Option<ChunkSeq>,
    },
    #[error("dictation input contained no complete chunk")]
    Empty,
    #[error("dictation sequence count overflowed")]
    Sequence,
    #[error("dictation was cancelled")]
    Cancelled,
    #[error("dictation could not observe Ctrl-C")]
    Signal(#[source] io::Error),
    #[error("dictation finish acceptance exceeded its wait budget")]
    AcceptanceTimedOut,
    #[error("dictation final transcript exceeded its wait budget")]
    FinalTimedOut,
    #[error("dictation could not represent its deadline")]
    Clock,
    #[error("dictation finish tracking closed unexpectedly")]
    FinishNotice(#[source] tokio::sync::oneshot::error::RecvError),
    #[error("the final transcript could not be written to stdout")]
    Output(#[source] io::Error),
}
#[derive(Debug, thiserror::Error)]
pub(super) enum Cleanup {
    #[error("the node did not confirm cancellation")]
    Call(#[source] CallError),
    #[error("cancellation exceeded its wait budget")]
    TimedOut(#[source] tokio::time::error::Elapsed),
}

/// A private speech failure never exposes a foreign message through main's chain renderer.
pub(crate) struct DictateError {
    primary: Failure,
    cleanup: Option<Cleanup>,
}
impl DictateError {
    pub(super) fn new(primary: Failure) -> Self {
        Self {
            primary,
            cleanup: None,
        }
    }
    pub(super) fn with_cleanup(primary: Failure, cleanup: Cleanup) -> Self {
        Self {
            primary,
            cleanup: Some(cleanup),
        }
    }
}
impl From<FileError> for DictateError {
    fn from(source: FileError) -> Self {
        Self::new(Failure::File(source))
    }
}
impl fmt::Debug for DictateError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}
impl fmt::Display for DictateError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.primary {
            Failure::Call {
                source: CallError::UnassignedProtocol { .. },
                ..
            } => {
                out.write_str("dictation is not available in this build; use a node and client with dictation support")?;
            }
            Failure::Call {
                source: CallError::NotInProtocol { .. },
                ..
            } => out.write_str("this node does not support this client's dictation protocol")?,
            Failure::Call {
                source: CallError::Remote(error),
                ..
            } => {
                if let Some(vessel_proto::ErrorDetail::Dictation { reason }) = &error.detail {
                    reason_words(reason, out)?;
                } else {
                    out.write_str("the node refused the dictation request")?;
                }
            }
            Failure::Stream(reason) => reason_words(reason, out)?,
            Failure::File(_)
            | Failure::Call {
                source:
                    CallError::Disconnected
                    | CallError::UnexpectedReply(_)
                    | CallError::RequestIdsExhausted,
                ..
            }
            | Failure::UnknownUpdate
            | Failure::EarlyFinal
            | Failure::Ack { .. }
            | Failure::Empty
            | Failure::Sequence
            | Failure::Cancelled
            | Failure::Signal(_)
            | Failure::AcceptanceTimedOut
            | Failure::FinalTimedOut
            | Failure::Clock
            | Failure::FinishNotice(_)
            | Failure::Output(_) => fmt::Display::fmt(&self.primary, out)?,
        }
        if self.cleanup.is_some() {
            out.write_str("; stream cancellation could not be confirmed")?;
        }
        Ok(())
    }
}
// The retained causes above may contain speech in a remote message. No public
// source or raw accessor is provided; fixed typed metadata is the public boundary.
impl std::error::Error for DictateError {}

fn reason_words(reason: &DictationReason, out: &mut fmt::Formatter<'_>) -> fmt::Result {
    let words = match reason {
        DictationReason::BadAudio => "the node refused the audio shape",
        DictationReason::TooLong => "the recording exceeded the stream duration bound",
        DictationReason::ReorderOverflow => "the node's held-audio budget was exceeded",
        DictationReason::TranscriptTooLong => "the transcript exceeded its byte bound",
        DictationReason::TimedOut => "dictation reached its finish deadline",
        DictationReason::Replaced => "another start replaced this recording",
        DictationReason::Upstream => "the transcription endpoint failed; audio was not resent",
        DictationReason::Cancelled => "dictation was cancelled",
        DictationReason::NoAudio => "the node accepted no audio",
        DictationReason::Disabled => "dictation is disabled on this node",
        DictationReason::KeyAbsent => "the node's selected dictation key is missing or empty",
        DictationReason::LocalNotBuilt { .. } => {
            "this build has no local transcription engine; select an endpoint on the node"
        }
        DictationReason::TooMany => {
            "the node or this connection has reached its dictation reservation bound"
        }
        DictationReason::Unknown => "the node returned an unknown dictation failure reason",
    };
    out.write_str(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;
    #[test]
    fn retained_remote_speech_and_cleanup_messages_are_absent_from_public_diagnostics() {
        const MARKER: &str = "private-speech-marker-6b78ce92";
        let raw = || {
            CallError::Remote(vessel_proto::WireError::new(
                vessel_proto::ErrorCode::Unavailable,
                MARKER,
            ))
        };
        let error = DictateError::with_cleanup(
            Failure::Call {
                method: Method::ChunkDictation,
                source: raw(),
            },
            Cleanup::Call(raw()),
        );
        assert!(!error.to_string().contains(MARKER));
        assert!(!format!("{error:?}").contains(MARKER));
        assert!(error.source().is_none());
        let Failure::Call {
            source: CallError::Remote(wire),
            ..
        } = &error.primary
        else {
            panic!("typed private cause was lost");
        };
        assert!(wire.message == MARKER);
        let Some(Cleanup::Call(CallError::Remote(wire))) = &error.cleanup else {
            panic!("typed cleanup cause was lost");
        };
        assert!(wire.message == MARKER);
        let rendered = crate::chain(&crate::error::CliError::Dictation(error));
        assert!(!rendered.contains(MARKER));
    }
}
