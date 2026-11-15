//! Voice STT uses the shared codec/HTTP lifecycle and its own confidence policy.

use super::{
    http::{Endpoint as HttpEndpoint, EndpointError, Operation},
    input_buffer::Utterance,
    stt_form::{Form, FormError},
    transcript::{self, Scores},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::{
    ResolvedValue,
    dictation::{EndpointBase, EndpointModel, Language},
    voice::VoiceTranscript,
};

pub(super) const MAX_RESPONSE_BYTES: usize = 512 * 1024;

#[derive(Debug, thiserror::Error)]
pub(super) enum SttError {
    #[error("voice transcription request could not complete")]
    Endpoint(#[from] EndpointError),
    #[error("voice transcription request could not be encoded")]
    Form(#[from] FormError),
    #[error("voice transcription response is invalid")]
    Decode(#[from] transcript::TranscriptError),
}

#[derive(Debug)]
pub(super) struct Endpoint {
    http: HttpEndpoint,
    model: EndpointModel,
    language: Language,
}
impl Endpoint {
    pub(super) async fn connect(
        base: &EndpointBase,
        model: EndpointModel,
        language: Language,
        key: &ResolvedValue,
    ) -> Result<Self, SttError> {
        let http = HttpEndpoint::connect(base, Operation::Transcribe, key).await?;
        Ok(Self {
            http,
            model,
            language,
        })
    }
    pub(super) async fn transcribe(
        &self,
        utterance: &Utterance,
        stop: &CancellationToken,
    ) -> Result<VoiceTranscript, SttError> {
        if stop.is_cancelled() {
            return Err(EndpointError::Cancelled.into());
        }
        let (kind, body) = Form::new(utterance, &self.model, &self.language)?.into_parts();
        let response = self.http.post(body, kind, stop).await?;
        if !response.status().is_success() {
            return Err(EndpointError::Rejected.into());
        }
        let bytes = super::http::read(response, MAX_RESPONSE_BYTES, stop).await?;
        Ok(transcript::decode(&bytes, Scores::for_model(&self.model))?)
    }
}

#[cfg(test)]
mod tests;
