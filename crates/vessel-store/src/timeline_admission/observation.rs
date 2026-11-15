//! Observation reads the same retained owner without reserving or loading inputs.
//! Initial queue/command identities are already accepted facts. A direct turn
//! remains unobserved until a separate actor-ticket binding has been committed.

use rusqlite::Connection;
use vessel_proto::Creator;
use vessel_proto::timeline::{ObservePromptReceiptReply, SubmitPrompt, SubmitPromptReply};

use crate::StoreError;

pub(crate) fn observe_input(
    connection: &Connection,
    request: &SubmitPrompt,
    creator: &Creator,
) -> Result<ObservePromptReceiptReply, StoreError> {
    let message = request.message.clone();
    let Some(known) = super::receipt::read_receipt(connection, request.session, &message)? else {
        return Ok(ObservePromptReceiptReply::Unobserved { message });
    };
    let fingerprint = super::ReceiptFingerprint::for_prompt(request, creator)?;
    let reply = super::receipt::replay(
        request.session,
        message,
        fingerprint,
        known.fingerprint,
        &known.state,
    )?;
    Ok(match reply {
        SubmitPromptReply::OutcomeUnknown { message } => {
            ObservePromptReceiptReply::OutcomeUnknown { message }
        }
        SubmitPromptReply::KeyConflict { message } => {
            ObservePromptReceiptReply::KeyConflict { message }
        }
        SubmitPromptReply::Refused { message, reason } => {
            ObservePromptReceiptReply::Refused { message, reason }
        }
        SubmitPromptReply::Accepted { receipt } => {
            let binding = super::binding::decode(
                request.session,
                receipt.accepted,
                known.binding.as_deref(),
            )?;
            ObservePromptReceiptReply::Accepted { receipt, binding }
        }
        // Read lookup cannot manufacture either fresh-admission outcome.
        SubmitPromptReply::Capacity { .. } | SubmitPromptReply::NotAdmitted { .. } => {
            return Err(StoreError::AdmissionIntegrity);
        }
    })
}
