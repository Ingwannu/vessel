//! One workspace row for pinned, project and status sections.
//! References: sidebar-workspace-row.tsx:467–521 and
//! sidebar-workspace-row-content.tsx:87–548; density.ts:23–32.
//!
//! [Decision Log] Navigation rows cannot supply the workspace title/meta rail,
//! trailing overlay or numeric hint geometry. This renderer owns those visual
//! rules; callers supply facts, prepared metadata and real shared-button actions.
//! GPUI owns drag lifetime. No workspace, route or node type enters this kit.

pub use crate::screen_header::Density;
pub use crate::sidebar_header_row::Selection;
use crate::{
    Tokens, Type, TypeExt as _, popup,
    theme::{radius, spacing},
};
use gpui::prelude::*;
use gpui::{AnyElement, App, ClickEvent, Div, FontWeight, SharedString, Stateful, Window, div, px};
mod paint;
#[cfg(test)]
mod tests;
mod trailing;
use paint::Paints;
pub use trailing::{MenuVisibility, Trailing};

/// The highlight spans the group width even when its content is indented.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Indent {
    /// Hoisted/pinned row on the sidebar's primary rail.
    #[default]
    Flush,
    /// Workspace nested below a project or status header.
    Grouped,
}
/// Creation has the source's distinct title opacity, not inferred status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Creation {
    /// Normal title opacity, fully opaque on hover.
    #[default]
    Idle,
    /// The provider reports creation pending.
    Creating,
}
/// A captured drag uses the pressed surface, border and shared shadow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DragState {
    /// No active drag of this exact row.
    #[default]
    Resting,
    /// The caller's typed GPUI drag captures this row.
    Dragging,
}
/// Presentation slots only. Numeric hint text is prepared from the view's
/// validated number; metadata remains in its existing owner's shared controls.
#[must_use]
pub struct SidebarWorkspaceRow {
    id: SharedString,
    title: SharedString,
    indent: Indent,
    selection: Selection,
    creation: Creation,
    drag: DragState,
    leading: Option<AnyElement>,
    meta: Option<AnyElement>,
    trailing: Trailing,
    shortcut: Option<SharedString>,
}
impl std::fmt::Debug for SidebarWorkspaceRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SidebarWorkspaceRow")
            .field("id", &self.id)
            .field("indent", &self.indent)
            .field("selection", &self.selection)
            .field("creation", &self.creation)
            .field("drag", &self.drag)
            .finish_non_exhaustive()
    }
}
impl SidebarWorkspaceRow {
    /// Stable UI identity and whole title; no path/name identity guessing.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            indent: Indent::Flush,
            selection: Selection::Inactive,
            creation: Creation::Idle,
            drag: DragState::Resting,
            leading: None,
            meta: None,
            trailing: Trailing::default(),
            shortcut: None,
        }
    }
    /// Indent content while retaining a full-width selected/hover fill.
    pub fn indent(mut self, indent: Indent) -> Self {
        self.indent = indent;
        self
    }
    /// Reflect the pane owner's selected target, never a private navigation copy.
    pub fn selected(mut self, selection: Selection) -> Self {
        self.selection = selection;
        self
    }
    /// Use the explicit creation affordance from the provider.
    pub fn creation(mut self, creation: Creation) -> Self {
        self.creation = creation;
        self
    }
    /// The caller holds GPUI's typed drag identity and supplies its current state.
    pub fn dragging(mut self, drag: DragState) -> Self {
        self.drag = drag;
        self
    }
    /// Supply the existing project/status visual in the shared 16/20 slot.
    pub fn leading(mut self, leading: AnyElement) -> Self {
        self.leading = Some(leading);
        self
    }
    /// Supply the host/branch/labels/services row using its owners' controls.
    pub fn metadata(mut self, meta: AnyElement) -> Self {
        self.meta = Some(meta);
        self
    }
    /// Source trailing content and the real shared-button action.
    pub fn trailing(mut self, trailing: Trailing) -> Self {
        self.trailing = trailing;
        self
    }
    /// Temporary workspace-number overlay, not a hardcoded keyboard binding.
    pub fn shortcut(mut self, label: impl Into<SharedString>) -> Self {
        self.shortcut = Some(label.into());
        self
    }
    /// One pointer target; the returned row accepts existing GPUI drag handlers.
    /// Keyboard activation/focus is the caller's explicit registration.
    pub fn element(
        self,
        density: Density,
        tokens: &Tokens,
        press: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Stateful<Div> {
        let paints = Paints::new(self.selection, self.drag, tokens);
        let id = self.id.clone();
        let indent = self.indent;
        let drag = self.drag;
        let content = contents(self, density, tokens, paints);
        frame(id, indent, drag, density, tokens, paints)
            .child(content)
            .on_click(press)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hint {
    Absent,
    Shown,
}
fn contents(row: SidebarWorkspaceRow, density: Density, tokens: &Tokens, paints: Paints) -> Div {
    let SidebarWorkspaceRow {
        id,
        title,
        creation,
        leading,
        meta,
        trailing,
        shortcut,
        ..
    } = row;
    let hint = if shortcut.is_some() {
        Hint::Shown
    } else {
        Hint::Absent
    };
    let label = div()
        .flex_1()
        .min_w_0()
        .truncate()
        .type_(Type::Label)
        .font_weight(FontWeight::NORMAL)
        .text_color(tokens.neutral.foreground)
        .opacity(match creation {
            Creation::Idle => 0.76,
            Creation::Creating => 0.92,
        })
        .when(creation == Creation::Idle, |label| {
            label.group_hover(id.clone(), |style| style.opacity(1.0))
        })
        .child(title);
    let title = div()
        .flex()
        .items_start()
        .justify_between()
        .gap(px(spacing::S2))
        .child(label)
        .children(trailing::render(id, trailing, hint, density, paints));
    let column = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .child(title)
        .children(meta);
    let main = div()
        .w_full()
        .flex()
        .items_start()
        .gap(px(spacing::S2))
        .child(
            div()
                .flex_none()
                .w(px(match density {
                    Density::Desktop => 16.0,
                    Density::Compact => 20.0,
                }))
                .h(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .children(leading),
        )
        .child(column);
    div()
        .relative()
        .w_full()
        .child(main)
        .children(shortcut.map(|label| badge(label, tokens)))
}
fn badge(label: SharedString, tokens: &Tokens) -> Div {
    div()
        .absolute()
        .top(px(1.0))
        .right_0()
        .min_w(px(18.0))
        .h(px(18.0))
        .px(px(spacing::S1))
        .rounded(px(radius::SM))
        .border_1()
        .border_color(tokens.neutral.surface2)
        .bg(tokens.neutral.surface0)
        .flex()
        .items_center()
        .justify_center()
        .type_(Type::Meta)
        .line_height(px(14.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tokens.neutral.foreground_muted)
        .child(label)
}

fn frame(
    id: SharedString,
    indent: Indent,
    drag: DragState,
    density: Density,
    tokens: &Tokens,
    paints: Paints,
) -> Stateful<Div> {
    div()
        .id(id.clone())
        .group(id)
        .drag_threshold(crate::drag::LIST_DRAG_DISTANCE)
        .relative()
        .w_full()
        .flex()
        .flex_col()
        .min_h(px(match density {
            Density::Desktop => 36.0,
            Density::Compact => 44.0,
        }))
        .mb(px(spacing::S1))
        .py(px(spacing::S2))
        .pl(px(match indent {
            Indent::Flush => spacing::S2,
            Indent::Grouped => spacing::S4,
        }))
        .pr(px(spacing::S3))
        .rounded(px(radius::R2))
        .gap(px(spacing::S1))
        .justify_center()
        .bg(paints.rest)
        .cursor_pointer()
        .hover(move |style| style.bg(paints.hover))
        .active(move |style| style.bg(paints.pressed))
        .when(drag == DragState::Dragging, |row| {
            row.border_1()
                .border_color(tokens.neutral.border)
                .shadow(popup::shadow_md(tokens))
        })
}
