//! A fresh framed client may have exactly the old visible Fleet. The lifecycle
//! notice is independent and must not be lost behind the shell's equality guard.
use super::fixtures::{Fixture, drain};
use super::peer;
use crate::{Connected, Event, schedules::Message};
use std::time::Duration;
const READY_WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn a_fast_new_connection_delivers_its_notice_before_an_identical_fleet() {
    let mut f = Fixture::new().await;
    f.state.refresh().await;
    let before = drain(&mut f.events)
        .into_iter()
        .find_map(|event| {
            if let Event::Fleet(fleet) = event {
                Some(fleet)
            } else {
                None
            }
        })
        .unwrap();
    let original_id = f.state.clients["laptop"].client.node().id;
    let mut replacement = peer::connect_with_node("laptop", original_id).await;
    assert!(!std::sync::Arc::ptr_eq(&f.laptop.calls, &replacement.calls));
    let client = replacement.client.take().unwrap();
    let original = &f.state.clients["laptop"];
    let agents = original.agents.clone();
    assert_eq!(client.node().id, original.client.node().id);
    // This is the same helper called at the actor's successful-connect site.
    let key = "laptop".parse().unwrap();
    tokio::time::timeout(READY_WAIT, f.state.connection_ready(&key, &client))
        .await
        .unwrap();
    let _old = f
        .state
        .clients
        .insert("laptop".into(), Connected { client, agents });
    f.state.refresh().await;
    let transcript = drain(&mut f.events);
    let notice = transcript
        .iter()
        .position(|event| {
            matches!(event,
                Event::Schedules(Message::Connected(key)) if key.as_str()=="laptop"
            )
        })
        .unwrap();
    let after = transcript
        .iter()
        .position(|event| matches!(event, Event::Fleet(_)))
        .unwrap();
    assert!(notice < after);
    let Event::Fleet(after) = &transcript[after] else {
        panic!("wrong event")
    };
    assert_eq!(before.machines, after.machines);
}
