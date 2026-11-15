//! Not the page's: an agent's plan updates as Paseo's task list says them
//! (`types/stream.ts`, `deriveTaskActivities` and the reducer around it),
//! on a sequence like Claude Code's: a list created, a step started, one
//! done as the next starts, a step added.

use serde_json::{Value, json};
use vessel_app_view::timeline::{EntryKind, PlanActivity, Timeline};
use vessel_proto::PlanStatus;

use crate::support::{Log, kinds};

fn plan(steps: &[(&str, &str)]) -> Value {
    json!({
        "event": "plan",
        "entries": steps
            .iter()
            .map(|(content, status)| json!({ "content": content, "priority": "medium", "status": status }))
            .collect::<Vec<_>>()
    })
}

fn activities(timeline: &Timeline) -> Vec<PlanActivity> {
    timeline
        .entries()
        .iter()
        .filter_map(|entry| {
            if let EntryKind::Plan { activity, .. } = &entry.kind {
                Some(activity.clone())
            } else {
                None
            }
        })
        .collect()
}

const FOUR: [&str; 4] = [
    "Read the fold",
    "긴 대화에서 스크롤이 끊기지 않는지 확인",
    "Write the regression test",
    "Run the whole gate",
];

fn with(statuses: [&'static str; 4]) -> Vec<(&'static str, &'static str)> {
    FOUR.iter().copied().zip(statuses).collect()
}

#[test]
fn a_list_created_then_started_then_one_done_as_the_next_starts() {
    let mut log = Log::default();
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&[
        log.agent(
            &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "go" }] }),
        ),
        log.agent(&plan(&with(["pending", "pending", "pending", "pending"]))),
    ]);
    assert_eq!(activities(&timeline), [PlanActivity::Created { count: 4 }]);
    // Sent again unchanged: no row, as in Paseo.
    let _change =
        timeline.apply(&[log.agent(&plan(&with(["pending", "pending", "pending", "pending"])))]);
    assert_eq!(activities(&timeline).len(), 1);
    let _change = timeline.apply(&[log.agent(&plan(&with([
        "in_progress",
        "pending",
        "pending",
        "pending",
    ])))]);
    // One update, two changes: two rows, each with a key of its own.
    let change = timeline.apply(&[log.agent(&plan(&with([
        "completed",
        "in_progress",
        "pending",
        "pending",
    ])))]);
    assert_eq!(
        activities(&timeline),
        [
            PlanActivity::Created { count: 4 },
            PlanActivity::Started(FOUR[0].to_owned()),
            PlanActivity::Completed(FOUR[0].to_owned()),
            PlanActivity::Started(FOUR[1].to_owned()),
        ]
    );
    assert_eq!(change.added_from, 3);
    let rows: Vec<_> = timeline.entries().iter().skip(3).collect();
    assert_ne!(rows[0].key, rows[1].key);
    for (at, row) in (3_usize..).zip(&rows) {
        assert_eq!(timeline.index_of(&row.key), Some(at));
    }
    // Each row keeps the plan as it stood then.
    let EntryKind::Plan { entries, .. } = &rows[1].kind else {
        panic!("a plan row");
    };
    assert_eq!(entries[1].status, PlanStatus::InProgress);
    assert_eq!(kinds(&timeline), ["prompt", "plan", "plan", "plan", "plan"]);
}

#[test]
fn a_step_added_to_a_list_just_created_is_part_of_the_creation() {
    let mut log = Log::default();
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&[
        log.agent(
            &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "go" }] }),
        ),
        log.agent(&plan(&with(["pending", "pending", "pending", "pending"]))),
    ]);
    let mut five = with(["pending", "pending", "pending", "pending"]);
    five.push(("Say what changed", "pending"));
    let change = timeline.apply(&[log.agent(&plan(&five))]);
    assert_eq!(activities(&timeline), [PlanActivity::Created { count: 5 }]);
    assert_eq!(change.changed, [1]);
}

#[test]
fn a_step_added_once_work_began_is_its_own_row() {
    let mut log = Log::default();
    let mut timeline = Timeline::default();
    let started = with(["in_progress", "pending", "pending", "pending"]);
    let _change = timeline.apply(&[
        log.agent(
            &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "go" }] }),
        ),
        log.agent(&plan(&with(["pending", "pending", "pending", "pending"]))),
        log.agent(&plan(&started)),
    ]);
    let mut five = started.clone();
    five.push(("Say what changed", "pending"));
    let _change = timeline.apply(&[log.agent(&plan(&five))]);
    assert_eq!(
        activities(&timeline).last(),
        Some(&PlanActivity::Added("Say what changed".to_owned()))
    );
}

#[test]
fn an_update_changing_nothing_read_refreshes_the_last_row() {
    let mut log = Log::default();
    let mut timeline = Timeline::default();
    let _change = timeline.apply(&[
        log.agent(
            &json!({ "event": "turn_started", "content": [{ "type": "text", "text": "go" }] }),
        ),
        log.agent(&plan(&with([
            "in_progress",
            "pending",
            "pending",
            "pending",
        ]))),
    ]);
    // A step going back to be done again later is not said (Paseo says
    // only steps started or completed); the row shows the plan as it is.
    let change =
        timeline.apply(&[log.agent(&plan(&with(["pending", "pending", "pending", "pending"])))]);
    assert_eq!(activities(&timeline).len(), 1);
    assert_eq!(change.changed, [1]);
    let EntryKind::Plan { entries, .. } = &timeline.entries()[1].kind else {
        panic!("a plan row");
    };
    assert_eq!(entries[0].status, PlanStatus::Pending);
}
