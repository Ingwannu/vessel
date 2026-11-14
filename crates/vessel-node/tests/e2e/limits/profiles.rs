//! Launch association across catalog reload, wake, move and node restart.
use super::*;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_sessions_limits_keep_the_launch_home_after_profile_reload() {
    use vessel_proto::{SessionMode, SessionSpec};
    within(async {
        let fixture=Fixture::new();
        let old=fixture.user_home().join("old-account");
        let new=fixture.user_home().join("new-account");
        std::fs::create_dir_all(&old).unwrap();std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join(".credentials.json"),br#"{"claudeAiOauth":{"accessToken":"synthetic-session-launch-login-for-offline-quota"}}"#).unwrap();
        let profiles=|home:&std::path::Path| format!("[profiles.alternate]\nharness='claude'\nenv={{CLAUDE_CONFIG_DIR='{}'}}\n",home.display());
        std::fs::write(fixture.data_dir().profiles_path(),profiles(&old)).unwrap();
        let body=fixture.user_home().join("claude-response.json");
        std::fs::write(&body,vessel_testkit::limits::CLAUDE_USAGE).unwrap();
        let probe=LimitProbe::Recorded {root:fixture.user_home().to_owned(),body,status:200,retry_after:None};
        let node=fixture.start_with(|config|config.limits=probe).await;
        let owner=fixture.connect(&node).await;
        let mut spec=SessionSpec::new(HarnessKind::ClaudeCode,SessionMode::Structured {model:None,resume:None,options:Vec::new()});
        spec.cwd=Some(fixture.cwd());spec.profile=Some("alternate".parse().unwrap());
        let session=owner.create_session(spec).await.unwrap().id;
        let mut stream=owner.follow(session,vessel_proto::EventSeq::ZERO).await.unwrap();
        crate::support::read_until(&mut stream,&mut Vec::new(),|record|crate::support::status_of(record)==Some(&vessel_proto::SessionStatus::Running)).await;
        let _saved = owner.usage_lookup(SetUsageLookup {policy:ExternalLookup::On}).await.unwrap();
        let selected=|scope,refresh| GetUsageLimits {scope,refresh};
        let own=LimitsScope::Session {session};
        let original=owner.usage_limits(selected(own.clone(),LimitRefresh::Refresh)).await.unwrap();
        assert!(matches!(original.state,AccountLimitState::Ready {..}));
        std::fs::write(fixture.data_dir().profiles_path(),profiles(&new)).unwrap();
        let _profiles = owner.reload_profiles().await.unwrap();
        assert!(matches!(owner.usage_limits(selected(own.clone(),LimitRefresh::Refresh)).await.unwrap().state,AccountLimitState::Ready {..}));
        let _parked=owner.call(vessel_proto::parking::ParkSession {session}).await.unwrap();
        let _awake=owner.call(vessel_proto::parking::WakeSession {session}).await.unwrap();
        assert!(matches!(owner.usage_limits(selected(own.clone(),LimitRefresh::Refresh)).await.unwrap().state,AccountLimitState::Ready {..}));
        let current=LimitsScope::Account {account:LimitAccount {harness:HarnessKind::ClaudeCode.into(),profile:Some("alternate".parse().unwrap())}};
        assert_eq!(owner.usage_limits(selected(current,LimitRefresh::Refresh)).await.unwrap().state,AccountLimitState::Failed {reason:LimitFailure::NoLogin,retained:None,retry_at:None});
        // Account and live-session cache entries share the public name but
        // remain keyed by their distinct captured binding.
        assert!(matches!(owner.usage_limits(selected(own,LimitRefresh::Cached)).await.unwrap().state,AccountLimitState::Ready {..}));
        node.stop().await.unwrap();
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stored_sessions_do_not_guess_a_current_profile_after_restart() {
    use crate::agents::structured;
    within(async {
        let fixture = Fixture::new();
        let probe = setup(&fixture);
        let node = fixture
            .start_with(|config| config.limits = probe.clone())
            .await;
        let owner = fixture.connect(&node).await;
        let session = structured(&owner, &fixture).await.id;
        let _saved = owner
            .usage_lookup(SetUsageLookup {
                policy: ExternalLookup::On,
            })
            .await
            .unwrap();
        node.stop().await.unwrap();
        let node = fixture.start_with(|config| config.limits = probe).await;
        let owner = fixture.connect(&node).await;
        let reply = owner
            .usage_limits(GetUsageLimits {
                scope: LimitsScope::Session { session },
                refresh: LimitRefresh::Refresh,
            })
            .await
            .unwrap();
        assert_eq!(
            reply.state,
            AccountLimitState::Failed {
                reason: LimitFailure::Unassociated,
                retained: None,
                retry_at: None
            }
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_a_successful_profile_move_changes_the_sessions_saved_login_binding() {
    use crate::support::{ask, read_until, status_of, structured_spec};
    use vessel_proto::{EventSeq, SessionStatus};
    within(async {
        let fixture = Fixture::new();
        let source = fixture.user_home().join("source-login");
        let target = fixture.user_home().join("target-login");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(
            source.join(".credentials.json"),
            br#"{"claudeAiOauth":{"accessToken":"synthetic-profile-source-only-offline-login"}}"#,
        )
        .unwrap();
        let profiles = [("source", &source), ("target", &target)]
            .map(|(name, home)| {
                format!(
                    "[profiles.{name}]\nharness='claude'\nenv={{CLAUDE_CONFIG_DIR='{}'}}\n",
                    home.display()
                )
            })
            .join("\n");
        std::fs::write(fixture.data_dir().profiles_path(), profiles).unwrap();
        let body = fixture.user_home().join("claude-response.json");
        std::fs::write(&body, vessel_testkit::limits::CLAUDE_USAGE).unwrap();
        let node = fixture
            .start_with(|config| {
                config.limits = LimitProbe::Recorded {
                    root: fixture.user_home().to_owned(),
                    body,
                    status: 200,
                    retry_after: None,
                }
            })
            .await;
        let owner = fixture.connect(&node).await;
        let mut spec = structured_spec(&fixture, None);
        spec.profile = Some("source".parse().unwrap());
        let session = owner.create_session(spec).await.unwrap().id;
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        let mut seen = Vec::new();
        read_until(&mut stream, &mut seen, |record| {
            status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        assert_eq!(
            ask(&owner, &mut stream, &mut seen, session, "plain").await,
            "echo: plain"
        );
        let _saved = owner
            .usage_lookup(SetUsageLookup {
                policy: ExternalLookup::On,
            })
            .await
            .unwrap();
        let request = || GetUsageLimits {
            scope: LimitsScope::Session { session },
            refresh: LimitRefresh::Refresh,
        };
        assert!(matches!(
            owner.usage_limits(request()).await.unwrap().state,
            AccountLimitState::Ready { .. }
        ));
        assert_eq!(
            remote(
                owner
                    .move_profile(session, Some("absent".parse().unwrap()), false)
                    .await
            ),
            ErrorCode::InvalidRequest
        );
        rewind_preserving_login(&owner, session, &seen).await;
        let unchanged = owner.usage_limits(request()).await.unwrap();
        assert_eq!(unchanged.account.profile.unwrap().as_str(), "source");
        assert!(matches!(unchanged.state, AccountLimitState::Ready { .. }));
        saved_transcript(&owner, session, &source, &fixture.cwd()).await;
        let _saved = owner
            .move_profile(session, Some("target".parse().unwrap()), false)
            .await
            .unwrap();
        let moved = owner.usage_limits(request()).await.unwrap();
        assert_eq!(moved.account.profile.unwrap().as_str(), "target");
        assert_eq!(
            moved.state,
            AccountLimitState::Failed {
                reason: LimitFailure::NoLogin,
                retained: None,
                retry_at: None
            }
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_policy_fails_off_and_a_saved_opt_in_restores_without_a_snapshot() {
    within(async {
        let fixture = Fixture::new();
        let probe = setup(&fixture);
        let path = fixture.data_path().join("usage-lookup.json");
        std::fs::write(&path, br#"{"policy":"on"}"#).unwrap();
        let node = fixture
            .start_with(|config| config.limits = probe.clone())
            .await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Refresh))
                .await
                .unwrap()
                .state,
            AccountLimitState::Off
        );
        let _saved = owner
            .usage_lookup(SetUsageLookup {
                policy: ExternalLookup::On,
            })
            .await
            .unwrap();
        assert!(matches!(
            owner
                .usage_limits(query(LimitRefresh::Refresh))
                .await
                .unwrap()
                .state,
            AccountLimitState::Ready { .. }
        ));
        node.stop().await.unwrap();
        let node = fixture.start_with(|config| config.limits = probe).await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner
                .usage_limits(query(LimitRefresh::Cached))
                .await
                .unwrap()
                .state,
            AccountLimitState::NotRequested
        );
        node.stop().await.unwrap();
    })
    .await;
}

async fn rewind_preserving_login(
    owner: &vessel_client::NodeClient,
    session: vessel_proto::SessionId,
    seen: &[vessel_proto::EventRecord],
) {
    let after = seen
        .iter()
        .rev()
        .find_map(|record| {
            matches!(
                &record.event,
                vessel_proto::SessionEvent::Agent {
                    event: vessel_proto::AgentEvent::TurnEnded { point: Some(_), .. }
                }
            )
            .then_some(record.seq)
        })
        .unwrap();
    let _rewound = owner
        .call(vessel_proto::forking::RewindSession { session, after })
        .await
        .unwrap();
    let reply = owner
        .usage_limits(GetUsageLimits {
            scope: LimitsScope::Session { session },
            refresh: LimitRefresh::Refresh,
        })
        .await
        .unwrap();
    assert_eq!(reply.account.profile.unwrap().as_str(), "source");
    assert!(matches!(reply.state, AccountLimitState::Ready { .. }));
}

async fn saved_transcript(
    owner: &vessel_client::NodeClient,
    session: vessel_proto::SessionId,
    source: &std::path::Path,
    cwd: &std::path::Path,
) {
    let native = owner
        .session(session)
        .await
        .unwrap()
        .harness_session
        .unwrap();
    let project = vessel_harness::fakes::claude_project(source, cwd).unwrap();
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join(format!("{native}.jsonl")), "{}\n").unwrap();
}
