//! The persistent provider client resolves names only through the node's
//! injected resolver (ADR 0104), and refuses to exist without one.
use super::{
    LimitContext,
    http::{self, Endpoint, HttpError},
};
use reqwest::{
    dns::{Addrs, Name, Resolve, Resolving},
    header::HeaderValue,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use vessel_proto::limits::LimitFailure;

/// Records every name reqwest hands to the injected hook and refuses it, so
/// no request can leave the machine and a default resolver would show up as
/// an unrecorded connection attempt instead of this refusal.
#[derive(Default)]
struct Refusing(Mutex<Vec<String>>);
impl Resolve for Refusing {
    fn resolve(&self, name: Name) -> Resolving {
        self.0.lock().unwrap().push(name.as_str().to_owned());
        Box::pin(async {
            Err::<Addrs, _>(Box::<dyn std::error::Error + Send + Sync>::from(
                "refused by the test resolver",
            ))
        })
    }
}

#[tokio::test]
async fn a_context_without_tracked_resolution_fails_closed_before_any_client() {
    let context = LimitContext::new();
    let admission = context.admit().unwrap();
    let refused = context.client(&admission).await.unwrap_err();
    assert!(matches!(refused, HttpError::Unresolved));
    assert_eq!(http::safe(&refused), (LimitFailure::Unsupported, None));
    // Nothing was initialised: a later call refuses the same way.
    assert!(matches!(
        context.client(&admission).await,
        Err(HttpError::Unresolved)
    ));
}

#[tokio::test]
async fn the_one_client_sends_every_vendor_name_through_the_injected_hook() {
    vessel_testkit::within(Duration::from_secs(30), async {
        let witness = Arc::new(Refusing::default());
        let shared = Arc::clone(&witness);
        let resolver: Arc<dyn Resolve> = shared;
        let context = LimitContext::resolving(resolver);
        let admission = context.admit().unwrap();
        let client = context.client(&admission).await.unwrap();
        let authorization = HeaderValue::from_static("Bearer synthetic-offline-token");
        let claude = client
            .get(Endpoint::ClaudeUsage, &authorization, None)
            .await
            .unwrap_err();
        assert!(matches!(claude, HttpError::Transport(_)));
        assert_eq!(http::safe(&claude), (LimitFailure::Unreachable, None));
        // Saved and additional reads share this same client for the context's
        // lifetime; the second read builds nothing new.
        let again = context.client(&admission).await.unwrap();
        assert!(std::ptr::eq(client, again));
        let codex = again
            .get(Endpoint::CodexUsage, &authorization, None)
            .await
            .unwrap_err();
        assert!(matches!(codex, HttpError::Transport(_)));
        assert_eq!(
            *witness.0.lock().unwrap(),
            ["api.anthropic.com", "chatgpt.com"]
        );
    })
    .await
    .unwrap();
}
