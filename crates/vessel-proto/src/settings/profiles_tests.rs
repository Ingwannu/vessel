//! The terminal profiles field (ADR 0117): absent unless set, kept empty when
//! emptied, hidden below its protocol.

use super::*;
use crate::terminal_profiles::{Argument, Id, Name, Profile, Program};

fn mine() -> Profiles {
    Profiles::new(vec![
        Profile::new(
            Id::parse("profile_m1_a").unwrap(),
            Name::parse("Mine").unwrap(),
            Program::parse("codex").unwrap(),
            vec![Argument::parse("--yolo").unwrap()],
            None,
        )
        .unwrap(),
    ])
    .unwrap()
}

#[test]
fn absent_means_never_set_and_an_empty_list_stays_empty() {
    let unset: NodeSettings = serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap();
    assert_eq!(unset.terminal_profiles, None);
    let emptied = NodeSettings {
        terminal_profiles: Some(Profiles::new(Vec::new()).unwrap()),
        ..unset.clone()
    };
    let text = serde_json::to_string(&emptied).unwrap();
    assert!(text.contains(r#""terminal_profiles":[]"#), "{text}");
    let back: NodeSettings = serde_json::from_str(&text).unwrap();
    assert_eq!(back, emptied);
    let change: SetNodeSettings =
        serde_json::from_str(r#"{"terminal_profiles":[{"id":"a","name":"A","command":"pi"}]}"#)
            .unwrap();
    let kept = change.terminal_profiles.unwrap();
    assert_eq!(kept.entries().len(), 1);
    assert!(kept.entries()[0].args().is_empty());
}

#[test]
fn a_set_list_is_hidden_below_a_synthetic_assigned_boundary() {
    let set = NodeSettings {
        terminal_profiles: Some(mine()),
        ..serde_json::from_str(r#"{"park_after":{"kind":"never"}}"#).unwrap()
    };
    let required = crate::PROTOCOL_VERSION;
    let older = crate::ProtocolVersion::new(required.get().checked_sub(1).unwrap());
    for (protocol, assignment) in [(older, Some(required)), (required, None)] {
        let projected = set.for_profiles_protocol(protocol, assignment).unwrap();
        assert_eq!(projected.terminal_profiles, None);
        assert!(
            !serde_json::to_string(&projected)
                .unwrap()
                .contains("terminal_profiles")
        );
    }
    assert_eq!(set.for_profiles_protocol(required, Some(required)), None);
    // While the number is pending, no peer is shown them.
    if crate::terminal_profiles::TERMINAL_PROFILES_PROTOCOL.is_none() {
        assert_eq!(set.for_protocol(required).unwrap().terminal_profiles, None);
    }
}
