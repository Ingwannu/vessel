//! Unsent/local replies traverse the same correlation path as a real answer.
use super::*;
use vessel_app_link::schedules::{Answer, Message, Value};
use vessel_app_link::{Command, Rejected, Unsent, schedules::ScheduleError};
use vessel_app_view::schedules::load::Failure;
use vessel_app_view::schedules::{load::MachineKey, page::Body};
fn initial() -> Option<(Data, vessel_app_view::schedules::page::Request)> {
    let mut area = Data::default();
    area.page.machines(&[MachineKey::of("laptop")?]);
    let context = vessel_app_view::connection::ConnectionEpochs::default()
        .admit(
            vessel_app_view::connection::MachineKey::from_book("laptop"),
            "00000000-0000-4000-8000-000000000001".parse().ok()?,
        )
        .ok()?;
    let _changed = area.page.admitted(context);
    let request = area
        .page
        .due(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        .pop()?;
    Some((area, request))
}
#[test]
fn a_returned_submission_settles_without_fabricating_node_success() {
    for why in [Rejected::Full, Rejected::Closed] {
        let (mut area, request) = initial().unwrap();
        let returned = Unsent::new(why, Command::Schedules(request.into()));
        let crate::unsent::Settle::Answer(event) =
            crate::unsent::settle(returned, "unused diagnostic")
        else {
            panic!("no answer")
        };
        let vessel_app_link::Event::Schedules(Message::Replied(answer)) = *event else {
            panic!("wrong answer")
        };
        let expected = match why {
            Rejected::Full => Failure::Busy,
            Rejected::Closed => Failure::Offline,
        };
        area.answered(answer);
        assert_eq!(area.page.body(), Body::Failed);
        assert_eq!(area.page.problems()[0].1, &expected);
    }
}
#[test]
fn a_wrong_list_payload_and_browser_transport_refusal_are_not_empty_success() {
    let (mut area, request) = initial().unwrap();
    area.answered(Answer {
        request,
        result: Ok(Value::Written(
            vessel_app_view::schedules::page::WriteValue::Deleted,
        )),
    });
    assert_eq!(area.page.body(), Body::Failed);
    assert_eq!(area.page.problems()[0].1, &Failure::Unexpected);
    let (mut area, request) = initial().unwrap();
    area.answered(Answer {
        request,
        result: Err(ScheduleError::NoTransport),
    });
    assert_eq!(area.page.body(), Body::Failed);
    assert_eq!(area.page.problems()[0].1, &Failure::NoTransport);
    assert!(!area.page.retry_available());
}

#[test]
fn a_connected_notice_resets_old_refusal_without_needing_changed_fleet_data() {
    use vessel_app_view::schedules::Action;
    use vessel_app_view::schedules::page::{RowKey, Snapshot};
    let info: vessel_proto::automation::ScheduleInfo = serde_json::from_str(include_str!(
        "../../../../view/tests/fixtures/schedules/info.json"
    ))
    .unwrap();
    let (mut area, request) = initial().unwrap();
    let row = RowKey {
        machine: request.machine().clone(),
        schedule: info.id,
    };
    area.page.listed(
        &request,
        Ok(Snapshot {
            scopes: Err(Failure::Older),
            schedules: vec![info],
            read_at: chrono::DateTime::UNIX_EPOCH,
            projects: Ok(vec![]),
            worktrees: Ok(vec![]),
        }),
    );
    let pause = area.page.act(&row, Action::Pause).unwrap();
    area.page.written(
        &pause,
        Err(Failure::Node(vessel_proto::WireError::new(
            vessel_proto::ErrorCode::Forbidden,
            "diagnostic-only",
        ))),
    );
    assert!(area.page.row(&row).unwrap().actions().is_empty());
    area.message(Message::Connected(row.machine.clone()));
    assert!(area.page.row(&row).unwrap().actions().is_empty());
    let original = request.connection();
    let mut admissions = vessel_app_view::connection::ConnectionEpochs::default();
    let _old = admissions
        .admit(original.machine().clone(), original.node())
        .unwrap();
    let fresh = admissions
        .admit(original.machine().clone(), original.node())
        .unwrap();
    let _changed = area.page.admitted(fresh);
    assert!(!area.page.row(&row).unwrap().actions().is_empty());
    // A delayed error from the old connection must not undo that reset.
    area.message(Message::Replied(Answer {
        request: pause,
        result: Err(ScheduleError::Offline),
    }));
    assert!(!area.page.row(&row).unwrap().actions().is_empty());
}

#[test]
fn only_an_accepted_list_can_replace_the_viewer_calendar() {
    use vessel_app_link::schedules::{ListValue, ViewerClockError};
    use vessel_app_view::schedules::clock::Clock;
    fn value(zone: &str) -> Value {
        Value::List(Box::new(ListValue {
            schedules: vec![],
            scopes: Err(ScheduleError::Offline),
            clock: Ok(Clock::known(zone.parse().unwrap()).unwrap()),
            projects: Ok(vec![]),
            worktrees: Ok(vec![]),
            read_at: chrono::DateTime::UNIX_EPOCH,
        }))
    }
    let (mut area, old) = initial().unwrap();
    area.page.refresh();
    let current = area.page.due(chrono::DateTime::UNIX_EPOCH).remove(0);
    area.answered(Answer {
        request: old,
        result: Ok(value("Asia/Seoul")),
    });
    assert!(matches!(area.clock, Err(ViewerClockError::Unavailable)));
    area.answered(Answer {
        request: current.clone(),
        result: Ok(value("America/New_York")),
    });
    assert_eq!(
        area.clock.as_ref().unwrap().name().unwrap().as_str(),
        "America/New_York"
    );
    area.answered(Answer {
        request: current,
        result: Ok(value("Asia/Seoul")),
    });
    assert_eq!(
        area.clock.as_ref().unwrap().name().unwrap().as_str(),
        "America/New_York"
    );
}

#[test]
fn a_retired_physical_context_cannot_install_its_calendar_after_rebinding() {
    use vessel_app_link::schedules::{ListValue, ViewerClockError};
    use vessel_app_view::schedules::clock::Clock;
    let (mut area, old) = initial().unwrap();
    let mut admissions = vessel_app_view::connection::ConnectionEpochs::default();
    let previous = old.connection();
    let _first = admissions
        .admit(previous.machine().clone(), previous.node())
        .unwrap();
    let next = admissions
        .admit(previous.machine().clone(), previous.node())
        .unwrap();
    assert!(area.page.admitted(next));
    area.answered(Answer {
        request: old,
        result: Ok(Value::List(Box::new(ListValue {
            schedules: vec![],
            scopes: Ok(vec![
                vessel_proto::Scope::Read,
                vessel_proto::Scope::Operate,
            ]),
            clock: Ok(Clock::known("Asia/Seoul".parse().unwrap()).unwrap()),
            projects: Ok(vec![]),
            worktrees: Ok(vec![]),
            read_at: chrono::DateTime::UNIX_EPOCH,
        }))),
    });
    assert!(matches!(area.clock, Err(ViewerClockError::Unavailable)));
    assert_eq!(area.page.body(), Body::Loading);
    assert_eq!(area.page.due(chrono::DateTime::UNIX_EPOCH).len(), 1);
}

#[test]
fn a_form_submission_refused_by_the_local_queue_settles_its_original_mailbox() {
    use vessel_app_view::schedules::editor::io;
    for (why, failure) in [
        (Rejected::Full, Failure::Busy),
        (Rejected::Closed, Failure::Offline),
    ] {
        let (mut data, original) = initial().unwrap();
        let request = io::Request::on(
            original.connection().clone(),
            original.ticket,
            io::Operation::Targets,
        );
        data.editor.begin(request.clone());
        let returned = Unsent::new(why, Command::Schedules(request.clone().into()));
        let crate::unsent::Settle::Answer(event) =
            crate::unsent::settle(returned, "unused private diagnostic")
        else {
            panic!("not a reply")
        };
        let vessel_app_link::Event::Schedules(message) = *event else {
            panic!("not schedules")
        };
        assert!(data.message(message));
        assert_eq!(data.editor.request(), Some(&request));
        assert_eq!(data.editor.failure(), Some(&failure));
        assert!(data.editor.value().is_none());
    }
}

#[test]
fn an_unsent_history_lookup_settles_only_the_original_read_family() {
    for why in [Rejected::Full, Rejected::Closed] {
        let (mut data, list) = initial().unwrap();
        let info: vessel_proto::automation::ScheduleInfo = serde_json::from_str(include_str!(
            "../../../../view/tests/fixtures/schedules/info.json"
        ))
        .unwrap();
        let key = vessel_app_view::schedules::page::RowKey {
            machine: list.machine().clone(),
            schedule: info.id,
        };
        data.page.listed(
            &list,
            Ok(vessel_app_view::schedules::page::Snapshot {
                schedules: vec![info],
                scopes: Ok(vec![vessel_proto::Scope::Read]),
                read_at: chrono::DateTime::UNIX_EPOCH,
                projects: Ok(vec![]),
                worktrees: Ok(vec![]),
            }),
        );
        let request = data.history.open(&data.page.row(&key).unwrap()).unwrap();
        let returned = Unsent::new(why, Command::Schedules(request.clone().into()));
        let crate::unsent::Settle::Answer(event) =
            crate::unsent::settle(returned, "unused diagnostic")
        else {
            panic!("no answer")
        };
        let vessel_app_link::Event::Schedules(Message::HistoryReplied(answer)) = *event else {
            panic!("wrong family")
        };
        assert_eq!(answer.request, request);
        assert!(data.message(Message::HistoryReplied(answer)));
        assert_eq!(data.history.body(), Some(Body::Failed));
        assert!(data.history.runs().is_empty());
        assert_eq!(
            data.history.failure(),
            Some(&match why {
                Rejected::Full => Failure::Busy,
                Rejected::Closed => Failure::Offline,
            })
        );
    }
}
