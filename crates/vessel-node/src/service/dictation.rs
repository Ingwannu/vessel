//! Dictation delegates behind generated operate scope and agent refusals.
use super::{Connection, NodeService};
use vessel_proto::{
    Ack, WireError,
    dictation::{
        CancelDictation, ChunkDictation, DictationAck, DictationUpdate, FinishAccepted,
        FinishDictation, NextDictation, StartDictation,
    },
};

impl NodeService {
    /// Prepare using the same canonical connection identity on every transport.
    pub(crate) async fn start_dictation(
        &self,
        connection: &Connection,
        request: StartDictation,
    ) -> Result<Ack, WireError> {
        self.dictation
            .start(&connection.dictation_context()?, request)
            .await
    }
    /// Route accepted PCM to that connection's exclusive stream actor.
    pub(crate) async fn chunk_dictation(
        &self,
        connection: &Connection,
        request: ChunkDictation,
    ) -> Result<DictationAck, WireError> {
        self.dictation
            .chunk(&connection.dictation_context()?, request)
            .await
    }
    /// Keep completion acceptance separate from transcript delivery.
    pub(crate) async fn finish_dictation(
        &self,
        connection: &Connection,
        request: FinishDictation,
    ) -> Result<FinishAccepted, WireError> {
        self.dictation
            .finish(&connection.dictation_context()?, request)
            .await
    }
    /// A cancelled RPC wait does not cancel the recording.
    pub(crate) async fn next_dictation(
        &self,
        connection: &Connection,
        request: NextDictation,
    ) -> Result<DictationUpdate, WireError> {
        self.dictation
            .next(&connection.dictation_context()?, request)
            .await
    }
    /// Join actual retirement before acknowledging explicit cancellation.
    pub(crate) async fn cancel_dictation(
        &self,
        connection: &Connection,
        request: CancelDictation,
    ) -> Result<Ack, WireError> {
        self.dictation
            .cancel(&connection.dictation_context()?, request)
            .await
    }
}
