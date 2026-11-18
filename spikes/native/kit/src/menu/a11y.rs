//! What a menu row is to assistive technology (TRACKER 808): Paseo's
//! `MenuItem` (`components/ui/menu/menu-item.tsx` 298-342) is a
//! `menuitem` named by its text.
//!
//! [Decision Log]
//! - Purpose and intent: every menu row has a role and a name in GPUI's
//!   accessibility tree, as Paseo's rows do; 792's Create row was unnamed.
//! - Existing implementation and constraints: one renderer draws ordinary
//!   rows, page rows and the form page's action row (`draw::row_body`);
//!   GPUI 0.3.10 sets a role, a name, toggled and expanded on an element,
//!   and a click handler gives its node a Click action, but it has no
//!   setter for disabled or has-popup.
//! - Alternatives considered: a custom element per row writing disabled
//!   and has-popup into its accesskit node (a second paint path for every
//!   row, and GPUI's debug tree records neither, so untestable here).
//! - Chosen approach: `Role::MenuItem` named by the words shown (the
//!   pending words while pending) and the row's hint or value after them,
//!   as Paseo's name comes from its text; a disabled or pending row takes
//!   no press, so it offers no Click; a page's row says whether its page is
//!   open (`aria_expanded`, `sub_row`).
//! - Pros, cons and impact: disabled and has-popup are not said as states
//!   (accepted by the coordinator, 2026-11-18, as a stated difference).

use gpui::prelude::*;
use gpui::{Div, SharedString, Stateful};

use super::MenuItem;

/// `row` as a menu item named by `item`'s words and hint.
pub(super) fn named(row: Stateful<Div>, item: &MenuItem) -> Stateful<Div> {
    row.role(gpui::Role::MenuItem).aria_label(name(item))
}

/// The words shown, then the hint or value shown after them.
fn name(item: &MenuItem) -> SharedString {
    match &item.value {
        Some(value) => format!("{} {value}", item.words()).into(),
        None => item.words().clone(),
    }
}
