// MODIFIED by Vessel: opt-in inferred links retain the original text and source map.
//! Host recognition decorates existing runs; it never creates an inline atom.

use std::ops::Range;

use gpui::SharedString;

use super::node::{LinkMark, TextMark};

/// Fenced code and explicit links are never offered to this recognizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineLinkKind {
    /// Ordinary prose within a paragraph.
    Text,
    /// A complete inline-code span, excluding fenced blocks.
    Code,
}

/// Authored hrefs may be percent encoded; inferred text must retain literal bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LinkSource {
    #[default]
    /// A Markdown/HTML destination explicitly authored in the document.
    Authored,
    /// A host-recognized visible token whose original bytes are preserved.
    Inferred(InlineLinkKind),
}

/// A link activation preserves provenance without exposing renderer internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkTarget {
    pub(crate) href: SharedString,
    pub(crate) source: LinkSource,
}

impl LinkTarget {
    /// Untrusted destination text supplied by the document or recognizer.
    #[must_use]
    pub fn href(&self) -> &SharedString {
        &self.href
    }
    /// Whether decoding as an authored URI would change a literal filename.
    #[must_use]
    pub fn source(&self) -> LinkSource {
        self.source
    }
}

/// A nonempty UTF-8 byte range and untrusted destination, validated at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognizedLink {
    range: Range<usize>,
    href: SharedString,
}

impl RecognizedLink {
    /// Reject empty ranges, split UTF-8 characters and control-bearing destinations.
    #[must_use]
    pub fn new(text: &str, range: Range<usize>, href: impl Into<SharedString>) -> Option<Self> {
        let href = href.into();
        if range.is_empty()
            || text.get(range.clone()).is_none()
            || href.is_empty()
            || href.chars().any(char::is_control)
        {
            return None;
        }
        Some(Self { range, href })
    }
}

type Recognize = fn(&str, InlineLinkKind) -> Vec<RecognizedLink>;

/// A pure function pointer keeps equivalent per-frame registrations parse-stable.
#[derive(Clone, Copy)]
pub struct InlineLinkRecognizer(Recognize);

impl std::fmt::Debug for InlineLinkRecognizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InlineLinkRecognizer")
            .finish_non_exhaustive()
    }
}
impl PartialEq for InlineLinkRecognizer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::fn_addr_eq(self.0, other.0)
    }
}
impl Eq for InlineLinkRecognizer {}

/// Bound hostile or mistaken host recognition without changing the underlying run.
const MAX_INLINE_LINKS_PER_RUN: usize = 256;

impl InlineLinkRecognizer {
    /// Register a pure recognition function, reused across equivalent render frames.
    #[must_use]
    pub fn new(recognize: Recognize) -> Self {
        Self(recognize)
    }

    pub(crate) fn marks(self, text: &str, kind: InlineLinkKind) -> Vec<(Range<usize>, TextMark)> {
        let mut links = (self.0)(text, kind);
        links.sort_by(|a, b| {
            a.range
                .start
                .cmp(&b.range.start)
                .then(a.range.end.cmp(&b.range.end))
        });
        let mut end = 0;
        links
            .into_iter()
            .filter_map(|link| {
                if link.range.start < end
                    || link.range.is_empty()
                    || text.get(link.range.clone()).is_none()
                {
                    return None;
                }
                end = link.range.end;
                Some((
                    link.range,
                    TextMark::default().link(LinkMark {
                        url: link.href,
                        source: LinkSource::Inferred(kind),
                        ..Default::default()
                    }),
                ))
            })
            .take(MAX_INLINE_LINKS_PER_RUN)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum InlineLinkScope {
    #[default]
    Recognize,
    ExplicitLink,
}

/// Inferred links can overlap code/emphasis. Reuse the renderer's existing sweep
/// only for opt-in recognized marks; its copy walk must see each byte once.
pub(crate) fn source_marks(marks: Vec<(Range<usize>, TextMark)>) -> Vec<(Range<usize>, TextMark)> {
    if !marks.iter().any(|(_, mark)| {
        mark.link
            .as_ref()
            .is_some_and(|link| matches!(link.source, LinkSource::Inferred(_)))
    }) {
        return marks;
    }
    super::inline::combine_ranges(marks, [], |mut combined: TextMark, layer| {
        combined.merge(layer.clone());
        combined
    })
}
