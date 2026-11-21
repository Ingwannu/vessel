//! Representative schedule data and regressions specific to the native list.
use super::*;
use vessel_proto::automation::ScheduleInfo;

pub(super) fn info() -> ScheduleInfo {
    serde_json::from_str(include_str!("../../tests/fixtures/schedules/info.json")).unwrap()
}

pub(super) fn visible(text: &str) -> String {
    text.chars()
        .filter(|ch| !matches!(ch, '\u{2068}' | '\u{2069}'))
        .collect()
}

#[test]
fn every_existing_session_target_uses_heartbeat_actions_regardless_of_owner() {
    use vessel_proto::{Creator, SessionId};

    let session: SessionId = "00000000-0000-4000-8000-000000000002".parse().unwrap();
    let base = info();
    assert!(!is_heartbeat(&base));
    assert!(actions(&base, &authority::Authority::Unknown).contains(&Action::RunNow));
    let catalog = crate::localization::Translator::get(crate::localization::ENGLISH).unwrap();
    for defined_by in [
        Creator::Owner,
        Creator::Device {
            id: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        },
        Creator::Agent {
            node: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
            session,
        },
        Creator::Agent {
            node: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
            session: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
        },
    ] {
        let mut schedule = base.clone();
        schedule.definition.target = ScheduleTarget::Session { session };
        schedule.defined_by = defined_by;
        let mut page = page::Page::default();
        page::tests::bound(&mut page, &[page::tests::key("laptop")]);
        let request = page.due().pop().unwrap();
        page.listed(&request, Ok(page::tests::snapshot(vec![schedule])));
        let rows = page.visible_rows();
        assert_eq!(rows[0].actions(), [Action::Delete]);
        assert_eq!(rows[0].menu_entries()[0].action, Action::Delete);
        assert_eq!(rows[0].menu_entries().len(), 1);
        let prepared = presentation::rows(
            &page,
            &crate::Fleet::default(),
            page::tests::now(),
            |_| None,
            catalog,
        );
        assert_eq!(prepared[0].menu_label.as_ref(), "Heartbeat actions");
        assert_eq!(prepared[0].menu[0].label.as_ref(), "Delete heartbeat");
    }
}
