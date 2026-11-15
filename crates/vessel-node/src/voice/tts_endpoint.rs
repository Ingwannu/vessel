//! Stream-owned endpoint TTS: bounded PCM, explicit TLS and no foreign diagnostics.
//!
//! It receives a freshly resolved value from the existing secret owner. It
//! never reads credentials, settings, environment or audio files itself.

use std::fmt;

use super::http::{self, Operation};
use reqwest::header;
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use vessel_proto::ResolvedValue;
use vessel_proto::dictation::{EndpointBase, Pcm16Mono};
use vessel_proto::voice::{MAX_PLAYBACK_BYTES, PlaybackPcm, SpeechText, TtsModel, TtsVoice};

use super::segments::MAX_SEGMENT_UNITS;

const PCM_FORMAT: &str = "audio/pcm;rate=24000;bits=16";

pub(super) use super::http::EndpointError as TtsError;

pub(super) struct Endpoint {
    http: http::Endpoint,
    model: TtsModel,
    voice: TtsVoice,
}

impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpeechEndpoint").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(super) struct Synthesized {
    pub(super) format: Pcm16Mono,
    pub(super) pcm: PlaybackPcm,
}

#[derive(Serialize)]
struct Request<'a> {
    model: &'a TtsModel,
    input: &'a str,
    voice: &'a TtsVoice,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ResponseFormat {
    Pcm,
}

impl Endpoint {
    pub(super) async fn connect(
        base: &EndpointBase,
        model: Option<TtsModel>,
        voice: Option<TtsVoice>,
        key: &ResolvedValue,
    ) -> Result<Self, TtsError> {
        let http = http::Endpoint::connect(base, Operation::Synthesize, key).await?;
        Ok(Self {
            http,
            model: model.unwrap_or_default(),
            voice: voice.unwrap_or_default(),
        })
    }

    pub(super) async fn synthesize(
        &self,
        segment: &SpeechText,
        stop: &CancellationToken,
    ) -> Result<Synthesized, TtsError> {
        if stop.is_cancelled() {
            return Err(TtsError::Cancelled);
        }
        if segment.as_str().encode_utf16().count() > MAX_SEGMENT_UNITS {
            return Err(TtsError::Segment);
        }
        let body = serde_json::to_vec(&Request {
            model: &self.model,
            input: segment.as_str(),
            voice: &self.voice,
            response_format: ResponseFormat::Pcm,
        })
        .map_err(|_private| TtsError::Configuration)?;
        let response = self
            .http
            .post(
                body,
                header::HeaderValue::from_static("application/json"),
                stop,
            )
            .await?;
        let pcm = collect(response, stop).await?;
        Ok(Synthesized {
            format: PCM_FORMAT
                .parse()
                .map_err(|_private| TtsError::Configuration)?,
            pcm: PlaybackPcm::new(pcm).map_err(|_private| TtsError::Format)?,
        })
    }
}

async fn collect(
    response: reqwest::Response,
    stop: &CancellationToken,
) -> Result<Vec<u8>, TtsError> {
    if !response.status().is_success() {
        return Err(TtsError::Rejected);
    }
    if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
        let content_type = content_type.to_str().map_err(|_private| TtsError::Format)?;
        let media_type = content_type
            .split(';')
            .next()
            .ok_or(TtsError::Format)?
            .trim();
        if !media_type.eq_ignore_ascii_case("audio/pcm")
            && !media_type.eq_ignore_ascii_case("application/octet-stream")
        {
            return Err(TtsError::Format);
        }
    }
    http::read(response, MAX_PLAYBACK_BYTES, stop).await
}

#[cfg(test)]
mod tests;
