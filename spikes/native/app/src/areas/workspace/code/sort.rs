//! The Files tree's order, as Paseo's file explorer sorts (its header's
//! sort trigger, `file-explorer-pane.tsx` 1236; `tree.ts`
//! `sortExplorerEntries`): Name, Modified, Size, a press moving to the next;
//! folders first, then by the order. The page's `files-tab.tsx` and
//! `use-entry-facts.ts`, with the same preference (`layout.explorer_sort`).
//!
//! [Decision Log]
//! - Purpose: the Modified sort (ADR 0105's "later work"), native after the
//!   page.
//! - Constraint: `file.list` gives names and kinds, not times; the node's
//!   `file.inspect` (protocol 27) gives both for one folder. Name needs
//!   neither, so a tree in Name order asks nothing more of the machine; a
//!   machine without the file panels protocol sorts by name and shows no
//!   trigger.
//! - Choice: a folder is inspected each time its listing arrives while
//!   Modified or Size is chosen, and every listed folder when one of them
//!   is chosen after Name (between the two the facts are already there);
//!   an answer for a root no longer shown, or a folder no longer listed,
//!   is dropped. A folder whose inspection fails keeps no facts: its
//!   entries sort last. The order is kept on the tab (`Files::sort`) so the
//!   many places that draw the tree again need no preferences; it is read
//!   again when the root's machine is known (`code_read_tab`) and when the
//!   preference changes.
//! - Cost: one more call per shown folder while Modified or Size is chosen.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::ui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, glyph};
use vessel_app_link::workspace::{Ask, FileAsk};
use vessel_app_view::prefs::{ExplorerSort, PreferenceChange};
use vessel_app_view::workspace::file_panels::offered;
use vessel_app_view::workspace::file_tree::{EntryFacts, Listing};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::RelPath;
use vessel_proto::workspace::directory_metadata::{
    IgnoredEntries, InspectDirectory, InspectedDirectory,
};

use crate::shell::Shell;

impl Shell {
    /// Whether the machine of the root shown has the file panels' calls.
    pub(super) fn code_panels_offered(&self) -> bool {
        self.areas
            .workspace
            .code
            .root()
            .is_some_and(|(machine, _)| {
                offered(
                    self.fleet
                        .machines
                        .iter()
                        .find(|known| known.name == machine)
                        .and_then(|known| known.protocol),
                )
            })
    }

    /// The order the tree is drawn in: the preference, or Name where the
    /// machine cannot say times and sizes.
    pub(super) fn explorer_sort(&self) -> ExplorerSort {
        if self.code_panels_offered() {
            self.prefs.settings.layout.explorer_sort
        } else {
            ExplorerSort::Name
        }
    }

    /// The order changed (the trigger, or the preferences read): the tree
    /// drawn again, and each listed folder inspected when it needs facts.
    pub(crate) fn explorer_sorted(&mut self) {
        let sort = self.explorer_sort();
        let files = &mut self.areas.workspace.code.files;
        let was = std::mem::replace(&mut files.sort, sort);
        files.relist();
        // From Modified to Size or back, every listed folder was inspected
        // as it arrived.
        if sort == ExplorerSort::Name || was != ExplorerSort::Name {
            return;
        }
        let listed: Vec<RelPath> = files
            .listings
            .iter()
            .filter(|(_, listing)| matches!(listing, Listing::Listed(_)))
            .map(|(dir, _)| dir.clone())
            .collect();
        for dir in listed {
            self.inspect_folder(&dir);
        }
    }

    /// Ask for folder `dir`'s times and sizes, when the order needs them.
    pub(super) fn inspect_folder(&mut self, dir: &RelPath) {
        if self.explorer_sort() == ExplorerSort::Name {
            return;
        }
        let Some((machine, root)) = self
            .areas
            .workspace
            .code
            .root()
            .map(|(machine, root)| (machine.to_owned(), *root))
        else {
            return;
        };
        self.ask(
            &machine,
            Ask::File(FileAsk::Inspect(InspectDirectory {
                root,
                directory: dir.clone(),
                ignored: IgnoredEntries::Include,
            })),
        );
    }

    /// A folder's times and sizes arrived.
    pub(in crate::areas::workspace) fn folder_inspected(
        &mut self,
        ask: &InspectDirectory,
        result: Result<InspectedDirectory, Problem>,
    ) {
        let panel = &mut self.areas.workspace.code;
        if panel.root().map(|(_, root)| root) != Some(&ask.root)
            || !matches!(
                panel.files.listings.get(&ask.directory),
                Some(Listing::Listed(_))
            )
        {
            return;
        }
        let Ok(inspected) = result else {
            return;
        };
        for entry in inspected.entries {
            let _was = panel.files.facts.insert(
                entry.path,
                EntryFacts {
                    modified: entry.modified.map(|at| at.0),
                    size: entry.size.map(|size| size.0),
                },
            );
        }
        panel.files.relist();
    }
}

/// Paseo's sort trigger: the order's word and a 12 chevron, 24 high, 4
/// in, a hover on the sidebar's hover surface; a press moves to the next
/// order. Nothing where the machine cannot say times and sizes. A button
/// named by its word, as Paseo's `Pressable` is named by its text (TRACKER
/// 808).
pub(super) fn trigger(
    shell: &Shell,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !shell.code_panels_offered() {
        return None;
    }
    let labels = shell.words().labels();
    let sort = shell.explorer_sort();
    let word = match sort {
        ExplorerSort::Name => labels.sort_name,
        ExplorerSort::Modified => labels.sort_modified,
        ExplorerSort::Size => labels.sort_size,
    };
    let neutral = tokens.neutral;
    let hover = super::tree::hover_surface(tokens);
    Some(
        div()
            .id("files-sort-trigger")
            .role(gpui::Role::Button)
            .aria_label(word)
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(spacing::S1))
            .h(px(spacing::S6))
            .px(px(spacing::S1))
            .rounded(px(radius::R1))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .type_(Type::Meta)
            .text_color(neutral.foreground_muted)
            .child(word)
            .child(glyph(Glyph::ChevronDown, 12.0, neutral.foreground_muted))
            .on_click(cx.listener(move |this, _event, window, cx| {
                this.prefer(&PreferenceChange::ExplorerSort(sort.next()), window, cx);
            }))
            .into_any_element(),
    )
}
