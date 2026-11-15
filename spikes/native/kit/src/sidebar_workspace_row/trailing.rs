//! Workspace trailing data stays in place under hover actions, as the source
//! `resolveTrailingActionVisibility` and `TrailingActionScrim` prescribe.
use super::paint::Paints;
use super::{Density, Hint};
use gpui::prelude::*;
use gpui::{
    AnyElement, Background, Div, Hsla, SharedString, div, linear_color_stop, linear_gradient, px,
};
/// Keep an open menu's trigger mounted after the pointer leaves its row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MenuVisibility {
    /// Normal desktop hover, permanent on compact.
    #[default]
    OnHover,
    /// A caller-owned menu is already open.
    Retained,
}
/// Real data and the caller's shared-button action. This type never constructs
/// a private button or knows which workspace action it dispatches.
#[must_use]
#[derive(Default)]
pub struct Trailing {
    content: Option<AnyElement>,
    action: Option<AnyElement>,
    menu: MenuVisibility,
}
impl std::fmt::Debug for Trailing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Trailing")
            .field("menu", &self.menu)
            .finish_non_exhaustive()
    }
}
impl Trailing {
    /// Timestamp/diff content from its typed projection, not a guessed string.
    pub fn content(mut self, content: AnyElement) -> Self {
        self.content = Some(content);
        self
    }
    /// A real shared Button/menu trigger. Its callbacks remain caller-owned.
    pub fn action(mut self, action: AnyElement) -> Self {
        self.action = Some(action);
        self
    }
    /// Preserve the menu trigger while its caller-owned menu is open.
    pub fn visibility(mut self, menu: MenuVisibility) -> Self {
        self.menu = menu;
        self
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    content: Content,
    action: Action,
    reserve: Reserve,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Content {
    Absent,
    Visible,
    Hidden,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Absent,
    Hover,
    Shown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reserve {
    Zero,
    Minimum,
}
fn layout(trailing: &Trailing, hint: Hint, density: Density) -> Layout {
    let has_action = trailing.action.is_some();
    let has_content = trailing.content.is_some() && !(has_action && density == Density::Compact);
    let content = if !has_content {
        Content::Absent
    } else if hint == Hint::Shown {
        Content::Hidden
    } else {
        Content::Visible
    };
    let action = if !has_action || hint == Hint::Shown {
        Action::Absent
    } else if density == Density::Compact || trailing.menu == MenuVisibility::Retained {
        Action::Shown
    } else {
        Action::Hover
    };
    let reserve = if has_content || (has_action && density == Density::Compact) {
        Reserve::Minimum
    } else {
        Reserve::Zero
    };
    Layout {
        content,
        action,
        reserve,
    }
}
pub(super) fn render(
    group: SharedString,
    trailing: Trailing,
    hint: Hint,
    density: Density,
    paints: Paints,
) -> Option<Div> {
    if trailing.content.is_none() && trailing.action.is_none() {
        return None;
    }
    let layout = layout(&trailing, hint, density);
    let mut slot = div()
        .relative()
        .flex_none()
        .min_h(px(20.0))
        .flex()
        .flex_col()
        .items_end()
        .justify_start()
        .when(layout.reserve == Reserve::Minimum, |slot| {
            slot.min_w(px(18.0))
        });
    if layout.content != Content::Absent {
        slot = slot.child(
            div()
                .when(layout.content == Content::Hidden, |content| {
                    content.opacity(0.0)
                })
                .children(trailing.content),
        );
    }
    if layout.action != Action::Absent {
        if density == Density::Desktop {
            slot = slot.child(scrim(group.clone(), paints));
        }
        slot=slot.child(div().id(format!("{group}-trailing-action")).absolute().top_0().right_0().flex()
            .when(layout.action==Action::Hover,|action|action.hidden().group_hover(group,Styled::flex))
            // Shared buttons own dispatch. Even a disabled nested action must
            // not turn its press into the parent workspace's navigation.
            .on_click(|_,_,cx|cx.stop_propagation()).children(trailing.action));
    }
    Some(slot)
}
const SCRIM_WIDTH: f32 = 48.0;
const SOLID_AT: f32 = 0.55;
fn gradient(color: Hsla) -> Background {
    let [clear, solid] = stops(color);
    linear_gradient(90.0, clear, solid)
}
fn stops(color: Hsla) -> [gpui::LinearColorStop; 2] {
    [
        linear_color_stop(color.opacity(0.0), 0.0),
        linear_color_stop(color, SOLID_AT),
    ]
}

fn scrim(group: SharedString, paints: Paints) -> gpui::Stateful<Div> {
    div()
        .id(format!("{group}-trailing-scrim"))
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .w(px(SCRIM_WIDTH))
        .hidden()
        .bg(gradient(paints.rest))
        .group_hover(group.clone(), move |style| {
            style.flex().bg(gradient(paints.hover))
        })
        .group_active(group, move |style| style.bg(gradient(paints.pressed)))
}
#[cfg(test)]
mod tests;
