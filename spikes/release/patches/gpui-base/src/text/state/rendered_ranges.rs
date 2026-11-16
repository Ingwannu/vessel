//! Opt-in HTML range lifecycle and opaque acceptance receipts for owner caches.
use super::*;
use crate::text::RenderedRangeSupport;

/// Evidence that this exact retained owner accepted a complete highlight set.
///
/// It is invalidated on clear and HTML replacement/commit, including accepted
/// empty sets. Comparing a request key alone cannot detect those transitions.
#[derive(Clone, Debug)]
pub struct RangeHighlightStamp {
    owner: EntityId,
    revision: usize,
    accepted: Arc<()>,
}

impl PartialEq for RangeHighlightStamp {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.revision == other.revision
            && Arc::ptr_eq(&self.accepted, &other.accepted)
    }
}
impl Eq for RangeHighlightStamp {}

impl TextViewState {
    /// Enable rendered-only HTML ranges on the same retained view and parser.
    /// Defaults to `MarkdownOnly`; no Markdown source positions are invented.
    pub fn rendered_range_support(mut self, support: RenderedRangeSupport) -> Self {
        self.rendered_range_support = support;
        self.reset_html_ranges();
        self.rendered_index = Arc::default();
        self.index_html_ranges(&self.parsed_content.document);
        self
    }

    /// The current acceptance receipt, absent after invalidation or clear.
    pub fn range_highlight_stamp(&self) -> Option<RangeHighlightStamp> {
        Some(RangeHighlightStamp {
            owner: self.entity_id,
            revision: self.committed_revision,
            accepted: self.range_accepted.clone()?,
        })
    }

    pub(super) fn ranges_supported(&self) -> bool {
        self.format == TextViewFormat::Markdown
            || (self.rendered_range_support == RenderedRangeSupport::HtmlAndMarkdown
                && self.committed_revision >= self.full_update_revision)
    }

    pub(super) fn reset_html_ranges(&mut self) {
        if self.format == TextViewFormat::Html {
            self.range_highlights = None;
            self.range_accepted = None;
            self.pending_reveal = None;
        }
    }

    pub(super) fn index_html_ranges(&self, document: &ParsedDocument) {
        if self.format == TextViewFormat::Html
            && self.rendered_range_support == RenderedRangeSupport::HtmlAndMarkdown
        {
            // This is reached only from accepted UI construction/commit. Eager
            // indexing makes every snapshot retain its own immutable keys even
            // when an append shares old leaf states with the next parsed AST.
            self.rendered_index.get_or_init(|| {
                RenderedIndex::html(document, self.entity_id, self.committed_revision)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn html(text: &str, cx: &mut TestAppContext) -> gpui::Entity<TextViewState> {
        cx.update(crate::init);
        let state = cx.update(|cx| {
            cx.new(|cx| {
                TextViewState::html(text, cx)
                    .rendered_range_support(RenderedRangeSupport::HtmlAndMarkdown)
            })
        });
        cx.run_until_parked();
        state
    }

    fn highlight(range: Range<usize>) -> RangeHighlight {
        RangeHighlight::new(range, gpui::hsla(0.1, 1., 0.6, 1.))
    }

    #[gpui::test]
    fn html_invalid_sets_preserve_prior_acceptance_and_empty_sets_clear(cx: &mut TestAppContext) {
        let state = html("<pre>한글🙂 &amp; literal</pre>", cx);
        state.update(cx, |state, cx| {
            state.set_range_highlights([highlight(0..3)], cx).unwrap();
            let accepted = state.range_highlight_stamp();
            assert!(accepted.is_some());
            for ranges in [
                vec![highlight(3..6), highlight(1..3)],
                vec![highlight(3..6), highlight(0..usize::MAX)],
                vec![highlight(3..6), highlight(Range { start: 4, end: 2 })],
            ] {
                assert_eq!(
                    state.set_range_highlights(ranges, cx),
                    Err(RangeHighlightError::InvalidRange(1))
                );
                assert_eq!(state.range_highlight_stamp(), accepted);
                assert!(state.range_highlights.is_some());
            }
            state.set_range_highlights([], cx).unwrap();
            assert!(state.range_highlights.is_none());
            assert!(state.range_highlight_stamp().is_some());
            assert_ne!(state.range_highlight_stamp(), accepted);
            state.clear_range_highlights(cx);
            assert!(state.range_highlight_stamp().is_none());
            assert_eq!(state.rendered_text().range_for_source(0..3), None);
        });
    }

    #[gpui::test]
    fn html_a_b_a_drops_old_ranges_and_reveal_even_when_text_repeats(cx: &mut TestAppContext) {
        let state = html("<p>alpha</p>", cx);
        state.update(cx, |state, cx| {
            let first = state.rendered_text();
            state.set_range_highlights([highlight(0..5)], cx).unwrap();
            state.reveal_range(0..5, cx).unwrap();
            let stamp = state.range_highlight_stamp();
            state.set_text("<p>bravo</p>", cx);
            assert!(state.pending_reveal.is_none());
            assert!(state.range_highlights.is_none());
            state.set_text("<p>alpha</p>", cx);
            assert_eq!(first.as_str(), state.rendered_text().as_str());
            assert_ne!(first, state.rendered_text());
            assert!(state.range_highlight_stamp().is_none());
            state.set_range_highlights([highlight(0..5)], cx).unwrap();
            assert_ne!(state.range_highlight_stamp(), stamp);
        });
        cx.run_until_parked();
        state.read_with(cx, |state, _| {
            assert!(state.pending_reveal.is_none());
            assert_eq!(state.rendered_text().as_str(), "alpha\n");
        });
    }

    #[gpui::test]
    fn pending_html_replace_refuses_old_ranges_and_stale_parse_cannot_restore_them(
        cx: &mut TestAppContext,
    ) {
        let state = html("<p>before</p>", cx);
        state.update(cx, |state, cx| {
            state.set_range_highlights([highlight(0..6)], cx).unwrap();
            state.reveal_range(0..6, cx).unwrap();
            let old_revision = state.committed_revision;
            let old_content = state.parsed_content.clone();
            let large = format!("<pre>{}</pre>", "new ".repeat(MAX_SYNC_FULL_REPLACE_BYTES));
            state.set_text(&large, cx);
            assert!(state.range_highlight_stamp().is_none());
            assert!(state.pending_reveal.is_none());
            assert_eq!(
                state.set_range_highlights([highlight(0..3)], cx),
                Err(RangeHighlightError::Unsupported)
            );
            state.set_text("<p>before</p>", cx);
            state.commit_parsed_update(
                ParsedUpdate {
                    revision: old_revision,
                    full_parse: true,
                    selection_compatible: false,
                    baseline_ack: false,
                    result: Ok(old_content),
                },
                cx,
            );
            assert!(state.range_highlight_stamp().is_none());
            assert!(state.pending_reveal.is_none());
            assert_eq!(state.rendered_text().as_str(), "before\n");
        });
        cx.run_until_parked();
        state.read_with(cx, |state, _| {
            assert_eq!(state.rendered_text().as_str(), "before\n");
            assert!(state.pending_reveal.is_none());
            assert!(state.range_highlight_stamp().is_none());
        });
    }
}
