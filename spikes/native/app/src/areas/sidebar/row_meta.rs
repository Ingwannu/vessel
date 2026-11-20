//! The line under a session's title, as Paseo's `WorkspaceMetaRow`
//! (`components/sidebar/workspace-meta-row/index.tsx`): what else is known
//! about the session, as peers separated by a dot, in Paseo's order and
//! with its default items: the machine, the change request, its checks, the
//! labels. Absent when there is nothing to say, so the row keeps one line.
//!
//! [Decision Log] (TRACKER 663)
//! - Before: the labels alone on a second line; the machine at the title's
//!   end in the tiny face (grouped by status); the pull request only on the
//!   board's card.
//! - Choice: Paseo's line: 12 on 16, 6 between items, a muted dot between
//!   them; the machine as Paseo's host badge (the `Server` glyph and its
//!   name, muted; shown where the row's group does not already name the
//!   machine, so grouped by status); the change request's state glyph and
//!   its number (its state's word after it unless open); the checks' glyph
//!   and one word in the checks' colour (passed, failed, running in the
//!   warning colour); the labels as one run of chips, pulled left by the
//!   chip's inset when they lead the line so the first letter sits on the
//!   title's rail.
//! - The checks' glyph (TRACKER 426) is Paseo's `CheckIndicator`: one
//!   ring whose inside changes, a check, a cross, or a pie filling by
//!   quarters from the node's tally while the run goes on
//!   (`CardPull::quarters`); the item is named "Checks passed", "Checks
//!   failed" or "Checks running" (Paseo's `*Accessible` words on its
//!   wrapper), so the word could go without the state going.
//! - Differences, named: Paseo's change request is a link to its page (here
//!   it says, the board's card and the Changes tab open it); Paseo's
//!   running services item has no Vessel source; its branch and project
//!   items are off by default and not drawn.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Div, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::Card;
use vessel_app_view::workspace::pull::{CardChecks, CardPull};

use crate::shell::Shell;

/// One size for every glyph on the line (Paseo's `META_ICON_SIZE`, the
/// host badge's).
const GLYPH: f32 = 12.0;
/// Between a glyph and its words.
const GLYPH_GAP: f32 = 3.0;
/// The labels' chip inset (`WORKSPACE_LABEL_CHIP_INSET`), taken back when
/// they lead the line. A negative literal: sizes take no arithmetic.
const CHIP_BACK: f32 = -6.0;
/// The host badge gives way before any neighbour (Paseo's `flexShrink: 100`).
const HOST_SHRINK: f32 = 100.0;

/// What the line carries, in Paseo's order.
enum Item {
    Host(String),
    Pull(CardPull),
    /// The checks, their pie's quarters and the item's id.
    Checks(CardChecks, u8, String),
    Labels(Vec<vessel_app_view::labels::Label>),
}

/// The line, when there is something to say.
pub(super) fn meta_line(
    card: &Card,
    machine: Option<&str>,
    shell: &Shell,
    tokens: &Tokens,
) -> Option<Div> {
    let mut items = Vec::new();
    if let Some(machine) = machine {
        items.push(Item::Host(machine.to_owned()));
    }
    let pull = card.detail.pull_request.as_deref().and_then(CardPull::of);
    if let Some(pull) = pull {
        items.push(Item::Pull(pull));
        if let Some(checks) = pull.checks {
            let id = format!("row-checks-{}-{}", card.machine, card.session);
            items.push(Item::Checks(checks, pull.quarters, id));
        }
    }
    let labels = shell
        .areas
        .labels
        .row(&card.machine, card.session, card.member_project())
        .map(|row| row.labels)
        .unwrap_or_default();
    if !labels.is_empty() {
        items.push(Item::Labels(labels));
    }
    if items.is_empty() {
        return None;
    }
    let mut line = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1_5))
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted);
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            line = line.child(
                div()
                    .flex_none()
                    .text_color(tokens.neutral.foreground_extra_muted)
                    .child("·"),
            );
        }
        line = line.child(draw(item, index == 0, shell, tokens));
    }
    Some(line)
}

fn draw(item: Item, leading: bool, shell: &Shell, tokens: &Tokens) -> AnyElement {
    let muted = tokens.neutral.foreground_muted;
    match item {
        Item::Host(name) => with_glyph(glyph(Glyph::Server, GLYPH, muted).into_any_element(), name)
            .flex_shrink(HOST_SHRINK)
            .into_any_element(),
        Item::Pull(pull) => {
            let mut words = pull.number.get().to_string();
            if pull.says_state {
                words.push(' ');
                words.push_str(pull.phase.words(shell.words));
            }
            with_glyph(
                crate::areas::pulls::state_glyph_sized(pull.phase, GLYPH, tokens),
                words,
            )
            .flex_none()
            .into_any_element()
        }
        Item::Checks(checks, quarters, id) => {
            let status = tokens.status();
            let colour = match checks {
                CardChecks::Passed => status.success.foreground,
                CardChecks::Failed => status.error.foreground,
                CardChecks::Running => status.warning.foreground,
            };
            with_glyph(
                glyph(indicator(checks, quarters), GLYPH, colour).into_any_element(),
                checks.words(shell.words).to_owned(),
            )
            .id(ElementId::Name(id.into()))
            .role(gpui::Role::Image)
            .aria_label(shell.t.lang.sidebar().checks_outcome(checks))
            .flex_none()
            .text_color(colour)
            .into_any_element()
        }
        Item::Labels(labels) => div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S1))
            .min_w_0()
            .flex_shrink(1.0)
            .when(leading, |labels| labels.ml(px(CHIP_BACK)))
            .children(
                labels
                    .iter()
                    .map(|label| crate::areas::labels::chip(label, tokens)),
            )
            .into_any_element(),
    }
}

/// Paseo's `CheckIndicator` for `checks`: the ring with a check, a cross,
/// or a pie filled `quarters` of the way (more than four reads as full).
pub(super) const fn indicator(checks: CardChecks, quarters: u8) -> Glyph {
    match (checks, quarters) {
        (CardChecks::Passed, _) => Glyph::ChecksPassed,
        (CardChecks::Failed, _) => Glyph::ChecksFailed,
        (CardChecks::Running, 0) => Glyph::ChecksPie0,
        (CardChecks::Running, 1) => Glyph::ChecksPie1,
        (CardChecks::Running, 2) => Glyph::ChecksPie2,
        (CardChecks::Running, 3) => Glyph::ChecksPie3,
        (CardChecks::Running, _) => Glyph::ChecksPie4,
    }
}

/// A glyph and its words, 3 apart; the words cut when the line runs out.
fn with_glyph(mark: AnyElement, words: String) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(GLYPH_GAP))
        .min_w_0()
        .child(mark)
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .child(words),
        )
}
