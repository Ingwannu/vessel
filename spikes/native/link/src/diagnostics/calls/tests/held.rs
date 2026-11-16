//! The real command dispatcher expires held reads and returns their permits.
use super::*;
use std::{sync::mpsc as wall_channel, thread};
use tokio::sync::{mpsc, oneshot};
use vessel_app_view::devices::{ConnectionContext, Ticket, Tickets};

const EVENT_ROOM: usize = MAX_PARALLEL_HOSTS + 1;
const HELD_REQUESTS: usize = MAX_PARALLEL_HOSTS.checked_mul(2).unwrap();
const BEFORE_EXPIRY: Duration = Duration::from_millis(1);
const WALL_SAFETY: Duration = Duration::from_secs(20);

struct HeldPeer {
    client: NodeClient,
    written: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

#[tokio::test(start_paused = true)]
async fn held_dispatch_expires_at_its_deadline_and_releases_real_permits() {
    let (finished, watching) = wall_channel::channel();
    let (abort, aborted) = oneshot::channel::<()>();
    let safety = thread::spawn(move || {
        if watching.recv_timeout(WALL_SAFETY).is_err() {
            let _gone = abort.send(());
        }
    });
    tokio::select! {
        () = deadline_case() => {},
        _expired = aborted => panic!("finite diagnostic test wall-clock safety expired"),
    }
    let _stopped = finished.send(());
    safety.join().unwrap();
}

async fn deadline_case() {
    let node_id = NodeId::random();
    let HeldPeer {
        client,
        written,
        release,
        task,
    } = held_peer(node_id).await;
    let (events, mut receiving) = mpsc::channel(EVENT_ROOM);
    let (_home, mut state) = fixture_state(events);
    let context = admit(&mut state, client, "held-host");
    let mut tickets = Tickets::default();
    let pending =
        occupy_and_refuse(&mut state, &context, &mut tickets, written, &mut receiving).await;
    expire_pending(&state, &mut receiving, pending).await;
    release.send(()).unwrap();
    let next = ask(&context, &mut tickets);
    let ticket = next.ticket;
    state.handle(crate::Command::Diagnostics(next)).await;
    let answer = next_answer(&mut receiving).await;
    assert_eq!(answer.ticket, ticket);
    assert_eq!(answer.result.unwrap().facts.unwrap().node, node_id);
    assert_eq!(
        state.diagnostic_calls.0.available_permits(),
        MAX_PARALLEL_HOSTS
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
}

async fn occupy_and_refuse(
    state: &mut crate::State,
    context: &ConnectionContext,
    tickets: &mut Tickets,
    written: oneshot::Receiver<()>,
    receiving: &mut mpsc::Receiver<crate::Event>,
) -> Vec<Ticket> {
    let mut pending = Vec::new();
    for _ in 0..MAX_PARALLEL_HOSTS {
        let next = ask(context, tickets);
        pending.push(next.ticket);
        state.handle(crate::Command::Diagnostics(next)).await;
    }
    written.await.unwrap();
    assert_eq!(state.diagnostic_calls.0.available_permits(), 0);
    let overflow = ask(context, tickets);
    let ticket = overflow.ticket;
    state.handle(crate::Command::Diagnostics(overflow)).await;
    let answer = next_answer(receiving).await;
    assert_eq!(answer.ticket, ticket);
    assert_eq!(answer.result, Err(Problem::Busy));
    pending
}

async fn expire_pending(
    state: &crate::State,
    receiving: &mut mpsc::Receiver<crate::Event>,
    mut pending: Vec<Ticket>,
) {
    let began = tokio::time::Instant::now();
    tokio::time::advance(NODE_INFO_DEADLINE.checked_sub(BEFORE_EXPIRY).unwrap()).await;
    tokio::task::yield_now().await;
    assert!(matches!(
        receiving.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert_eq!(state.diagnostic_calls.0.available_permits(), 0);
    tokio::time::advance(BEFORE_EXPIRY).await;
    for _ in 0..MAX_PARALLEL_HOSTS {
        let answer = next_answer(receiving).await;
        let position = pending
            .iter()
            .position(|ticket| *ticket == answer.ticket)
            .unwrap();
        let _settled = pending.remove(position);
        let result = answer.result.unwrap();
        assert_eq!(result.facts, Err(Problem::TimedOut));
        assert_eq!(
            result.daemon,
            DaemonSnapshot::Unavailable(Problem::TimedOut)
        );
    }
    assert!(pending.is_empty());
    assert!(began.elapsed() <= NODE_INFO_DEADLINE.checked_add(BEFORE_EXPIRY).unwrap());
    assert_eq!(
        state.diagnostic_calls.0.available_permits(),
        MAX_PARALLEL_HOSTS
    );
}

fn ask(context: &ConnectionContext, tickets: &mut Tickets) -> Ask {
    Ask {
        context: context.clone(),
        ticket: tickets.next_ticket().unwrap(),
    }
}
async fn next_answer(receiving: &mut mpsc::Receiver<crate::Event>) -> Answer {
    let crate::Event::Diagnostics(Message::Answered(answer)) = receiving.recv().await.unwrap()
    else {
        panic!("expected a real diagnostic dispatch answer");
    };
    answer
}

async fn held_peer(admitted: NodeId) -> HeldPeer {
    let (local, mut remote) = tokio::io::duplex(FRAME_BYTES);
    let (seen, written) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let task = tokio::spawn(async move {
        welcome(&mut remote, admitted).await;
        hold_requests(&mut remote, seen, released).await;
        for _ in 0..2 {
            let ClientFrame::Request { id, call } = read(&mut remote).await else {
                panic!("released permit must admit next read");
            };
            assert!(matches!(&call, Call::NodeInfo(_) | Call::Diagnostics(_)));
            let outcome = if matches!(call, Call::NodeInfo(_)) {
                Outcome::Ok(Reply::NodeInfo(node(admitted)))
            } else {
                Outcome::Err(WireError::new(ErrorCode::Unsupported, CANARY))
            };
            write(&mut remote, ServerFrame::Response { id, outcome }).await;
        }
        std::future::pending::<()>().await;
    });
    HeldPeer {
        client: framed_client(local).await,
        written,
        release,
        task,
    }
}

async fn hold_requests(
    remote: &mut DuplexStream,
    seen: oneshot::Sender<()>,
    released: oneshot::Receiver<()>,
) {
    let mut held = Vec::new();
    for _ in 0..HELD_REQUESTS {
        let ClientFrame::Request { id, call } = read(remote).await else {
            panic!("expected a held node-info request");
        };
        assert!(matches!(call, Call::NodeInfo(_) | Call::Diagnostics(_)));
        held.push(id);
    }
    seen.send(()).unwrap();
    released.await.unwrap();
    for _ in 0..HELD_REQUESTS {
        let ClientFrame::Cancel { id } = read(remote).await else {
            panic!("expired reads must cancel their RPCs");
        };
        let position = held.iter().position(|held| *held == id).unwrap();
        let _cancelled = held.remove(position);
    }
    assert!(held.is_empty());
}
