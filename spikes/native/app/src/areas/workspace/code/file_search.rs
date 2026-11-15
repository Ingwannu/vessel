//! Shared file-index acquisition for the explorer, command center and
//! composer. Consumers own their presentation and generation checks;
//! this owner routes exact node/root/query replies without a second index.

use std::collections::HashMap;
use std::num::NonZeroU32;

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::FileAsk;
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::{file_query::FileQuery, file_scope::FileScope};
use vessel_proto::workspace::{FindFiles, FoundFiles, SearchKey};

use crate::shell::Shell;

use vessel_app_view::workspace::find::{
    FILE_SEARCH_LIMIT, FileSearchRefusal, MAX_FILE_SEARCH_OWNERS,
};

/// A reusable validated file search; no ambient cwd or selected tab is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSearchRequest {
    /// Exact original connection and physical root; never resolved by name later.
    pub(crate) scope: FileScope,
    /// Bounded protocol query.
    pub(crate) query: FileQuery,
    /// A stable per-field key allocated by `Shell::file_search_key`.
    pub(crate) key: SearchKey,
    /// Requested result count, bounded by `FILE_SEARCH_LIMIT`.
    pub(crate) limit: NonZeroU32,
}

/// Consumers register a UI callback, not a network or filesystem handler.
pub(crate) type FileSearchReceiver =
    fn(&mut Shell, &FileSearchRequest, Result<FoundFiles, Problem>, &mut Context<'_, Shell>);

pub(super) struct RegisteredSearch {
    pub(super) ticket: FileTicket,
    pub(super) request: FileSearchRequest,
    pub(super) receive: FileSearchReceiver,
}

#[derive(Default)]
pub(super) struct Searches(pub(super) HashMap<SearchKey, RegisteredSearch>);

impl Shell {
    /// Allocate from one window-wide sequence, avoiding cross-field
    /// cancellation between the explorer, composer and command center.
    pub(crate) fn file_search_key(&mut self) -> SearchKey {
        self.areas.workspace.code.find_keys.next()
    }

    /// Register one field's newest request and send through the existing link.
    pub(crate) fn search_workspace_files(
        &mut self,
        request: FileSearchRequest,
        receive: FileSearchReceiver,
    ) -> Result<(), FileSearchRefusal> {
        if request.limit.get() > FILE_SEARCH_LIMIT {
            return Err(FileSearchRefusal::Limit);
        }
        let ticket = self
            .areas
            .workspace
            .code
            .tickets
            .issue()
            .map_err(FileSearchRefusal::Exhausted)?;
        let searches = &mut self.areas.workspace.code.searches.0;
        if searches.len() >= MAX_FILE_SEARCH_OWNERS && !searches.contains_key(&request.key) {
            return Err(FileSearchRefusal::Capacity);
        }
        let ask = match &request.query {
            FileQuery::Matching(query) => FileAsk::TrackedFind {
                ask: FindFiles {
                    root: request.scope.root(),
                    query: query.clone(),
                    key: Some(request.key),
                    limit: Some(request.limit.get()),
                },
                ticket,
            },
            FileQuery::All => FileAsk::List {
                ask: vessel_proto::workspace::ListDir {
                    root: request.scope.root(),
                    dir: vessel_proto::workspace::RelPath::root(),
                    ignored: false,
                },
                ticket,
            },
        };
        let scope = request.scope.clone();
        let _replaced = searches.insert(
            request.key,
            RegisteredSearch {
                ticket,
                request,
                receive,
            },
        );
        self.send_file_scope(scope, ask);
        Ok(())
    }

    /// Closing/clearing a field invalidates replies already in flight.
    pub(crate) fn cancel_file_search(&mut self, key: SearchKey) {
        let _cancelled = self.areas.workspace.code.searches.0.remove(&key);
    }

    pub(in crate::areas::workspace) fn file_search_answered(
        &mut self,
        scope: &FileScope,
        ask: &FindFiles,
        ticket: FileTicket,
        result: Result<FoundFiles, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if result.as_ref().is_ok_and(|found| found.superseded) {
            return;
        }
        if let Some((request, receive)) = self.take_file_search(scope, ask, ticket) {
            receive(self, &request, result, cx);
        }
    }

    fn take_file_search(
        &mut self,
        scope: &FileScope,
        ask: &FindFiles,
        ticket: FileTicket,
    ) -> Option<(FileSearchRequest, FileSearchReceiver)> {
        let key = ask.key?;
        let searches = &mut self.areas.workspace.code.searches.0;
        let pending = searches.get(&key)?;
        if pending.ticket != ticket
            || pending.request.scope != *scope
            || pending.request.scope.root() != ask.root
            || pending.request.query != FileQuery::Matching(ask.query.clone())
        {
            return None;
        }
        searches
            .remove(&key)
            .map(|pending| (pending.request, pending.receive))
    }
}

#[cfg(test)]
#[path = "file_search_tests.rs"]
mod tests;
