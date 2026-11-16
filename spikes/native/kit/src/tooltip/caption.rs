//! Closed caption/chord content for the existing tooltip, not arbitrary UI.
//! Display labels come from the owner's one effective shortcut projection.
use super::{Configuration, Content, Place, Tooltips};
use crate::{ControlSizing, Tokens, command, theme::spacing};
use gpui::prelude::*;
use gpui::{Div, SharedString, Stateful, div, px};
use std::sync::Arc;

/// Already-formatted display chords; this is not a binding or action registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayChords(Arc<[SharedString]>);
impl DisplayChords {
    /// Preserve caller order/text; empty labels cannot create empty badges.
    #[must_use]
    pub fn new(chords: impl IntoIterator<Item = SharedString>) -> Self {
        Self(
            chords
                .into_iter()
                .filter(|chord| !chord.is_empty())
                .collect::<Vec<_>>()
                .into(),
        )
    }
    /// Empty also represents unavailable or unassigned shortcuts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// The caller-prepared whole label for each chord, in sequence order.
    #[must_use]
    pub fn as_slice(&self) -> &[SharedString] {
        &self.0
    }
}

/// Whole tooltip caption plus optional display chords, with no action callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCaption {
    pub(super) label: SharedString,
    keys: DisplayChords,
}
impl KeyCaption {
    /// The owner supplies localization and effective keys before drawing.
    #[must_use]
    pub fn new(label: impl Into<SharedString>, keys: DisplayChords) -> Self {
        Self {
            label: label.into(),
            keys,
        }
    }
}
impl Tooltips {
    /// Same controller, configured timing and touch suppression; only its
    /// closed body content changes. Empty keys reserve no child or gap.
    #[must_use]
    pub fn on_caption(
        &self,
        trigger: Stateful<Div>,
        place: &Place,
        caption: KeyCaption,
        configuration: Configuration,
        sizing: ControlSizing,
    ) -> Stateful<Div> {
        self.attach(
            trigger,
            place,
            Content::Caption(caption),
            configuration,
            sizing,
        )
    }
}
impl Content {
    pub(super) fn decorate(&self, card: Stateful<Div>, tokens: &Tokens) -> Stateful<Div> {
        match self {
            Self::Text(_) | Self::Code(_) => {
                card.children(super::lines(self.text(), tokens.neutral.foreground))
            }
            Self::Caption(caption) => card.child(body(caption, tokens)),
        }
    }
}
fn body(caption: &KeyCaption, tokens: &Tokens) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .min_w_0()
        .gap(px(spacing::S2))
        .child(
            div()
                .flex_shrink(1.0)
                .min_w_0()
                .child(caption.label.clone()),
        )
        .children((!caption.keys.is_empty()).then(|| chord_badges(&caption.keys, tokens)))
}
fn chord_badges(keys: &DisplayChords, tokens: &Tokens) -> Div {
    // Source Shortcut's sequence wraps inside the bounded tooltip. Reuse its
    // existing badge renderer; ordinary command rows keep their old defaults.
    command::key_badges(keys.as_slice(), tokens)
        .flex_shrink(1.0)
        .min_w_0()
        .flex_wrap()
}

#[cfg(test)]
mod tests;
