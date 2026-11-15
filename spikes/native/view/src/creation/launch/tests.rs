//! Reference launch distinctions never discard a prepared chat payload.

use super::*;
use vessel_proto::terminal_profiles::{Argument, Name, Profile, Program};

#[test]
fn empty_text_alone_creates_no_chat_but_context_or_files_keep_chat_intent() {
    let profiles = Profiles::new(Vec::new()).unwrap();
    assert_eq!(
        Launch::default()
            .prepare(" \n ", PayloadPresence::Empty, &profiles)
            .unwrap(),
        LaunchPlan::EmptyWorkspace
    );
    assert_eq!(
        Launch::Chat
            .prepare(" \n ", PayloadPresence::Present, &profiles)
            .unwrap(),
        LaunchPlan::Chat(Arc::from(" \n "))
    );
    assert_eq!(
        Launch::Chat
            .prepare(" original \n", PayloadPresence::Empty, &profiles)
            .unwrap(),
        LaunchPlan::Chat(Arc::from(" original \n"))
    );
}

#[test]
fn blank_shell_types_trimmed_text_and_one_cr_without_using_a_profile() {
    let profiles = Profiles::new(Vec::new()).unwrap();
    assert_eq!(
        Launch::BlankTerminal
            .prepare(" \n ", PayloadPresence::Empty, &profiles)
            .unwrap(),
        LaunchPlan::BlankTerminal(None)
    );
    let LaunchPlan::BlankTerminal(Some(input)) = Launch::BlankTerminal
        .prepare("  printf 'a b' | cat\n", PayloadPresence::Empty, &profiles)
        .unwrap()
    else {
        panic!("blank shell expected");
    };
    assert_eq!(input.terminal_data(), "printf 'a b' | cat\r");
    assert!(!format!("{input:?}").contains("printf"));
}

#[test]
fn removed_profile_falls_back_only_before_submit_and_never_silently_at_prepare() {
    let profiles = Profiles::new(Vec::new()).unwrap();
    let selected = Launch::Profile(Id::parse("removed").unwrap());
    assert_eq!(selected.reconciled(&profiles), Launch::Chat);
    assert_eq!(
        selected.prepare("text", PayloadPresence::Empty, &profiles),
        Err(LaunchProblem::MissingProfile)
    );
    assert_eq!(
        Launch::BlankTerminal.reconciled(&profiles),
        Launch::BlankTerminal
    );
}

#[test]
fn terminal_targets_refuse_chat_payload_and_profile_argv_never_becomes_shell_input() {
    let profiles = Profiles::shipped().unwrap();
    let selected = Launch::Profile(profiles.entries()[0].id().clone());
    assert_eq!(
        selected.prepare("text", PayloadPresence::Present, &profiles),
        Err(LaunchProblem::ChatPayload)
    );
    assert_eq!(
        Launch::BlankTerminal.prepare("text", PayloadPresence::Present, &profiles),
        Err(LaunchProblem::ChatPayload)
    );
    let LaunchPlan::Profile {
        command,
        naming_prompt,
        ..
    } = selected
        .prepare("literal $(touch x)", PayloadPresence::Empty, &profiles)
        .unwrap()
    else {
        panic!("profile expected");
    };
    assert_eq!(naming_prompt.as_ref(), "literal $(touch x)");
    assert!(
        command
            .args()
            .iter()
            .any(|argument| argument.as_str().contains("literal $(touch x)"))
    );
}

#[test]
fn a_profile_without_a_placeholder_has_no_naming_prompt_and_preserves_argv() {
    let profile = Profile::new(
        Id::parse("custom").unwrap(),
        Name::parse("Custom").unwrap(),
        Program::parse("tool").unwrap(),
        vec![Argument::parse("fixed arg").unwrap()],
        None,
    )
    .unwrap();
    let selected = Launch::Profile(profile.id().clone());
    let profiles = Profiles::new(vec![profile]).unwrap();
    let plan = selected
        .prepare("private text", PayloadPresence::Empty, &profiles)
        .unwrap();
    assert!(!format!("{plan:?}").contains("private text"));
    let LaunchPlan::Profile {
        command,
        naming_prompt,
        ..
    } = plan
    else {
        panic!("profile expected");
    };
    assert_eq!(naming_prompt.as_ref(), "");
    assert_eq!(command.args()[0].as_str(), "fixed arg");
}
