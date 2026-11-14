//! A TCP path between a client and a node that can freeze half-open, for
//! the keepalive tests: `/secure` (`secure.rs`) and `/rpc` (`parking.rs`).

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use vessel_client::NodeUrl;
use vessel_proto::RouteUrl;

/// A TCP forwarder that can freeze the connections it carries: they stay
/// open but carry nothing to the client, and what the node sends is read
/// and dropped, as on a path that went half-open (a laptop asleep, a NAT
/// mapping gone). New connections still pass. It reports each frozen
/// connection the node then closes.
pub(crate) struct Freezer {
    address: std::net::SocketAddr,
    freeze: tokio::sync::watch::Sender<u64>,
    /// One `()` for each frozen connection the node closed.
    pub(crate) closed_by_node: tokio::sync::mpsc::Receiver<()>,
    task: JoinHandle<()>,
}

impl Freezer {
    /// Forward connections made to it to `upstream`.
    pub(crate) async fn start(upstream: std::net::SocketAddr) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (freeze, _) = tokio::sync::watch::channel(0_u64);
        let (report, closed_by_node) = tokio::sync::mpsc::channel(8);
        let generations = freeze.clone();
        let task = tokio::spawn(async move {
            while let Ok((down, _peer)) = listener.accept().await {
                let up = tokio::net::TcpStream::connect(upstream).await.unwrap();
                let frozen = generations.subscribe();
                drop(tokio::spawn(carry(down, up, frozen, report.clone())));
            }
        });
        Self {
            address,
            freeze,
            closed_by_node,
            task,
        }
    }

    /// It, as a direct route of a pairing offer.
    pub(crate) fn route(&self) -> RouteUrl {
        format!("ws://{}", self.address).parse().unwrap()
    }

    /// It, as a node's address for a token connection (`/rpc`).
    pub(crate) fn url(&self) -> NodeUrl {
        self.address.to_string().parse().unwrap()
    }

    /// Freeze every connection open now.
    pub(crate) fn freeze(&self) {
        self.freeze
            .send_modify(|generation| *generation = generation.saturating_add(1));
    }
}

impl Drop for Freezer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn carry(
    down: tokio::net::TcpStream,
    up: tokio::net::TcpStream,
    mut frozen: tokio::sync::watch::Receiver<u64>,
    closed_by_node: tokio::sync::mpsc::Sender<()>,
) {
    use tokio::io::AsyncReadExt as _;
    let born = *frozen.borrow();
    let (mut down_read, mut down_write) = down.into_split();
    let (mut up_read, mut up_write) = up.into_split();
    let carrying = async {
        tokio::try_join!(
            tokio::io::copy(&mut down_read, &mut up_write),
            tokio::io::copy(&mut up_read, &mut down_write),
        )
    };
    tokio::select! {
        _done = carrying => return,
        _frozen = frozen.wait_for(|generation| *generation > born) => {}
    }
    // Frozen: the client hears nothing more; the node's pings are read and
    // dropped until the node gives up and closes.
    let mut sink = [0_u8; 4096];
    while matches!(up_read.read(&mut sink).await, Ok(read) if read > 0) {}
    let _reported = closed_by_node.send(()).await;
    // The client's side stays open, silent, until the test ends.
    std::future::pending::<()>().await;
}
