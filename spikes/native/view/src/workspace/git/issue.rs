//! A notice retains its reason instead of a sentence made in the old
//! language: changing the app's language rewords an open failure too.

use vessel_proto::workspace::{GitRefusal, InvalidText};

use crate::workspace::problem::Problem;

use super::SwitchProblem;

/// A git failure or refusal shown beside its action/form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    /// A read or write could not produce an outcome.
    Call(Problem),
    /// The repository refused a write.
    Refused(GitRefusal),
    /// The commit message could not become a request.
    Message(InvalidText),
    /// A typed branch name could not become a request.
    Name(InvalidText),
    /// A listed branch could not safely become a local checkout.
    Switch(SwitchProblem),
    /// App-side correlation keys are exhausted.
    Keys,
}

impl Issue {
    /// A refused/preparation/call failure is an error notice. Enumerating
    /// the reasons forces a new meaning to choose its own signal too.
    #[must_use]
    pub const fn alert_variant(&self) -> crate::alert::Variant {
        match self {
            Self::Call(_)
            | Self::Refused(_)
            | Self::Message(_)
            | Self::Name(_)
            | Self::Switch(_)
            | Self::Keys => crate::alert::Variant::Error,
        }
    }
}
