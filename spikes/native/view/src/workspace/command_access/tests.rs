use super::*;
use crate::connection::{ConnectionEpochs, MachineKey};
use vessel_proto::NodeId;

fn target() -> Target {
    Target {
        context: ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), NodeId::random())
            .unwrap(),
        project: ProjectId::random(),
    }
}
fn commands(approved: bool) -> ProjectCommands {
    ProjectCommands::Listed {
        setup: vec!["echo approved".parse().unwrap()],
        teardown: vec![],
        scripts: vec![],
        terminals: vec![],
        service_ports: None,
        digest: "a".repeat(64).parse().unwrap(),
        approved,
        last_approved: None,
    }
}
fn ready(approved: bool) -> (Model, Target) {
    let target = target();
    let mut model = Model::default();
    let key = model.begin_read(target.clone()).unwrap();
    assert!(model.accept_read(&target, key, Ok(commands(approved))));
    (model, target)
}
#[test]
fn refreshed_or_reopened_confirmations_cannot_retarget_a_new_snapshot() {
    let (mut model, target) = ready(false);
    let old = model.ticket().unwrap();
    let key = model.begin_read(target.clone()).unwrap();
    assert!(model.begin_write(&old, true).is_none());
    assert!(model.accept_read(&target, key, Ok(commands(false))));
    assert!(model.begin_write(&old, true).is_none());
    let current = model.ticket().unwrap();
    assert_ne!(current.element_key(), old.element_key());
    assert!(model.begin_write(&current, true).is_some());
    assert!(model.begin_write(&current, true).is_none());
}
#[test]
fn changing_an_admission_drops_old_reads_and_grants_without_replay() {
    let mut epochs = ConnectionEpochs::default();
    let node = NodeId::random();
    let project = ProjectId::random();
    let first = Target {
        context: epochs
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap(),
        project,
    };
    let mut model = Model::default();
    let read = model.begin_read(first.clone()).unwrap();
    assert!(model.accept_read(&first, read, Ok(commands(false))));
    let ticket = model.ticket().unwrap();
    let write = model.begin_write(&ticket, true).unwrap();
    let replacement = Target {
        context: epochs
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap(),
        project,
    };
    let new_read = model.begin_read(replacement.clone()).unwrap();
    assert!(!model.accept_write(&first, write, &ticket.digest, true, Ok(commands(true))));
    assert!(!model.accept_read(&first, read, Ok(commands(false))));
    assert!(model.ticket().is_none());
    assert!(model.accept_read(&replacement, new_read, Ok(commands(false))));
}
#[test]
fn unknown_write_requires_a_fresh_read_and_a_mismatched_digest_cannot_settle_it() {
    let (mut model, target) = ready(false);
    let ticket = model.ticket().unwrap();
    let key = model.begin_write(&ticket, true).unwrap();
    let other: CommandsDigest = "b".repeat(64).parse().unwrap();
    assert!(!model.accept_write(&target, key, &other, true, Ok(commands(true))));
    assert!(model.accept_write(&target, key, &ticket.digest, true, Err(Problem::Unsent)));
    assert!(model.ticket().is_none());
    assert!(model.begin_write(&ticket, true).is_none());
    let read = model.begin_read(target.clone()).unwrap();
    assert!(model.accept_read(&target, read, Ok(commands(true))));
    let current = model.ticket().unwrap();
    assert!(model.begin_write(&current, false).is_some());
}

#[test]
fn unsupported_executable_fields_refuse_new_grants_but_allow_withdrawal() {
    for approved in [false, true] {
        let target = target();
        let mut data = commands(approved);
        let ProjectCommands::Listed { terminals, .. } = &mut data else {
            panic!("listed fixture")
        };
        terminals.push(vessel_proto::workspace::FileTerminal {
            name: None,
            command: "echo automatic".parse().unwrap(),
        });
        let mut model = Model::default();
        let key = model.begin_read(target.clone()).unwrap();
        assert!(model.accept_read(&target, key, Ok(data)));
        let ticket = model.ticket().unwrap();
        if approved {
            assert!(model.begin_write(&ticket, false).is_some());
        } else {
            assert!(model.begin_write(&ticket, true).is_none());
            assert!(model.writing.is_none());
        }
    }
}
