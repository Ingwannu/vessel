//! The actual header draws shared chrome and keeps its original action target.
use super::*;
use crate::areas::session::Open;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, IntoElement, px};
use vessel_app_view::layout::{AgentRole, OpenIntent, Placement, Target};
use vessel_proto::SessionId;

struct Probe(Entity<Shell>);

fn retained_body(
    _shell: &Shell,
    _tokens: &Tokens,
    _window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) -> Div {
    div()
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            let tokens = vessel_app_kit::tokens(cx);
            super::super::draw::frame(shell, retained_body, &tokens, window, cx)
        })
    }
}

fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    Probe(cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
        shell.route = crate::routes::Route::Session;
        let session = SessionId::random();
        shell.areas.session.open = Some(Open {
            machine: "isolated".to_owned(),
            session,
            title: "retained".to_owned(),
            agent: "fake".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        with_layout(&shell, |layout| {
            let _tab = layout.open(
                Target::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )?;
            Ok(())
        })
        .unwrap();
        shell
    }))
}

fn place(shell: &Shell, control: Control) -> gpui::Bounds<gpui::Pixels> {
    shell.areas.layout.tip_places.borrow()[&(PaneId::MAIN, control)]
        .get()
        .unwrap()
}

#[gpui::test]
fn pointer_header_uses_shared_hitboxes_and_switches_actual_maximize_restore(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(
            place(shell, Control::New).size,
            gpui::size(px(20.0), px(20.0))
        );
        assert!(
            !shell
                .areas
                .layout
                .tip_places
                .borrow()
                .contains_key(&(PaneId::MAIN, Control::Focus))
        );
        // Paseo's "…" while panes can split, in the same 20-point chrome.
        assert_eq!(
            place(shell, Control::More).size,
            gpui::size(px(20.0), px(20.0))
        );
        // Order: + | maximise | … (maximise only with two or more panes).
        assert!(place(shell, Control::New).right() <= place(shell, Control::More).left());
    });
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            with_layout(shell, |layout| {
                let _pane = layout.split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)?;
                layout.focus(PaneId::MAIN)
            })
            .unwrap();
            cx.notify();
        });
    });
    cx.run_until_parked();
    let focus = probe.read_with(cx, |probe, cx| place(probe.0.read(cx), Control::Focus));
    assert_eq!(focus.size, gpui::size(px(20.0), px(20.0)));
    cx.simulate_click(focus.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(
            shell.areas.layout.maximized,
            Some((super::super::key(shell).unwrap(), PaneId::MAIN))
        );
        assert_eq!(Control::Focus.glyph(true), Glyph::PanelMinimize);
        assert_eq!(
            Control::Focus.label(shell, true),
            shell.words.layout().restore()
        );
    });
    let restore = probe.read_with(cx, |probe, cx| place(probe.0.read(cx), Control::Focus));
    cx.simulate_click(restore.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert!(shell.areas.layout.maximized.is_none());
        assert_eq!(Control::Focus.glyph(false), Glyph::PanelMaximize);
        assert_eq!(
            Control::Focus.label(shell, false),
            shell.words.layout().maximize()
        );
        assert_eq!(
            with_layout(shell, |layout| Ok(layout.root().panes().len())).unwrap(),
            2
        );
    });
}

#[gpui::test]
fn compact_header_keeps_fixed_glyph_frame_policy_and_actual_new_tab_dispatch(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(390.0), px(844.0)));
    probe.update(cx, |probe, cx| {
        probe.0.update(cx, |shell, cx| {
            // Discard measurements from the initial wide fixture paint before
            // asserting which controls the subsequent compact paint installed.
            shell.areas.layout.tip_places.borrow_mut().clear();
            cx.notify();
        });
    });
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert!(
            !shell
                .areas
                .layout
                .tip_places
                .borrow()
                .contains_key(&(PaneId::MAIN, Control::Right))
        );
        // Compact tooltips intentionally install no measurement/listener. The
        // actual row contains only New, so its painted bounds are the hitbox.
        assert!(
            shell.areas.layout.tip_places.borrow()[&(PaneId::MAIN, Control::New)]
                .get()
                .is_none()
        );
    });
    let new = cx.debug_bounds("layout-main-pane-controls").unwrap();
    assert_eq!(new.size, gpui::size(px(32.0), px(32.0)));
    cx.simulate_click(new.center(), Modifiers::none());
    cx.run_until_parked();
    // Paseo's "+" is a menu: it opens the new-tab menu and adds no tab.
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert!(shell.areas.layout.header_menu.is_open());
        assert_eq!(
            shell.areas.layout.header_menu_for.as_ref().map(|menu| menu.kind),
            Some(MenuKind::NewTab)
        );
        with_layout(shell, |layout| {
            assert_eq!(layout.tabs().count(), 1);
            Ok(())
        })
        .unwrap();
    });
}

#[gpui::test]
fn retained_header_callback_refuses_changed_original_selection_without_split(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        shell.route = crate::routes::Route::Session;
        shell.areas.session.open = Some(Open {
            machine: "isolated".to_owned(),
            session: SessionId::random(),
            title: "retained".to_owned(),
            agent: "fake".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        let context = shell.layout_action_context().unwrap();
        let old = PaneAction {
            op: PaneOp::Split(Position::Right),
            pane: PaneId::MAIN,
            context,
        };
        let _tab = shell
            .layout_open_target(
                old.context.workspace(),
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                window,
                cx,
            )
            .unwrap();
        let before = shell.areas.layout.book.borrow().clone();
        old.run(shell, window, cx);
        assert_eq!(*shell.areas.layout.book.borrow(), before);
        assert!(shell.areas.layout.maximized.is_none());
    });
}

/// Paseo's new-tab menu: the launch catalog in the pane's order, only what
/// starts here enabled; Agent opens the new-agent page.
#[gpui::test]
fn the_new_tab_menu_lists_the_catalog_and_agent_opens_the_new_agent_page(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    let new = probe.read_with(cx, |probe, cx| place(probe.0.read(cx), Control::New));
    cx.simulate_click(new.center(), Modifiers::none());
    cx.run_until_parked();
    // Below "+", its left edge at the button's, 4 down.
    let first = cx.debug_bounds("menu-row-0").unwrap();
    assert!(first.top() > new.bottom());
    assert!(first.left() >= new.left());
    let launches = cx.update(|window, cx| {
        let shell = probe.read(cx).0.read(cx);
        rows(shell, MenuKind::NewTab, window)
            .iter()
            .map(|(op, item)| (*op, item.disabled))
            .collect::<Vec<_>>()
    });
    {
        assert_eq!(
            launches,
            [
                (Some(PaneOp::Launch(Launch::Agent)), false),
                (None, true),
                (None, true),
                (None, true),
                (None, true),
                (None, true),
                (None, true),
            ]
        );
    }
    cx.simulate_click(first.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(shell.route, Route::Draft);
        assert!(!shell.areas.layout.header_menu.is_open());
    });
}

/// Paseo's "…": a 220-wide menu below the button, its end at the button's,
/// with both splits and their chords; a row splits the pane it was opened on.
#[gpui::test]
fn the_more_menu_splits_the_pane_it_was_opened_on(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(1280.0), px(900.0)));
    cx.run_until_parked();
    let more = probe.read_with(cx, |probe, cx| place(probe.0.read(cx), Control::More));
    cx.simulate_click(more.center(), Modifiers::none());
    cx.run_until_parked();
    cx.update(|window, cx| {
        let shell = probe.read(cx).0.read(cx);
        assert_eq!(
            shell.areas.layout.header_menu_for.as_ref().map(|menu| menu.kind),
            Some(MenuKind::Split)
        );
        let rows = rows(shell, MenuKind::Split, window);
        assert_eq!(
            rows.iter().map(|(op, _)| *op).collect::<Vec<_>>(),
            [
                Some(PaneOp::Split(Position::Right)),
                Some(PaneOp::Split(Position::Bottom)),
            ]
        );
        assert_eq!(rows[0].1.leading, Some(Glyph::Columns2));
        assert_eq!(rows[1].1.leading, Some(Glyph::Rows2));
        assert_eq!(rows[0].1.shortcut, caps(Action::SplitRight, shell));
    });
    let right = cx.debug_bounds("menu-row-0").unwrap();
    assert!(right.top() > more.bottom());
    cx.simulate_click(right.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        assert_eq!(
            with_layout(shell, |layout| Ok(layout.root().panes().len())).unwrap(),
            2
        );
    });
}
