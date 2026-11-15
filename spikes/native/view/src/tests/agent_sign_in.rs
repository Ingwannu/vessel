//! Login help retains the node's account selector, exact command and
//! methods. No login command is invented from an agent name.

use crate::Lang;
use crate::agent_status::readiness::{SignInRow, SignInStep, sign_in_help};
use vessel_proto::agents::{SignInHelp, SignedIn};

#[test]
fn command_account_and_methods_become_labelled_steps_in_both_languages() {
    let state: SignedIn = serde_json::from_value(serde_json::json!({
        "state":"no", "help":{
            "command":"agent auth login --device", "home":"AGENT_HOME",
            "methods":[{"id":"browser","name":"Browser login","description":"The agent's explanation"},
                       {"id":"key","name":"API key"}]
        }
    })).unwrap();
    let SignedIn::No { help } = &state else {
        panic!("signed out fixture")
    };
    let command = help.command.as_deref().unwrap();
    let home = help.home.as_ref().unwrap();
    for (
        lang,
        account_label,
        account_instruction,
        command_label,
        command_instruction,
        method_label,
        method_instruction,
    ) in [
        (
            Lang::En,
            "Account folder",
            "Use this profile's account folder, selected by the variable below. Its value stays on the machine.",
            "Sign-in command",
            "Run this command on the selected machine, using this profile's account.",
            "Sign-in method from machine",
            "Open the agent on this machine and choose the sign-in method shown below.",
        ),
        (
            Lang::Ko,
            "계정 폴더",
            "아래 변수가 선택하는 프로필의 계정 폴더를 사용하세요. 변수 값은 머신에만 보관됩니다.",
            "로그인 명령",
            "선택한 머신에서 이 프로필의 계정으로 아래 명령을 실행하세요.",
            "머신이 알려준 로그인 방법",
            "이 머신에서 에이전트를 열고 아래 로그인 방법을 선택하세요.",
        ),
    ] {
        assert_eq!(
            sign_in_help(lang, &state),
            [
                SignInRow {
                    label: account_label,
                    instruction: account_instruction,
                    step: SignInStep::AccountFolder(home)
                },
                SignInRow {
                    label: command_label,
                    instruction: command_instruction,
                    step: SignInStep::Command(command)
                },
                SignInRow {
                    label: method_label,
                    instruction: method_instruction,
                    step: SignInStep::Method(&help.methods[0])
                },
                SignInRow {
                    label: method_label,
                    instruction: method_instruction,
                    step: SignInStep::Method(&help.methods[1])
                },
            ]
        );
    }
}

#[test]
fn no_command_or_method_falls_back_to_the_agents_interface_without_guessing() {
    let state = SignedIn::No {
        help: SignInHelp::default(),
    };
    assert_eq!(
        sign_in_help(Lang::En, &state),
        [SignInRow {
            label: "Open the agent",
            instruction: "Open the agent on this machine and sign in there.",
            step: SignInStep::OpenAgent
        }]
    );
    assert_eq!(
        sign_in_help(Lang::Ko, &state),
        [SignInRow {
            label: "에이전트 열기",
            instruction: "이 머신에서 에이전트를 열고 로그인하세요.",
            step: SignInStep::OpenAgent
        }]
    );
    for lang in [Lang::En, Lang::Ko] {
        assert!(sign_in_help(lang, &SignedIn::Yes).is_empty());
        assert!(sign_in_help(lang, &SignedIn::NotRequired).is_empty());
        assert!(
            sign_in_help(
                lang,
                &SignedIn::Unknown {
                    reason: vessel_proto::agents::UnknownReason::NotChecked
                }
            )
            .is_empty()
        );
    }
}

#[test]
fn a_command_alone_or_a_method_alone_does_not_add_an_unnecessary_fallback() {
    let command: SignedIn =
        serde_json::from_value(serde_json::json!({"state":"no","help":{"command":"sign-in"}}))
            .unwrap();
    let method: SignedIn = serde_json::from_value(
        serde_json::json!({"state":"no","help":{"methods":[{"id":"own","name":"Own method"}]}}),
    )
    .unwrap();
    for lang in [Lang::En, Lang::Ko] {
        let command = sign_in_help(lang, &command);
        assert_eq!(command.len(), 1);
        assert_eq!(command[0].step, SignInStep::Command("sign-in"));
        let method = sign_in_help(lang, &method);
        assert_eq!(method.len(), 1);
        assert!(matches!(method[0].step, SignInStep::Method(_)));
    }
}
