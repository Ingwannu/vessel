use crate::shell::{Shell, language_tests};

#[gpui::test]
fn a_language_change_keeps_the_query_caret_selection_and_focus(cx: &mut gpui::TestAppContext) {
    language_tests::init(cx);
    language_tests::preserves(
        cx,
        Shell::open_find,
        |shell| &shell.areas.conversation.find.as_ref().unwrap().field,
        |shell| {
            let bar = shell.areas.conversation.find.as_ref().unwrap();
            assert_eq!(bar.query, "typed ABCDE");
        },
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[gpui::test]
fn a_language_change_rewords_update_facts_without_changing_them_or_input(
    cx: &mut gpui::TestAppContext,
) {
    use crate::update::{UpdateState, Updated};

    language_tests::init(cx);
    language_tests::preserves(
        cx,
        |shell, window, cx| {
            shell.open_find(window, cx);
            shell.updates.state = UpdateState::Ready {
                label: "0.0.0.0.8".parse().unwrap(),
                notes: None,
            };
            shell.updates.announcement = Some(Updated {
                label: "0.0.0.0.7".to_owned(),
                notes: Some("The running build's notes".to_owned()),
            });
            shell.updates.refresh(shell.words);
            assert_eq!(
                shell.updates.status(shell.words).as_ref(),
                "Update ready: v\u{2068}0.0.0.0.8\u{2069}"
            );
        },
        |shell| &shell.areas.conversation.find.as_ref().unwrap().field,
        |shell| {
            assert_eq!(
                shell.updates.status(shell.words).as_ref(),
                "업데이트 준비됨: v\u{2068}0.0.0.0.8\u{2069}"
            );
            assert_eq!(
                shell.updates.banner(shell.words).as_ref(),
                "업데이트를 마쳤습니다: \u{2068}0.0.0.0.7\u{2069}"
            );
            let UpdateState::Ready { label, notes } = &shell.updates.state else {
                panic!("the ready state changed while translating");
            };
            assert_eq!(label.to_string(), "0.0.0.0.8");
            assert!(notes.is_none());
            assert_eq!(
                shell
                    .updates
                    .announcement
                    .as_ref()
                    .unwrap()
                    .notes
                    .as_deref(),
                Some("The running build's notes")
            );
        },
    );
}
