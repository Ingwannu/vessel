//! Prepared identity, feedback and operation words remain renderer independent.
use super::super::page::{Ask, Snapshot};
use super::super::tests::visible;
use super::*;
use crate::localization::{ENGLISH, Locale, Translator};
fn setup() -> Page {
    let mut page = Page::default();
    let key = super::super::load::MachineKey::of("laptop").unwrap();
    crate::schedules::page::tests::bound(&mut page, &[key]);
    let request = page.due(DateTime::UNIX_EPOCH).pop().unwrap();
    page.listed(
        &request,
        Ok(Snapshot {
            scopes: Err(crate::schedules::load::Failure::Older),
            schedules: vec![
                serde_json::from_str(include_str!("../../../tests/fixtures/schedules/info.json"))
                    .unwrap(),
            ],
            read_at: DateTime::UNIX_EPOCH,
            projects: Ok(vec![]),
            worktrees: Ok(vec![]),
        }),
    );
    page
}
#[test]
fn exact_target_metadata_and_menu_are_prepared_for_each_language() {
    let page = setup();
    let now = "2026-11-14T08:30:00Z".parse().unwrap();
    let clock = Clock::known("Asia/Seoul".parse().unwrap()).unwrap();
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let english = rows(&page, &Fleet::default(), now, &clock, en);
    let korean = rows(&page, &Fleet::default(), now, &clock, ko);
    assert_eq!(english[0].title.as_ref(), "빌드와 테스트 상태 확인");
    assert_eq!(
        english[0].target,
        TargetLine::Text("Chat details unavailable".into())
    );
    assert_eq!(
        korean[0].target,
        TargetLine::Text("대화 정보를 불러올 수 없음".into())
    );
    assert_eq!(english[0].agent, None);
    assert_eq!(
        english[0].metadata.as_ref(),
        "\u{2068}Every 1 minute 30 seconds\u{2069} · \u{2068}Created \u{2068}\u{2068}1\u{2069}d ago\u{2069}\u{2069} · \u{2068}Never run\u{2069} · \u{2068}Next run \u{2068}in \u{2068}30\u{2069}m\u{2069} · Your time: \u{2068}Sat, Nov 14, 2026 at 6:00 PM Asia/Seoul\u{2069}\u{2069}"
    );
    assert_eq!(
        visible(&english[0].metadata),
        "Every 1 minute 30 seconds · Created 1d ago · Never run · Next run in 30m · Your time: Sat, Nov 14, 2026 at 6:00 PM Asia/Seoul"
    );
    assert_eq!(
        english[0]
            .menu
            .iter()
            .map(|entry| entry.label.as_ref())
            .collect::<Vec<_>>(),
        [
            "Edit recurring prompt",
            "Pause recurring prompt",
            "Run now",
            "Delete recurring prompt"
        ]
    );
    assert_eq!(english, rows(&page, &Fleet::default(), now, &clock, en));
}
#[test]
fn pending_identity_and_forbidden_have_one_signal_and_disable_edit() {
    let mut page = setup();
    let en = Translator::get(ENGLISH).unwrap();
    let now = DateTime::UNIX_EPOCH;
    let row = page.visible_rows().remove(0).key;
    let request = page.act(&row, Action::Pause).unwrap();
    assert!(matches!(request.ask, Ask::Pause(_)));
    let prepared = rows(&page, &Fleet::default(), now, &Clock::unknown(), en);
    assert_eq!(prepared[0].menu[0].action, Action::Edit);
    assert_eq!(prepared[0].menu[0].state, EntryState::Disabled);
    assert_eq!(
        prepared[0].menu[1],
        MenuEntry {
            action: Action::Pause,
            state: EntryState::Pending,
            label: "Pause recurring prompt".into(),
            pending: Some("Pausing...".into())
        }
    );
    page.written(
        &request,
        Err(super::super::load::Failure::Node(
            vessel_proto::WireError::new(vessel_proto::ErrorCode::Forbidden, "not for display"),
        )),
    );
    let prepared = rows(&page, &Fleet::default(), now, &Clock::unknown(), en);
    assert_eq!(
        prepared[0].notice,
        Some(Notice {
            label: "This device can only view this machine's schedules.".into(),
            variant: alert::Variant::Info
        })
    );
    assert!(prepared[0].menu.is_empty());
}

#[test]
fn filter_dots_report_connection_evidence_without_inventing_connecting_state() {
    use super::super::{load::MachineKey, page::MachineFilter};
    use crate::{Machine, Reach, badge::Variant};
    let mut fleet = Fleet::default();
    let key = MachineKey::of("laptop").unwrap();
    assert_eq!(
        machine_leading(&MachineFilter::All, &fleet),
        MachineLeading::All
    );
    assert_eq!(
        machine_leading(&MachineFilter::One(key.clone()), &fleet),
        MachineLeading::Status(Variant::Muted)
    );
    fleet
        .machines
        .push(Machine::unreachable("laptop".into(), "not display text"));
    assert_eq!(
        machine_leading(&MachineFilter::One(key.clone()), &fleet),
        MachineLeading::Status(Variant::Error)
    );
    fleet.machines[0].reach = Reach::Online;
    assert_eq!(
        machine_leading(&MachineFilter::One(key), &fleet),
        MachineLeading::Status(Variant::Success)
    );
}
#[test]
fn project_relative_paths_stay_separate_from_the_body_face_identity() {
    let name = "Service".parse().unwrap();
    let dir = "한글/subfolder".parse().unwrap();
    assert_eq!(
        target_line(
            &Line::Project {
                name: &name,
                dir: Some(&dir)
            },
            Translator::get(ENGLISH).unwrap()
        ),
        TargetLine::Project {
            name: "Service".into(),
            dir: "한글/subfolder".into()
        }
    );
}

#[test]
fn old_paused_rows_and_empty_filters_unsubscribe_but_active_countdowns_keep_a_clock() {
    use super::super::time::RefreshTier;
    use vessel_proto::automation::ScheduleState;
    let mut page = setup();
    let now = "2026-11-30T00:00:00Z".parse().unwrap();
    assert_eq!(refresh_tier(&page, now), Some(RefreshTier::Minute));
    let key = page.visible_rows()[0].key.clone();
    let request = page.act(&key, Action::Pause).unwrap();
    let mut info = page.row(&key).unwrap().schedule.clone();
    info.state = ScheduleState::Paused { since: now };
    page.written(
        &request,
        Ok(super::super::page::WriteValue::Schedule(Box::new(info))),
    );
    assert_eq!(refresh_tier(&page, now), None);
    page.bucket = super::super::Bucket::Ended;
    assert_eq!(refresh_tier(&page, now), None);
}

#[test]
fn history_is_a_read_only_entry_without_changing_write_authority() {
    let mut page = setup();
    let en = Translator::get(ENGLISH).unwrap();
    let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
    let row = page.visible_rows().remove(0).key;
    let request = page.act(&row, Action::Pause).unwrap();
    page.written(
        &request,
        Err(super::super::load::Failure::Node(
            vessel_proto::WireError::new(vessel_proto::ErrorCode::Forbidden, "private diagnostic"),
        )),
    );
    for (catalog, label) in [(en, "View runs"), (ko, "실행 기록 보기")] {
        let prepared = rows(
            &page,
            &Fleet::default(),
            DateTime::UNIX_EPOCH,
            &Clock::unknown(),
            catalog,
        );
        assert!(prepared[0].menu.is_empty());
        assert_eq!(prepared[0].history.as_deref(), Some(label));
        assert!(prepared[0].has_menu());
    }
    page.connected(&row.machine);
    let offline = rows(
        &page,
        &Fleet::default(),
        DateTime::UNIX_EPOCH,
        &Clock::unknown(),
        en,
    );
    assert_eq!(offline[0].history, None);
    assert!(!offline[0].has_menu());
}
