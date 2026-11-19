//! Why a diagnostics call has no answer (ADR 0111): the machine is not
//! connected, the node refused it (its code only, never its words: the
//! Devices page's rule), or it did not answer in time.

use vessel_proto::ErrorCode;

/// Why a diagnostics call has no answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// No live connection to the machine.
    #[error("the machine is not connected")]
    Offline,
    /// The node refused the call, with this code (`forbidden`: this
    /// device may read but not administer; `unsupported`: an older node).
    #[error("the node refused the call ({0:?})")]
    Refused(ErrorCode),
    /// No answer within the call's bound.
    #[error("the machine did not answer in time")]
    TimedOut,
}

impl Problem {
    /// Its words in the copied report, which is fixed English technical
    /// text (Paseo's is too): the code in the wire words' rule, never the
    /// node's text.
    #[must_use]
    pub fn report_text(self) -> String {
        match self {
            Self::Offline => "machine is not connected".to_owned(),
            Self::Refused(code) => format!(
                "refused ({})",
                vessel_proto::diagnostics::text::words(&code)
            ),
            Self::TimedOut => "no answer in time".to_owned(),
        }
    }
}
