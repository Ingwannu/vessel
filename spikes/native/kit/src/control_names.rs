//! The accessible names of the kit's own wordless controls: the dialog's close
//! button and the sheet header's back arrow (TRACKER 782).
//!
//! [Decision Log]
//! - Purpose: Paseo names these controls for assistive technology
//!   (`adaptive-modal-sheet.tsx` 411-434: `accessibilityRole="button"`,
//!   `t("common.actions.close")`, and the back arrow's own label or
//!   `t("common.actions.back")`). Vessel's were bare glyphs with no role and
//!   no name, so a screen reader met an unnamed, unactionable shape.
//! - Constraint: the kit has no words of its own; every word comes from the
//!   app's catalog. `DialogContent` is built by struct literal in every
//!   dialog, so a new required field would change every caller.
//! - Alternatives: a name field on each dialog (every caller changes, and a
//!   forgotten one is still unnamed); English words fixed in the kit (wrong
//!   for Korean).
//! - Choice: one `Global` holding the names, set by the app from its catalog
//!   at startup and at every language change; the kit's controls read it when
//!   drawn. Until the app sets it, the names are English.
//! - Impact: the app owns one call per language change. A control drawn with
//!   a caller's own name (`HeaderBack::with_label`) ignores the global.

use gpui::{App, Global, SharedString};

/// The names the kit's wordless controls announce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlNames {
    /// The dialog's close button (`common.actions.close`).
    pub close: SharedString,
    /// The sheet header's back arrow (`common.actions.back`).
    pub back: SharedString,
}

impl Default for ControlNames {
    fn default() -> Self {
        Self {
            close: SharedString::new_static("Close"),
            back: SharedString::new_static("Back"),
        }
    }
}

impl Global for ControlNames {}

/// Name the kit's wordless controls in the app's language. Call at startup
/// and whenever the language changes; the next frame announces the new names.
pub fn set_control_names(names: ControlNames, cx: &mut App) {
    cx.set_global(names);
}

/// The names in use: the app's, or English before the app sets them.
#[must_use]
pub fn control_names(cx: &App) -> ControlNames {
    cx.try_global::<ControlNames>()
        .cloned()
        .unwrap_or_default()
}
