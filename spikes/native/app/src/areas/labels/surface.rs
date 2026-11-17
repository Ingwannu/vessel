//! Immutable surface/decision ownership for the shared captured-dismiss hook.

use vessel_app_view::labels::{ManagerState, Ticket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Opening(Ticket);

impl Opening {
    pub(super) const fn new(ticket: Ticket) -> Self {
        Self(ticket)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Dismiss {
    Manager(Opening),
    Confirmation {
        opening: Opening,
        inspection: Ticket,
    },
}

pub(super) enum Outcome {
    Close,
    CancelConfirmation,
    Stale,
}

impl Dismiss {
    pub(super) const fn capture(opening: Opening, state: &ManagerState) -> Self {
        match state {
            ManagerState::Confirming { ticket, .. } => Self::Confirmation {
                opening,
                inspection: *ticket,
            },
            ManagerState::Listing
            | ManagerState::Editing(_)
            | ManagerState::Saving { .. }
            | ManagerState::Inspecting { .. }
            | ManagerState::Deleting { .. } => Self::Manager(opening),
        }
    }

    pub(super) fn decide(self, current: Option<Opening>, state: &ManagerState) -> Outcome {
        match self {
            Self::Manager(opening) => {
                if current != Some(opening) || matches!(state, ManagerState::Confirming { .. }) {
                    Outcome::Stale
                } else {
                    Outcome::Close
                }
            }
            Self::Confirmation {
                opening,
                inspection,
            } => {
                if current == Some(opening)
                    && matches!(state, ManagerState::Confirming { ticket, .. } if *ticket == inspection)
                {
                    Outcome::CancelConfirmation
                } else {
                    Outcome::Stale
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::labels::{Color, Count, Draft, Label, Tickets};

    fn draft() -> Draft {
        Draft::new(Label {
            name: "Ready".parse().unwrap(),
            color: Color::Violet,
        })
    }

    #[test]
    fn labels_old_surface_and_old_inspection_cannot_dismiss_a_new_decision() {
        let mut tickets = Tickets::default();
        let old = Opening::new(tickets.issue().unwrap());
        let current = Opening::new(tickets.issue().unwrap());
        let old_close = Dismiss::capture(old, &ManagerState::Listing);
        assert!(matches!(
            old_close.decide(Some(current), &ManagerState::Listing),
            Outcome::Stale
        ));
        let inspection = tickets.issue().unwrap();
        let confirming = ManagerState::Confirming {
            ticket: inspection,
            draft: draft(),
            count: Count::new(1),
        };
        assert!(matches!(
            Dismiss::capture(current, &ManagerState::Listing).decide(Some(current), &confirming),
            Outcome::Stale
        ));
        let close = Dismiss::capture(current, &confirming);
        assert!(matches!(
            close.decide(Some(current), &confirming),
            Outcome::CancelConfirmation
        ));
        let newer = ManagerState::Confirming {
            ticket: tickets.issue().unwrap(),
            draft: draft(),
            count: Count::new(2),
        };
        assert!(matches!(
            close.decide(Some(current), &newer),
            Outcome::Stale
        ));
        assert!(matches!(
            close.decide(Some(current), &ManagerState::Editing(draft())),
            Outcome::Stale
        ));
    }

    #[test]
    fn labels_actual_close_is_distinct_from_hiding_an_accepted_write() {
        let mut tickets = Tickets::default();
        let opening = Opening::new(tickets.issue().unwrap());
        let saving = ManagerState::Saving {
            ticket: tickets.issue().unwrap(),
            draft: draft(),
        };
        let close = Dismiss::capture(opening, &saving);
        assert!(matches!(
            close.decide(Some(opening), &saving),
            Outcome::Close
        ));
        assert!(matches!(close.decide(None, &saving), Outcome::Stale));
    }
}
