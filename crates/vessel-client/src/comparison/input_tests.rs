//! Input bounds and metadata-only snapshots cannot become executable owners.
use super::*;
use vessel_proto::{AttachmentId, AttachmentKind, HarnessKind, ModelId};

fn selection(model: &str) -> Selection {
    Selection::new(
        HarnessKind::ClaudeCode.into(),
        None,
        Some(ModelId::new(model)),
        Vec::new(),
    )
    .unwrap()
}
fn base() -> WorktreeBase {
    WorktreeBase::Ref {
        name: "main".parse().unwrap(),
    }
}
fn plan(targets: Vec<Selection>) -> Result<ComparisonPlan, InputError> {
    ComparisonPlan::new(ProjectId::random(), base(), targets)
}
#[test]
fn target_count_duplicates_and_post_construction_edits_are_checked() {
    assert!(matches!(plan(Vec::new()), Err(InputError::Count)));
    assert!(plan(vec![selection("one")]).is_ok());
    assert!(matches!(
        plan(vec![selection("one"), selection("one")]),
        Err(InputError::Duplicate)
    ));
    assert!(matches!(
        plan((0..9).map(|at| selection(&at.to_string())).collect()),
        Err(InputError::Count)
    ));
    let mut edited = selection("one");
    edited.model = Some(ModelId::new("한".repeat(6000)));
    assert!(matches!(plan(vec![edited]), Err(InputError::Encoding(_))));
    assert!(matches!(
        Selection::new(
            HarnessKind::ClaudeCode.into(),
            None,
            Some(ModelId::new("\n".repeat(9000))),
            Vec::new()
        ),
        Err(InputError::Encoding(_))
    ));
}
#[test]
fn choices_have_a_fixed_count_and_no_repeated_ids() {
    let choice = |at: usize| OptionChoice {
        id: vessel_proto::OptionId::new(format!("choice-{at}")),
        value: vessel_proto::ChoiceValue::Toggle { on: false },
    };
    assert!(matches!(
        Selection::new(
            HarnessKind::ClaudeCode.into(),
            None,
            None,
            (0..33).map(choice).collect()
        ),
        Err(InputError::Choices)
    ));
    assert!(matches!(
        Selection::new(
            HarnessKind::ClaudeCode.into(),
            None,
            None,
            vec![choice(0), choice(0)]
        ),
        Err(InputError::Choices)
    ));
}
#[test]
fn manifests_refuse_fields_that_could_change_session_placement_or_history() {
    assert!(serde_json::from_str::<Selection>(r#"{"harness":"claude-code"}"#).is_ok());
    assert!(serde_json::from_str::<Selection>(r#"{"harness":"claude-code","options":[{"id":"effort","kind":"select","value":"high"}]}"#).is_ok());
    for text in [
        r#"{"harness":{"added":"goose","cwd":"bad"}}"#,
        r#"{"harness":"claude-code","options":[{"id":"model","kind":"select","value":"a","resume":"bad"}]}"#,
    ] {
        assert!(serde_json::from_str::<Selection>(text).is_err());
    }
    for extra in ["cwd", "resume", "side_question_of", "place", "mode"] {
        let text = format!(r#"{{"harness":"claude-code","{extra}":null}}"#);
        assert!(serde_json::from_str::<Selection>(&text).is_err(), "{extra}");
    }
}
#[test]
fn finished_source_refs_are_bounded_and_input_debug_does_not_echo_text() {
    let session = SessionId::random();
    let file = AttachmentRef {
        id: AttachmentId::random(),
        name: "common.txt".parse().unwrap(),
        kind: AttachmentKind::File,
        mime: "text/plain".parse().unwrap(),
        size: 3,
    };
    assert!(SourceAttachments::new(session, Vec::new()).is_err());
    assert!(SourceAttachments::new(session, vec![file.clone(), file.clone()]).is_err());
    let mut too_large = file.clone();
    too_large.size = AttachmentLimits::DEFAULT.per_session.saturating_add(1);
    assert!(SourceAttachments::new(session, vec![too_large]).is_err());
    assert!(ComparisonInput::new(None, None).is_err());
    let input = ComparisonInput::new(
        Some("private draft".parse().unwrap()),
        Some(SourceAttachments::new(session, vec![file]).unwrap()),
    )
    .unwrap();
    assert!(!format!("{input:?}").contains("private draft"));
}
