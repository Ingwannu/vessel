//! Source acquisition changes metadata on the existing owner before dispatch.
use super::{
    Place, Shell,
    shown_source::{DiffSource, RootReceipt},
};
use vessel_app_link::{Command, workspace::code_source::Request};
use vessel_app_view::workspace::{changes::FileState, file_scope::FileScope};
use vessel_proto::{
    SessionId,
    workspace::{GetChanges, GetFileDiff, RelPath},
};
impl Shell {
    pub(super) fn source_root(&mut self, machine: &str, session: SessionId) -> bool {
        let context = self
            .fleet
            .machines
            .iter()
            .find(|item| item.name == machine)
            .and_then(vessel_app_view::Machine::connection_context);
        let Some(context) = context else {
            return false;
        };
        let panel = &mut self.areas.workspace.code;
        panel.changes.sources.invalidate();
        let Ok(ticket) = panel.tickets.issue() else {
            if let Some(target) = panel.target.as_mut() {
                target.place = Place::Failed(String::new());
            }
            return true;
        };
        let Some(target) = panel.target.as_mut() else {
            return true;
        };
        target.admission = Some(RootReceipt {
            context: context.clone(),
            session,
            ticket,
            guard: None,
        });
        self.command(Command::CodeSource(Request::Root {
            context,
            session,
            ticket,
        }));
        true
    }
    pub(super) fn source_summary(&mut self) {
        let panel = &mut self.areas.workspace.code;
        panel.changes.sources.invalidate();
        let Some(target) = panel.target.as_ref() else {
            return;
        };
        let Some(origin) = target.admission.as_ref().filter(|origin| origin.current()) else {
            return;
        };
        let Place::Root(root) = target.place else {
            return;
        };
        let Ok(ticket) = panel.tickets.issue() else {
            return;
        };
        let request = Request::Summary {
            scope: FileScope::new(origin.context.clone(), root),
            root_ticket: origin.ticket,
            ticket,
            ask: GetChanges {
                root,
                scope: panel.changes.scope.scope(),
                ignore_whitespace: false,
            },
        };
        panel.changes.sources.requested = Some(request.clone());
        self.command(Command::CodeSource(request));
    }
    pub(super) fn source_diff(&mut self, path: &RelPath, from: Option<u32>) {
        let panel = &mut self.areas.workspace.code;
        let Some(summary) = panel.source().cloned() else {
            return;
        };
        let before = match panel.changes.states.get(path) {
            Some(FileState::Loading(_)) => return,
            Some(FileState::Read(body))
                if from.is_some()
                    && panel.diff_source(path).is_some()
                    && matches!(body, vessel_app_view::workspace::diff_rows::DiffBody::Rows { more, .. } if *more == from) =>
            {
                Some(body.clone())
            }
            _ if from.is_some() => return,
            _ => None,
        };
        let Ok(ticket) = panel.tickets.issue() else {
            return;
        };
        let request = Request::Diff {
            scope: summary.scope.clone(),
            root_ticket: summary.root.ticket,
            summary: summary.ticket,
            ticket,
            ask: GetFileDiff {
                root: summary.scope.root(),
                scope: summary.comparison.scope(),
                path: path.clone(),
                ignore_whitespace: false,
                from_hunk: from,
            },
        };
        let _previous = panel
            .changes
            .states
            .insert(path.clone(), FileState::Loading(before));
        let _previous = panel.changes.sources.diffs.insert(
            path.clone(),
            DiffSource {
                request: request.clone(),
                ready: None,
            },
        );
        self.command(Command::CodeSource(request));
    }
}
