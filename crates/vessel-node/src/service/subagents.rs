//! Human-only detach orchestration; storage and publication belong to Sessions.

use super::{Connection, NodeService, wire};
use crate::principal::Principal;
use vessel_proto::{
    ErrorCode, Method, WireError,
    subagents::{DetachSession, DetachedSession},
};

impl NodeService {
    pub(crate) async fn detach_session(
        &self,
        connection: &Connection,
        request: DetachSession,
    ) -> Result<DetachedSession, WireError> {
        person(connection.principal())?;
        let mut result = self
            .sessions
            .detach(request.session)
            .await
            .map_err(|error| wire(&error))?;
        result.session = Box::new(self.with_workspace(*result.session));
        Ok(result)
    }
}

fn person(principal: &Principal) -> Result<(), WireError> {
    principal.permit(Method::DetachSession)?;
    if principal.agent().is_some() {
        return Err(WireError::new(
            ErrorCode::Forbidden,
            "only a person may detach a managed child",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope, SessionId};

    #[test]
    fn only_humans_with_operate_can_detach_including_delegated_origins() {
        assert!(person(&Principal::Owner).is_ok());
        for scopes in [vec![Scope::Operate], vec![Scope::Read]] {
            let permitted = scopes.contains(&Scope::Operate);
            let device = Principal::Device {
                id: DeviceId::random(),
                scopes: scopes.clone(),
            };
            assert_eq!(person(&device).is_ok(), permitted);
            let origin = Origin {
                node: NodeId::random(),
                by: OriginBy::Owner,
                scopes: scopes.clone(),
            };
            let delegated =
                Principal::delegated(DeviceId::random(), &scopes, origin, NodeId::random());
            assert_eq!(person(&delegated).is_ok(), permitted);
        }
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        };
        assert_eq!(person(&agent).unwrap_err().code, ErrorCode::Forbidden);
        let origin = Origin {
            node: NodeId::random(),
            by: OriginBy::Agent {
                session: SessionId::random(),
                depth: 0,
            },
            scopes: vec![Scope::Read, Scope::Operate],
        };
        let delegated = Principal::delegated(
            DeviceId::random(),
            &[Scope::Read, Scope::Operate],
            origin,
            NodeId::random(),
        );
        assert_eq!(person(&delegated).unwrap_err().code, ErrorCode::Forbidden);
    }
}
