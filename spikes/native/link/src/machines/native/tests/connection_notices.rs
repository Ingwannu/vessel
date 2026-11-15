//! A reconnect notice precedes an otherwise equal Fleet. The shared actor's
//! fresh admission epoch is intentionally different, even for the same node.
use super::fixtures::{Fixture, drain};
use super::peer;
use crate::{
    Event,
    schedules::Message,
    testing::{phase, within},
};

#[tokio::test]
async fn a_fast_new_connection_delivers_its_notice_before_an_otherwise_identical_fleet() {
    within(async {
        let mut f = phase("fixture", Fixture::new()).await;
        phase("initial refresh", f.state.refresh()).await;
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
        let original_id = f.state.clients["laptop"].context.node();
        let original_epoch = f.state.clients["laptop"].context.epoch();
        let mut replacement = phase(
            "same-node reconnect",
            peer::connect_with_node("laptop", original_id),
        )
        .await;
        assert!(!std::sync::Arc::ptr_eq(&f.laptop.calls, &replacement.calls));
        let client = replacement.client.take().unwrap();
        let agents = f.state.clients["laptop"].agents.clone();
        let binding = crate::machines::edits::entries(&f.home.book())
            .unwrap()
            .into_iter()
            .find(|(key, _)| key.as_str() == "laptop")
            .unwrap()
            .1;
        let connected = f
            .state
            .admit_connection("laptop", &binding, client, agents)
            .unwrap();
        assert_eq!(connected.context.node(), original_id);
        assert_ne!(connected.context.epoch(), original_epoch);
        let key = "laptop".parse().unwrap();
        phase(
            "kept lists then area notice",
            f.state.connection_ready(&key, &connected.client),
        )
        .await;
        let previous = f.state.clients.insert("laptop".into(), connected).unwrap();
        previous.retire();
        phase("replacement refresh", f.state.refresh()).await;
        let transcript = drain(&mut f.events);
        let notice = transcript
            .iter()
            .position(|event| {
                matches!(event,
                    Event::Schedules(Message::Connected(key)) if key.as_str() == "laptop"
                )
            })
            .unwrap();
        let after_index = transcript
            .iter()
            .position(|event| matches!(event, Event::Fleet(_)))
            .unwrap();
        assert!(notice < after_index);
        let Event::Fleet(after) = &transcript[after_index] else {
            panic!("wrong event")
        };
        let mut after = after.machines.clone();
        // Only the newly exported admission changes. Removing it here compares
        // ALL legacy display facts, not a hand-picked subset of the Fleet.
        for (before, after) in before.machines.iter().zip(&mut after) {
            if before.name == "laptop" {
                assert_ne!(before.connection_epoch, after.connection_epoch);
                after.connection_epoch = before.connection_epoch;
            }
        }
        assert_eq!(before.machines, after);
    })
    .await;
}
