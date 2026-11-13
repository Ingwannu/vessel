//! The store's side of `session.search` (protocol 12, schema 13, ADR
//! 0037): one page of the index, read on the blocking pool.

use vessel_proto::search::{SearchMessages, SearchPage};

use super::Store;
use crate::error::StoreError;
use crate::reader::Reader;

impl Store {
    /// One page of a search over every session's messages, or one
    /// session's (an unknown session has none). A message still being
    /// written is found once it ends.
    ///
    /// # Errors
    /// The read failed, or a stored message is corrupt.
    pub async fn search(&self, request: SearchMessages) -> Result<SearchPage, StoreError> {
        self.read(move |reader: &Reader| {
            reader.with(|connection| crate::search::query::search(connection, &request))
        })
        .await
    }
}

impl Store {
    /// Wait until every session's messages are in the search index: at
    /// once, unless sessions stored before it existed (or left behind by a
    /// failed write) are still being caught up while the writer is idle.
    /// For tests, and for saying so when a search may miss old messages.
    pub async fn indexed(&self) {
        let mut indexing = self.indexing.clone();
        // An error means the writer stopped: nothing will be indexed.
        let _settled = indexing.wait_for(|indexing| !indexing).await;
    }
}
