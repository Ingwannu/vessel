//! Exercise the shipped trigger through GPUI's accessibility snapshot and hit path.
use gpui::{
    AppContext as _, Entity, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext,
    size,
};
use vessel_app_kit::{
    Scheme, Tokens,
    ui::{Context, Window, px},
};
use vessel_app_view::{
    Fleet, Machine, Reach,
    connection::{ConnectionEpochs, MachineKey as BookKey},
    schedules::{load::MachineKey, page::Snapshot},
};
use vessel_proto::{
    Creator, NodeId, Scope, SessionMode, SessionSpec,
    automation::{
        AfterRun, Cadence, CatchUp, ScheduleDefinition, ScheduleInfo, ScheduleState, ScheduleTarget,
    },
};

use crate::{routes::Route, shell::Shell};

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            super::super::render(shell, &Tokens::of(Scheme::Dark), window, cx)
        })
    }
}

fn listed(shell: &mut Shell, heartbeat: bool) {
    let at = chrono::DateTime::UNIX_EPOCH;
    let context = ConnectionEpochs::default()
        .admit(BookKey::from_book("laptop"), NodeId::random())
        .unwrap();
    let mut machine = Machine::unreachable("laptop".into(), "fixture");
    machine.reach = Reach::Online;
    machine.node = Some(context.node());
    machine.protocol = Some(vessel_proto::ProtocolVersion::new(47));
    machine.connection_epoch = Some(context.epoch());
    shell.fleet = Fleet {
        machines: vec![machine],
        read_at: Some(at),
    };
    shell.route = Route::Schedules;
    let area = &mut shell.areas.schedules;
    area.data
        .page
        .machines(&[MachineKey::of("laptop").unwrap()]);
    assert!(area.data.page.admissions(&shell.fleet));
    let request = area.data.page.due().pop().unwrap();
    let schedule = ScheduleInfo {
        id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        definition: ScheduleDefinition {
            name: None,
            cadence: Cadence::Cron {
                expr: "0 * * * *".parse().unwrap(),
                zone: "UTC".parse().unwrap(),
            },
            target: if heartbeat {
                ScheduleTarget::Session {
                    session: vessel_proto::SessionId::random(),
                }
            } else {
                ScheduleTarget::NewSession {
                    spec: Box::new(SessionSpec::new(
                        "claude-code".parse::<vessel_proto::HarnessRef>().unwrap(),
                        SessionMode::Structured {
                            model: None,
                            resume: None,
                            options: Vec::new(),
                        },
                    )),
                    after: AfterRun::Archive,
                    isolation: None,
                }
            },
            prompt: "Check the build report".parse().unwrap(),
            catch_up: CatchUp::Once,
            max_runs: None,
            ends_at: None,
        },
        state: ScheduleState::Active { next_due: at },
        runs: 0,
        last_due: None,
        last_run: None,
        defined_by: Creator::Owner,
        created_at: at,
        updated_at: at,
        revision: 0,
    };
    area.data.page.listed(
        &request,
        Ok(Snapshot {
            schedules: vec![schedule],
            scopes: Ok(vec![Scope::Read, Scope::Operate]),
            read_at: at,
            worktrees: Ok(Vec::new()),
            projects: Ok(Vec::new()),
        }),
    );
    assert!(area.rebuild(&shell.fleet, shell.words));
}

fn trigger(cx: &mut VisualTestContext, label: &str) -> String {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let tree = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    let name = format!(r#""label": "{label}""#);
    let mut matches = tree
        .split(r#""aria":"#)
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria))
        .filter(|aria| aria.contains(&name));
    let node = matches.next().expect("named schedule menu trigger");
    assert!(matches.next().is_none(), "one listed row");
    node.to_owned()
}

#[gpui::test]
fn the_row_menu_is_a_clickable_button_with_its_expansion_state(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            listed(&mut shell, false);
            shell
        }),
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    let shell = probe.update(cx, |probe, _| probe.shell.clone());
    let label = shell.read_with(cx, |shell, _| shell.words.schedules().more());
    let closed = trigger(cx, label);
    assert!(closed.contains(r#""role": "Button""#), "{closed}");
    assert!(closed.contains(r#""Click""#), "{closed}");
    assert!(closed.contains(r#""expanded": false"#), "{closed}");

    let at = shell.read_with(cx, |shell, _| {
        let control = shell.areas.schedules.ui.anchors.values().next().unwrap();
        control.anchor.get().unwrap().center()
    });
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        let area = &shell.areas.schedules;
        assert!(area.ui.menu.is_open());
        assert_eq!(
            area.ui.selected.as_ref().unwrap().key(),
            &area.data.rows[0].key
        );
    });
    let open = trigger(cx, label);
    assert!(open.contains(r#""expanded": true"#), "{open}");
}

#[gpui::test]
fn a_heartbeat_row_opens_read_only_detail_and_keeps_delete(cx: &mut TestAppContext) {
    use crate::shell::language_tests::{init, named_button};
    use vessel_app_view::schedules::Action;

    init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            listed(&mut shell, true);
            shell
        }),
    });
    cx.simulate_resize(size(px(1280.0), px(900.0)));
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| {
            let row = &shell.areas.schedules.data.rows[0];
            let intent = row.intent.clone().unwrap();
            assert!(super::super::editor::openable(shell, &intent));
            assert_eq!(
                row.menu
                    .iter()
                    .map(|entry| entry.action)
                    .collect::<Vec<_>>(),
                [Action::Delete]
            );
            shell.schedule_edit_open(&intent, window, cx);
        });
    });
    cx.update(|window, _| window.refresh());
    assert!(named_button(cx, "Close"));
    assert!(!named_button(cx, "Save changes"));
    assert!(!named_button(cx, "Cadence (Every hour)"));
}
