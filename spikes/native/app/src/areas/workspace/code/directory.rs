//! Directory sorting consumes one accepted snapshot, never per-entry stat calls.
//! Old peers retain the existing requested-parent list without invented dates.
use crate::shell::Shell;
use std::collections::HashMap;
use vessel_app_link::workspace::FileAsk;
use vessel_app_view::workspace::{
    file_scope::FileScope, file_ticket::FileTicket, problem::Problem,
};
use vessel_proto::workspace::directory_metadata::{
    FileModifiedMillis, IgnoreState, IgnoredEntries, InspectDirectory, InspectedDirectory,
};
use vessel_proto::workspace::panel_files::FILE_PANELS_PROTOCOL;
use vessel_proto::workspace::{DirEntry, DirListing, ListDir, RelPath};

impl Shell {
    pub(super) fn file_panels_available(&self, scope: &FileScope) -> bool {
        let Some(since) = FILE_PANELS_PROTOCOL else {
            return false;
        };
        self.fleet
            .machines
            .iter()
            .find(|machine| machine.name == scope.connection().machine().as_str())
            .is_some_and(|machine| {
                machine.connection_context().as_ref() == Some(scope.connection())
                    && machine.protocol.is_some_and(|version| version >= since)
            })
    }
    pub(super) fn request_directory(&mut self, scope: FileScope, ask: ListDir, ticket: FileTicket) {
        let request = if self.file_panels_available(&scope) {
            FileAsk::Inspect {
                ask: InspectDirectory {
                    root: ask.root,
                    directory: ask.dir,
                    ignored: if ask.ignored {
                        IgnoredEntries::Include
                    } else {
                        IgnoredEntries::Exclude
                    },
                },
                ticket,
            }
        } else {
            FileAsk::List { ask, ticket }
        };
        self.send_file_scope(scope, request);
    }
    pub(super) fn directory_inspected(
        &mut self,
        scope: &FileScope,
        ask: &InspectDirectory,
        ticket: FileTicket,
        result: Result<InspectedDirectory, Problem>,
    ) {
        let files = &self.areas.workspace.code.files;
        if self.areas.workspace.code.scope().as_ref() != Some(scope)
            || files.listing_tickets.get(&ask.directory) != Some(&ticket)
        {
            return;
        }
        let legacy = ListDir {
            root: ask.root,
            dir: ask.directory.clone(),
            ignored: ask.ignored == IgnoredEntries::Include,
        };
        let projected = result.and_then(|snapshot| project(&ask.directory, snapshot));
        let (listing, modified) = match projected {
            Ok((listing, modified)) => (Ok(listing), modified),
            Err(problem) => (Err(problem), HashMap::new()),
        };
        self.folder_read(
            scope.connection().machine().as_str(),
            &legacy,
            ticket,
            listing,
        );
        self.areas.workspace.code.files.modified.extend(modified);
        self.areas.workspace.code.files.relist();
    }
}

type Projected = (DirListing, HashMap<RelPath, FileModifiedMillis>);
fn project(directory: &RelPath, snapshot: InspectedDirectory) -> Result<Projected, Problem> {
    let mut entries = Vec::with_capacity(snapshot.entries.len());
    let mut modified = HashMap::new();
    for entry in snapshot.entries {
        // Never reinterpret an unexpected descendant as a direct child; its
        // path and the basename used by the existing tree must name one entry.
        if entry.path.parent().as_ref() != Some(directory) {
            return Err(invalid_snapshot());
        }
        let Some(name) = entry.path.file_name() else {
            return Err(invalid_snapshot());
        };
        entries.push(DirEntry {
            name: name.to_owned(),
            kind: entry.kind,
            size: entry.size.map(|size| size.0),
            ignored: entry.ignored == IgnoreState::Ignored,
        });
        if let Some(timestamp) = entry.modified {
            let _previous = modified.insert(entry.path, timestamp);
        }
    }
    Ok((
        DirListing {
            entries,
            more: snapshot.more,
        },
        modified,
    ))
}

fn invalid_snapshot() -> Problem {
    Problem::Node {
        code: vessel_proto::ErrorCode::InvalidRequest,
        message: String::new(),
    }
}
#[cfg(test)]
mod tests;
