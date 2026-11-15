//! Initial picker rows reuse a single requested-root list, not a second index.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::workspace::{
    file_query::FileQuery, file_scope::FileScope, file_ticket::FileTicket, problem::Problem,
};
use vessel_proto::workspace::{DirListing, EntryKind, FoundFile, FoundFiles, ListDir, RelPath};
impl Shell {
    pub(super) fn file_listing_search_pending(
        &self,
        scope: &FileScope,
        ticket: FileTicket,
    ) -> bool {
        self.areas
            .workspace
            .code
            .searches
            .0
            .values()
            .any(|pending| {
                pending.ticket == ticket
                    && pending.request.scope == *scope
                    && pending.request.query == FileQuery::All
            })
    }
    pub(super) fn file_listing_search_answered(
        &mut self,
        scope: &FileScope,
        ask: &ListDir,
        ticket: FileTicket,
        result: Result<DirListing, Problem>,
        cx: &mut Context<'_, Self>,
    ) {
        if !ask.dir.is_root() || ask.root != scope.root() {
            return;
        }
        let searches = &mut self.areas.workspace.code.searches.0;
        let key = searches.iter().find_map(|(key, pending)| {
            (pending.ticket == ticket
                && pending.request.scope == *scope
                && pending.request.query == FileQuery::All)
                .then_some(*key)
        });
        let Some(key) = key else {
            return;
        };
        let Some(pending) = searches.remove(&key) else {
            return;
        };
        let result = result.map(|listing| root_files(listing, pending.request.limit));
        (pending.receive)(self, &pending.request, result, cx);
    }
}
fn root_files(listing: DirListing, limit: std::num::NonZeroU32) -> FoundFiles {
    let indexed = u32::try_from(
        listing
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File)
            .count(),
    )
    .unwrap_or(u32::MAX);
    let limit = usize::try_from(limit.get()).unwrap_or(usize::MAX);
    let matches = listing
        .entries
        .into_iter()
        .filter(|entry| entry.kind == EntryKind::File && !entry.ignored)
        .filter_map(|entry| RelPath::root().child(&entry.name).ok())
        .take(limit)
        .map(|path| FoundFile {
            path,
            positions: Vec::new(),
        })
        .collect();
    // A complete parent listing is not proof the workspace index was searched.
    FoundFiles {
        matches,
        indexed,
        complete: false,
        superseded: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::workspace::DirEntry;
    #[test]
    fn initial_parent_results_are_bounded_confined_and_honestly_incomplete() {
        let entry = |name: &str, kind, ignored| DirEntry {
            name: name.into(),
            kind,
            size: None,
            ignored,
        };
        let result = root_files(
            DirListing {
                entries: vec![
                    entry("src", EntryKind::Dir, false),
                    entry("../escape", EntryKind::File, false),
                    entry("ignored", EntryKind::File, true),
                    entry("first.rs", EntryKind::File, false),
                    entry("second.rs", EntryKind::File, false),
                ],
                more: false,
            },
            std::num::NonZeroU32::new(1).unwrap(),
        );
        assert_eq!(
            result
                .matches
                .iter()
                .map(|item| item.path.as_str())
                .collect::<Vec<_>>(),
            ["first.rs"]
        );
        assert!(
            !result.complete,
            "one complete parent is not a complete workspace index"
        );
        assert!(result.matches.iter().all(|item| item.positions.is_empty()));
    }
}
