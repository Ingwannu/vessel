//! The timeline on screen: the kit's long list over the rows, each row in
//! the session's one chat column; a run of tool calls folded into its
//! summary row. What floats over the frame is `overlays`.

use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::session::is_live;
use vessel_app_view::timeline::{Entry, EntryKind, presented_prompt};

use super::entries::{self, Paint, id};
use super::look::{Look, WORK_LINE, WORK_SIZE};
use super::tail::TailRow;
use super::{ConversationArea, Seat};
use crate::shell::Shell;

/// The conversation's body: its rows in the long list, or the empty
/// lane's words; the find bar and the outline rail over them.
pub(crate) fn timeline(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    super::read::check(shell, window, cx);
    super::tail::sync(shell, window, cx);
    // A rewound message back into the composer, on a frame with a window.
    if shell.areas.conversation.rewinding.has_restore() {
        cx.defer_in(window, Shell::rewind_restore);
    }
    // A fork's new session opened, on a frame with a window.
    if shell.areas.conversation.forking.has_opening() {
        cx.defer_in(window, Shell::forked_open);
    }
    let list = rows(shell, Seat::SESSION, tokens, window, cx);
    if shell.areas.conversation.timeline.entries().is_empty() {
        return list;
    }
    let bar = super::find::bar(shell, tokens, window, cx);
    if breakpoint::phone(window) {
        return div()
            .flex_1()
            .min_h(px(0.0))
            .size_full()
            .face(Type::Body)
            .flex()
            .flex_col()
            .children(bar)
            .child(div().flex_1().min_h(px(0.0)).child(list))
            .into_any_element();
    }
    // Every row draws its words in the kit's faces with Pretendard behind
    // them, whether or not the row names its own type.
    div()
        .flex_1()
        .min_h(px(0.0))
        .size_full()
        .face(Type::Body)
        .relative()
        .child(list)
        .child(super::outline::width_probe(shell, cx))
        .children(super::outline::rail(shell, tokens, window, cx))
        .children(bar)
        .into_any_element()
}

/// The conversation kept at `seat` as its rows: the long list, or the
/// empty lane's words; every toggle changes the state at `seat`. Another
/// area embedding a conversation (a side question's panel) places this
/// inside an element with an id of its own, so its rows' ids do not meet
/// the session area's, and gives it a height (the list fills its parent).
pub(crate) fn rows(
    shell: &Shell,
    seat: Seat,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(area) = seat.get(shell) else {
        return div().into_any_element();
    };
    area.bench
        .frame(area.timeline.entries().len(), &area.list, window);
    if area.timeline.entries().is_empty() {
        return paint(shell, area, seat, tokens, window).map_or_else(
            || div().into_any_element(),
            |paint| entries::empty(&paint).into_any_element(),
        );
    }
    // A fork's origin's rows come first (`origin`).
    let head = area.head();
    let origin = seat.origin();
    let row_seat = seat.clone();
    let list = area
        .list
        .element(
            cx.processor(move |shell: &mut Shell, row: usize, window, cx| {
                let tokens = vessel_app_kit::tokens(cx);
                match row.checked_sub(head) {
                    Some(own) => row_element(shell, row_seat.clone(), own, &tokens, window, cx),
                    None => row_element(shell, origin.clone(), row, &tokens, window, cx),
                }
            }),
        )
        .into_any_element();
    div()
        .size_full()
        .relative()
        .child(list)
        .children(super::tool_sheet::overlay(shell, seat, tokens, window, cx))
        .into_any_element()
}

/// How the rows of `area` (kept at `seat`) are drawn.
pub(super) fn paint<'a>(
    shell: &'a Shell,
    area: &'a ConversationArea,
    seat: Seat,
    tokens: &'a Tokens,
    window: &Window,
) -> Option<Paint<'a>> {
    let headers = &seat.root_get(shell)?.forking.headers;
    Some(Paint {
        catalog: shell.words,
        words: shell.t.lang.conversation(),
        tokens,
        look: Look::of(tokens),
        find: area
            .find
            .as_ref()
            .and_then(|bar| bar.marking(&area.list, breakpoint::phone(window))),
        seat,
        entries: area.timeline.entries(),
        agent: area.info.as_deref().map(|info| &info.spec.harness),
        rewinds: rewinds(shell, area).then_some(&area.timeline),
        forks: std::ptr::eq(area, &raw const shell.areas.conversation)
            .then_some((&area.timeline, &area.forking)),
        fork_headers: headers,
        above: area.above(),
    })
}

/// Whether a rewind may be asked of `area`'s session now: the session
/// area's own conversation (not a side question's), live, between turns
/// (the node rewinds only then).
fn rewinds(shell: &Shell, area: &ConversationArea) -> bool {
    std::ptr::eq(area, &raw const shell.areas.conversation)
        && area.info.as_deref().is_some_and(is_live)
        && !shell.areas.session.conversation.turn_running()
}

/// Row `row`: its entry, in the lane; a folded run's lead with the run's
/// summary over it.
fn row_element(
    shell: &Shell,
    seat: Seat,
    row: usize,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(area) = seat.get(shell) else {
        return div().into_any_element();
    };
    let Some(paint) = paint(shell, area, seat, tokens, window) else {
        return div().into_any_element();
    };
    if let Some(tail) = area.tail_row(row) {
        return lane(tail_element(&paint, area, tail));
    }
    let Some(index) = area.rows.entry(row) else {
        return div().into_any_element();
    };
    let Some(entry) = area.timeline.entries().get(index) else {
        return div().into_any_element();
    };
    // Lifecycle and accounting lines are not drawn: Paseo's stream holds
    // messages, thoughts, tools, plans and questions; the session's state
    // is the header's, its usage the context meter's (slice M). What a
    // rewind left behind is no longer the conversation (slice R1).
    // A read-only origin/child has no approval dock; its pending plan remains
    // readable without introducing permission actions into the embedded stream.
    let waiting_plan = area.shown.is_some()
        && entry.kind.as_tool().is_some_and(|tool| {
            vessel_app_view::timeline::plan_text(tool).is_some()
                && area.timeline.asks_permission(&tool.id)
        });
    if quiet(&entry.kind) || area.timeline.rewound(index) || waiting_plan {
        return div().into_any_element();
    }
    let run = area
        .folds
        .as_ref()
        .and_then(|folds| folds.run_of(index))
        .filter(|run| run.lead == index)
        .filter(|_| {
            !area.turns.as_ref().is_some_and(|turns| {
                turns
                    .folds
                    .containing(index)
                    .is_some_and(|turn| turns.open.contains(&turn.host))
            })
        });
    let mut content = div().flex().flex_col().gap(px(2.0));
    let run_open = run.is_some_and(|run| area.open_runs.contains(&run.lead));
    if let Some(run) = run {
        content = content.child(run_header(
            &paint,
            (RunHeaderKind::Tools, run.lead),
            paint.words.run_summary(&run.summary),
            run_open,
            Shell::toggle_run,
            cx,
        ));
    }
    if run.is_none() || run_open {
        content = content.child(entry_element(&paint, area, (index, entry), window, cx));
    }
    lane(super::turns::decorate(&paint, area, index, content, cx))
}

/// A row after the log's own: the person's message still on its way
/// (settling in when fresh), or the working footer.
fn tail_element(paint: &Paint<'_>, area: &ConversationArea, tail: TailRow) -> AnyElement {
    match tail {
        TailRow::Pending(text) => {
            let index = area.timeline.entries().len();
            area.pending_drawn();
            let row = super::bubbles::pending(paint, index, &text);
            if area.pending_fresh() {
                super::motion::settle(
                    ElementId::NamedInteger("pending-arrive".into(), id(index)),
                    row,
                )
            } else {
                row
            }
        }
        TailRow::Working(since) => super::footer::working(paint, since),
    }
}

/// Entry `index` drawn: a person's message with Paseo's trailing row (its
/// time, rewind where it can go, copy), a reply that just came growing in.
fn entry_element(
    paint: &Paint<'_>,
    area: &ConversationArea,
    (index, entry): (usize, &Entry),
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let drawn = entries::entry(
        paint,
        index,
        entry,
        area.entry_expanded(index),
        super::pacing::presentation(area, entry),
        window,
        cx,
    );
    let said = if let EntryKind::Prompt(prompt) = &entry.kind {
        Some(presented_prompt(&prompt.text))
    } else if let EntryKind::Steer { text, .. } = &entry.kind {
        Some(presented_prompt(text))
    } else {
        None
    };
    let drawn = if let Some(text) = said.filter(|text| !text.is_empty()) {
        super::rewind::with_row(
            paint,
            index,
            (drawn, text, entry.at),
            &area.rewinding,
            breakpoint::phone(window),
            window,
            cx,
        )
    } else {
        drawn
    };
    // A reply that just came grows in (`motion`).
    let arriving = matches!(entry.kind, EntryKind::Message { .. })
        && area
            .fresh
            .is_some_and(|(from, at)| index >= from && at.elapsed() < super::motion::FRESH);
    if arriving {
        super::motion::arrive(
            ElementId::NamedInteger("reply-arrive".into(), id(index)),
            drawn,
        )
    } else {
        drawn
    }
}

/// The entries the conversation keeps but does not draw (the owner's
/// report on 0.0.0.0.13; Paseo's stream has no such lines): the
/// session's status changes (its state is the header's chip), the agent
/// being ready, released or resumed (the header's), model calls (the
/// gateway's accounting) and the agent's own log lines (the Session
/// actions' "Agent log", in a dialog: the coordinator on slice M).
const fn quiet(kind: &EntryKind) -> bool {
    matches!(
        kind,
        EntryKind::Status { .. }
            | EntryKind::Ready { .. }
            | EntryKind::Parking(_)
            | EntryKind::Model(_)
            | EntryKind::Log { .. }
    )
}

/// A row in the session's one chat column (the session area's: the
/// conversation and the composer share it).
fn lane(content: AnyElement) -> AnyElement {
    crate::areas::session::column(div().py(px(4.0)).child(content)).into_any_element()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RunHeaderKind {
    Tools,
    Turn,
}

/// A folded run's summary row (GOAT's work group header: its words and a
/// chevron), for tool calls; a click turns it with `toggle`. `name` and
/// the lead entry make its id. No rule under it (slice M): Paseo's
/// summary row (`agent-stream/turn-collapse/summary-line.tsx`, its
/// `ExpandableBadge`) has none, and a rule under every run read as a rule
/// between turns.
pub(super) fn run_header(
    paint: &Paint<'_>,
    (kind, lead): (RunHeaderKind, usize),
    summary: String,
    open: bool,
    toggle: fn(&mut Shell, &Seat, usize, &mut Context<'_, Shell>),
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let hover = paint.tokens.neutral.foreground;
    let seat = paint.seat.clone();
    let name = match kind {
        RunHeaderKind::Tools => "run",
        RunHeaderKind::Turn => "turn",
    };
    div()
        .id(ElementId::NamedInteger(name.into(), id(lead)))
        .flex()
        .items_center()
        .gap(px(4.0))
        .pb(px(6.0))
        .face(Type::Meta)
        .text_size(px(vessel_app_kit::fonts::ui(WORK_SIZE)))
        .line_height(px(vessel_app_kit::fonts::ui(WORK_LINE)))
        .text_color(paint.look.secondary)
        .cursor_pointer()
        .hover(move |row| row.text_color(hover))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            if kind == RunHeaderKind::Tools
                && breakpoint::phone(window)
                && shell.run_sheet(&seat, lead, window, cx)
            {
                return;
            }
            toggle(shell, &seat, lead, cx);
        }))
        .child(entries::disclosure(open, paint.look.secondary))
        .child(div().min_w(px(0.0)).truncate().child(summary))
        .into_any_element()
}
