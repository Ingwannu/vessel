//! Classify local pairing failures before an app-internal event reaches UI.
//!
//! [Decision Log]
//! - Purpose: the confirmation dialog must not display an English diagnostic
//!   or a route that could contain a credential.
//! - Constraint: the node and client wire contracts stay unchanged; their
//!   diagnostics do not carry translated sentences.
//! - Choice: preserve the action the person can take as a typed view problem.
//!   Each source variant is explicit, so a new variant needs a decision here.
//! - Cost: UI shows a recovery category rather than the low-level diagnostic.

use crate::LinkError;
use vessel_app_view::pairing::PairProblem;

pub(crate) fn problem(error: &LinkError) -> PairProblem {
    match error {
        #[cfg(not(target_arch = "wasm32"))]
        LinkError::AttachmentCall(_) => PairProblem::Connection,
        LinkError::AttachmentTarget(_) | LinkError::AttachmentRead(_) => PairProblem::Connection,
        LinkError::BookFault(fault) => PairProblem::Book(fault.clone()),
        LinkError::Name(_) => PairProblem::Name,
        LinkError::NoHome | LinkError::Book(_) | LinkError::Credential(_) | LinkError::Io(_) => {
            PairProblem::Storage
        }
        LinkError::NoToken
        | LinkError::Token(_)
        | LinkError::NoCredential
        | LinkError::Connect(_)
        | LinkError::SshNotYet
        | LinkError::Call(_)
        | LinkError::NotConnected(_) => PairProblem::Connection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_text_reaches_each_typed_decoder_problem_without_display_copy() {
        use vessel_app_view::pairing::LinkProblem;
        for (input, problem) in [
            ("not a link", LinkProblem::WrongKind),
            ("vessel://pair#%", LinkProblem::Encoding),
            // Empty JSON decodes as base64url but is not a pairing offer.
            ("vessel://pair#e30", LinkProblem::Offer),
        ] {
            let error = crate::read_link(input).unwrap_err();
            assert_eq!(LinkProblem::from(&error), problem);
        }
    }

    #[test]
    fn each_host_error_preserves_its_recovery_without_copying_its_diagnostic() {
        let fault = vessel_app_view::files::FileFault {
            path: "nodes.toml".to_owned(),
            at: Some((3, 9)),
            problem: vessel_app_view::files::FileProblem::WrongKind,
            field: Some("token".to_owned()),
        };
        for (error, expected) in [
            (
                LinkError::BookFault(fault.clone()),
                PairProblem::Book(fault),
            ),
            (LinkError::NoHome, PairProblem::Storage),
            (LinkError::Name("invalid".into()), PairProblem::Name),
            (LinkError::Book("unwritable".into()), PairProblem::Storage),
            (
                LinkError::Credential("unwritable".into()),
                PairProblem::Storage,
            ),
            (LinkError::Io("worker".into()), PairProblem::Storage),
            (LinkError::NoToken, PairProblem::Connection),
            (LinkError::Token("invalid".into()), PairProblem::Connection),
            (LinkError::NoCredential, PairProblem::Connection),
            (
                LinkError::Connect("route unavailable".into()),
                PairProblem::Connection,
            ),
            (LinkError::SshNotYet, PairProblem::Connection),
            (LinkError::Call("refused".into()), PairProblem::Connection),
            (
                LinkError::NotConnected("machine".into()),
                PairProblem::Connection,
            ),
        ] {
            assert_eq!(problem(&error), expected);
        }
    }
}
