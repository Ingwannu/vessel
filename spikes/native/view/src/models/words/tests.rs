//! Whole bilingual sentences, including every typed read/refusal category.
//! Fluent message references reuse the actual control label without adding
//! variable-placeable bidi isolation (fluent-bundle resolver/pattern.rs).
use super::*;
use crate::localization::{ENGLISH, LanguagePreference, Models, Translator, resolve_language};
fn english() -> Models<'static> {
    Translator::get(ENGLISH).unwrap().models()
}
fn korean() -> Models<'static> {
    Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .models()
}
#[test]
fn every_read_failure_has_a_complete_sentence_in_each_language() {
    for (problem, en, ko) in [
        (
            Problem::Offline,
            "The machine is not connected. Connect it, then select “Refresh”.",
            "머신에 연결되지 않았습니다. 연결한 뒤 “새로 고침” 버튼을 누르세요.",
        ),
        (
            Problem::Busy,
            "Vessel is busy. Wait a moment, then select “Refresh”.",
            "Vessel이 요청을 처리하는 중입니다. 잠시 후 “새로 고침” 버튼을 누르세요.",
        ),
        (
            Problem::Timeout,
            "The machine did not finish reading models in time. Select “Refresh” to try again.",
            "머신이 제한 시간 안에 모델을 읽지 못했습니다. 다시 시도하려면 “새로 고침” 버튼을 누르세요.",
        ),
        (
            Problem::NoTransport,
            "This browser cannot connect to a machine yet.",
            "이 브라우저에서는 아직 머신에 연결할 수 없습니다.",
        ),
        (
            Problem::Denied,
            "This device cannot read models on this machine. Ask its administrator for access.",
            "이 기기에는 이 머신의 모델을 읽을 권한이 없습니다. 관리자에게 접근 권한을 요청하세요.",
        ),
        (
            Problem::Unsupported,
            "Update Vessel on this machine to read its configured models.",
            "구성된 모델을 읽으려면 이 머신의 Vessel을 업데이트하세요.",
        ),
        (
            Problem::Failed,
            "The machine could not read its models. Select “Refresh” to try again.",
            "머신이 모델을 읽지 못했습니다. 다시 시도하려면 “새로 고침” 버튼을 누르세요.",
        ),
    ] {
        assert_eq!(english().problem(problem), en);
        assert_eq!(korean().problem(problem), ko);
        assert_eq!(english().issue(Issue::Read(problem)), en);
        assert_eq!(korean().issue(Issue::Read(problem)), ko);
    }
}
#[test]
fn every_refused_read_has_a_complete_sentence_in_each_language() {
    for (problem, en, ko) in [
        (
            StartProblem::NoMachine,
            "Choose a machine to see its models.",
            "모델을 볼 머신을 선택하세요.",
        ),
        (
            StartProblem::Busy,
            "The current model read is still running. Wait for it to finish.",
            "현재 모델을 읽는 중입니다. 읽기가 끝날 때까지 기다리세요.",
        ),
        (
            StartProblem::Exhausted,
            "Model requests have reached their limit. Close and reopen Vessel.",
            "모델 요청 한도에 도달했습니다. Vessel을 닫았다가 다시 여세요.",
        ),
    ] {
        assert_eq!(english().start_problem(problem), en);
        assert_eq!(korean().start_problem(problem), ko);
        assert_eq!(english().issue(Issue::Start(problem)), en);
        assert_eq!(korean().issue(Issue::Start(problem)), ko);
    }
}
#[test]
fn public_api_labels_replace_wire_spellings_in_both_languages() {
    for words in [english(), korean()] {
        for (api, expected) in [
            (
                vessel_proto::ModelApi::AnthropicMessages,
                "Anthropic Messages",
            ),
            (vessel_proto::ModelApi::OpenAiResponses, "OpenAI Responses"),
            (
                vessel_proto::ModelApi::OpenAiChatCompletions,
                "OpenAI Chat Completions",
            ),
        ] {
            assert_eq!(words.api_name(api), expected);
        }
    }
}
#[test]
fn empty_loading_search_and_profile_use_never_claim_agent_compatibility() {
    let en = english();
    let ko = korean();
    for (actual, expected) in [
        (en.routes(), "Configured models"),
        (ko.routes(), "구성된 모델"),
        (en.loading(), "Loading models…"),
        (ko.loading(), "모델을 읽는 중…"),
        (en.empty(), "No models are configured on this machine."),
        (ko.empty(), "이 머신에 구성된 모델이 없습니다."),
        (en.no_matches(), "No models match your search."),
        (ko.no_matches(), "검색과 일치하는 모델이 없습니다."),
        (en.used_by(), "Used by profiles"),
        (ko.used_by(), "사용 중인 프로필"),
        (
            en.usage_unavailable(),
            "Update Vessel on this machine to see which profiles use each model.",
        ),
        (
            ko.usage_unavailable(),
            "모델을 사용하는 프로필을 보려면 이 머신의 Vessel을 업데이트하세요.",
        ),
    ] {
        assert_eq!(actual, expected);
    }
}

#[test]
fn read_buttons_keep_their_pending_words_and_failure_tone_in_view() {
    use crate::models::Refresh;
    for (state, en_refresh, ko_refresh, en_retry, ko_retry) in [
        (Refresh::Ready, "Refresh", "새로 고침", "Retry", "다시 시도"),
        (
            Refresh::Pending,
            "Refreshing…",
            "새로 고치는 중…",
            "Retrying…",
            "다시 시도하는 중…",
        ),
        (
            Refresh::Unavailable,
            "Refresh",
            "새로 고침",
            "Retry",
            "다시 시도",
        ),
    ] {
        assert_eq!(english().refresh_label(state), en_refresh);
        assert_eq!(korean().refresh_label(state), ko_refresh);
        assert_eq!(english().retry_label(state), en_retry);
        assert_eq!(korean().retry_label(state), ko_retry);
    }
    for issue in [
        Issue::Read(Problem::Offline),
        Issue::Read(Problem::Busy),
        Issue::Read(Problem::Timeout),
        Issue::Read(Problem::NoTransport),
        Issue::Read(Problem::Denied),
        Issue::Read(Problem::Unsupported),
        Issue::Read(Problem::Failed),
        Issue::Start(StartProblem::NoMachine),
        Issue::Start(StartProblem::Busy),
        Issue::Start(StartProblem::Exhausted),
    ] {
        assert_eq!(issue.variant(), crate::alert::Variant::Error);
    }
}
