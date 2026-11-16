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

struct NavigationProbe {
    shell: gpui::Entity<Shell>,
}

impl gpui::Render for NavigationProbe {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        use gpui::ParentElement as _;
        self.shell.update(cx, |shell, cx| {
            let field = shell.areas.conversation.find.as_ref().map(|bar| {
                bar.field.element(
                    &vessel_app_kit::Tokens::of(vessel_app_kit::Scheme::Dark),
                    false,
                    cx,
                )
            });
            gpui::div().children(field)
        })
    }
}

#[gpui::test]
fn navigation_retirement_keeps_the_actual_field_selection_and_hidden_input(
    cx: &mut gpui::TestAppContext,
) {
    use super::{Finder, Match};
    use crate::areas::conversation::ConversationArea;
    use gpui::AppContext as _;
    use vessel_proto::EventSeq;

    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            shell.open_find(window, cx);
            shell
        });
        NavigationProbe { shell }
    });
    cx.run_until_parked();
    cx.simulate_input("typed ABCDE");
    cx.simulate_keystrokes("left left shift-left");
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let area = &mut shell.areas.conversation;
            let bar = area.find.as_mut().unwrap();
            let field = bar.field.id();
            let selection = bar.field.selection(cx);
            let generation = std::rc::Rc::clone(&bar.generation);
            bar.finder = Some(Finder::new(&bar.query));
            bar.matches = vec![Match { entry: 0, nth: 0 }];
            bar.refreshing = true;
            bar.again = true;
            bar.keep = bar.matches.first().copied();
            bar.at = Some(EventSeq::new(42));
            area.reveal = Some(EventSeq::new(99));
            assert!(area.navigation_held());
            let mut hidden = std::mem::replace(area, ConversationArea::new(cx));
            shell.open_find(window, cx);
            let active = shell.areas.conversation.find.as_ref().unwrap().field.id();
            hidden.navigation_retired();
            let bar = hidden.find.as_ref().unwrap();
            assert_eq!(bar.field.id(), field);
            assert_eq!(bar.field.text(cx), "typed ABCDE");
            assert_eq!(bar.field.selection(cx), selection);
            assert_eq!(bar.query, "typed ABCDE");
            assert!(!std::rc::Rc::ptr_eq(&bar.generation, &generation));
            assert!(bar.finder.is_none() && bar.matches.is_empty());
            assert!(!bar.refreshing && !bar.again);
            assert!(bar.keep.is_none() && bar.at.is_none());
            assert_eq!(hidden.reveal, Some(EventSeq::new(99)));
            let bar = shell.areas.conversation.find.as_ref().unwrap();
            assert_eq!(bar.field.id(), active);
            assert!(bar.field.focus_handle(cx).is_focused(window));
            assert!(bar.query.is_empty());
            shell.areas.conversation = hidden;
            shell
                .areas
                .conversation
                .find
                .as_ref()
                .unwrap()
                .field
                .focus(window, cx);
        });
    });
    cx.run_until_parked();
    cx.simulate_input("X");
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, cx| {
            let bar = shell.areas.conversation.find.as_ref().unwrap();
            assert_eq!(bar.field.text(cx), "typed ABXDE");
            assert_eq!(bar.query, "typed ABXDE");
        });
    });
}

#[gpui::test]
fn navigation_pending_reveals_hold_and_remain_with_their_existing_owner(
    cx: &mut gpui::TestAppContext,
) {
    use crate::areas::conversation::{ConversationArea, origin::Origin};
    use vessel_proto::EventSeq;

    cx.update(|cx| {
        let mut root = ConversationArea::new(cx);
        assert!(!root.navigation_held());
        let mut leaf = ConversationArea::new(cx);
        leaf.reveal = Some(EventSeq::new(99));
        let mut middle = ConversationArea::new(cx);
        middle.origin = Origin::Shown(Box::new(leaf));
        root.origin = Origin::Shown(Box::new(middle));
        assert!(root.navigation_held());
        root.navigation_retired();
        assert!(
            root.navigation_held(),
            "retirement cannot silently claim Ready"
        );
        let leaf = root.origin.area_mut().unwrap().origin.area_mut().unwrap();
        assert_eq!(leaf.reveal, Some(EventSeq::new(99)));
        leaf.reveal = None; // Only the corresponding owner's settlement releases it.
        assert!(!root.navigation_held());
    });
}

#[gpui::test]
fn changing_the_active_session_pane_can_deactivate_find_without_changing_route(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::AppContext as _;
    use vessel_proto::EventSeq;

    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| FocusProbe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.route = crate::routes::Route::Session;
            shell.find_at("remembered", EventSeq::new(10), window, cx);
            shell.areas.conversation.reveal = Some(EventSeq::new(10));
            let old =
                std::rc::Rc::clone(&shell.areas.conversation.find.as_ref().unwrap().generation);
            shell.conversation_deactivated(window, cx);
            assert_eq!(shell.route, crate::routes::Route::Session);
            assert!(shell.areas.conversation.find.is_none());
            assert_eq!(shell.areas.conversation.find_query, "remembered");
            assert_eq!(shell.areas.conversation.reveal, None);
            assert!(!shell.areas.conversation.navigation_held());
            assert!(shell.find_step(&old, cx));
            assert!(window.focused(cx).is_none());
        });
    });
}

#[gpui::test]
fn reopening_retains_the_query_and_edits_cancel_only_its_pending_destination(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::AppContext as _;
    use vessel_proto::EventSeq;
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| FocusProbe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.find_at("retained needle", EventSeq::new(10), window, cx);
            shell.areas.conversation.reveal = Some(EventSeq::new(10));
            shell.close_find(window, cx);
            assert_eq!(shell.areas.conversation.reveal, None);
            assert_eq!(shell.areas.conversation.find_query, "retained needle");
            shell.open_find(window, cx);
            assert_eq!(
                shell.areas.conversation.find.as_ref().unwrap().query,
                "retained needle"
            );
            assert!(
                shell.areas.conversation.find_query.is_empty(),
                "the query has one owner"
            );
            shell.find_target(EventSeq::new(10), cx);
            shell.areas.conversation.reveal = Some(EventSeq::new(10));
            shell.find_changed("replacement".to_owned(), cx);
            assert_eq!(shell.areas.conversation.reveal, None);
            shell.find_target(EventSeq::new(10), cx);
            shell.areas.conversation.reveal = Some(EventSeq::new(20));
            shell.find_changed("another".to_owned(), cx);
            assert_eq!(
                shell.areas.conversation.reveal,
                Some(EventSeq::new(20)),
                "an unrelated inbox destination stays"
            );
            shell.find_target(EventSeq::new(10), cx);
            shell.close_find(window, cx);
            assert_eq!(shell.areas.conversation.reveal, Some(EventSeq::new(20)));
            shell.find_at("failed needle", EventSeq::new(10), window, cx);
            shell.areas.conversation.reveal = Some(EventSeq::new(10));
            shell.find_failed(vessel_app_view::timeline::FindFailure::Reveal);
            assert_eq!(shell.areas.conversation.reveal, None);
        });
    });
    cx.run_until_parked();
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
        });
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
        });
    });
}

#[gpui::test]
fn a_cancelled_search_cannot_run_against_a_reopened_bar(cx: &mut gpui::TestAppContext) {
    use gpui::AppContext as _;
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| FocusProbe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.open_find(window, cx);
            shell.find_changed("old query".to_owned(), cx);
            assert!(shell.areas.conversation.navigation_held());
            let old =
                std::rc::Rc::clone(&shell.areas.conversation.find.as_ref().unwrap().generation);
            shell.close_find(window, cx);
            assert!(!shell.areas.conversation.navigation_held());
            shell.open_find(window, cx);
            shell.find_changed("new query".to_owned(), cx);
            assert!(
                shell.find_step(&old, cx),
                "the old task must stop immediately"
            );
            let new = shell.areas.conversation.find.as_ref().unwrap();
            assert_eq!(new.query, "new query");
            assert!(
                new.finder.is_some(),
                "the old task did not finish the new search"
            );
            crate::routes::arrive(crate::routes::Route::Board, shell, window, cx);
            assert!(
                shell.areas.conversation.find.is_none(),
                "deactivation closes find"
            );
            assert!(
                window.focused(cx).is_none(),
                "deactivation never focuses a hidden composer"
            );
        });
    });
    cx.run_until_parked();
}

#[gpui::test]
fn history_and_connection_failures_disable_navigation_until_retry_or_edit(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::AppContext as _;
    use vessel_app_view::timeline::{FindFailure, FindPhase};
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| FocusProbe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.open_find(window, cx);
            shell.find_changed("needle".to_owned(), cx);
            shell.find_failed(FindFailure::HistoryChanged);
            shell.find_grew(cx);
            assert_eq!(
                shell.areas.conversation.find.as_ref().unwrap().phase,
                FindPhase::Failed(FindFailure::HistoryChanged)
            );
            assert!(
                shell
                    .areas
                    .conversation
                    .find
                    .as_ref()
                    .unwrap()
                    .finder
                    .is_none()
            );
            shell.find_retry(window, cx);
            assert_eq!(
                shell.areas.conversation.find.as_ref().unwrap().phase,
                FindPhase::Searching
            );
            shell.find_failed(FindFailure::Connection);
            shell.find_changed("replacement".to_owned(), cx);
            assert_eq!(
                shell.areas.conversation.find.as_ref().unwrap().phase,
                FindPhase::Searching
            );
            shell.find_changed(String::new(), cx);
            assert_eq!(
                shell.areas.conversation.find.as_ref().unwrap().phase,
                FindPhase::Idle
            );
        });
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_missing_destination_loads_then_fails_and_a_closed_target_cannot_fail_a_new_bar(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::AppContext as _;
    use vessel_app_view::timeline::{FindFailure, FindPhase};
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        cx.observe(&shell, |_probe, _shell, cx| cx.notify())
            .detach();
        FocusProbe { shell }
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.find_at("needle", vessel_proto::EventSeq::new(10), window, cx);
        });
    });
    cx.run_until_parked();
    cx.executor().advance_clock(super::DEBOUNCE);
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        assert_eq!(
            probe
                .shell
                .read(cx)
                .areas
                .conversation
                .find
                .as_ref()
                .unwrap()
                .phase,
            FindPhase::Loading
        );
    });
    let half = super::REVEAL_DEADLINE.checked_div(2).unwrap();
    cx.executor().advance_clock(half);
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.find_at("needle", vessel_proto::EventSeq::new(10), window, cx);
        });
    });
    cx.run_until_parked();
    cx.executor().advance_clock(half);
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        assert_eq!(
            probe
                .shell
                .read(cx)
                .areas
                .conversation
                .find
                .as_ref()
                .unwrap()
                .phase,
            FindPhase::Loading,
            "reselecting the same destination owns a fresh deadline"
        );
    });
    cx.executor().advance_clock(half);
    cx.run_until_parked();
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            assert_eq!(
                shell.areas.conversation.find.as_ref().unwrap().phase,
                FindPhase::Failed(FindFailure::Reveal)
            );
            shell.find_at("needle", vessel_proto::EventSeq::new(20), window, cx);
            shell.close_find(window, cx);
            shell.open_find(window, cx);
            shell.find_changed("new".to_owned(), cx);
        });
    });
    cx.run_until_parked();
    cx.executor().advance_clock(super::REVEAL_DEADLINE);
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        assert_eq!(
            probe
                .shell
                .read(cx)
                .areas
                .conversation
                .find
                .as_ref()
                .unwrap()
                .phase,
            FindPhase::Ready
        );
    });
}

#[gpui::test]
fn retiring_hidden_navigation_keeps_its_input_and_cannot_mutate_the_active_resident(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::AppContext as _;
    use std::rc::Rc;
    use vessel_app_view::timeline::{FindFailure, FindPhase};
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| FocusProbe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
    });
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            shell.open_find(window, cx);
            shell
                .areas
                .conversation
                .find
                .as_ref()
                .unwrap()
                .field
                .set_text("retained input", window, cx);
            shell.find_changed("retained input".to_owned(), cx);
            let bar = shell.areas.conversation.find.as_mut().unwrap();
            bar.matches = vec![super::Match { entry: 0, nth: 0 }];
            bar.keep = bar.matches.first().copied();
            bar.refreshing = true;
            bar.again = true;
            let generation = Rc::clone(&bar.generation);
            let target = Rc::clone(&bar.target);
            let focus = bar.field.focus_handle(cx);
            assert!(shell.areas.conversation.navigation_held());
            let mut hidden = std::mem::replace(
                &mut shell.areas.conversation,
                crate::areas::conversation::ConversationArea::new(cx),
            );
            hidden.navigation_retired();
            let bar = hidden.find.as_ref().unwrap();
            assert_eq!(bar.query, "retained input");
            assert_eq!(bar.field.text(cx), "retained input");
            assert_eq!(bar.field.focus_handle(cx), focus);
            assert_eq!(bar.phase, FindPhase::Failed(FindFailure::Connection));
            assert!(!Rc::ptr_eq(&bar.generation, &generation));
            assert!(!Rc::ptr_eq(&bar.target, &target));
            assert!(bar.finder.is_none());
            assert!(bar.matches.is_empty() && bar.keep.is_none());
            assert!(!bar.refreshing && !bar.again);
            assert!(bar.at.is_none());
            assert!(
                hidden.navigation_held(),
                "retained input holds its resource"
            );
            assert!(shell.areas.conversation.find.is_none());
            assert!(!shell.areas.conversation.navigation_held());
        });
    });
    cx.run_until_parked();
}

#[gpui::test]
fn retiring_empty_find_cancels_its_slice_without_showing_a_failure(cx: &mut gpui::TestAppContext) {
    use super::{DEBOUNCE, FindPhase};
    use gpui::AppContext as _;
    use std::rc::Rc;
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            shell.open_find(window, cx);
            shell.start_search(false, cx);
            let before = Rc::clone(&shell.areas.conversation.find.as_ref().unwrap().generation);
            shell.areas.conversation.navigation_retired();
            let bar = shell.areas.conversation.find.as_ref().unwrap();
            assert!(bar.query.is_empty());
            assert_eq!(bar.phase, FindPhase::Idle);
            assert!(bar.finder.is_none());
            assert!(!Rc::ptr_eq(&before, &bar.generation));
            assert!(shell.find_step(&before, cx), "old work is already retired");
            shell
        });
        NavigationProbe { shell }
    });
    cx.run_until_parked();
    cx.executor().advance_clock(DEBOUNCE);
    cx.run_until_parked();
    probe.update(cx, |probe, cx| {
        probe.shell.read_with(cx, |shell, _| {
            let bar = shell.areas.conversation.find.as_ref().unwrap();
            assert_eq!(bar.phase, FindPhase::Idle);
            assert!(bar.query.is_empty() && bar.finder.is_none());
        });
    });
}
