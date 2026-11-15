//! Real framed clients prove setup never redirects work to a replaced alias.

use super::fixtures::{Fixture, next};
use crate::onboarding::{Answer, Ask};
use crate::{Event, LinkError};
use vessel_app_view::devices::MachineKey;
use vessel_app_view::onboarding::{
    Layout, Model, State, Step,
    scope::{Problem, Target},
};

#[tokio::test]
async fn replacement_is_refused_and_a_stale_release_cannot_remove_the_new_visit() {
    let mut fixture = Fixture::new().await;
    let original = fixture.state.client("laptop").unwrap().node().id;
    let target = Target::authenticated(MachineKey::from_book("laptop"), original);
    let mut model = Model::default();
    model.reopen(target.clone(), Layout::Wide).unwrap();
    let first = match model.state() {
        State::Open(open) => open.visit(),
        _ => panic!("setup did not open"),
    };
    fixture
        .state
        .onboarding(Ask::Bind {
            target: target.clone(),
            visit: first,
        })
        .await;
    assert!(matches!(
        next(&mut fixture.events).await,
        Event::Onboarding(Answer::Bound { result: Ok(()), .. })
    ));
    let replacement = fixture.state.clients.get("other").unwrap().clone();
    let _old = fixture
        .state
        .clients
        .insert("laptop".to_owned(), replacement);
    assert!(matches!(
        fixture.state.client("laptop"),
        Err(LinkError::Onboarding(Problem::Changed))
    ));
    // The unrelated host is still usable: this fence is scoped to the flow.
    assert!(fixture.state.client("other").is_ok());
    model.go_to(Step::Providers).unwrap();
    let second = match model.state() {
        State::Open(open) => open.visit(),
        _ => panic!("setup closed"),
    };
    fixture
        .state
        .onboarding(Ask::Bind {
            target,
            visit: second,
        })
        .await;
    assert!(matches!(
        next(&mut fixture.events).await,
        Event::Onboarding(Answer::Bound {
            result: Err(Problem::Changed),
            ..
        })
    ));
    fixture.state.onboarding(Ask::Release(first)).await;
    let _released = next(&mut fixture.events).await;
    assert!(matches!(
        fixture.state.client("laptop"),
        Err(LinkError::Onboarding(Problem::Changed))
    ));
    fixture
        .state
        .onboarding_agents(
            crate::agents::MachineKey::from_book("laptop"),
            vessel_proto::RequestId::new(1),
            crate::agents::Ask::List,
        )
        .await;
    assert!(
        matches!(next(&mut fixture.events).await, Event::Onboarding(Answer::Bound {visit, result: Err(Problem::Changed), ..}) if visit == second)
    );
    assert!(matches!(
        next(&mut fixture.events).await,
        Event::Agents {
            answer: Err(crate::agents::AgentsError::Setup(Problem::Changed)),
            ..
        }
    ));
    assert!(
        fixture.other.calls.lock().unwrap().is_empty(),
        "setup request reached the replacement node"
    );
    fixture.state.onboarding(Ask::Release(second)).await;
    let _released = next(&mut fixture.events).await;
    assert!(fixture.state.client("laptop").is_ok());
}
