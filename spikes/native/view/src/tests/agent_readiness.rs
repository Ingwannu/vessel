//! Protocol 11 extends the page's discovery fixtures with explicit facts:
//! unknown sign-in never becomes success, and installs target one part.

use vessel_proto::agents::{AgentInfo, Installed, SignedIn};
use vessel_proto::{HarnessKind, HarnessRef};

use crate::Lang;
use crate::agent_status::readiness::{authentication, installation, installs, version};

fn installed(value: serde_json::Value) -> Installed {
    serde_json::from_value(value).unwrap()
}

#[test]
fn installation_is_a_sentence_separate_from_sign_in() {
    let available = installed(serde_json::json!({"state":"found", "path":"/bin/agent",
        "version":"1.2.3", "structured":{"state":"available"}}));
    assert_eq!(
        installation(Lang::En, &available),
        "Installed. Can run as a conversation."
    );
    assert_eq!(version(&available), Some("1.2.3"));
    assert_eq!(
        installation(Lang::Ko, &available),
        "설치되어 있으며 대화로 실행할 수 있습니다."
    );
    let adapter = installed(serde_json::json!({"state":"found", "path":"/bin/codex",
        "structured":{"state":"unavailable", "reason":"codex-acp is not installed"}}));
    assert_eq!(
        installation(Lang::En, &adapter),
        "Installed. Cannot run as a conversation: codex-acp is not installed"
    );
    let missing = installed(serde_json::json!({"state":"missing", "program":"missing-agent"}));
    assert!(installation(Lang::Ko, &missing).contains("설치되지 않았습니다"));
    assert_eq!(version(&missing), None);
    for state in ["too_old", "too_new"] {
        let incompatible = installed(serde_json::json!({"state":state, "path":"/bin/agent",
            "version":"0.1", "speaks":2,"needs":1}));
        assert_eq!(version(&incompatible), Some("0.1"));
        assert!(installation(Lang::En, &incompatible).contains("Cannot run as a conversation"));
        assert!(installation(Lang::Ko, &incompatible).contains("ACP 2"));
    }
}

#[test]
fn every_unknown_reason_and_failure_step_has_both_languages() {
    let mut reasons = vec![
        "not_installed",
        "not_checked",
        "in_use",
        "not_said",
        "no_home",
        "unread",
    ]
    .into_iter()
    .map(|reason| serde_json::json!({"reason":reason}))
    .collect::<Vec<_>>();
    reasons.push(serde_json::json!({"reason":"timed_out", "after":1000}));
    for step in ["spawn", "initialize", "session_new", "exited"] {
        reasons.push(serde_json::json!({"reason":"failed", "step":step}));
    }
    for reason in reasons {
        let signed: SignedIn =
            serde_json::from_value(serde_json::json!({"state":"unknown", "reason":reason}))
                .unwrap();
        let install = installed(serde_json::json!({"state":"unknown", "reason":reason}));
        assert!(authentication(Lang::En, &signed).starts_with("Sign-in is not known."));
        assert!(
            authentication(Lang::Ko, &signed).starts_with("로그인 여부를 확인하지 못했습니다.")
        );
        assert!(installation(Lang::En, &install).starts_with("Installation is not known."));
        assert!(installation(Lang::Ko, &install).starts_with("설치 여부를 확인하지 못했습니다."));
        assert_eq!(version(&install), None);
    }
    assert!(authentication(Lang::En, &SignedIn::Yes).contains("validity is checked when"));
    assert!(authentication(Lang::Ko, &SignedIn::Yes).contains("유효 여부"));
    assert_eq!(
        authentication(Lang::En, &SignedIn::NotRequired),
        "Sign-in is not needed for this model route."
    );
    let no = SignedIn::No {
        help: vessel_proto::agents::SignInHelp::default(),
    };
    assert!(authentication(Lang::En, &no).starts_with("Signed out."));
    assert!(authentication(Lang::Ko, &no).starts_with("로그아웃 상태입니다."));
}

#[test]
fn installer_offers_target_the_missing_part_only_and_never_guess_for_added_agents() {
    let agents: Vec<AgentInfo> = serde_json::from_value(serde_json::json!([
        {"agent":"codex", "detail":{"kind":"known", "installs":[
            {"part":"harness", "command":"npm install -g codex"},
            {"part":"adapter", "command":"npm install -g codex-acp"}
        ],"sign_in":{"methods":[]}}}
    ]))
    .unwrap();
    let agent = HarnessRef::Known(HarnessKind::Codex);
    let missing = installed(serde_json::json!({"state":"missing", "program":"codex"}));
    assert_eq!(
        installs(&agent, &missing, &agents)[0].command,
        "npm install -g codex"
    );
    let found = |state| {
        installed(serde_json::json!({"state":"found", "path":"/bin/codex", "structured":state}))
    };
    let adapter = found(serde_json::json!({"state":"unavailable", "reason":"missing adapter"}));
    assert_eq!(
        installs(&agent, &adapter, &agents)[0].command,
        "npm install -g codex-acp"
    );
    assert!(
        installs(
            &agent,
            &found(serde_json::json!({"state":"available"})),
            &agents
        )
        .is_empty()
    );
    let added: HarnessRef = serde_json::from_value(serde_json::json!({"added":"example"})).unwrap();
    assert!(installs(&added, &missing, &agents).is_empty());
    for state in ["too_old", "too_new"] {
        let mismatch = installed(
            serde_json::json!({"state":state, "path":"/bin/codex", "speaks":2, "needs":1}),
        );
        assert!(installs(&agent, &mismatch, &agents).is_empty());
    }
    let unknown =
        installed(serde_json::json!({"state":"unknown", "reason":{"reason":"not_checked"}}));
    assert!(installs(&agent, &unknown, &agents).is_empty());
}

#[test]
fn installation_progress_and_each_failure_are_sentences_in_both_languages() {
    use vessel_proto::agents::InstallState;
    let cases = [
        (InstallState::Running, "Installing…", "설치 중…"),
        (
            InstallState::Succeeded,
            "Installation finished.",
            "설치가 완료되었습니다.",
        ),
        (
            InstallState::Failed { code: Some(7) },
            "Installation failed with exit code 7.",
            "설치에 실패했습니다. 종료 코드: 7.",
        ),
        (
            InstallState::Failed { code: None },
            "Installation failed or was stopped.",
            "설치에 실패했거나 중지되었습니다.",
        ),
        (
            InstallState::TimedOut,
            "Installation took too long and was stopped.",
            "설치 시간이 초과되어 중지되었습니다.",
        ),
    ];
    for (state, en, ko) in cases {
        assert_eq!(Lang::En.agents().install_state(state), en);
        assert_eq!(Lang::Ko.agents().install_state(state), ko);
    }
}
