//! Fixed vendor reads. Raw bytes stay at this format boundary, never in
//! a domain reply, a log or a database. Requests exist only behind explicit local policy.
use chrono::{DateTime, TimeDelta, Utc};
use reqwest::{StatusCode, header::HeaderValue};
use std::{fmt, sync::Arc, time::Duration};
use zeroize::Zeroizing;
const MAX: usize = 1_048_576;

/// No caller supplies a URL, redirect, proxy or method.
#[derive(Debug, Clone, Copy)]
pub(super) enum Endpoint {
    ClaudeUsage,
    CodexUsage,
    Copilot,
    Cursor,
    Grok,
    Kimi,
    MiniMaxGlobal,
    MiniMaxChina,
    OpenCodeGo,
    Zai,
}
impl Endpoint {
    const fn url(self) -> &'static str {
        match self {
            Self::ClaudeUsage => "https://api.anthropic.com/api/oauth/usage",
            Self::CodexUsage => "https://chatgpt.com/backend-api/wham/usage",
            Self::Copilot => "https://api.github.com/copilot_internal/user",
            Self::Cursor => {
                "https://api2.cursor.sh/aiserver.v1.DashboardService/GetCurrentPeriodUsage"
            }
            Self::Grok => "https://cli-chat-proxy.grok.com/v1/billing?format=credits",
            Self::Kimi => "https://api.kimi.com/coding/v1/usages",
            Self::MiniMaxGlobal => "https://api.minimax.io/v1/token_plan/remains",
            Self::MiniMaxChina => "https://api.minimaxi.com/v1/token_plan/remains",
            Self::OpenCodeGo => "https://opencode.ai/zen/go/v1/usage",
            Self::Zai => "https://api.z.ai/api/biz/subscription/list",
        }
    }
    fn request(
        self,
        client: &reqwest::Client,
        account: Option<&HeaderValue>,
    ) -> reqwest::RequestBuilder {
        let request = match self {
            Endpoint::Cursor => client
                .post(self.url())
                .header("content-type", "application/json")
                .header("Connect-Protocol-Version", "1")
                .body("{}"),
            Endpoint::ClaudeUsage
            | Endpoint::CodexUsage
            | Endpoint::Copilot
            | Endpoint::Grok
            | Endpoint::Kimi
            | Endpoint::MiniMaxGlobal
            | Endpoint::MiniMaxChina
            | Endpoint::OpenCodeGo
            | Endpoint::Zai => client.get(self.url()),
        };
        let mut request = request.header("accept", "application/json");
        match self {
            Endpoint::ClaudeUsage => {
                request = request.header("anthropic-beta", "oauth-2025-04-20");
            }
            Endpoint::CodexUsage => {
                if let Some(account) = account {
                    request = request.header("ChatGPT-Account-Id", account.clone());
                }
            }
            Endpoint::Copilot => {
                request = request
                    .header("Editor-Version", "vscode/1.96.2")
                    .header("Editor-Plugin-Version", "copilot-chat/0.26.7")
                    .header("User-Agent", "GitHubCopilotChat/0.26.7")
                    .header("X-Github-Api-Version", "2025-04-01");
            }
            Endpoint::Grok => {
                request = request.header("X-XAI-Token-Auth", "xai-grok-cli");
            }
            Endpoint::Cursor
            | Endpoint::Kimi
            | Endpoint::MiniMaxGlobal
            | Endpoint::MiniMaxChina
            | Endpoint::OpenCodeGo
            | Endpoint::Zai => {}
        }
        request
    }
}
#[derive(Debug, thiserror::Error)]
pub(super) enum HttpError {
    #[error("the local status connection setup ended unexpectedly")]
    Task(#[source] tokio::task::JoinError),
    #[error("the provider status request could not be completed")]
    Transport(#[source] reqwest::Error),
    #[error("the provider status connection could not be configured")]
    Tls(#[source] rustls::Error),
    #[error("the provider rejected the saved sign-in")]
    Rejected,
    #[error("the provider is limiting status checks")]
    RateLimited { retry_at: Option<DateTime<Utc>> },
    #[error("the provider status response exceeded its supported size or shape")]
    ChangedResponse,
}
/// Sensitive response bytes: even a successful profile includes private
/// account data. Debug says only its size; parser errors never cross the edge.
pub(super) struct Body(Zeroizing<Vec<u8>>);
impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VendorBody")
            .field("bytes", &self.0.len())
            .finish()
    }
}
impl Body {
    pub(super) fn into_bytes(self) -> Zeroizing<Vec<u8>> {
        self.0
    }
}
#[derive(Debug)]
pub(super) struct Http {
    client: reqwest::Client,
}
impl Http {
    /// Called only by the explicitly enabled live source. Recorded test
    /// sources never construct a client or read platform certificates.
    pub(super) fn new() -> Result<Self, HttpError> {
        let found = rustls_native_certs::load_native_certs();
        let mut roots = rustls::RootCertStore::empty();
        let (_added, _ignored) = roots.add_parsable_certificates(found.certs);
        let mut tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(HttpError::Tls)?
        .with_root_certificates(roots)
        .with_no_client_auth();
        tls.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .user_agent("vessel")
            .build()
            .map_err(HttpError::Transport)?;
        Ok(Self { client })
    }
    /// Both header values must be marked sensitive by the credential
    /// parser. They are per-request, never default client headers.
    pub(super) async fn get(
        &self,
        endpoint: Endpoint,
        authorization: &HeaderValue,
        account: Option<&HeaderValue>,
    ) -> Result<Body, HttpError> {
        let request = endpoint
            .request(&self.client, account)
            .header("authorization", authorization.clone());
        let mut response = request.send().await.map_err(HttpError::Transport)?;
        match response.status() {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(HttpError::Rejected),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_at = response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|text| retry_after(text, Utc::now()));
                return Err(HttpError::RateLimited { retry_at });
            }
            status if !status.is_success() => return Err(HttpError::ChangedResponse),
            _ => {}
        }
        let mut body = Zeroizing::new(Vec::new());
        while let Some(chunk) = response.chunk().await.map_err(HttpError::Transport)? {
            if body.len().saturating_add(chunk.len()) > MAX {
                return Err(HttpError::ChangedResponse);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Body(body))
    }
}
pub(super) fn retry_after(text: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if let Ok(seconds) = text.trim().parse::<u64>() {
        let seconds = i64::try_from(seconds).ok()?;
        return now.checked_add_signed(TimeDelta::try_seconds(seconds)?);
    }
    DateTime::parse_from_rfc2822(text)
        .ok()
        .map(|at| at.with_timezone(&Utc))
}

pub(super) async fn read(
    client: &Http,
    source: vessel_proto::limits::LimitLoginStore,
    credential: &super::credential::Credential,
) -> Result<Zeroizing<Vec<u8>>, (vessel_proto::limits::LimitFailure, Option<DateTime<Utc>>)> {
    use vessel_proto::limits::{LimitFailure, LimitLoginStore};
    let endpoint = match source {
        LimitLoginStore::ClaudeCode => Endpoint::ClaudeUsage,
        LimitLoginStore::Codex | LimitLoginStore::OpenCode => Endpoint::CodexUsage,
    };
    let bearer = Zeroizing::new(format!("Bearer {}", credential.token.as_str()));
    let mut authorization =
        HeaderValue::from_str(&bearer).map_err(|_sensitive| (LimitFailure::NoLogin, None))?;
    authorization.set_sensitive(true);
    let account = credential
        .account
        .as_ref()
        .map(|account| {
            let mut value = HeaderValue::from_str(account)
                .map_err(|_sensitive| (LimitFailure::NoLogin, None))?;
            value.set_sensitive(true);
            Ok(value)
        })
        .transpose()?;
    client
        .get(endpoint, &authorization, account.as_ref())
        .await
        .map(|body| body.0)
        .map_err(|error| safe(&error))
}
pub(super) fn safe(
    error: &HttpError,
) -> (vessel_proto::limits::LimitFailure, Option<DateTime<Utc>>) {
    use vessel_proto::limits::LimitFailure;
    match error {
        HttpError::Transport(_) | HttpError::Tls(_) | HttpError::Task(_) => {
            (LimitFailure::Unreachable, None)
        }
        HttpError::Rejected => (LimitFailure::Rejected, None),
        HttpError::RateLimited { retry_at } => (LimitFailure::RateLimited, *retry_at),
        HttpError::ChangedResponse => (LimitFailure::ChangedResponse, None),
    }
}
