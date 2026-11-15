use super::*;
use crate::schedules::load::{MachineKey, Tickets};
use crate::schedules::page::tests::{context, rebound};
fn request() -> Request {
    Request::on(
        context(&MachineKey::of("laptop").unwrap()),
        Tickets::default().issue().unwrap(),
        Operation::Targets,
    )
}
fn targets() -> Value {
    Value::Targets(Box::new(Targets {
        projects: vec![],
        agents: vec![],
        harnesses: vec![],
        timezone: None,
    }))
}
#[test]
fn same_alias_new_admission_and_different_operation_cannot_settle_old_editor_work() {
    let old = request();
    let mut mailbox = Mailbox::default();
    mailbox.begin(old.clone());
    let wrong = Request::on(
        rebound(&MachineKey::of("laptop").unwrap()),
        old.ticket(),
        Operation::Targets,
    );
    assert!(!mailbox.settle(&wrong, Ok(targets())));
    let wrong = Request::on(
        old.connection().clone(),
        old.ticket(),
        Operation::Options(GetHarnessOptions {
            harness: "claude-code".parse().unwrap(),
            profile: None,
        }),
    );
    assert!(!mailbox.settle(&wrong, Ok(targets())));
    assert_eq!(mailbox.request(), Some(&old));
    assert!(mailbox.value().is_none());
    assert!(mailbox.settle(&old, Ok(targets())));
    assert!(matches!(mailbox.value(), Some(Value::Targets(_))));
    assert!(!mailbox.settle(&old, Err(Failure::Offline)));
    mailbox.invalidate();
    assert!(mailbox.value().is_none());
    assert!(mailbox.request().is_none());
    assert!(!mailbox.settle(&old, Ok(targets())));
}
#[test]
fn a_matching_ticket_still_cannot_install_another_methods_payload() {
    let request = request();
    let mut mailbox = Mailbox::default();
    mailbox.begin(request.clone());
    assert!(mailbox.settle(
        &request,
        Ok(Value::Saved(Box::new(crate::schedules::tests::info())))
    ));
    assert!(mailbox.value().is_none());
    assert_eq!(mailbox.failure(), Some(&Failure::Unexpected));
}
#[test]
fn update_reply_must_name_the_schedule_that_was_opened() {
    let info = crate::schedules::tests::info();
    let request = Request::on(
        request().connection().clone(),
        Tickets::default().issue().unwrap(),
        Operation::Save(Box::new(Save::Update(
            vessel_proto::automation::UpdateSchedule {
                schedule: info.id,
                revision: info.revision,
                definition: info.definition.clone(),
            },
        ))),
    );
    let mut wrong = info.clone();
    wrong.id = "00000000-0000-4000-8000-000000000099".parse().unwrap();
    let mut mailbox = Mailbox::default();
    mailbox.begin(request.clone());
    assert!(mailbox.settle(&request, Ok(Value::Saved(Box::new(wrong)))));
    assert_eq!(mailbox.failure(), Some(&Failure::Unconfirmed));
    mailbox.begin(request.clone());
    assert!(mailbox.settle(&request, Ok(Value::Saved(Box::new(info)))));
    assert!(mailbox.failure().is_none());
}

#[test]
fn reconnect_clears_read_data_and_keeps_a_dispatched_save_uncertain() {
    let read = request();
    let key = MachineKey::of("laptop").unwrap();
    let mut mailbox = Mailbox::default();
    mailbox.begin(read.clone());
    assert!(mailbox.settle(&read, Ok(targets())));
    assert!(!mailbox.connected(&MachineKey::of("other").unwrap()));
    assert!(mailbox.connected(&key));
    assert!(mailbox.value().is_none());
    assert_eq!(mailbox.failure(), Some(&Failure::ConnectionChanged));
    assert!(!mailbox.connected(&key));
    let info = crate::schedules::tests::info();
    let save = Request::on(
        read.connection().clone(),
        read.ticket(),
        Operation::Save(Box::new(Save::Create(
            vessel_proto::automation::CreateSchedule {
                definition: info.definition,
                paused: false,
            },
        ))),
    );
    mailbox.begin(save.clone());
    assert!(mailbox.connected(&key));
    assert_eq!(mailbox.failure(), Some(&Failure::Unconfirmed));
    assert!(!mailbox.settle(
        &save,
        Ok(Value::Saved(Box::new(crate::schedules::tests::info())))
    ));
    assert!(!mailbox.reconcile(&crate::Fleet {
        machines: vec![],
        read_at: None
    }));
    assert_eq!(mailbox.failure(), Some(&Failure::Unconfirmed));
}
