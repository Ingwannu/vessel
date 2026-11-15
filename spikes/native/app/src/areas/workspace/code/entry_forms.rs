//! Entry forms carry their originating root and opening through queued UI events.
//! A new selection cannot lend authority to an old Enter/Delete/Cancel callback.

use vessel_app_kit::TextField;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::workspace::PathChange;
use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_proto::workspace::{CreateDir, DeletePath, FolderName, RelPath, RenamePath};

use super::actions::{EditKind, Editing, RemovalDepth, Target};
use crate::shell::Shell;

impl Shell {
    pub(super) fn start_entry_form(
        &mut self,
        kind: EditKind,
        target: Target,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if !matches!(kind, EditKind::Rename(_)) && !target.open {
            self.toggle_folder(&target.path, cx);
        }
        let ticket = target.opening.ticket;
        let field = TextField::new(
            self.words().labels().name_placeholder,
            move |shell: &mut Shell, text, _, cx| {
                shell.finish_edit(ticket, &text, cx);
            },
            window,
            cx,
        );
        if let EditKind::Rename(path) = &kind {
            field.set_text(path.file_name().unwrap_or_default(), window, cx);
        }
        field.focus(window, cx);
        let files = &mut self.areas.workspace.code.files;
        files.actions.refused = None;
        files.actions.editing = Some(Editing {
            opening: target.opening,
            kind,
            field,
        });
        files.relist();
    }

    pub(super) fn finish_edit(
        &mut self,
        ticket: FileTicket,
        text: &str,
        cx: &mut Context<'_, Self>,
    ) {
        let panel = &self.areas.workspace.code;
        let Some(editing) =
            panel.files.actions.editing.as_ref().filter(|editing| {
                editing.opening.ticket == ticket && editing.opening.current(panel)
            })
        else {
            return;
        };
        let (opening, kind) = (editing.opening.clone(), editing.kind.clone());
        let Ok(name) = text.trim().parse::<FolderName>() else {
            self.areas.workspace.code.files.actions.refused =
                Some(self.words().labels().name_refused.to_owned());
            cx.notify();
            return;
        };
        let target = match &kind {
            EditKind::NewFile(dir) | EditKind::NewFolder(dir) => dir.child(name.as_str()),
            EditKind::Rename(path) => path
                .parent()
                .unwrap_or_else(RelPath::root)
                .child(name.as_str()),
        };
        let Ok(target) = target else {
            self.areas.workspace.code.files.actions.refused =
                Some(self.words().labels().name_refused.to_owned());
            cx.notify();
            return;
        };
        let root = opening.scope.root();
        let change = match kind {
            EditKind::NewFile(_) => PathChange::CreateFile { root, path: target },
            EditKind::NewFolder(_) => PathChange::CreateDir(CreateDir { root, path: target }),
            EditKind::Rename(from) => PathChange::Rename(RenamePath {
                root,
                from,
                to: target,
            }),
        };
        self.begin_file_mutation_for(opening.scope, change, cx);
        cx.notify();
    }

    pub(super) fn cancel_edit(&mut self, ticket: FileTicket, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        if files
            .actions
            .editing
            .as_ref()
            .is_some_and(|editing| editing.opening.ticket == ticket)
        {
            files.actions.editing = None;
            files.relist();
            cx.notify();
        }
    }

    pub(super) fn cancel_delete(&mut self, ticket: FileTicket, cx: &mut Context<'_, Self>) {
        let prompt = &mut self.areas.workspace.code.files.actions.deleting;
        if prompt
            .take_if(|prompt| prompt.opening.ticket == ticket)
            .is_some()
        {
            cx.notify();
        }
    }

    pub(super) fn confirm_delete(&mut self, ticket: FileTicket, cx: &mut Context<'_, Self>) {
        let Some(prompt) = self
            .areas
            .workspace
            .code
            .files
            .actions
            .deleting
            .take_if(|prompt| prompt.opening.ticket == ticket)
        else {
            return;
        };
        // Discard the matched old capability, never reinterpret it at a new root.
        if !prompt.opening.current(&self.areas.workspace.code) {
            cx.notify();
            return;
        }
        self.begin_file_mutation_for(
            prompt.opening.scope.clone(),
            PathChange::Delete(DeletePath {
                root: prompt.opening.scope.root(),
                path: prompt.path,
                recursive: matches!(prompt.depth, RemovalDepth::DirectoryTree),
            }),
            cx,
        );
        cx.notify();
    }
}
