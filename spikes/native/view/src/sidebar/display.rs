//! The sidebar's Display menu, as Paseo's (`components/sidebar/
//! display-preferences/menu.tsx`): a root of decisions, each with its value
//! at the right, opening a page of its choices; choosing one sets it and
//! closes the menu. The decisions are the page's (`features/shell/
//! sidebar-view-menu.tsx`): grouping, order, and whether ended sessions are
//! listed, kept in the preferences (`prefs::SidebarView`).
//!
//! [Decision Log]
//! - Purpose: the person chooses how the sidebar lists sessions from the
//!   sidebar itself, as in Paseo and the page.
//! - Constraint: the app's menu draws pages in place with a way back (the
//!   kit's combobox, Paseo's compact sheet); it has no flyout beside a
//!   row (Paseo's wide popover opens a page by hover intent).
//! - Choice: Paseo's root and pages (its rows, its "options carry icons,
//!   the root rows do not", its check on the chosen one, closing on a
//!   choice); the page's "Show ended sessions" stays a switch on the root,
//!   flipped in place, since it is one bit and Paseo has no such decision.
//! - Rejected: the page's flat menu (two headed radio groups and a
//!   switch): Paseo, the reference, moved to pages so each decision reads
//!   as one row with its value.
//! - Here, not in the app: what each row is and what choosing it does,
//!   so it is tested without a window.

use crate::prefs::{Grouping, PreferenceChange, SessionSort, SidebarView};
use crate::text::SidebarWords;

/// Which page of the menu shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DisplayPage {
    /// The decisions.
    #[default]
    Root,
    /// Grouping: by machine or by status.
    Grouping,
    /// The order within a group.
    Sort,
}

/// What leads an option (Paseo's option icons; the app picks the glyph).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionMark {
    /// Grouping by machine (Paseo's host mark).
    Machine,
    /// Grouping by status (Paseo's dashed circle).
    Status,
}

/// What a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A decision: its value at the right, a page behind it.
    Opens {
        /// The value now.
        value: &'static str,
    },
    /// A switch flipped in place.
    Switch {
        /// On or off now.
        on: bool,
    },
    /// One choice of a page.
    Choice {
        /// It is the one in effect.
        checked: bool,
        /// Its leading mark, if the page's options carry one.
        mark: Option<OptionMark>,
    },
}

/// One row of a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayRow {
    /// Its words.
    pub label: &'static str,
    /// What it is.
    pub kind: RowKind,
}

/// What choosing a row does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayAct {
    /// Show that page.
    Open(DisplayPage),
    /// Keep this change; the menu stays open (a switch).
    Flip(PreferenceChange),
    /// Keep this change and close the menu (a choice).
    Choose(PreferenceChange),
}

impl DisplayPage {
    /// The page's title (with its way back); the root has none.
    #[must_use]
    pub const fn title(self, words: &SidebarWords) -> Option<&'static str> {
        match self {
            Self::Root => None,
            Self::Grouping => Some(words.grouping),
            Self::Sort => Some(words.sort),
        }
    }

    /// The page's rows for `view`.
    #[must_use]
    pub fn rows(self, view: SidebarView, words: &SidebarWords) -> Vec<DisplayRow> {
        match self {
            Self::Root => vec![
                DisplayRow {
                    label: words.grouping,
                    kind: RowKind::Opens {
                        value: words.group(view.group),
                    },
                },
                DisplayRow {
                    label: words.sort,
                    kind: RowKind::Opens {
                        value: words.sort_by(view.sort),
                    },
                },
                DisplayRow {
                    label: words.show_ended,
                    kind: RowKind::Switch { on: view.ended },
                },
            ],
            Self::Grouping => GROUPINGS
                .iter()
                .map(|&(group, mark)| DisplayRow {
                    label: words.group(group),
                    kind: RowKind::Choice {
                        checked: view.group == group,
                        mark: Some(mark),
                    },
                })
                .collect(),
            Self::Sort => SORTS
                .iter()
                .map(|&sort| DisplayRow {
                    label: words.sort_by(sort),
                    kind: RowKind::Choice {
                        checked: view.sort == sort,
                        mark: None,
                    },
                })
                .collect(),
        }
    }

    /// What choosing row `index` of this page does; `None` past the rows.
    #[must_use]
    pub fn choose(self, index: usize, view: SidebarView) -> Option<DisplayAct> {
        match self {
            Self::Root => match index {
                0 => Some(DisplayAct::Open(Self::Grouping)),
                1 => Some(DisplayAct::Open(Self::Sort)),
                2 => Some(DisplayAct::Flip(PreferenceChange::SidebarEnded(
                    !view.ended,
                ))),
                _ => None,
            },
            Self::Grouping => GROUPINGS
                .get(index)
                .map(|&(group, _)| DisplayAct::Choose(PreferenceChange::SidebarGroup(group))),
            Self::Sort => SORTS
                .get(index)
                .map(|&sort| DisplayAct::Choose(PreferenceChange::SidebarSort(sort))),
        }
    }
}

/// The groupings, in the menu's order, with their marks.
const GROUPINGS: [(Grouping, OptionMark); 2] = [
    (Grouping::Machine, OptionMark::Machine),
    (Grouping::Status, OptionMark::Status),
];

/// The orders, in the menu's order.
const SORTS: [SessionSort; 2] = [SessionSort::Created, SessionSort::Activity];
