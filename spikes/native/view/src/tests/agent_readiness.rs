//! Full English/Korean sentences for each protocol readiness state. The
//! discovery cases in `agent_status.rs` remain the page's existing fixtures.

use crate::Lang;
use crate::agent_status::readiness::{self, AgentBadge, MachineMessage};
use crate::badge::Variant;
use vessel_proto::agents::{AgentInfo, Installed, SignedIn};
use vessel_proto::{HarnessKind, HarnessRef};

fn installed(value: serde_json::Value) -> Installed {
    serde_json::from_value(value).unwrap()
}
fn agent() -> HarnessRef {
    HarnessRef::Known(HarnessKind::Codex)
}
fn offers() -> Vec<AgentInfo> {
    serde_json::from_value(serde_json::json!([
        {"agent":"codex", "detail":{"kind":"known", "installs":[
            {"part":"harness", "command":"npm install -g codex"},
            {"part":"adapter", "command":"npm install -g codex-acp"}
        ],"sign_in":{"methods":[]}}}
    ]))
    .unwrap()
}

#[test]
fn installation_states_have_exact_badges_and_complete_instructions_in_both_languages() {
    let cases = [
        (
            serde_json::json!({"state":"found","path":"/bin/codex","version":"1.2.3","structured":{"state":"available"}}),
            AgentBadge::Available,
            Variant::Success,
            "Available",
            "사용 가능",
            "Installed. Can chat.",
            "설치되어 있으며 대화할 수 있습니다.",
            Some("1.2.3"),
        ),
        (
            serde_json::json!({"state":"missing","program":"codex"}),
            AgentBadge::NotInstalled,
            Variant::Muted,
            "Not installed",
            "미설치",
            "Not installed on this machine. Select “Install” below.",
            "이 머신에 설치되지 않았습니다. 아래의 “설치” 버튼을 선택하세요.",
            None,
        ),
        (
            serde_json::json!({"state":"too_old","path":"/bin/codex","version":"0.1","speaks":0,"needs":1}),
            AgentBadge::NeedsAttention,
            Variant::Warning,
            "Needs attention",
            "확인 필요",
            "This agent is older than this version of Vessel supports. Update the agent. Select “Check status” to check again.",
            "이 에이전트는 현재 Vessel이 지원하는 버전보다 오래되었습니다. 에이전트를 업데이트하세요. “상태 확인” 버튼을 선택해 다시 확인하세요.",
            Some("0.1"),
        ),
        (
            serde_json::json!({"state":"too_new","path":"/bin/codex","version":"9.0","speaks":2,"needs":1}),
            AgentBadge::NeedsAttention,
            Variant::Warning,
            "Needs attention",
            "확인 필요",
            "This agent is newer than this version of Vessel supports. Update Vessel.",
            "이 에이전트는 현재 Vessel이 지원하는 버전보다 최신입니다. Vessel을 업데이트하세요.",
            Some("9.0"),
        ),
    ];
    for (json, badge, variant, en_label, ko_label, en, ko, version) in cases {
        let installed = installed(json);
        assert_eq!(AgentBadge::of(&installed), badge);
        assert_eq!(readiness::version(&installed), version);
        assert_eq!(
            readiness::program(&installed).as_deref(),
            Some(if badge == AgentBadge::NotInstalled {
                "codex"
            } else {
                "/bin/codex"
            })
        );
        for (lang, label, sentence) in [(Lang::En, en_label, en), (Lang::Ko, ko_label, ko)] {
            let shown = readiness::installation(lang, &agent(), &installed, &offers());
            assert_eq!(shown.badge.label, label);
            assert_eq!(shown.badge.variant, variant);
            assert_eq!(shown.sentence, sentence);
            assert_eq!(shown.machine_message, None);
        }
    }
}

#[test]
fn a_machine_diagnostic_is_attributed_separately_and_an_adapter_instruction_needs_an_offer() {
    let installed = installed(serde_json::json!({"state":"found","path":"/bin/codex",
        "structured":{"state":"unavailable","reason":"codex-acp is not installed"}}));
    let cases = [
        (
            Lang::En,
            "Needs attention",
            "Installed, but its chat adapter is missing. Select “Install adapter” below.",
            "Installed, but it cannot chat on this machine.",
            "Message from machine",
        ),
        (
            Lang::Ko,
            "확인 필요",
            "설치되어 있지만 대화에 필요한 어댑터가 없습니다. 아래의 “어댑터 설치” 버튼을 선택하세요.",
            "설치되어 있지만 이 머신에서는 대화할 수 없습니다.",
            "머신 메시지",
        ),
    ];
    for (lang, badge, with_offer, without_offer, label) in cases {
        let shown = readiness::installation(lang, &agent(), &installed, &offers());
        assert_eq!(
            shown.badge,
            readiness::Badge {
                label: badge,
                variant: Variant::Warning
            }
        );
        assert_eq!(shown.sentence, with_offer);
        assert_eq!(
            shown.machine_message,
            Some(MachineMessage {
                label,
                text: "codex-acp is not installed"
            })
        );
        let shown = readiness::installation(lang, &agent(), &installed, &[]);
        assert_eq!(shown.sentence, without_offer);
        assert_eq!(
            shown.machine_message,
            Some(MachineMessage {
                label,
                text: "codex-acp is not installed"
            })
        );
    }
}

fn unknown_cases() -> [(serde_json::Value, AgentBadge, &'static str, &'static str); 11] {
    [
        (
            serde_json::json!({"reason":"not_installed"}),
            AgentBadge::Unknown,
            "The agent is not installed on this machine.",
            "이 머신에 에이전트가 설치되지 않았습니다.",
        ),
        (
            serde_json::json!({"reason":"not_checked"}),
            AgentBadge::Unknown,
            "Select “Check status” to check it.",
            "“상태 확인” 버튼을 선택해 확인하세요.",
        ),
        (
            serde_json::json!({"reason":"in_use"}),
            AgentBadge::Unknown,
            "A chat is using this account. Select “Check status” after it ends.",
            "이 계정을 사용하는 대화가 있습니다. 대화가 끝난 뒤 “상태 확인” 버튼을 선택하세요.",
        ),
        (
            serde_json::json!({"reason":"not_said"}),
            AgentBadge::Unknown,
            "The agent did not provide this information during the check.",
            "에이전트가 확인 과정에서 이 정보를 알려주지 않았습니다.",
        ),
        (
            serde_json::json!({"reason":"no_home"}),
            AgentBadge::Unknown,
            "Set an absolute account-folder path in profiles.toml on this machine, then select “Check status”.",
            "이 머신의 profiles.toml에서 계정 폴더를 절대 경로로 설정한 뒤 “상태 확인” 버튼을 선택하세요.",
        ),
        (
            serde_json::json!({"reason":"unread"}),
            AgentBadge::Unknown,
            "The machine could not understand the agent's answer. Update the agent, then select “Check status”.",
            "머신이 에이전트의 응답을 해석하지 못했습니다. 에이전트를 업데이트한 뒤 “상태 확인” 버튼을 선택하세요.",
        ),
        (
            serde_json::json!({"reason":"timed_out","after":1000}),
            AgentBadge::Error,
            "The check took too long. Select “Check status” to try again.",
            "확인 시간이 초과되었습니다. “상태 확인” 버튼을 선택해 다시 시도하세요.",
        ),
        (
            serde_json::json!({"reason":"failed","step":"spawn"}),
            AgentBadge::Error,
            "The agent could not start. Check that it is installed, then select “Check status”.",
            "에이전트를 시작하지 못했습니다. 설치 여부를 확인한 뒤 “상태 확인” 버튼을 선택하세요.",
        ),
        (
            serde_json::json!({"reason":"failed","step":"initialize"}),
            AgentBadge::Error,
            "Could not connect to the agent. Select “Check status” to try again.",
            "에이전트에 연결하지 못했습니다. “상태 확인” 버튼을 선택해 다시 시도하세요.",
        ),
        (
            serde_json::json!({"reason":"failed","step":"session_new"}),
            AgentBadge::Error,
            "The agent could not open a chat for the check. Select “Check status” to try again.",
            "확인을 위한 대화를 열지 못했습니다. “상태 확인” 버튼을 선택해 다시 시도하세요.",
        ),
        (
            serde_json::json!({"reason":"failed","step":"exited"}),
            AgentBadge::Error,
            "The agent closed before answering. Select “Check status” to try again.",
            "에이전트가 응답하기 전에 종료되었습니다. “상태 확인” 버튼을 선택해 다시 시도하세요.",
        ),
    ]
}

#[test]
fn every_unknown_reason_has_its_exact_body_in_both_authentication_and_installation() {
    for (reason, badge, en, ko) in unknown_cases() {
        let installed = installed(serde_json::json!({"state":"unknown","reason":reason}));
        let signed: SignedIn =
            serde_json::from_value(serde_json::json!({"state":"unknown","reason":reason})).unwrap();
        assert_eq!(AgentBadge::of(&installed), badge);
        let variant = match badge {
            AgentBadge::Unknown => Variant::Muted,
            AgentBadge::Error => Variant::Error,
            AgentBadge::Available | AgentBadge::NotInstalled | AgentBadge::NeedsAttention => {
                panic!("fixture's unknown-state badge")
            }
        };
        for (lang, install, auth, label) in [
            (
                Lang::En,
                format!("Installation is not known. {en}"),
                format!("Sign-in is not known. {en}"),
                if badge == AgentBadge::Error {
                    "Error"
                } else {
                    "Not known"
                },
            ),
            (
                Lang::Ko,
                format!("설치 여부를 확인하지 못했습니다. {ko}"),
                format!("로그인 여부를 확인하지 못했습니다. {ko}"),
                if badge == AgentBadge::Error {
                    "오류"
                } else {
                    "확인 안 됨"
                },
            ),
        ] {
            let shown = readiness::installation(lang, &agent(), &installed, &[]);
            assert_eq!(shown.sentence, install);
            assert_eq!(shown.badge, readiness::Badge { label, variant });
            assert_eq!(shown.machine_message, None);
            assert_eq!(readiness::authentication(lang, &signed), auth);
            assert_eq!(readiness::version(&installed), None);
        }
    }
}

#[test]
fn authentication_verdicts_are_exact_and_do_not_claim_credentials_were_validated() {
    let cases = [
        (
            SignedIn::Yes,
            "Sign-in details are saved. They will be checked when you send a message.",
            "로그인 정보가 저장되어 있습니다. 메시지를 보낼 때 유효 여부를 확인합니다.",
        ),
        (
            SignedIn::NotRequired,
            "No sign-in needed. This profile uses a model provider configured on this machine.",
            "로그인이 필요하지 않습니다. 이 프로필은 머신에 설정된 모델 공급자를 사용합니다.",
        ),
        (
            SignedIn::No {
                help: vessel_proto::agents::SignInHelp::default(),
            },
            "Signed out. Follow the sign-in steps below.",
            "로그아웃 상태입니다. 아래 단계에 따라 로그인하세요.",
        ),
    ];
    for (state, en, ko) in cases {
        assert_eq!(readiness::authentication(Lang::En, &state), en);
        assert_eq!(readiness::authentication(Lang::Ko, &state), ko);
    }
}

#[test]
fn installer_offers_target_the_missing_part_and_never_guess_for_added_or_unknown_agents() {
    let offers = offers();
    let missing = installed(serde_json::json!({"state":"missing","program":"codex"}));
    assert_eq!(
        readiness::installs(&agent(), &missing, &offers)[0].command,
        "npm install -g codex"
    );
    let adapter = installed(
        serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"unavailable","reason":"missing adapter"}}),
    );
    assert_eq!(
        readiness::installs(&agent(), &adapter, &offers)[0].command,
        "npm install -g codex-acp"
    );
    let available = installed(
        serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"available"}}),
    );
    assert!(readiness::installs(&agent(), &available, &offers).is_empty());
    let added: HarnessRef = serde_json::from_value(serde_json::json!({"added":"example"})).unwrap();
    assert!(readiness::installs(&added, &missing, &offers).is_empty());
    for state in ["too_old", "too_new"] {
        let mismatch =
            installed(serde_json::json!({"state":state,"path":"/bin/codex","speaks":2,"needs":1}));
        assert!(readiness::installs(&agent(), &mismatch, &offers).is_empty());
    }
    let unknown =
        installed(serde_json::json!({"state":"unknown","reason":{"reason":"not_checked"}}));
    assert!(readiness::installs(&agent(), &unknown, &offers).is_empty());
}

#[test]
fn installation_progress_and_each_failure_have_exact_sentences_in_both_languages() {
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

mod screen;
