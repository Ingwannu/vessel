//! Node contract shapes and renderer-independent transition expectations.
use super::*;
use crate::schedules::Action;
use crate::schedules::tests::info;
use vessel_proto::automation::{ScheduleInfo, ScheduleState};
use vessel_proto::{ErrorCode, WireError};
pub(crate) fn now() -> DateTime<Utc> {
    "2026-11-13T00:00:00Z".parse().unwrap()
}
pub(super) fn key(name: &str) -> MachineKey {
    MachineKey::of(name).unwrap()
}
pub(crate) fn snapshot(schedules: Vec<ScheduleInfo>) -> Snapshot {
    Snapshot {
        scopes: Err(Failure::Older),
        schedules,
        read_at: now(),
        worktrees: Ok(Vec::new()),
        projects: Ok(Vec::new()),
    }
}
pub(crate) fn context(key: &MachineKey) -> crate::connection::ConnectionContext {
    crate::connection::ConnectionEpochs::default()
        .admit(
            crate::connection::MachineKey::from_book(key.as_str()),
            "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        )
        .unwrap()
}
pub(crate) fn rebound(key: &MachineKey) -> crate::connection::ConnectionContext {
    let mut epochs = crate::connection::ConnectionEpochs::default();
    let original = context(key);
    let _old = epochs
        .admit(original.machine().clone(), original.node())
        .unwrap();
    epochs
        .admit(original.machine().clone(), original.node())
        .unwrap()
}
pub(crate) fn bound(page: &mut Page, keys: &[MachineKey]) {
    page.machines(keys);
    for key in keys {
        let _changed = page.admitted(context(key));
    }
}
pub(crate) fn loaded() -> (Page, RowKey) {
    let mut page = Page::default();
    bound(&mut page, &[key("laptop")]);
    let request = page.due(now()).remove(0);
    let row = RowKey {
        machine: request.machine().clone(),
        schedule: info().id,
    };
    page.listed(&request, Ok(snapshot(vec![info()])));
    (page, row)
}
#[test]
fn partial_failures_do_not_replace_rows() {
    let mut page = Page::default();
    assert_eq!(page.body(), Body::Loading);
    bound(&mut page, &[]);
    assert_eq!(page.body(), Body::Empty);
    bound(&mut page, &[key("a"), key("b")]);
    let requests = page.due(now());
    assert_eq!(requests.len(), 2);
    page.listed(&requests[0], Err(Failure::Offline));
    assert_eq!(page.body(), Body::Failed);
    page.listed(&requests[1], Err(Failure::Older));
    assert_eq!(page.body(), Body::Failed);
    assert_eq!(page.problems().len(), 2);
    page.refresh();
    let requests = page.due(now());
    page.listed(&requests[0], Ok(snapshot(vec![info()])));
    page.listed(&requests[1], Err(Failure::Offline));
    assert_eq!(page.body(), Body::Content);
    assert_eq!(page.problems().len(), 1);
    page.bucket = Bucket::Ended;
    assert_eq!(page.body(), Body::Empty);
    page.machine = MachineFilter::One(key("b"));
    assert_eq!(page.body(), Body::Failed);
    bound(&mut page, &[key("a")]);
    assert_eq!(page.machine, MachineFilter::All);
    assert_eq!(page.body(), Body::Empty);
}
#[test]
fn refresh_is_bounded_retains_rows_and_does_not_retry_failures() {
    let mut page = Page::default();
    bound(&mut page, &["a", "b", "c", "d", "e"].map(key));
    let requests = page.due(now());
    assert_eq!(requests.len(), 4);
    assert!(page.due(now()).is_empty());
    page.listed(&requests[0], Ok(snapshot(vec![info()])));
    assert_eq!(page.due(now()).len(), 1);
    page.listed(&requests[1], Err(Failure::Offline));
    assert!(page.due(now()).is_empty());
    let (mut page, row) = loaded();
    assert!(page.due(now() + chrono::TimeDelta::seconds(4)).is_empty());
    let refresh = page.due(now() + chrono::TimeDelta::seconds(5)).remove(0);
    page.listed(&refresh, Err(Failure::Offline));
    assert_eq!(page.body(), Body::Content);
    assert!(page.row(&row).is_some());
    assert_eq!(page.problems().len(), 1);
    assert!(page.due(now() + chrono::TimeDelta::hours(1)).is_empty());
}
#[test]
fn authoritative_mutation_invalidates_old_reads() {
    let (mut page, row) = loaded();
    let refresh = page.due(now() + chrono::TimeDelta::seconds(5)).remove(0);
    let pause = page.act(&row, Action::Pause).unwrap();
    assert!(matches!(
        page.row(&row).unwrap().schedule.state,
        ScheduleState::Active { .. }
    ));
    assert!(page.row(&row).unwrap().actions().is_empty());
    assert!(page.act(&row, Action::Pause).is_none());
    let mut paused = info();
    paused.state = ScheduleState::Paused { since: now() };
    page.written(&pause, Ok(WriteValue::Schedule(Box::new(paused))));
    page.listed(&refresh, Ok(snapshot(vec![info()])));
    assert!(matches!(
        page.row(&row).unwrap().schedule.state,
        ScheduleState::Paused { .. }
    ));
    assert_eq!(
        page.row(&row).unwrap().actions(),
        [Action::Resume, Action::RunNow, Action::Delete]
    );
    let delete = page.act(&row, Action::Delete).unwrap();
    page.written(&delete, Ok(WriteValue::Deleted));
    assert!(page.row(&row).is_none());
    assert_eq!(page.body(), Body::Empty);
}
#[test]
fn forbidden_is_row_local_evidence_reset_by_a_new_connection() {
    let (mut page, row) = loaded();
    let pause = page.act(&row, Action::Pause).unwrap();
    let mut other = info();
    other.id = "00000000-0000-4000-8000-000000000009".parse().unwrap();
    page.written(
        &pause,
        Err(Failure::Node(WireError::new(
            ErrorCode::Forbidden,
            "read only",
        ))),
    );
    assert!(page.row(&row).unwrap().actions().is_empty());
    page.refresh();
    let refresh = page.due(now()).remove(0);
    page.listed(&refresh, Ok(snapshot(vec![info(), other.clone()])));
    assert!(page.row(&row).unwrap().actions().is_empty());
    assert!(
        !page
            .row(&RowKey {
                schedule: other.id,
                ..row.clone()
            })
            .unwrap()
            .actions()
            .is_empty()
    );
    page.connected(&row.machine);
    assert!(page.row(&row).unwrap().actions().is_empty());
    let _changed = page.admitted(rebound(&row.machine));
    assert!(!page.row(&row).unwrap().actions().is_empty());
    assert_eq!(page.row(&row).unwrap().mutation.failure(), None);
    page.written(
        &pause,
        Err(Failure::Node(WireError::new(ErrorCode::Forbidden, "late"))),
    );
    assert!(!page.row(&row).unwrap().actions().is_empty());
}
#[test]
fn stale_tickets_filters_and_unknown_values_keep_their_identity() {
    let (mut page, row) = loaded();
    page.refresh();
    let stale = page.due(now()).remove(0);
    page.refresh();
    let current = page.due(now()).remove(0);
    page.listed(&stale, Err(Failure::Offline));
    assert!(page.problems().is_empty());
    let mut unknown = info();
    unknown.state = ScheduleState::Unknown;
    page.listed(&current, Ok(snapshot(vec![unknown])));
    assert_eq!(
        page.buckets(),
        [Bucket::Active, Bucket::Ended, Bucket::Unknown]
    );
    assert_eq!(page.body(), Body::Empty);
    page.bucket = Bucket::Unknown;
    assert_eq!(page.body(), Body::Content);
    assert_eq!(page.row(&row).unwrap().actions(), [Action::Delete]);
    bound(&mut page, &[]);
    page.listed(&current, Ok(snapshot(vec![info()])));
    assert_eq!(page.body(), Body::Empty);
    assert!(page.keys().next().is_none());
}

#[test]
fn read_only_feedback_is_a_single_line_in_both_languages() {
    let (mut page, row) = loaded();
    let pause = page.act(&row, Action::Pause).unwrap();
    page.written(
        &pause,
        Err(Failure::Node(WireError::new(
            ErrorCode::Forbidden,
            "diagnostic",
        ))),
    );
    assert_eq!(page.row(&row).unwrap().feedback(), Some(Feedback::ReadOnly));
    assert_eq!(
        crate::Lang::En.schedules().read_only,
        "This device can only view this machine's schedules."
    );
    assert_eq!(
        crate::Lang::Ko.schedules().read_only,
        "이 머신의 예약은 이 기기에서 조회만 할 수 있습니다."
    );
}

#[test]
fn a_full_pool_gives_action_feedback_instead_of_a_dead_click() {
    let (mut page, row) = loaded();
    bound(&mut page, &["laptop", "a", "b", "c", "d"].map(key));
    assert_eq!(page.due(now()).len(), 4);
    assert!(page.act(&row, Action::Pause).is_none());
    assert_eq!(
        page.row(&row).unwrap().feedback(),
        Some(Feedback::Failed(&Failure::Busy))
    );
}

#[test]
fn run_now_uses_the_node_outcome_without_inventing_a_successful_run_count() {
    let (mut page, row) = loaded();
    let request = page.act(&row, Action::RunNow).unwrap();
    let run=serde_json::from_str(r#"{"number":1,"due_at":"2026-11-13T00:00:00Z","at":"2026-11-13T00:00:00Z","trigger":{"trigger":"due"},"outcome":{"outcome":"interrupted"}}"#).unwrap();
    page.written(&request, Ok(WriteValue::Run(run)));
    let row = page.row(&row).unwrap();
    assert_eq!(row.schedule.runs, 0);
    assert_eq!(
        crate::schedules::history::outcome(&row.schedule.last_run.as_ref().unwrap().outcome),
        crate::schedules::history::Outcome::Interrupted
    );
}

#[test]
fn polling_a_paused_state_does_not_reverse_the_pending_menu_caption() {
    let (mut page, key) = loaded();
    let pause = page.act(&key, Action::Pause).unwrap();
    let read = page.due(now() + chrono::TimeDelta::seconds(5)).remove(0);
    let mut paused = info();
    paused.state = ScheduleState::Paused { since: now() };
    page.listed(&read, Ok(snapshot(vec![paused.clone()])));
    let row = page.row(&key).unwrap();
    assert_eq!(
        row.menu_entries(),
        [
            MenuEntry {
                action: Action::Pause,
                state: EntryState::Pending
            },
            MenuEntry {
                action: Action::RunNow,
                state: EntryState::Disabled
            },
            MenuEntry {
                action: Action::Delete,
                state: EntryState::Disabled
            },
        ]
    );
    assert!(page.act(&key, Action::Resume).is_none());
    page.written(&pause, Ok(WriteValue::Schedule(Box::new(paused))));
    assert_eq!(
        page.row(&key).unwrap().menu_entries(),
        [
            MenuEntry {
                action: Action::Resume,
                state: EntryState::Enabled
            },
            MenuEntry {
                action: Action::RunNow,
                state: EntryState::Enabled
            },
            MenuEntry {
                action: Action::Delete,
                state: EntryState::Enabled
            },
        ]
    );
}
