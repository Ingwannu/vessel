//! Independent effects and retained successes, using the actual attachment store.
use super::*;
use crate::support::within;
use vessel_client::comparison::{FailureKind, JobError};
use vessel_proto::workspace::ListWorktrees;
use vessel_proto::{ContentBlock, Delivery, ErrorCode};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn models_share_text_and_stored_bytes_in_distinct_worktrees() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let client = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&client).await;
        let job = prepared(&client, project.id, &["fake-small", "fake-large"]).await;
        let before = job.prepare().await.unwrap();
        let source = before.targets[0].resources.session.unwrap();
        let bytes = Bytes::from(vec![b'x'; (1 << 20) + 73]);
        let file = attached(&client, source, bytes.clone()).await;
        let sent = job
            .start(input("describe", Some((source, file.clone()))))
            .await
            .unwrap();
        assert!(sent.input_bound);
        assert!(
            sent.targets
                .iter()
                .all(|target| matches!(target.status, TargetStatus::Started)),
            "{sent:?}"
        );
        let worktrees = client
            .call(ListWorktrees {
                project: Some(project.id),
            })
            .await
            .unwrap()
            .worktrees;
        assert_eq!(worktrees.len(), 2);
        assert_eq!(
            worktrees[0].base.as_ref().unwrap().commit,
            worktrees[1].base.as_ref().unwrap().commit
        );
        assert_ne!(worktrees[0].id, worktrees[1].id);
        for (target, model) in sent.targets.iter().zip(["fake-small", "fake-large"]) {
            let session = target.resources.session.unwrap();
            let info = client.session(session).await.unwrap();
            let vessel_proto::SessionMode::Structured {
                model: selected, ..
            } = info.spec.mode
            else {
                panic!("structured");
            };
            assert_eq!(selected.unwrap().as_str(), model);
            let mut stream = client.follow(session, EventSeq::ZERO).await.unwrap();
            let mut seen = Vec::new();
            crate::support::read_until(&mut stream, &mut seen, |record| matches!(&record.event,
                vessel_proto::SessionEvent::Agent { event: vessel_proto::AgentEvent::TurnEnded { .. } })).await;
            assert!(seen.iter().any(|record| matches!(&record.event,
                vessel_proto::SessionEvent::Agent { event: vessel_proto::AgentEvent::MessageChunk { content: ContentBlock::Text { text } } }
                if text == &format!("text 8; link common.txt text/plain {} read {}", bytes.len(), bytes.len()))));
            let history = turns(&client, session).await;
            assert_eq!(history.len(), 1);
            assert_eq!(
                history[0][0],
                ContentBlock::Text {
                    text: "describe".to_owned()
                }
            );
            let ContentBlock::Attachment { attachment } = &history[0][1] else {
                panic!("file");
            };
            if session != source {
                assert_ne!(attachment.id, file.id);
            }
            let mut copied = Vec::new();
            while u64::try_from(copied.len()).unwrap() < attachment.size {
                let piece = client
                    .read_attachment(session, attachment.id, u64::try_from(copied.len()).unwrap())
                    .await
                    .unwrap();
                copied.extend_from_slice(&piece.data);
            }
            assert_eq!(copied, bytes);
        }
        assert!(matches!(
            job.start(input("another", None)).await,
            Err(JobError::AlreadyBound)
        ));
        assert!(matches!(
            job.continue_target(sent.targets[0].index).await,
            Err(JobError::CannotContinue)
        ));
        assert_eq!(
            job.cancel()
                .await
                .unwrap()
                .targets
                .iter()
                .filter(|entry| matches!(entry.status, TargetStatus::Started))
                .count(),
            2
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_busy_target_is_refused_without_queue_or_steer_and_other_start_survives() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let client = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&client).await;
        let job = prepared(&client, project.id, &["fake-small", "fake-large"]).await;
        let busy = job.snapshot().targets[1].resources.session.unwrap();
        let _started = client
            .prompt(
                busy,
                vec![ContentBlock::Text {
                    text: "wait".to_owned(),
                }],
                Delivery::Start,
            )
            .await
            .unwrap();
        let sent = job.start(input("plain", None)).await.unwrap();
        assert!(matches!(sent.targets[0].status, TargetStatus::Started));
        assert!(
            matches!(&sent.targets[1].status, TargetStatus::Failed(failure)
            if failure.code == Some(ErrorCode::Conflict))
        );
        assert_eq!(turns(&client, busy).await.len(), 1);
        assert!(
            client
                .conversation(busy)
                .await
                .unwrap()
                .queue
                .entries
                .is_empty()
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bad_model_is_independent_and_a_nonmember_source_cannot_bind_input() {
    within(async {
        let fixture = Fixture::new();
        let node = start_node(&fixture).await;
        let client = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&client).await;
        let job = client.comparison(
            node.info().id,
            plan(project.id, &["fake-small", "not-a-model"]),
        );
        let ready = job.prepare().await.unwrap();
        assert!(matches!(ready.targets[0].status, TargetStatus::Ready));
        assert!(matches!(ready.targets[1].status, TargetStatus::Failed(_)));
        let outside = client
            .create_session(crate::support::structured_spec(&fixture, None))
            .await
            .unwrap()
            .id;
        let file = attached(
            &client,
            ready.targets[0].resources.session.unwrap(),
            Bytes::from_static(b"original"),
        )
        .await;
        let rejected = job
            .start(input("plain", Some((outside, file))))
            .await
            .unwrap();
        assert!(!rejected.input_bound);
        assert_eq!(
            rejected.input_rejected.as_ref().unwrap().kind,
            FailureKind::Source
        );
        assert!(
            turns(&client, ready.targets[0].resources.session.unwrap())
                .await
                .is_empty()
        );
        let sent = job.start(input("plain", None)).await.unwrap();
        assert!(matches!(sent.targets[0].status, TargetStatus::Started));
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_common_media_rejects_before_any_prompt_and_can_be_corrected() {
    within(async {
        let fixture = Fixture::new();
        std::fs::write(
            fixture.data_dir().profiles_path(),
            "[profiles.hears]\nharness = \"claude\"\nenv = { FAKE_ACP_AUDIO = \"1\" }\n",
        )
        .unwrap();
        let node = start_node(&fixture).await;
        let client = fixture.connect(&node).await;
        let (_folder, project) = crate::tidying::project(&client).await;
        let selections = vec![
            Selection::new(
                HarnessKind::ClaudeCode.into(),
                Some("hears".parse().unwrap()),
                Some(vessel_proto::ModelId::new("fake-small")),
                Vec::new(),
            )
            .unwrap(),
            Selection::new(
                HarnessKind::ClaudeCode.into(),
                None,
                Some(vessel_proto::ModelId::new("fake-large")),
                Vec::new(),
            )
            .unwrap(),
        ];
        let job = client.comparison(
            node.info().id,
            ComparisonPlan::new(
                project.id,
                WorktreeBase::Ref {
                    name: "main".parse().unwrap(),
                },
                selections,
            )
            .unwrap(),
        );
        let _prepared = job.prepare().await.unwrap();
        let source = job.snapshot().targets[0].resources.session.unwrap();
        let audio = client
            .attach(
                source,
                "voice.wav".parse().unwrap(),
                "audio/wav".parse().unwrap(),
                Bytes::from_static(b"fake audio"),
            )
            .await
            .unwrap();
        let rejected = job
            .start(input("plain", Some((source, audio))))
            .await
            .unwrap();
        assert!(!rejected.input_bound);
        assert_eq!(
            rejected.input_rejected.as_ref().unwrap().kind,
            FailureKind::Input
        );
        for target in &rejected.targets {
            assert!(
                turns(&client, target.resources.session.unwrap())
                    .await
                    .is_empty()
            );
        }
        let corrected = job.start(input("plain", None)).await.unwrap();
        assert!(
            corrected
                .targets
                .iter()
                .all(|target| matches!(target.status, TargetStatus::Started))
        );
        node.stop().await.unwrap();
    })
    .await;
}
