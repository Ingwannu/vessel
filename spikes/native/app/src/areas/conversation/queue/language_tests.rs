use super::*;
use crate::shell::language_tests;

#[gpui::test]
fn a_language_change_keeps_the_queue_edit_caret_selection_and_focus(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            let prompt = QueuedPrompt {
                id: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
                revision: 7,
                content: vec![ContentBlock::Text {
                    text: "original queued text".into(),
                }],
                queued_at: chrono::DateTime::UNIX_EPOCH,
                by: vessel_proto::Creator::default(),
            };
            shell.queue_edit(&prompt, window, cx);
        },
        |shell| &shell.areas.conversation.queue_edit.as_ref().unwrap().field,
        |shell| {
            let edit = shell.areas.conversation.queue_edit.as_ref().unwrap();
            assert_eq!(edit.revision, 7);
            assert_eq!(
                edit.entry.to_string(),
                "00000000-0000-4000-8000-000000000002"
            );
        },
    );
}
