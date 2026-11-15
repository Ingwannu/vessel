use super::*;
use crate::shell::language_tests;
use gpui::TestAppContext;
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::workspace::scripts::{Action, Snapshot, Surface};
use vessel_proto::ports::{PortList, PortScan};
use vessel_proto::scripts::{ScriptList, ScriptPlace};
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalPlace};
use vessel_proto::workspace::ProjectCommands;
use vessel_proto::{NodeId, ProjectId, TerminalId};

fn target() -> Target {
    Target {
        context: ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), NodeId::random())
            .unwrap(),
        place: ScriptPlace::Project {
            project: ProjectId::random(),
        },
        surface: Surface::Project,
    }
}
fn sent(scripts: &mut Scripts, target: &Target) -> (RequestKey, Action) {
    let ScriptPlace::Project { project } = target.place else {
        panic!("project fixture");
    };
    let terminal = TerminalId::random();
    scripts.model.select(Some(target.clone()));
    let read = scripts.model.begin_read().unwrap();
    assert!(scripts.model.accept_read(
        target,
        read,
        Ok(Snapshot {
            host: None,
            scripts: ScriptList {
                root: "/fixture".into(),
                commands: ProjectCommands::None,
                runs: vec![TerminalInfo::new(
                    terminal,
                    TerminalPlace::Project { project, dir: None },
                    "/fixture".into(),
                    TerminalKind::Service {
                        script: "dev".parse().unwrap(),
                        command: "serve".parse().unwrap()
                    },
                    chrono::DateTime::UNIX_EPOCH
                )]
            },
            ports: Ok(PortList {
                ports: vec![],
                scan: PortScan::Pending,
                revision: 0,
                more: false
            }),
        })
    ));
    let action = Action::Stop(terminal);
    (scripts.model.begin_write(action.clone()).unwrap(), action)
}

#[gpui::test]
fn project_navigation_retains_original_write_and_cannot_apply_its_reply_to_successor(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, cx| {
        let original = target();
        let scripts = &mut shell.areas.workspace.scripts;
        let (key, action) = sent(scripts, &original);
        scripts.retire_project();
        assert!(scripts.has_pending(&original));
        let successor = target();
        scripts.model.select(Some(successor.clone()));
        let result = Err(vessel_app_view::workspace::problem::Problem::Gone(
            "unknown outcome".into(),
        ));
        shell.scripts_answer(
            ScriptAnswer::Written {
                target: original.clone(),
                key,
                result: result.clone(),
            },
            cx,
        );
        let scripts = &shell.areas.workspace.scripts;
        assert_eq!(scripts.model.target.as_ref(), Some(&successor));
        assert!(scripts.model.writing.is_none());
        assert!(!scripts.has_pending(&original));
        let old = &scripts.retired_projects[0];
        assert_eq!(old.target(), &original);
        assert_eq!(old.action(), &action);
        assert_eq!(old.receipt(), Some(&result));
    });
}

#[gpui::test]
fn acknowledgement_keeps_pending_and_later_unseen_received_results(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, _cx| {
        let scripts = &mut shell.areas.workspace.scripts;
        let first = target();
        let (key, _) = sent(scripts, &first);
        scripts.retire_project();
        assert!(scripts.retired_projects[0].accept(&first, key, Ok(None)));
        let drawn = scripts.received();
        let next = target();
        let (next_key, _) = sent(scripts, &next);
        scripts.retire_project();
        scripts.dismiss_received(&drawn);
        assert_eq!(scripts.retired_projects.len(), 1);
        assert!(scripts.has_pending(&next));
        assert!(scripts.retired_projects[0].accept(&next, next_key, Ok(None)));
        scripts.dismiss_received(&drawn);
        assert_eq!(
            scripts.retired_projects.len(),
            1,
            "an old button cannot discard an unseen reply"
        );
        let current = scripts.received();
        scripts.dismiss_received(&current);
        assert!(scripts.retired_projects.is_empty());
    });
}

#[gpui::test]
fn project_slots_are_charged_before_dispatch_and_pending_records_are_never_evicted(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, _window, _cx| {
        let scripts = &mut shell.areas.workspace.scripts;
        for _ in 0..MAX_RETAINED_SURFACES {
            assert!(scripts.admits_project_write());
            let _sent = sent(scripts, &target());
            scripts.retire_project();
        }
        assert!(!scripts.admits_project_write());
        assert!(scripts.received().is_empty());
        scripts.dismiss_received(&[]);
        assert_eq!(scripts.retired_projects.len(), MAX_RETAINED_SURFACES);
    });
}
