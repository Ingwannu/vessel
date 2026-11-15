//! CLI success requires the requested outcome, not merely a settled request.
//! The future typed RPC adapter calls these only for durable Settled receipts;
//! legacy acknowledgements, disappearance and timeout never enter this path.
use vessel_proto::{
    PermissionId, PermissionOptionId, PermissionOutcome, PermissionRequest, Question,
    QuestionAnswer, QuestionId, QuestionOutcome,
};

/// A cancellation or competing answer cannot be printed as the requested success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the requested response was not confirmed; list pending requests before retrying")]
pub(crate) struct Unconfirmed;

pub(crate) fn permission(
    request: &PermissionRequest,
    selected: &PermissionOptionId,
    actual_id: PermissionId,
    outcome: &PermissionOutcome,
) -> Result<(), Unconfirmed> {
    let offered = request.options.iter().any(|option| &option.id == selected);
    let matches = match outcome {
        PermissionOutcome::Selected { option } => option == selected,
        PermissionOutcome::Cancelled => false,
    };
    if offered && actual_id == request.id && matches {
        Ok(())
    } else {
        Err(Unconfirmed)
    }
}

pub(crate) fn question(
    request: &Question,
    answer: &QuestionAnswer,
    actual_id: QuestionId,
    outcome: &QuestionOutcome,
) -> Result<(), Unconfirmed> {
    let matches = match (answer, outcome) {
        (QuestionAnswer::Accept { values: expected }, QuestionOutcome::Answered { values }) => {
            expected == values
        }
        (QuestionAnswer::Decline, QuestionOutcome::Declined) => true,
        (QuestionAnswer::Accept { .. }, QuestionOutcome::Declined | QuestionOutcome::Cancelled)
        | (
            QuestionAnswer::Decline,
            QuestionOutcome::Answered { .. } | QuestionOutcome::Cancelled,
        ) => false,
    };
    if actual_id == request.id && matches && request.check(answer).is_ok() {
        Ok(())
    } else {
        Err(Unconfirmed)
    }
}

#[cfg(test)]
mod tests;
