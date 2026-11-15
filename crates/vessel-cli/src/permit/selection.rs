//! Allow without an id is a batch; deny requires explicit batch intent.
use crate::selector::IdPrefix;

/// The command determines whether an omitted request is an intentional batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Allow,
    Deny,
}

/// A validated selection intent, before looking up any pending request.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Scope<'a> {
    All,
    One(&'a IdPrefix),
}

/// A missing denial scope is refused before connecting or changing state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("request id is required for deny unless --all is specified")]
pub(crate) struct SelectionError;

pub(super) fn scope(
    direction: Direction,
    request: Option<&IdPrefix>,
) -> Result<Scope<'_>, SelectionError> {
    match (direction, request) {
        (Direction::Allow, None) => Ok(Scope::All),
        (Direction::Deny, None) => Err(SelectionError),
        (Direction::Allow | Direction::Deny, Some(request)) => Ok(Scope::One(request)),
    }
}
