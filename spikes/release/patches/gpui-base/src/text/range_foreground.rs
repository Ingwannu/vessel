//! Color-only range refinement over the existing authored inline styles.
//!
//! Resolve overlap in input order before streaming fade. This preserves code
//! typography and link decorations and changes the same retained run cache that
//! normal authored ink uses; no separate paint or selection layer is introduced.
use super::{
    inline::{InlineHighlight, combine_highlights, fade_highlights},
    node::{NodeContext, slice_fades},
    stream_fade::TextLeafKey,
};
use gpui::Hsla;
use std::ops::Range;

fn refine(
    highlights: Vec<(Range<usize>, InlineHighlight)>,
    colors: &[(Range<usize>, Hsla)],
) -> Vec<(Range<usize>, InlineHighlight)> {
    if colors.is_empty() {
        return highlights;
    }
    let boundaries = colors
        .iter()
        .map(|(range, _)| (range.clone(), InlineHighlight::default()));
    let mut combined = combine_highlights(highlights, boundaries);
    for (range, highlight) in &mut combined {
        if let Some((_, color)) = colors.iter().rev().find(|(color_range, _)| {
            color_range.start <= range.start && color_range.end >= range.end
        }) {
            highlight.style.color = Some(*color);
        }
    }
    combined
}

impl NodeContext {
    /// Apply owner ink after authored styles, then the existing streamed fade.
    pub(super) fn text_highlights(
        &self,
        highlights: Vec<(Range<usize>, InlineHighlight)>,
        key: Option<TextLeafKey>,
        start: usize,
        end: usize,
    ) -> Vec<(Range<usize>, InlineHighlight)> {
        let colors = match (&self.range_highlights, key) {
            (Some(frame), Some(key)) => frame.foregrounds(key),
            _ => &[],
        };
        let colors = colors
            .iter()
            .filter_map(|(range, color)| {
                let from = range.start.max(start);
                let to = range.end.min(end);
                (from < to).then(|| (from - start..to - start, *color))
            })
            .collect::<Vec<_>>();
        fade_highlights(
            refine(highlights, &colors),
            &slice_fades(self.stream_fades(key), start, end),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::HighlightStyle;
    #[test]
    fn last_supplied_ink_wins_even_when_an_earlier_color_starts_later() {
        let first = gpui::hsla(0., 1., 0.4, 1.);
        let last = gpui::hsla(0.5, 1., 0.4, 1.);
        let authored = InlineHighlight::from(HighlightStyle {
            font_weight: Some(gpui::FontWeight::BOLD),
            ..Default::default()
        });
        let runs = refine(vec![(0..8, authored)], &[(3..7, first), (0..5, last)]);
        for (range, highlight) in runs {
            assert_eq!(highlight.style.font_weight, Some(gpui::FontWeight::BOLD));
            assert_eq!(
                highlight.style.color,
                if range.start < 5 {
                    Some(last)
                } else if range.start < 7 {
                    Some(first)
                } else {
                    None
                }
            );
        }
    }
}
