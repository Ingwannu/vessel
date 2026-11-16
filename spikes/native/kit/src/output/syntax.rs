//! Resolve semantic ink at draw time, then let GPUI merge disjoint attributes.
use crate::code_editor::{CodeStyles, SyntaxKind};
use gpui::{HighlightStyle, Hsla};
use std::ops::Range;

pub(super) fn highlights(
    emphasis: &[Range<usize>],
    syntax: &[(Range<usize>, Option<SyntaxKind>)],
    background: Option<Hsla>,
    styles: &CodeStyles,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let ink = syntax
        .iter()
        .filter_map(|(range, kind)| kind.map(|kind| (range.clone(), styles.highlight(kind))));
    let wash = background.into_iter().flat_map(|background| {
        emphasis.iter().cloned().map(move |range| {
            (
                range,
                HighlightStyle {
                    background_color: Some(background),
                    ..HighlightStyle::default()
                },
            )
        })
    });
    // Foreground and background are independent attributes, so their merge order
    // cannot choose one color over another. The engine emits disjoint spans.
    gpui::combine_highlights(ink, wash).collect()
}

#[cfg(test)]
mod tests;
