//! The archive (the page's): every session in no lane, ended or archived,
//! newest first, each opening its conversation read-only.

use chrono::Utc;
use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;

use super::common::title_bar;
use super::session::Open;
use crate::shell::Shell;

pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let now = shell.fleet.read_at.unwrap_or_else(Utc::now);
    let cards = shell.fleet.archive();
    let mut rows = div().flex().flex_col();
    if cards.is_empty() {
        rows = rows.child(
            div()
                .p(px(24.0))
                .type_(Type::Body)
                .text_color(tokens.text_secondary)
                .child(shell.t.nothing_ended),
        );
    }
    for card in &cards {
        rows = rows.child(row(card, shell, tokens, now, cx));
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(
            shell.t.archive.to_owned(),
            Some(shell.t.sessions(cards.len())),
            tokens,
        ))
        .child(div().id("archive").flex_1().overflow_y_scroll().child(rows))
}

fn row(
    card: &Card,
    shell: &Shell,
    tokens: &Tokens,
    now: chrono::DateTime<Utc>,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let open = Open::of(card);
    div()
        .id(ElementId::Name(format!("archived-{}", card.session).into()))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(tokens.border_faint)
        .cursor_pointer()
        .hover(|style| style.bg(tokens.bg_hover))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Label)
                .text_color(tokens.text_primary)
                .child(shell.t.title(&card.title).to_owned()),
        )
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.text_tertiary)
                .child(format!(
                    "{} · {} · {} · {}",
                    shell.t.phrase(&card.phrase),
                    card.machine,
                    card.agent,
                    shell.t.since(card.minutes_since(now))
                )),
        )
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_session(open.clone(), window, cx);
        }))
}
