//! A session card's pull request line, under its branch (track S's card
//! mounts it with one line), as the card's other lines: Paseo's sidebar
//! meta row
//! (`components/sidebar/workspace-meta-row/index.tsx`): the state's glyph
//! and the number (the state's word after it unless open), then the
//! checks' glyph and one word in the checks' colour.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{AnyElement, IntoElement as _, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::board::Card;
use vessel_app_view::localization::Translator;
use vessel_app_view::workspace::pull::{CardChecks, CardPull};

/// Paseo's `META_ICON_SIZE` (the host badge's glyph).
const META_GLYPH: f32 = 12.0;

/// The items, when the card's session has a pull request read.
pub(crate) fn card_items(card: &Card, words: &Translator, tokens: &Tokens) -> Option<AnyElement> {
    let pull = CardPull::of(card.detail.pull_request.as_deref()?)?;
    let status = tokens.status();
    let mut number = format!("#{}", pull.number.get());
    if pull.says_state {
        number.push(' ');
        number.push_str(pull.phase.words(words));
    }
    let checks = pull.checks.map(|checks| {
        let (which, colour) = match checks {
            CardChecks::Passed => (Glyph::CircleCheck, status.success.foreground),
            CardChecks::Failed => (Glyph::CircleX, status.error.foreground),
            CardChecks::Running => (Glyph::CircleDot, tokens.neutral.foreground_muted),
        };
        div()
            .flex()
            .items_center()
            .gap(px(spacing::S1))
            .text_color(colour)
            .child(glyph(which, META_GLYPH, colour).flex_none())
            .child(checks.words(words).to_owned())
    });
    Some(
        // The card's own line (`board/card.rs` `lines`): 6 above, the
        // tiny face, never wrapped.
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S2))
            .mt(px(spacing::S1_5))
            .whitespace_nowrap()
            .type_(Type::Tiny)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::S1))
                    .text_color(tokens.neutral.foreground_muted)
                    .child(super::link::state_glyph_sized(
                        pull.phase, META_GLYPH, tokens,
                    ))
                    .child(number),
            )
            .children(checks)
            .into_any_element(),
    )
}
