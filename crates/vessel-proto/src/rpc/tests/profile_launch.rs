//! ADR 0117: a profile launch and a settings change of the profiles need
//! `TERMINAL_PROFILES_PROTOCOL`; without them the calls are as they were.
use super::*;
use crate::SessionId;
use crate::settings::SetNodeSettings;
use crate::terminal_profiles::{Id, ProfileLaunch, Profiles, TERMINAL_PROFILES_PROTOCOL};
use crate::terminals::{CreateTerminal, TerminalPlace};

#[test]
fn a_profile_launch_and_a_profiles_change_need_their_protocol() {
    let create = |profile| {
        CreateTerminal {
            place: TerminalPlace::Session {
                session: SessionId::random(),
            },
            size: None,
            title: None,
            profile,
        }
        .into_call()
    };
    assert_eq!(create(None).since(), Some(ProtocolVersion::new(11)));
    let launch = ProfileLaunch {
        id: Id::parse("claude").unwrap(),
        prompt: None,
    };
    // Pending (`None`), it is refused; assigned, the later of the two.
    let shell = ProtocolVersion::new(11);
    assert_eq!(
        create(Some(launch)).since(),
        TERMINAL_PROFILES_PROTOCOL.map(|needed| needed.max(shell))
    );
    let change = SetNodeSettings {
        terminal_profiles: Some(Profiles::shipped().unwrap()),
        ..SetNodeSettings::default()
    };
    let plain = SetNodeSettings::default().into_call().since().unwrap();
    assert_eq!(
        change.into_call().since(),
        TERMINAL_PROFILES_PROTOCOL.map(|needed| needed.max(plain))
    );
}

#[test]
fn the_profiles_are_at_most_the_newest_protocol() {
    // Only the newest protocol asserts equality with `PROTOCOL_VERSION`:
    // ADR 0089's attention protocol is drafted after it.
    assert!(TERMINAL_PROFILES_PROTOCOL <= Some(crate::PROTOCOL_VERSION));
}

#[test]
fn a_launch_without_a_prompt_has_none_on_the_wire() {
    let launch = ProfileLaunch {
        id: Id::parse("codex").unwrap(),
        prompt: None,
    };
    assert_eq!(serde_json::to_string(&launch).unwrap(), r#"{"id":"codex"}"#);
    let back: ProfileLaunch = serde_json::from_str(r#"{"id":"codex","prompt":"fix it"}"#).unwrap();
    assert_eq!(back.prompt.unwrap().as_str(), "fix it");
}
