//! One real connection; the scripted edge is test-only and uses the same Call/Reply types.
use vessel_client::{CallError, NodeClient, Unary};

pub(super) enum Client {
    Node(NodeClient),
    #[cfg(test)]
    Script(tokio::sync::mpsc::Sender<Frame>),
}
impl Client {
    pub(super) async fn call<R: Unary>(&self, request: R) -> Result<R::Response, CallError> {
        match self {
            Self::Node(client) => client.call(request).await,
            #[cfg(test)]
            Self::Script(sender) => {
                let slot = sender
                    .reserve()
                    .await
                    .map_err(|_closed| CallError::Disconnected)?;
                let (reply, answer) = tokio::sync::oneshot::channel();
                slot.send(Frame {
                    call: request.into_call(),
                    reply,
                });
                let reply = answer.await.map_err(|_closed| CallError::Disconnected)??;
                R::response_from(reply).map_err(CallError::UnexpectedReply)
            }
        }
    }
    #[cfg(test)]
    pub(super) fn scripted() -> (Self, tokio::sync::mpsc::Receiver<Frame>) {
        const PENDING_FRAMES: usize = 4;
        let (sender, receiver) = tokio::sync::mpsc::channel(PENDING_FRAMES);
        (Self::Script(sender), receiver)
    }
}
#[cfg(test)]
pub(super) struct Frame {
    pub(super) call: vessel_proto::Call,
    pub(super) reply: tokio::sync::oneshot::Sender<Result<vessel_proto::Reply, CallError>>,
}
