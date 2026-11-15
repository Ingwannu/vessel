//! One bounded metadata exchange; API grammar and TLS stay in vessel-llm.
//! Route choice, key reads, admission and session records belong to the caller.

use bytes::Bytes;
use std::{fmt, time::Duration};
use vessel_llm::{
    client::{Client as HttpClient, Endpoint, Limits},
    completion::{self, Completion, MAX_COMPLETION_BYTES},
    ir,
};

#[cfg(test)]
use vessel_llm::client::Redactor;

use super::prepare::Prepared;

/// The whole exchange includes response headers and body in this clock.
pub(super) const TITLE_CALL_LIMIT: Duration = Duration::from_secs(20);
/// Connection establishment is also inside the whole exchange clock.
const TITLE_CONNECT_LIMIT: Duration = Duration::from_secs(10);
/// No retry may turn one generation into several paid attempts.
const TITLE_ATTEMPTS: u32 = 1;

pub(super) fn policy() -> Limits {
    Limits {
        connect: TITLE_CONNECT_LIMIT,
        first_byte: TITLE_CALL_LIMIT,
        idle: TITLE_CALL_LIMIT,
        max_body: MAX_COMPLETION_BYTES,
        retries: TITLE_ATTEMPTS.saturating_sub(1),
        max_retry_wait: Duration::ZERO,
    }
}

/// The cause is retained typed, but provider messages can quote input.
pub(super) struct ProviderFailure(ir::Failure);
impl ProviderFailure {
    pub(super) const fn class(&self) -> ir::FailureClass {
        self.0.class
    }
}
impl fmt::Debug for ProviderFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderFailure")
            .field("class", &self.class())
            .finish_non_exhaustive()
    }
}
impl fmt::Display for ProviderFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("metadata provider call failed")
    }
}
// The upstream Failure is a typed value, not an Error. Keep it sealed rather
// than adding its potentially content-bearing message to an error chain.
impl std::error::Error for ProviderFailure {}

#[derive(Debug, thiserror::Error)]
pub(super) enum ClientError {
    #[error("metadata client does not use the required bounds and attempt policy")]
    Policy,
    #[error("metadata request does not match the endpoint API")]
    ApiMismatch,
    #[error("metadata call exceeded {} seconds", TITLE_CALL_LIMIT.as_secs())]
    Deadline(#[from] tokio::time::error::Elapsed),
    #[error("metadata transport failed")]
    Provider(#[from] ProviderFailure),
}

pub(super) struct Client {
    transport: Transport,
}
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MetadataClient(..)")
    }
}

enum Transport {
    Http(HttpClient),
    #[cfg(test)]
    Recorded(std::sync::Arc<Recorded>),
    #[cfg(test)]
    Stalled(std::sync::Arc<tokio::sync::Notify>),
}

impl Client {
    /// Test cancellation at an observed in-flight exchange, without wall-clock sleeps.
    #[cfg(test)]
    pub(super) fn stalled() -> (Self, std::sync::Arc<tokio::sync::Notify>) {
        let entered = std::sync::Arc::new(tokio::sync::Notify::new());
        (
            Self {
                transport: Transport::Stalled(std::sync::Arc::clone(&entered)),
            },
            entered,
        )
    }

    /// Reject an accidentally reused proxy policy before any work can leave.
    pub(super) fn new(client: HttpClient) -> Result<Self, ClientError> {
        if client.limits() != policy() {
            return Err(ClientError::Policy);
        }
        Ok(Self {
            transport: Transport::Http(client),
        })
    }

    /// An encoded request is sent once; the outer clock also bounds body reads.
    pub(super) async fn complete(
        &self,
        endpoint: &Endpoint,
        request: Prepared,
    ) -> Result<Completion, ClientError> {
        if request.api() != endpoint.api {
            return Err(ClientError::ApiMismatch);
        }
        tokio::time::timeout(TITLE_CALL_LIMIT, self.read(endpoint, request.into_body())).await?
    }

    async fn read(&self, endpoint: &Endpoint, body: Bytes) -> Result<Completion, ClientError> {
        let (body, redactor) = match &self.transport {
            Transport::Http(client) => {
                let answer = client
                    .send(endpoint, body, &[])
                    .await
                    .map_err(ProviderFailure)?;
                let redactor = answer.redactor();
                (answer.body().await.map_err(ProviderFailure)?, redactor)
            }
            #[cfg(test)]
            Transport::Recorded(record) => {
                let _previous = record
                    .calls
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                (
                    record.reply.clone().map_err(ProviderFailure)?,
                    Redactor::new(endpoint.key.as_ref()),
                )
            }
            #[cfg(test)]
            Transport::Stalled(entered) => {
                entered.notify_one();
                std::future::pending::<(Bytes, Redactor)>().await
            }
        };
        Ok(completion::decode(endpoint.api, &body, &redactor))
    }
}

#[cfg(test)]
struct Recorded {
    reply: Result<Bytes, ir::Failure>,
    calls: std::sync::atomic::AtomicU32,
}

#[cfg(test)]
mod tests;
