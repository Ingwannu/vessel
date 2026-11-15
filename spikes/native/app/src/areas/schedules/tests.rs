//! Unsent/local replies traverse the same correlation path as a real answer.
use super::*;
use vessel_app_link::{Command, Rejected, Unsent, schedules::ScheduleError};
use vessel_app_view::schedules::{load::MachineKey, page::Body};
fn initial() -> Option<(Area, vessel_app_view::schedules::page::Request)> {
    let mut area = Area::default();
    area.page.machines(&[MachineKey::of("laptop")?]);
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
        let returned = Unsent::new(why, Command::Schedules(request));
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
        machine: request.machine.clone(),
        schedule: info.id,
    };
    area.page.listed(
        &request,
        Ok(Snapshot {
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
    assert!(!area.page.row(&row).unwrap().actions().is_empty());
    // A delayed error from the old connection must not undo that reset.
    area.message(Message::Replied(Answer {
        request: pause,
        result: Err(ScheduleError::Offline),
    }));
    assert!(!area.page.row(&row).unwrap().actions().is_empty());
}
