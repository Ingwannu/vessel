//! Cases from `schedules/aggregated-schedules.test.ts` (all four) and
//! `screens/schedules-screen-state.test.ts` (failed loading goes to retry).
//! The fixture is independent of a renderer's enum names; no reference code.
use super::tests::bound;
use super::tests::{key, now, snapshot};
use super::*;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Host {
    Connecting,
    Empty,
    Content,
    Failed,
}
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Expected {
    Loading,
    Empty,
    Content,
    Failed,
}
impl From<Expected> for Body {
    fn from(value: Expected) -> Self {
        match value {
            Expected::Loading => Self::Loading,
            Expected::Empty => Self::Empty,
            Expected::Content => Self::Content,
            Expected::Failed => Self::Failed,
        }
    }
}
#[derive(Deserialize)]
struct Case {
    id: String,
    hosts: Vec<Host>,
    body: Expected,
}
#[test]
fn reference_aggregate_inputs_keep_the_same_body_decisions() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/schedules/aggregate.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 5);
    for case in cases {
        let mut page = Page::default();
        bound(&mut page, &[key("host-a"), key("host-b")]);
        let requests = page.due(now());
        for (request, host) in requests.iter().zip(case.hosts) {
            match host {
                Host::Connecting => {}
                Host::Empty => {
                    page.listed(request, Ok(snapshot(Vec::new())));
                }
                Host::Content => {
                    page.listed(request, Ok(snapshot(vec![super::super::tests::info()])));
                }
                Host::Failed => {
                    page.listed(request, Err(Failure::Offline));
                }
            }
        }
        assert_eq!(page.body(), case.body.into(), "{}", case.id);
    }
}
#[test]
fn reference_single_host_and_empty_filters_stay_hidden_until_needed() {
    let mut page = Page::default();
    assert!(!page.multiple_machines());
    assert!(!page.filters_visible());
    bound(&mut page, &[key("host-a")]);
    assert!(!page.multiple_machines());
    let read = page.due(now()).remove(0);
    page.listed(&read, Ok(snapshot(Vec::new())));
    assert!(!page.filters_visible());
    bound(&mut page, &[key("host-a"), key("host-b")]);
    assert!(page.multiple_machines());
    let read = page.due(now()).remove(0);
    page.listed(&read, Ok(snapshot(vec![super::super::tests::info()])));
    assert!(page.filters_visible());
    page.machine = MachineFilter::One(key("host-a"));
    assert!(page.filters_visible());
}
