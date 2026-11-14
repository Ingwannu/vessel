//! Actual daemon/runtime restart recovery, with fake providers only.
#![cfg(test)]
use vessel_client::NodeClient;
use vessel_proto::automation::PromptText;
use vessel_proto::restart::SetRestartRecovery;
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};
use vessel_proto::{
    AgentEvent, ContentBlock, Delivery, EventSeq, QueueHold, SessionId, SessionStatus,
};
#[cfg(target_os = "linux")]
mod crash;
mod gated;
mod order;
mod prompts;
mod protocol;
fn write_profile(f: &Fixture, gate: Option<std::net::SocketAddr>) {
    let env = gate
        .map(|address| format!("env = {{ FAKE_ACP_LOAD_GATE = \"{address}\" }}\n"))
        .unwrap_or_default();
    std::fs::write(
        f.data_dir().profiles_path(),
        format!("[profiles.recovery]\nharness = \"claude\"\n{env}"),
    )
    .unwrap();
}
fn fixture() -> Fixture {
    let f = Fixture::new();
    write_profile(&f, None);
    f
}

use crate::support::{Fixture, agent_of, read_all, read_until, status_of, structured_spec, within};

async fn policy(client: &NodeClient, enabled: bool, prompt: Option<&str>) {
    let changed = client
        .call(SetNodeSettings {
            restart_recovery: Some(SetRestartRecovery {
                enabled: Some(enabled),
                prompt: prompt.map(|text| text.parse::<PromptText>().unwrap()),
            }),
            ..SetNodeSettings::default()
        })
        .await
        .unwrap();
    assert_eq!(changed.restart_recovery.enabled, enabled);
}
fn text(value: &str) -> Vec<ContentBlock> {
    vec![ContentBlock::Text {
        text: value.to_owned(),
    }]
}
async fn running(client: &NodeClient, fixture: &Fixture) -> SessionId {
    let id = client
        .create_session(structured_spec(fixture, None).with_profile("recovery".parse().unwrap()))
        .await
        .unwrap()
        .id;
    let mut stream = client.follow(id, EventSeq::ZERO).await.unwrap();
    read_until(&mut stream, &mut Vec::new(), |record| {
        status_of(record) == Some(&SessionStatus::Running)
    })
    .await;
    id
}
fn started(record: &vessel_proto::EventRecord, prompt: &str) -> bool {
    matches!(agent_of(record), Some(AgentEvent::TurnStarted { content }) if content == &text(prompt))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_clean_stop_resumes_the_same_conversation_once_and_keeps_its_queue_held() {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let client = f.connect(&node).await;
        let default = client
            .call(GetNodeSettings {
                include_restart_recovery: true,
            })
            .await
            .unwrap();
        assert!(!default.restart_recovery.enabled);
        assert_eq!(default.restart_recovery.prompt.as_str(), "Resume working.");
        policy(&client, true, Some("loaded")).await;
        let id = running(&client, &f).await;
        assert_eq!(
            client
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        assert!(matches!(
            client
                .prompt(id, text("queued-not-sent"), Delivery::Queue)
                .await
                .unwrap(),
            vessel_proto::Delivered::Queued { .. }
        ));
        let original = client.session(id).await.unwrap();
        node.stop().await.unwrap();
        let gate = gated::Gate::install(&f).await;
        let node = f.start().await;
        let client = f.connect(&node).await;
        let (_, mut held) = gate.arrive().await;
        let mut stream = client.follow(id, original.next_seq).await.unwrap();
        let mut seen = Vec::new();
        tokio::io::AsyncWriteExt::write_all(&mut held, &[1])
            .await
            .unwrap();
        drop(held);
        read_until(&mut stream, &mut seen, |record| started(record, "loaded")).await;
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        let after = client.session(id).await.unwrap();
        assert_eq!(after.id, original.id);
        assert_eq!(after.harness_session, original.harness_session);
        assert_eq!(after.created_by, original.created_by);
        assert_eq!(after.created_at, original.created_at);
        assert_eq!(after.profile, original.profile);
        assert_eq!(after.model, original.model);
        let queue = client.conversation(id).await.unwrap().queue;
        assert_eq!(queue.held, Some(QueueHold::Ended));
        assert_eq!(queue.entries.len(), 1);
        let log = read_all(&client, id).await;
        assert_eq!(
            log.iter()
                .filter(|record| started(record, "loaded"))
                .count(),
            1
        );
        assert_eq!(
            log.iter()
                .filter(|record| started(record, "queued-not-sent"))
                .count(),
            0
        );
        assert!(
            !log.iter().any(|record| matches!(
                agent_of(record),
                Some(AgentEvent::UserMessageChunk { .. })
            ))
        );
        node.stop().await.unwrap();
        let node = f.start().await;
        let client = f.connect(&node).await;
        assert_eq!(
            read_all(&client, id)
                .await
                .iter()
                .filter(|record| started(record, "loaded"))
                .count(),
            1,
            "idle at the next stop: no new recovery turn"
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_off_consumes_old_intent_and_later_on_does_not_reconstruct_it() {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let client = f.connect(&node).await;
        let id = running(&client, &f).await;
        assert_eq!(
            client
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        node.stop().await.unwrap();
        let node = f.start().await;
        let client = f.connect(&node).await;
        let log = read_all(&client, id).await;
        assert_eq!(
            client.session(id).await.unwrap().status,
            SessionStatus::Interrupted
        );
        policy(&client, true, None).await;
        assert_eq!(read_all(&client, id).await, log);
        node.stop().await.unwrap();
        let node = f.start().await;
        let client = f.connect(&node).await;
        assert_eq!(read_all(&client, id).await, log);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_cancel_and_kill_do_not_resume_but_a_new_accepted_turn_rearms() {
    within(async {
        let f = fixture();
        let node = f.start().await;
        let client = f.connect(&node).await;
        policy(&client, true, Some("loaded")).await;
        let cancelled = running(&client, &f).await;
        assert_eq!(
            client
                .prompt(cancelled, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        client
            .send(
                cancelled,
                vessel_proto::SessionInput::CancelTurn { turn: None },
            )
            .await
            .unwrap();
        let mut stream = client.follow(cancelled, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        let killed = running(&client, &f).await;
        assert_eq!(
            client
                .prompt(killed, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        client.kill(killed).await.unwrap();
        let rearmed = running(&client, &f).await;
        assert_eq!(
            client
                .prompt(rearmed, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        client
            .send(
                rearmed,
                vessel_proto::SessionInput::CancelTurn { turn: None },
            )
            .await
            .unwrap();
        let mut stream = client.follow(rearmed, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        assert_eq!(
            client
                .prompt(rearmed, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        node.stop().await.unwrap();
        let gate = gated::Gate::install(&f).await;
        let node = f.start().await;
        let client = f.connect(&node).await;
        let (_, mut held) = gate.arrive().await;
        let mut stream = client.follow(rearmed, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        tokio::io::AsyncWriteExt::write_all(&mut held, &[1])
            .await
            .unwrap();
        drop(held);
        read_until(&mut stream, &mut seen, |record| started(record, "loaded")).await;
        for id in [cancelled, killed] {
            assert!(
                !read_all(&client, id)
                    .await
                    .iter()
                    .any(|record| started(record, "loaded"))
            );
        }
        node.stop().await.unwrap();
    })
    .await;
}
