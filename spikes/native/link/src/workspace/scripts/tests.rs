//! Exercise existing wire calls and correlated refusal paths, not a fake API.
use super::peer;
use super::*;
use crate::connections::{Connected, SelectedConnection};
use crate::testing::{Home, state};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::workspace::git::RequestKeys;
use vessel_app_view::workspace::scripts::Surface;
use vessel_proto::ports::{PortList, PortScan};
use vessel_proto::scripts::{ScriptList, ScriptPlace, ScriptRefusal};
use vessel_proto::workspace::ProjectCommands;
use vessel_proto::{Ack, Call, ErrorCode, ProjectId, Reply, TerminalId, WireError};
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
fn selected(peer: &peer::Peer) -> (SelectedConnection, Connected, Target) {
    let home = Home::new();
    let (mut actor, _events) = state(&home);
    let context = actor
        .admissions
        .admit(MachineKey::from_book("laptop"), peer.client.node().id)
        .unwrap();
    let connected = Connected::new(
        peer.client.clone(),
        vec![],
        vessel_client::book::BookEntry {
            address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
            token: None,
            credential: None,
            id: None,
        },
        context.clone(),
    );
    let _prior = actor.clients.insert("laptop".into(), connected.clone());
    let selected = actor.selected_connection(&context).unwrap();
    (
        selected,
        connected,
        Target {
            context,
            place: ScriptPlace::Project {
                project: ProjectId::random(),
            },
            surface: Surface::Project,
        },
    )
}
fn empty() -> ScriptList {
    ScriptList {
        root: "/project".into(),
        commands: ProjectCommands::None,
        runs: vec![],
    }
}
#[tokio::test]
async fn discovery_uses_only_existing_script_and_port_calls_and_keeps_partial_error() {
    let peer = peer::connect(|call| {
        if matches!(call, Call::ListScripts(_)) {
            Ok(Reply::ListScripts(empty()))
        } else if matches!(call, Call::ListPorts(_)) {
            Err(WireError::new(
                ErrorCode::Unsupported,
                "scanner not available",
            ))
        } else {
            panic!("unexpected call: {call:?}")
        }
    })
    .await;
    let (client, _connected, target) = selected(&peer);
    let key = RequestKeys::default().issue().unwrap();
    let answer = run(
        &client,
        ScriptAsk::Read {
            target: target.clone(),
            key,
        },
    )
    .await;
    let ScriptAnswer::Read {
        target: returned,
        key: returned_key,
        result,
    } = answer
    else {
        panic!("wrong receipt");
    };
    assert_eq!(returned, target);
    assert_eq!(returned_key, key);
    let snapshot = result.unwrap();
    assert_eq!(snapshot.scripts, empty());
    assert_eq!(snapshot.host, None);
    assert!(snapshot.ports.is_err());
    let calls = peer.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
        calls
            .iter()
            .any(|call| matches!(call,Call::ListScripts(ask) if ask.place==target.place))
    );
    assert!(
        calls
            .iter()
            .any(|call| matches!(call, Call::ListPorts(ListPorts { changed_from: None })))
    );
}
#[tokio::test]
async fn run_restart_stop_preserve_exact_ids_and_do_not_retry_a_refusal() {
    let peer = peer::connect(|call| {
        if matches!(call, Call::RunScript(_)) {
            Ok(Reply::RunScript(ScriptRunOutcome::NeedsApproval {
                commands: ProjectCommands::None,
            }))
        } else if matches!(call, Call::RestartScript(_)) {
            Ok(Reply::RestartScript(ScriptRunOutcome::Refused {
                reason: ScriptRefusal::NoSuchScript,
            }))
        } else if matches!(call, Call::KillTerminal(_)) {
            Ok(Reply::KillTerminal(Ack {}))
        } else {
            panic!("unexpected call: {call:?}")
        }
    })
    .await;
    let (client, _connected, target) = selected(&peer);
    let terminal = TerminalId::random();
    let mut keys = RequestKeys::default();
    let actions = [
        Action::Run("dev".parse().unwrap()),
        Action::Restart(terminal),
        Action::Stop(terminal),
    ];
    for action in actions {
        let key = keys.issue().unwrap();
        let answer = run(
            &client,
            ScriptAsk::Write {
                target: target.clone(),
                key,
                action: action.clone(),
            },
        )
        .await;
        let ScriptAnswer::Written {
            target: returned,
            key: returned_key,
            result,
        } = answer
        else {
            panic!("wrong receipt");
        };
        assert_eq!(returned, target);
        assert_eq!(returned_key, key);
        assert!(result.is_ok());
    }
    let calls = peer.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert!(
        matches!(calls.first(),Some(Call::RunScript(ask)) if ask.script.as_str()=="dev" && ask.place==target.place)
    );
    assert!(
        calls
            .iter()
            .any(|call| matches!(call,Call::RestartScript(ask) if ask.terminal==terminal))
    );
    assert!(
        calls
            .iter()
            .any(|call| matches!(call,Call::KillTerminal(ask) if ask.terminal==terminal))
    );
}
#[test]
fn offline_and_unsent_keep_the_original_owner_and_key() {
    let target = target();
    let key = RequestKeys::default().issue().unwrap();
    let asks = [
        ScriptAsk::Read {
            target: target.clone(),
            key,
        },
        ScriptAsk::Write {
            target: target.clone(),
            key,
            action: Action::Run("dev".parse().unwrap()),
        },
    ];
    for ask in asks {
        match refused(ask, &Problem::Unsent) {
            ScriptAnswer::Read {
                target: returned,
                key: returned_key,
                result,
            } => {
                assert_eq!(target, returned);
                assert_eq!(key, returned_key);
                assert_eq!(result, Err(Problem::Unsent));
            }
            ScriptAnswer::Written {
                target: returned,
                key: returned_key,
                result,
            } => {
                assert_eq!(target, returned);
                assert_eq!(key, returned_key);
                assert_eq!(result, Err(Problem::Unsent));
            }
        }
    }
}
#[tokio::test]
async fn a_script_read_failure_still_settles_after_a_successful_scan() {
    let peer = peer::connect(|call| {
        if matches!(call, Call::ListScripts(_)) {
            Err(WireError::new(ErrorCode::NotFound, "gone"))
        } else if matches!(call, Call::ListPorts(_)) {
            Ok(Reply::ListPorts(PortList {
                ports: vec![],
                scan: PortScan::Pending,
                revision: 0,
                more: false,
            }))
        } else {
            panic!("unexpected call: {call:?}")
        }
    })
    .await;
    let (client, _connected, target) = selected(&peer);
    let answer = run(
        &client,
        ScriptAsk::Read {
            target,
            key: RequestKeys::default().issue().unwrap(),
        },
    )
    .await;
    assert!(matches!(
        answer,
        ScriptAnswer::Read {
            result: Err(Problem::Node {
                code: ErrorCode::NotFound,
                ..
            }),
            ..
        }
    ));
}

#[tokio::test]
async fn retiring_after_capture_before_first_poll_sends_no_script_frames() {
    let peer = peer::connect(|call| panic!("retired operation sent: {call:?}")).await;
    let (client, connected, target) = selected(&peer);
    let ask = ScriptAsk::Write {
        target,
        key: RequestKeys::default().issue().unwrap(),
        action: Action::Run("dev".parse().unwrap()),
    };
    let pending = run(&client, ask);
    connected.retire();
    assert!(matches!(
        pending.await,
        ScriptAnswer::Written {
            result: Err(Problem::Gone(_)),
            ..
        }
    ));
    assert!(peer.calls.lock().unwrap().is_empty());
}
