//! A side panel's resizable edge as the shell keeps it, for the sidebar
//! and the code panel (the owner, 2026-11-14, word 29): the kit's handle
//! (`vessel_app_kit::resize`), the width while it is dragged, and the
//! width kept in the preferences once it is let go.
//!
//! [Decision Log]
//! - Purpose: both panels are dragged, drawn and kept by the same code;
//!   only their limits (`vessel_app_view::panels`), their preference and
//!   the room they are measured against differ.
//! - Choice: while dragging, the width asked for is held here and the
//!   frame drawn from it (clamped by the panel's limits each frame); on
//!   release the width the panel was drawn at is kept, in whole points
//!   (`Limits::kept`), so a width the window could not give is not
//!   kept as if it had been. A double-click forgets the width.
//! - Alternatives: saving on every move (a write of the preferences file
//!   per pointer event); the width in the view only (lost on restart, and
//!   the owner asked for it kept).

use std::cell::Cell;
use std::rc::Rc;

use vessel_app_kit::Tokens;
use vessel_app_kit::resize::{self, Edge, ResizeEvents, ResizeState};
use vessel_app_kit::ui::{AnyElement, Context, ElementId, Window};
use vessel_app_view::panels::Limits;
use vessel_app_view::prefs::PreferenceChange;

use crate::shell::Shell;

/// A panel's edge.
pub(crate) struct PanelEdge {
    /// The kit's handle.
    handle: ResizeState,
    /// The width asked for while dragging.
    dragged: Rc<Cell<Option<f32>>>,
}

/// Which panel, and how its width is kept.
#[derive(Clone, Copy)]
pub(crate) struct Panel {
    /// Its limits.
    pub(crate) limits: Limits,
    /// Its edge in the shell.
    pub(crate) edge: fn(&Shell) -> &PanelEdge,
    /// Its kept width.
    pub(crate) kept: fn(&Shell) -> Option<u16>,
    /// The preference that keeps it.
    pub(crate) change: fn(Option<u16>) -> PreferenceChange,
    /// The edge it is dragged at.
    pub(crate) side: Edge,
}

impl PanelEdge {
    pub(crate) fn new() -> Self {
        Self {
            handle: ResizeState::new(),
            dragged: Rc::new(Cell::new(None)),
        }
    }
}

impl Panel {
    /// The width drawn in `room` points (see `Limits::width`).
    pub(crate) fn width(self, shell: &Shell, room: f32) -> f32 {
        let edge = (self.edge)(shell);
        let asked = edge
            .dragged
            .get()
            .or_else(|| (self.kept)(shell).map(f32::from));
        self.limits.width(asked, room)
    }

    /// The measure and the handle, to put inside the panel (`relative`),
    /// after its content.
    pub(crate) fn handle(
        self,
        id: impl Into<ElementId>,
        shell: &Shell,
        tokens: &Tokens,
        window: &Window,
        cx: &mut Context<'_, Shell>,
    ) -> [AnyElement; 2] {
        let edge = (self.edge)(shell);
        let this = cx.entity().downgrade();
        let dragged = Rc::clone(&edge.dragged);
        let drag: resize::OnWidth = Rc::new(move |width, _window, cx| {
            dragged.set(Some(width));
            let _drawn = this.update(cx, |_shell, cx| cx.notify());
        });
        let this = cx.entity().downgrade();
        let done: resize::OnDone = Rc::new(move |window, cx| {
            let _kept = this.update(cx, |shell, cx| {
                let edge = (self.edge)(shell);
                let drawn = edge.handle.width().unwrap_or(self.limits.default);
                edge.dragged.set(None);
                shell.prefer(&(self.change)(self.limits.kept(drawn)), window, cx);
            });
        });
        let this = cx.entity().downgrade();
        let reset: resize::OnDone = Rc::new(move |window, cx| {
            let _kept = this.update(cx, |shell, cx| {
                (self.edge)(shell).dragged.set(None);
                shell.prefer(&(self.change)(None), window, cx);
            });
        });
        let events = ResizeEvents { drag, done, reset };
        [
            resize::measure(&edge.handle),
            resize::handle(id, &edge.handle, self.side, tokens, &events, window),
        ]
    }
}
