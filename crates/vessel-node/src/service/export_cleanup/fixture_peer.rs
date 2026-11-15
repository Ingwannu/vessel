//! Raw test edge that deliberately sends metadata before any assignment exists.
//! Bypassing reply projection here forces real envelope refusal/cleanup wiring.

use crate::metadata_export::tests::{exported, info};
use crate::principal::Principal;
use crate::transfers::{Holder, IDLE, TransferError, Transfers};
use futures_util::{SinkExt as _, StreamExt as _};
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::oneshot;
use vessel_client::NodeClient;
use vessel_proto::{
    Ack, Call, ClientFrame, ClientInfo, CloseTransfer, Deliver, DeviceId, Forwarded, NodeId,
    NodeInfo, Origin, Outcome, Reply, Request as _, RequestId, Scope, ServerFrame, SessionListing,
    TransferId, Welcome,
};
use vessel_rpc::{
    ClientConfig,
    memory::{self, MemoryTransport},
};

const FIXTURE_FRAMES: usize = 8;

/// Keep the raw carrier alive until the real cleanup has acknowledged closure.
pub(super) struct FixturePeer {
    _root: tempfile::TempDir,
    held: oneshot::Receiver<(TransferId, Holder)>,
    release: Option<oneshot::Sender<()>>,
    stop: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<Origin>,
    transfers: Arc<Transfers>,
}

impl FixturePeer {
    /// The node description is fictional; no secure pairing or protocol assignment
    /// is claimed for this intentionally nonconforming in-memory responder.
    pub(super) async fn start(mut node: NodeInfo, via: NodeId) -> (NodeClient, Self) {
        node.id = NodeId::random();
        node.name = "raw-export-fixture".into();
        let (client_end, mut server) = memory::pair(NonZeroUsize::new(FIXTURE_FRAMES).unwrap());
        let handshake = async move {
            let hello = server.next().await.unwrap().unwrap();
            assert!(matches!(
                serde_json::from_str::<ClientFrame>(&hello).unwrap(),
                ClientFrame::Hello(_)
            ));
            let welcome = ServerFrame::Welcome(Welcome {
                protocol: vessel_proto::PROTOCOL_VERSION,
                node,
            });
            server
                .send(serde_json::to_string(&welcome).unwrap())
                .await
                .unwrap();
            server
        };
        let config = ClientConfig::default();
        let (client, server) = tokio::join!(
            NodeClient::over(
                client_end,
                ClientInfo {
                    name: "raw-export-fixture".into(),
                    version: "0".into()
                },
                &config
            ),
            handshake
        );
        let root = tempfile::tempdir().unwrap();
        let transfers = Arc::new(Transfers::new(root.path().to_owned(), IDLE));
        let (held, closed) = oneshot::channel();
        let (release, released) = oneshot::channel();
        let (stop, stopped) = oneshot::channel();
        let source = Arc::clone(&transfers);
        let task = tokio::spawn(serve(server, source, via, held, released, stopped));
        (
            client.unwrap(),
            Self {
                _root: root,
                held: closed,
                release: Some(release),
                stop: Some(stop),
                task,
                transfers,
            },
        )
    }

    /// Seeing the actual close proves the envelope reached its refusal arm.
    pub(super) async fn close_seen(&mut self) -> (TransferId, Holder) {
        (&mut self.held).await.unwrap()
    }

    /// Hold the remote close until caller cancellation or result checks are ready.
    pub(super) fn release(&mut self) {
        self.release.take().unwrap().send(()).unwrap();
    }

    /// Keep the peer connected through settlement, then verify the registry released
    /// the exact handle, instead of accepting a dropped or unrelated response.
    pub(super) async fn finish(mut self, transfer: TransferId, holder: Holder) -> Origin {
        assert!(matches!(self.transfers.close(holder, transfer).await,
            Err(TransferError::Unknown(id)) if id == transfer));
        self.stop.take().unwrap().send(()).unwrap();
        self.task.await.unwrap()
    }
}

async fn request(server: &mut MemoryTransport) -> (RequestId, Deliver) {
    let text = server.next().await.unwrap().unwrap();
    let ClientFrame::Request {
        id,
        call: Call::Deliver(delivered),
    } = serde_json::from_str(&text).unwrap()
    else {
        panic!("expected an actual member delivery");
    };
    (id, delivered)
}

async fn answer(server: &mut MemoryTransport, id: RequestId, reply: Reply) {
    let frame = ServerFrame::Response {
        id,
        outcome: Outcome::Ok(Reply::Deliver(Forwarded {
            reply: Box::new(reply),
        })),
    };
    server
        .send(serde_json::to_string(&frame).unwrap())
        .await
        .unwrap();
}

async fn serve(
    mut server: MemoryTransport,
    transfers: Arc<Transfers>,
    via: NodeId,
    held: oneshot::Sender<(TransferId, Holder)>,
    release: oneshot::Receiver<()>,
    stop: oneshot::Receiver<()>,
) -> Origin {
    let (id, export) = request(&mut server).await;
    assert!(matches!(export.call.as_ref(), Call::ExportSession(_)));
    let principal = Principal::delegated(
        DeviceId::random(),
        &[Scope::Read, Scope::Operate],
        export.origin.clone(),
        via,
    );
    let holder = Holder::of(&principal);
    let transfer = transfers
        .commit(transfers.stage(false).unwrap(), holder, std::iter::empty())
        .unwrap();
    let mut current = info();
    current.listing = SessionListing::Background;
    let mut value = exported(current);
    let Reply::ExportSession(manifest) = value.reply.as_mut() else {
        panic!("expected manifest");
    };
    manifest.transfer = transfer;
    answer(&mut server, id, *value.reply).await;
    let (id, close) = request(&mut server).await;
    assert_eq!(
        close.origin, export.origin,
        "cleanup must retain its original principal and scopes"
    );
    let Call::CloseTransfer(request) = *close.call else {
        panic!("expected only the returned transfer's close");
    };
    assert_eq!(request.transfer, transfer);
    principal.permit(CloseTransfer::METHOD).unwrap();
    held.send((transfer, holder)).unwrap();
    release.await.unwrap();
    transfers.close(holder, transfer).await.unwrap();
    answer(&mut server, id, Reply::CloseTransfer(Ack)).await;
    // Do not race the client's response reader by closing this carrier early.
    stop.await.unwrap();
    export.origin
}
