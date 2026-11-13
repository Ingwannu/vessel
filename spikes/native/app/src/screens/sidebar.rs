//! The sidebar: the machines in the book, each with its live sessions
//! (DESIGN.md section 6: rows on the shell, no edge of its own), the
//! board's link, adding a machine, and the build's label at its foot.

use vessel_app_kit::ui::{
    Context, Div, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, dot, kicker};
use vessel_app_view::{Card, Machine, Reach};

use super::card_color;
use crate::shell::{Open, Screen, Shell};

pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let board_active = shell.screen == Screen::Board;
    let mut machines = div().flex().flex_col().gap(px(14.0));
    for machine in &shell.fleet.machines {
        machines = machines.child(machine_rows(machine, shell, tokens, cx));
    }
    if shell.loaded && shell.fleet.machines.is_empty() {
        machines = machines.child(
            div()
                .px(px(10.0))
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child("No machines yet. Add one to begin."),
        );
    }
    div()
        .w(px(264.0))
        .h_full()
        .flex()
        .flex_col()
        .bg(tokens.bg_shell)
        .p(px(8.0))
        .gap(px(12.0))
        .child(
            div()
                .px(px(10.0))
                .pt(px(6.0))
                .type_(Type::Title)
                .child("Vessel"),
        )
        .child(
            row("board", "Board", board_active, tokens).on_click(
                cx.listener(|this, _event, window, cx| this.show(Screen::Board, window, cx)),
            ),
        )
        .child(
            div()
                .id("machines")
                .flex_1()
                .overflow_y_scroll()
                .child(machines),
        )
        .child(
            row(
                "add-machine",
                "Add a machine",
                shell.screen == Screen::Pair,
                tokens,
            )
            .on_click(cx.listener(|this, _event, window, cx| this.show(Screen::Pair, window, cx))),
        )
        .child(
            div()
                .id("build")
                .px(px(10.0))
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .cursor_pointer()
                .child(format!("Vessel {} · reload", crate::LABEL))
                .on_click(cx.listener(|this, _event, _window, _cx| this.reload())),
        )
}

fn machine_rows(
    machine: &Machine,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let (state, color) = match &machine.reach {
        Reach::Online => ("online", tokens.st_ready),
        Reach::Unreachable(_) => ("unreachable", tokens.danger),
    };
    let mut rows = div().flex().flex_col().gap(px(2.0)).child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .px(px(10.0))
            .pb(px(2.0))
            .child(kicker(machine.name.clone(), tokens))
            .child(dot(color))
            .child(
                div()
                    .type_(Type::Tiny)
                    .text_color(tokens.text_tertiary)
                    .child(state),
            ),
    );
    if let Reach::Unreachable(why) = &machine.reach {
        rows = rows.child(
            div()
                .px(px(10.0))
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(why.clone()),
        );
    }
    for card in machine.sessions.iter().filter(|card| card.lane.is_some()) {
        rows = rows.child(session_row(card, shell, tokens, cx));
    }
    rows
}

fn session_row(
    card: &Card,
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let active = shell.screen == Screen::Session
        && shell
            .open
            .as_ref()
            .is_some_and(|open| open.session == card.session);
    let open = Open {
        machine: card.machine.clone(),
        session: card.session.clone(),
        title: card.title.clone(),
        agent: card.agent.clone(),
        structured: card.structured,
    };
    div()
        .id(vessel_app_kit::ui::ElementId::Name(
            format!("session-{}", card.session).into(),
        ))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(10.0))
        .py(px(5.0))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .child(dot(card_color(card, tokens)))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Label)
                .text_color(if active {
                    tokens.text_primary
                } else {
                    tokens.text_secondary
                })
                .child(card.title.clone()),
        )
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_session(open.clone(), window, cx);
        }))
}

fn row(
    id: &'static str,
    label: &'static str,
    active: bool,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    div()
        .id(id)
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .type_(Type::Label)
        .text_color(if active {
            tokens.text_primary
        } else {
            tokens.text_secondary
        })
        .when(active, |row| row.bg(tokens.bg_active))
        .hover(|style| style.bg(tokens.bg_hover))
        .cursor_pointer()
        .child(label)
}

use vessel_app_kit::ui::FluentBuilder as _;
