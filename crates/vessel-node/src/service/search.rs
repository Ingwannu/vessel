//! Protocol 12's `session.search` (track A2, ADR 0037): one page of the
//! store's index of every session's messages. Routing only: what a
//! message is, the order and the bounds are the store's (`search`).

use vessel_proto::search::{SearchMessages, SearchPage};
use vessel_proto::{ErrorCode, WireError};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;

/// Searches a node runs at once. A page's work is bounded (the store's
/// walks), but one that finds nothing reads its whole bound (20,000
/// messages, about 130 ms measured, a second under load) on the blocking
/// pool; an agent's tool can ask in a loop. Beyond this, `unavailable`, and
/// the caller asks again.
pub(super) const AT_ONCE: usize = 2;

impl NodeService {
    /// `session.search`: a read, so not ownership-checked; an agent reads
    /// the fleet's sessions (ADR 0013) and may search them. A named
    /// session must exist (`not_found`), as for `session.read`; at most
    /// [`AT_ONCE`] run at a time (`unavailable`).
    pub(crate) async fn search_messages(
        &self,
        _: &Connection,
        request: SearchMessages,
    ) -> Result<SearchPage, WireError> {
        let Ok(_place) = self.searches.try_acquire() else {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "the node is running as many searches as it runs at once; ask again",
            ));
        };
        if let Some(session) = request.session {
            let _known = self
                .sessions
                .get(session)
                .await
                .map_err(|error| wire(&error))?;
        }
        self.sessions
            .store()
            .search(request)
            .await
            .map_err(|error| wire(&ServiceError::Store(error)))
    }
}
