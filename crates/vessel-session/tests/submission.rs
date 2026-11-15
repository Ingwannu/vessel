//! Atomic keyed policies exercise only the private ACP test process.
use support::{Fixture, Log, structured, within};
use vessel_proto::timeline::{
    ClientMessageId, NativeCommandRefusal, PromptDisposition, SubmissionAcceptedKind,
    SubmissionDelivery, SubmissionRefusal,
};
use vessel_proto::{AgentEvent, ContentBlock, Creator, SessionEvent};
use vessel_session::{LaunchOptions, SubmissionDispatch};
#[path = "support/runtime.rs"]
#[expect(dead_code, reason = "shared private fake runtime fixture")]
mod support;
fn text(value: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text { text: value.into() }]
}
async fn ready(log: &mut Log) {
    log.until(|event| {
        matches!(
            event,
            SessionEvent::Agent {
                event: AgentEvent::Ready { .. }
            }
        )
    })
    .await;
}
#[tokio::test]
async fn unavailable_steer_interrupts_held_turn_once_and_preserves_message_identity() {
    within(Box::pin(async {
        let fixture=Fixture::new();
        let profiles=vessel_harness::parse_profiles("[profiles.no-steer]\nharness='claude'\nenv={FAKE_ACP_NO_STEER='1'}",Some(&fixture.bin)).unwrap();
        let name="no-steer".parse().unwrap();
        let profile=profiles.get(&name).unwrap().clone();
        let mut spec=structured(vec![]); spec.profile=Some(name);
        let handle=fixture.start_with(spec,LaunchOptions {profile:Some(profile),..LaunchOptions::default()}).await;
        let mut log=Log::of(&handle); ready(&mut log).await;
        assert!(!handle.conversation().unwrap().abilities.unwrap().steer);
        let first=ClientMessageId::random();
        assert!(matches!(handle.submit_identified(text("wait"),first,SubmissionDelivery::Start,Creator::Owner,None).await.unwrap(),SubmissionDispatch::Accepted(SubmissionAcceptedKind::Prompt {disposition:PromptDisposition::Started})));
        log.until(|event|matches!(event,SessionEvent::Agent {event:AgentEvent::TurnStarted {..}})).await;
        let held=log.records.iter().find(|record|matches!(record.event,SessionEvent::Agent {event:AgentEvent::TurnStarted {..}})).unwrap().seq;
        let message=ClientMessageId::random();
        let result=handle.submit_identified(text("replacement"),message.clone(),SubmissionDelivery::SteerOrInterrupt,Creator::Owner,None).await.unwrap();
        assert!(matches!(result,SubmissionDispatch::Accepted(SubmissionAcceptedKind::Prompt {disposition:PromptDisposition::InterruptedAndStarted {interrupted}}) if interrupted==held),"{result:?}");
        log.until(|event|matches!(event,SessionEvent::Agent {event:AgentEvent::TurnStarted {client_message_id:Some(id),..}} if id==&message)).await;
        assert_eq!(log.records.iter().filter(|record|matches!(record.event,SessionEvent::Agent {event:AgentEvent::TurnStarted {..}})).count(),2);
        handle.kill(); let _ended=handle.ended().await;
    })).await;
}
#[tokio::test]
async fn available_steer_joins_without_starting_or_cancelling_another_turn() {
    within(Box::pin(async {
        let fixture=Fixture::new(); let handle=fixture.start(structured(vec![])).await;
        let mut log=Log::of(&handle); ready(&mut log).await;
        let _started=handle.submit_identified(text("wait"),ClientMessageId::random(),SubmissionDelivery::Start,Creator::Owner,None).await.unwrap();
        let message=ClientMessageId::random();
        let result=handle.submit_identified(text("inside"),message.clone(),SubmissionDelivery::SteerOrInterrupt,Creator::Owner,None).await.unwrap();
        assert!(matches!(result,SubmissionDispatch::Accepted(SubmissionAcceptedKind::Prompt {disposition:PromptDisposition::Steered})),"{result:?}");
        log.until(|event|matches!(event,SessionEvent::Agent {event:AgentEvent::Steered {client_message_id:Some(id),..}} if id==&message)).await;
        assert_eq!(log.records.iter().filter(|record|matches!(record.event,SessionEvent::Agent {event:AgentEvent::TurnStarted {..}})).count(),1);
        assert!(!log.records.iter().any(|record|matches!(record.event,SessionEvent::Agent {event:AgentEvent::TurnEnded {..}})));
        handle.kill(); let _ended=handle.ended().await;
    })).await;
}
#[tokio::test]
async fn unimplemented_native_transport_refuses_without_a_model_turn() {
    within(Box::pin(async {
        let fixture=Fixture::new(); let handle=fixture.start(structured(vec![])).await;
        let mut log=Log::of(&handle); ready(&mut log).await;
        let refused=ClientMessageId::random();
        let result=handle.submit_identified(text("/compact"),refused.clone(),SubmissionDelivery::Start,Creator::Owner,Some(vessel_proto::native_commands::NativeCommandIntent::Compact)).await.unwrap();
        assert!(matches!(result,SubmissionDispatch::Refused(SubmissionRefusal::NativeCommand {cause:NativeCommandRefusal::Unsupported})),"{result:?}");
        let sent=ClientMessageId::random();
        let _started=handle.submit_identified(text("plain"),sent,SubmissionDelivery::Start,Creator::Owner,None).await.unwrap();
        log.until(|event|matches!(event,SessionEvent::Agent {event:AgentEvent::TurnEnded {..}})).await;
        assert!(!log.records.iter().any(|record|matches!(&record.event,SessionEvent::Agent {event:AgentEvent::TurnStarted {client_message_id:Some(id),..}} if id==&refused)));
        assert_eq!(log.records.iter().filter(|record|matches!(record.event,SessionEvent::Agent {event:AgentEvent::TurnStarted {..}})).count(),1);
        handle.kill(); let _ended=handle.ended().await;
    })).await;
}
