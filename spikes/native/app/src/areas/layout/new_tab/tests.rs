//! The New tab panel (TRACKER 336): Paseo's groups in its order, the rail,
//! the keys, and what a press does.
use super::*;
use crate::areas::session::Open;
use crate::routes::Route;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{Entity, IntoElement, Pixels};
use vessel_app_view::layout::{AgentRole, LayoutAction, LayoutMode, OpenIntent, Placement, Target};
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

/// An open session with its agent tab and a New tab tab, the New tab shown.
fn fixture(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    Probe(cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
        shell.route = Route::Session;
        let session = SessionId::random();
        shell.areas.session.open = Some(Open {
            machine: "laptop".to_owned(),
            session,
            title: "Fix the cart".to_owned(),
            agent: "Claude".to_owned(),
            structured: true,
            live: true,
            archived_at: None,
            binding: None,
        });
        super::super::with_layout(&shell, |layout| {
            let _agent = layout.open(
                Target::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )?;
            let new = layout.open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )?;
            layout.select(new)
        })
        .unwrap();
        shell
    }))
}

fn at(cx: &mut TestAppContext, width: f32) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(gpui::size(px(width), px(844.0)));
    cx.run_until_parked();
    (probe, cx)
}

/// A per-row selector: the test context takes only static ones.
fn row(id: &str) -> &'static str {
    format!("new-tab-row-{id}").leak()
}

fn top(cx: &mut VisualTestContext, id: &str) -> Pixels {
    cx.debug_bounds(row(id)).unwrap().top()
}

/// The New tab tab's owner, as the panel draws it.
fn owner(probe: &Entity<Probe>, cx: &mut VisualTestContext) -> Owner {
    probe.read_with(cx, |probe, cx| {
        let shell = probe.0.read(cx);
        let workspace = super::super::key(shell).unwrap();
        let tab = super::super::with_layout(shell, |layout| {
            Ok(layout
                .root()
                .panes()
                .into_iter()
                .find_map(vessel_app_view::layout::Pane::focused))
        })
        .unwrap()
        .unwrap();
        Owner::Tab(workspace, tab)
    })
}

const BUILT_INS: [&str; 7] = [
    "agent",
    "terminal",
    "changes",
    "diff",
    "files",
    "browser",
    "pullrequest",
];
const PROFILES: [&str; 4] = [
    "profile-claude",
    "profile-codex",
    "profile-opencode",
    "profile-pi",
];

#[gpui::test]
fn the_built_ins_come_in_paseos_order_then_the_profiles_under_their_label(cx: &mut TestAppContext) {
    let (_probe, cx) = at(cx, 1280.0);
    let order: Vec<&str> = BUILT_INS.iter().chain(&PROFILES).copied().collect();
    let tops: Vec<Pixels> = order.iter().map(|id| top(cx, id)).collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    let label = cx.debug_bounds("new-tab-group-label").unwrap();
    assert!(label.top() > top(cx, "pullrequest"));
    assert!(label.bottom() <= top(cx, "profile-claude"));
    let edit = cx.debug_bounds("new-tab-edit-profiles").unwrap();
    assert_eq!(edit.size, gpui::size(px(EDIT_HIT), px(EDIT_HIT)));
    // The old line and button are gone.
    assert!(cx.debug_bounds("layout-launch-agent").is_none());
}

#[gpui::test]
fn the_rail_is_at_most_380_wide_and_centred(cx: &mut TestAppContext) {
    let (_probe, cx) = at(cx, 1280.0);
    let rail = cx.debug_bounds("new-tab-rail").unwrap();
    let panel = cx.debug_bounds("new-tab-panel").unwrap();
    assert_eq!(rail.size.width, px(RAIL_MAX));
    let off =
        f32::from(rail.center().x).mul_add(1.0, f32::from(panel.center().x).mul_add(-1.0, 0.0));
    assert!(off.abs() < 1.0, "{rail:?} in {panel:?}");
    let agent = cx.debug_bounds(row("agent")).unwrap();
    assert!(agent.size.height >= px(ROW_MIN));
}

#[gpui::test]
fn on_a_phone_diff_is_left_out(cx: &mut TestAppContext) {
    let (_probe, cx) = at(cx, 390.0);
    assert!(cx.debug_bounds(row("diff")).is_none());
    assert!(cx.debug_bounds(row("changes")).is_some());
    let rail = cx.debug_bounds("new-tab-rail").unwrap();
    // 24 in from each side of the pane.
    assert!(rail.size.width < px(RAIL_MAX));
}

#[gpui::test]
fn the_new_tab_shortcut_gives_the_new_panel_the_keys(cx: &mut TestAppContext) {
    let (probe, cx) = at(cx, 1280.0);
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            assert!(shell.run_layout_action(LayoutAction::NewTab, window, cx));
        });
    });
    cx.run_until_parked();
    let owner = owner(&probe, cx);
    cx.update(|window, cx| {
        probe.read_with(cx, |probe, cx| {
            let keys = &probe.0.read(cx).areas.layout.new_tab;
            assert_eq!(keys.owner.as_ref(), Some(&owner));
            assert_eq!(keys.row, 0, "Agent, the first ready row");
            assert!(keys.focus.is_focused(window));
        });
    });
}

#[gpui::test]
fn the_arrows_stay_on_ready_rows_and_enter_opens_the_new_agent_page(cx: &mut TestAppContext) {
    let (probe, cx) = at(cx, 1280.0);
    let owner = owner(&probe, cx);
    probe.update_in(cx, |probe, window, cx| {
        probe.0.update(cx, |shell, cx| {
            shell.new_tab_take_keys(owner, None, window, cx);
        });
    });
    cx.run_until_parked();
    for key in ["down", "down", "up"] {
        cx.simulate_keystrokes(key);
        cx.run_until_parked();
        // Agent is the only ready row: the arrows wrap onto it.
        probe.read_with(cx, |probe, cx| {
            assert_eq!(probe.0.read(cx).areas.layout.new_tab.row, 0, "{key}");
        });
    }
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        assert_eq!(probe.0.read(cx).route, Route::Draft);
    });
}

#[gpui::test]
fn a_press_on_a_disabled_row_starts_nothing_and_lights_the_first_ready_row(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = at(cx, 1280.0);
    let terminal = cx.debug_bounds(row("terminal")).unwrap();
    cx.simulate_click(terminal.center(), Modifiers::none());
    cx.run_until_parked();
    let owner = owner(&probe, cx);
    cx.update(|window, cx| {
        probe.read_with(cx, |probe, cx| {
            let shell = probe.0.read(cx);
            assert_eq!(shell.route, Route::Session);
            let keys = &shell.areas.layout.new_tab;
            assert_eq!(keys.owner.as_ref(), Some(&owner));
            assert_eq!(keys.row, 0);
            assert!(keys.focus.is_focused(window));
        });
    });
}

#[gpui::test]
fn a_press_on_agent_opens_the_new_agent_page(cx: &mut TestAppContext) {
    let (probe, cx) = at(cx, 1280.0);
    let agent = cx.debug_bounds(row("agent")).unwrap();
    cx.simulate_click(agent.center(), Modifiers::none());
    cx.run_until_parked();
    probe.read_with(cx, |probe, cx| {
        assert_eq!(probe.0.read(cx).route, Route::Draft);
    });
}

#[gpui::test]
fn every_label_starts_on_one_rail(cx: &mut TestAppContext) {
    let (_probe, cx) = at(cx, 1280.0);
    // Paseo's one glyph box: a brand mark's clear space moves no label.
    let ends: Vec<Pixels> = BUILT_INS
        .iter()
        .chain(&PROFILES)
        .map(|id| {
            cx.debug_bounds(format!("new-tab-icon-{id}").leak())
                .unwrap()
        })
        .map(|icon| icon.right())
        .collect();
    assert!(ends.windows(2).all(|pair| pair[0] == pair[1]), "{ends:?}");
}
