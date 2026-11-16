//! Cache actual range acceptance for the exact retained text owner and palette.
use super::{FindMarks, FindPalette};
use gpui_base::text::RangeHighlightStamp;
use gpui_component::text::TextViewState;

/// A successful complete set, rather than an external request key alone.
#[derive(Clone)]
pub(crate) struct AppliedMark {
    key: u64,
    current: Option<usize>,
    palette: FindPalette,
    stamp: RangeHighlightStamp,
}

impl AppliedMark {
    pub(super) fn accepted(
        state: &TextViewState,
        marks: &FindMarks<'_>,
        palette: FindPalette,
    ) -> Option<Self> {
        Some(Self {
            key: marks.key,
            current: marks.current,
            palette,
            stamp: state.range_highlight_stamp()?,
        })
    }

    /// Reparse/clear, owner replacement and palette changes all require retry.
    pub(crate) fn is_current(
        &self,
        state: &TextViewState,
        marks: &FindMarks<'_>,
        palette: FindPalette,
    ) -> bool {
        self.key == marks.key
            && self.current == marks.current
            && self.palette == palette
            && state.range_highlight_stamp().as_ref() == Some(&self.stamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext, px};
    use gpui_base::text::RenderedRangeSupport;

    #[gpui::test]
    fn refused_marks_retry_and_equal_keys_do_not_survive_reparse_or_palette_changes(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let state = cx.update(|cx| {
            cx.new(|cx| {
                TextViewState::html("<p>한글</p>", cx)
                    .rendered_range_support(RenderedRangeSupport::HtmlAndMarkdown)
            })
        });
        cx.run_until_parked();
        let list = crate::VirtualList::new(false);
        let palette = FindPalette {
            found: gpui::hsla(0.1, 1., 0.8, 1.),
            current: gpui::hsla(0.1, 1., 0.5, 1.),
            current_ink: gpui::hsla(0., 0., 0.1, 1.),
        };
        let invalid = |_: &str| vec![1..3];
        let valid = |_: &str| vec![0..3];
        let mut marks = FindMarks {
            list: &list,
            clear_top: px(0.),
            key: 7,
            spans: &invalid,
            current: Some(0),
        };
        state.update(cx, |state, cx| {
            assert!(super::super::mark(state, &marks, palette, cx).is_none());
            assert!(state.range_highlight_stamp().is_none());
            marks.spans = &valid;
            let applied = super::super::mark(state, &marks, palette, cx).unwrap();
            assert!(applied.is_current(state, &marks, palette));
            let changed = FindPalette {
                current_ink: gpui::hsla(0., 0., 0.9, 1.),
                ..palette
            };
            assert!(!applied.is_current(state, &marks, changed));
            state.set_text("<p>다른</p>", cx);
            assert!(!applied.is_current(state, &marks, palette));
            assert!(super::super::mark(state, &marks, palette, cx).is_some());
        });
    }
}
