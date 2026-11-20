//! Discovery, inheritance, serialized reset and restart boundaries.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_open_and_ordinary_creation_share_one_side_and_restart_keeps_it() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let parent = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap();
        let _stream = ready(&client, parent.id).await;
        let mut supplied = structured_spec(&fixture, None);
        supplied.side_question_of = Some(parent.id);
        supplied.harness = HarnessKind::OpenCode.into();
        supplied.profile = Some("missing-profile".parse().unwrap());
        supplied.cwd = Some(fixture.cwd().join("missing-folder"));
        supplied.mode = SessionMode::Structured {
            model: Some(vessel_proto::ModelId::new("caller-override")),
            resume: Some(vessel_proto::HarnessSessionId::new("must-not-resume")),
            options: Vec::new(),
        };
        let (one, two, three) = tokio::join!(
            client.open_side_question(parent.id),
            client.open_side_question(parent.id),
            client.create_session(supplied)
        );
        let side = one.unwrap();
        assert_eq!(side.id, two.unwrap().id);
        assert_eq!(side.id, three.unwrap().id);
        assert_eq!(side.spec.harness, parent.spec.harness);
        assert_eq!(side.spec.profile, parent.spec.profile);
        assert_eq!(side.spec.cwd, parent.spec.cwd);
        assert!(matches!(
            side.spec.mode,
            SessionMode::Structured { resume: None, .. }
        ));
        let _side_stream = ready(&client, side.id).await;
        assert_eq!(
            code(client.open_side_question(side.id).await),
            ErrorCode::Unsupported
        );
        let terminal = client
            .create_session(terminal_spec(&fixture, HarnessKind::ClaudeCode))
            .await
            .unwrap();
        assert_eq!(
            code(client.open_side_question(terminal.id).await),
            ErrorCode::Unsupported
        );
        assert_eq!(
            code(
                client
                    .prompt_side_question(parent.id, "wrong target".parse().unwrap())
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        assert_eq!(
            code(client.open_side_question(SessionId::random()).await),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let found = client.open_side_question(parent.id).await.unwrap();
        assert_eq!(found.id, side.id);
        assert_eq!(found.status, SessionStatus::Interrupted);
        assert_eq!(
            code(
                client
                    .prompt_side_question(side.id, "ended".parse().unwrap())
                    .await
            ),
            ErrorCode::Conflict
        );
        let _answer = client.resume_session(side.id).await.unwrap();
        let _archived = client.mark(parent.id, Mark::Archive).await.unwrap();
        let _deleted = client.delete_session(parent.id).await.unwrap();
        assert!(!read_all(&client, side.id).await.is_empty());
        assert_eq!(
            code(
                client
                    .prompt_side_question(side.id, "gone".parse().unwrap())
                    .await
            ),
            ErrorCode::NotFound
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resetting_uses_current_choices_and_a_stale_reset_cannot_delete_the_replacement() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let parent = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let side = client.open_side_question(parent.id).await.unwrap();
        let _side_stream = ready(&client, side.id).await;
        let choice = vessel_proto::OptionChoice {
            id: vessel_proto::OptionId::new("model"),
            value: vessel_proto::ChoiceValue::Select {
                value: vessel_proto::OptionValueId::new("fake-small"),
            },
        };
        let _answer = client.set_option(parent.id, choice.clone()).await.unwrap();
        let (reset, opened) = tokio::join!(
            client.reset_side_question(parent.id, side.id),
            client.open_side_question(parent.id)
        );
        let reset = reset.unwrap();
        let opened = opened.unwrap();
        assert!(opened.id == side.id || opened.id == reset.id);
        assert_ne!(reset.id, side.id);
        assert_eq!(code(client.session(side.id).await), ErrorCode::NotFound);
        let SessionMode::Structured {
            options, resume, ..
        } = &reset.spec.mode
        else {
            panic!("a side is structured");
        };
        assert!(resume.is_none());
        assert!(options.contains(&choice));
        assert_eq!(
            code(client.reset_side_question(parent.id, side.id).await),
            ErrorCode::Conflict
        );
        assert_eq!(
            client.open_side_question(parent.id).await.unwrap().id,
            reset.id
        );
        let current = client
            .sessions()
            .await
            .unwrap()
            .into_iter()
            .filter(|info| {
                info.spec.side_question_of == Some(parent.id) && info.marks.archived_at.is_none()
            })
            .count();
        assert_eq!(current, 1);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_parent_profile_move_does_not_restart_its_side_and_reset_inherits_the_move() {
    within(async {
        let fixture = Fixture::new();
        let homes = tempfile::tempdir().unwrap();
        for home in ["first", "second"] {
            std::fs::create_dir_all(homes.path().join(home)).unwrap();
        }
        let profiles = format!(
            "[profiles.first]\nharness = \"claude\"\nenv = {{ CLAUDE_CONFIG_DIR = '{}' }}\n[profiles.second]\nharness = \"claude\"\nenv = {{ CLAUDE_CONFIG_DIR = '{}' }}\n",
            homes.path().join("first").display(), homes.path().join("second").display()
        );
        std::fs::write(fixture.data_dir().profiles_path(), profiles).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("first".parse().unwrap());
        let parent = client.create_session(spec).await.unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let parent = client.session(parent.id).await.unwrap();
        let project = vessel_harness::fakes::claude_project(&homes.path().join("first"), &fixture.cwd()).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join(format!("{}.jsonl", parent.harness_session.unwrap())), "{}\n").unwrap();
        let side = client.open_side_question(parent.id).await.unwrap();
        let _side_stream = ready(&client, side.id).await;
        let native_side = client.session(side.id).await.unwrap().harness_session;
        let _answer = client.move_profile(parent.id, Some("second".parse().unwrap()), false).await.unwrap();
        let unchanged = client.open_side_question(parent.id).await.unwrap();
        assert_eq!(unchanged.id, side.id);
        assert_eq!(unchanged.profile, Some("first".parse().unwrap()));
        assert_eq!(unchanged.harness_session, native_side);
        let replaced = client.reset_side_question(parent.id, side.id).await.unwrap();
        assert_eq!(replaced.profile, Some("second".parse().unwrap()));
        assert_eq!(replaced.spec.side_question_of, Some(parent.id));
        node.stop().await.unwrap();
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_over_does_not_resurrect_an_older_restored_side() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let parent = client
            .create_session(structured_spec(&fixture, None))
            .await
            .unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let old = client.open_side_question(parent.id).await.unwrap();
        let _old_stream = ready(&client, old.id).await;
        let _answer = client.mark(old.id, Mark::Archive).await.unwrap();
        let current = client.open_side_question(parent.id).await.unwrap();
        let _current_stream = ready(&client, current.id).await;
        let _answer = client.mark(old.id, Mark::Unarchive).await.unwrap();
        assert_eq!(
            client.open_side_question(parent.id).await.unwrap().id,
            current.id
        );
        let replacement = client
            .reset_side_question(parent.id, current.id)
            .await
            .unwrap();
        assert_ne!(replacement.id, old.id);
        assert_ne!(replacement.id, current.id);
        assert_eq!(
            client.open_side_question(parent.id).await.unwrap().id,
            replacement.id
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_replacement_leaves_old_deleted_and_open_retries_after_profile_repair() {
    within(async {
        let fixture = Fixture::new();
        let profiles = "[profiles.side]\nharness = \"claude\"\n";
        std::fs::write(fixture.data_dir().profiles_path(), profiles).unwrap();
        let node = fixture.start().await;
        let client = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("side".parse().unwrap());
        let parent = client.create_session(spec).await.unwrap();
        let _parent_stream = ready(&client, parent.id).await;
        let old = client.open_side_question(parent.id).await.unwrap();
        let _side_stream = ready(&client, old.id).await;
        std::fs::write(fixture.data_dir().profiles_path(), "").unwrap();
        let _answer = client.reload_profiles().await.unwrap();
        assert_eq!(
            code(client.reset_side_question(parent.id, old.id).await),
            ErrorCode::InvalidRequest
        );
        assert_eq!(code(client.session(old.id).await), ErrorCode::NotFound);
        std::fs::write(fixture.data_dir().profiles_path(), profiles).unwrap();
        let _answer = client.reload_profiles().await.unwrap();
        let replacement = client.open_side_question(parent.id).await.unwrap();
        assert_ne!(replacement.id, old.id);
        assert_eq!(replacement.spec.side_question_of, Some(parent.id));
        node.stop().await.unwrap();
    })
    .await;
}
