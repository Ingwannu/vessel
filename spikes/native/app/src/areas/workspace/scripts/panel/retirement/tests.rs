//! Actual committed layout mutations invoke the real Scripts owner, before polling.
use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use std::cell::RefCell;
use std::rc::Rc;
use vessel_app_link::{
    Command,
    workspace::{Ask, ScriptAsk},
};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{AgentRole, OpenIntent, Placement, RetargetRequest, Target as Panel};
use vessel_app_view::workspace::scripts::{Action, Snapshot, Target};
use vessel_proto::{
    NodeId, SessionId,
    ports::{PortList, PortScan},
    scripts::{ScriptEntry, ScriptKind, ScriptList},
    workspace::ProjectCommands,
};

fn script_commands(receiver: &mut tokio::sync::mpsc::Receiver<Command>) -> Vec<ScriptAsk> {
    std::iter::from_fn(|| receiver.try_recv().ok())
        .filter_map(|command| {
            if let Command::Workspace {
                ask: Ask::Scripts(ask),
                ..
            } = command
            {
                Some(ask)
            } else {
                None
            }
        })
        .collect()
}
fn opened(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) -> Target {
    start(shell, cx);
    let session = SessionId::random();
    let workspace = LayoutKey::Session {
        machine: vessel_app_view::layout::HostAlias::parse("fixture").unwrap(),
        session,
    };
    let tab = shell
        .layout_open_target(
            &workspace,
            Panel::Scripts,
            OpenIntent::Reveal,
            Placement::Focused,
            window,
            cx,
        )
        .unwrap();
    let instance = shell
        .areas
        .workspace
        .scripts
        .panel_keys
        .borrow_mut()
        .issue()
        .unwrap();
    let surface = Surface::Panel {
        workspace: workspace.clone(),
        tab,
        instance,
    };
    let context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let mut machine = vessel_app_view::Machine::unreachable("fixture".into(), "isolated fixture");
    machine.node = Some(context.node());
    machine.connection_epoch = Some(context.epoch());
    shell.fleet.machines.push(machine);
    let target = Target::legacy_panel(context, surface.clone()).unwrap();
    let mut controls = Controls::new(surface, window, cx);
    controls.model.select(Some(target.clone()));
    let request = controls.model.begin_read().unwrap();
    assert!(controls.model.accept_read(
        &target,
        request,
        Ok(Snapshot {
            host: None,
            scripts: ScriptList {
                root: "/fixture".into(),
                runs: vec![],
                commands: ProjectCommands::Listed {
                    setup: vec![],
                    teardown: vec![],
                    terminals: vec![],
                    service_ports: None,
                    scripts: vec![ScriptEntry::new("dev".parse().unwrap(), ScriptKind::Service,
                "serve".parse().unwrap(), None, None).unwrap()],
                    digest: "a".repeat(64).parse().unwrap(),
                    approved: true,
                    last_approved: None,
                }
            },
            ports: Ok(PortList {
                ports: vec![],
                scan: PortScan::Pending,
                revision: 0,
                more: false
            }),
        })
    ));
    assert!(
        shell
            .areas
            .workspace
            .scripts
            .panels
            .borrow_mut()
            .insert((workspace, tab), controls)
            .is_none()
    );
    target
}
fn aba(shell: &mut Shell, target: &Target, cx: &mut Context<'_, Shell>) {
    let (workspace, tab) = key(&target.surface).unwrap();
    for next in [Panel::WorkingDiff, Panel::Scripts] {
        let _tab = shell
            .update_layout(
                workspace,
                |layout| {
                    layout.retarget(RetargetRequest {
                        tab,
                        target: next,
                        intent: OpenIntent::Reveal,
                        role: AgentRole::Unknown,
                    })
                },
                cx,
            )
            .unwrap();
    }
    assert!(shell.layout_tab_matches(workspace, tab, &Panel::Scripts));
}

#[gpui::test]
fn sent_write_and_unknown_receipt_survive_synchronous_aba_retirement(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let receiver = Rc::new(RefCell::new(None));
    let receiver_slot = Rc::clone(&receiver);
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let (shell, commands) = language_tests::with_commands(window, cx);
        *receiver_slot.borrow_mut() = Some(commands);
        shell
    });
    shell.update_in(cx, |shell, window, cx| {
        let target = opened(shell, window, cx);
        let (workspace, tab) = key(&target.surface).unwrap();
        let action = Action::Run("dev".parse().unwrap());
        let request = shell
            .areas
            .workspace
            .scripts
            .panels
            .borrow_mut()
            .get_mut(&(workspace.clone(), tab))
            .unwrap()
            .model
            .begin_write(action.clone())
            .unwrap();
        shell.ask(
            target.machine(),
            Ask::Scripts(ScriptAsk::Write {
                target: target.clone(),
                key: request,
                action: action.clone(),
            }),
        );
        aba(shell, &target, cx);
        assert!(!shell.script_panel_is_current(&target));
        let commands = script_commands(receiver.borrow_mut().as_mut().unwrap());
        assert_eq!(commands.len(), 1);
        let ScriptAsk::Write {
            target: sent,
            key: sent_key,
            action: sent_action,
        } = &commands[0]
        else {
            panic!("original detached command missing");
        };
        assert_eq!((sent, *sent_key, sent_action), (&target, request, &action));
        let result = Err(vessel_app_view::workspace::problem::Problem::Gone(
            "unknown outcome".into(),
        ));
        shell.script_panel_answer(
            ScriptAnswer::Written {
                target: target.clone(),
                key: request,
                result: result.clone(),
            },
            cx,
        );
        let states = shell.areas.workspace.scripts.panels.borrow();
        let controls = states.get(&(workspace.clone(), tab)).unwrap();
        assert!(controls.retired);
        assert!(controls.model.target.is_none());
        assert!(controls.model.writing.is_none());
        let write = controls.last_write.as_ref().unwrap();
        assert_eq!(write.target(), &target);
        assert_eq!(write.action(), &action);
        assert_eq!(write.receipt(), Some(&result));
        assert!(script_commands(receiver.borrow_mut().as_mut().unwrap()).is_empty());
    });
}

#[gpui::test]
fn idle_retirement_removes_old_controls_and_new_surface_cannot_accept_old_read(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = opened(shell, window, cx);
        let (workspace, tab) = key(&target.surface).unwrap();
        aba(shell, &target, cx);
        assert!(
            !shell
                .areas
                .workspace
                .scripts
                .panels
                .borrow()
                .contains_key(&(workspace.clone(), tab))
        );
        assert!(!shell.script_panel_is_current(&target));
        let instance = shell
            .areas
            .workspace
            .scripts
            .panel_keys
            .borrow_mut()
            .issue()
            .unwrap();
        let surface = Surface::Panel {
            workspace: workspace.clone(),
            tab,
            instance,
        };
        assert_ne!(surface, target.surface);
        let mut successor = Controls::new(surface, window, cx);
        successor.model.select(Target::legacy_panel(
            target.context.clone(),
            successor.surface.clone(),
        ));
        let request = successor.model.begin_read().unwrap();
        assert!(!successor.model.accept_read(
            &target,
            request,
            Err(vessel_app_view::workspace::problem::Problem::Unsent)
        ));
        assert!(successor.model.action_ticket().is_none());
    });
}

#[gpui::test]
fn connection_retirement_retains_write_receipt_before_any_layout_or_poll(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let target = opened(shell, window, cx);
        assert!(shell.script_panel_is_current(&target));
        let (workspace, tab) = key(&target.surface).unwrap();
        let request = shell
            .areas
            .workspace
            .scripts
            .panels
            .borrow_mut()
            .get_mut(&(workspace.clone(), tab))
            .unwrap()
            .model
            .begin_write(Action::Run("dev".parse().unwrap()))
            .unwrap();
        shell.fleet.machines[0].connection_epoch = None;
        assert!(!shell.script_panel_is_current(&target));
        let result = Err(vessel_app_view::workspace::problem::Problem::Gone(
            "unknown outcome".into(),
        ));
        shell.script_panel_answer(
            ScriptAnswer::Written {
                target: target.clone(),
                key: request,
                result: result.clone(),
            },
            cx,
        );
        let states = shell.areas.workspace.scripts.panels.borrow();
        let controls = states.get(&(workspace.clone(), tab)).unwrap();
        assert!(controls.retired);
        assert!(controls.model.target.is_none());
        assert_eq!(
            controls.last_write.as_ref().unwrap().receipt(),
            Some(&result)
        );
        assert!(shell.layout_tab_matches(workspace, tab, &Panel::Scripts));
    });
}

#[gpui::test]
fn retirement_reload_requires_a_receipt_and_a_new_explicit_action(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let receiver = Rc::new(RefCell::new(None));
    let receiver_slot = Rc::clone(&receiver);
    let (shell, cx) = cx.add_window_view(move |window, cx| {
        let (shell, commands) = language_tests::with_commands(window, cx);
        *receiver_slot.borrow_mut() = Some(commands);
        shell
    });
    shell.update_in(cx, |_, window, _| window.activate_window());
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert!(window.is_window_active());
        let target = opened(shell, window, cx);
        let (workspace, tab) = key(&target.surface).unwrap();
        let vessel_proto::scripts::ScriptPlace::Session { session } = target.place else {
            panic!("legacy session fixture");
        };
        shell.route = crate::routes::Route::Session;
        shell.areas.session.open = Some(crate::areas::session::Open {
            machine: "fixture".into(), session, title: "isolated".into(),
            agent: "fake".into(), structured: true, live: true,
        });
        assert!(shell.layout_panel_visible(workspace, tab, window));
        let action = Action::Run("dev".parse().unwrap());
        let (old_ticket, request, snapshot) = {
            let mut states = shell.areas.workspace.scripts.panels.borrow_mut();
            let model = &mut states.get_mut(&(workspace.clone(), tab)).unwrap().model;
            let ticket = model.action_ticket().unwrap();
            let snapshot = model.read.ready().unwrap().clone();
            let request = model.begin_write_at(&ticket, action.clone()).unwrap();
            (ticket, request, snapshot)
        };
        aba(shell, &target, cx);
        reload_after_receipt(shell, &target.surface, window, cx);
        assert!(shell.areas.workspace.scripts.panels.borrow()
            .get(&(workspace.clone(), tab)).unwrap().retired);
        let result = Err(vessel_app_view::workspace::problem::Problem::Gone("unknown outcome".into()));
        shell.script_panel_answer(ScriptAnswer::Written {
            target: target.clone(), key: request, result: result.clone(),
        }, cx);
        reload_after_receipt(shell, &target.surface, window, cx);
        let (next, read) = {
            let states = shell.areas.workspace.scripts.panels.borrow();
            let controls = states.get(&(workspace.clone(), tab)).unwrap();
            assert!(!controls.retired);
            assert_ne!(controls.surface, target.surface);
            assert!(controls.model.action_ticket().is_none());
            assert!(controls.model.writing.is_none());
            assert_eq!(controls.last_write.as_ref().unwrap().receipt(), Some(&result));
            (controls.model.target.clone().unwrap(), controls.model.read.outstanding().unwrap())
        };
        reload_after_receipt(shell, &target.surface, window, cx);
        shell.script_panel_action(&old_ticket, action.clone(), window, cx);
        let commands = script_commands(receiver.borrow_mut().as_mut().unwrap());
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], ScriptAsk::Read { target, key } if target == &next && *key == read));
        shell.script_panel_answer(ScriptAnswer::Read { target: next, key: read, result: Ok(snapshot) }, cx);
        let fresh = shell.areas.workspace.scripts.panels.borrow()
            .get(&(workspace.clone(), tab)).unwrap().model.action_ticket().unwrap();
        shell.script_panel_action(&fresh, action, window, cx);
        let commands = script_commands(receiver.borrow_mut().as_mut().unwrap());
        assert_eq!(commands.len(), 1);
        assert!(matches!(&commands[0], ScriptAsk::Write { target, .. } if target == &fresh.target));
        assert!(shell.areas.workspace.scripts.panels.borrow()
            .get(&(workspace.clone(), tab)).unwrap().last_write.is_none());
    });
}
