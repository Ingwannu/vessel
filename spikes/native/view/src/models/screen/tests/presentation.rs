//! `ProviderModalBody` and `provider-diagnostic-models.test.ts` state precedence.
use super::*;
use crate::agent_status::pages::MachineKey;
use crate::models::page::Snapshot;

fn selected() -> State<Problem> {
    let mut state = State::default();
    state.select(Some(MachineKey::from_book("fixture")));
    state
}
fn snapshot(routes: Listed<ModelRoute>) -> Snapshot {
    Snapshot {
        node: vessel_proto::NodeId::random(),
        routes,
        profiles: Listed::Available(vec![]),
        agents: vec![],
    }
}
fn rows() -> Vec<ModelRoute> {
    serde_json::from_str(r#"[{"name":"grok-build","provider":"grok","api":"openai_responses","model":"grok-build"}]"#).unwrap()
}
fn show(state: &State<Problem>) -> Presentation<'_> {
    presentation(state, None, "", |problem| *problem)
}
#[test]
fn initial_loading_failure_empty_and_missing_machine_are_distinct() {
    let mut state = State::<Problem>::default();
    assert!(matches!(show(&state).content, Content::NoMachine));
    assert_eq!(show(&state).refresh, Refresh::Unavailable);
    state = selected();
    let ticket = state.begin().unwrap();
    assert!(matches!(show(&state).content, Content::Loading));
    assert_eq!(show(&state).refresh, Refresh::Pending);
    assert!(state.received(
        &MachineKey::from_book("fixture"),
        ticket,
        Err(Problem::Denied)
    ));
    assert!(matches!(
        show(&state).content,
        Content::Failed(Issue::Read(Problem::Denied))
    ));
    assert_eq!(show(&state).retained_issue, None);
    assert_eq!(show(&state).refresh, Refresh::Ready);
    let ticket = state.begin().unwrap();
    assert!(state.received(
        &MachineKey::from_book("fixture"),
        ticket,
        Ok(snapshot(Listed::Available(vec![])))
    ));
    assert!(matches!(show(&state).content, Content::Empty));
}
#[test]
fn refresh_and_failure_retain_rows_but_switching_machine_clears_them() {
    // Reference: Grok Build cached on local, then Grok refreshing. Switching
    // server must not reuse it, matching the reference's fourth fixture.
    let mut state = selected();
    let machine = MachineKey::from_book("fixture");
    let ticket = state.begin().unwrap();
    assert!(state.received(&machine, ticket, Ok(snapshot(Listed::Available(rows())))));
    let refresh = state.begin().unwrap();
    let Content::Rows { first, rest, .. } = show(&state).content else {
        panic!("cached rows hidden")
    };
    assert_eq!(
        std::iter::once(first)
            .chain(rest)
            .map(|row| row.model.as_str())
            .collect::<Vec<_>>(),
        ["grok-build"]
    );
    assert_eq!(show(&state).refresh, Refresh::Pending);
    assert!(state.received(&machine, refresh, Err(Problem::Timeout)));
    assert!(matches!(show(&state).content, Content::Rows { .. }));
    assert_eq!(
        show(&state).retained_issue,
        Some(Issue::Read(Problem::Timeout))
    );
    state.select(Some(MachineKey::from_book("other-server")));
    let _reading = state.begin().unwrap();
    assert!(matches!(show(&state).content, Content::Loading));
    assert_eq!(show(&state).retained_issue, None);
}
#[test]
fn unsupported_is_not_empty_and_search_is_not_a_missing_machine() {
    let mut state = selected();
    let machine = MachineKey::from_book("fixture");
    let ticket = state.begin().unwrap();
    assert!(state.received(&machine, ticket, Ok(snapshot(Listed::OlderNode))));
    assert!(matches!(show(&state).content, Content::Unsupported));
    let ticket = state.begin().unwrap();
    assert!(state.received(&machine, ticket, Ok(snapshot(Listed::Available(rows())))));
    assert!(matches!(
        presentation(&state, None, "absent", |p| *p).content,
        Content::NoMatches
    ));
    let p = presentation(&state, Some(StartProblem::Exhausted), "", |p| *p);
    assert_eq!(p.refresh, Refresh::Unavailable);
    assert!(matches!(p.content, Content::Rows { .. }));
    assert_eq!(
        p.retained_issue,
        Some(Issue::Start(StartProblem::Exhausted))
    );
}

#[test]
fn known_empty_refresh_and_failed_empty_follow_provider_body_precedence() {
    let mut state = selected();
    let machine = MachineKey::from_book("fixture");
    let ticket = state.begin().unwrap();
    assert!(state.received(&machine, ticket, Ok(snapshot(Listed::Available(vec![])))));
    assert!(matches!(
        presentation(&state, None, "query", |p| *p).content,
        Content::NoMatches
    ));
    let refresh = state.begin().unwrap();
    assert!(matches!(show(&state).content, Content::Loading));
    assert!(state.received(&machine, refresh, Err(Problem::Failed)));
    assert!(matches!(
        presentation(&state, None, "query", |p| *p).content,
        Content::Failed(Issue::Read(Problem::Failed))
    ));
    assert_eq!(show(&state).retained_issue, None);
    assert!(state.page().snapshot().is_some());
}

#[test]
fn a_repeated_gesture_remains_pending_without_an_error_or_new_ticket() {
    let mut state = selected();
    let machine = MachineKey::from_book("fixture");
    let ticket = state.begin().unwrap();
    assert_eq!(state.begin(), Err(StartProblem::Busy));
    let initial = presentation(&state, Some(StartProblem::Busy), "", |p| *p);
    assert!(matches!(initial.content, Content::Loading));
    assert_eq!(initial.retained_issue, None);
    assert_eq!(state.page().pending(), Some(ticket));
    assert!(state.received(&machine, ticket, Ok(snapshot(Listed::Available(rows())))));
    let refresh = state.begin().unwrap();
    assert_eq!(state.begin(), Err(StartProblem::Busy));
    let cached = presentation(&state, Some(StartProblem::Busy), "", |p| *p);
    assert!(matches!(cached.content, Content::Rows { .. }));
    assert_eq!(cached.retained_issue, None);
    assert_eq!(cached.refresh, Refresh::Pending);
    assert_eq!(state.page().pending(), Some(refresh));
}
