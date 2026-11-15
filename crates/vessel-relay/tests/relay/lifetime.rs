//! A completed registration drain leaves an already handed-off secure carrier alive.
use super::support::{Relay, tls, token};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use vessel_relay::{LinkState, NodeLink, dial};
use vessel_secure::{Config, NodeKeypair, initiate, routing_id};
const OUTER: Duration = Duration::from_secs(30);
const PHASE: Duration = Duration::from_secs(5);
const ACCEPTED: usize = 1;

struct Running {
    stop: CancellationToken,
    task: JoinHandle<()>,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.stop.cancel();
        self.task.abort();
    }
}
async fn phase<T>(name: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(PHASE, Box::pin(future))
        .await
        .unwrap_or_else(|_elapsed| panic!("relay lifetime phase: {name}"))
}

#[tokio::test]
async fn registration_stop_acknowledges_after_handoff_without_closing_encrypted_data() {
    vessel_testkit::within(
        OUTER,
        Box::pin(async {
            let mut relay = Relay::start().await;
            let key = Arc::new(NodeKeypair::generate().unwrap());
            let link = NodeLink::new(relay.url.clone(), Arc::clone(&key), token(), tls());
            let mut state = link.state();
            let stop = CancellationToken::new();
            let (accept, mut accepted) = mpsc::channel(ACCEPTED);
            let mut running = Running {
                stop: stop.clone(),
                task: tokio::spawn(link.run(
                    move |carrier| {
                        accept.try_send(carrier).unwrap();
                    },
                    stop,
                )),
            };
            phase(
                "configured registration ready",
                state.wait_for(|state| *state == LinkState::Registered),
            )
            .await
            .unwrap();
            let route_id = routing_id(key.public());
            let transport_tls = tls();
            let (client, server) = tokio::join!(
                phase(
                    "client reaches registered node",
                    dial(&relay.url, &route_id, &transport_tls)
                ),
                phase("gateway receives carrier ownership", accepted.recv()),
            );
            let client = client.unwrap();
            let server = server.unwrap();
            let secure = Config::default();
            let (client, server) = tokio::join!(
                phase("client encrypted handshake", initiate(client, key.public())),
                phase(
                    "gateway encrypted handshake",
                    vessel_relay::respond(server, &key, &secure)
                ),
            );
            let mut client = client.unwrap();
            let mut server = server.unwrap();
            phase(
                "encrypted message before disable",
                client.send("before".to_owned()),
            )
            .await
            .unwrap();
            assert_eq!(
                phase("gateway receives before disable", server.next())
                    .await
                    .unwrap()
                    .unwrap(),
                "before"
            );
            running.stop.cancel();
            phase(
                "registration and all pending opens drained",
                &mut running.task,
            )
            .await
            .unwrap();
            assert_eq!(*state.borrow(), LinkState::Stopped);
            phase(
                "encrypted message after disable acknowledgement",
                client.send("after".to_owned()),
            )
            .await
            .unwrap();
            assert_eq!(
                phase("existing gateway connection survives", server.next())
                    .await
                    .unwrap()
                    .unwrap(),
                "after"
            );
            phase(
                "reply on existing encrypted connection",
                server.send("still connected".to_owned()),
            )
            .await
            .unwrap();
            assert_eq!(
                phase("client receives after disable", client.next())
                    .await
                    .unwrap()
                    .unwrap(),
                "still connected"
            );
            assert!(accepted.try_recv().is_err());
            drop(client);
            drop(server);
            phase("isolated relay server stopped", relay.stop_owned()).await;
        }),
    )
    .await
    .expect("relay lifetime outer safety");
}
