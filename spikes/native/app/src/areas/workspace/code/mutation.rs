//! Root-confined mutations hold affected clean buffers readonly until
//! their exact acknowledgement. Dirty or pending I/O refuses the operation
//! rather than losing content during a rename/delete race.

use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::{FileAsk, PathChange};
use vessel_app_view::workspace::explorer::replace_prefix;
use vessel_app_view::workspace::file_scope::FileScope;
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::NodeId;
use vessel_proto::workspace::RelPath;

use super::buffers::FileKey;
use super::{CodePanel, Viewer};
use crate::shell::Shell;

pub(super) struct PendingMutation {
    pub(super) scope: FileScope,
    pub(super) uncertain: bool,
    ticket: FileTicket,
    change: PathChange,
}

fn prefix(change: &PathChange) -> Option<&RelPath> {
    match change {
        PathChange::Rename(ask) => Some(&ask.from),
        PathChange::Delete(ask) => Some(&ask.path),
        PathChange::CreateDir(_) | PathChange::CreateFile { .. } => None,
    }
}

fn within(path: &RelPath, prefix: &RelPath) -> bool {
    path == prefix
        || path
            .as_str()
            .strip_prefix(prefix.as_str())
            .is_some_and(|tail| tail.starts_with('/'))
}

fn affected(key: &FileKey, node: NodeId, change: &PathChange) -> bool {
    key.node == node
        && key.root == change.root()
        && prefix(change).is_some_and(|prefix| within(&key.path, prefix))
}

fn busy(viewer: &Viewer) -> bool {
    !viewer.can_evict()
}

impl CodePanel {
    fn affected_busy(&self, node: NodeId, change: &PathChange) -> bool {
        let destination_open = |key: &FileKey| {
            key.node == node
                && key.root == change.root()
                && matches!(change, PathChange::Rename(ask) if within(&key.path, &ask.to))
        };
        // Even a clean stale destination has its own lifetime. Never
        // replace its retained key by collecting a renamed source over it.
        if self.file_key().as_ref().is_some_and(destination_open)
            || self.files.retained.keys().any(destination_open)
        {
            return true;
        }
        self.file_key()
            .filter(|key| affected(key, node, change))
            .and(self.files.viewer.as_ref())
            .is_some_and(busy)
            || self
                .files
                .retained
                .iter()
                .any(|(key, viewer)| affected(key, node, change) && busy(viewer))
    }

    fn hold_mutation(
        &mut self,
        node: NodeId,
        change: &PathChange,
        hold: bool,
        cx: &mut Context<'_, Shell>,
    ) {
        let active = self
            .file_key()
            .is_some_and(|key| affected(&key, node, change));
        let viewers = self.files.viewer.iter_mut().filter(|_| active).chain(
            self.files
                .retained
                .iter_mut()
                .filter_map(|(key, viewer)| affected(key, node, change).then_some(viewer)),
        );
        for viewer in viewers {
            if let Some(editor) = viewer
                .edit
                .as_ref()
                .and_then(|editing| editing.editor.as_ref())
            {
                editor.set_editability(
                    if hold {
                        vessel_app_kit::Editability::ReadOnly
                    } else {
                        vessel_app_kit::Editability::Editable
                    },
                    cx,
                );
            }
        }
    }

    pub(super) fn file_mutating(&self, key: &FileKey) -> bool {
        self.mutation
            .as_ref()
            .is_some_and(|pending| affected(key, pending.scope.node(), &pending.change))
    }
}

impl Shell {
    pub(super) fn refuse_file_mutation(&mut self, key: &FileKey) -> bool {
        if self.areas.workspace.code.file_mutating(key) {
            self.areas.workspace.code.files.actions.refused = Some(
                if self
                    .areas
                    .workspace
                    .code
                    .mutation
                    .as_ref()
                    .is_some_and(|pending| pending.uncertain)
                {
                    self.words().labels().editor_save_failed
                } else {
                    self.words().labels().file_busy
                }
                .to_owned(),
            );
            true
        } else {
            false
        }
    }
    pub(super) fn begin_file_mutation_for(
        &mut self,
        scope: FileScope,
        change: PathChange,
        cx: &mut Context<'_, Self>,
    ) {
        if scope.root() != change.root() {
            return;
        }
        let node = scope.node();
        let panel = &self.areas.workspace.code;
        if panel.mutation.is_some() || panel.affected_busy(node, &change) {
            self.areas.workspace.code.files.actions.refused =
                Some(self.words().labels().file_busy.to_owned());
            return;
        }
        let Some(ticket) = self.next_file_ticket() else {
            return;
        };
        let panel = &mut self.areas.workspace.code;
        panel.hold_mutation(node, &change, true, cx);
        panel.mutation = Some(PendingMutation {
            scope: scope.clone(),
            uncertain: false,
            change: change.clone(),
            ticket,
        });
        self.send_file_scope(scope, FileAsk::Path { change, ticket });
    }

    pub(super) fn file_mutation_answered_for(
        &mut self,
        scope: &FileScope,
        change: &PathChange,
        ticket: FileTicket,
        result: Result<(), Problem>,
        dispatched: bool,
        cx: &mut Context<'_, Self>,
    ) {
        let uncertain_words = self.words().labels().editor_save_failed.to_owned();
        let panel = &mut self.areas.workspace.code;
        let owns = panel.mutation.as_ref().is_some_and(|pending| {
            pending.scope == *scope && pending.ticket == ticket && pending.change == *change
        });
        if !owns {
            return;
        }
        if dispatched
            && result.as_ref().is_err_and(|problem| {
                problem.code() != Some(vessel_proto::ErrorCode::Conflict)
                    && !vessel_app_view::workspace::save::known_not_written(problem)
            })
        {
            if let Some(pending) = panel.mutation.as_mut() {
                pending.uncertain = true;
            }
            panel.files.actions.refused = Some(uncertain_words);
            cx.notify();
            return;
        }
        let Some(pending) = panel.mutation.take_if(|pending| {
            pending.scope == *scope && pending.ticket == ticket && pending.change == *change
        }) else {
            return;
        };
        panel.hold_mutation(pending.scope.node(), change, false, cx);
        if result.is_ok() {
            self.rewrite_mutated_resources(pending.scope.node(), change, cx);
        }
        // An old workspace's acknowledgement may settle its buffers but
        // must never clear the new workspace's inline form or file viewer.
        if self.areas.workspace.code.scope().as_ref() == Some(scope) {
            self.path_changed(change, result);
            self.remember_file_tree();
        }
        cx.notify();
    }

    fn rewrite_mutated_resources(
        &mut self,
        node: NodeId,
        change: &PathChange,
        cx: &Context<'_, Self>,
    ) {
        let panel = &mut self.areas.workspace.code;
        let active = panel
            .file_key()
            .is_some_and(|key| affected(&key, node, change));
        if active && let Some(viewer) = panel.files.viewer.take() {
            panel.files.viewer = rewrite_viewer(viewer, change, cx);
        }
        panel.files.retained = panel
            .files
            .retained
            .drain()
            .filter_map(|(mut key, viewer)| {
                if !affected(&key, node, change) {
                    return Some((key, viewer));
                }
                let viewer = rewrite_viewer(viewer, change, cx)?;
                key.path = viewer.path.clone();
                Some((key, viewer))
            })
            .collect();
        if panel
            .scope()
            .is_none_or(|scope| scope.node() != node || scope.root() != change.root())
        {
            return;
        }
        let Some(prefix) = prefix(change) else {
            return;
        };
        panel
            .files
            .listing_tickets
            .retain(|path, _| !within(path, prefix));
        let renamed = |path: RelPath| -> Option<RelPath> {
            if !within(&path, prefix) {
                return Some(path);
            }
            match change {
                PathChange::Rename(ask) => replace_prefix(&path, &ask.from, &ask.to),
                PathChange::Delete(_) => None,
                PathChange::CreateDir(_) | PathChange::CreateFile { .. } => Some(path),
            }
        };
        panel.files.modified = panel
            .files
            .modified
            .drain()
            .filter_map(|(path, timestamp)| renamed(path).map(|path| (path, timestamp)))
            .collect();
        panel.files.open = panel.files.open.drain().filter_map(renamed).collect();
        panel.files.listings = panel
            .files
            .listings
            .drain()
            .filter_map(|(path, listing)| {
                if within(&path, prefix)
                    && matches!(
                        listing,
                        vessel_app_view::workspace::file_tree::Listing::Loading
                    )
                {
                    return None;
                }
                renamed(path).map(|path| (path, listing))
            })
            .collect();
        panel.files.relist();
    }
}

fn rewrite_viewer(
    mut viewer: Viewer,
    change: &PathChange,
    cx: &Context<'_, Shell>,
) -> Option<Viewer> {
    match change {
        PathChange::Delete(_) => return None,
        PathChange::Rename(ask) => viewer.path = replace_prefix(&viewer.path, &ask.from, &ask.to)?,
        PathChange::CreateDir(_) | PathChange::CreateFile { .. } => return Some(viewer),
    }
    // Editor callbacks capture a path key. Rebuild only the controls after
    // a rename; the clean model and cursor intent remain resource-owned.
    if let Some(editing) = viewer.edit.as_mut() {
        if let Some(editor) = editing.editor.as_ref() {
            let (line, column) = editor.cursor(cx);
            viewer.goto = Some(
                vessel_app_view::workspace::file_location::FileLocation::point(
                    vessel_proto::lsp::Position {
                        line: std::num::NonZeroU32::new(line).unwrap_or(std::num::NonZeroU32::MIN),
                        column: std::num::NonZeroU32::new(column)
                            .unwrap_or(std::num::NonZeroU32::MIN),
                    },
                ),
            );
        }
        editing.editor = None;
        editing.find = None;
        editing.status = None;
    }
    viewer.intelligence = super::intelligence::State::Idle;
    Some(viewer)
}

#[cfg(test)]
#[path = "mutation_tests.rs"]
mod tests;
