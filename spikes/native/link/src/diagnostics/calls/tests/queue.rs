//! A full UI queue cannot install old data or strand the original ticket.
use super::*;
use crate::testing::{phase, within};
use vessel_app_view::devices::Tickets;

const QUEUE_ROOM: usize = 1;

#[derive(Clone, Copy)]
enum End {
    Retired,
    Closed,
}
#[tokio::test]
async fn full_queue_settles_retired_and_closed_reads_with_only_a_correlated_refusal() {
    within(async {
        for end in [End::Retired, End::Closed] {
            queued_case(end).await;
        }
    })
    .await;
}
async fn queued_case(end: End) {
    let id = NodeId::random();
    let (client, mut task) = peer(id, Outcome::Ok(Reply::NodeInfo(node(id)))).await;
    let observer = client.clone();
    let (events, mut receiving) = tokio::sync::mpsc::channel(QUEUE_ROOM);
    let (_home, mut state) = fixture_state(events.clone());
    let context = admit(&mut state, client, "queued-diagnostic");
    let selected = state.selected_connection(&context).unwrap();
    let mut tickets = Tickets::default();
    let ticket = tickets.next_ticket().unwrap();
    events
        .send(crate::Event::Fleet(vessel_app_view::Fleet::default()))
        .await
        .unwrap();
    let result = HostResult {
        admitted: id,
        facts: Ok(NodeFacts {
            node: id,
            protocol: selected.protocol(),
            state: NodeState::Serving,
        }),
        daemon: DaemonSnapshot::Unavailable(Problem::Unsupported),
    };
    let sending = publish(
        &selected,
        &events,
        Ask {
            context: context.clone(),
            ticket,
        },
        Ok(result),
    );
    tokio::pin!(sending);
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(sending.as_mut().poll(cx).is_pending()))
            .await
    );
    let expected = match end {
        End::Retired => {
            state.clients["queued-diagnostic"].retire();
            Problem::ConnectionChanged
        }
        End::Closed => {
            task.abort();
            assert!(
                phase("reap closed peer", &mut task)
                    .await
                    .unwrap_err()
                    .is_cancelled()
            );
            phase("original selected transport closed", observer.closed()).await;
            Problem::Offline
        }
    };
    assert!(matches!(
        phase("drain full UI queue", receiving.recv()).await,
        Some(crate::Event::Fleet(_))
    ));
    phase("settle retired publication", sending).await;
    let Some(crate::Event::Diagnostics(Message::Answered(answer))) =
        phase("correlated refusal", receiving.recv()).await
    else {
        panic!("expected diagnostic refusal");
    };
    assert!(answer.guard.is_none());
    assert_eq!(answer.result, Err(expected));
    let received = answer.receive();
    assert_eq!(received.ticket, ticket);
    assert_eq!(received.context, context);
    assert_eq!(received.result, Err(expected));
    assert!(received.guard.is_none());
    assert!(receiving.try_recv().is_err());
    if matches!(end, End::Retired) {
        task.abort();
        assert!(
            phase("reap retired peer", task)
                .await
                .unwrap_err()
                .is_cancelled()
        );
    }
}
