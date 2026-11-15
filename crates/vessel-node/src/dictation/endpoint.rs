//! A stream-owned OpenAI-compatible HTTP client, with typed metadata and headers.
//!
//! It reaches only the validated admin-selected endpoint using one selected node
//! secret. Redirects and environment proxies are off; idle pooling follows the commit cap.
//! Only bounded in-memory multipart audio is sent; no audio or response is logged.

use super::{
    audio::Segment,
    multipart::{Fields, Multipart},
    provider_json,
    roots::Factory,
};
use reqwest::{Client, Url, header::HeaderValue};
use std::{sync::Arc, time::Duration};
use vessel_llm::client::{ApiKey, Redactor};
use vessel_proto::{
    ResolvedValue,
    dictation::{EndpointBase, EndpointModel, Language, Transcript},
};

const CONNECT: Duration = Duration::from_secs(10);
const REQUEST: Duration = Duration::from_secs(60);
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_UPLOAD_ATTEMPTS: u32 = 1;

/// Retain lower causes once without letting their payload enter diagnostics.
#[derive(thiserror::Error)]
pub(super) enum ProviderError {
    #[error("the dictation endpoint could not complete the request")]
    Http(#[source] reqwest::Error),
    #[error("the dictation TLS roots could not be loaded")]
    Roots(#[source] Arc<super::roots::RootsError>),
    #[error("the dictation endpoint has no usable credential")]
    KeyAbsent,
    #[error("the dictation endpoint address could not be constructed")]
    Address(#[source] vessel_proto::dictation::DictationDecodeError),
    #[error("the dictation credential cannot be used in a request header")]
    Header(#[source] reqwest::header::InvalidHeaderValue),
    #[error("the dictation audio could not be encoded")]
    Form(#[from] super::multipart::FormError),
    #[error("the dictation endpoint response exceeds {max} bytes", max = MAX_RESPONSE_BYTES)]
    ResponseTooLarge,
    #[error("the dictation endpoint response count exceeds this platform's range")]
    Count(#[source] std::num::TryFromIntError),
    #[error("the dictation endpoint returned no usable transcript")]
    Decode(#[from] provider_json::DecodeError),
}

// A peer-controlled transport cause must not enter a caller's Debug log.
// Error::source still retains that typed cause for classification; wire errors
// map once to fixed reasons and never format a foreign error chain.
impl std::fmt::Debug for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl ProviderError {
    fn transport(source: reqwest::Error) -> Self {
        Self::Http(source.without_url())
    }
}

/// Construct before starting any root-loading or network work.
pub(super) struct Specification {
    url: Url,
    model: EndpointModel,
    language: Language,
    authorization: HeaderValue,
    redactor: Redactor,
}

impl Specification {
    /// Reject header or path failures before any asynchronous provider preparation.
    pub(super) fn new(
        base: &EndpointBase,
        model: EndpointModel,
        language: Language,
        key: &ResolvedValue,
    ) -> Result<Self, ProviderError> {
        let key = key.expose().trim();
        if key.is_empty() {
            return Err(ProviderError::KeyAbsent);
        }
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {key}")).map_err(ProviderError::Header)?;
        authorization.set_sensitive(true);
        let url = base.transcription_url().map_err(ProviderError::Address)?;
        let redactor = Redactor::new(Some(&ApiKey::new(key.to_owned())));
        Ok(Self {
            url,
            model,
            language,
            authorization,
            redactor,
        })
    }

    /// Build a stream-owned client from already loaded roots, never global policy.
    pub(super) async fn connect(self, roots: &Factory) -> Result<Endpoint, ProviderError> {
        let roots = roots.get().await.map_err(ProviderError::Roots)?;
        let http = Client::builder()
            .use_preconfigured_tls(roots.for_client())
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            // A failed upload may already have been accepted. Disable protocol
            // NACKs as well as retries on HTTP statuses, with one attempt only.
            .retry(
                reqwest::retry::never()
                    .max_retries_per_request(MAX_UPLOAD_ATTEMPTS.saturating_sub(1)),
            )
            .connect_timeout(CONNECT)
            .pool_max_idle_per_host(super::MAX_IN_FLIGHT_COMMITS)
            .build()
            .map_err(ProviderError::transport)?;
        Ok(Endpoint {
            http,
            specification: Arc::new(self),
            #[cfg(test)]
            headers_received: None,
        })
    }
}

/// Owning the HTTP client per stream bounds hosts and credentials with stream life.
#[derive(Clone)]
pub(super) struct Endpoint {
    http: Client,
    specification: Arc<Specification>,
    // A phase observation only: tests must distinguish body and header waits.
    #[cfg(test)]
    headers_received: Option<Arc<tokio::sync::Notify>>,
}

impl Endpoint {
    /// The request timeout covers headers and the bounded response body.
    pub(super) async fn transcribe(&self, segment: &Segment) -> Result<Transcript, ProviderError> {
        let spec = &self.specification;
        let form = Multipart::new(
            segment,
            &Fields {
                model: &spec.model,
                language: &spec.language,
            },
        )?;
        let (content_type, bytes) = form.into_parts();
        let response = self
            .http
            .post(spec.url.clone())
            .timeout(REQUEST)
            .header(reqwest::header::AUTHORIZATION, spec.authorization.clone())
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await
            .map_err(ProviderError::transport)?;
        let status = response.status();
        #[cfg(test)]
        if let Some(ready) = &self.headers_received {
            ready.notify_one();
        }
        let bytes = read(response).await?;
        provider_json::decode(status, &bytes, &spec.redactor).map_err(ProviderError::Decode)
    }
}

async fn read(mut response: reqwest::Response) -> Result<Vec<u8>, ProviderError> {
    if let Some(length) = response.content_length() {
        let length = usize::try_from(length).map_err(ProviderError::Count)?;
        if length > MAX_RESPONSE_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(ProviderError::transport)? {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(ProviderError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
#[path = "endpoint_tests.rs"]
pub(super) mod tests;
