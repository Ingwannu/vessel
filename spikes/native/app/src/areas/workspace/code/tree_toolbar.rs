//! Tree actions reuse kit controls and the existing preference write path.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, button};
use vessel_app_view::workspace::explorer::{ExplorerSort, HiddenFiles};
use vessel_proto::workspace::RelPath;

use super::actions::{Action, Target};
use super::file_open::FileOpenContext;
use crate::shell::Shell;

impl Shell {
    fn tree_new(
        &mut self,
        origin: &FileOpenContext,
        action: Action,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        // A delayed toolbar click cannot acquire a successor root's identity.
        if !self.file_tree_scope_is_current(origin) {
            return;
        }
        let Some(opening) = self.entry_opening() else {
            return;
        };
        let ticket = opening.ticket;
        self.areas.workspace.code.files.actions.target = Some(Target {
            opening,
            path: RelPath::root(),
            folder: true,
            open: true,
        });
        self.file_action(ticket, action, window, cx);
    }

    fn tree_options_changed(&mut self, cx: &mut Context<'_, Self>) {
        let files = &mut self.areas.workspace.code.files;
        files.relist();
        self.prefs.explorer = files.options;
        self.command(vessel_app_link::Command::SavePrefs(Box::new(
            self.prefs.clone(),
        )));
        cx.notify();
    }

    fn tree_sort_changed(&mut self, cx: &mut Context<'_, Self>) {
        let metadata = self
            .file_admitted_scope()
            .is_some_and(|scope| self.file_panels_available(&scope));
        let options = &mut self.areas.workspace.code.files.options;
        options.sort = match options.sort {
            ExplorerSort::Name if metadata => ExplorerSort::Modified,
            ExplorerSort::Name | ExplorerSort::Modified => ExplorerSort::Size,
            ExplorerSort::Size => ExplorerSort::Name,
        };
        self.tree_options_changed(cx);
    }

    fn tree_hidden_changed(&mut self, cx: &mut Context<'_, Self>) {
        let options = &mut self.areas.workspace.code.files.options;
        options.hidden = match options.hidden {
            HiddenFiles::Show => HiddenFiles::Hide,
            HiddenFiles::Hide => HiddenFiles::Show,
        };
        self.restore_visible_tree();
        self.tree_options_changed(cx);
    }
}

pub(super) fn toolbar(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let labels = shell.words().labels();
    let files = &shell.areas.workspace.code.files;
    let new_file = shell.file_open_context();
    let new_folder = new_file.clone();
    let sort = match files.options.sort {
        ExplorerSort::Name => labels.by_name,
        ExplorerSort::Modified => labels.file_sort_modified,
        ExplorerSort::Size => labels.file_sort_size,
    };
    let hidden = match files.options.hidden {
        HiddenFiles::Show => labels.hide_hidden_files,
        HiddenFiles::Hide => labels.show_hidden_files,
    };
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(spacing::S1))
        .p(px(spacing::S2))
        .child(
            button("files-refresh", labels.files_refresh, false, tokens).on_click(cx.listener(
                |shell, _, _, cx| {
                    shell.reread_files();
                    cx.notify();
                },
            )),
        )
        .child(
            button("files-sort", sort, false, tokens)
                .on_click(cx.listener(|shell, _, _, cx| shell.tree_sort_changed(cx))),
        )
        .child(
            button("files-hidden", hidden, false, tokens)
                .on_click(cx.listener(|shell, _, _, cx| shell.tree_hidden_changed(cx))),
        )
        .child(
            button("files-new-file", labels.new_file, false, tokens).on_click(cx.listener(
                move |shell, _, window, cx| {
                    if let Some(origin) = &new_file {
                        shell.tree_new(origin, Action::NewFile, window, cx);
                    }
                },
            )),
        )
        .child(
            button("files-new-folder", labels.new_folder, false, tokens).on_click(cx.listener(
                move |shell, _, window, cx| {
                    if let Some(origin) = &new_folder {
                        shell.tree_new(origin, Action::NewFolder, window, cx);
                    }
                },
            )),
        )
}

#[cfg(test)]
mod tests;
