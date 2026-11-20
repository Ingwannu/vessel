//! The sidebar grouped by project (TRACKER 682) drawn by the real sidebar:
//! a project is a group named by it, its header a button that says whether
//! it is open and folds it (kept), Show more / Show less past its limit,
//! New workspace under a git project with no session; the shortcut numbers
//! count the tree as drawn.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext,
    Window, size,
};
use vessel_app_kit::ui::px;
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::board::RunsIn;
use vessel_app_view::prefs::Grouping;
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_app_view::{Card, Fleet, Lang, Machine, Reach};
use vessel_proto::workspace::{GitSupport, GitVersion, ProjectInfo, ProjectList, ProjectState};
use vessel_proto::{
    HarnessKind, ProjectId, SessionId, SessionInfo, SessionMode, SessionSpec, SessionStatus,
};

use super::Numbers;
use crate::shell::{Shell, language_tests};

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            super::render(shell, &Tokens::of(Scheme::Dark), false, window, cx)
        })
    }
}

pub(super) fn project_id(index: u16) -> ProjectId {
    format!("{index:08x}-0000-4000-8000-00000000000a")
        .parse()
        .unwrap()
}

/// shop and docs are git repositories, notes is not.
fn projects() -> ProjectsView {
    let at = "2026-11-19T08:00:00Z".parse().unwrap();
    ProjectsView::Ready(ProjectList {
        git: GitSupport::Found {
            version: GitVersion::MINIMUM,
        },
        more: false,
        projects: [("shop", true), ("docs", true), ("notes", false)]
            .into_iter()
            .zip(0_u16..)
            .map(|((name, git), index)| ProjectInfo {
                id: project_id(index),
                name: name.parse().unwrap(),
                root: format!("/fixture/{name}").into(),
                git,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: at,
                revision: 1,
            })
            .collect(),
    })
}

/// A live session `index` (newer as `index` grows), in `project` if any.
fn session(index: u16, project: Option<ProjectId>) -> Card {
    let at = chrono::DateTime::from_timestamp(i64::from(index), 0).unwrap();
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
    card.detail.runs_in = project.map(RunsIn::Checkout);
    card
}

/// shop holds `in_shop` sessions, notes one, and one is in no project.
pub(super) fn setup(
    cx: &mut TestAppContext,
    in_shop: u16,
) -> (Entity<Shell>, &mut VisualTestContext) {
    language_tests::init(cx);
    let saved = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), {
            let saved = Rc::clone(&saved);
            move |window, cx| {
                let shell = cx.new(|cx| {
                    let mut shell = Shell::without_io(window, cx);
                    let mut sessions: Vec<Card> = (0..in_shop)
                        .map(|index| session(index, Some(project_id(0))))
                        .collect();
                    sessions.push(session(100, Some(project_id(2))));
                    sessions.push(session(101, None));
                    shell.fleet = Fleet {
                        machines: vec![Machine {
                            connection_epoch: None,
                            node: None,
                            name: "fixture".to_owned(),
                            reach: Reach::Online,
                            sessions,
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
                *saved.borrow_mut() = Some(shell.clone());
                let probe = cx.new(|_| Probe { shell });
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
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    let shell = saved.borrow().as_ref().unwrap().clone();
    (shell, cx)
}

/// Every `aria` object of GPUI's real snapshot, as text (as the sort
/// trigger's test reads it), after a fresh frame.
pub(super) fn snapshot(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Vec<String> {
    shell.update(cx, |_, cx| cx.notify());
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    json.split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .collect()
}

pub(super) fn named<'a>(arias: &'a [String], role: &str, name: &str) -> Vec<&'a String> {
    let role = format!(r#""{role}""#);
    let label = format!(r#""label": "{name}""#);
    arias
        .iter()
        .filter(|aria| aria.contains(&role) && aria.contains(&label))
        .collect()
}

#[gpui::test]
fn project_is_the_default_and_each_project_is_a_group_its_header_saying_open(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = setup(cx, 2);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.prefs.settings.sidebar.group, Grouping::Project);
    });
    let arias = snapshot(&shell, cx);
    for name in ["shop", "docs", "notes"] {
        assert_eq!(named(&arias, "Group", name).len(), 1, "{name}'s group");
        let header = named(&arias, "Button", name);
        assert_eq!(header.len(), 1, "{name}'s header");
        assert!(header[0].contains(r#""expanded": true"#), "{name} open");
    }
    // docs is a git project with no session: Paseo's New workspace row
    // under it (and the header's plus, named the same); notes is not git,
    // so only its sessions.
    assert_eq!(
        named(&arias, "Button", "Create a new workspace for docs").len(),
        2
    );
    assert_eq!(
        named(&arias, "Button", "Create a new workspace for notes").len(),
        0
    );
}

#[gpui::test]
fn a_header_folds_its_project_kept_and_the_numbers_skip_it(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    let numbered = |shell: &Shell| -> Vec<Option<usize>> {
        let held = Numbers::of_shell(shell, true);
        let machine = &shell.fleet.machines[0];
        machine.sessions.iter().map(|card| held.of(card)).collect()
    };
    shell.read_with(cx, |shell, _| {
        // By name: notes, then shop (newest first), then the one in no
        // project (docs has none).
        assert_eq!(numbered(shell), [Some(3), Some(2), Some(1), Some(4)]);
    });
    shell.update(cx, |shell, cx| {
        shell.fold_project("fixture", project_id(0), cx);
    });
    shell.read_with(cx, |shell, _| {
        assert!(
            shell
                .prefs
                .settings
                .folded_projects
                .contains("fixture", project_id(0))
        );
        assert_eq!(numbered(shell), [None, None, Some(1), Some(2)]);
    });
    let arias = snapshot(&shell, cx);
    let header = named(&arias, "Button", "shop");
    assert!(header[0].contains(r#""expanded": false"#));
    // Pressed again, it opens.
    shell.update(cx, |shell, cx| {
        shell.fold_project("fixture", project_id(0), cx);
    });
    shell.read_with(cx, |shell, _| {
        assert!(shell.prefs.settings.folded_projects.is_empty());
    });
}

#[gpui::test]
fn past_seven_sessions_show_more_then_show_less(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 9);
    let arias = snapshot(&shell, cx);
    assert_eq!(named(&arias, "Button", "Show more").len(), 1);
    assert!(named(&arias, "Button", "Show less").is_empty());
    shell.update(cx, |shell, _| {
        let _new = shell
            .areas
            .sidebar
            .shown_all
            .insert(("fixture".to_owned(), project_id(0)));
    });
    let arias = snapshot(&shell, cx);
    assert!(named(&arias, "Button", "Show more").is_empty());
    assert_eq!(named(&arias, "Button", "Show less").len(), 1);
}

/// The fixture machine's list, read.
fn ready(shell: &mut Shell) -> &mut ProjectList {
    match shell.areas.workspace.projects.get_mut("fixture") {
        Some(ProjectsView::Ready(list)) => list,
        _ => panic!("the fixture's list is read"),
    }
}

#[gpui::test]
fn a_header_not_ready_is_a_button_named_by_it_that_does_not_fold(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    shell.update(cx, |shell, cx| {
        ready(shell).projects[2].state = ProjectState::Missing;
        cx.notify();
    });
    let arias = snapshot(&shell, cx);
    let header = named(&arias, "Button", "notes");
    assert_eq!(header.len(), 1);
    assert!(!header[0].contains("expanded"), "{}", header[0]);
}

#[gpui::test]
fn a_whole_list_drops_the_folds_of_projects_it_no_longer_holds(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    shell.update(cx, |shell, cx| {
        shell.fold_project("fixture", project_id(0), cx);
        shell.fold_project("fixture", project_id(1), cx);
        // shop went on the node: the next list holds docs and notes.
        let _shop = ready(shell).projects.remove(0);
        shell.folds_listed("fixture");
        let folds = &shell.prefs.settings.folded_projects;
        assert!(!folds.contains("fixture", project_id(0)));
        assert!(folds.contains("fixture", project_id(1)));
        // A list cut short drops nothing: docs may be past the cut.
        let cut = ready(shell);
        cut.more = true;
        let _docs = cut.projects.remove(0);
        shell.folds_listed("fixture");
        assert!(
            shell
                .prefs
                .settings
                .folded_projects
                .contains("fixture", project_id(1))
        );
    });
}
