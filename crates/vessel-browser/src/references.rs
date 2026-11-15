//! One target's current element references, independent of pane/frame lifetime.
//!
//! Reusing @e1 after every snapshot makes an old tool argument select a new
//! element. Keep a monotonic namespace across replacements; invalidation
//! removes every prior binding before traversal starts, including on failure.

use std::collections::BTreeMap;

use vessel_proto::browser::{ElementRef, InvalidBrowserIdentity};

/// Paseo's accessibility traversal assigns at most this many current refs.
pub(crate) const MAX_REFERENCES: usize = 500;

/// A snapshot generation differs from state/frame transport revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapshotGeneration(u64);

/// A range is consumed before traversal; failed/late scripts cannot reuse its names.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SnapshotNamespace {
    start: u64,
    generation: SnapshotGeneration,
}
impl SnapshotNamespace {
    pub(crate) fn at(self, offset: u32) -> Result<ElementRef, ReferenceError> {
        if usize::try_from(offset).map_or(true, |offset| offset >= MAX_REFERENCES) {
            return Err(ReferenceError::Limit);
        }
        let ordinal = self
            .start
            .checked_add(u64::from(offset))
            .ok_or(ReferenceError::Exhausted)?;
        format!("@e{ordinal}")
            .parse()
            .map_err(ReferenceError::Encoding)
    }
    /// Serialized as a decimal string, never through a JavaScript Number.
    pub(crate) fn start_decimal(self) -> String {
        self.start.to_string()
    }
}

/// A failed reference allocation never falls back to a different element.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReferenceError {
    /// The bounded snapshot may truncate, but cannot allocate extra references.
    #[error("browser snapshot reference limit reached")]
    Limit,
    /// Close the target rather than wrap an id into an earlier namespace.
    #[error("browser reference namespace exhausted")]
    Exhausted,
    /// A ref from before invalidation, another target or a failed traversal.
    #[error("browser element reference is no longer current")]
    Stale,
    /// Retain an unexpected generated-id parser failure without losing its cause.
    #[error("browser reference could not be encoded")]
    Encoding(#[source] InvalidBrowserIdentity),
}

/// A target actor owns this map and the backend values; no global reference registry.
pub(crate) struct References<T> {
    current: BTreeMap<ElementRef, T>,
    next: u64,
    generation: SnapshotGeneration,
}
impl<T> std::fmt::Debug for References<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("References")
            .field("current_count", &self.current.len())
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}
impl<T> Default for References<T> {
    fn default() -> Self {
        Self {
            current: BTreeMap::new(),
            next: 1,
            generation: SnapshotGeneration(0),
        }
    }
}
impl<T> References<T> {
    /// Call before snapshot traversal, navigation, close or failed selection.
    ///
    /// # Errors
    /// A generation overflow requires closing the target, never reusing a generation.
    pub(crate) fn invalidate(&mut self) -> Result<SnapshotGeneration, ReferenceError> {
        self.current.clear();
        let generation = self
            .generation
            .0
            .checked_add(1)
            .ok_or(ReferenceError::Exhausted)?;
        self.generation = SnapshotGeneration(generation);
        Ok(self.generation)
    }

    /// Reserve the complete bounded namespace before dispatch, including unsuccessful snapshots.
    pub(crate) fn begin_snapshot(&mut self) -> Result<SnapshotNamespace, ReferenceError> {
        let generation = self.invalidate()?;
        let size = u64::try_from(MAX_REFERENCES).map_err(|_overflow| ReferenceError::Exhausted)?;
        let next = self
            .next
            .checked_add(size)
            .ok_or(ReferenceError::Exhausted)?;
        let namespace = SnapshotNamespace {
            start: self.next,
            generation,
        };
        self.next = next;
        Ok(namespace)
    }

    /// A result with no refs must still prove it belongs to the current traversal.
    pub(crate) fn check_namespace(
        &self,
        namespace: SnapshotNamespace,
    ) -> Result<(), ReferenceError> {
        if namespace.generation == self.generation {
            Ok(())
        } else {
            Err(ReferenceError::Stale)
        }
    }

    /// Only the decoder may retain a graph binding from its current reserved range.
    pub(crate) fn retain(
        &mut self,
        namespace: SnapshotNamespace,
        reference: ElementRef,
        value: T,
    ) -> Result<(), ReferenceError> {
        self.check_namespace(namespace)?;
        let ordinal = reference
            .as_str()
            .strip_prefix("@e")
            .and_then(|digits| digits.parse::<u64>().ok())
            .ok_or(ReferenceError::Stale)?;
        let offset = ordinal
            .checked_sub(namespace.start)
            .and_then(|offset| u32::try_from(offset).ok())
            .ok_or(ReferenceError::Stale)?;
        if namespace.at(offset)? != reference || self.current.contains_key(&reference) {
            return Err(ReferenceError::Stale);
        }
        if self.current.len() >= MAX_REFERENCES {
            return Err(ReferenceError::Limit);
        }
        let _absent = self.current.insert(reference, value);
        Ok(())
    }

    /// Retain one real backend element from the current traversal.
    ///
    /// # Errors
    /// Refuses the current snapshot's cap or exhausted namespace without inserting.
    pub(crate) fn assign(&mut self, value: T) -> Result<ElementRef, ReferenceError> {
        if self.current.len() >= MAX_REFERENCES {
            return Err(ReferenceError::Limit);
        }
        let next = self.next.checked_add(1).ok_or(ReferenceError::Exhausted)?;
        let id: ElementRef = format!("@e{}", self.next)
            .parse()
            .map_err(ReferenceError::Encoding)?;
        self.next = next;
        let _old = self.current.insert(id.clone(), value);
        Ok(id)
    }

    /// Only current refs resolve; callers must also recheck target ownership/actionability.
    ///
    /// # Errors
    /// A previously valid argument is explicitly stale, never retried against a new element.
    pub(crate) fn resolve(&self, reference: &ElementRef) -> Result<&T, ReferenceError> {
        self.current.get(reference).ok_or(ReferenceError::Stale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_never_aliases_an_old_ref_and_failed_traversal_stays_empty() {
        let mut refs = References::default();
        let _generation = refs.invalidate().unwrap();
        let old = refs.assign("credential field").unwrap();
        assert_eq!(refs.resolve(&old).unwrap(), &"credential field");
        let _generation = refs.invalidate().unwrap();
        assert!(matches!(refs.resolve(&old), Err(ReferenceError::Stale)));
        let current = refs.assign("delete button").unwrap();
        assert_ne!(old, current);
        assert!(refs.resolve(&old).is_err());
        let _generation = refs.invalidate().unwrap();
        assert!(refs.resolve(&current).is_err());
        assert!(!format!("{refs:?}").contains("delete button"));
    }

    #[test]
    fn reserved_ranges_preserve_large_integer_names_and_exhaust_before_wrap() {
        let mut refs = References::<()> {
            next: 9_007_199_254_740_993,
            ..References::default()
        };
        let namespace = refs.begin_snapshot().unwrap();
        assert_eq!(namespace.at(0).unwrap().as_str(), "@e9007199254740993");
        assert_eq!(namespace.at(499).unwrap().as_str(), "@e9007199254741492");
        let config = crate::snapshot::Config::new(namespace);
        let value = serde_json::to_value(config).unwrap();
        assert_eq!(value["firstRef"], "9007199254740993");
        let next = refs.begin_snapshot().unwrap();
        assert_eq!(next.at(0).unwrap().as_str(), "@e9007199254741493");
        assert!(refs.check_namespace(namespace).is_err());
        refs.next = u64::MAX;
        assert!(matches!(
            refs.begin_snapshot(),
            Err(ReferenceError::Exhausted)
        ));
    }

    #[test]
    fn cap_and_overflow_do_not_wrap_or_retain_an_excess_binding() {
        let mut refs = References::default();
        for _ in 0..MAX_REFERENCES {
            let _id = refs.assign(()).unwrap();
        }
        assert!(matches!(refs.assign(()), Err(ReferenceError::Limit)));
        let _generation = refs.invalidate().unwrap();
        refs.next = u64::MAX;
        assert!(matches!(refs.assign(()), Err(ReferenceError::Exhausted)));
        assert!(refs.current.is_empty());
    }
}
