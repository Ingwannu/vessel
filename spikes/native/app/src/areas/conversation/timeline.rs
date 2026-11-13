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
use vessel_app_view::timeline::Logs;

use super::entries::{self, Paint, id};
use super::look::{Look, WORK_LINE, WORK_SIZE};
use super::{Confirm, ConversationArea};
use crate::shell::Shell;

/// The conversation's body: its rows in the long list, or the empty
/// lane's words.
pub(crate) fn timeline(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    area.bench
        .frame(area.timeline.entries().len(), &area.list, window);
    let paint = Paint {
        words: shell.t.lang.conversation(),
        tokens,
        look: Look::of(tokens),
        find: area
            .find
            .as_ref()
            .and_then(|bar| bar.marking(&area.list, breakpoint::phone(window))),
    };
    if area.timeline.entries().is_empty() {
        return entries::empty(&paint).into_any_element();
    }
    let list = area
        .list
        .element(cx.processor(|shell: &mut Shell, row: usize, window, cx| {
            let tokens = vessel_app_kit::tokens(cx);
            row_element(shell, row, &tokens, window, cx)
        }));
    let bar = super::find::bar(shell, tokens, window, cx);
    if breakpoint::phone(window) {
        return div()
            .flex_1()
            .min_h(px(0.0))
            .size_full()
            .flex()
            .flex_col()
            .children(bar)
            .child(div().flex_1().min_h(px(0.0)).child(list))
            .into_any_element();
    }
    div()
        .flex_1()
        .min_h(px(0.0))
        .size_full()
        .relative()
        .child(list)
        .children(bar)
        .into_any_element()
}

/// Row `row`: its entry, in the lane; a folded run's lead with the run's
/// summary over it.
fn row_element(
    shell: &Shell,
    row: usize,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let area = &shell.areas.conversation;
    let paint = Paint {
        words: shell.t.lang.conversation(),
        tokens,
        look: Look::of(tokens),
        find: area
            .find
            .as_ref()
            .and_then(|bar| bar.marking(&area.list, breakpoint::phone(window))),
    };
    let Some(index) = area.rows.entry(row) else {
        return div().into_any_element();
    };
    let Some(entry) = area.timeline.entries().get(index) else {
        return div().into_any_element();
    };
    // A run of the agent's own log lines is one row at its first line
    // (never collapsed into rows of their own: `view::timeline::logs`).
    if let Some(run) = area.logs.run_of(index).filter(|run| run.lead == index) {
        let open = area.open_logs.contains(&run.lead);
        let mut content = div().flex().flex_col().gap(px(4.0)).child(run_header(
            &paint,
            ("log", run.lead),
            paint.words.agent_log(run.lines()),
            open,
            Shell::toggle_log,
            cx,
        ));
        if open {
            let entries = area.timeline.entries();
            content = content.child(div().ml(px(28.0)).mb(px(6.0)).child(lines_box(
                ElementId::NamedInteger("log-lines".into(), id(run.lead)),
                u64::try_from(run.lines()).unwrap_or(u64::MAX),
                || Lines::text(&Logs::text(run, entries)),
                Start::Top,
                tokens,
                window,
                cx,
            )));
        }
        return lane(content.into_any_element());
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
        content = content.child(entries::entry(
            &paint,
            index,
            entry,
            open(area, index),
            window,
            cx,
        ));
    }
    lane(content.into_any_element())
}

/// Whether entry `index`'s disclosure is open: a tool call while it
/// works, else closed; turned by the person.
fn open(area: &ConversationArea, index: usize) -> bool {
    let default = area
        .timeline
        .entries()
        .get(index)
        .and_then(|entry| entry.kind.as_tool())
        .is_some_and(vessel_app_view::timeline::ToolView::working);
    default != area.toggled.contains(&index)
}

/// A row in the session's one chat column (the session area's: the
/// conversation and the composer share it).
fn lane(content: AnyElement) -> AnyElement {
    crate::areas::session::column(div().py(px(4.0)).child(content)).into_any_element()
}

/// A folded run's summary row (GOAT's work group header: its words and a
/// chevron, a rule under it), for tool calls or the agent's log; a click
/// turns it with `toggle`. `name` and the lead entry make its id.
fn run_header(
    paint: &Paint<'_>,
    (name, lead): (&'static str, usize),
    summary: String,
    open: bool,
    toggle: fn(&mut Shell, usize, &mut Context<'_, Shell>),
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let hover = paint.tokens.neutral.foreground;
    div()
        .id(ElementId::NamedInteger(name.into(), id(lead)))
        .flex()
        .items_center()
        .gap(px(4.0))
        .pb(px(6.0))
        .border_b_1()
        .border_color(paint.look.border)
        .text_size(px(WORK_SIZE))
        .line_height(px(WORK_LINE))
        .text_color(paint.look.secondary)
        .cursor_pointer()
        .hover(move |row| row.text_color(hover))
        .on_click(cx.listener(move |shell, _event, _window, cx| toggle(shell, lead, cx)))
        .child(entries::disclosure(open, paint.look.secondary))
        .child(div().min_w(px(0.0)).truncate().child(summary))
        .into_any_element()
}

/// What floats over the frame: the Session actions menu when open, the
/// confirm when one is asked.
pub(crate) fn overlays(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let area = &shell.areas.conversation;
    let narrow = breakpoint::phone(window);
    let words = shell.t.lang.conversation();
    let mut floating = Vec::new();
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
    floating
}
