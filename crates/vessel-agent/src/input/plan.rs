//! Correlated provider plan decisions, separate from ordinary permissions.
//! A completed native proposal can outlive its successful turn; its full key
//! therefore travels to the one owning driver rather than being a prompt or
//! an option label. This carrier alone enables no provider transition.

use super::{AgentInput, Command};
use crate::InputError;
use tokio::sync::oneshot;
use vessel_proto::timeline::{DecidePlan, PlanDecisionRefusal, PlanDecisionReply};

pub(crate) type PlanReply = oneshot::Sender<Result<PlanDecisionReply, InputError>>;

impl AgentInput {
    /// Ask the owning provider to decide the exact proposal identified by `request`.
    /// An accepted implementation requires actual native follow-up acceptance.
    ///
    /// # Errors
    /// A closed command port or lost reply. If a request might have reached the
    /// provider, the actor must preserve uncertainty and must not resend it.
    pub async fn decide_plan(&self, request: DecidePlan) -> Result<PlanDecisionReply, InputError> {
        let (reply, result) = oneshot::channel();
        self.commands
            .send(Command::DecidePlan { request, reply })
            .await
            .map_err(|_closed| InputError::Closed)?;
        result.await.unwrap_or(Err(InputError::Closed))
    }
}

/// Explicit refusal until a checked provider implementation replaces its arm.
/// Nothing is sent as a user prompt, option change or permission response.
pub(crate) fn unsupported(request: DecidePlan, reply: PlanReply) {
    let _closed = reply.send(Ok(PlanDecisionReply::Refused {
        proposal: request.proposal,
        reason: PlanDecisionRefusal::Unsupported,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::timeline::{PlanDecision, PlanProposalKey, PlanProposalSource};

    fn request(decision: PlanDecision) -> DecidePlan {
        DecidePlan {
            proposal: PlanProposalKey {
                thread: "thread-owned".parse().unwrap(),
                turn: "turn-complete".parse().unwrap(),
                source: PlanProposalSource::LegacyTurnPlan,
            },
            decision,
        }
    }

    #[tokio::test]
    async fn every_decision_keeps_its_exact_key_and_has_no_prompt_fallback() {
        for decision in [
            PlanDecision::Dismiss,
            PlanDecision::Implement,
            PlanDecision::ImplementResume,
        ] {
            let (sender, mut commands) = tokio::sync::mpsc::channel(1);
            let input = AgentInput::new(sender);
            let expected = request(decision);
            let sent = expected.clone();
            let pending = tokio::spawn(async move { input.decide_plan(sent).await });
            let Some(Command::DecidePlan { request, reply }) = commands.recv().await else {
                panic!("dedicated plan command")
            };
            assert_eq!(request, expected);
            unsupported(request, reply);
            assert_eq!(
                pending.await.unwrap().unwrap(),
                PlanDecisionReply::Refused {
                    proposal: expected.proposal,
                    reason: PlanDecisionRefusal::Unsupported,
                }
            );
            assert!(commands.recv().await.is_none());
        }
    }

    #[tokio::test]
    async fn closed_before_dispatch_and_lost_reply_never_report_accepted() {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        drop(receiver);
        assert!(matches!(
            AgentInput::new(sender)
                .decide_plan(request(PlanDecision::Implement))
                .await,
            Err(InputError::Closed)
        ));
        let (sender, mut commands) = tokio::sync::mpsc::channel(1);
        let input = AgentInput::new(sender);
        let pending =
            tokio::spawn(async move { input.decide_plan(request(PlanDecision::Implement)).await });
        let Some(Command::DecidePlan { reply, .. }) = commands.recv().await else {
            panic!("dedicated plan command")
        };
        drop(reply);
        assert!(matches!(pending.await.unwrap(), Err(InputError::Closed)));
    }
}
