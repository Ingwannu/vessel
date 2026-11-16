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
            let generation = bar.generation;
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
            assert_ne!(bar.generation, generation);
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
