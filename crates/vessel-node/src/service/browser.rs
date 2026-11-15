//! ADR 0078's pending entry points, with no product engine admitted yet.
//!
//! Human control is not an alternate agent tool path. Fleet delegation is
//! deliberately refused: a foreign session id cannot select a local profile.
//! Keeping absence as a typed refusal prevents a hidden model/test fixture
//! from appearing as a supported browser before runtime qualification.

use vessel_proto::browser::{
    BrowserCall, BrowserControl, BrowserFailure, BrowserResult, BrowserWatched,
    ClearBrowserProfile, WatchBrowser,
};
use vessel_proto::{Ack, ErrorCode, ErrorDetail, Method, NodeId, SessionId, WireError};

use super::{Connection, NodeService};
use crate::principal::Principal;

impl NodeService {
    #[expect(
        clippy::unused_async,
        reason = "the generated guard requires the uniform async handler shape"
    )]
    pub(crate) async fn browser_call(
        &self,
        connection: &Connection,
        _: BrowserCall,
    ) -> Result<BrowserResult, WireError> {
        let _session = local_agent(connection.principal(), self.lifecycle.info().id)?;
        // Persisted On is only a preference. No session-start tool offer or
        // qualified engine exists here, so guessing a pending method cannot enable it.
        Err(wire(BrowserFailure::Disabled))
    }

    #[expect(
        clippy::unused_async,
        reason = "the generated guard requires the uniform async handler shape"
    )]
    pub(crate) async fn browser_control(
        &self,
        connection: &Connection,
        _: BrowserControl,
    ) -> Result<BrowserResult, WireError> {
        human(connection.principal(), Method::BrowserControl)?;
        Err(wire(BrowserFailure::NoEngine))
    }

    #[expect(
        clippy::unused_async,
        reason = "the generated guard requires the uniform async handler shape"
    )]
    pub(crate) async fn watch_browser(
        &self,
        connection: &Connection,
        _: WatchBrowser,
    ) -> Result<BrowserWatched, WireError> {
        human(connection.principal(), Method::WatchBrowser)?;
        Err(wire(BrowserFailure::NoEngine))
    }

    #[expect(
        clippy::unused_async,
        reason = "the generated guard requires the uniform async handler shape"
    )]
    pub(crate) async fn clear_browser_profile(
        &self,
        connection: &Connection,
        _: ClearBrowserProfile,
    ) -> Result<Ack, WireError> {
        human(connection.principal(), Method::ClearBrowserProfile)?;
        Err(wire(BrowserFailure::NoEngine))
    }
}

pub(super) fn human(principal: &Principal, method: Method) -> Result<(), WireError> {
    principal.permit(method)?;
    match principal {
        Principal::Owner | Principal::Device { .. } => Ok(()),
        Principal::Agent { .. } | Principal::Delegated { .. } => Err(WireError::new(
            ErrorCode::Forbidden,
            "browser panel control requires a direct human connection",
        )),
    }
}

fn local_agent(principal: &Principal, here: NodeId) -> Result<SessionId, WireError> {
    principal.permit(Method::BrowserCall)?;
    if let Principal::Agent { node, session, .. } = principal
        && *node == here
    {
        return Ok(*session);
    }
    Err(WireError::new(
        ErrorCode::Forbidden,
        "browser tools require this node's own session credential",
    ))
}

fn wire(reason: BrowserFailure) -> WireError {
    WireError {
        code: reason.code(),
        message: reason.to_string(),
        detail: Some(ErrorDetail::Browser { reason }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{DeviceId, Origin, OriginBy, Scope};

    #[test]
    fn panel_paths_require_a_direct_human_with_their_own_scope() {
        let device = |scopes| Principal::Device {
            id: DeviceId::random(),
            scopes,
        };
        for method in [Method::BrowserControl, Method::WatchBrowser] {
            assert!(human(&Principal::Owner, method).is_ok());
            assert!(human(&device(vec![Scope::Operate]), method).is_ok());
            assert!(human(&device(vec![Scope::Read]), method).is_err());
        }
        assert!(human(&device(vec![Scope::Operate]), Method::ClearBrowserProfile).is_err());
        assert!(human(&device(vec![Scope::Admin]), Method::ClearBrowserProfile).is_ok());

        let delegated = Principal::delegated(
            DeviceId::random(),
            &Scope::ALL,
            Origin {
                node: NodeId::random(),
                by: OriginBy::Owner,
                scopes: Scope::ALL.to_vec(),
            },
            NodeId::random(),
        );
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: Scope::ALL.to_vec(),
        };
        for principal in [delegated, agent] {
            for method in [
                Method::BrowserControl,
                Method::WatchBrowser,
                Method::ClearBrowserProfile,
            ] {
                assert_eq!(
                    human(&principal, method).unwrap_err().code,
                    ErrorCode::Forbidden
                );
            }
        }
    }

    #[test]
    fn a_named_generic_browser_setting_is_direct_human_admin_only() {
        let device = |scopes| Principal::Device {
            id: DeviceId::random(),
            scopes,
        };
        assert!(human(&Principal::Owner, Method::SetNodeSettings).is_ok());
        assert!(human(&device(vec![Scope::Admin]), Method::SetNodeSettings).is_ok());
        assert!(human(&device(vec![Scope::Operate]), Method::SetNodeSettings).is_err());
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: Scope::ALL.to_vec(),
        };
        assert!(human(&agent, Method::SetNodeSettings).is_err());
        let delegated = Principal::delegated(
            DeviceId::random(),
            &Scope::ALL,
            Origin {
                node: NodeId::random(),
                by: OriginBy::Owner,
                scopes: Scope::ALL.to_vec(),
            },
            NodeId::random(),
        );
        assert!(human(&delegated, Method::SetNodeSettings).is_err());
    }

    #[test]
    fn an_agent_cannot_select_another_nodes_profile_or_delegate_its_identity() {
        let here = NodeId::random();
        let session = SessionId::random();
        let local = Principal::Agent {
            node: here,
            session,
            scopes: vec![Scope::Operate],
        };
        assert_eq!(local_agent(&local, here).unwrap(), session);
        assert!(local_agent(&local, NodeId::random()).is_err());
        assert!(local_agent(&Principal::Owner, here).is_err());
        let delegated = Principal::delegated(
            DeviceId::random(),
            &Scope::ALL,
            Origin {
                node: here,
                by: OriginBy::Agent { session, depth: 1 },
                scopes: vec![Scope::Operate],
            },
            NodeId::random(),
        );
        assert!(local_agent(&delegated, here).is_err());
    }

    #[test]
    fn no_engine_and_uncertain_effect_are_typed_and_round_trip() {
        for reason in [
            BrowserFailure::NoEngine,
            BrowserFailure::Disabled,
            BrowserFailure::TimedOut {
                effect: vessel_proto::browser::BrowserEffect::Uncertain,
            },
        ] {
            let error = wire(reason);
            let decoded: WireError =
                serde_json::from_str(&serde_json::to_string(&error).unwrap()).unwrap();
            assert_eq!(decoded.detail, Some(ErrorDetail::Browser { reason }));
        }
    }
}
