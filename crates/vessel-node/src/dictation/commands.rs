//! Bounded stream calls; accepted audio survives cancellation of its reply.

use super::{
    control::{Control, Retention},
    stream_error::{StreamError, StreamResult},
    updates::{Event, Reply},
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use vessel_proto::dictation::{ChunkSeq, DictationPcm, WithinMs};

type ChunkReply = oneshot::Sender<StreamResult<Option<ChunkSeq>>>;
type FinishReply = oneshot::Sender<StreamResult<WithinMs>>;
pub(super) enum Data {
    Chunk {
        seq: ChunkSeq,
        pcm: DictationPcm,
        reply: ChunkReply,
    },
    Finish {
        last: ChunkSeq,
        reply: FinishReply,
    },
}
pub(super) enum Current {
    Chunk(ChunkReply),
    Finish(FinishReply),
}
impl Current {
    pub(super) fn refuse(self, error: Arc<StreamError>) {
        match self {
            Self::Chunk(reply) => drop(reply.send(Err(error))),
            Self::Finish(reply) => drop(reply.send(Err(error))),
        }
    }
}

#[derive(Clone)]
pub(super) struct Handle {
    pub(super) data: mpsc::Sender<Data>,
    pub(super) next: mpsc::Sender<oneshot::Sender<Reply<Arc<StreamError>>>>,
    pub(super) control: Control<StreamError>,
    pub(super) closed: CancellationToken,
}
impl Handle {
    pub(super) async fn chunk(
        &self,
        seq: ChunkSeq,
        pcm: DictationPcm,
    ) -> StreamResult<Option<ChunkSeq>> {
        let slot = self
            .data
            .reserve()
            .await
            .map_err(|source| Arc::new(StreamError::Send(source)))?;
        let (reply, response) = oneshot::channel();
        slot.send(Data::Chunk { seq, pcm, reply });
        response
            .await
            .map_err(|source| Arc::new(StreamError::Reply(source)))?
    }
    pub(super) async fn finish(&self, last: ChunkSeq) -> StreamResult<WithinMs> {
        let slot = self
            .data
            .reserve()
            .await
            .map_err(|source| Arc::new(StreamError::Send(source)))?;
        let (reply, response) = oneshot::channel();
        slot.send(Data::Finish { last, reply });
        response
            .await
            .map_err(|source| Arc::new(StreamError::Reply(source)))?
    }
    pub(super) async fn next(&self) -> StreamResult<Event<Arc<StreamError>>> {
        let slot = self
            .next
            .reserve()
            .await
            .map_err(|source| Arc::new(StreamError::Send(source)))?;
        let (reply, response) = oneshot::channel();
        slot.send(reply);
        match response
            .await
            .map_err(|source| Arc::new(StreamError::Reply(source)))?
        {
            Reply::Update(event) => Ok(event),
            Reply::Refused(source) => Err(Arc::new(StreamError::Read(source))),
        }
    }
    pub(super) async fn cancel(&self) {
        let _first = self
            .control
            .stop(Arc::new(StreamError::Cancelled), Retention::Forget);
        self.closed.cancelled().await;
    }
    pub(super) async fn replace(&self) {
        let _first = self
            .control
            .stop(Arc::new(StreamError::Replaced), Retention::Forget);
        self.closed.cancelled().await;
    }
}
