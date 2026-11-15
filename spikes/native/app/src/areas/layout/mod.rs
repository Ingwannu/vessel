//! Shared pane adapter; renderers own content and lifecycle, never docking state.
mod action_context;
mod actions;
mod book;
mod draw;
mod header;
mod menu;
// The desktop coordinator removes this exact staging attribute with its real caller.
#[cfg(test)]
mod preferences;
mod queries;
pub(crate) mod registry;
mod rename;
mod resize;
// The product cutover removes this gate together with converted global storage.
#[cfg(test)]
pub(crate) mod residents;
mod sheets;

use crate::shell::Shell;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use vessel_app_kit::MenuState;
use vessel_app_kit::ui::{App, Context, Window};
use vessel_app_view::layout::{
    HostAlias, Layout, LayoutError, LayoutKey, LayoutMode, PaneId, TabId,
};

pub(crate) use draw::frame;

/// Presentation retained across navigation; buffers stay with feature owners.
pub(crate) struct LayoutArea {
    #[cfg(test)]
    retirements: registry::retirement::Owners,
    book: RefCell<Option<BTreeMap<LayoutKey, Layout>>>,
    menu: MenuState,
    tips: vessel_app_kit::Tooltips,
    tip_places: RefCell<BTreeMap<(PaneId, header::Control), vessel_app_kit::tooltip::Place>>,
    rename: Option<rename::Rename>,
    rename_ticket: vessel_app_view::layout::RenameTicket,
    rename_dialog: vessel_app_kit::DialogState,
    menu_tab: Option<(LayoutKey, TabId)>,
    maximized: Option<(LayoutKey, PaneId)>,
    drop_preview: Cell<Option<(PaneId, vessel_app_view::layout::Position)>>,
    tab_drop_preview: Cell<Option<(TabId, vessel_app_view::layout::TabInsertion)>>,
    resizing: std::rc::Rc<Cell<Option<vessel_app_view::layout::GroupId>>>,
}
impl LayoutArea {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            #[cfg(test)]
            retirements: registry::retirement::Owners::new(),
            book: RefCell::new(None),
            menu: MenuState::new(cx),
            tips: vessel_app_kit::Tooltips::new(),
            tip_places: RefCell::new(BTreeMap::new()),
            rename: None,
            rename_ticket: vessel_app_view::layout::RenameTicket::ZERO,
            rename_dialog: vessel_app_kit::DialogState::new(cx),
            menu_tab: None,
            maximized: None,
            drop_preview: Cell::new(None),
            tab_drop_preview: Cell::new(None),
            resizing: std::rc::Rc::new(Cell::new(None)),
        }
    }
}
fn key(shell: &Shell) -> Result<LayoutKey, LayoutError> {
    let open = shell
        .areas
        .session
        .open
        .as_ref()
        .ok_or(LayoutError::InvalidState)?;
    Ok(LayoutKey::Session {
        machine: HostAlias::parse(&open.machine)?,
        session: open.session,
    })
}
pub(crate) fn mode(window: &Window) -> LayoutMode {
    LayoutMode::for_width(
        f32::from(window.viewport_size().width),
        !cfg!(any(target_os = "android", target_os = "ios")),
    )
}
fn initialized(shell: &Shell) {
    if shell.areas.layout.book.borrow().is_none() {
        let entries = shell
            .prefs
            .layouts
            .iter()
            .map(|entry| (entry.key.clone(), entry.layout.clone()))
            .collect();
        *shell.areas.layout.book.borrow_mut() = Some(entries);
    }
}
fn with_layout<T>(
    shell: &Shell,
    change: impl FnOnce(&mut Layout) -> Result<T, LayoutError>,
) -> Result<T, LayoutError> {
    let key = key(shell)?;
    with_layout_at(shell, &key, change)
}
fn with_layout_at<T>(
    shell: &Shell,
    key: &LayoutKey,
    change: impl FnOnce(&mut Layout) -> Result<T, LayoutError>,
) -> Result<T, LayoutError> {
    initialized(shell);
    let mut book = shell.areas.layout.book.borrow_mut();
    let book = book.as_mut().ok_or(LayoutError::InvalidState)?;
    // Copy metadata only: neither failed batches nor a failed first open may
    // publish a partial target or manufacture an empty workspace entry.
    let mut next = book.get(key).cloned().unwrap_or_default();
    let value = change(&mut next)?;
    let _previous = book.insert(key.clone(), next);
    Ok(value)
}

impl Shell {
    fn layout_result(&mut self, result: Result<(), LayoutError>, cx: &mut Context<'_, Self>) {
        match result {
            Ok(()) => {
                initialized(self);
                if let Some(book) = self.areas.layout.book.borrow().as_ref() {
                    self.prefs.layouts = book
                        .iter()
                        .map(|(key, layout)| {
                            vessel_app_view::layout::RememberedLayout::new(key.clone(), layout)
                        })
                        .collect();
                }
                self.command(vessel_app_link::Command::SavePrefs(Box::new(
                    self.prefs.clone(),
                )));
            }
            Err(LayoutError::Modified) => {
                self.notice = Some(self.words.layout().modified().to_owned());
            }
            Err(
                LayoutError::InvalidIdentity
                | LayoutError::MissingPane
                | LayoutError::MissingTab
                | LayoutError::UnsupportedHost
                | LayoutError::SplitsUnavailable
                | LayoutError::TreeTooDeep
                | LayoutError::InvalidState
                | LayoutError::Exhausted,
            ) => self.notice = Some(self.words.layout().refused().to_owned()),
        }
        cx.notify();
    }
}
