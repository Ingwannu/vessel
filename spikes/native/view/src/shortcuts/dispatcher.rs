//! Action-handler ownership for the single shortcut engine. View owns ordering
//! and stale-cleanup protection; the app supplies current activation and calls
//! the owning feature's adapter. No callback, node or UI type is stored here.

use std::collections::BTreeMap;
use std::str::FromStr;

/// Workspace-scoped ids can contain node/workspace/pane identities.
const MAX_HANDLER_ID_BYTES: usize = 512;

/// Identifies one retained handler rather than one action or binding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandlerId(String);

impl FromStr for HandlerId {
    type Err = DispatchError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty()
            || text.len() > MAX_HANDLER_ID_BYTES
            || text.chars().any(char::is_control)
        {
            return Err(DispatchError::InvalidId);
        }
        Ok(Self(text.to_owned()))
    }
}

impl HandlerId {
    /// Stable identity, available to the caller's typed adapter table.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which feature context an action addresses; focus is checked independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionScope {
    /// App-wide action.
    Global,
    /// Current composer.
    MessageInput,
    /// Current sidebar selection.
    Sidebar,
    /// Current workspace and pane.
    Workspace,
}

/// An explicit result keeps unhandled actions eligible for the next handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handled {
    /// The feature performed the action.
    Yes,
    /// No action was performed; the next candidate may try.
    No,
}

/// High-priority overlays precede ordinary workspace and app handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Priority(i16);

impl Priority {
    /// An exact ordinal; the caller need not invent arbitrary float weights.
    #[must_use]
    pub const fn new(value: i16) -> Self {
        Self(value)
    }
}

/// A registration is deliberately separate from the action invocation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handler<A> {
    /// Retained feature instance.
    pub id: HandlerId,
    /// Closed action identities accepted by this handler.
    pub actions: Vec<A>,
    /// Highest priority runs first, newest registration wins ties.
    pub priority: Priority,
}

/// Only this registration's token may remove a handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerOwner {
    id: HandlerId,
    generation: u64,
}

/// A failed replacement leaves the previous handler intact.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DispatchError {
    /// Handler ids are bounded, nonempty and contain no control characters.
    #[error("invalid keyboard handler identity")]
    InvalidId,
    /// An empty action set is probably a missing feature registration.
    #[error("keyboard handler has no actions")]
    NoActions,
    /// Never recycle a token that a retained cleanup could still hold.
    #[error("keyboard handler generation exhausted")]
    GenerationExhausted,
}

#[derive(Debug)]
struct Registered<A> {
    handler: Handler<A>,
    generation: u64,
}

/// Handler metadata only; `dispatch` consults current activation at call time.
#[derive(Debug)]
pub struct Dispatcher<A> {
    handlers: BTreeMap<HandlerId, Registered<A>>,
    generation: u64,
}

impl<A> Default for Dispatcher<A> {
    fn default() -> Self {
        Self {
            handlers: BTreeMap::new(),
            generation: 0,
        }
    }
}

impl<A: PartialEq> Dispatcher<A> {
    /// Replace a retained instance, returning generation-guarded cleanup.
    ///
    /// # Errors
    /// Empty action sets and generation exhaustion are refused atomically.
    pub fn register(&mut self, handler: Handler<A>) -> Result<HandlerOwner, DispatchError> {
        if handler.actions.is_empty() {
            return Err(DispatchError::NoActions);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(DispatchError::GenerationExhausted)?;
        let id = handler.id.clone();
        self.generation = generation;
        drop(self.handlers.insert(
            id.clone(),
            Registered {
                handler,
                generation,
            },
        ));
        Ok(HandlerOwner { id, generation })
    }

    /// A stale cleanup is harmless after a handler instance is replaced.
    pub fn remove(&mut self, owner: &HandlerOwner) {
        if self
            .handlers
            .get(&owner.id)
            .is_some_and(|entry| entry.generation == owner.generation)
        {
            drop(self.handlers.remove(&owner.id));
        }
    }

    /// Ask active handlers in reference order until one performs the action.
    /// `enabled` queries the caller's current workspace/feature state, so a
    /// stale registration never grants authority after a pane or node changes.
    #[must_use]
    pub fn dispatch(
        &self,
        action: &A,
        mut enabled: impl FnMut(&HandlerId) -> bool,
        mut handle: impl FnMut(&HandlerId, &A) -> Handled,
    ) -> Handled {
        let mut candidates: Vec<_> = self
            .handlers
            .values()
            .filter(|entry| entry.handler.actions.contains(action))
            .collect();
        candidates.sort_by(|left, right| {
            (right.handler.priority, right.generation)
                .cmp(&(left.handler.priority, left.generation))
        });
        for entry in candidates {
            if enabled(&entry.handler.id) && handle(&entry.handler.id, action) == Handled::Yes {
                return Handled::Yes;
            }
        }
        Handled::No
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler(name: &str, priority: i16) -> Handler<u8> {
        Handler {
            id: name.parse().unwrap(),
            actions: vec![1],
            priority: Priority::new(priority),
        }
    }

    #[test]
    fn high_priority_then_latest_registration_with_fallthrough() {
        let mut dispatcher = Dispatcher::default();
        let _old = dispatcher.register(handler("old", 0)).unwrap();
        let _new = dispatcher.register(handler("new", 0)).unwrap();
        let _overlay = dispatcher.register(handler("overlay", 10)).unwrap();
        let mut called = Vec::new();
        assert_eq!(
            dispatcher.dispatch(
                &1,
                |_| true,
                |id, _| {
                    called.push(id.as_str().to_owned());
                    if id.as_str() == "old" {
                        Handled::Yes
                    } else {
                        Handled::No
                    }
                }
            ),
            Handled::Yes
        );
        assert_eq!(called, ["overlay", "new", "old"]);
        assert_eq!(
            dispatcher.dispatch(&2, |_| true, |_, _| Handled::Yes),
            Handled::No
        );
    }

    #[test]
    fn inactive_new_registration_does_not_remove_or_authorize_the_old_token() {
        let mut dispatcher = Dispatcher::default();
        let old = dispatcher.register(handler("workspace", 0)).unwrap();
        let new = dispatcher.register(handler("workspace", 1)).unwrap();
        dispatcher.remove(&old);
        assert_eq!(
            dispatcher.dispatch(
                &1,
                |_| false,
                |_, _| panic!("an inactive or removed handler must not run")
            ),
            Handled::No
        );
        assert_eq!(
            dispatcher.dispatch(&1, |_| true, |_, _| Handled::Yes),
            Handled::Yes
        );
        dispatcher.remove(&new);
        assert_eq!(
            dispatcher.dispatch(
                &1,
                |_| true,
                |_, _| panic!("an inactive or removed handler must not run")
            ),
            Handled::No
        );
    }
}
