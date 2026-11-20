//! A multi-provider session cannot read a different provider's saved login.
use super::*;
use vessel_harness::{
    Lookup,
    fakes::{Role, install},
};
use vessel_proto::{EventSeq, SessionMode, SessionSpec, SessionStatus};

fn saved_login(fixture: &Fixture) -> (std::path::PathBuf, LimitProbe) {
    let login = fixture.user_home().join(".local/share/opencode/auth.json");
    std::fs::create_dir_all(login.parent().unwrap()).unwrap();
    std::fs::write(
        &login,
        br#"{"openai":{"type":"oauth","access":"synthetic-opencode-login-for-offline-model-check"}}"#,
    )
    .unwrap();
    let body = fixture.user_home().join("response.json");
    std::fs::write(&body, vessel_testkit::limits::CODEX_USAGE).unwrap();
    let probe = LimitProbe::Recorded {
        root: fixture.user_home().to_owned(),
        body,
        status: 200,
        retry_after: None,
    };
    (login, probe)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opencode_session_without_openai_evidence_never_uses_its_saved_openai_login() {
    within(async {
        let fixture = Fixture::new();
        let bin = tempfile::tempdir().unwrap();
        let _installed = install(
            bin.path(),
            HarnessKind::OpenCode,
            Role::Structured,
            &vessel_testkit::Fake::Acp.path().unwrap(),
        )
        .unwrap();
        let (login, probe) = saved_login(&fixture);
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = Lookup::new([bin.path().to_owned()]);
                config.limits = probe;
            })
            .await;
        let owner = fixture.connect(&node).await;
        let mode = SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        };
        let spec = SessionSpec::new(HarnessKind::OpenCode, mode).with_cwd(fixture.cwd());
        let session = owner.create_session(spec).await.unwrap().id;
        let mut stream = owner.follow(session, EventSeq::ZERO).await.unwrap();
        crate::support::read_until(&mut stream, &mut Vec::new(), |record| {
            crate::support::status_of(record) == Some(&SessionStatus::Running)
        })
        .await;
        let _saved = owner
            .usage_lookup(SetUsageLookup {
                policy: ExternalLookup::On,
            })
            .await
            .unwrap();
        let account = GetUsageLimits {
            scope: LimitsScope::Account {
                account: LimitAccount {
                    harness: HarnessKind::OpenCode.into(),
                    profile: None,
                },
            },
            refresh: LimitRefresh::Refresh,
        };
        assert!(matches!(
            owner.usage_limits(account).await.unwrap().state,
            AccountLimitState::Ready { .. }
        ));
        // The fake's unqualified `cheap` model proves no provider. Deleting
        // the file distinguishes refusal BEFORE a local saved-login read.
        std::fs::remove_file(login).unwrap();
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
                retry_at: None,
            }
        );
        node.stop().await.unwrap();
    })
    .await;
}
