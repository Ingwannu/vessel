//! The Files tree's rows (Paseo's `tree-primitives.tsx` and
//! `file-explorer-pane.tsx` values): a folder's chevron or a file's glyph
//! in one 16 slot, 12 a level from 12, 6 above and below; the row a name
//! is typed in where an action asked for one; a right click opens the
//! entry's actions.

use super::file_open::FileOpenContext;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_view::workspace::file_tree::{Listing, TreeRow};
use vessel_proto::workspace::{EntryKind, RelPath};

impl Shell {
    /// Visibility of the existing legacy Files host plus its original scope.
    /// W12 supplies actual typed pane visibility when that host is mounted.
    pub(super) fn file_tree_scope_is_current(&self, origin: &FileOpenContext) -> bool {
        self.route == crate::routes::Route::Session
            && self.areas.workspace.code.open
            && self.areas.workspace.code.tab == super::Tab::Files
            && self.file_open_context_is_current(origin)
    }
    /// A rendered row retains its original scope across queued input delivery.
    pub(super) fn activate_file_row(
        &mut self,
        origin: &FileOpenContext,
        path: &RelPath,
        kind: EntryKind,
        cx: &mut Context<'_, Self>,
    ) {
        if !self.file_tree_scope_is_current(origin) {
            return;
        }
        if kind == EntryKind::Dir {
            self.toggle_folder(path, cx);
        } else {
            self.open_file(path.clone(), cx);
        }
    }
}

use crate::shell::Shell;

/// A level's indent.
const INDENT: f32 = 12.0;

pub(super) fn indented(depth: usize) -> f32 {
    // Whole levels, at most 32 deep: an f32 from a small integer.
    let levels = u16::try_from(depth.min(32)).unwrap_or(32);
    // Paseo's `treeRowPaddingLeft`: 12, then 12 a level. `mul_add`: the
    // lints deny arithmetic operators on floats.
    f32::from(levels).mul_add(INDENT, 12.0)
}

/// Paseo's sidebar hover surface (`surfaceSidebarHover`: the third
/// surface in the dark, the first in the light), which its trees use.
const fn hover_surface(tokens: &Tokens) -> vessel_app_kit::ui::Hsla {
    match tokens.scheme {
        vessel_app_kit::Scheme::Dark => tokens.neutral.surface3,
        vessel_app_kit::Scheme::Light => tokens.neutral.surface1,
    }
}

/// A row of the tree.
pub(super) fn tree_row(
    shell: &Shell,
    index: usize,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let files = &shell.areas.workspace.code.files;
    let neutral = tokens.neutral;
    let labels = shell.words().labels();
    let row = match files.slot(index) {
        super::Slot::Edit(depth) => return super::actions_view::edit_row(shell, depth, tokens, cx),
        super::Slot::Row(at) => match files.rows.get(at) {
            Some(row) => row,
            None => return div().into_any_element(),
        },
    };
    let quiet = |depth: usize, text: String| {
        div()
            .pl(px(indented(depth)))
            .pr(px(8.0))
            .py(px(4.0))
            .type_(Type::Meta)
            .text_color(neutral.foreground_muted)
            .child(text)
            .into_any_element()
    };
    match row {
        TreeRow::Loading { depth, .. } => quiet(*depth, labels.loading.to_owned()),
        TreeRow::Failed { message, depth, .. } => {
            super::viewer::failed(labels.list_failed, message, indented(*depth), tokens)
                .into_any_element()
        }
        TreeRow::More { depth, .. } => quiet(*depth, labels.more_entries.to_owned()),
        TreeRow::Empty { depth, .. } => quiet(*depth, labels.empty_folder.to_owned()),
        TreeRow::Entry { .. } => entry_row(shell, index, row, tokens, cx),
    }
}

/// An entry's row: a click opens a file or folds a folder, a right click
/// opens the entry's actions.
fn entry_row(
    shell: &Shell,
    index: usize,
    row: &TreeRow,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let TreeRow::Entry {
        dir,
        index: entry,
        path,
        depth,
        open,
    } = row
    else {
        return div().into_any_element();
    };
    let files = &shell.areas.workspace.code.files;
    let neutral = tokens.neutral;
    let Some(Listing::Listed(listing)) = files.listings.get(dir) else {
        return div().into_any_element();
    };
    let Some(entry) = listing.entries.get(*entry) else {
        return div().into_any_element();
    };
    let kind = entry.kind;
    let folder = kind == EntryKind::Dir;
    let origin = shell.file_open_context();
    let path = path.clone();
    // Paseo's tree row (`tree-primitives.tsx`, `file-explorer-pane.tsx`):
    // 6 above and below, 8 at the right, one 16 slot (a folder's
    // chevron, extra muted, or the file's glyph), 8 to the name in
    // the foreground at 0.76, whole on hover, the sidebar's hover
    // surface. A file ignored by git is dimmer still.
    let wash = hover_surface(tokens);
    let name_ink = if entry.ignored {
        neutral.foreground_extra_muted
    } else {
        neutral.foreground.opacity(0.76)
    };
    let name_hover = neutral.foreground;
    div()
        .id(("code-tree", index))
        .group("code-tree-row")
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .py(px(6.0))
        .pl(px(indented(*depth)))
        .pr(px(8.0))
        .cursor_pointer()
        .hover(move |style| style.bg(wash))
        .child(
            div()
                .size(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(if folder {
                    glyph(
                        if *open {
                            Glyph::ChevronDown
                        } else {
                            Glyph::ChevronRight
                        },
                        16.0,
                        neutral.foreground_extra_muted,
                    )
                } else {
                    glyph(Glyph::File, 16.0, neutral.foreground_muted)
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .type_(Type::Body)
                .text_color(name_ink)
                .group_hover("code-tree-row", move |style| style.text_color(name_hover))
                .child(entry.name.clone()),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener({
                let path = path.clone();
                let origin = origin.clone();
                let open = *open;
                move |this, event: &MouseDownEvent, window, cx| {
                    if origin
                        .as_ref()
                        .is_none_or(|original| !this.file_tree_scope_is_current(original))
                    {
                        return;
                    }
                    this.open_file_actions(path.clone(), folder, open, event.position, window, cx);
                    cx.stop_propagation();
                }
            }),
        )
        .on_click(cx.listener(move |this, _event, _window, cx| {
            if let Some(origin) = &origin {
                this.activate_file_row(origin, &path, kind, cx);
            }
        }))
        .into_any_element()
}
