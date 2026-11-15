//! Node-backed schedules screen. View owns every decision, Link every call;
//! this area owns the shared controls, their anchors and cached display inputs.
mod controller;
mod data;
mod draw;
mod filters;
mod overlays;
mod row;
mod state;
#[cfg(test)]
mod tests;
use crate::shell::Shell;
pub(crate) use controller::{arrive, leaving, sidebar};
use data::Data;
pub(crate) use draw::render;
use std::collections::BTreeMap;
use vessel_app_kit::{
    ComboboxState,
    dialog::DialogState,
    menu::MenuState,
    select::Anchor,
    ui::{Context, SharedString, Window},
};
use vessel_app_view::schedules::page::RowKey;

#[derive(Debug)]
pub(crate) struct Area {
    pub(super) data: Data,
    pub(super) ui: Ui,
    pub(super) ticking: bool,
}
#[derive(Debug)]
pub(super) struct Ui {
    picker: ComboboxState,
    anchor: Anchor,
    menu: MenuState,
    selected: Option<RowKey>,
    deleting: Option<DeletePrompt>,
    dialog: DialogState,
    anchors: BTreeMap<RowKey, RowControl>,
}
#[derive(Debug)]
pub(super) struct RowControl {
    anchor: Anchor,
    id: SharedString,
}
#[derive(Debug)]
pub(super) struct DeletePrompt {
    key: RowKey,
    title: SharedString,
    body: SharedString,
}
impl Area {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            data: Data::default(),
            ui: Ui {
                picker: ComboboxState::new(window, cx),
                anchor: Anchor::default(),
                menu: MenuState::new(cx),
                selected: None,
                deleting: None,
                dialog: DialogState::new(cx),
                anchors: BTreeMap::new(),
            },
            ticking: false,
        }
    }
}
