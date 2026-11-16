use super::*;

#[test]
fn exact_root_names_and_whole_goal_arguments_follow_the_reference() {
    let cases = [
        ("/goal", GoalIntent::Usage),
        (" \u{feff}/goal  PaUsE \n", GoalIntent::Pause),
        ("/goal\tRESUME", GoalIntent::Resume),
        ("/goal\r\nclear", GoalIntent::Clear),
        (
            "/goal resume now",
            GoalIntent::Set {
                objective: "resume now".parse().unwrap(),
            },
        ),
        (
            "/goal  한글\nobjective  ",
            GoalIntent::Set {
                objective: "한글\nobjective".parse().unwrap(),
            },
        ),
    ];
    for (text, expected) in cases {
        let intent = NativeCommandIntent::parse(NativeCommandKind::Goal, text).unwrap();
        assert_eq!(intent, NativeCommandIntent::Goal(expected));
        assert!(intent.matches_text(text));
        assert!(!intent.matches_text("/compact"));
    }
    for text in ["/compact", " \u{feff}/compact ignored arguments\n"] {
        assert_eq!(
            NativeCommandIntent::parse(NativeCommandKind::Compact, text).unwrap(),
            NativeCommandIntent::Compact
        );
    }
    for text in [
        "/Goal",
        "text /goal",
        "//goal",
        "/goal/path",
        "/goal\u{0085}pause",
        "/",
    ] {
        assert_eq!(
            NativeCommandIntent::parse(NativeCommandKind::Goal, text),
            Err(NativeCommandParseError::Mismatch)
        );
    }
}

#[test]
fn objective_bounds_are_bytes_and_invalid_content_is_never_truncated_or_printed() {
    let maximum = "x".repeat(MAX_GOAL_OBJECTIVE_BYTES);
    assert_eq!(maximum.parse::<GoalObjective>().unwrap().as_str(), maximum);
    assert_eq!(
        format!("{maximum}x").parse::<GoalObjective>(),
        Err(GoalObjectiveError::TooLong)
    );
    let unicode = "한".repeat(MAX_GOAL_OBJECTIVE_BYTES.div_euclid("한".len()));
    assert!(unicode.parse::<GoalObjective>().is_ok());
    assert_eq!(
        format!("{unicode}한").parse::<GoalObjective>(),
        Err(GoalObjectiveError::TooLong)
    );
    for value in ["", "  \n\t", "\u{feff}"] {
        assert_eq!(
            value.parse::<GoalObjective>(),
            Err(GoalObjectiveError::Empty)
        );
    }
    for value in [
        "sensitive\0value",
        "sensitive\u{0085}value",
        "sensitive\u{007f}value",
    ] {
        assert_eq!(
            value.parse::<GoalObjective>(),
            Err(GoalObjectiveError::Control)
        );
        let error = NativeCommandIntent::parse(NativeCommandKind::Goal, &format!("/goal {value}"))
            .unwrap_err();
        assert!(!error.to_string().contains("sensitive"));
    }
    let intent = NativeCommandIntent::Goal(GoalIntent::Set {
        objective: "sensitive objective".parse().unwrap(),
    });
    assert!(!format!("{intent:?}").contains("sensitive"));
}

#[test]
fn statuses_derive_their_kind_without_an_invalid_pair_or_freeform_output() {
    for status in [
        NativeCommandStatus::CompactStarted,
        NativeCommandStatus::GoalSet,
        NativeCommandStatus::GoalPaused,
        NativeCommandStatus::GoalResumed,
        NativeCommandStatus::GoalCleared,
        NativeCommandStatus::GoalUsage,
    ] {
        assert_eq!(
            status.kind(),
            if status == NativeCommandStatus::CompactStarted {
                NativeCommandKind::Compact
            } else {
                NativeCommandKind::Goal
            }
        );
        let encoded = serde_json::to_string(&status).unwrap();
        assert_eq!(
            serde_json::from_str::<NativeCommandStatus>(&encoded).unwrap(),
            status
        );
    }
    assert!(serde_json::from_str::<NativeCommandStatus>("\"future_status\"").is_err());
}

#[test]
fn wire_decode_validates_objectives_and_rejects_extra_operation_fields() {
    let intent = NativeCommandIntent::Goal(GoalIntent::Set {
        objective: "한글 objective".parse().unwrap(),
    });
    let encoded = serde_json::to_string(&intent).unwrap();
    assert_eq!(
        encoded,
        r#"{"kind":"goal","action":{"kind":"set","objective":"한글 objective"}}"#
    );
    assert_eq!(
        serde_json::from_str::<NativeCommandIntent>(&encoded).unwrap(),
        intent
    );
    for text in [
        r#"{"kind":"goal","action":{"kind":"set","objective":""}}"#,
        r#"{"kind":"goal","action":{"kind":"set","objective":"bad\u0000value"}}"#,
        r#"{"kind":"goal","action":{"kind":"pause","objective":"extra"}}"#,
        r#"{"kind":"goal","action":{"kind":"usage","objective":"extra"}}"#,
        r#"{"kind":"goal","action":{"kind":"resume","objective":"extra"}}"#,
        r#"{"kind":"goal","action":{"kind":"clear","objective":"extra"}}"#,
        r#"{"kind":"compact","action":{"objective":"extra"}}"#,
        r#"{"kind":"goal","action":{"kind":"set","objective":"ok","extra":true}}"#,
        r#"{"kind":"compact","extra":true}"#,
        r#"{"kind":"unknown"}"#,
    ] {
        assert!(
            serde_json::from_str::<NativeCommandIntent>(text).is_err(),
            "{text}"
        );
    }
}

#[test]
fn native_original_text_is_bounded_before_a_receipt_or_echo_is_admitted() {
    let text = format!("{} /compact", " ".repeat(MAX_NATIVE_COMMAND_ORIGINAL_BYTES));
    assert!(!NativeCommandIntent::Compact.matches_content(&[crate::ContentBlock::Text { text }]));
}
