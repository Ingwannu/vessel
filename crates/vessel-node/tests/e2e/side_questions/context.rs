//! The latest plan survives outside the bounded transcript tail.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn current_plan_is_included_after_more_than_a_transcript_page_of_output() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let parent = client.create_session(structured_spec(&fixture, None)).await.unwrap();
        let mut stream = ready(&client, parent.id).await;
        client.send(parent.id, prompt("make a plan")).await.unwrap();
        let _first_turn = permission(&client, parent.id, &mut stream).await;
        client.send(parent.id, prompt("entries 256")).await.unwrap();
        read_until(&mut stream, &mut Vec::new(), |record| matches!(agent_of(record), Some(AgentEvent::TurnEnded { .. }))).await;
        let before = read_all(&client, parent.id).await;
        let side = client.open_side_question(parent.id).await.unwrap();
        let mut side_stream = ready(&client, side.id).await;
        let _answer = client.prompt_side_question(side.id, "계획을 알려줘".parse().unwrap()).await.unwrap();
        let seen = permission(&client, side.id, &mut side_stream).await;
        assert!(seen.iter().any(|record| matches!(agent_of(record), Some(AgentEvent::MessageChunk { content: ContentBlock::Text { text } }) if text.contains("Latest recorded plan:") && text.contains("Run the fake tool") && text.ends_with("계획을 알려줘 (model fake-large)"))));
        assert_eq!(read_all(&client, parent.id).await, before);
        node.stop().await.unwrap();
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serialized_provider_limit_rejects_without_a_turn_or_sticky_context() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let parent = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap();
        let mut stream = ready(&client, parent.id).await;
        for _ in 0..4 {
            client
                .send(parent.id, prompt(&"한".repeat(5_000)))
                .await
                .unwrap();
            let _turn = permission(&client, parent.id, &mut stream).await;
        }
        let before = read_all(&client, parent.id).await;
        let side = client.open_side_question(parent.id).await.unwrap();
        let mut side_stream = ready(&client, side.id).await;
        let cursor = client.session(side.id).await.unwrap().next_seq;
        let large = format!("{}x", "\n".repeat(16_000));
        assert_eq!(
            code(
                client
                    .prompt_side_question(side.id, large.parse().unwrap())
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        assert_eq!(client.session(side.id).await.unwrap().next_seq, cursor);
        let _answer = client
            .prompt_side_question(side.id, "small".parse().unwrap())
            .await
            .unwrap();
        let seen = permission(&client, side.id, &mut side_stream).await;
        let requests: Vec<_> = seen
            .iter()
            .filter_map(|record| {
                if let Some(AgentEvent::TurnStarted { content }) = agent_of(record) {
                    Some(content)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            requests,
            vec![&vec![ContentBlock::Text {
                text: "small".to_owned()
            }]]
        );
        assert_eq!(read_all(&client, parent.id).await, before);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generated_context_replayed_by_load_does_not_become_user_history() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(fixture.data_dir().profiles_path(), "[profiles.replay]\nharness = \"claude\"\nenv = { FAKE_ACP_REPLAY_USER = \"generated parent context replay\" }\n").unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("replay".parse().unwrap());
        let parent = client.create_session(spec).await.unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let side = client.open_side_question(parent.id).await.unwrap();
        let mut stream = ready(&client, side.id).await;
        let _answer = client.prompt_side_question(side.id, "original question".parse().unwrap()).await.unwrap();
        let _turn = permission(&client, side.id, &mut stream).await;
        let _parked = client.call(vessel_proto::parking::ParkSession { session: side.id }).await.unwrap();
        let _answer = client.prompt_side_question(side.id, "question after parking".parse().unwrap()).await.unwrap();
        let woken = permission(&client, side.id, &mut stream).await;
        assert!(woken.iter().any(|record| matches!(record.event, vessel_proto::SessionEvent::Woken)));
        assert!(client.session(side.id).await.unwrap().parked.is_none());
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let _answer = client.resume_session(side.id).await.unwrap();
        let records = read_all(&client, side.id).await;
        assert!(!records.iter().any(|record| matches!(agent_of(record), Some(AgentEvent::UserMessageChunk { .. }))));
        let questions: Vec<_> = records.iter().filter_map(|record| if let Some(AgentEvent::TurnStarted { content }) = agent_of(record) { Some(content.clone()) } else { None }).collect();
        assert_eq!(questions, vec![
            vec![ContentBlock::Text { text: "original question".to_owned() }],
            vec![ContentBlock::Text { text: "question after parking".to_owned() }],
        ]);
        node.stop().await.unwrap();
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_side_wake_records_no_question_and_leaves_the_parent_unchanged() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.data_dir().profiles_path(),
            "[profiles.breaks]\nharness = \"claude\"\nenv = { FAKE_ACP_LOAD_FAILS = \"1\" }\n",
        )
        .unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("breaks".parse().unwrap());
        let parent = client.create_session(spec).await.unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let before = read_all(&client, parent.id).await;
        let side = client.open_side_question(parent.id).await.unwrap();
        let _side_stream = ready(&client, side.id).await;
        let _parked = client
            .call(vessel_proto::parking::ParkSession { session: side.id })
            .await
            .unwrap();
        assert_eq!(
            code(
                client
                    .prompt_side_question(side.id, "must not appear".parse().unwrap())
                    .await
            ),
            ErrorCode::SpawnFailed
        );
        let failed = client.session(side.id).await.unwrap();
        assert!(matches!(
            failed.parked.map(|parked| parked.reason),
            Some(vessel_proto::parking::ParkReason::WakeFailed { .. })
        ));
        let records = read_all(&client, side.id).await;
        assert!(!records.iter().any(|record| matches!(
            agent_of(record),
            Some(AgentEvent::TurnStarted { .. } | AgentEvent::UserMessageChunk { .. })
        )));
        assert_eq!(read_all(&client, parent.id).await, before);
        node.stop().await.unwrap();
    })
    .await;
}
