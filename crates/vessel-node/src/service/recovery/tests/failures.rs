//! Failed providers/folders consume only their attempt, never its neighbour.
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_missing_folder_fails_without_blocking_an_independent_working_session_or_retrying() {
    vessel_testkit::within(Duration::from_secs(60), Box::pin(async {
        let mut f = Fixture::new();
        let node = f.start().await;
        let owner = f.client(&node).await;
        set(&node, true).await;
        let normal = f.work.clone();
        f.work = normal.join("removed");
        std::fs::create_dir(&f.work).unwrap();
        let removed = f.active(&node, &owner).await;
        let removed_path = f.work.clone();
        f.work = normal;
        let working = f.active(&node, &owner).await;
        node.stop().await.unwrap();
        std::fs::remove_dir(removed_path).unwrap();
        let node = f.start().await;
        let owner = f.client(&node).await;
        done(&node).await;
        assert!(matches!(owner.session(removed.id).await.unwrap().status, SessionStatus::Failed { .. }));
        let mut stream = owner.follow(working.id, working.next_seq).await.unwrap();
        let mut prompts = 0;
        loop {
            let record = stream.next().await.unwrap().unwrap();
            if matches!(record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { .. } }) { prompts += 1; }
            if matches!(record.event, SessionEvent::Agent { event: AgentEvent::TurnEnded { .. } }) { break; }
        }
        assert_eq!(prompts, 1);
        let failed = owner.read(removed.id, EventSeq::ZERO, EVENT_PAGE).await.unwrap();
        assert!(!failed.records.iter().any(|record| matches!(&record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { content } } if content == &vec![ContentBlock::Text { text: "loaded".into() }])));
        node.stop().await.unwrap();
        let node = f.start().await;
        let owner = f.client(&node).await;
        done(&node).await;
        assert_eq!(owner.read(removed.id, EventSeq::ZERO, EVENT_PAGE).await.unwrap().records, failed.records);
        node.stop().await.unwrap();
    })).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn unsupported_resume_load_failure_and_unavailable_provider_are_not_retried() {
    vessel_testkit::within(Duration::from_secs(60), Box::pin(async {
        for variable in ["FAKE_ACP_NO_LOAD", "FAKE_ACP_LOAD_FAILS", "FAKE_ACP_EXIT_EARLY"] {
            let f = Fixture::new();
            let node = f.start().await;
            let owner = f.client(&node).await;
            set(&node, true).await;
            let original = f.active(&node, &owner).await;
            node.stop().await.unwrap();
            std::fs::write(f.data.profiles_path(), format!("[profiles.recovery]\nharness = \"claude\"\nenv = {{ {variable} = \"1\" }}\n")).unwrap();
            let node = f.start().await;
            let owner = f.client(&node).await;
            done(&node).await;
            assert!(matches!(owner.session(original.id).await.unwrap().status, SessionStatus::Failed { .. }), "{variable}");
            let failed = owner.read(original.id, EventSeq::ZERO, EVENT_PAGE).await.unwrap();
            assert!(!failed.records.iter().any(|record| matches!(&record.event, SessionEvent::Agent { event: AgentEvent::TurnStarted { content } } if content == &vec![ContentBlock::Text { text: "loaded".into() }])));
            node.stop().await.unwrap();
            let node = f.start().await;
            let owner = f.client(&node).await;
            done(&node).await;
            assert_eq!(owner.read(original.id, EventSeq::ZERO, EVENT_PAGE).await.unwrap().records, failed.records, "{variable}");
            node.stop().await.unwrap();
        }
    })).await.unwrap();
}
