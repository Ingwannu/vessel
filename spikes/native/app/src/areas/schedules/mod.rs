//! The Schedules route owns focus and prepared paint data. View owns row
//! state and permissions, and link owns admission and all node effects.
mod controller;
mod data;
mod draw;
mod filters;
mod overlays;
mod row;
mod state;

pub(crate) use controller::{arrive, leaving, sidebar};
pub(crate) use draw::render;

use crate::shell::Shell;
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
use vessel_app_view::schedules::{
    page::{Intent, RowKey},
    time::RefreshTier,
};

pub(crate) struct Area {
    data: data::Data,
    ui: Ui,
    tick: Option<(RefreshTier, gpui::Task<()>)>,
}

struct Ui {
    picker: ComboboxState,
    anchor: Anchor,
    menu: MenuState,
    selected: Option<Rc<Intent>>,
    deleting: Option<Rc<DeletePrompt>>,
    dialog: DialogState,
    anchors: BTreeMap<RowKey, RowControl>,
    pending_focus: BTreeSet<Popup>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Popup {
    Actions,
    Hosts,
    Delete,
}

struct RowControl {
    anchor: Anchor,
    id: SharedString,
}

struct DeletePrompt {
    intent: Intent,
    title: SharedString,
    body: SharedString,
}

impl Area {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Shell>) -> Self {
        Self {
            data: data::Data::default(),
            ui: Ui {
                picker: ComboboxState::new(window, cx),
                anchor: Anchor::default(),
                menu: MenuState::new(cx),
                selected: None,
                deleting: None,
                dialog: DialogState::new(cx),
                anchors: BTreeMap::new(),
                pending_focus: BTreeSet::new(),
            },
            tick: None,
        }
    }
}

/// Settle the original ticket when the local link queue did not take it.
pub(crate) fn refuse(
    request: vessel_app_view::schedules::page::Request,
    why: vessel_app_link::Rejected,
) -> vessel_app_link::Event {
    use vessel_app_link::{Event, Rejected, schedules::Answer};
    use vessel_app_view::schedules::load::Failure;
    let reason = match why {
        Rejected::Full => Failure::Busy,
        Rejected::Closed => Failure::Offline,
    };
    Event::Schedules(Answer::refused(request, reason))
}
