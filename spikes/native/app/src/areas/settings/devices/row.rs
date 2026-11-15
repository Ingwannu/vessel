//! One device's production row composition, shared with its bounds test.
//!
//! [Decision Log]
//! - Purpose: the revoke action keeps the same trailing edge with or
//!   without a badge, even when the kit wraps its control below the name.
//! - Constraint: the settings row and button remain the shared kit's.
//! - Alternatives: a full-width control would move every action below
//!   its name; changing the kit would alter other areas' rows.
//! - Choice: one natural-width badge/action group with automatic leading
//!   margin and no shrinking. It stays right-aligned on either flex line.
//! - Why: retain the ordinary row's geometry and the distinguishing badge.
//! - Cost: a wrapped group uses another line; names still wrap freely.
use vessel_app_kit::ui::{
    Div, FluentBuilder as _, IntoElement as _, ParentElement as _, Stateful, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, breakpoint::TOUCH_TARGET, settings, theme::spacing};

#[cfg(test)]
mod tests;

pub(super) fn compose(
    title: impl Into<gpui::SharedString>,
    hint: impl Into<gpui::SharedString>,
    badge: Option<Div>,
    action: Stateful<Div>,
    first: bool,
    compact: bool,
    tokens: &Tokens,
) -> Div {
    let control = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .ml_auto()
        .gap(px(spacing::S2))
        .children(badge)
        .child(action.when(compact, |action| action.min_h(px(TOUCH_TARGET))));
    settings::row_at(
        title,
        Some(hint.into()),
        None,
        Some(control.into_any_element()),
        first,
        compact,
        tokens,
    )
}
