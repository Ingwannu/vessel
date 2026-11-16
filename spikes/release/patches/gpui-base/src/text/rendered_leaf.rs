//! Parse-scoped identity for spanless HTML leaves in the existing rendered index.
//!
//! Source offsets remain a separate key domain. The index assigns these keys to
//! the same retained inline states paint and selection already use, on the UI
//! commit path only; the background parser never mutates displayed leaf state.
use gpui::{ElementId, EntityId};

/// Opt-in admission of rendered byte ranges for HTML; Markdown is unchanged.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RenderedRangeSupport {
    /// Preserve the existing HTML Unsupported result.
    #[default]
    MarkdownOnly,
    /// Address the rendered text of the current committed HTML parse.
    HtmlAndMarkdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct RenderedScope {
    owner: EntityId,
    revision: usize,
}

impl RenderedScope {
    pub(super) fn new(owner: EntityId, revision: usize) -> Self {
        Self { owner, revision }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct RenderedLeafKey {
    scope: RenderedScope,
    block: usize,
    ordinal: usize,
}

impl RenderedLeafKey {
    pub(super) fn new(scope: RenderedScope, block: usize, ordinal: usize) -> Self {
        Self {
            scope,
            block,
            ordinal,
        }
    }

    pub(super) fn block(self) -> usize {
        self.block
    }

    pub(super) fn element_id(self) -> ElementId {
        // The fixed opaque id cannot hold owner + revision + ordinal without
        // truncation. Keep all components, paying one small name allocation for
        // an opted-in HTML leaf instead of weakening identity across reparses.
        ElementId::Name(
            format!(
                "rendered-leaf:{}:{}:{}",
                self.scope.owner, self.scope.revision, self.ordinal
            )
            .into(),
        )
    }
}
