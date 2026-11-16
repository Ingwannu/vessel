//! One model owner, with all parsing on the background executor. Every
//! completion checks its opening before taking any current state.

use gpui::AppContext as _;
use vessel_app_kit::ui::Context;
use vessel_app_link::{ActAnswer, Acted, TerminalOpening};
use vessel_app_view::terminal::{GridSize, TerminalLog, TerminalModel};
use vessel_proto::{EventRecord, SessionId};

use super::Phase;
use crate::shell::Shell;

impl Shell {
    /// Records are deduplicated by the current opening's log cursor.
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

    /// A terminal receipt is consumed here even when stale, so a refusal
    /// cannot appear again as conversation furniture.
    pub(crate) fn terminal_acted(
        &mut self,
        answer: &ActAnswer,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(opening) = &answer.terminal else {
            return false;
        };
        let area = &mut self.areas.terminal;
        if !area.owns(opening)
            || area.phase != Phase::Loading
            || opening.machine().as_str() != answer.machine
            || opening.session() != answer.session
        {
            return true;
        }
        let Ok(Acted::Log(records)) = &answer.result else {
            tracing::warn!("a terminal's log could not be read");
            area.phase = Phase::Unread;
            self.terminal_release_held(cx);
            cx.notify();
            return true;
        };
        let Some(mut model) = area.model.take() else {
            return true;
        };
        area.apply_scrollback(&mut model);
        refit(&mut model, area.fit);
        let mut log = std::mem::take(&mut area.log);
        let records = records.clone();
        let opening = opening.clone();
        let replay = cx.background_spawn(async move {
            for bytes in log.apply(&records) {
                replay_all(&mut model, bytes);
            }
            (model, log)
        });
        cx.spawn(async move |this, cx| {
            let (model, log) = replay.await;
            let _gone = this.update(cx, |shell, cx| {
                shell.terminal_loaded(&opening, model, log, cx);
            });
        })
        .detach();
        true
    }

    /// History handover, accepted only by the opening that requested it.
    pub(super) fn terminal_loaded(
        &mut self,
        opening: &TerminalOpening,
        mut model: TerminalModel,
        log: TerminalLog,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        if !area.owns(opening) || area.phase != Phase::Loading {
            return;
        }
        area.apply_scrollback(&mut model);
        refit(&mut model, area.fit);
        area.model = Some(model);
        area.log = log;
        area.phase = Phase::Ready;
        self.terminal_release_held(cx);
        cx.notify();
    }

    fn terminal_release_held(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        let held = std::mem::take(&mut area.held);
        for bytes in area.log.apply(&held) {
            area.pending.extend_from_slice(bytes);
        }
        self.terminal_apply(cx);
    }

    /// No per-event shortcut: even a queued burst of tiny records performs
    /// zero parsing on the UI thread. Only one executor owns the model.
    pub(super) fn terminal_apply(&mut self, cx: &mut Context<'_, Self>) {
        let area = &mut self.areas.terminal;
        if area.pending.is_empty() {
            self.terminal_draw(cx);
            return;
        }
        let Some(opening) = area.shown.clone() else {
            return;
        };
        let Some(mut model) = area.model.take() else {
            return;
        };
        let bytes = std::mem::take(&mut area.pending);
        let replay = cx.background_spawn(async move {
            replay_all(&mut model, &bytes);
            model
        });
        cx.spawn(async move |this, cx| {
            let model = replay.await;
            let _gone = this.update(cx, |shell, cx| shell.terminal_applied(&opening, model, cx));
        })
        .detach();
    }

    /// Output handover shares the same ownership rule as history.
    pub(super) fn terminal_applied(
        &mut self,
        opening: &TerminalOpening,
        mut model: TerminalModel,
        cx: &mut Context<'_, Self>,
    ) {
        let area = &mut self.areas.terminal;
        if !area.owns(opening) {
            return;
        }
        area.apply_scrollback(&mut model);
        refit(&mut model, area.fit);
        area.model = Some(model);
        self.terminal_draw(cx);
        self.terminal_apply(cx);
    }
}

/// Reflow only after measured geometry differs from the owned model.
pub(super) fn refit(model: &mut TerminalModel, fit: Option<GridSize>) {
    if let Some(fit) = fit
        && model.grid().size() != fit
    {
        model.resize(fit);
    }
}

/// Preserve byte order across the parser's bounded feed calls.
pub(super) fn replay_all(model: &mut TerminalModel, bytes: &[u8]) {
    let mut rest = bytes;
    while !rest.is_empty() {
        let taken = model.replay(rest).consumed;
        if taken == 0 {
            break;
        }
        rest = rest.get(taken..).unwrap_or_default();
    }
}
