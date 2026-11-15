//! Observation reads the same retained owner without reserving or loading inputs.
//! Initial queue/command identities are already accepted facts. A direct turn
//! remains unobserved until a separate actor-ticket binding has been committed.

use rusqlite::Connection;
use vessel_proto::Creator;
use vessel_proto::timeline::{
    ObservePromptReceiptReply, PromptDisposition, PromptReceiptBinding, SubmissionAcceptedKind,
    SubmitPrompt, SubmitPromptReply,
};

use crate::StoreError;

pub(crate) fn observe_input(
    connection: &Connection,
    request: &SubmitPrompt,
    creator: &Creator,
) -> Result<ObservePromptReceiptReply, StoreError> {
    let message = request.message.clone();
    Ok(match super::lookup_input(connection, request, creator)? {
        None => ObservePromptReceiptReply::Unobserved { message },
        Some(SubmitPromptReply::OutcomeUnknown { message }) => {
            ObservePromptReceiptReply::OutcomeUnknown { message }
        }
        Some(SubmitPromptReply::KeyConflict { message }) => {
            ObservePromptReceiptReply::KeyConflict { message }
        }
        Some(SubmitPromptReply::Refused { message, reason }) => {
            ObservePromptReceiptReply::Refused { message, reason }
        }
        Some(SubmitPromptReply::Accepted { receipt }) => {
            let binding = initial_binding(receipt.accepted);
            ObservePromptReceiptReply::Accepted { receipt, binding }
        }
        // Read lookup cannot manufacture either fresh-admission outcome.
        Some(SubmitPromptReply::Capacity { .. } | SubmitPromptReply::NotAdmitted { .. }) => {
            return Err(StoreError::AdmissionIntegrity);
        }
    })
}

fn initial_binding(accepted: SubmissionAcceptedKind) -> PromptReceiptBinding {
    match accepted {
        SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Queued { entry },
        } => PromptReceiptBinding::Queued { entry },
        SubmissionAcceptedKind::Command { echo, .. } => PromptReceiptBinding::Command { echo },
        SubmissionAcceptedKind::Prompt {
            disposition:
                PromptDisposition::Started
                | PromptDisposition::Steered
                | PromptDisposition::InterruptedAndStarted { .. },
        } => PromptReceiptBinding::Unobserved,
    }
}
