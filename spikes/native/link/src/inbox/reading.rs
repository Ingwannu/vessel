//! Passive clear rechecks pending requests before marking only the observed
//! watermark. A newly arriving record lies beyond it and stays unread.
use super::node::problem;
use super::transport::Client;
use vessel_app_view::inbox::Problem;
use vessel_proto::{EventSeq, SessionId};
pub(super) async fn clear(
    client: &Client,
    session: SessionId,
    through: EventSeq,
) -> Result<(), Problem> {
    if vessel_proto::Method::ReadInboxThrough
        .since()
        .is_some_and(|since| since <= client.protocol())
    {
        return client
            .call(vessel_proto::inbox_attention::ReadInboxThrough { session, through })
            .await
            .map(|_ack| ())
            .map_err(problem);
    }
    // Existing Operate peers retain the old safe watermark path. Read-only
    // clients are never promoted to Operate while waiting for the new ingress.
    let authority = client
        .call(vessel_proto::GetDeviceSelf {})
        .await
        .map_err(problem)?;
    if !authority.scopes.contains(&vessel_proto::Scope::Operate) {
        return Err(Problem::Refused(vessel_proto::ErrorCode::Unsupported));
    }
    let current = client
        .call(vessel_proto::conversation::GetConversation { session })
        .await
        .map_err(problem)?;
    if !current.permissions.is_empty() || !current.questions.is_empty() {
        return Err(Problem::Refused(vessel_proto::ErrorCode::Conflict));
    }
    client
        .call(vessel_proto::conversation::MarkSession {
            session,
            mark: vessel_proto::Mark::Read { through },
        })
        .await
        .map(|_info| ())
        .map_err(problem)
}
