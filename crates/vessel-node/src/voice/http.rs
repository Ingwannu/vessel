//! Credential/client lifecycle shared by voice STT and TTS, not dictation policy.
//!
//! Central node TLS and secret resolution remain their original owners. This
//! module owns only per-site requests, fixed deadlines and bounded responses;
//! foreign transport causes are classified and destroyed before diagnostics.

use reqwest::{Client, Url, header};
use std::{fmt, time::Duration};
use vessel_proto::{ResolvedValue, dictation::EndpointBase};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum EndpointError {
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
    #[error("speech response is not the requested format")]
    Format,
    #[error("speech response exceeds its capacity")]
    Capacity,
    #[error("speech segment exceeds its synthesis limit")]
    Segment,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Operation {
    Transcribe,
    Synthesize,
}
impl Operation {
    const fn path(self) -> &'static str {
        match self {
            Self::Transcribe => "transcriptions",
            Self::Synthesize => "speech",
        }
    }
}

pub(super) struct Endpoint {
    client: Client,
    url: Url,
    authorization: header::HeaderValue,
}
impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceHttpEndpoint").finish_non_exhaustive()
    }
}
impl Endpoint {
    pub(super) async fn connect(
        base: &EndpointBase,
        operation: Operation,
        key: &ResolvedValue,
    ) -> Result<Self, EndpointError> {
        let key = key.expose().trim();
        if key.is_empty() {
            return Err(EndpointError::Credential);
        }
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {key}"))
            .map_err(|_private| EndpointError::Credential)?;
        authorization.set_sensitive(true);
        let mut url: Url = base
            .as_str()
            .parse()
            .map_err(|_private| EndpointError::Configuration)?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|()| EndpointError::Configuration)?;
            let _path = path.pop_if_empty().push("audio").push(operation.path());
        }
        let tls = tokio::task::spawn_blocking(crate::http_tls::config)
            .await
            .map_err(|_private| EndpointError::Tls)?
            .map_err(|_private| EndpointError::Tls)?;
        let client = Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .retry(reqwest::retry::never().max_retries_per_request(0))
            .pool_max_idle_per_host(0)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_private| EndpointError::Configuration)?;
        Ok(Self {
            client,
            url,
            authorization,
        })
    }
    pub(super) async fn post(
        &self,
        body: Vec<u8>,
        content_type: header::HeaderValue,
    ) -> Result<reqwest::Response, EndpointError> {
        self.client
            .post(self.url.clone())
            .header(header::AUTHORIZATION, self.authorization.clone())
            .header(header::CONTENT_TYPE, content_type)
            .body(body)
            .send()
            .await
            .map_err(transport)
    }
}

pub(super) fn transport(error: reqwest::Error) -> EndpointError {
    let kind = if error.is_timeout() {
        EndpointError::TimedOut
    } else {
        EndpointError::Transport
    };
    // The foreign cause is intentionally destroyed at this privacy boundary.
    drop(error);
    kind
}

pub(super) async fn read(
    mut response: reqwest::Response,
    maximum: usize,
) -> Result<Vec<u8>, EndpointError> {
    let limit = u64::try_from(maximum).map_err(|_private| EndpointError::Capacity)?;
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(EndpointError::Capacity);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        if body
            .len()
            .checked_add(chunk.len())
            .is_none_or(|length| length > maximum)
        {
            return Err(EndpointError::Capacity);
        }
        body.try_reserve(chunk.len())
            .map_err(|_private| EndpointError::Capacity)?;
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
