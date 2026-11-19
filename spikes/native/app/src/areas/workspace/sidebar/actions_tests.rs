//! A project row (TRACKER 662) drawn by the real sidebar: its badge says
//! the most urgent of its sessions as a status; Remove asks first, the row
//! spins until the node answers, and the answer is said as Paseo's row
//! says it (nothing when done, a toast when refused).
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window, point,
    size,
};
use vessel_app_kit::ui::px;
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_app_view::{Attention, Card, Fleet, Lang, Machine, Reach};
use vessel_proto::workspace::{GitSupport, GitVersion, ProjectInfo, ProjectList, ProjectState};
use vessel_proto::{
    HarnessKind, ProjectId, SessionId, SessionInfo, SessionMode, SessionSpec, SessionStatus,
};

use super::{Origin, Target};
use crate::shell::{Shell, language_tests};

struct Probe {
    shell: Entity<Shell>,
    /// A stand-in for whatever had the keys before the menu (the composer).
    opener: FocusHandle,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let sidebar = self.shell.update(cx, |shell, cx| {
            crate::areas::sidebar::render(shell, &Tokens::of(Scheme::Dark), false, window, cx)
        });
        gpui::div()
            .size_full()
            .child(gpui::div().track_focus(&self.opener).size(px(10.0)))
            .child(sidebar)
    }
}

fn project_id(index: u16) -> ProjectId {
    format!("{index:08x}-0000-4000-8000-00000000000a")
        .parse()
        .unwrap()
}

fn projects() -> ProjectsView {
    let at = "2026-11-19T08:00:00Z".parse().unwrap();
    ProjectsView::Ready(ProjectList {
        git: GitSupport::Found {
            version: GitVersion::MINIMUM,
        },
        more: false,
        projects: ["shop", "docs", "site"]
            .into_iter()
            .zip(0_u16..)
            .map(|(name, index)| ProjectInfo {
                id: project_id(index),
                name: name.parse().unwrap(),
                root: format!("/fixture/{name}").into(),
                git: true,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: at,
                revision: 1,
            })
            .collect(),
    })
}

/// A live session in `project`, doing `attention`.
fn session(index: u16, project: ProjectId, attention: Attention) -> Card {
    let at = "2026-11-19T08:00:00Z".parse().unwrap();
    let id: SessionId = format!("{index:08x}-0000-4000-8000-00000000000b")
        .parse()
        .unwrap();
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let mut info = SessionInfo::new(id, spec, at);
    info.status = SessionStatus::Running;
    let mut card = Card::from_session("fixture", &info);
    card.attention = attention;
    card.detail.runs_in = Some(vessel_app_view::board::RunsIn::Checkout(project));
    card
}

fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, FocusHandle, &mut VisualTestContext) {
    language_tests::init(cx);
    let saved = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), {
            let saved = Rc::clone(&saved);
            move |window, cx| {
                let shell = cx.new(|cx| {
                    let mut shell = Shell::without_io(window, cx);
                    shell.fleet = Fleet {
                        machines: vec![Machine {
                            connection_epoch: None,
                            node: None,
                            name: "fixture".to_owned(),
                            reach: Reach::Online,
                            sessions: vec![
                                session(0, project_id(0), Attention::Working),
                                session(1, project_id(0), Attention::AtRest),
                                session(2, project_id(1), Attention::NeedsYou),
                            ],
                            agents: Vec::new(),
                            protocol: Some(vessel_proto::PROTOCOL_VERSION),
                            version: None,
                        }],
                        read_at: None,
                    };
                    shell.t = Lang::En.text();
                    let _previous = shell
                        .areas
                        .workspace
                        .projects
                        .insert("fixture".to_owned(), projects());
                    shell
                });
                let opener = cx.focus_handle();
                *saved.borrow_mut() = Some((shell.clone(), opener.clone()));
                let probe = cx.new(|_| Probe { shell, opener });
                vessel_app_kit::root(probe, window, cx)
            }
        })
        .unwrap()
    });
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.run_until_parked();
    cx.simulate_resize(size(px(1280.0), px(800.0)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let (shell, opener) = saved.borrow().as_ref().unwrap().clone();
    (shell, opener, cx)
}

fn shop() -> Target {
    Target {
        machine: "fixture".to_owned(),
        project: project_id(0),
        name: "shop".to_owned(),
    }
}

#[gpui::test]
fn each_projects_badge_says_its_most_urgent_session(cx: &mut TestAppContext) {
    let (shell, _opener, cx) = setup(cx);
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    shell.update(cx, |_, cx| cx.notify());
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    // Every `aria` object of GPUI's real snapshot, as text (as the sort
    // trigger's test reads it): the statuses' names.
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    let mut said: Vec<String> = json
        .split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .filter(|aria| aria.contains(r#""role": "Status""#))
        .filter_map(|aria| {
            let (_, label) = aria.split_once(r#""label": ""#)?;
            label.split_once('"').map(|(label, _)| label.to_owned())
        })
        .collect();
    said.sort();
    // shop: working over done; docs: its session needs input; site: none,
    // so no badge.
    assert_eq!(said, ["Needs input", "Working"]);
    // Each project's kebab is a button named "Project actions" that takes a
    // Click (its click opens the menu, R2).
    let kebabs: Vec<&str> = json
        .split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria))
        .filter(|aria| aria.contains(r#""label": "Project actions""#))
        .collect();
    assert_eq!(kebabs.len(), 3);
    assert!(kebabs.iter().all(|aria| aria.contains(r#""Button""#)));
    assert!(kebabs.iter().all(|aria| aria.contains(r#""Click""#)));
}

#[gpui::test]
fn a_click_alone_opens_the_project_menu_once(cx: &mut TestAppContext) {
    let (shell, _opener, cx) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        // No press went down first: assistive technology's Click.
        shell.project_menu_activate(shop(), point(px(200.0), px(200.0)), window, cx);
        assert!(shell.areas.workspace.sidebar.menu_is_for(&shop()));
        // The click that ends a pointer's press leaves it open.
        shell.project_menu_activate(shop(), point(px(200.0), px(200.0)), window, cx);
        assert!(shell.areas.workspace.sidebar.menu_is_for(&shop()));
    });
}

#[gpui::test]
fn the_remove_question_gives_the_keys_back_when_it_closes(cx: &mut TestAppContext) {
    let (shell, opener, cx) = setup(cx);
    cx.update(|window, cx| opener.focus(window, cx));
    let ask = |cx: &mut VisualTestContext| {
        shell.update_in(cx, |shell, window, cx| {
            shell.project_menu_open(shop(), point(px(200.0), px(200.0)), window, cx);
            // The menu gives the keys back to its opener before it runs a
            // choice (menu/draw.rs).
            opener.focus(window, cx);
            shell.project_menu_chose(1, window, cx);
        });
        cx.run_until_parked();
    };
    ask(cx);
    cx.update(|window, cx| {
        assert!(!opener.is_focused(window));
        assert!(window.focused(cx).is_some());
    });
    // Cancel: the keys go back.
    shell.update_in(cx, |shell, window, cx| shell.project_remove_dismiss(window, cx));
    cx.run_until_parked();
    cx.update(|window, _| assert!(opener.is_focused(window)));
    // A second question remembers the opener afresh; Remove gives them back.
    ask(cx);
    cx.update(|window, _| assert!(!opener.is_focused(window)));
    shell.update_in(cx, |shell, window, cx| shell.project_remove_confirmed(window, cx));
    cx.run_until_parked();
    cx.update(|window, _| assert!(opener.is_focused(window)));
}

#[gpui::test]
fn remove_asks_first_then_the_row_spins_until_the_node_answers(cx: &mut TestAppContext) {
    let (shell, _opener, cx) = setup(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.project_menu_open(shop(), point(px(200.0), px(200.0)), window, cx);
        // The menu's second row: Remove project.
        shell.project_menu_chose(1, window, cx);
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        let rows = &shell.areas.workspace.sidebar;
        assert_eq!(rows.confirm, Some(shop()));
        assert!(!rows.is_removing("fixture", project_id(0)));
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.project_remove_confirmed(window, cx);
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        let rows = &shell.areas.workspace.sidebar;
        assert_eq!(rows.confirm, None);
        assert!(rows.is_removing("fixture", project_id(0)));
    });
    // Done: the row goes, quietly.
    shell.update(cx, |shell, cx| {
        shell.project_removal_answered("fixture", project_id(0), Ok(()), cx);
    });
    shell.read_with(cx, |shell, _| {
        assert!(
            !shell
                .areas
                .workspace
                .sidebar
                .is_removing("fixture", project_id(0))
        );
        let list = &shell.areas.workspace.projects["fixture"];
        assert!(list.project(&project_id(0)).is_none());
        assert_eq!(shell.notice, None);
    });
}

#[gpui::test]
fn a_refused_removal_from_the_row_is_a_toast_and_the_row_stays(cx: &mut TestAppContext) {
    let (shell, _opener, cx) = setup(cx);
    shell.update(cx, |shell, cx| {
        shell.remove_project_from("fixture", project_id(1), Origin::Sidebar);
        shell.project_removal_answered(
            "fixture",
            project_id(1),
            Err(Problem::Gone("the connection dropped".to_owned())),
            cx,
        );
    });
    shell.read_with(cx, |shell, _| {
        assert!(
            !shell
                .areas
                .workspace
                .sidebar
                .is_removing("fixture", project_id(1))
        );
        let list = &shell.areas.workspace.projects["fixture"];
        assert!(list.project(&project_id(1)).is_some());
        let toast = shell.areas.toasts.shown().unwrap();
        assert_eq!(toast.tone, ToastTone::Error);
        assert!(
            toast.text.starts_with("docs was not removed: "),
            "{}",
            toast.text
        );
    });
}
