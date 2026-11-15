//! A terminal session's screen (ADR 0073, slice T1): read only. Its log
//! is read whole and replayed off the UI thread, then the records that
//! follow are applied as they come; the grid is a view of its own.
//!
//! [Decision Log]
//! - Purpose: a session that is a terminal (`SessionMode::Terminal`)
//!   showed one sentence; its screen is the program's own drawing.
//! - Constraint: a terminal's log is kept whole on the node and can be
//!   megabytes; the link follows a session from its first record, one
//!   record an event, on the UI thread.
//! - Choice: on open, the whole log is read by `session.read` pages
//!   (`Act::Log`) and replayed into a model on the background executor
//!   while the screen says it is reading; records the follow delivers in
//!   the meantime are held and applied after it, through the same
//!   `TerminalLog`, which skips what the read already had (the hand-over
//!   of ADR 0073). After that, a batch is applied on the UI thread when it
//!   is small (at most [`INLINE`] bytes) and on the background executor
//!   when it is not. Every byte goes through `replay`: T1 answers nothing
//!   (T2 adds `feed` and the viewer's duty).
//! - Size: the grid is as many cells as fit the screen (as Paseo's viewer
//!   fits its own xterm.js), measured from the frame before. The screen's
//!   box is measured while the log is still read, so the history is
//!   replayed at the screen's width, not reflowed after; a later change
//!   waits [`SETTLE`] for the window to stop moving (the plan's 100 ms),
//!   so a drag reflows the history once. Nothing is sent to the node (T2
//!   sends the size of a view that has focus and has typed).
//! - Cursor: Paseo's bar blinks every 500 ms; under reduced motion it is
//!   still. A blink changes only the grid view's frame.
//! - Region rule: the grid view is updated and notified only from here.

mod body;
mod convert;
#[cfg(test)]
mod convert_tests;
mod grid;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use chrono::Utc;
use gpui::AppContext as _;
use gpui::Task;
use vessel_app_kit::Scheme;
use vessel_app_kit::terminal_grid::{CellSize, GridCursor, GridFrame};
use vessel_app_kit::ui::{Bounds, Context, Entity, Pixels, px};
use vessel_app_link::{Act, ActAnswer, ActKind, Acted, Command};
use vessel_app_view::terminal::grid::{Generation, Scroll};
use vessel_app_view::terminal::paint::rows;
use vessel_app_view::terminal::{GridSize, ScrollbackLines, TerminalLog, TerminalModel};
use vessel_app_view::timeline::Until;
use vessel_proto::{EventRecord, SessionEvent, SessionId, TerminalSize};

pub(crate) use body::body;
use grid::TerminalGrid;

use crate::shell::Shell;

/// A batch at most this long is applied on the UI thread; a longer one on
/// the background executor. Measured (release, 2026-11-15, a scratch
/// bench over `replay` with a full history): text costs about 3 ms per
/// 64 KiB, but line feeds cost a row each, 11.8 ms per 16 KiB at 200
/// columns and 51 ms at 1,024; at 4 KiB the worst is about 3 ms at 200
/// columns, while a keystroke's echo stays on the UI thread.
const INLINE: usize = 4 * 1024;

/// The cursor's blink: Paseo's `cursorBlink`, every half second.
const BLINK: Duration = Duration::from_millis(500);

/// How long the screen's size must hold before the model takes it.
const SETTLE: Duration = Duration::from_millis(100);

/// The text size: the code size, as Paseo's pane passes its code font
/// size to the terminal (`terminal-pane.tsx` 1188; 12 by default,
/// `storage.ts` 71; `terminal-font.ts`'s 13 is only the fallback when
/// none is given).
fn font_size() -> Pixels {
    px(vessel_app_kit::fonts::code(12.0))
}

/// Where the screen is in reading the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// No terminal session is shown.
    Idle,
    /// Its log is being read and replayed.
    Loading,
    /// Drawn; records apply as they come.
    Ready,
    /// The log could not be read; what comes next is still drawn.
    Unread,
}

/// Whether the cursor's bar is in the shown half of its blink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blink {
    On,
    Off,
}

/// The session the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    machine: String,
    session: SessionId,
}

/// The terminal screen's state.
pub(crate) struct TerminalArea {
    shown: Option<Shown>,
    pub(crate) phase: Phase,
    /// The model; `None` while the background executor holds it.
    model: Option<TerminalModel>,
    log: TerminalLog,
    /// Records the follow delivered while the log was read.
    held: Vec<EventRecord>,
    /// Bytes not yet applied.
    pending: Vec<u8>,
    /// The grid view.
    grid: Entity<TerminalGrid>,
    /// The screen's box in the last frame.
    probe: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The cells that box holds, once measured; kept when another
    /// terminal session opens in the same box.
    fit: Option<GridSize>,
    /// The wait before a changed size is taken.
    settling: Option<Task<()>>,
    blink: Blink,
    /// What the grid shows: the model's change count, the blink and the
    /// scheme it was drawn in.
    drawn: Option<(Generation, Blink, Scheme)>,
    /// The blink's timer, while one runs.
    blinking: Option<Task<()>>,
}

impl TerminalArea {
    pub(crate) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            shown: None,
            phase: Phase::Idle,
            model: None,
            log: TerminalLog::default(),
            held: Vec::new(),
            pending: Vec::new(),
            grid: cx.new(|_cx| TerminalGrid::empty()),
            probe: Rc::new(Cell::new(None)),
            fit: None,
            settling: None,
            blink: Blink::On,
            drawn: None,
            blinking: None,
        }
    }

    fn shows(&self, session: &SessionId) -> bool {
        self.shown
            .as_ref()
            .is_some_and(|shown| shown.session == *session)
    }
}

impl Shell {
    /// The session area opened a terminal session drawn at `size` until
    /// the screen is measured: read its log, and follow it.
    pub(crate) fn terminal_opened(
        &mut self,
        machine: &str,
        session: SessionId,
        size: TerminalSize,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        let size = area
            .fit
            .unwrap_or_else(|| GridSize::within(size.cols, size.rows).unwrap_or_default());
        area.shown = Some(Shown {
            machine: machine.to_owned(),
            session,
        });
        area.phase = Phase::Loading;
        area.model = Some(TerminalModel::new(size, ScrollbackLines::default()));
        area.log = TerminalLog::default();
        area.held.clear();
        area.pending.clear();
        area.drawn = None;
        self.command(Command::Act {
            machine: machine.to_owned(),
            session,
            act: Act::Log(Until::Time(Utc::now())),
        });
        self.terminal_blink(cx);
    }

    /// Another session (or none) is shown.
    pub(crate) fn terminal_closed(&mut self) {
        let area = &mut self.areas.terminal;
        area.shown = None;
        area.phase = Phase::Idle;
        area.model = None;
        area.held.clear();
        area.pending.clear();
        area.settling = None;
        area.blinking = None;
    }

    /// Whether `records` are only output of the terminal shown. Then the
    /// grid, which the area notifies by itself, is all they change, and
    /// the shell is not drawn again for them (the region rule; measured
    /// before this, every batch a program printed drew the shell, its
    /// sidebar and the session's frame).
    pub(crate) fn terminal_output_only(
        &self,
        session: &SessionId,
        records: &[EventRecord],
    ) -> bool {
        self.areas.terminal.shows(session)
            && records
                .iter()
                .all(|record| matches!(record.event, SessionEvent::Terminal(_)))
    }

    /// Records of the session followed.
    pub(crate) fn terminal_records(
        &mut self,
        session: &SessionId,
        records: &[EventRecord],
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        if !area.shows(session) {
            return;
        }
        match area.phase {
            Phase::Idle => {}
            Phase::Loading => area.held.extend_from_slice(records),
            Phase::Ready | Phase::Unread => {
                for bytes in area.log.apply(records) {
                    area.pending.extend_from_slice(bytes);
                }
                self.terminal_apply(cx);
            }
        }
    }

    /// The log read: replayed off the UI thread, then the held records.
    pub(crate) fn terminal_acted(&mut self, answer: &ActAnswer, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if answer.kind != ActKind::Log
            || !area.shows(&answer.session)
            || area.phase != Phase::Loading
        {
            return;
        }
        let Ok(Acted::Log(records)) = &answer.result else {
            tracing::warn!("a terminal's log could not be read");
            area.phase = Phase::Unread;
            let held = std::mem::take(&mut area.held);
            for bytes in area.log.apply(&held) {
                area.pending.extend_from_slice(bytes);
            }
            self.terminal_apply(cx);
            // The body changes from the reading line to the grid.
            cx.notify();
            return;
        };
        let (Some(mut model), session) = (area.model.take(), answer.session) else {
            return;
        };
        // Measured while the log was read: the empty model takes the
        // screen's size now, so nothing is reflowed after the replay.
        refit(&mut model, area.fit);
        let mut log = std::mem::take(&mut area.log);
        let records = records.clone();
        let replay = cx.background_spawn(async move {
            for bytes in log.apply(&records) {
                replay_all(&mut model, bytes);
            }
            (model, log)
        });
        cx.spawn(async move |this, cx| {
            let (model, log) = replay.await;
            let _gone = this.update(cx, |shell, cx| {
                shell.terminal_loaded(session, model, log, cx);
            });
        })
        .detach();
    }

    /// The log replayed: the records held meanwhile follow it.
    fn terminal_loaded(
        &mut self,
        session: SessionId,
        mut model: TerminalModel,
        log: TerminalLog,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        if !area.shows(&session) {
            return;
        }
        refit(&mut model, area.fit);
        area.model = Some(model);
        area.log = log;
        area.phase = Phase::Ready;
        let held = std::mem::take(&mut area.held);
        for bytes in area.log.apply(&held) {
            area.pending.extend_from_slice(bytes);
        }
        self.terminal_apply(cx);
        // The body changes from the reading line to the grid; this is the
        // one time the shell draws again for the terminal's sake.
        cx.notify();
    }

    /// Apply the pending bytes: here when few, off the UI thread when not.
    fn terminal_apply(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if area.pending.len() <= INLINE {
            if let Some(model) = area.model.as_mut() {
                replay_all(model, &area.pending);
                area.pending.clear();
            }
            self.terminal_draw(cx);
            return;
        }
        let Some(mut model) = area.model.take() else {
            // The background executor has it; the bytes wait for its return.
            return;
        };
        let bytes = std::mem::take(&mut area.pending);
        let Some(session) = area.shown.as_ref().map(|shown| shown.session) else {
            return;
        };
        let replay = cx.background_spawn(async move {
            replay_all(&mut model, &bytes);
            model
        });
        cx.spawn(async move |this, cx| {
            let mut model = replay.await;
            let _gone = this.update(cx, |shell, cx| {
                let area = &mut shell.areas.terminal;
                if area.shows(&session) {
                    // A size settled while the model was away.
                    refit(&mut model, area.fit);
                    area.model = Some(model);
                    shell.terminal_apply(cx);
                }
            });
        })
        .detach();
    }

    /// The grid's frame, when what it shows changed.
    pub(super) fn terminal_draw(&mut self, cx: &mut Context<'_, Self>) {
        let tokens = vessel_app_kit::tokens(cx);
        let area = &mut self.areas.terminal;
        let Some(model) = area.model.as_ref() else {
            return;
        };
        let now = (model.grid().generation(), area.blink, tokens.scheme);
        if area.drawn == Some(now) {
            return;
        }
        area.drawn = Some(now);
        let palette = convert::palette(&tokens.terminal);
        let snapshot = model.snapshot(Scroll(0));
        let cursor = snapshot
            .cursor
            .filter(|_| area.blink == Blink::On)
            .map(|cell| GridCursor {
                row: cell.row,
                col: cell.col,
                color: convert::hsla(palette.cursor),
            });
        let frame = GridFrame {
            rows: rows(&snapshot, &palette).iter().map(convert::row).collect(),
            cursor,
            background: convert::hsla(palette.background),
            font_size: font_size(),
        };
        area.grid.update(cx, |grid, cx| {
            grid.frame = Rc::new(frame);
            cx.notify();
        });
    }

    /// The screen's box was drawn with cells of `cell`: the grid takes
    /// that cell size, and the model as many cells as fit, at once the
    /// first time and after [`SETTLE`] when the box changes.
    pub(super) fn terminal_measured(&mut self, cell: CellSize, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if area.grid.read(cx).cell != cell {
            area.grid.update(cx, |grid, cx| {
                grid.cell = cell;
                cx.notify();
            });
        }
        let Some(bounds) = area.probe.get() else {
            return;
        };
        let (cols, rows) = cell.fit(bounds.size);
        let Ok(size) = GridSize::within(cols, rows) else {
            return;
        };
        let first = match area.fit {
            Some(fit) if fit == size => return,
            Some(_) => false,
            None => true,
        };
        area.fit = Some(size);
        if first {
            self.terminal_resize(cx);
            return;
        }
        area.settling = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            let _gone = this.update(cx, Shell::terminal_resize);
        }));
    }

    /// The model takes the measured size, if it is here (one away on the
    /// background executor takes it when it returns).
    fn terminal_resize(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        let fit = area.fit;
        if let Some(model) = area.model.as_mut() {
            refit(model, fit);
            // Drawn only if the resize changed the grid (its generation).
            self.terminal_draw(cx);
        }
    }

    /// Start the cursor's blink, unless motion is reduced.
    fn terminal_blink(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        area.blink = Blink::On;
        if cx.reduce_motion() {
            area.blinking = None;
            return;
        }
        area.blinking = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let going = this.update(cx, |shell, cx| {
                    let area = &mut shell.areas.terminal;
                    area.blink = match area.blink {
                        Blink::On => Blink::Off,
                        Blink::Off => Blink::On,
                    };
                    shell.terminal_draw(cx);
                });
                if going.is_err() {
                    break;
                }
            }
        }));
    }
}

/// `model` resized to `fit` when measured and different.
fn refit(model: &mut TerminalModel, fit: Option<GridSize>) {
    if let Some(fit) = fit
        && model.grid().size() != fit
    {
        model.resize(fit);
    }
}

/// Every byte of `bytes` through `replay`, one budget a call.
fn replay_all(model: &mut TerminalModel, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let taken = model.replay(rest).consumed;
        if taken == 0 {
            break;
        }
        rest = rest.get(taken..).unwrap_or_default();
    }
}
