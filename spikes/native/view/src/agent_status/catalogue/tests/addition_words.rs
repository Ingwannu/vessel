//! Whole form/action instructions, independent from their visible field inputs.
use crate::agent_status::{
    addition::{Failure, Refused},
    pages::StartProblem,
};
use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};

#[test]
fn every_add_form_refusal_has_its_whole_instruction_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    let invalid_name = "has spaces"
        .parse::<vessel_proto::agents::AgentName>()
        .unwrap_err();
    let invalid_program = ""
        .parse::<vessel_proto::agents::AgentProgram>()
        .unwrap_err();
    for (problem, english, korean) in [
        (
            Refused::Name(invalid_name),
            "Choose another provider name. Use 1–64 letters, numbers, dots, underscores or hyphens; built-in names are reserved.",
            "다른 프로바이더 이름을 입력하세요. 영문자, 숫자, 점, 밑줄, 하이픈을 1~64자로 사용할 수 있으며 기본 제공 이름은 예약되어 있습니다.",
        ),
        (
            Refused::Taken,
            "A provider already uses this name on the machine. Choose another name.",
            "이 머신에서 이미 사용 중인 프로바이더 이름입니다. 다른 이름을 입력하세요.",
        ),
        (
            Refused::Program(invalid_program),
            "Enter a valid program name or absolute path. Put its arguments in the separate field.",
            "올바른 프로그램 이름 또는 절대 경로를 입력하세요. 인수는 별도 입력란에 입력하세요.",
        ),
        (
            Refused::MachineChanged,
            "The machine or registry entry changed. Close this form and open it again from the current list.",
            "머신 또는 레지스트리 항목이 바뀌었습니다. 이 창을 닫고 현재 목록에서 다시 여세요.",
        ),
        (
            Refused::Changed,
            "The machine or registry entry changed. Close this form and open it again from the current list.",
            "머신 또는 레지스트리 항목이 바뀌었습니다. 이 창을 닫고 현재 목록에서 다시 여세요.",
        ),
        (
            Refused::Unavailable,
            "Read the machine’s provider list before adding a provider. Close this form and select “Refresh”.",
            "프로바이더를 추가하기 전에 머신의 목록을 읽어야 합니다. 이 창을 닫고 “새로 고침” 버튼을 누르세요.",
        ),
        (
            Refused::Request(StartProblem::Busy),
            "Settings is busy. Try again when the current operation finishes.",
            "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
        ),
        (
            Refused::Request(StartProblem::Capacity),
            "Settings is busy. Try again when the current operation finishes.",
            "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
        ),
        (
            Refused::Request(StartProblem::Exhausted),
            "Settings cannot reserve another request. Restart Vessel before trying again.",
            "설정에서 새 요청을 만들 수 없습니다. Vessel을 다시 시작한 다음 시도하세요.",
        ),
    ] {
        assert_eq!(en.form_problem(&problem), english);
        assert_eq!(ko.form_problem(&problem), korean);
    }
}
#[test]
fn every_add_failure_has_its_whole_instruction_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    for (problem, english, korean) in [
        (
            Failure::Changed,
            "The machine behind this name changed. Close this form and select “Refresh” before trying again.",
            "이 이름으로 연결된 머신이 바뀌었습니다. 이 창을 닫고 “새로 고침” 버튼을 누른 다음 다시 시도하세요.",
        ),
        (
            Failure::Unsupported,
            "Update Vessel on this machine before adding a provider.",
            "프로바이더를 추가하기 전에 이 머신의 Vessel을 업데이트하세요.",
        ),
        (
            Failure::BuildUnavailable,
            "This version of Vessel cannot add providers yet. Update Vessel on this device before trying again.",
            "현재 Vessel 버전에서는 아직 프로바이더를 추가할 수 없습니다. 이 기기의 Vessel을 업데이트한 뒤 다시 시도하세요.",
        ),
        (
            Failure::Exhausted,
            "Settings cannot reserve another request. Restart Vessel before trying again.",
            "설정에서 새 요청을 만들 수 없습니다. Vessel을 다시 시작한 다음 시도하세요.",
        ),
        (
            Failure::Permission,
            "This device cannot add providers on the machine. Ask the machine’s administrator for access.",
            "이 기기에는 머신에 프로바이더를 추가할 권한이 없습니다. 머신 관리자에게 접근 권한을 요청하세요.",
        ),
        (
            Failure::Conflict,
            "The provider name or registry version conflicts with the machine’s current definitions. Read the list again and choose another name if needed.",
            "프로바이더 이름 또는 레지스트리 버전이 머신의 현재 정의와 충돌합니다. 목록을 다시 읽고 필요하면 다른 이름을 선택하세요.",
        ),
        (
            Failure::Rejected,
            "The machine could not add this provider. Check the name and command, then try again.",
            "머신에 프로바이더를 추가하지 못했습니다. 이름과 명령을 확인한 다음 다시 시도하세요.",
        ),
        (
            Failure::Unknown,
            "Could not confirm whether the provider was added. Close this form and select “Refresh” before trying again.",
            "프로바이더가 추가되었는지 확인하지 못했습니다. 이 창을 닫고 “새로 고침” 버튼을 누른 다음 다시 시도하세요.",
        ),
        (
            Failure::Offline,
            "The machine is not connected. Connect it before adding a provider.",
            "머신이 연결되어 있지 않습니다. 연결한 다음 프로바이더를 추가하세요.",
        ),
        (
            Failure::Busy,
            "Settings is busy. Try again when the current operation finishes.",
            "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
        ),
        (
            Failure::NoTransport,
            "This browser cannot connect to a machine yet.",
            "이 브라우저에서는 아직 머신에 연결할 수 없습니다.",
        ),
    ] {
        assert_eq!(en.addition_failure(problem), english);
        assert_eq!(ko.addition_failure(problem), korean);
    }
}

#[test]
fn every_local_removal_refusal_names_a_real_recovery_control() {
    use crate::agent_status::removal::Refused;
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    for (problem, english, korean) in [
        (
            Refused::RefreshRequired,
            "Select “Refresh” to read the current agent definitions before removing an agent.",
            "에이전트를 제거하기 전에 “새로 고침” 버튼을 눌러 현재 정의를 읽으세요.",
        ),
        (
            Refused::Unavailable,
            "Only added agent definitions reported by this machine can be removed.",
            "이 머신에서 보고한 추가 에이전트 정의만 제거할 수 있습니다.",
        ),
        (
            Refused::MachineChanged,
            "The machine or agent definition changed. Open the removal confirmation again from the current list.",
            "머신 또는 에이전트 정의가 바뀌었습니다. 현재 목록에서 제거 확인 창을 다시 여세요.",
        ),
        (
            Refused::NodeChanged,
            "The machine or agent definition changed. Open the removal confirmation again from the current list.",
            "머신 또는 에이전트 정의가 바뀌었습니다. 현재 목록에서 제거 확인 창을 다시 여세요.",
        ),
        (
            Refused::DefinitionChanged,
            "The machine or agent definition changed. Open the removal confirmation again from the current list.",
            "머신 또는 에이전트 정의가 바뀌었습니다. 현재 목록에서 제거 확인 창을 다시 여세요.",
        ),
        (
            Refused::Request(StartProblem::Busy),
            "Settings is busy. Try again when the current operation finishes.",
            "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
        ),
        (
            Refused::Request(StartProblem::Capacity),
            "Settings is busy. Try again when the current operation finishes.",
            "설정 작업이 진행 중입니다. 현재 작업이 끝난 뒤 다시 시도하세요.",
        ),
        (
            Refused::Request(StartProblem::Exhausted),
            "Settings cannot reserve another request. Restart Vessel before trying again.",
            "설정에서 새 요청을 만들 수 없습니다. Vessel을 다시 시작한 다음 시도하세요.",
        ),
    ] {
        assert_eq!(en.removal_refused(problem), english);
        assert_eq!(ko.removal_refused(problem), korean);
    }
    assert_eq!(
        en.remove_title("A long provider").unwrap(),
        "Remove \u{2068}A long provider\u{2069}?"
    );
    assert_eq!(
        ko.remove_title("긴 프로바이더").unwrap(),
        "\u{2068}긴 프로바이더\u{2069} 제거"
    );
    assert_eq!(
        en.remove_help(),
        "This removes the added agent definition from this machine. It cannot be undone."
    );
    assert_eq!(
        ko.remove_help(),
        "이 머신에 추가된 에이전트 정의를 제거합니다. 되돌릴 수 없습니다."
    );
}

#[test]
fn a_format_failure_is_one_complete_error_and_never_a_partial_confirmation() {
    use crate::agent_status::removal::{self, Notice};
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    let error = crate::localization::FormatError::Missing("agent-catalog-remove-title");
    assert_eq!(
        removal::notice(Some(removal::Refused::Unavailable), Some(&error)),
        Some(Notice::Words)
    );
    assert_eq!(removal::notice(None, None), None);
    assert_eq!(
        en.removal_notice(Notice::Words),
        "Could not prepare the removal confirmation. Change the app language, then try again."
    );
    assert_eq!(
        ko.removal_notice(Notice::Words),
        "제거 확인 창의 문구를 준비하지 못했습니다. 앱 언어를 변경한 뒤 다시 시도하세요."
    );
    assert_eq!(Notice::Words.variant(), crate::alert::Variant::Error);
    assert_eq!(
        Notice::Refused(removal::Refused::Request(StartProblem::Exhausted)).variant(),
        crate::alert::Variant::Error
    );
    for problem in [
        removal::Refused::RefreshRequired,
        removal::Refused::Unavailable,
        removal::Refused::MachineChanged,
        removal::Refused::NodeChanged,
        removal::Refused::DefinitionChanged,
        removal::Refused::Request(StartProblem::Busy),
        removal::Refused::Request(StartProblem::Capacity),
    ] {
        assert_eq!(
            Notice::Refused(problem).variant(),
            crate::alert::Variant::Warning
        );
    }
}
#[test]
fn form_validation_tones_distinguish_fields_from_context_and_capacity() {
    let name = "has spaces"
        .parse::<vessel_proto::agents::AgentName>()
        .unwrap_err();
    let program = ""
        .parse::<vessel_proto::agents::AgentProgram>()
        .unwrap_err();
    for problem in [
        Refused::Name(name),
        Refused::Taken,
        Refused::Program(program),
        Refused::Request(StartProblem::Exhausted),
    ] {
        assert_eq!(problem.variant(), crate::alert::Variant::Error);
    }
    for problem in [
        Refused::Unavailable,
        Refused::Changed,
        Refused::MachineChanged,
        Refused::Request(StartProblem::Busy),
        Refused::Request(StartProblem::Capacity),
    ] {
        assert_eq!(problem.variant(), crate::alert::Variant::Warning);
    }
}

#[test]
fn the_same_retained_name_rewords_as_a_whole_confirmation_without_changing_it() {
    let en = Translator::get(ENGLISH).unwrap().agent_catalog();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .agent_catalog();
    let name = "Research · 한글";
    assert_eq!(
        en.remove_title(name).unwrap(),
        "Remove \u{2068}Research · 한글\u{2069}?"
    );
    assert_eq!(
        ko.remove_title(name).unwrap(),
        "\u{2068}Research · 한글\u{2069} 제거"
    );
    assert_eq!(name, "Research · 한글");
}
