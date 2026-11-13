//! First-screen rules, beside the complete readiness sentence fixtures.
use super::{HarnessRef, Lang, MachineMessage, agent, installed, offers, readiness};

#[test]
fn a_missing_program_names_install_only_when_the_machine_offers_it() {
    let missing = installed(serde_json::json!({"state":"missing","program":"codex"}));
    let added: HarnessRef = serde_json::from_value(serde_json::json!({"added":"example"})).unwrap();
    for (lang, plain, offered) in [
        (
            Lang::En,
            "Not installed on this machine.",
            "Not installed on this machine. Select “Install” below.",
        ),
        (
            Lang::Ko,
            "이 머신에 설치되지 않았습니다.",
            "이 머신에 설치되지 않았습니다. 아래의 “설치” 버튼을 선택하세요.",
        ),
    ] {
        assert_eq!(
            readiness::installation(lang, &agent(), &missing, &[]).sentence,
            plain
        );
        assert_eq!(
            readiness::installation(lang, &agent(), &missing, &offers()).sentence,
            offered
        );
        assert_eq!(
            readiness::installation(lang, &added, &missing, &offers()).sentence,
            plain
        );
    }
}

#[test]
fn defaults_are_offered_only_by_a_compatible_installed_chat_program() {
    for (json, expected) in [
        (
            serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"available"}}),
            true,
        ),
        (
            serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"unavailable","reason":"no adapter"}}),
            false,
        ),
        (
            serde_json::json!({"state":"missing","program":"codex"}),
            false,
        ),
        (
            serde_json::json!({"state":"too_old","path":"/bin/codex","speaks":0,"needs":1}),
            false,
        ),
        (
            serde_json::json!({"state":"too_new","path":"/bin/codex","speaks":2,"needs":1}),
            false,
        ),
        (
            serde_json::json!({"state":"unknown","reason":{"reason":"not_checked"}}),
            false,
        ),
    ] {
        assert_eq!(readiness::offers_defaults(&installed(json)), expected);
    }
}

#[test]
fn raw_machine_diagnostics_are_informational_not_readiness_verdicts() {
    let message = MachineMessage {
        label: "Message from machine",
        text: "raw diagnostic",
    };
    assert_eq!(message.variant(), crate::alert::Variant::Info);
}

#[test]
fn list_summaries_do_not_name_controls_that_only_the_detail_view_shows() {
    let cases = [
        (
            serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"available"}}),
            "Installed. Can chat.",
            "설치되어 있으며 대화할 수 있습니다.",
        ),
        (
            serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"unavailable","reason":"missing adapter"}}),
            "Installed, but its chat adapter is missing.",
            "설치되어 있지만 대화에 필요한 어댑터가 없습니다.",
        ),
        (
            serde_json::json!({"state":"missing","program":"codex"}),
            "Not installed on this machine.",
            "이 머신에 설치되지 않았습니다.",
        ),
        (
            serde_json::json!({"state":"too_old","path":"/bin/codex","speaks":0,"needs":1}),
            "This agent is older than this version of Vessel supports. Update the agent.",
            "이 에이전트는 현재 Vessel이 지원하는 버전보다 오래되었습니다. 에이전트를 업데이트하세요.",
        ),
        (
            serde_json::json!({"state":"too_new","path":"/bin/codex","speaks":2,"needs":1}),
            "This agent is newer than this version of Vessel supports. Update Vessel.",
            "이 에이전트는 현재 Vessel이 지원하는 버전보다 최신입니다. Vessel을 업데이트하세요.",
        ),
        (
            serde_json::json!({"state":"unknown","reason":{"reason":"not_checked"}}),
            "Installation is not known.",
            "설치 여부를 확인하지 못했습니다.",
        ),
        (
            serde_json::json!({"state":"unknown","reason":{"reason":"failed","step":"initialize"}}),
            "Could not check this agent.",
            "에이전트를 확인하지 못했습니다.",
        ),
    ];
    for (json, en, ko) in cases {
        let state = installed(json);
        assert_eq!(
            readiness::summary(Lang::En, &agent(), &state, &offers()),
            en
        );
        assert_eq!(
            readiness::summary(Lang::Ko, &agent(), &state, &offers()),
            ko
        );
    }
    let unavailable = installed(
        serde_json::json!({"state":"found","path":"/bin/codex","structured":{"state":"unavailable","reason":"unknown"}}),
    );
    assert_eq!(
        readiness::summary(Lang::En, &agent(), &unavailable, &[]),
        "Installed, but it cannot chat on this machine."
    );
    assert_eq!(
        readiness::summary(Lang::Ko, &agent(), &unavailable, &[]),
        "설치되어 있지만 이 머신에서는 대화할 수 없습니다."
    );
}

#[test]
fn list_sign_in_facts_remain_separate_from_the_installation_badge() {
    for (json, en, ko) in [
        (
            serde_json::json!({"state":"yes"}),
            "Sign-in details saved.",
            "로그인 정보가 저장되어 있습니다.",
        ),
        (
            serde_json::json!({"state":"no","help":{"methods":[]}}),
            "Signed out.",
            "로그아웃 상태입니다.",
        ),
        (
            serde_json::json!({"state":"not_required"}),
            "No sign-in needed.",
            "로그인이 필요하지 않습니다.",
        ),
        (
            serde_json::json!({"state":"unknown","reason":{"reason":"timed_out","after":30000}}),
            "Sign-in check took too long.",
            "로그인 확인 시간이 초과되었습니다.",
        ),
    ] {
        let status = serde_json::from_value(json).unwrap();
        assert_eq!(readiness::authentication_summary(Lang::En, &status), en);
        assert_eq!(readiness::authentication_summary(Lang::Ko, &status), ko);
    }
    for reason in [
        "not_installed",
        "not_checked",
        "in_use",
        "not_said",
        "no_home",
        "unread",
    ] {
        let status = serde_json::from_value(
            serde_json::json!({"state":"unknown","reason":{"reason":reason}}),
        )
        .unwrap();
        assert_eq!(
            readiness::authentication_summary(Lang::En, &status),
            "Sign-in is not known."
        );
        assert_eq!(
            readiness::authentication_summary(Lang::Ko, &status),
            "로그인 여부를 확인하지 못했습니다."
        );
    }
    for step in ["spawn", "initialize", "session_new", "exited"] {
        let status = serde_json::from_value(
            serde_json::json!({"state":"unknown","reason":{"reason":"failed","step":step}}),
        )
        .unwrap();
        assert_eq!(
            readiness::authentication_summary(Lang::En, &status),
            "Sign-in check failed."
        );
        assert_eq!(
            readiness::authentication_summary(Lang::Ko, &status),
            "로그인 확인에 실패했습니다."
        );
    }
}

#[test]
fn an_empty_install_log_never_claims_a_completed_job_is_still_waiting() {
    assert_eq!(
        Lang::En.agents().install_empty(),
        "No installer output was received."
    );
    assert_eq!(
        Lang::Ko.agents().install_empty(),
        "설치 프로그램에서 받은 출력이 없습니다."
    );
}

#[test]
fn missing_cached_facts_are_explained_without_internal_cache_terms() {
    assert_eq!(
        Lang::En.agents().status_unread(),
        "The machine has not reported this agent’s status yet."
    );
    assert_eq!(
        Lang::Ko.agents().status_unread(),
        "머신이 아직 이 에이전트의 상태를 알려주지 않았습니다."
    );
}
