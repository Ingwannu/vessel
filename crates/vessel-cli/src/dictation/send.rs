//! Send bounded file chunks serially while an independent next call drains results.
use super::{client::Client, error::Failure, file::FileInput};
use vessel_client::Unary;
use vessel_proto::dictation::{
    ChunkDictation, ChunkSeq, DictationId, FinishAccepted, FinishDictation,
};

pub(super) async fn call<R: Unary>(client: &Client, request: R) -> Result<R::Response, Failure> {
    client.call(request).await.map_err(|source| Failure::Call {
        method: <R as vessel_proto::Request>::METHOD,
        source,
    })
}

pub(super) async fn upload(
    client: &Client,
    id: &DictationId,
    input: &mut FileInput,
    finishing: tokio::sync::oneshot::Sender<()>,
) -> Result<FinishAccepted, Failure> {
    let mut sequence = ChunkSeq::new(0);
    let mut final_seq = None;
    while let Some(pcm) = input.next().await? {
        let ack = call(
            client,
            ChunkDictation {
                id: id.clone(),
                seq: sequence,
                pcm,
            },
        )
        .await?;
        if ack.through != Some(sequence) {
            return Err(Failure::Ack {
                expected: sequence,
                got: ack.through,
            });
        }
        final_seq = Some(sequence);
        sequence = ChunkSeq::new(sequence.get().checked_add(1).ok_or(Failure::Sequence)?);
    }
    let final_seq = final_seq.ok_or(Failure::Empty)?;
    let _closed = finishing.send(());
    call(
        client,
        FinishDictation {
            id: id.clone(),
            final_seq,
        },
    )
    .await
}
