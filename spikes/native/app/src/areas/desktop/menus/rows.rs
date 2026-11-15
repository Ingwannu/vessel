//! One command vocabulary supplies the native menu bar and the kit popup.

use super::{DesktopAction, Edit, Group, Item, MenuItem, Row, Shell};
use vessel_app_view::desktop::{CopyState, HelpAction, PasteState};

impl Row {
    fn new(item: Item, label: &str) -> Self {
        Self {
            item,
            view: MenuItem::new(label.to_owned()),
        }
    }

    fn grouped(mut self) -> Self {
        self.view.starts_group = true;
        self
    }
}

pub(in crate::areas::desktop) fn rows(shell: &Shell, group: Group) -> Vec<Row> {
    let mut rows = match group {
        Group::All => [
            Group::File,
            Group::Edit,
            Group::View,
            Group::Window,
            Group::Help,
        ]
        .into_iter()
        .flat_map(|group| rows(shell, group))
        .collect(),
        Group::File => file(shell),
        Group::Edit => edit(shell),
        Group::View => view(shell),
        Group::Window => {
            let w = shell.words.desktop_shell();
            vec![
                Row::new(Item::Minimize, w.minimize()),
                Row::new(Item::Maximize, w.maximize()),
            ]
        }
        Group::Help => {
            let w = shell.words.desktop_shell();
            vec![
                Row::new(Item::Help(HelpAction::About), w.about()).grouped(),
                Row::new(Item::Help(HelpAction::Changelog), w.whats_new()),
                Row::new(Item::Help(HelpAction::ReportIssue), w.report_issue()),
            ]
        }
        Group::Context => context(shell),
    };
    if !shell.desktop_commands_admitted() {
        for row in &mut rows {
            row.view.disabled = true;
        }
    }
    rows
}

fn file(shell: &Shell) -> Vec<Row> {
    let w = shell.words.desktop_shell();
    vec![
        Row::new(Item::Desktop(DesktopAction::NewWindow), w.new_window()),
        Row::new(Item::Desktop(DesktopAction::CloseWindow), w.close_window()),
        Row::new(Item::Quit, w.quit()).grouped(),
    ]
}

fn edit(shell: &Shell) -> Vec<Row> {
    let w = shell.words.desktop_shell();
    [
        (Edit::Undo, w.undo()),
        (Edit::Redo, w.redo()),
        (Edit::Cut, w.cut()),
        (Edit::Copy, w.copy()),
        (Edit::Paste, w.paste()),
        (Edit::SelectAll, w.select_all()),
    ]
    .into_iter()
    .map(|(edit, label)| {
        let row = Row::new(Item::Edit(edit), label);
        if matches!(edit, Edit::Undo | Edit::Cut | Edit::SelectAll) {
            row.grouped()
        } else {
            row
        }
    })
    .collect()
}

fn view(shell: &Shell) -> Vec<Row> {
    let w = shell.words.desktop_shell();
    vec![
        Row::new(Item::Reload, w.reload()).grouped(),
        Row::new(Item::ForceReload, w.force_reload()),
        Row::new(Item::Desktop(DesktopAction::ZoomIn), w.zoom_in()).grouped(),
        Row::new(Item::Desktop(DesktopAction::ZoomOut), w.zoom_out()),
        Row::new(Item::Desktop(DesktopAction::ActualSize), w.actual_size()),
        Row::new(Item::Desktop(DesktopAction::Fullscreen), w.fullscreen()).grouped(),
    ]
}

fn context(shell: &Shell) -> Vec<Row> {
    let w = shell.words.desktop_shell();
    [
        (Edit::Copy, w.copy()),
        (Edit::Paste, w.paste()),
        (Edit::SelectAll, w.select_all()),
    ]
    .into_iter()
    .map(|(edit, label)| {
        let mut row = Row::new(Item::Edit(edit), label);
        row.view.disabled = match edit {
            Edit::Copy => shell.areas.desktop.edit.copy == CopyState::Unavailable,
            Edit::Paste => shell.areas.desktop.edit.paste == PasteState::Unavailable,
            Edit::Undo | Edit::Redo | Edit::Cut | Edit::SelectAll => false,
        };
        if matches!(edit, Edit::SelectAll) {
            row.grouped()
        } else {
            row
        }
    })
    .collect()
}
