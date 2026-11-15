//! Both live and recorded bodies pass the same finite byte/HTTP status boundary.
use super::{
    AccountUsageProbe,
    credential::{Credential, Region},
};
use crate::limits::{
    LimitContext,
    context::Admission,
    http::{self, Endpoint},
};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use vessel_proto::{limits::LimitFailure, usage_reports::BuiltinUsageSource};
use zeroize::Zeroizing;
impl AccountUsageProbe {
    pub(super) async fn body(
        &self,
        context: &LimitContext,
        admission: &Admission,
        source: BuiltinUsageSource,
        credential: &Credential,
        now: DateTime<Utc>,
        stop: &CancellationToken,
    ) -> Result<Zeroizing<Vec<u8>>, (LimitFailure, Option<DateTime<Utc>>)> {
        match self {
            Self::Disabled => Err((LimitFailure::Unsupported, None)),
            Self::Local(_) => {
                let client = context
                    .client(admission)
                    .await
                    .map_err(|error| http::safe(&error))?;
                if stop.is_cancelled() {
                    return Err((LimitFailure::Unsupported, None));
                }
                let prefix = if source == BuiltinUsageSource::Copilot {
                    "token"
                } else {
                    "Bearer"
                };
                let text = Zeroizing::new(format!("{prefix} {}", credential.token.as_str()));
                let mut authorization = reqwest::header::HeaderValue::from_str(&text)
                    .map_err(|_private| (LimitFailure::NoLogin, None))?;
                authorization.set_sensitive(true);
                let endpoint =
                    endpoint(source, credential.region).ok_or((LimitFailure::Unsupported, None))?;
                client
                    .get(endpoint, &authorization, None)
                    .await
                    .map(http::Body::into_bytes)
                    .map_err(|error| http::safe(&error))
            }
            Self::Recorded {
                root,
                body,
                status,
                retry_after,
                ..
            } => {
                match *status {
                    401 | 403 => return Err((LimitFailure::Rejected, None)),
                    429 => {
                        return Err((
                            LimitFailure::RateLimited,
                            retry_after
                                .as_deref()
                                .and_then(|text| http::retry_after(text, now)),
                        ));
                    }
                    200..=299 => {}
                    _ => return Err((LimitFailure::ChangedResponse, None)),
                }
                let held = Arc::clone(admission);
                let root = root.clone();
                let body = body.clone();
                let cancelled = stop.clone();
                tokio::task::spawn_blocking(move || {
                    let _held = held;
                    if cancelled.is_cancelled() {
                        return Err((LimitFailure::Unsupported, None));
                    }
                    let root = root
                        .canonicalize()
                        .map_err(|_private| (LimitFailure::UnsafeLoginStore, None))?;
                    let body = body
                        .canonicalize()
                        .map_err(|_private| (LimitFailure::ChangedResponse, None))?;
                    if !body.starts_with(root) {
                        return Err((LimitFailure::UnsafeLoginStore, None));
                    }
                    crate::limits::credential::bounded(&body)
                        .map_err(|_private| (LimitFailure::ChangedResponse, None))
                })
                .await
                .map_err(|_task| (LimitFailure::ChangedResponse, None))?
            }
        }
    }
}
fn endpoint(source: BuiltinUsageSource, region: Region) -> Option<Endpoint> {
    Some(match source {
        BuiltinUsageSource::Copilot => Endpoint::Copilot,
        BuiltinUsageSource::Cursor => Endpoint::Cursor,
        BuiltinUsageSource::Grok => Endpoint::Grok,
        BuiltinUsageSource::Kimi => Endpoint::Kimi,
        BuiltinUsageSource::MiniMax => match region {
            Region::Global => Endpoint::MiniMaxGlobal,
            Region::China => Endpoint::MiniMaxChina,
        },
        BuiltinUsageSource::OpenCodeGo => Endpoint::OpenCodeGo,
        BuiltinUsageSource::Zai => Endpoint::Zai,
        BuiltinUsageSource::Claude | BuiltinUsageSource::Codex => return None,
    })
}
