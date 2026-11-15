//! Settings › Overview for the chosen machine, as Paseo's `HostSettingsPage`
//! (`screens/settings/host-page.tsx`): what its rename dialog accepts and
//! sends, and its remove.
//!
//! [Decision Log]
//! - Purpose: Paseo renames and removes a host from its Overview; Vessel's
//!   machine book already does both (`machine_book::Edit`, written by the
//!   CLI's own book code), so the page asks for those edits and nothing new.
//! - Choice: Paseo's `AdaptiveRenameModal` rules: an unchanged name sends
//!   nothing, an invalid one says why and sends nothing, and the dialog
//!   stays open until the book answers (`Editing::pending`); Paseo's remove
//!   confirm does the same. A name another machine has is refused here
//!   before the book refuses it too.
//! - Cost: the book's own refusal (`Failure::Taken` when another device
//!   took the name meanwhile) still arrives and is shown in the dialog.

use crate::machine_book::{Edit, Failure, Name};
use crate::machines::NameProblem;

/// What the rename dialog's text asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// The same name: nothing to send (Paseo's submit is disabled).
    Unchanged,
    /// A valid new name: this edit.
    Send(Edit),
    /// Why the text cannot be a name.
    Problem(NameProblem),
}

/// What `typed` asks of a machine named `current`, given every other
/// machine's name in the book.
#[must_use]
pub fn ask(current: &Name, typed: &str, others: &[&str]) -> Ask {
    let typed = typed.trim();
    if typed == current.as_str() {
        return Ask::Unchanged;
    }
    if others.contains(&typed) {
        return Ask::Problem(NameProblem::Taken(typed.to_owned()));
    }
    match Name::parse(typed) {
        Ok(to) => Ask::Send(Edit::Rename {
            from: current.clone(),
            to,
        }),
        Err(problem) => Ask::Problem(problem),
    }
}

/// A rename or remove dialog while it is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editing {
    /// The machine it is about.
    pub machine: Name,
    /// The edit sent and not yet answered.
    pub pending: Option<Edit>,
    /// The book's refusal of the last edit sent.
    pub refused: Option<Failure>,
}

impl Editing {
    /// The dialog opened for `machine`.
    #[must_use]
    pub const fn open(machine: Name) -> Self {
        Self {
            machine,
            pending: None,
            refused: None,
        }
    }

    /// Whether Rename may be pressed: a nonblank changed draft, while idle.
    /// Enter still validates and can show the blank-name error, as in Paseo.
    #[must_use]
    pub fn rename_enabled(&self, typed: &str) -> bool {
        let typed = typed.trim();
        self.pending.is_none() && !typed.is_empty() && typed != self.machine.as_str()
    }

    /// `edit` was sent.
    pub fn sent(&mut self, edit: Edit) {
        self.pending = Some(edit);
        self.refused = None;
    }

    /// The book answered `edit`: whether the dialog closes (it succeeded).
    /// An answer to another edit changes nothing.
    pub fn answered(&mut self, edit: &Edit, result: &Result<(), Failure>) -> bool {
        if self.pending.as_ref() != Some(edit) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(()) => true,
            Err(failure) => {
                self.refused = Some(failure.clone());
                false
            }
        }
    }
}
