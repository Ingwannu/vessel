//! Passive clear rechecks pending requests before marking only the observed
//! watermark. A newly arriving record lies beyond it and stays unread.
use super::node::problem;
use vessel_app_view::inbox::Problem;
use vessel_client::NodeClient;
use vessel_proto::{EventSeq, SessionId};
pub(super) async fn clear(
    client: &NodeClient,
    session: SessionId,
    through: EventSeq,
) -> Result<(), Problem> {
    let current = client.conversation(session).await.map_err(problem)?;
    if !current.permissions.is_empty() || !current.questions.is_empty() {
        return Err(Problem::Refused(vessel_proto::ErrorCode::Conflict));
    }
    client
        .mark(session, vessel_proto::Mark::Read { through })
        .await
        .map(|_info| ())
        .map_err(problem)
}
