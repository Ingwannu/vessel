//! The timeline on screen: the kit's long list over the rows, each row in
//! the session's one chat column; a run of tool calls
//! folded into its summary row, a run of the agent's log lines into one
//! row opening to a box; and what floats over the frame (the
//! Session actions menu, a confirm).

use vessel_app_kit::output::Lines;
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{
    DialogContent, DialogSize, Start, Tokens, Type, TypeExt as _, breakpoint, button, dialog,
    lines_box, menu,
};
use vessel_app_view::session::is_live;
use vessel_app_view::timeline::{EntryKind, Logs};

use super::entries::{self, Paint, id};
use super::look::{Look, WORK_LINE, WORK_SIZE};
use super::tail::TailRow;
use super::{Confirm, ConversationArea, Seat};
use crate::shell::Shell;

/// The conversation's body: its rows in the long list, or the empty
/// lane's words; the find bar and the outline rail over them.
pub(crate) fn timeline(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
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
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Some(area) = (seat.get)(shell) else {
        return div().into_any_element();
    };
    area.bench
        .frame(area.timeline.entries().len(), &area.list, window);
    if area.timeline.entries().is_empty() {
        let paint = paint(shell, area, seat, tokens, window);
        return entries::empty(&paint).into_any_element();
    }
    // A fork's origin's rows come first (`origin`).
    let head = area.head();
    area.list
        .element(
            cx.processor(move |shell: &mut Shell, row: usize, window, cx| {
                let tokens = vessel_app_kit::tokens(cx);
                match row.checked_sub(head) {
                    Some(own) => row_element(shell, seat, own, &tokens, window, cx),
                    None => row_element(shell, Seat::ORIGIN, row, &tokens, window, cx),
                }
            }),
        )
        .into_any_element()
}

/// How the rows of `area` (kept at `seat`) are drawn.
fn paint<'a>(
    shell: &'a Shell,
    area: &'a ConversationArea,
    seat: Seat,
    tokens: &'a Tokens,
    window: &Window,
) -> Paint<'a> {
    Paint {
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
        fleet: &shell.fleet,
        above: area.above(),
    }
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
    let Some(area) = (seat.get)(shell) else {
        return div().into_any_element();
    };
    let paint = paint(shell, area, seat, tokens, window);
    if let Some(tail) = area.tail_row(row) {
        return lane(match tail {
            TailRow::Pending(text) => {
                let index = area.timeline.entries().len();
                area.pending_drawn();
                let row = super::bubbles::pending(&paint, index, &text);
                if area.pending_fresh() {
                    super::motion::settle(
                        ElementId::NamedInteger("pending-arrive".into(), id(index)),
                        row,
                    )
                } else {
                    row
                }
            }
            TailRow::Working(since) => super::footer::working(&paint, since),
        });
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
    if quiet(&entry.kind) || area.timeline.rewound(index) {
        return div().into_any_element();
    }
    let run = area
        .folds
        .as_ref()
        .and_then(|folds| folds.run_of(index))
        .filter(|run| run.lead == index);
    let mut content = div().flex().flex_col().gap(px(2.0));
    let run_open = run.is_some_and(|run| area.open_runs.contains(&run.lead));
    if let Some(run) = run {
        content = content.child(run_header(
            &paint,
            ("run", run.lead),
            paint.words.run_summary(&run.summary),
            run_open,
            Shell::toggle_run,
            cx,
        ));
    }
    if run.is_none() || run_open {
        let drawn = entries::entry(&paint, index, entry, open(area, index), window, cx);
        // A person's message carries Paseo's trailing row: its time,
        // rewind where it can go, copy (`rewind`).
        let said = if let EntryKind::Prompt(prompt) = &entry.kind {
            Some(prompt.text.as_str())
        } else if let EntryKind::Steer { text, .. } = &entry.kind {
            Some(text.as_str())
        } else {
            None
        };
        let drawn = if let Some(text) = said {
            super::rewind::with_row(
                &paint,
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
        content = content.child(if arriving {
            super::motion::arrive(
                ElementId::NamedInteger("reply-arrive".into(), id(index)),
                drawn,
            )
        } else {
            drawn
        });
    }
    lane(content.into_any_element())
}

/// Whether entry `index`'s disclosure is open: a tool call while it
/// works, else closed; turned by the person. A tool call whose permission
/// waits stays closed: the card over the composer shows its detail.
fn open(area: &ConversationArea, index: usize) -> bool {
    let default = area
        .timeline
        .entries()
        .get(index)
        .and_then(|entry| entry.kind.as_tool())
        .is_some_and(|tool| tool.working() && !area.timeline.asks_permission(&tool.id));
    default != area.toggled.contains(&index)
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

/// A folded run's summary row (GOAT's work group header: its words and a
/// chevron), for tool calls; a click turns it with `toggle`. `name` and
/// the lead entry make its id. No rule under it (slice M): Paseo's
/// summary row (`agent-stream/turn-collapse/summary-line.tsx`, its
/// `ExpandableBadge`) has none, and a rule under every run read as a rule
/// between turns.
fn run_header(
    paint: &Paint<'_>,
    (name, lead): (&'static str, usize),
    summary: String,
    open: bool,
    toggle: fn(&mut Shell, Seat, usize, &mut Context<'_, Shell>),
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let hover = paint.tokens.neutral.foreground;
    let seat = paint.seat;
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
        .on_click(cx.listener(move |shell, _event, _window, cx| toggle(shell, seat, lead, cx)))
        .child(entries::disclosure(open, paint.look.secondary))
        .child(div().min_w(px(0.0)).truncate().child(summary))
        .into_any_element()
}

/// The menus and tooltips of going back in the conversation: rewind
/// under a person's message (`rewind`), fork under a turn's footer
/// (`fork`).
fn going_back(
    shell: &Shell,
    tokens: &Tokens,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let area = &shell.areas.conversation;
    let mut floating = Vec::new();
    floating.extend(menu(
        "rewind-menu",
        &area.rewinding.menu,
        &super::rewind::menu_items(shell),
        &Shell::rewind_menu_handlers(),
        tokens,
        narrow,
        cx,
    ));
    floating.extend(area.rewinding.tips.render(tokens));
    floating.extend(menu(
        "fork-menu",
        &area.forking.menu,
        &super::fork::menu_items(shell),
        &Shell::fork_menu_handlers(),
        tokens,
        narrow,
        cx,
    ));
    floating.extend(area.forking.tips.render(tokens));
    floating
}

/// What floats over the frame: the Session actions menu when open, the
/// confirm when one is asked.
pub(crate) fn overlays(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let area = &shell.areas.conversation;
    let narrow = breakpoint::phone(window);
    let words = shell.t.lang.conversation();
    let mut floating = Vec::new();
    floating.extend(super::search::example(shell, tokens, window, cx));
    floating.extend(going_back(shell, tokens, narrow, cx));
    let items = super::bar::menu_items(shell);
    if let Some(menu) = menu(
        "session-actions",
        &area.menu,
        &items,
        &Shell::menu_handlers(),
        tokens,
        narrow,
        cx,
    ) {
        floating.push(menu);
    }
    if let Some(confirm) = area.confirm {
        let machine = area
            .shown
            .as_ref()
            .map(|shown| shown.machine.clone())
            .unwrap_or_default();
        let live = area.info.as_deref().is_some_and(is_live);
        let short: String = area
            .shown
            .as_ref()
            .map(|shown| shown.session.to_string().chars().take(4).collect())
            .unwrap_or_default();
        let (title, body, accept) = match confirm {
            Confirm::Archive => (
                words.archive_title().to_owned(),
                words.archive_words(live, &machine, true),
                words.confirm(),
            ),
            Confirm::Delete => (
                words.delete_title().to_owned(),
                words.delete_words(&machine),
                words.delete(),
            ),
            Confirm::Kill => (
                words.kill_title(&short),
                words.kill_words(&machine),
                words.kill(),
            ),
        };
        let dismiss = match confirm {
            Confirm::Archive => words.no(),
            Confirm::Delete | Confirm::Kill => words.cancel(),
        };
        let footer = vec![
            button("confirm-dismiss", dismiss, false, tokens)
                .on_click(
                    cx.listener(|shell, _event, window, cx| shell.dismiss_confirm(window, cx)),
                )
                .into_any_element(),
            button("confirm-accept", accept, true, tokens)
                .when(confirm != Confirm::Archive, |accept| {
                    accept.bg(tokens.neutral.destructive)
                })
                .on_click(cx.listener(|shell, _event, _window, cx| shell.confirmed(cx)))
                .into_any_element(),
        ];
        floating.push(dialog(
            &area.dialog,
            DialogContent {
                title: title.into(),
                body: div().type_(Type::Body).child(body).into_any_element(),
                footer,
                size: DialogSize::Normal,
            },
            tokens,
            window,
            narrow,
            Shell::dismiss_confirm,
            cx,
        ));
    }
    if area.agent_log == super::AgentLog::Shown {
        floating.push(agent_log(shell, tokens, window, narrow, cx));
    }
    floating
}

/// The agent's own log (its adapter's lines on stderr), the whole of it
/// in one box: the Session actions' "Agent log". Its lines were rows
/// between turns; Paseo shows no adapter log in the stream (slice M).
fn agent_log(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    narrow: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let words = shell.t.lang.conversation();
    let lines = Logs::all(area.timeline.entries());
    let count = lines.lines().count();
    let body = div()
        .max_h(px(420.0))
        .child(lines_box(
            "agent-log-lines",
            u64::try_from(count).unwrap_or(u64::MAX),
            || Lines::text(&lines),
            Start::End,
            tokens,
            window,
            cx,
        ))
        .into_any_element();
    let footer = vec![
        button("agent-log-close", words.close(), false, tokens)
            .on_click(cx.listener(|shell, _event, window, cx| shell.dismiss_log(window, cx)))
            .into_any_element(),
    ];
    dialog(
        &area.dialog,
        DialogContent {
            title: words.agent_log(count).into(),
            body,
            footer,
            size: DialogSize::Wide,
        },
        tokens,
        window,
        narrow,
        Shell::dismiss_log,
        cx,
    )
}
