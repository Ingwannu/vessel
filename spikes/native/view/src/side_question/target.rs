//! A side panel follows a validated machine-book entry. A successful local
//! edit changes where this panel sends requests, never the remote session.
//!
//! [Decision Log]
//! - Purpose: a rename must not leave side input addressed to an old name;
//!   forgetting a machine must discard its local panel and pending replies.
//! - Constraint: the book's durable edit is authoritative; failed writes do
//!   not call this transition. Reuse its Edit and Name contract.
//! - Choice: return a typed local effect for the app hook, keeping the parent
//!   id on a rename and closing on a forget. No remote operation is implied.
//! - Cost: the app must refresh its stream identity when moving the name so
//!   late results addressed to the old name cannot restore stale state.

use vessel_proto::SessionId;

use crate::machine_book::{Edit, Name};

/// One node-local parent, addressed through its validated book entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The same name the connection registry uses.
    pub machine: Name,
    /// The original conversation; the side id is discovered separately.
    pub parent: SessionId,
}

/// What a successful book edit does to this panel locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookEffect {
    /// This edit concerns another machine, or changes only the default.
    Keep,
    /// Retain this conversation and address it through the new book name.
    Move(Target),
    /// Close and clear local state, including pending work and replies.
    Close,
}

impl Target {
    /// Called only after the durable book write succeeded. The app applies
    /// this to its own conversation, draft and in-flight request identity.
    #[must_use]
    pub fn after_book_edit(&self, edit: &Edit) -> BookEffect {
        match edit {
            Edit::Rename { from, to } if from == &self.machine => BookEffect::Move(Self {
                machine: to.clone(),
                parent: self.parent,
            }),
            Edit::Forget(name) if name == &self.machine => BookEffect::Close,
            Edit::Rename { .. } | Edit::MakeDefault(_) | Edit::Forget(_) => BookEffect::Keep,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> Target {
        Target {
            machine: Name::parse("laptop").unwrap(),
            parent: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        }
    }

    #[test]
    fn a_successful_rename_preserves_the_parent_and_moves_the_address() {
        let target = target();
        assert_eq!(
            target.after_book_edit(&Edit::Rename {
                from: target.machine.clone(),
                to: Name::parse("work-laptop").unwrap(),
            }),
            BookEffect::Move(Target {
                machine: Name::parse("work-laptop").unwrap(),
                parent: target.parent,
            })
        );
    }

    #[test]
    fn a_forget_only_closes_the_machine_it_names() {
        let target = target();
        assert_eq!(
            target.after_book_edit(&Edit::Forget(target.machine.clone())),
            BookEffect::Close
        );
        assert_eq!(
            target.after_book_edit(&Edit::Forget(Name::parse("server").unwrap())),
            BookEffect::Keep
        );
        assert_eq!(
            target.after_book_edit(&Edit::Rename {
                from: Name::parse("server").unwrap(),
                to: Name::parse("other-server").unwrap()
            }),
            BookEffect::Keep
        );
        assert_eq!(
            target.after_book_edit(&Edit::MakeDefault(target.machine.clone())),
            BookEffect::Keep
        );
    }
}
