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

struct FocusProbe {
    shell: gpui::Entity<Shell>,
}
impl gpui::Render for FocusProbe {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div};
        self.shell.update(cx, |shell, cx| {
            let tokens = vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark);
            div()
                .size_full()
                .relative()
                .child(shell.areas.session.composer.element(&tokens, false, cx))
                .children(super::bar::bar(shell, &tokens, window, cx))
                .into_any_element()
        })
    }
}

#[gpui::test]
fn escape_returns_the_keys_to_the_composer_that_opened_find(cx: &mut gpui::TestAppContext) {
    use gpui::AppContext as _;
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let shell = Shell::without_io(window, cx);
            shell.areas.session.composer.focus(window, cx);
            shell
        });
        cx.observe(&shell, |_probe, _shell, cx| cx.notify())
            .detach();
        FocusProbe { shell }
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.open_find(window, cx);
            assert!(
                shell
                    .areas
                    .conversation
                    .find
                    .as_ref()
                    .unwrap()
                    .field
                    .focus_handle(cx)
                    .is_focused(window)
            );
        })
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.read_with(cx, |shell, cx| {
            assert!(shell.areas.conversation.find.is_none());
            assert!(
                shell
                    .areas
                    .session
                    .composer
                    .focus_handle(cx)
                    .is_focused(window)
            );
        })
    });
}
