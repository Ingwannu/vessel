//! The board: four lanes of session cards across every machine (DESIGN.md
//! section 6.12: a lane's heading in its colour with a count; a card's
//! title in two lines, its phrase, and machine, agent and age at its foot).

use chrono::Utc;
use vessel_app_kit::ui::{
    Context, Div, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, dot};
use vessel_app_view::{Card, Lane};

use super::{card_color, title_bar};
use crate::shell::{Open, Shell};

pub(crate) fn render(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let now = shell.fleet.read_at.unwrap_or_else(Utc::now);
    let count: usize = shell
        .fleet
        .machines
        .iter()
        .map(|machine| machine.sessions.len())
        .sum();
    let mut lanes = div().flex_1().flex().flex_row().min_h(px(0.0));
    for lane in Lane::ALL {
        lanes = lanes.child(lane_column(lane, shell, tokens, now, cx));
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(title_bar(
            "Board".to_owned(),
            Some(format!(
                "{} machines · {count} sessions",
                shell.fleet.machines.len()
            )),
            tokens,
        ))
        .child(lanes)
}

fn lane_color(lane: Lane, tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    match lane {
        Lane::Building => tokens.st_building,
        Lane::Validating => tokens.st_validating,
        Lane::InReview => tokens.st_review,
        Lane::Ready => tokens.st_ready,
    }
}

fn lane_column(
    lane: Lane,
    shell: &Shell,
    tokens: &Tokens,
    now: chrono::DateTime<Utc>,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let cards = shell.fleet.lane(lane);
    let mut list = div().flex().flex_col().gap(px(8.0)).p(px(10.0));
    for card in &cards {
        list = list.child(card_view(card, tokens, now, cx));
    }
    div()
        .flex_1()
        .min_w(px(180.0))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(tokens.border_faint)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .px(px(14.0))
                .pt(px(12.0))
                .pb(px(4.0))
                .child(dot(lane_color(lane, tokens)))
                .child(
                    div()
                        .type_(Type::Kicker)
                        .text_color(lane_color(lane, tokens))
                        .child(lane.label()),
                )
                .child(
                    div()
                        .type_(Type::Kicker)
                        .text_color(tokens.text_tertiary)
                        .child(cards.len().to_string()),
                ),
        )
        .child(
            div()
                .id(vessel_app_kit::ui::ElementId::Name(
                    format!("lane-{}", lane.label()).into(),
                ))
                .flex_1()
                .overflow_y_scroll()
                .child(list),
        )
}

fn card_view(
    card: &Card,
    tokens: &Tokens,
    now: chrono::DateTime<Utc>,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let open = Open {
        machine: card.machine.clone(),
        session: card.session.clone(),
        title: card.title.clone(),
        agent: card.agent.clone(),
        structured: card.structured,
    };
    let phrase_color = card_color(card, tokens);
    div()
        .id(vessel_app_kit::ui::ElementId::Name(
            format!("card-{}", card.session).into(),
        ))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(10.0))
        .bg(tokens.bg_surface)
        .border_1()
        .border_color(if card.needs_you {
            tokens.st_needs_you
        } else {
            tokens.border_subtle
        })
        .rounded(px(vessel_app_kit::theme::radius::R4))
        .cursor_pointer()
        .hover(|style| style.border_color(tokens.border_strong))
        .child(
            div()
                .type_(Type::Card)
                .text_color(tokens.text_primary)
                .line_clamp(2)
                .child(card.title.clone()),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(dot(phrase_color))
                .child(
                    div()
                        .type_(Type::Tiny)
                        .text_color(if card.needs_you || card.working {
                            phrase_color
                        } else {
                            tokens.text_secondary
                        })
                        .child(card.phrase.clone()),
                ),
        )
        .child(
            div()
                .type_(Type::Tiny)
                .text_color(tokens.text_tertiary)
                .child(format!(
                    "{} · {} · {}",
                    card.machine,
                    card.agent,
                    card.since(now)
                )),
        )
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.open_session(open.clone(), window, cx);
        }))
}
