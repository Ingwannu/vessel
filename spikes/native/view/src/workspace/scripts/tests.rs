//! Rules exercised without UI, a real node or a real agent.
use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use chrono::{TimeZone as _, Utc};
use vessel_proto::ports::{ListeningPort, PortList, PortOwner, PortScan};
use vessel_proto::scripts::{ScriptEntry, ScriptKind, ScriptList, ScriptPlace};
use vessel_proto::terminals::{ServiceState, TerminalInfo, TerminalKind, TerminalPlace};
use vessel_proto::workspace::ProjectCommands;
use vessel_proto::{ProjectId, SessionId, TerminalId};

fn target() -> Target {
    Target {
        context: ConnectionEpochs::default()
            .admit(
                MachineKey::from_book("laptop"),
                vessel_proto::NodeId::random(),
            )
            .unwrap(),
        place: ScriptPlace::Project {
            project: ProjectId::random(),
        },
        surface: Surface::Project,
    }
}
fn run(target: &Target) -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        TerminalPlace::Project {
            project: match target.place {
                ScriptPlace::Project { project } => project,
                ScriptPlace::Session { .. } => panic!("project fixture"),
            },
            dir: None,
        },
        "/project".into(),
        TerminalKind::Service {
            script: "dev".parse().unwrap(),
            command: "serve".parse().unwrap(),
        },
        Utc.timestamp_opt(100, 0).unwrap(),
    )
}
fn snapshot(_target: &Target) -> Snapshot {
    Snapshot {
        host: None,
        scripts: ScriptList {
            root: "/project".into(),
            commands: ProjectCommands::Listed {
                setup: vec![],
                teardown: vec![],
                terminals: vec![],
                service_ports: None,
                scripts: vec![
                    ScriptEntry::new(
                        "dev".parse().unwrap(),
                        ScriptKind::Service,
                        "serve".parse().unwrap(),
                        None,
                        None,
                    )
                    .unwrap(),
                ],
                digest: "a".repeat(64).parse().unwrap(),
                approved: true,
                last_approved: None,
            },
            runs: vec![],
        },
        ports: Ok(PortList {
            ports: vec![],
            scan: PortScan::Pending,
            revision: 0,
            more: false,
        }),
    }
}
fn ready() -> (Model, Target) {
    let target = target();
    let mut model = Model::default();
    model.select(Some(target.clone()));
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(snapshot(&target))));
    (model, target)
}
#[test]
fn older_opens_and_other_machines_cannot_replace_a_snapshot() {
    let target = target();
    let mut model = Model::default();
    model.select(Some(target.clone()));
    let old = model.begin_read().unwrap();
    model.select(Some(target.clone()));
    let current = model.begin_read().unwrap();
    assert!(!model.accept_read(&target, old, Ok(snapshot(&target))));
    let mut other = target.clone();
    other.context = ConnectionEpochs::default()
        .admit(MachineKey::from_book("server"), target.context.node())
        .unwrap();
    assert!(!model.accept_read(&other, current, Ok(snapshot(&target))));
    assert!(model.accept_read(&target, current, Ok(snapshot(&target))));
    assert!(!model.accept_read(&target, current, Ok(snapshot(&target))));
}
#[test]
fn only_current_approved_names_run_and_duplicate_presses_send_nothing() {
    let (mut model, target) = ready();
    let action = Action::Run("dev".parse().unwrap());
    assert!(!model.permits(&Action::Run("missing".parse().unwrap())));
    let key = model.begin_write(action.clone()).unwrap();
    assert!(model.begin_write(action.clone()).is_none());
    assert!(model.begin_read().is_none());
    assert!(model.accept_write(
        &target,
        key,
        Err(crate::workspace::problem::Problem::Unsent)
    ));
    assert!(model.begin_write(action).is_none());
    assert!(matches!(model.issue, Some(Issue::Call(_))));
}

#[test]
fn retirement_withdraws_tickets_but_retains_the_original_write_and_unknown_receipt() {
    let (mut model, target) = ready();
    let ticket = model.action_ticket().unwrap();
    let action = Action::Run("dev".parse().unwrap());
    let key = model.begin_write_at(&ticket, action.clone()).unwrap();
    let mut retired = model.retire().unwrap();
    assert_eq!(retired.target(), &target);
    assert_eq!(retired.action(), &action);
    assert!(retired.receipt().is_none());
    assert!(model.target.is_none());
    assert!(model.action_ticket().is_none());
    assert!(model.begin_write_at(&ticket, action.clone()).is_none());
    assert!(model.begin_read().is_none());
    let mut other = target.clone();
    other.place = ScriptPlace::Project {
        project: ProjectId::random(),
    };
    assert!(!retired.accept(&other, key, Ok(None)));
    let unknown = Err(crate::workspace::problem::Problem::Gone(
        "unknown outcome".into(),
    ));
    assert!(retired.accept(&target, key, unknown.clone()));
    assert_eq!(retired.receipt(), Some(&unknown));
    assert!(!retired.accept(&target, key, Ok(None)));
    model.select(Some(target.clone()));
    assert!(model.begin_write_at(&ticket, action).is_none());
    assert!(model.begin_read().is_some());
    assert_eq!(retired.receipt(), Some(&unknown));
}
#[test]
fn refresh_failure_and_unapproved_sets_never_authorize_a_write() {
    let (mut model, target) = ready();
    let action = Action::Run("dev".parse().unwrap());
    let key = model.begin_read().unwrap();
    assert!(model.read.shown().is_some());
    assert!(!model.permits(&action));
    assert!(model.accept_read(&target, key, Err(crate::workspace::problem::Problem::Older)));
    assert!(!model.permits(&action));
    let mut next = snapshot(&target);
    if let ProjectCommands::Listed { approved, .. } = &mut next.scripts.commands {
        *approved = false;
    }
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(next)));
    assert!(!model.permits(&action));
}

#[test]
fn retained_rows_survive_reconciliation_without_authorizing_actions_or_other_owners() {
    let (mut model, target) = ready();
    let action = Action::Run("dev".parse().unwrap());
    let write = model.begin_write(action.clone()).unwrap();
    assert!(model.accept_write(&target, write, Ok(None)));
    assert!(model.shown().is_some());
    assert!(model.read.ready().is_none());
    assert!(!model.permits(&action));
    let read = model.begin_read().unwrap();
    assert!(model.shown().is_some());
    assert!(!model.permits(&action));
    assert!(model.accept_read(
        &target,
        read,
        Err(crate::workspace::problem::Problem::Unsent)
    ));
    assert!(model.shown().is_some());
    assert!(!model.permits(&action));
    let fresh = model.begin_read().unwrap();
    assert!(model.accept_read(&target, fresh, Ok(snapshot(&target))));
    assert!(model.permits(&action));
    model.select(Some(Target {
        context: target.context,
        place: ScriptPlace::Project {
            project: ProjectId::random(),
        },
        surface: Surface::Project,
    }));
    assert!(model.shown().is_none());
    assert!(!model.permits(&action));
}
#[test]
fn latest_run_and_port_ownership_exclude_other_processes_and_ended_runs() {
    let target = target();
    let old = run(&target);
    let mut new = old.clone();
    new.id = TerminalId::random();
    new.created_at = Utc.timestamp_opt(101, 0).unwrap();
    let list = vec![new.clone(), old.clone()];
    assert_eq!(
        latest_run(&"dev".parse().unwrap(), &list).unwrap().id,
        new.id
    );
    let listener = |owner| ListeningPort {
        port: 3000,
        address: "127.0.0.1".parse().unwrap(),
        pid: 1,
        process: None,
        owner,
    };
    let ports = PortList {
        ports: vec![
            listener(PortOwner::Terminal { terminal: old.id }),
            listener(PortOwner::Terminal { terminal: new.id }),
            listener(PortOwner::Session {
                session: SessionId::random(),
            }),
        ],
        scan: PortScan::Pending,
        revision: 0,
        more: false,
    };
    assert_eq!(listeners(Some(&new), &ports).len(), 1);
    new.status = vessel_proto::SessionStatus::Exited { code: Some(0) };
    assert!(listeners(Some(&new), &ports).is_empty());
}
#[test]
fn service_backoff_and_unknown_are_never_called_healthy() {
    let target = target();
    let mut run = run(&target);
    assert_eq!(run_state(Some(&run)), RunState::Running);
    run.service = Some(ServiceState::BackingOff {
        attempt: 2,
        code: Some(1),
        restart_at: Utc.timestamp_opt(102, 0).unwrap(),
    });
    assert_eq!(run_state(Some(&run)), RunState::Backoff);
    run.service = Some(ServiceState::Unknown);
    assert_eq!(run_state(Some(&run)), RunState::Unknown);
}
#[test]
fn stop_and_restart_cannot_target_a_shell_or_an_old_run() {
    let (mut model, target) = ready();
    let mut snapshot = snapshot(&target);
    let old = run(&target);
    let mut current = old.clone();
    current.id = TerminalId::random();
    current.created_at = Utc.timestamp_opt(102, 0).unwrap();
    snapshot.scripts.runs = vec![old.clone(), current.clone()];
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(snapshot)));
    assert!(!model.permits(&Action::Restart(old.id)));
    assert!(model.permits(&Action::Restart(current.id)));
    assert!(model.permits(&Action::Stop(current.id)));
    assert!(!model.permits(&Action::Run("dev".parse().unwrap())));
}

#[test]
fn browser_preview_rechecks_socket_identity_and_refuses_remote_loopback() {
    let (mut model, target) = ready();
    let mut next = snapshot(&target);
    let run = run(&target);
    let listener = ListeningPort {
        port: 3000,
        address: "127.0.0.1".parse().unwrap(),
        pid: 1,
        process: None,
        owner: PortOwner::Terminal { terminal: run.id },
    };
    next.host = Some(PreviewHost::parse("remote.example").unwrap());
    next.scripts.runs = vec![run.clone()];
    next.ports = Ok(PortList {
        ports: vec![listener.clone()],
        scan: PortScan::Scanned {
            at: Utc.timestamp_opt(102, 0).unwrap(),
        },
        revision: 1,
        more: false,
    });
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(next)));
    let preview = Preview::observed(&run, listener);
    assert_eq!(
        model.preview_url(&preview, ""),
        Err(PreviewProblem::RemoteLoopback)
    );
    assert_eq!(
        model.preview_url(&preview, "localhost").unwrap().as_str(),
        "http://localhost:3000/"
    );
    let mut stale = preview.clone();
    stale.listener.pid = 2;
    assert_eq!(
        model.preview_url(&stale, "localhost"),
        Err(PreviewProblem::Stale)
    );
    assert!(model.begin_write(Action::Restart(run.id)).is_some());
    assert_eq!(
        model.preview_url(&preview, "localhost"),
        Err(PreviewProblem::Stale)
    );
}

#[test]
fn removing_a_script_entry_does_not_hide_its_retained_supervised_run() {
    let target = target();
    let mut next = snapshot(&target);
    let run = run(&target);
    next.scripts.runs = vec![run.clone()];
    assert!(retained_runs(&next.scripts).is_empty());
    next.scripts.commands = ProjectCommands::None;
    assert_eq!(retained_runs(&next.scripts), vec![&run]);
    assert_eq!(Entry::retained(&run).unwrap().name.as_str(), "dev");
    next.scripts.runs.first_mut().unwrap().status =
        vessel_proto::SessionStatus::Exited { code: Some(0) };
    assert!(retained_runs(&next.scripts).is_empty());
}

#[test]
fn declared_port_is_displayed_but_never_becomes_an_observed_listener() {
    let target = target();
    let mut snapshot = snapshot(&target);
    let declared = ScriptEntry::new(
        "dev".parse().unwrap(),
        ScriptKind::Service,
        "serve".parse().unwrap(),
        None,
        Some(
            "4321"
                .parse::<vessel_proto::workspace::ServicePort>()
                .unwrap(),
        ),
    )
    .unwrap();
    let entry = Entry::of(&declared);
    assert_eq!(entry.port, declared.port());
    assert_eq!(entry.kind, declared.kind());
    let terminal = run(&target);
    snapshot.scripts.runs.push(terminal.clone());
    if let Ok(ports) = &mut snapshot.ports {
        ports.scan = PortScan::Scanned {
            at: chrono::DateTime::UNIX_EPOCH,
        };
        assert!(listeners(Some(&terminal), ports).is_empty());
    }
}

#[test]
fn a_service_restart_cannot_reuse_an_old_preview_callback_even_with_the_same_id_and_socket() {
    let (mut model, target) = ready();
    let mut current = snapshot(&target);
    let run = run(&target);
    let listener = ListeningPort {
        port: 3000,
        address: "127.0.0.1".parse().unwrap(),
        pid: 1,
        process: None,
        owner: PortOwner::Terminal { terminal: run.id },
    };
    let preview = Preview::observed(&run, listener.clone());
    current.scripts.runs = vec![run];
    current.ports = Ok(PortList {
        ports: vec![listener],
        scan: PortScan::Scanned {
            at: Utc.timestamp_opt(102, 0).unwrap(),
        },
        revision: 1,
        more: false,
    });
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(current.clone())));
    assert!(model.preview_url(&preview, "localhost").is_ok());
    current.scripts.runs[0].revision = current.scripts.runs[0].revision.checked_add(1).unwrap();
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(current.clone())));
    assert_eq!(
        model.preview_url(&preview, "localhost"),
        Err(PreviewProblem::Stale)
    );
    let now = Preview::observed(
        &current.scripts.runs[0],
        current.ports.as_ref().unwrap().ports[0].clone(),
    );
    assert!(model.preview_url(&now, "localhost").is_ok());
    current.scripts.runs[0].service = Some(ServiceState::BackingOff {
        attempt: 2,
        code: Some(1),
        restart_at: Utc.timestamp_opt(103, 0).unwrap(),
    });
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(&target, key, Ok(current)));
    assert_eq!(
        model.preview_url(&now, "localhost"),
        Err(PreviewProblem::Stale)
    );
}

#[test]
fn reconnecting_the_same_node_never_accepts_a_previous_admission_reply() {
    let mut epochs = ConnectionEpochs::default();
    let node = vessel_proto::NodeId::random();
    let old = Target {
        context: epochs.admit(MachineKey::from_book("laptop"), node).unwrap(),
        place: ScriptPlace::Project {
            project: ProjectId::random(),
        },
        surface: Surface::Project,
    };
    let mut model = Model::default();
    model.select(Some(old.clone()));
    let old_key = model.begin_read().unwrap();
    let fresh = Target {
        context: epochs.admit(MachineKey::from_book("laptop"), node).unwrap(),
        place: old.place,
        surface: Surface::Project,
    };
    model.select(Some(fresh.clone()));
    let fresh_key = model.begin_read().unwrap();
    assert!(!model.accept_read(&old, old_key, Ok(snapshot(&old))));
    assert!(model.read.ready().is_none());
    assert!(model.accept_read(&fresh, fresh_key, Ok(snapshot(&fresh))));
}

#[path = "tests/panels.rs"]
mod panels;

#[path = "tests/receipts.rs"]
mod receipts;
