//! Exact request text, including the reference's newline and multibyte prompts.
use super::{fixture, gated::Gate, policy, running, started, text};
use crate::support::{agent_of, read_all, read_until, within};
use tokio::io::AsyncWriteExt as _;
use vessel_proto::{
    AgentEvent, ContentBlock, Delivery, PermissionOptionId, PermissionOutcome, SessionInput,
};

async fn echoed(configured: Option<&str>) {
    within(async {
        let prompt = configured.unwrap_or("Resume working.");
        let f = fixture();
        let node = f.start().await;
        let owner = f.connect(&node).await;
        policy(&owner, true, configured).await;
        let id = running(&owner, &f).await;
        assert_eq!(
            owner
                .prompt(id, text("wait"), Delivery::Start)
                .await
                .unwrap(),
            vessel_proto::Delivered::Started
        );
        let before = owner.session(id).await.unwrap().next_seq;
        node.stop().await.unwrap();
        let gate = Gate::install(&f).await;
        let node = f.start().await;
        let owner = f.connect(&node).await;
        let (_, mut held) = gate.arrive().await;
        let mut stream = owner.follow(id, before).await.unwrap();
        held.write_all(&[1]).await.unwrap();
        drop(held);
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| started(record, prompt)).await;
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::PermissionRequested(_)))
        })
        .await;
        let asked = seen
            .iter()
            .find_map(|record| match agent_of(record) {
                Some(AgentEvent::PermissionRequested(asked)) => Some(asked.id),
                _ => None,
            })
            .unwrap();
        let echoes = seen
            .iter()
            .filter_map(|record| match agent_of(record) {
                Some(AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text },
                }) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(echoes, vec![format!("echo: {prompt} (model fake-large)")]);
        owner
            .send(
                id,
                SessionInput::AnswerPermission {
                    id: asked,
                    outcome: PermissionOutcome::Selected {
                        option: PermissionOptionId::new("allow-once"),
                    },
                },
            )
            .await
            .unwrap();
        read_until(&mut stream, &mut seen, |record| {
            matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))
        })
        .await;
        assert_eq!(
            read_all(&owner, id)
                .await
                .iter()
                .filter(|record| started(record, prompt))
                .count(),
            1
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_prompt_is_the_exact_actual_provider_request() {
    echoed(None).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reference_custom_prompt_keeps_its_trailing_newline_in_the_provider_request() {
    echoed(Some("Continue verbatim.\n")).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custom_multibyte_prompt_keeps_spaces_tabs_and_line_endings_in_the_provider_request() {
    echoed(Some("  계속 작업하세요.\t\r\n")).await;
}
