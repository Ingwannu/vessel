//! Native command acceptance shares actor ordering but never changes turn tickets.
use super::submission::SubmissionDispatch;
use crate::structured::StructuredActor;
use vessel_agent::NativeCommandDispatch;
use vessel_proto::native_commands::NativeCommandIntent;
use vessel_proto::timeline::{
    ClientMessageId, NativeCommandRefusal, SubmissionAcceptedKind, SubmissionDelivery,
    SubmissionRefusal,
};
use vessel_proto::{AgentEvent, ContentBlock, SessionEvent};

impl StructuredActor {
    pub(super) async fn submit_native_command(
        &mut self,
        content: Vec<ContentBlock>,
        message: ClientMessageId,
        delivery: SubmissionDelivery,
        intent: NativeCommandIntent,
    ) -> SubmissionDispatch {
        if delivery != SubmissionDelivery::Start || !intent.matches_content(&content) {
            return SubmissionDispatch::Refused(SubmissionRefusal::InvalidContent);
        }
        let input = self.input.clone();
        let kind = intent.kind();
        let offered = self
            .pump(async move { input.native_command_offers().await })
            .await;
        match offered {
            Ok(Ok(offers)) if offers.supports(kind) => {}
            Ok(Ok(_)) => return refused(NativeCommandRefusal::NotOffered),
            Ok(Err(_)) | Err(_) => return refused(NativeCommandRefusal::InvalidState),
        }
        let input = self.input.clone();
        let dispatched = self
            .pump(async move { input.native_command(intent).await })
            .await;
        match dispatched {
            Ok(Ok(NativeCommandDispatch::Accepted { status })) if status.kind() == kind => {
                let echo = self.life.out.next_seq();
                self.life
                    .publish(
                        &mut self.agent,
                        SessionEvent::Agent {
                            event: AgentEvent::CommandAccepted {
                                client_message_id: message,
                                content,
                                status,
                            },
                        },
                    )
                    .await;
                if self.life.out.next_seq() == echo {
                    return SubmissionDispatch::OutcomeUnknown;
                }
                SubmissionDispatch::Accepted(SubmissionAcceptedKind::Command { status, echo })
            }
            Ok(Ok(NativeCommandDispatch::Refused { reason })) => refused(reason),
            Ok(Ok(
                NativeCommandDispatch::Accepted { .. } | NativeCommandDispatch::OutcomeUnknown,
            ))
            | Ok(Err(_))
            | Err(_) => SubmissionDispatch::OutcomeUnknown,
        }
    }
}
fn refused(cause: NativeCommandRefusal) -> SubmissionDispatch {
    SubmissionDispatch::Refused(SubmissionRefusal::NativeCommand { cause })
}
