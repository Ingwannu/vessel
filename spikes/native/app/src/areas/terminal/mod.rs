//! A terminal session's read-only screen. The area owns lifecycle and
//! adapts the view's one parser/grid to the kit's one renderer (ADR 0073).
//! Reads and replays carry one opening token; no parsing runs in a UI update.

mod body;
#[cfg(test)]
#[cfg(target_os = "linux")]
pub(crate) mod capture;
mod convert;
#[cfg(test)]
mod convert_tests;
mod draw;
mod grid;
mod presentation;
mod replay;
mod scroll;
#[cfg(test)]
mod scroll_tests;
#[cfg(test)]
mod settings;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod tests;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use chrono::Utc;
use gpui::{AppContext as _, FocusHandle, Subscription, Task};
use vessel_app_kit::ui::{Bounds, Context, Entity, Pixels, px};
use vessel_app_link::{Act, Command, NodeName, TerminalOpening};
use vessel_app_view::terminal::{GridSize, ScrollbackLines, TerminalLog, TerminalModel, Viewport};
use vessel_app_view::timeline::Until;
use vessel_proto::{EventRecord, SessionEvent, SessionId, TerminalSize};

pub(crate) use body::body;
use draw::DrawKey;
use grid::TerminalGrid;
use presentation::Playback;
#[cfg(test)]
use replay::replay_all;
use scroll::Wheel;

use crate::shell::Shell;

/// Paseo's blinking bar changes half every half second.
const BLINK: Duration = Duration::from_millis(500);
/// The measured box must hold before history is reflowed again.
const SETTLE: Duration = Duration::from_millis(100);

fn font_size() -> Pixels {
    px(vessel_app_kit::fonts::code(12.0))
}

/// The retained read state also determines which application notice is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// No terminal session is shown.
    Idle,
    /// Its log is being read and replayed.
    Loading,
    /// Drawn; records apply as they come.
    Ready,
    /// Earlier output was unreadable; later output still draws.
    Unread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blink {
    On,
    Off,
}

/// The terminal area retains data, tasks and the identity which owns them.
pub(crate) struct TerminalArea {
    shown: Option<TerminalOpening>,
    pub(crate) phase: Phase,
    model: Option<TerminalModel>,
    log: TerminalLog,
    scrollback: ScrollbackLines,
    held: Vec<EventRecord>,
    pending: Vec<u8>,
    grid: Entity<TerminalGrid>,
    probe: Rc<Cell<Option<Bounds<Pixels>>>>,
    fit: Option<GridSize>,
    settling: Option<Task<()>>,
    blink: Blink,
    drawn: Option<DrawKey>,
    blinking: Option<Task<()>>,
    focus: FocusHandle,
    observers: Vec<Subscription>,
    playback: Playback,
    viewport: Viewport,
    wheel: Wheel,
}

impl TerminalArea {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            shown: None,
            phase: Phase::Idle,
            model: None,
            log: TerminalLog::default(),
            scrollback: ScrollbackLines::default(),
            held: Vec::new(),
            pending: Vec::new(),
            grid: cx.new(|_cx| TerminalGrid::empty()),
            probe: Rc::new(Cell::new(None)),
            fit: None,
            settling: None,
            blink: Blink::On,
            drawn: None,
            blinking: None,
            focus: cx.focus_handle(),
            observers: Vec::new(),
            playback: Playback::Steady,
            viewport: Viewport::default(),
            wheel: Wheel::default(),
        }
    }

    fn shows(&self, session: &SessionId) -> bool {
        self.shown
            .as_ref()
            .is_some_and(|opening| opening.session() == *session)
    }

    fn owns(&self, opening: &TerminalOpening) -> bool {
        self.shown.as_ref() == Some(opening)
    }
}

impl Shell {
    /// Adapt the legacy session area's raw book name once, before any terminal work.
    pub(crate) fn terminal_follow(
        &mut self,
        open: &crate::areas::session::Open,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if open.structured {
            self.terminal_closed();
        } else {
            let Ok(machine) = open.machine.parse::<NodeName>() else {
                self.notice = Some(self.t.terminal_unread.to_owned());
                return false;
            };
            self.terminal_opened(machine, open.session, TerminalSize::default(), cx);
        }
        true
    }

    /// A new opening invalidates every outstanding read and replay, even
    /// if this machine/session pair was shown earlier.
    pub(crate) fn terminal_opened(
        &mut self,
        machine: NodeName,
        session: SessionId,
        size: TerminalSize,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        let size = area
            .fit
            .unwrap_or_else(|| GridSize::within(size.cols, size.rows).unwrap_or_default());
        let opening = TerminalOpening::new(machine, session);
        area.shown = Some(opening.clone());
        area.phase = Phase::Loading;
        area.model = Some(TerminalModel::new(size, area.scrollback));
        area.log = TerminalLog::default();
        area.held.clear();
        area.pending.clear();
        area.drawn = None;
        area.blinking = None;
        area.settling = None;
        area.blink = Blink::On;
        area.viewport = Viewport::default();
        area.wheel = Wheel::default();
        self.command(Command::Act {
            machine: opening.machine().to_string(),
            session,
            act: Act::TerminalLog {
                opening,
                until: Until::Time(Utc::now()),
            },
        });
        self.terminal_draw(cx);
    }

    /// Closing invalidates outstanding ownership before discarding any data.
    pub(crate) fn terminal_closed(&mut self) {
        let area = &mut self.areas.terminal;
        area.shown = None;
        area.phase = Phase::Idle;
        area.model = None;
        area.held.clear();
        area.pending.clear();
        area.settling = None;
        area.blinking = None;
        area.playback = Playback::Steady;
    }

    /// Terminal-only output redraws its retained grid, not the shell.
    pub(crate) fn terminal_output_only(
        &self,
        session: &SessionId,
        records: &[EventRecord],
    ) -> bool {
        self.areas.terminal.shows(session)
            && records
                .iter()
                .all(|r| matches!(r.event, SessionEvent::Terminal(_)))
    }
}
