//! Read-only recovery and supervised actor-ticket persistence use the same owner.
//! A dropped ticket or failed watermark never fabricates a target or resends work.

use tokio::sync::{oneshot, watch};
use vessel_proto::timeline::{
    ObservePromptReceipt, ObservePromptReceiptReply, PromptReceiptBinding,
};
use vessel_proto::timeline::{PromptDisposition, SubmissionAcceptedKind};
use vessel_proto::{Creator, EventSeq, SessionId};
use vessel_session::SubmissionTarget;
use vessel_store::ReceiptBindingToken;

use super::{RECEIPT_COMMIT_TIMEOUT, Sessions};
use crate::{error::ServiceError, persist};

pub(super) enum ReceiptTarget {
    Ready(SubmissionTarget),
    Queued(oneshot::Receiver<SubmissionTarget>),
}

pub(super) struct ReceiptBarrier {
    pub(super) through: EventSeq,
    pub(super) target: Option<ReceiptTarget>,
}

pub(super) async fn target_barrier(
    accepted: SubmissionAcceptedKind,
    target: oneshot::Receiver<SubmissionTarget>,
    queued_through: EventSeq,
) -> Option<ReceiptBarrier> {
    match accepted {
        SubmissionAcceptedKind::Command { echo, .. } => Some(ReceiptBarrier {
            through: echo.next()?,
            target: None,
        }),
        SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Queued { .. },
        } => Some(ReceiptBarrier {
            through: queued_through,
            target: Some(ReceiptTarget::Queued(target)),
        }),
        SubmissionAcceptedKind::Prompt {
            disposition:
                PromptDisposition::Started
                | PromptDisposition::Steered
                | PromptDisposition::InterruptedAndStarted { .. },
        } => {
            let observed = tokio::time::timeout(RECEIPT_COMMIT_TIMEOUT, target)
                .await
                .ok()?
                .ok()?;
            let SubmissionTarget::Turn { started } = observed else {
                return None;
            };
            if matches!(accepted, SubmissionAcceptedKind::Prompt { disposition: PromptDisposition::InterruptedAndStarted { interrupted } } if interrupted == started)
            {
                return None;
            }
            Some(ReceiptBarrier {
                through: started.next()?,
                target: Some(ReceiptTarget::Ready(observed)),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn direct_barrier_uses_the_ticket_and_closed_tickets_stay_uncertain() {
        let (sent, received) = oneshot::channel();
        sent.send(SubmissionTarget::Turn {
            started: EventSeq::new(17),
        })
        .unwrap();
        let accepted = SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Started,
        };
        let barrier = target_barrier(accepted, received, EventSeq::new(999))
            .await
            .unwrap();
        assert_eq!(barrier.through, EventSeq::new(18));
        let (sent, received) = oneshot::channel();
        drop(sent);
        assert!(
            target_barrier(accepted, received, EventSeq::new(999))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_replacement_never_binds_the_turn_that_was_interrupted() {
        let accepted = SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::InterruptedAndStarted {
                interrupted: EventSeq::new(17),
            },
        };
        let (sent, received) = oneshot::channel();
        sent.send(SubmissionTarget::Turn {
            started: EventSeq::new(17),
        })
        .unwrap();
        assert!(
            target_barrier(accepted, received, EventSeq::new(999))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn queued_and_command_acceptance_do_not_wait_for_a_model_turn() {
        let (sent, received) = oneshot::channel();
        let queued = SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Queued {
                entry: vessel_proto::QueueEntryId::random(),
            },
        };
        let barrier = target_barrier(queued, received, EventSeq::new(27))
            .await
            .unwrap();
        assert_eq!(barrier.through, EventSeq::new(27));
        assert!(matches!(barrier.target, Some(ReceiptTarget::Queued(_))));
        drop(sent);
        let (_held, received) = oneshot::channel();
        let command = SubmissionAcceptedKind::Command {
            status: vessel_proto::native_commands::NativeCommandStatus::CompactStarted,
            echo: EventSeq::new(32),
        };
        let barrier = target_barrier(command, received, EventSeq::new(999))
            .await
            .unwrap();
        assert_eq!(barrier.through, EventSeq::new(33));
        assert!(barrier.target.is_none());
    }
}

impl Sessions {
    pub(crate) async fn observe_prompt_receipt(
        &self,
        request: ObservePromptReceipt,
        by: Creator,
    ) -> Result<ObservePromptReceiptReply, ServiceError> {
        // Ordinary lookup recognizes archived/stored sessions but never starts one.
        let _found = self.find(request.request.session).await?;
        Ok(self.store.observe_prompt_receipt(request, by).await?)
    }

    pub(super) fn retain_receipt_target(
        &self,
        session: SessionId,
        token: ReceiptBindingToken,
        target: ReceiptTarget,
        mut durability: watch::Receiver<persist::Durability>,
    ) {
        let store = self.store.clone();
        // The existing tracker retains this task until the owning actor settles
        // or closes its ticket. Queue waits share that actor's lifetime.
        drop(self.tasks.spawn(async move {
            let target = match target {
                ReceiptTarget::Ready(target) => target,
                ReceiptTarget::Queued(waiting) => {
                    let Ok(target) = waiting.await else { return; };
                    target
                }
            };
            let (target, through) = match target {
                SubmissionTarget::Turn { started } => {
                    let Some(through) = started.next() else { return; };
                    (PromptReceiptBinding::Turn { started }, through)
                }
                SubmissionTarget::Withdrawn { entry, through } => {
                    (PromptReceiptBinding::Withdrawn { entry }, through)
                }
            };
            let persisted = tokio::time::timeout(
                RECEIPT_COMMIT_TIMEOUT,
                persist::wait_until(session, &mut durability, through),
            ).await;
            if !matches!(persisted, Ok(Ok(()))) { return; }
            if let Err(error) = store.bind_prompt_receipt(token, target).await {
                tracing::warn!(%session, %error, "receipt target remains unobserved after commit failure");
            }
        }));
    }
}
