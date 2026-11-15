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
    client
        .read_inbox_through(session, through)
        .await
        .map_err(problem)
}
