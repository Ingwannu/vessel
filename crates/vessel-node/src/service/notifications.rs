//! Local recipients are authenticated connections, never user-supplied device
//! IDs. Terminal producers reuse the terminal ownership checks and info watch.
use super::{Connection, NodeService};
use crate::principal::Principal;
use vessel_proto::notifications::{
    ClaimNotification, ClearTerminalAttention, NotificationClaim, NotificationSource,
    ReportTerminalActivity, TerminalAttention,
};
use vessel_proto::{ErrorCode, Scope, WireError};
use vessel_store::{NotificationAuthority, PushAttention};

fn authority(principal: &Principal) -> Result<NotificationAuthority, WireError> {
    match principal {
        Principal::Owner => Ok(NotificationAuthority::Owner),
        Principal::Device { id, scopes } if scopes.contains(&Scope::Read) => {
            Ok(NotificationAuthority::Device(*id))
        }
        Principal::Device { .. } | Principal::Agent { .. } | Principal::Delegated { .. } => {
            Err(WireError::new(
                ErrorCode::Forbidden,
                "only a person's directly connected app can claim local notification delivery",
            ))
        }
    }
}
fn activity_error(error: vessel_session::terminals::AttentionError) -> WireError {
    let code = match error {
        vessel_session::terminals::AttentionError::Stale
        | vessel_session::terminals::AttentionError::Ended => ErrorCode::Conflict,
        vessel_session::terminals::AttentionError::Exhausted => ErrorCode::Unavailable,
    };
    WireError::new(code, error.to_string())
}
impl NodeService {
    pub(crate) async fn claim_notification(
        &self,
        connection: &Connection,
        request: ClaimNotification,
    ) -> Result<NotificationClaim, WireError> {
        let authority = authority(connection.principal())?;
        tokio::select! {
            biased;
            () = connection.outbound.closed() => Err(WireError::new(ErrorCode::Unavailable,"notification connection closed; keep the inbox open")),
            claim = self.claim_connected_notification(connection, authority, request.source) => claim,
        }
    }
    async fn claim_connected_notification(
        &self,
        connection: &Connection,
        authority: NotificationAuthority,
        source: NotificationSource,
    ) -> Result<NotificationClaim, WireError> {
        match source {
            NotificationSource::Session { key, cursor } => {
                self.push
                    .claim_session(
                        connection.presence_id,
                        authority,
                        PushAttention { key, cursor },
                    )
                    .await
            }
            NotificationSource::Terminal { terminal, revision } => {
                let entry = self
                    .terminal_for(connection.principal(), terminal)
                    .map_err(|error| error.wire())?;
                if let NotificationAuthority::Device(device) = authority {
                    let granted = self
                        .devices
                        .scopes_now(device)
                        .await
                        .map_err(|error| super::wire(&error))?;
                    if !granted.is_some_and(|scopes| scopes.contains(&Scope::Read)) {
                        return Err(WireError::new(
                            ErrorCode::Forbidden,
                            "this device no longer has notification read authority",
                        ));
                    }
                }
                let target = vessel_proto::push::PushFocus::Terminal { terminal };
                let recipient = self.push.recipient_for(connection.presence_id, target);
                if recipient != NotificationClaim::Admitted {
                    return Ok(recipient);
                }
                Ok(entry.handle.claim_attention(revision))
            }
        }
    }
    pub(crate) async fn clear_terminal_attention(
        &self,
        connection: &Connection,
        request: ClearTerminalAttention,
    ) -> Result<TerminalAttention, WireError> {
        if connection.principal().agent().is_some() {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "only a person clears terminal attention",
            ));
        }
        let entry = self
            .terminal_for(connection.principal(), request.terminal)
            .map_err(|error| error.wire())?;
        entry
            .handle
            .clear_attention(request.revision)
            .map_err(activity_error)
    }
    pub(crate) async fn report_terminal_activity(
        &self,
        connection: &Connection,
        request: ReportTerminalActivity,
    ) -> Result<TerminalAttention, WireError> {
        let entry = self
            .terminal_for(connection.principal(), request.terminal)
            .map_err(|error| error.wire())?;
        entry
            .handle
            .report_activity(request.revision, request.signal)
            .map_err(activity_error)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn claim_authority_never_borrows_a_delegates_or_agents_connection_identity() {
        assert!(matches!(
            authority(&Principal::Owner),
            Ok(NotificationAuthority::Owner)
        ));
        for scopes in [vec![], vec![Scope::Operate]] {
            assert_eq!(
                authority(&Principal::Device {
                    id: vessel_proto::DeviceId::random(),
                    scopes
                })
                .unwrap_err()
                .code,
                ErrorCode::Forbidden
            );
        }
        let agent = Principal::Agent {
            node: vessel_proto::NodeId::random(),
            session: vessel_proto::SessionId::random(),
            scopes: Scope::ALL.to_vec(),
        };
        assert_eq!(authority(&agent).unwrap_err().code, ErrorCode::Forbidden);
        let delegated = Principal::delegated(
            vessel_proto::DeviceId::random(),
            &Scope::ALL,
            vessel_proto::Origin {
                node: vessel_proto::NodeId::random(),
                by: vessel_proto::OriginBy::Owner,
                scopes: Scope::ALL.to_vec(),
            },
            vessel_proto::NodeId::random(),
        );
        assert_eq!(
            authority(&delegated).unwrap_err().code,
            ErrorCode::Forbidden
        );
    }
}
