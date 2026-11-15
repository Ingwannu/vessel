//! One typed internal native command port, independent of prompt/turn delivery.
use super::{AgentInput, Command};
use crate::InputError;
use tokio::sync::oneshot;
use vessel_proto::native_commands::{NativeCommandIntent, NativeCommandStatus};
use vessel_proto::timeline::NativeCommandRefusal;

/// A real provider adapter chooses this only from its actual dispatch outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandDispatch {
    /// The actual native operation was accepted; this says nothing about a turn.
    Accepted { status: NativeCommandStatus },
    /// Closed pre-effect refusal; never carries raw RPC or objective content.
    Refused { reason: NativeCommandRefusal },
    /// A possibly sent mutating operation lost its acknowledgement.
    OutcomeUnknown,
}
/// Read-only capabilities of the actual input owner; never a persisted catalog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NativeCommandOffers {
    /// No current native operation is proven.
    #[default]
    None,
    /// The current transport can compact context.
    Compact,
    /// The current run has proven launch/version/autonomous goal support.
    Goal,
    /// Both operations are proven for this current run.
    CompactAndGoal,
}
impl NativeCommandOffers {
    /// Check an explicit kind, never a command label or global manifest.
    #[must_use]
    pub const fn supports(self, kind: vessel_proto::native_commands::NativeCommandKind) -> bool {
        use vessel_proto::native_commands::NativeCommandKind;
        matches!(
            (self, kind),
            (
                Self::Compact | Self::CompactAndGoal,
                NativeCommandKind::Compact
            ) | (Self::Goal | Self::CompactAndGoal, NativeCommandKind::Goal)
        )
    }
}

pub(crate) type NativeCommandReply = oneshot::Sender<Result<NativeCommandDispatch, InputError>>;
impl AgentInput {
    /// Read the live driver's current run capabilities without a provider mutation.
    /// # Errors
    /// A retired/closed driver cannot provide a current capability answer.
    pub async fn native_command_offers(&self) -> Result<NativeCommandOffers, InputError> {
        let (reply, result) = oneshot::channel();
        self.commands
            .send(Command::NativeCommandOffers { reply })
            .await
            .map_err(|_closed| InputError::Closed)?;
        result.await.map_err(|_closed| InputError::Closed)
    }

    /// Dispatch one already admitted intent; the driver rechecks its real run state.
    /// # Errors
    /// A closed command port or lost internal reply. A caller must retain uncertainty
    /// when a mutating request may already have reached the provider.
    pub async fn native_command(
        &self,
        intent: NativeCommandIntent,
    ) -> Result<NativeCommandDispatch, InputError> {
        let (reply, result) = oneshot::channel();
        self.commands
            .send(Command::NativeCommand { intent, reply })
            .await
            .map_err(|_closed| InputError::Closed)?;
        result.await.unwrap_or(Err(InputError::Closed))
    }
}

/// Every current adapter explicitly refuses. Only W22's checked transport may
/// replace its arm; neither a command label nor a version string enables it.
pub(crate) fn unsupported(_intent: NativeCommandIntent, reply: NativeCommandReply) {
    let _closed = reply.send(Ok(NativeCommandDispatch::Refused {
        reason: NativeCommandRefusal::Unsupported,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn native_intent_has_its_own_correlated_result_without_a_prompt_command() {
        let (sender, mut commands) = tokio::sync::mpsc::channel(1);
        let input = AgentInput::new(sender);
        let task =
            tokio::spawn(async move { input.native_command(NativeCommandIntent::Compact).await });
        let Some(Command::NativeCommand { intent, reply }) = commands.recv().await else {
            panic!("native command")
        };
        assert_eq!(intent, NativeCommandIntent::Compact);
        unsupported(intent, reply);
        assert_eq!(
            task.await.unwrap().unwrap(),
            NativeCommandDispatch::Refused {
                reason: NativeCommandRefusal::Unsupported
            }
        );
        assert!(commands.recv().await.is_none());
    }
    #[tokio::test]
    async fn capabilities_are_read_from_the_live_owner_and_retirement_refuses_the_query() {
        let (sender, mut commands) = tokio::sync::mpsc::channel(1);
        let input = AgentInput::new(sender);
        let asking = input.clone();
        let task = tokio::spawn(async move { asking.native_command_offers().await });
        let Some(Command::NativeCommandOffers { reply }) = commands.recv().await else {
            panic!("live query")
        };
        reply.send(NativeCommandOffers::Compact).unwrap();
        let offers = task.await.unwrap().unwrap();
        assert!(offers.supports(vessel_proto::native_commands::NativeCommandKind::Compact));
        assert!(!offers.supports(vessel_proto::native_commands::NativeCommandKind::Goal));
        drop(commands);
        assert!(matches!(
            input.native_command_offers().await,
            Err(InputError::Closed)
        ));
    }
}
