//! Diagnostics owns its drain proof; Scenario's Devices ACK is insufficient.
use super::*;
use crate::testing::{peer as framed, phase, within};
use vessel_app_view::devices::Tickets;

const EVENT_ROOM: usize = 1;

#[tokio::test]
async fn dropping_actor_and_egress_drains_diagnostics_before_peer_close_or_sdk_deadline() {
    within(async {
        let mut peer = framed::controlled("diagnostic-drain").await;
        let client = peer.client.take().unwrap();
        // Keep physical transport alive so a generic disconnect cannot make
        // this pass in place of original-admission retirement cancellation.
        let observer = client.clone();
        let (events, receiving) = tokio::sync::mpsc::channel(EVENT_ROOM);
        let (_home, mut state) = fixture_state(events);
        let context = admit(&mut state, client, "diagnostic-drain");
        let slots = Arc::clone(&state.diagnostic_calls.0);
        let mut tickets = Tickets::default();
        state
            .handle(crate::Command::Diagnostics(Ask {
                context,
                ticket: tickets.next_ticket().unwrap(),
            }))
            .await;
        let held = phase(
            "actual diagnostic RPC held",
            peer.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap();
        assert!(matches!(
            held.call,
            Call::NodeInfo(_) | Call::Diagnostics(_)
        ));
        assert_eq!(slots.available_permits(), MAX_PARALLEL_HOSTS - 1);
        drop(state);
        drop(receiving);
        phase(
            "all diagnostic workers drained after actor and egress drop",
            async {
                let mut drained = Vec::with_capacity(MAX_PARALLEL_HOSTS);
                for _ in 0..MAX_PARALLEL_HOSTS {
                    drained.push(slots.acquire().await.unwrap());
                }
                assert_eq!(drained.len(), MAX_PARALLEL_HOSTS);
            },
        )
        .await;
        assert_eq!(slots.available_permits(), MAX_PARALLEL_HOSTS);
        assert!(!peer.task.as_ref().unwrap().is_finished());
        let closed = observer.closed();
        tokio::pin!(closed);
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                closed.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        // The held reply remains owned through the cancellation/drain proof.
        // Now explicitly abort and reap the one fixture peer before returning.
        drop(held);
        peer.task.as_ref().unwrap().abort();
        let joined = phase("reap diagnostic drain peer", peer.task.as_mut().unwrap()).await;
        if let Err(error) = joined {
            assert!(error.is_cancelled());
        }
        let _reaped = peer.task.take();
    })
    .await;
}
