//! Typed usage boundary. No credentials or vendor error text can enter its replies.
//! Inventory uses the existing profile.list contract; accounting and quota calls
//! are registered only after their node contracts reach the native baseline.

use vessel_app_view::devices::MachineKey;
use vessel_app_view::usage::{Account, ReadRequest, RequestFailure};

/// Usage operations retain the exact identity reserved by the view model.
#[derive(Debug, Clone)]
pub enum Ask {
    /// Configured bindings, including defaults; never scans login files.
    Accounts(ReadRequest<MachineKey>),
}

/// Typed observations stay separate from the UI's request-lifetime decisions.
#[derive(Debug)]
pub enum Answer {
    /// Configured account inventory, or its sanitized failure.
    Accounts {
        /// The original correlation and machine selection.
        request: ReadRequest<MachineKey>,
        /// Human account labels and typed profile identities only.
        result: Result<Vec<Account>, RequestFailure>,
    },
}
impl Ask {
    /// A full queue, closed link or unsupported surface settles its own request.
    #[must_use]
    pub fn refused(self, reason: RequestFailure) -> Answer {
        match self {
            Self::Accounts(request) => Answer::Accounts {
                request,
                result: Err(reason),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::usage::{Phase, Reader, configured_accounts};

    /// A plain in-memory node implementation has no path to credentials or processes.
    struct FakeNode;
    impl FakeNode {
        fn answer(ask: Ask) -> Answer {
            match ask {
                Ask::Accounts(request) => Answer::Accounts {
                    request,
                    result: Ok(configured_accounts(&[])),
                },
            }
        }
    }

    #[test]
    fn the_fake_boundary_and_queue_refusal_keep_correlation() {
        let mut state: Reader<MachineKey, Vec<Account>> = Reader::default();
        let machine = MachineKey::from_book("test-machine");
        let request = state.request(machine.clone()).unwrap();
        let expected = request.ticket;
        let Answer::Accounts { request, result } = FakeNode::answer(Ask::Accounts(request));
        assert_eq!(request.selection, machine);
        assert_eq!(request.ticket, expected);
        assert!(state.answer(request.ticket, result).is_none());
        assert!(matches!(state.phase(), Phase::Ready(accounts) if accounts.is_empty()));
        let request = state.request(machine.clone()).unwrap();
        let Answer::Accounts { request, result } =
            Ask::Accounts(request).refused(RequestFailure::Busy);
        assert_eq!(request.selection, machine);
        assert!(state.answer(request.ticket, result).is_none());
        assert!(matches!(
            state.phase(),
            Phase::Failed {
                reason: RequestFailure::Busy,
                retained: Some(_)
            }
        ));
    }
}
