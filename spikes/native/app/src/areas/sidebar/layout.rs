//! One bounded sidebar body; row drawing stays with its existing owner.
//! Only the last listed item has a focus/reveal target in this repair.
//! Activation and complete row keyboard accessibility remain separate work.

use crate::shell::Shell;
use gpui::FocusHandle;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, IntoElement, ParentElement as _, Styled as _, div, px,
};
use vessel_app_kit::{ScrollState, ScrollViewport};

/// Keeps one body position and the limited focus/reveal checkpoints.
pub(super) struct Layout {
    /// Measurements and offset of the sole scrolling body.
    pub(super) scroll: ScrollState,
    /// The existing new-conversation navigation row.
    pub(super) navigation: FocusHandle,
    end: FocusHandle,
    /// The existing Settings button outside the body.
    pub(super) footer: FocusHandle,
}
impl Layout {
    /// Keep handles across renders instead of recreating focus during layout.
    pub(super) fn new(cx: &mut Context<'_, Shell>) -> Self {
        Self {
            scroll: ScrollState::new(),
            navigation: cx.focus_handle().tab_stop(true),
            end: cx.focus_handle().tab_stop(true),
            footer: cx.focus_handle().tab_stop(true),
        }
    }
}

/// These three measured sections keep their natural heights, without a
/// second scroller or a project/session height budget.
pub(super) fn body(
    state: &Layout,
    projects: Option<AnyElement>,
    heading: Div,
    groups: Div,
) -> impl IntoElement {
    let projects = match projects {
        Some(projects) => div().pb(px(spacing::S3)).child(projects),
        None => div(),
    };
    // Compact header rows own their own inset; keep the legacy body inset
    // here so the one shared scroller retains its original measured width.
    div().flex_1().min_h_0().px(px(spacing::S2)).child(
        ScrollViewport::new("sidebar-body", &state.scroll)
            .section(projects)
            .section(div().pb(px(spacing::S3)).child(heading))
            .section(groups)
            .element(),
    )
}

/// Which existing row ends the list; no invisible focus sentinel is added.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EndTarget {
    Earlier,
    Last,
}
impl EndTarget {
    /// Select the existing final row without adding a focus-only row.
    pub(super) fn at(index: usize, last: Option<usize>) -> Self {
        if Some(index) == last {
            Self::Last
        } else {
            Self::Earlier
        }
    }
    /// Both the final group and its final row must end the full list.
    pub(super) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Last, Self::Last) => Self::Last,
            (Self::Earlier, Self::Earlier | Self::Last) | (Self::Last, Self::Earlier) => {
                Self::Earlier
            }
        }
    }
    /// Reuse row content unchanged while sharing the kit reveal behavior.
    pub(super) fn wrap(self, row: impl IntoElement, state: &Layout) -> AnyElement {
        match self {
            Self::Earlier => row.into_any_element(),
            Self::Last => state
                .scroll
                .focus_target("sidebar-last-item", &state.end, row)
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests;
