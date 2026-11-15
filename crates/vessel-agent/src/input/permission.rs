//! Rich fields are refused at the provider boundary before the ordinary answer mutates it.

use super::AgentInput;
use crate::InputError;
use vessel_proto::timeline::{PermissionInterrupt, PermissionResponse, PermissionResponseRefusal};
use vessel_proto::{PermissionId, PermissionOutcome};

/// Applied only means the driver accepted; the node must await its durable resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDispatch {
    /// Ordinary offered decision accepted by the driver.
    Applied(PermissionOutcome),
    /// No provider/queue/turn effects were attempted.
    Refused(PermissionResponseRefusal),
    /// A possibly applied action lost its correlated acknowledgement.
    OutcomeUnknown,
}

impl AgentInput {
    /// Respond without dropping edits, explanations, or interrupt intent.
    /// Current provider protocols only implement offered option decisions.
    ///
    /// # Errors
    /// The ordinary stale/conflicting/unknown-option or connection errors.
    pub async fn respond_permission(
        &self,
        permission: PermissionId,
        response: PermissionResponse,
    ) -> Result<PermissionDispatch, InputError> {
        if let Some(reason) = permission_response_refusal(&response) {
            return Ok(PermissionDispatch::Refused(reason));
        }
        self.answer(permission, response.outcome.clone()).await?;
        Ok(PermissionDispatch::Applied(response.outcome))
    }
}

/// Current response edges refuse extras as a whole before applying an offered option.
/// The synthetic plan owner reuses this boundary rather than silently dropping fields.
#[must_use]
pub fn permission_response_refusal(
    response: &PermissionResponse,
) -> Option<PermissionResponseRefusal> {
    if response.modified_input.is_some() {
        return Some(PermissionResponseRefusal::ModifiedInputUnsupported);
    }
    if response.message.is_some() {
        return Some(PermissionResponseRefusal::MessageUnsupported);
    }
    if response.interrupt != PermissionInterrupt::Continue {
        return Some(PermissionResponseRefusal::InterruptUnsupported);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use vessel_proto::RawJson;

    #[tokio::test]
    async fn unsupported_rich_fields_refuse_before_a_driver_command_can_mutate_state() {
        let (sender, mut commands) = mpsc::channel(1);
        let input = AgentInput::new(sender);
        let mut response = PermissionResponse {
            outcome: PermissionOutcome::Cancelled,
            modified_input: Some(RawJson::from_text("{}").unwrap()),
            message: None,
            interrupt: PermissionInterrupt::Continue,
        };
        for expected in [
            PermissionResponseRefusal::ModifiedInputUnsupported,
            PermissionResponseRefusal::MessageUnsupported,
            PermissionResponseRefusal::InterruptUnsupported,
        ] {
            assert_eq!(
                input
                    .respond_permission(PermissionId::random(), response.clone())
                    .await
                    .unwrap(),
                PermissionDispatch::Refused(expected)
            );
            assert!(matches!(
                commands.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ));
            match expected {
                PermissionResponseRefusal::ModifiedInputUnsupported => {
                    response.modified_input = None;
                    response.message = Some("No".into());
                }
                PermissionResponseRefusal::MessageUnsupported => {
                    response.message = None;
                    response.interrupt = PermissionInterrupt::Interrupt;
                }
                PermissionResponseRefusal::InterruptUnsupported => {}
                PermissionResponseRefusal::UnknownOption
                | PermissionResponseRefusal::PlanDecision { .. } => {
                    panic!("not a rich-field capability")
                }
            }
        }
    }
}
