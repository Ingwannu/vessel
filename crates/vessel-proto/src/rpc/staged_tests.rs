//! Staged rows (`(staged FEATURE)`, the Decision Log in methods.rs): the
//! per-row `Handler` macro gives a staged row a default body answering
//! `Unsupported`, while every other row stays a method its implementor must
//! write; and no staged row of the real table is assigned a number.

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use super::*;
use crate::{GetNode, Harness, ListHarnesses, NodeInfo};

/// A staged feature's constant, as a values package declares it.
const STAGED_DEMO: Option<ProtocolVersion> = None;

/// A trait made by the same per-row macro as `Handler`: one staged row
/// and one assigned row.
trait Demo: Sync {
    type Context: Sync;

    handler_method!((staged STAGED_DEMO); ; staged_call; GetNode; NodeInfo);
    handler_method!(2; ; assigned_call; ListHarnesses; Vec<Harness>);
}

/// Implements only the assigned row: the staged one needs no handler.
struct OnlyAssigned;

impl Demo for OnlyAssigned {
    type Context = ();

    async fn assigned_call(
        &self,
        _context: &(),
        _request: ListHarnesses,
    ) -> Result<Vec<Harness>, WireError> {
        Ok(Vec::new())
    }
}

/// The value of a future that is ready at once.
fn now<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    match pin!(future).poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future should be ready at once"),
    }
}

#[test]
fn a_staged_row_without_a_handler_answers_unsupported() {
    let error = now(OnlyAssigned.staged_call(&(), GetNode)).unwrap_err();
    assert_eq!(error.code, crate::ErrorCode::Unsupported);
    assert!(
        error
            .message
            .contains(&format!("`{}`", Method::NodeInfo.as_str())),
        "{}",
        error.message
    );
    assert!(
        now(OnlyAssigned.assigned_call(&(), ListHarnesses))
            .unwrap()
            .is_empty()
    );
}

/// A staged method is served by nothing yet, so it must stay refused on
/// every connection: assigning its number is the wiring package's step.
#[test]
fn no_staged_row_is_assigned_a_number() {
    for method in Method::ALL.iter().filter(|method| method.is_staged()) {
        assert_eq!(method.since(), None, "{} is staged", method.as_str());
    }
}
