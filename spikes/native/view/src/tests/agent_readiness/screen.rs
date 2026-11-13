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
