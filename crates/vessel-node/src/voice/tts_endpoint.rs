//! Stream-owned endpoint TTS: bounded PCM, explicit TLS and no foreign diagnostics.
//!
//! It receives a freshly resolved value from the existing secret owner. It
//! never reads credentials, settings, environment or audio files itself.

use std::fmt;
use std::time::Duration;

use reqwest::{Client, Url, header};
use serde::Serialize;
use vessel_proto::ResolvedValue;
use vessel_proto::dictation::{EndpointBase, EndpointModel, Pcm16Mono};
use vessel_proto::voice::{MAX_PLAYBACK_BYTES, PlaybackPcm, SpeechText};

use super::segments::MAX_SEGMENT_UNITS;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const PCM_FORMAT: &str = "audio/pcm;rate=24000;bits=16";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum TtsError {
    #[error("speech endpoint configuration is invalid")]
    Configuration,
    #[error("speech endpoint credential is absent or invalid")]
    Credential,
    #[error("speech TLS configuration could not be prepared")]
    Tls,
    #[error("speech request could not complete")]
    Transport,
    #[error("speech request exceeded its deadline")]
    TimedOut,
    #[error("speech endpoint rejected the request")]
    Rejected,
    #[error("speech response is not the requested PCM format")]
    Format,
    #[error("speech response exceeds its capacity")]
    Capacity,
    #[error("speech segment exceeds its synthesis limit")]
    Segment,
}

pub(super) struct Endpoint {
    client: Client,
    url: Url,
    authorization: header::HeaderValue,
    model: Option<EndpointModel>,
    voice: Option<EndpointModel>,
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
    model: &'a str,
    input: &'a str,
    voice: &'a str,
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
        model: Option<EndpointModel>,
        voice: Option<EndpointModel>,
        key: &ResolvedValue,
    ) -> Result<Self, TtsError> {
        let key = key.expose().trim();
        if key.is_empty() {
            return Err(TtsError::Credential);
        }
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {key}"))
            .map_err(|_private| TtsError::Credential)?;
        authorization.set_sensitive(true);
        // Reuse the existing validated base, adding only this operation's path.
        let mut url: Url = base
            .as_str()
            .parse()
            .map_err(|_private| TtsError::Configuration)?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|()| TtsError::Configuration)?;
            let _path = path.pop_if_empty().push("audio").push("speech");
        }
        let tls = tokio::task::spawn_blocking(crate::http_tls::config)
            .await
            .map_err(|_private| TtsError::Tls)?
            .map_err(|_private| TtsError::Tls)?;
        let client = Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .retry(reqwest::retry::never().max_retries_per_request(0))
            .pool_max_idle_per_host(0)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_private| TtsError::Configuration)?;
        Ok(Self {
            client,
            url,
            authorization,
            model,
            voice,
        })
    }

    pub(super) async fn synthesize(&self, segment: &SpeechText) -> Result<Synthesized, TtsError> {
        if segment.as_str().encode_utf16().count() > MAX_SEGMENT_UNITS {
            return Err(TtsError::Segment);
        }
        let body = serde_json::to_vec(&Request {
            model: self.model.as_ref().map_or("tts-1", EndpointModel::as_str),
            input: segment.as_str(),
            voice: self.voice.as_ref().map_or("alloy", EndpointModel::as_str),
            response_format: ResponseFormat::Pcm,
        })
        .map_err(|_private| TtsError::Configuration)?;
        let response = self
            .client
            .post(self.url.clone())
            .header(header::AUTHORIZATION, self.authorization.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(transport)?;
        let pcm = collect(response).await?;
        Ok(Synthesized {
            format: PCM_FORMAT
                .parse()
                .map_err(|_private| TtsError::Configuration)?,
            pcm: PlaybackPcm::new(pcm).map_err(|_private| TtsError::Format)?,
        })
    }
}

fn transport(error: reqwest::Error) -> TtsError {
    let kind = if error.is_timeout() {
        TtsError::TimedOut
    } else {
        TtsError::Transport
    };
    // The foreign cause is intentionally destroyed at this privacy boundary.
    drop(error);
    kind
}

async fn collect(mut response: reqwest::Response) -> Result<Vec<u8>, TtsError> {
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
    let limit = u64::try_from(MAX_PLAYBACK_BYTES).map_err(|_private| TtsError::Capacity)?;
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(TtsError::Capacity);
    }
    let mut pcm = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        if pcm
            .len()
            .checked_add(chunk.len())
            .is_none_or(|length| length > MAX_PLAYBACK_BYTES)
        {
            return Err(TtsError::Capacity);
        }
        pcm.try_reserve(chunk.len())
            .map_err(|_private| TtsError::Capacity)?;
        pcm.extend_from_slice(&chunk);
    }
    Ok(pcm)
}

#[cfg(test)]
mod tests;
