//! Source ownership for the existing command center. Targets remain generic so
//! plugins and feature areas carry their own validated action types; this
//! registry never interprets an RPC name, filesystem path or node identity.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

/// Bounds source-controlled labels and ids before they enter a retained list.
const MAX_ID_BYTES: usize = 256;

macro_rules! identifier {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// The validated source spelling, preserved for stable identity.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = RegistrationError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if text.is_empty()
                    || text.len() > MAX_ID_BYTES
                    || text.chars().any(char::is_control)
                {
                    return Err(RegistrationError::InvalidId);
                }
                Ok(Self(text.to_owned()))
            }
        }
    };
}

identifier!(
    SourceId,
    "The feature or plugin that owns a command contribution set."
);
identifier!(
    ContributionId,
    "An entry id, unique within its source rather than globally."
);
identifier!(
    GroupId,
    "A contribution section shared by rows from one or more sources."
);

/// Identity has separate source and local components, so colons cannot alias.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultId {
    /// The registering feature.
    pub source: SourceId,
    /// Its local entry.
    pub entry: ContributionId,
}

/// Section and row ordering use integers; tenths express Paseo's 1.1 groups
/// without float ordering or rounding across a boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Rank(i16);

impl Rank {
    /// An exact ordinal; negative workspace ranks precede root actions.
    #[must_use]
    pub const fn new(ordinal: i16) -> Self {
        Self(ordinal)
    }
}

/// Query-only choices must not fill the initial command center listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Also shown with an empty query.
    Always,
    /// Shown only after searching.
    Query,
}

/// A choice keeps a nonempty breadcrumb path; it cannot produce a blank row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoicePath {
    first: String,
    rest: Vec<String>,
}

impl ChoicePath {
    /// Build a path at a feature's translation boundary.
    ///
    /// # Errors
    /// Refuses empty or control-only path labels instead of silently dropping them.
    pub fn new(first: String, rest: Vec<String>) -> Result<Self, RegistrationError> {
        if std::iter::once(&first)
            .chain(&rest)
            .any(|label| label.trim().is_empty())
        {
            return Err(RegistrationError::EmptyLabel);
        }
        Ok(Self { first, rest })
    }

    /// Visible segments, also used as independent search fields.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.first.as_str()).chain(self.rest.iter().map(String::as_str))
    }
}

/// The same presentation model serves built-in and plugin contributions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presentation {
    /// An immediate action with an optional second line.
    Action {
        /// Already translated by the owning feature.
        title: String,
        /// A user-visible explanation, also searchable.
        subtitle: Option<String>,
    },
    /// A setting choice; paths can be navigated as nested pages.
    Choice {
        /// Nonempty, translated breadcrumb labels.
        path: ChoicePath,
        /// Whether this is the current value.
        selection: Selection,
    },
}

/// Kept separate from action availability: a checked choice still runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Current setting or assigned label.
    Selected,
    /// Another available setting or label.
    Unselected,
}

/// Declarative contribution; running its typed target remains the owner's job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contribution<T> {
    /// Unique within a source.
    pub id: ContributionId,
    /// Stable section key.
    pub group: GroupId,
    /// Section order, before per-row rank.
    pub group_rank: Rank,
    /// Optional section heading, prepared by its owner.
    pub section_title: Option<String>,
    /// Row order, retained on equal search scores.
    pub rank: Rank,
    /// Hidden search synonyms; never used as a displayed label.
    pub keywords: Vec<String>,
    /// Whether a query is required.
    pub visibility: Visibility,
    /// Action or breadcrumb choice.
    pub presentation: Presentation,
    /// Validated feature action, rechecked by the owner on dispatch.
    pub target: T,
}

/// A stale cleanup cannot remove a newer source registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    source: SourceId,
    generation: u64,
}

/// Registration failures leave the prior source snapshot intact.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistrationError {
    /// Ids are nonempty, bounded and contain no control characters.
    #[error("invalid command contribution identity")]
    InvalidId,
    /// A row cannot render or search without visible text.
    #[error("command contribution label is empty")]
    EmptyLabel,
    /// The source supplied the same local id more than once.
    #[error("duplicate command contribution id")]
    DuplicateId,
    /// Ownership generations cannot safely wrap to a stale token.
    #[error("command contribution generation exhausted")]
    GenerationExhausted,
}

#[derive(Debug, Clone)]
struct Registration<T> {
    generation: u64,
    contributions: Vec<Contribution<T>>,
}

/// Retained extension of the palette's entries, with atomic source replacement.
#[derive(Debug, Clone)]
pub struct Registry<T> {
    next_generation: u64,
    sources: BTreeMap<SourceId, Registration<T>>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            next_generation: 0,
            sources: BTreeMap::new(),
        }
    }
}

impl<T> Registry<T> {
    /// Capture the exact registration generation when presenting a result.
    #[must_use]
    pub fn owner_of(&self, id: &ResultId) -> Option<Owner> {
        let registration = self.sources.get(&id.source)?;
        registration
            .contributions
            .iter()
            .find(|entry| entry.id == id.entry)?;
        Some(Owner {
            source: id.source.clone(),
            generation: registration.generation,
        })
    }

    /// Revalidate a captured row before dispatch. Replacement or removal makes
    /// the old result unavailable rather than invoking a different contribution.
    #[must_use]
    pub fn resolve(&self, id: &ResultId, owner: &Owner) -> Option<&Contribution<T>> {
        let registration = self.sources.get(&id.source)?;
        if owner.source != id.source || owner.generation != registration.generation {
            return None;
        }
        registration
            .contributions
            .iter()
            .find(|entry| entry.id == id.entry)
    }

    /// Replace one source atomically and return its cleanup token.
    ///
    /// # Errors
    /// Duplicate ids, empty action titles and generation exhaustion are refused
    /// before replacing the current snapshot.
    pub fn replace(
        &mut self,
        source: SourceId,
        contributions: Vec<Contribution<T>>,
    ) -> Result<Owner, RegistrationError> {
        let mut ids = BTreeSet::new();
        for contribution in &contributions {
            if !ids.insert(&contribution.id) {
                return Err(RegistrationError::DuplicateId);
            }
            if let Presentation::Action { title, .. } = &contribution.presentation
                && title.trim().is_empty()
            {
                return Err(RegistrationError::EmptyLabel);
            }
        }
        let generation = self
            .next_generation
            .checked_add(1)
            .ok_or(RegistrationError::GenerationExhausted)?;
        self.next_generation = generation;
        drop(self.sources.insert(
            source.clone(),
            Registration {
                generation,
                contributions,
            },
        ));
        Ok(Owner { source, generation })
    }

    /// Remove only the registration represented by this token.
    pub fn remove(&mut self, owner: &Owner) {
        if self
            .sources
            .get(&owner.source)
            .is_some_and(|entry| entry.generation == owner.generation)
        {
            drop(self.sources.remove(&owner.source));
        }
    }

    /// Stable registry order shared by projection and keyboard selection.
    #[must_use]
    pub fn snapshot(&self) -> Vec<(ResultId, &Contribution<T>)> {
        let mut rows: Vec<_> = self
            .sources
            .iter()
            .flat_map(|(source, registration)| {
                registration.contributions.iter().map(|entry| {
                    (
                        ResultId {
                            source: source.clone(),
                            entry: entry.id.clone(),
                        },
                        entry,
                    )
                })
            })
            .collect();
        rows.sort_by(|(left_id, left), (right_id, right)| {
            (left.group_rank, &left.group, left.rank, left_id).cmp(&(
                right.group_rank,
                &right.group,
                right.rank,
                right_id,
            ))
        });
        rows
    }
}
