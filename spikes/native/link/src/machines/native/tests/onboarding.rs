//! Real framed clients prove setup never redirects work to a replaced alias.

use super::fixtures::{Fixture, next};
use crate::onboarding::{Answer, Ask};
use crate::{Event, LinkError};
use vessel_app_view::onboarding::{
    Layout, Model, State, Step,
    scope::{Problem, Target},
};

#[tokio::test]
async fn replacement_is_refused_and_a_stale_release_cannot_remove_the_new_visit() {
    let mut fixture = Fixture::new().await;
    let target = Target::authenticated(fixture.state.clients["laptop"].context.clone());
    let mut model = Model::default();
    model.reopen(target.clone(), Layout::Wide).unwrap();
    let first = match model.state() {
        State::Open(open) => open.visit(),
        State::Deciding | State::Hidden => panic!("setup did not open"),
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
    let Some(replacement) = fixture.state.clients.get("other").cloned() else {
        panic!("missing replacement fixture");
    };
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
        State::Deciding | State::Hidden => panic!("setup closed"),
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

#[tokio::test]
async fn same_node_reconnect_refuses_old_resolve_and_bind_without_a_request() {
    let mut fixture = Fixture::new().await;
    let original = fixture.state.clients["laptop"].clone();
    let old = original.context.clone();
    original.retire();
    let fresh = fixture
        .state
        .admissions
        .admit(old.machine().clone(), old.node())
        .unwrap();
    let replacement = original.renamed(fresh.clone());
    let _old = fixture
        .state
        .clients
        .insert("laptop".to_owned(), replacement);
    let ticket = vessel_app_view::devices::Tickets::default()
        .next_ticket()
        .unwrap();
    fixture
        .state
        .onboarding(Ask::Resolve {
            context: old.clone(),
            ticket,
        })
        .await;
    assert!(matches!(
        next(&mut fixture.events).await,
        Event::Onboarding(Answer::Resolved {
            result: Err(Problem::Changed),
            ..
        })
    ));
    let mut model = Model::default();
    model
        .reopen(Target::authenticated(old.clone()), Layout::Wide)
        .unwrap();
    let State::Open(open) = model.state() else {
        panic!("fixture did not open");
    };
    fixture
        .state
        .onboarding(Ask::Bind {
            target: Target::authenticated(old),
            visit: open.visit(),
        })
        .await;
    assert!(matches!(
        next(&mut fixture.events).await,
        Event::Onboarding(Answer::Bound {
            result: Err(Problem::Changed),
            ..
        })
    ));
    assert!(matches!(
        fixture.state.onboarding_client("laptop"),
        Err(LinkError::Onboarding(Problem::Changed))
    ));
    assert!(fixture.laptop.calls.lock().unwrap().is_empty());
    assert_ne!(fresh.epoch(), open.host().context().epoch());
}
