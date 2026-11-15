//! Measures the actual sidebar renderers, not a second height model.
use crate::shell::{Shell, language_tests};
use gpui::{
    AppContext as _, Bounds, Context, Entity, FocusHandle, IntoElement, Pixels, Render,
    ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, Window, point, size,
};
use std::{cell::RefCell, rc::Rc};
use vessel_app_kit::ui::px;
use vessel_app_kit::{Scheme, SectionIndex, Tokens};
use vessel_app_view::prefs::Grouping;
use vessel_app_view::workspace::projects::ProjectsView;
use vessel_app_view::{Card, Fleet, Lang, Machine, Reach};
use vessel_proto::workspace::{GitSupport, GitVersion, ProjectInfo, ProjectList, ProjectState};
use vessel_proto::{HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec, SessionStatus};

struct Probe {
    shell: Entity<Shell>,
    frames: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let frames = Rc::clone(&self.frames);
        self.shell.update(cx, |shell, cx| {
            super::super::render(shell, &Tokens::of(Scheme::Dark), false, window, cx)
                .on_children_prepainted(move |bounds, _, _| *frames.borrow_mut() = bounds)
        })
    }
}

fn machine(name: &str) -> Machine {
    let at = "2026-11-11T08:00:00Z".parse().unwrap();
    let sessions = (0_u16..24)
        .map(|index| {
            let id: SessionId = format!("{index:08x}-0000-4000-8000-000000000000")
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
            let mut session = SessionInfo::new(id, spec, at);
            session.status = SessionStatus::Running;
            Card::from_session(name, &session)
        })
        .collect();
    Machine {
        connection_epoch: None,
        node: None,
        name: name.to_owned(),
        reach: Reach::Online,
        sessions,
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
    }
}

fn projects() -> ProjectsView {
    let at = "2026-11-11T08:00:00Z".parse().unwrap();
    ProjectsView::Ready(ProjectList {
        git: GitSupport::Found {
            version: GitVersion::MINIMUM,
        },
        more: false,
        projects: (0_u16..11)
            .map(|index| ProjectInfo {
                id: format!("{index:08x}-0000-4000-8000-000000000000")
                    .parse()
                    .unwrap(),
                name: format!("Project {index}").parse().unwrap(),
                root: format!("/fixture/project-{index}").into(),
                git: true,
                default_branch: None,
                state: ProjectState::Ready,
                created_at: at,
                revision: 1,
            })
            .collect(),
    })
}

fn setup(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    group: Grouping,
) -> (Entity<Probe>, &mut VisualTestContext) {
    let saved = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), {
            let saved = Rc::clone(&saved);
            move |window, cx| {
                let shell = cx.new(|cx| {
                    let mut shell = Shell::without_io(window, cx);
                    shell.fleet = Fleet {
                        machines: vec![machine("fixture")],
                        read_at: None,
                    };
                    shell.prefs.settings.sidebar.group = group;
                    shell.t = Lang::En.text();
                    let _previous = shell
                        .areas
                        .workspace
                        .projects
                        .insert("fixture".to_owned(), projects());
                    shell
                });
                let probe = cx.new(|_| Probe {
                    shell,
                    frames: Rc::default(),
                });
                *saved.borrow_mut() = Some(probe.clone());
                vessel_app_kit::root(probe, window, cx)
            }
        })
        .unwrap()
    });
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.run_until_parked();
    cx.simulate_resize(size(px(width), px(height)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let probe = saved.borrow().as_ref().unwrap().clone();
    (probe, cx)
}

fn fixed(probe: &Entity<Probe>, cx: &VisualTestContext, height: f32) {
    probe.read_with(cx, |probe, cx| {
        let frames = probe.frames.borrow();
        assert_eq!(frames.len(), 4, "navigation, one body, add-machine, footer");
        assert!(frames[0].size.height > px(0.0));
        assert!(frames[1].size.height > px(0.0));
        assert!(frames[2].bottom() <= px(height));
        assert!(frames[3].bottom() <= px(height));
        assert!(frames[1].bottom() <= frames[2].top());
        let state = &probe.shell.read(cx).areas.sidebar.layout;
        // Header rows own their outer inset; the body wrapper retains the
        // original scrolling width, with no vertical padding or second scroll.
        let body = frames[1];
        let expected = Bounds::new(
            point(px(f32::from(body.left()).mul_add(1.0, 8.0)), body.top()),
            size(
                px(f32::from(body.size.width).mul_add(1.0, -16.0)),
                body.size.height,
            ),
        );
        assert_eq!(state.scroll.bounds(), expected);
        for index in 0..3 {
            assert!(
                state
                    .scroll
                    .section_bounds(SectionIndex::new(index))
                    .is_some()
            );
        }
    });
}

fn target(
    probe: &Entity<Probe>,
    cx: &VisualTestContext,
    pick: fn(&super::Layout) -> &FocusHandle,
) -> FocusHandle {
    probe.read_with(cx, |probe, cx| {
        pick(&probe.shell.read(cx).areas.sidebar.layout).clone()
    })
}

#[gpui::test]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "finite measured fixture pixels"
)]
fn actual_sidebar_keeps_foot_and_reaches_the_end_after_unfold(cx: &mut TestAppContext) {
    language_tests::init(cx);
    for (width, height) in [(800.0, 600.0), (1280.0, 800.0)] {
        for group in [Grouping::Machine, Grouping::Status] {
            let (probe, cx) = setup(cx, width, height, group);
            fixed(&probe, cx, height);
            let folded = probe.read_with(cx, |probe, cx| {
                probe
                    .shell
                    .read(cx)
                    .areas
                    .sidebar
                    .layout
                    .scroll
                    .section_bounds(SectionIndex::new(0))
                    .unwrap()
                    .size
                    .height
            });
            probe.update(cx, |probe, cx| {
                probe.shell.update(cx, |shell, cx| {
                    shell.areas.workspace.projects_unfolded = true;
                    cx.notify();
                });
            });
            cx.run_until_parked();
            fixed(&probe, cx, height);
            probe.read_with(cx, |probe, cx| {
                assert!(
                    probe
                        .shell
                        .read(cx)
                        .areas
                        .sidebar
                        .layout
                        .scroll
                        .section_bounds(SectionIndex::new(0))
                        .unwrap()
                        .size
                        .height
                        > folded
                );
            });
            let body = probe.read_with(cx, |probe, cx| {
                probe.shell.read(cx).areas.sidebar.layout.scroll.bounds()
            });
            cx.simulate_event(ScrollEvent {
                position: body.center(),
                delta: ScrollDelta::Pixels(point(px(0.0), px(-2000.0))),
                ..ScrollEvent::default()
            });
            cx.run_until_parked();
            fixed(&probe, cx, height);
            probe.read_with(cx, |probe, cx| {
                assert!(probe.shell.read(cx).areas.sidebar.layout.scroll.offset().y < px(0.0));
            });
            let nav = target(&probe, cx, |layout| &layout.navigation);
            cx.update(|window, cx| nav.focus(window, cx));
            cx.simulate_keystrokes("tab");
            cx.run_until_parked();
            let end = target(&probe, cx, |layout| &layout.end);
            cx.update(|window, _| assert!(end.is_focused(window)));
            probe.read_with(cx, |probe, cx| {
                let scroll = &probe.shell.read(cx).areas.sidebar.layout.scroll;
                let groups = scroll.section_bounds(SectionIndex::new(2)).unwrap();
                assert_eq!(
                    groups.bottom() + scroll.offset().y,
                    scroll.bounds().bottom()
                );
            });
            cx.simulate_keystrokes("tab");
            cx.run_until_parked();
            let footer = target(&probe, cx, |layout| &layout.footer);
            cx.update(|window, _| assert!(footer.is_focused(window)));
            fixed(&probe, cx, height);
        }
    }
}
