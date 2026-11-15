//! Node-backed schedules screen. View owns every decision, Link every call;
//! this area owns the shared controls, their anchors and cached display inputs.
mod controller;
mod data;
mod draw;
mod editor;
mod filters;
mod history;
mod overlays;
mod row;
mod state;
#[cfg(test)]
mod tests;
use crate::shell::Shell;
pub(crate) use controller::{arrive, leaving, sidebar};
use data::Data;
pub(crate) use draw::render;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use vessel_app_kit::{
    ComboboxState,
    dialog::DialogState,
    menu::MenuState,
    select::Anchor,
    ui::{Context, SharedString, Window},
};
use vessel_app_view::schedules::page::{Intent, RowKey};

#[derive(Debug)]
pub(crate) struct Area {
    data: Data,
    pub(super) ui: Ui,
    pub(super) tick: Option<Tick>,
}
#[derive(Debug)]
pub(super) struct Ui {
    editor: editor::Ui,
    picker: ComboboxState,
    anchor: Anchor,
    menu: MenuState,
    selected: Option<Rc<Intent>>,
    deleting: Option<Rc<DeletePrompt>>,
    dialog: DialogState,
    history: Option<vessel_app_view::schedules::history::reader::Visit>,
    history_dialog: DialogState,
    anchors: BTreeMap<RowKey, RowControl>,
    pending_focus: BTreeSet<Popup>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Popup {
    Actions,
    Hosts,
    Delete,
}
#[derive(Debug)]
pub(super) struct RowControl {
    anchor: Anchor,
    id: SharedString,
}
#[derive(Debug)]
pub(super) struct DeletePrompt {
    intent: Intent,
    title: SharedString,
    body: SharedString,
}
impl Area {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            data: Data::default(),
            ui: Ui {
                editor: editor::Ui::new(window, cx),
                picker: ComboboxState::new(window, cx),
                anchor: Anchor::default(),
                menu: MenuState::new(cx),
                selected: None,
                deleting: None,
                dialog: DialogState::new(cx),
                history: None,
                history_dialog: DialogState::new(cx),
                anchors: BTreeMap::new(),
                pending_focus: BTreeSet::new(),
            },
            tick: None,
        }
    }
}

#[derive(Debug)]
pub(super) struct Tick {
    tier: vessel_app_view::schedules::time::RefreshTier,
    task: vessel_app_kit::ui::Task<()>,
}
impl Tick {
    fn cancel(self) {
        drop(self.task);
    }
}
