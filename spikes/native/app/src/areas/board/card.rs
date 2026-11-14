//! A session's card (the page's `session-card.tsx` and its CSS): who
//! started it when not its owner; its agent's mark, its title in two
//! lines, its pin and unread marks and its action; its folder (cut from
//! the left, where two paths differ), its branch, its short id and agent;
//! its status line in its tone with a spinner while it works, and where
//! and how long ago; a failed session's error. A card that needs its
//! owner is tinted with the needs-you colour. The whole card opens the
//! session; its links and buttons are their own targets.
//!
//! [Decision Log]
//! - Purpose: the page's card, every line of it.
//! - Choice: the action slot shows on the card's hover (always at a
//!   phone's width, where there is no hover), hidden rather than
//!   transparent, so a hidden button is not clicked.
//! - Cost: the page's 3.5 s breath on a needs-you card is not drawn: the
//!   tint and the border stay still (one animation per such card, every
//!   frame, for a word the card already says).

use chrono::{DateTime, Utc};
use gpui::FontWeight;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::slider::spinner;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, Hsla, InteractiveElement as _,
    IntoElement as _, MouseButton, ParentElement as _, Stateful, StatefulInteractiveElement as _,
    Styled, div, px, rgb,
};
use vessel_app_kit::{Scheme, Tokens, Type, TypeExt as _, alert, fonts::MONO, glyph, popup};
use vessel_app_view::Card;
use vessel_app_view::board::{CardActions, Origin, Row, Tone};

use super::acts::{Ask, Asking};
use super::lane_color;
use crate::areas::session::Open;
use crate::shell::Shell;

/// The hover group of one card (its action slot shows on it).
const GROUP: &str = "board-card";

/// Where a card is drawn.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Place {
    /// In the archive (its actions restore and delete; its tone is the
    /// archive's).
    pub(crate) archive: bool,
    /// The board shows one machine (the foot leaves its name out).
    pub(crate) scoped: bool,
    /// The board's clock.
    pub(crate) now: DateTime<Utc>,
    /// A phone's width: the actions always show.
    pub(crate) narrow: bool,
}

/// One card.
pub(crate) fn card(
    shell: &Shell,
    row: &Row<'_>,
    place: Place,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let card = row.card;
    let neutral = tokens.neutral;
    let base = neutral.surface1;
    let bg = if card.needs_you() {
        base.blend(tokens.st_needs_you.opacity(0.08))
    } else {
        base
    };
    let hover = bg.blend(neutral.foreground.opacity(0.04));
    let lane = if place.archive { None } else { card.lane };
    let tone = tone_color(card.tone(), lane, tokens);
    let open = Open::of(card);
    let head = div()
        .flex()
        .flex_col()
        .px(px(14.0))
        .py(px(10.0))
        .children(origin(shell, row, tokens, cx))
        .child(title_row(shell, card, place, tokens, cx))
        .children(lines(card, tokens))
        .children(crate::areas::pulls::card_items(card, shell.words, tokens));
    let foot = foot(shell, card, place, tone, tokens);
    // A failed session's error is a failure on screen: the kit's alert.
    let error = card.detail.failure.clone().map(|failure| {
        div()
            .px(px(spacing::S3_5))
            .pb(px(spacing::S2))
            .child(alert::alert(
                alert::Content {
                    title: Some(failure.into()),
                    description: None,
                    action: None,
                },
                alert::Variant::Error,
                alert::Size::ExtraSmall,
                tokens,
            ))
    });
    div()
        .id(ElementId::Name(format!("card-{}", card.session).into()))
        .group(GROUP)
        .flex()
        .flex_col()
        .flex_none()
        .bg(bg)
        .border_1()
        .border_color(if card.needs_you() {
            tokens.st_needs_you
        } else {
            neutral.border
        })
        .rounded(px(popup::MENU_RADIUS))
        .overflow_hidden()
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.open_session(open.clone(), window, cx);
        }))
        // In the archive's grid a card is as tall as its row, its foot
        // at the bottom.
        .child(head.when(place.archive, Styled::flex_1))
        .child(foot)
        .children(error)
        .into_any_element()
}

/// The card's foot: what it is doing, in its tone, and where and since
/// when.
fn foot(shell: &Shell, card: &Card, place: Place, tone: Hsla, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    let since = shell.t.since(card.minutes_since(place.now));
    let meta = if place.scoped {
        since
    } else {
        format!("{} · {since}", card.machine)
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .px(px(14.0))
        .py(px(10.0))
        .border_t_1()
        .border_color(neutral.border)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .flex_1()
                .min_w(px(0.0))
                .gap(px(4.0))
                .children(card.is_working().then(|| {
                    spinner(
                        ElementId::Name(format!("card-spin-{}", card.session).into()),
                        12.0,
                        tone,
                    )
                }))
                .child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .type_(Type::Tiny)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tone)
                        .child(shell.t.phrase(&card.phrase)),
                ),
        )
        .child(
            div()
                .flex_none()
                .whitespace_nowrap()
                .type_(Type::Tiny)
                .text_color(neutral.foreground_muted)
                .child(meta),
        )
}

/// The colour of a card's status line: its lane's in a lane, unless its
/// phrase says more (the page's `tone-*`).
fn tone_color(tone: Tone, lane: Option<vessel_app_view::Lane>, tokens: &Tokens) -> Hsla {
    match tone {
        Tone::Lane => lane.map_or(tokens.neutral.foreground_extra_muted, |lane| {
            lane_color(lane, tokens)
        }),
        Tone::NeedsYou => tokens.st_needs_you,
        Tone::Ready => tokens.st_ready,
        // The page's `--st-merged`; the theme has no merged colour yet.
        Tone::Merged => match tokens.scheme {
            Scheme::Light => rgb(0x0093_33ea).into(),
            Scheme::Dark => rgb(0x00c0_84fc).into(),
        },
        Tone::Danger => tokens.neutral.destructive,
        Tone::Quiet => tokens.neutral.foreground_extra_muted,
    }
}

/// Who started it, when not its machine's owner: a paired device, or
/// another session's agent (its id opening that session when listed).
fn origin(
    shell: &Shell,
    row: &Row<'_>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let words = shell.t.lang.board();
    let neutral = tokens.neutral;
    let tag = div()
        .flex()
        .flex_row()
        .items_center()
        .mb(px(6.0))
        .whitespace_nowrap()
        .type_(Type::Tiny)
        .text_color(neutral.foreground_extra_muted);
    match row.origin() {
        Origin::Owner => None,
        Origin::Device(id) => Some(tag.child(words.device(&id))),
        Origin::Agent {
            id,
            parent,
            elsewhere,
        } => {
            let short = div().font_family(MONO).child(id);
            let short = match parent {
                Some(parent) => {
                    let open = Open::of(parent);
                    short
                        .id(ElementId::Name(
                            format!("card-parent-{}", row.card.session).into(),
                        ))
                        .text_color(neutral.foreground_muted)
                        .underline()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(cx.listener(move |shell, _event, window, cx| {
                            cx.stop_propagation();
                            shell.open_session(open.clone(), window, cx);
                        }))
                        .into_any_element()
                }
                None => short.into_any_element(),
            };
            Some(
                tag.child(words.agent_of)
                    .child(short)
                    .child(words.agent_of_after)
                    .children(elsewhere.map(|machine| words.elsewhere(machine))),
            )
        }
    }
}

/// The agent's mark, the title, the marks, the action.
fn title_row(
    shell: &Shell,
    card: &Card,
    place: Place,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let marks = card.detail.marks.pinned || card.detail.marks.unread;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.0))
        .child(crate::agent_mark::mark(
            card.detail.harness.as_ref(),
            vessel_app_kit::agent_mark::Size::Fourteen,
            vessel_app_kit::agent_mark::Space::Compact,
            tokens,
        ))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .type_(Type::Card)
                .text_color(neutral.foreground)
                .line_clamp(2)
                .text_ellipsis()
                .child(shell.t.title(&card.title).to_owned()),
        )
        .when(marks, |row| {
            row.child(
                div()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap(px(6.0))
                    .when(card.detail.marks.pinned, |marks| {
                        marks.child(glyph(Glyph::Pin, 12.0, neutral.foreground_extra_muted))
                    })
                    .when(card.detail.marks.unread, |marks| {
                        marks.child(
                            div()
                                .size(px(8.0))
                                .rounded_full()
                                .bg(neutral.foreground_muted.opacity(0.6)),
                        )
                    }),
            )
        })
        .children(actions(shell, card, place, tokens, cx))
}

/// The folder, the branch, the id and who.
fn lines(card: &Card, tokens: &Tokens) -> Vec<Div> {
    let neutral = tokens.neutral;
    let line = || {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .min_w(px(0.0))
            .mt(px(6.0))
            .whitespace_nowrap()
            .type_(Type::Tiny)
            .text_color(neutral.foreground_muted)
    };
    let mode = if card.structured {
        Glyph::Message
    } else {
        Glyph::Terminal
    };
    let mut lines = vec![
        line()
            .child(glyph(mode, 12.0, neutral.foreground_muted))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis_start()
                    .font_family(MONO)
                    .child(card.folder().to_owned()),
            ),
    ];
    if let Some(branch) = &card.detail.branch {
        lines.push(
            line()
                .child(glyph(Glyph::Branch, 12.0, neutral.foreground_extra_muted))
                .child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .font_family(MONO)
                        .text_size(px(11.0))
                        .line_height(px(16.0))
                        .text_color(neutral.foreground_extra_muted)
                        .child(branch.clone()),
                ),
        );
    }
    lines.push(
        line()
            .child(
                div()
                    .flex_none()
                    .font_family(MONO)
                    .text_color(neutral.foreground_extra_muted)
                    .child(card.short_id()),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(card.who()),
            ),
    );
    lines
}

/// The card's action slot: kill or archive in a lane, restore and delete
/// in the archive; quiet while an act it asked is unanswered.
fn actions(
    shell: &Shell,
    card: &Card,
    place: Place,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Div> {
    let busy = shell
        .areas
        .board
        .asked
        .iter()
        .any(|asked| asked.machine == card.machine && asked.session == card.session);
    let id = |what: &str| ElementId::Name(format!("card-{what}-{}", card.session).into());
    let asking = |ask: Ask| Asking::of(ask, card);
    let words = shell.t.lang.board();
    // Each button says what it does when pointed at (the page's
    // `Tooltip`s), by the kit's tooltip; none on a phone, as Paseo's.
    let tip = |button: Stateful<Div>, what: &str, text: &'static str| {
        let key = format!("{what}-{}-{}", card.machine, card.session);
        let board = &shell.areas.board;
        board
            .tips
            .on(button, &board.place(&key), text, place.narrow)
    };
    let buttons: Vec<AnyElement> =
        match card.actions(shell.fleet.protocol_of(&card.machine), place.archive) {
            CardActions::None => return None,
            CardActions::Kill => {
                let asking = asking(Ask::Kill);
                let button = icon_button(
                    id("kill"),
                    Glyph::Stop,
                    Some(tokens.neutral.destructive),
                    !busy,
                    tokens,
                )
                .when(!busy, |button| {
                    button.on_click(cx.listener(move |shell, _event, window, cx| {
                        cx.stop_propagation();
                        shell.ask_card(asking.clone(), window, cx);
                    }))
                });
                vec![tip(button, "kill", words.kill).into_any_element()]
            }
            CardActions::Archive => {
                let asking = asking(Ask::Archive);
                let button = icon_button(id("archive"), Glyph::Archive, None, !busy, tokens).when(
                    !busy,
                    |button| {
                        button.on_click(cx.listener(move |shell, _event, window, cx| {
                            cx.stop_propagation();
                            shell.ask_card(asking.clone(), window, cx);
                        }))
                    },
                );
                vec![tip(button, "archive", words.archive_session).into_any_element()]
            }
            CardActions::Restore { again } => {
                let restore = asking(Ask::Delete);
                let delete = restore.clone();
                let restore_button = icon_button(id("restore"), Glyph::Reset, None, !busy, tokens)
                    .when(!busy, |button| {
                        button.on_click(cx.listener(move |shell, _event, _window, cx| {
                            cx.stop_propagation();
                            shell.restore_card(&restore, again, cx);
                        }))
                    });
                let delete_button = icon_button(id("delete"), Glyph::Trash, None, !busy, tokens)
                    .when(!busy, |button| {
                        button.on_click(cx.listener(move |shell, _event, window, cx| {
                            cx.stop_propagation();
                            shell.ask_card(delete.clone(), window, cx);
                        }))
                    });
                let restore_words = if again {
                    words.restore_session
                } else {
                    words.unarchive_hint
                };
                vec![
                    tip(restore_button, "restore", restore_words).into_any_element(),
                    tip(delete_button, "delete", words.delete_permanently).into_any_element(),
                ]
            }
        };
    Some(
        div()
            .flex()
            .flex_row()
            .flex_none()
            .gap(px(2.0))
            .my(px(-5.0))
            .mr(px(-4.0))
            .when(!place.narrow, |slot| {
                slot.invisible().group_hover(GROUP, Styled::visible)
            })
            .children(buttons),
    )
}

/// A 24 square glyph button with the hover's wash; `danger`: its glyph
/// turns that colour on hover (the page's kill).
fn icon_button(
    id: ElementId,
    icon: Glyph,
    danger: Option<Hsla>,
    enabled: bool,
    tokens: &Tokens,
) -> Stateful<Div> {
    let neutral = tokens.neutral;
    let group = format!("{id}");
    let mark = glyph(icon, 14.0, neutral.foreground_muted);
    let mark = match danger {
        Some(color) if enabled => {
            mark.group_hover(group.clone(), move |style| style.text_color(color))
        }
        Some(_) | None => mark,
    };
    div()
        .id(id)
        .group(group)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(24.0))
        .rounded(px(popup::ROW_RADIUS))
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .when(!enabled, |button| button.opacity(0.5))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(neutral.surface2))
        })
        .child(mark)
}
