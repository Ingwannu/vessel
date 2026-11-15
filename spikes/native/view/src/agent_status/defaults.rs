//! Prepare a complete default change before the existing preference writer.
//!
//! [Decision Log]
//! - Purpose: selecting a long offered model must not silently discard it.
//! - Constraint: the legacy writer filters oversized choices. Its storage
//!   contract already exists in prefs::Choices; this area cannot raise it.
//! - Choice: reuse option eligibility/picks and the canonical saved-choice
//!   validator, then expose the unchanged validated slice to the writer.
//! - Alternatives: truncate identifiers, add another store, or duplicate the
//!   storage limits. All would lose intent or diverge from existing callers.
//! - Cost: a refused selection keeps previous defaults and needs a visible
//!   error. This preflight does not claim that a later disk save succeeded.

use std::collections::BTreeMap;
use vessel_proto::{OptionChoice, SessionOption};

use crate::{Picks, choices_for, prefs};

/// Refusal before preferences or a file are changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SaveProblem {
    /// A stale popup no longer identifies a value the agent offers.
    #[error("the agent no longer offers this choice")]
    NotOffered,
    /// Repeated ids cannot identify one unambiguous option.
    #[error("the agent reported conflicting option ids")]
    Conflicting,
    /// Retain the existing saved-choice validator's typed refusal.
    #[error(transparent)]
    Storage(#[from] prefs::TooMany),
}
impl SaveProblem {
    /// A rejected change belongs beside the defaults, never a success badge.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        match self {
            Self::NotOffered | Self::Conflicting | Self::Storage(_) => crate::alert::Variant::Error,
        }
    }
}

/// Only a fully representable choice list may reach the legacy writer.
#[derive(Debug, PartialEq, Eq)]
pub struct Prepared(Vec<OptionChoice>);
impl Prepared {
    /// The exact choices that were checked, without a second filtered store.
    #[must_use]
    pub fn as_slice(&self) -> &[OptionChoice] {
        &self.0
    }
}

/// Apply one current offered selection, retaining the existing rules that
/// omit default-valued or no-longer-offered saved picks.
///
/// # Errors
/// Refuses stale/ambiguous options and any complete result the existing
/// storage contract cannot keep; the caller's previous choices are borrowed.
pub fn prepare(
    options: &[SessionOption],
    previous: &[OptionChoice],
    choice: OptionChoice,
) -> Result<Prepared, SaveProblem> {
    let mut target = options.iter().filter(|option| option.id == choice.id);
    let option = target.next().ok_or(SaveProblem::NotOffered)?;
    if target.next().is_some() {
        return Err(SaveProblem::Conflicting);
    }
    if !crate::options::offered(option, &choice) {
        return Err(SaveProblem::NotOffered);
    }
    let mut picks = Picks::from_choices(previous.to_vec());
    picks.pick(choice);
    checked(choices_for(options, &picks))
}
fn checked(choices: Vec<OptionChoice>) -> Result<Prepared, SaveProblem> {
    if choices.len() > prefs::MAX_CHOICES {
        return Err(prefs::TooMany {
            max: prefs::MAX_CHOICES,
        }
        .into());
    }
    let mut map = BTreeMap::new();
    for choice in &choices {
        if map
            .insert(choice.id.clone(), choice.value.clone())
            .is_some()
        {
            return Err(SaveProblem::Conflicting);
        }
    }
    let _validated = prefs::Choices::try_from(map)?;
    Ok(Prepared(choices))
}

#[cfg(test)]
mod tests;
mod words;
