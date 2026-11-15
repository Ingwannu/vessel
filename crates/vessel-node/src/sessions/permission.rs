//! Rich permission acknowledgement observes the exact committed resolution.
//! Sampling next_seq immediately after the driver reply would miss actor output.

use super::{Found, Sessions};
use crate::{error::ServiceError, persist};
use std::time::Duration;
use vessel_agent::{InputError, PermissionDispatch};
use vessel_proto::timeline::{
    PermissionResponseRefusal, RespondPermission, RespondPermissionReply,
};
use vessel_proto::{
    AgentEvent, EventSeq, PermissionId, PermissionOutcome, SessionEvent, SessionId,
};
use vessel_session::SessionError;

/// A response must not hold a connection call slot indefinitely after a lost acknowledgement.
const RESOLUTION_TIMEOUT: Duration = Duration::from_secs(30);
/// Bound each catch-up page rather than accumulating a private event stream.
const RESOLUTION_PAGE: u32 = 128;

impl Sessions {
    pub(crate) async fn respond_permission(
        &self,
        request: RespondPermission,
    ) -> Result<RespondPermissionReply, ServiceError> {
        let permission = request.permission;
        match tokio::time::timeout(RESOLUTION_TIMEOUT, self.respond_permission_inner(request)).await
        {
            Ok(reply) => reply,
            Err(_elapsed) => {
                Ok(vessel_proto::timeline::RespondPermissionReply::OutcomeUnknown { permission })
            }
        }
    }

    async fn respond_permission_inner(
        &self,
        request: RespondPermission,
    ) -> Result<RespondPermissionReply, ServiceError> {
        let permission = request.permission;
        let Found::Live {
            handle,
            mut durability,
        } = self.find(request.session).await?
        else {
            return Ok(RespondPermissionReply::Vanished { permission });
        };
        let from = handle.info().next_seq;
        let outcome = match handle
            .respond_permission(permission, request.response)
            .await
        {
            Ok(PermissionDispatch::Refused(reason)) => {
                return Ok(RespondPermissionReply::Refused { permission, reason });
            }
            Ok(PermissionDispatch::Applied(outcome)) => outcome,
            Err(SessionError::Input(InputError::PermissionNotFound(_))) => {
                return Ok(RespondPermissionReply::Vanished { permission });
            }
            Err(SessionError::Input(InputError::PermissionSettled(_))) => {
                return Ok(RespondPermissionReply::Conflict { permission });
            }
            Err(SessionError::Input(InputError::UnknownOption { .. })) => {
                return Ok(RespondPermissionReply::Refused {
                    permission,
                    reason: PermissionResponseRefusal::UnknownOption,
                });
            }
            Err(SessionError::Input(InputError::Closed)) => {
                return Ok(RespondPermissionReply::OutcomeUnknown { permission });
            }
            Err(error) => return Err(error.into()),
        };
        let observed = tokio::time::timeout(RESOLUTION_TIMEOUT, async {
            let Some((seq, resolved)) = self
                .permission_resolution(request.session, permission, from)
                .await?
            else {
                return Ok(None);
            };
            let Some(next) = seq.next() else {
                return Ok(None);
            };
            persist::wait_until(request.session, &mut durability, next).await?;
            Ok::<_, ServiceError>(Some((seq, resolved)))
        })
        .await;
        let (seq, resolved) = match observed {
            Ok(Ok(Some(record))) => record,
            Ok(Ok(None)) | Err(_) => {
                return Ok(RespondPermissionReply::OutcomeUnknown { permission });
            }
            Ok(Err(error)) => return Err(error),
        };
        if resolved != outcome {
            return Ok(RespondPermissionReply::Conflict { permission });
        }
        Ok(RespondPermissionReply::Settled {
            permission,
            outcome,
            seq,
        })
    }

    async fn permission_resolution(
        &self,
        session: SessionId,
        permission: PermissionId,
        mut cursor: EventSeq,
    ) -> Result<Option<(EventSeq, PermissionOutcome)>, ServiceError> {
        loop {
            let page = self
                .read_waiting(session, cursor, RESOLUTION_PAGE, RESOLUTION_TIMEOUT)
                .await?;
            for record in page.records {
                if let SessionEvent::Agent {
                    event: AgentEvent::PermissionResolved { id, outcome },
                } = record.event
                    && id == permission
                {
                    return Ok(Some((record.seq, outcome)));
                }
            }
            if page.next == cursor && page.at_end {
                return Ok(None);
            }
            cursor = page.next;
        }
    }
}
