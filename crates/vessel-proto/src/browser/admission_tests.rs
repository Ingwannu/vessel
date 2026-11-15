//! Pending rows must not become callable merely because a value type exists.

use super::*;
use crate::{Method, MethodShape, Request, Scope, WorkspaceId};

#[test]
fn browser_methods_have_one_pending_version_and_explicit_authority_and_shape() {
    for (method, scope, shape) in [
        (Method::BrowserCall, Scope::Operate, MethodShape::Unary),
        (Method::BrowserControl, Scope::Operate, MethodShape::Unary),
        (Method::WatchBrowser, Scope::Operate, MethodShape::Stream),
        (
            Method::ClearBrowserProfile,
            Scope::Admin,
            MethodShape::Unary,
        ),
    ] {
        assert_eq!(method.since(), BROWSER_PROTOCOL);
        assert_eq!(method.scope(), Some(scope));
        assert_eq!(method.shape(), shape);
        assert!(!method.forwardable());
    }
    let call = BrowserCall {
        command: BrowserCommand::ListTabs(NoBrowserArgs {}),
    }
    .into_call();
    assert_eq!(call.required_protocol(), BROWSER_PROTOCOL);
    let control = BrowserControl {
        workspace: WorkspaceId::random(),
        opening_session: None,
        command: BrowserControlCommand::Drive(BrowserCommand::ListTabs(NoBrowserArgs {})),
    }
    .into_call();
    assert_eq!(control.required_protocol(), BROWSER_PROTOCOL);
}

#[test]
fn typed_browser_refusals_round_trip_without_page_diagnostics() {
    let failure = BrowserFailure::TimedOut {
        effect: BrowserEffect::Uncertain,
    };
    let error = crate::WireError {
        code: failure.code(),
        message: failure.to_string(),
        detail: Some(crate::ErrorDetail::Browser { reason: failure }),
    };
    let decoded: crate::WireError =
        serde_json::from_str(&serde_json::to_string(&error).unwrap()).unwrap();
    assert_eq!(decoded, error);
}
