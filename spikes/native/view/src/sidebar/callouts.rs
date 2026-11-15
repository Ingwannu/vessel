//! Admission and lifetime of the one sidebar callout, following
//! contexts/sidebar-callout-state.ts. Facts and payloads stay with providers;
//! no platform, node query, rendering or callback belongs in this pure store.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
/// Bound retained dismissal keys from the preference-file edge.
pub const MAX_DISMISSALS: usize = 128;
/// Provider registration is bounded even when callers fail to unregister.
pub const MAX_CALLOUTS: usize = 32;
/// A dismissal key contains identity, never content or credentials.
pub const MAX_DISMISSAL_KEY_BYTES: usize = 512;
/// Stable provider-owned dismissal identity, trimmed once at its input edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DismissalKey(Arc<str>);
/// A malformed persisted key is refused rather than interpreted as an id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a callout dismissal key must be nonempty, bounded and contain no controls")]
pub struct InvalidKey;
impl TryFrom<String> for DismissalKey {
    type Error = InvalidKey;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let key = raw.trim();
        if key.is_empty()
            || key.len() > MAX_DISMISSAL_KEY_BYTES
            || key.chars().any(char::is_control)
        {
            return Err(InvalidKey);
        }
        Ok(Self(Arc::from(key)))
    }
}
impl From<DismissalKey> for String {
    fn from(key: DismissalKey) -> Self {
        key.0.to_string()
    }
}
/// Source priority sorts descending; equal priorities keep first registration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(i32);
impl Priority {
    /// Preserve a provider's explicit source priority; absent means default0.
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self(value)
    }
}
/// Replacing a registration invalidates its prior cleanup token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token(u64);
/// The only failures refuse mutation, retaining the preceding state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// At most the named number of independent providers may be retained.
    #[error("the sidebar callout provider limit was reached")]
    ProvidersFull,
    /// Persisted dismissals are never silently discarded to admit another.
    #[error("the sidebar callout dismissal limit was reached")]
    DismissalsFull,
    /// Never wrap a token and allow stale cleanup to remove a new callout.
    #[error("the sidebar callout registration counter was exhausted")]
    TokensExhausted,
}
/// One provider's typed payload and its current registration metadata.
#[derive(Debug)]
pub struct Entry<I, C> {
    id: I,
    priority: Priority,
    token: Token,
    dismissal: Option<DismissalKey>,
    content: C,
}
impl<I, C> Entry<I, C> {
    /// The exact provider identity to use when dispatching or dismissing.
    #[must_use]
    pub const fn id(&self) -> &I {
        &self.id
    }
    /// Borrow the payload selected by priority, without cloning state per frame.
    #[must_use]
    pub const fn content(&self) -> &C {
        &self.content
    }
}
/// One process/window's admission. Persistence loads explicitly; until then,
/// callouts with durable keys stay hidden while ephemeral notices may show.
#[derive(Debug)]
pub struct State<I, C> {
    entries: Vec<Entry<I, C>>,
    dismissed: Vec<DismissalKey>,
    loaded: Load,
    next: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Load {
    Waiting,
    Ready,
}
impl<I, C> Default for State<I, C> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            dismissed: Vec::new(),
            loaded: Load::Waiting,
            next: 0,
        }
    }
}
impl<I: PartialEq, C> State<I, C> {
    /// Complete the existing preference read once, retaining each logical key.
    /// # Errors
    /// Refuses too many stored keys; no partial load or false Ready state.
    pub fn load_dismissed(&mut self, keys: &[DismissalKey]) -> Result<(), Problem> {
        if keys.len() > MAX_DISMISSALS {
            return Err(Problem::DismissalsFull);
        }
        let mut unique = Vec::new();
        for key in keys {
            if !unique.contains(key) {
                unique.push(key.clone());
            }
        }
        self.dismissed = unique;
        self.loaded = Load::Ready;
        Ok(())
    }
    /// Register/replace one provider without changing its tie-breaking order.
    /// # Errors
    /// Refuses capacity or counter exhaustion before altering any entry.
    pub fn register(
        &mut self,
        id: I,
        priority: Priority,
        dismissal: Option<DismissalKey>,
        content: C,
    ) -> Result<Token, Problem> {
        let found = self.entries.iter().position(|entry| entry.id == id);
        if found.is_none() && self.entries.len() >= MAX_CALLOUTS {
            return Err(Problem::ProvidersFull);
        }
        let next = self.next.checked_add(1).ok_or(Problem::TokensExhausted)?;
        let token = Token(next);
        let replacement = Entry {
            id,
            priority,
            token,
            dismissal,
            content,
        };
        if let Some(entry) = found.and_then(|index| self.entries.get_mut(index)) {
            *entry = replacement;
        } else {
            self.entries.push(replacement);
        }
        self.next = next;
        Ok(token)
    }
    /// Cleanup can only remove the registration that originally received it.
    pub fn unregister(&mut self, id: &I, token: Token) {
        self.entries
            .retain(|entry| &entry.id != id || entry.token != token);
    }
    /// The highest-priority admitted payload; stable order breaks ties.
    #[must_use]
    pub fn active(&self) -> Option<&Entry<I, C>> {
        let mut chosen: Option<&Entry<I, C>> = None;
        for entry in &self.entries {
            if let Some(key) = &entry.dismissal
                && (self.loaded == Load::Waiting || self.dismissed.contains(key))
            {
                continue;
            }
            if chosen.is_none_or(|prior| entry.priority > prior.priority) {
                chosen = Some(entry);
            }
        }
        chosen
    }
    /// Remove one notice and retain its key for the existing preference writer.
    /// The returned payload lets its provider run its own dismissal callback.
    /// # Errors
    /// If a durable key cannot be retained, the callout stays registered.
    pub fn dismiss(&mut self, id: &I) -> Result<Option<C>, Problem> {
        let Some(index) = self.entries.iter().position(|entry| &entry.id == id) else {
            return Ok(None);
        };
        let key = self
            .entries
            .get(index)
            .and_then(|entry| entry.dismissal.as_ref());
        if let Some(key) = key
            && !self.dismissed.contains(key)
        {
            if self.dismissed.len() >= MAX_DISMISSALS {
                return Err(Problem::DismissalsFull);
            }
            self.dismissed.push(key.clone());
        }
        Ok(Some(self.entries.remove(index).content))
    }
    /// The complete retained keys to persist after an admitted dismissal.
    #[must_use]
    pub fn dismissed(&self) -> &[DismissalKey] {
        &self.dismissed
    }
    /// Provider reset never clears dismissal preferences or reuses old tokens.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
#[cfg(test)]
mod tests;
