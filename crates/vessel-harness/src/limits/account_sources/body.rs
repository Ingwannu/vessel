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
        #[cfg(feature = "fakes")]
        if let Self::Recorded { access, .. } = self
            && let Some(reading) = &access.recorded_http
        {
            // Invoke only inside the admitted future: revoked work must not even
            // poll the fake transport, exactly as for the fixed-origin live path.
            return cancellable(stop, async {
                reading()
                    .await
                    .map(Zeroizing::new)
                    .map_err(|reason| (reason, None))
            })
            .await;
        }
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
                cancellable(stop, async {
                    client
                        .get(endpoint, &authorization, None)
                        .await
                        .map(http::Body::into_bytes)
                        .map_err(|error| http::safe(&error))
                })
                .await
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
// Match the saved-login path: a cancelled capability must not poll a queued
// HTTP operation, and cancellation during HTTP must drop that operation too.
async fn cancellable<T>(
    stop: &CancellationToken,
    reading: impl Future<Output = Result<T, (LimitFailure, Option<DateTime<Utc>>)>>,
) -> Result<T, (LimitFailure, Option<DateTime<Utc>>)> {
    tokio::select! {
        biased;
        () = stop.cancelled() => Err((LimitFailure::Unsupported, None)),
        result = reading => result,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    const CANCELLATION_TEST_BUDGET: std::time::Duration = std::time::Duration::from_secs(20);
    struct Dropped(Arc<AtomicBool>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    #[tokio::test]
    async fn revoked_http_capability_does_not_poll_the_request() {
        let stop = CancellationToken::new();
        stop.cancel();
        let polled = AtomicBool::new(false);
        let result = cancellable(&stop, async {
            polled.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert_eq!(result, Err((LimitFailure::Unsupported, None)));
        assert!(!polled.load(Ordering::SeqCst));
    }
    #[tokio::test]
    async fn revocation_drops_an_already_polled_http_operation() {
        let stop = CancellationToken::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let record = Arc::clone(&dropped);
        let (started, ready) = tokio::sync::oneshot::channel();
        let reading = async move {
            let _finished = Dropped(record);
            started.send(()).unwrap();
            std::future::pending::<Result<(), (LimitFailure, Option<DateTime<Utc>>)>>().await
        };
        let revoke = async {
            ready.await.unwrap();
            stop.cancel();
        };
        let (result, ()) = tokio::time::timeout(CANCELLATION_TEST_BUDGET, async {
            tokio::join!(cancellable(&stop, reading), revoke)
        })
        .await
        .unwrap();
        assert_eq!(result, Err((LimitFailure::Unsupported, None)));
        assert!(dropped.load(Ordering::SeqCst));
    }
}
