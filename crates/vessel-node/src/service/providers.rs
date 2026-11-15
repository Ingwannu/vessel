//! Provider authorization and wire projection. The owners retain native causes;
//! replies contain typed safe states and no command, environment or stderr text.

use super::{Connection, NodeService};
use crate::{ProviderCacheError, ProviderConfigurationError, ProviderDiscoveryError};
use vessel_proto::{
    ErrorCode, Method, WireError,
    providers::{
        DiagnosticRead, GetProviderDiagnostic, GetProviders, ProviderDiagnostic,
        ProviderSnapshotEntry, SetProviderEnabled,
    },
};

impl NodeService {
    pub(crate) async fn get_providers(
        &self,
        connection: &Connection,
        request: GetProviders,
    ) -> Result<Vec<ProviderSnapshotEntry>, WireError> {
        discovery_authority(connection.principal(), request.read)?;
        self.providers
            .snapshot(request.scope, request.read)
            .await
            .map_err(|error| discovery(&error))
    }

    pub(crate) async fn get_provider_diagnostic(
        &self,
        connection: &Connection,
        request: GetProviderDiagnostic,
    ) -> Result<ProviderDiagnostic, WireError> {
        discovery_authority(connection.principal(), request.read)?;
        self.providers
            .diagnostic(request.provider, request.scope, request.read)
            .await
            .map_err(|error| discovery(&error))
    }

    pub(crate) async fn set_provider_enabled(
        &self,
        connection: &Connection,
        request: SetProviderEnabled,
    ) -> Result<ProviderSnapshotEntry, WireError> {
        crate::added::person(connection.principal()).map_err(|_agent| {
            WireError::new(
                ErrorCode::Forbidden,
                "provider participation is changed by a person",
            )
        })?;
        self.providers
            .set_enabled(request.provider, request.enabled)
            .await
            .map_err(|error| configuration(&error))
    }
}

fn discovery_authority(
    principal: &crate::principal::Principal,
    read: DiagnosticRead,
) -> Result<(), WireError> {
    if read != DiagnosticRead::Cached {
        principal.permit(Method::CreateSession).map_err(|_scope| {
            WireError::new(
                ErrorCode::Forbidden,
                "provider discovery needs operate scope",
            )
        })?;
        crate::added::person(principal).map_err(|_agent| {
            WireError::new(
                ErrorCode::Forbidden,
                "provider discovery is a person's explicit request",
            )
        })?;
    }
    Ok(())
}

fn discovery(error: &ProviderDiscoveryError) -> WireError {
    match error {
        ProviderDiscoveryError::Cache(ProviderCacheError::ReplyTooLarge) => WireError::new(
            ErrorCode::Unavailable,
            "provider snapshot is too large; request individual provider diagnostics",
        ),
        ProviderDiscoveryError::Cache(ProviderCacheError::UnknownProvider) => {
            WireError::new(ErrorCode::NotFound, "provider is not configured")
        }
        ProviderDiscoveryError::Superseded => WireError::new(
            ErrorCode::Conflict,
            "provider configuration changed during discovery; read its current state",
        ),
        ProviderDiscoveryError::Deadline => WireError::new(
            ErrorCode::Unavailable,
            "provider snapshot discovery deadline elapsed",
        ),
        ProviderDiscoveryError::Cache(_) | ProviderDiscoveryError::Capacity => WireError::new(
            ErrorCode::Unavailable,
            "provider discovery capacity is full",
        ),
        ProviderDiscoveryError::Stopped => WireError::new(
            ErrorCode::Unavailable,
            "provider discovery owner is stopped",
        ),
    }
}

fn configuration(error: &ProviderConfigurationError) -> WireError {
    match error {
        ProviderConfigurationError::Unknown => {
            WireError::new(ErrorCode::NotFound, "provider is not configured")
        }
        ProviderConfigurationError::Invalid(_) => WireError::new(
            ErrorCode::InvalidRequest,
            "provider configuration generation is invalid",
        ),
        ProviderConfigurationError::Owner(error) => discovery(error),
        ProviderConfigurationError::Stopped => WireError::new(
            ErrorCode::Unavailable,
            "provider configuration owner is stopped",
        ),
        ProviderConfigurationError::Store(_) | ProviderConfigurationError::Task(_) => {
            WireError::new(
                ErrorCode::Unavailable,
                "provider configuration could not be kept; its previous generation remains applied",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::principal::Principal;
    use vessel_proto::{DeviceId, NodeId, Scope, SessionId};

    #[test]
    fn passive_reads_do_not_grant_active_discovery_to_readers_or_agents() {
        let read_device = Principal::Device {
            id: DeviceId::random(),
            scopes: vec![Scope::Read],
        };
        let operator = Principal::Device {
            id: DeviceId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        };
        let agent = Principal::Agent {
            node: NodeId::random(),
            session: SessionId::random(),
            scopes: vec![Scope::Read, Scope::Operate],
        };
        for principal in [&read_device, &operator, &agent, &Principal::Owner] {
            assert!(discovery_authority(principal, DiagnosticRead::Cached).is_ok());
        }
        for read in [DiagnosticRead::Discover, DiagnosticRead::Refresh] {
            for principal in [&read_device, &agent] {
                assert_eq!(
                    discovery_authority(principal, read).unwrap_err().code,
                    ErrorCode::Forbidden
                );
            }
            assert!(discovery_authority(&operator, read).is_ok());
            assert!(discovery_authority(&Principal::Owner, read).is_ok());
        }
    }
}
