//! `SelectField` MD chrome through the one select frame/interaction renderer.
//!
//! [Decision Log]
//! - Purpose: form choices are rectangular fields, not Settings/HostFilter pills.
//! - Existing seam: select trigger owns frame, anchor, label and chevron.
//! - Choice: typed value/activity content adds placeholder/loading presentation;
//!   old signatures/literals/callers stay unchanged and use their old defaults.
//! - Source: pinned1c00 ui/select-field.tsx136–193/344–394 and control-geometry
//!   235–250/286–306. MD32/44, px12/r8, leading caller18, loading14/slot16.
//! - Why/cost: no area chrome or parallel select; caller still owns disabled
//!   handler/target guards. Border-only focus follows Vessel's existing field
//!   policy; this adds no reference outline ring on the containing surface.
use super::{Anchor, Presentation, TriggerContent, Visibility};
use crate::{ButtonState, ControlSizing, theme::Tokens};
use gpui::{AnyElement, Div, ElementId, SharedString, Stateful};

/// Selected values and missing choices have different source text colours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// An already localized selected value.
    Selected(SharedString),
    /// An already localized missing-selection instruction.
    Placeholder(SharedString),
}
/// Fetching choices may keep cached rows selectable; it is not Disabled.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// Source chevron with no spinner slot.
    #[default]
    Idle,
    /// Numeric14 `ActivityIndicator` in a separate16-point slot.
    Loading,
}
/// Source MD form field; no unrelated presentation/size is accepted and ignored.
pub struct Content {
    /// Selected text or missing-selection words, prepared outside paint.
    pub value: Value,
    /// Shared glyph/mark in the caller's source18-point option slot.
    pub leading: Option<AnyElement>,
    /// MD pointer32 or compact44.
    pub sizing: ControlSizing,
    /// Disabled is independent of loading; callers attach no disabled handler.
    pub availability: ButtonState,
    /// Open field gets the same accent border as hover/press.
    pub visibility: Visibility,
    /// Keep existing values while a fresh list is being read.
    pub activity: Activity,
}
impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormContent")
            .field("value", &self.value)
            .field("availability", &self.availability)
            .field("activity", &self.activity)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Look {
    pub(super) placeholder: bool,
    pub(super) activity: Activity,
}
/// Same select renderer, source field geometry and typed placeholder/loading.
#[must_use]
pub fn trigger(
    id: impl Into<ElementId>,
    content: Content,
    anchor: &Anchor,
    tokens: &Tokens,
) -> Stateful<Div> {
    let (label, placeholder) = match content.value {
        Value::Selected(label) => (label, false),
        Value::Placeholder(label) => (label, true),
    };
    super::render(
        id.into(),
        TriggerContent {
            label,
            leading: content.leading,
            presentation: Presentation::Form,
            sizing: content.sizing,
            availability: content.availability,
            visibility: content.visibility,
        },
        Some(Look {
            placeholder,
            activity: content.activity,
        }),
        anchor,
        tokens,
    )
}
