//! Decode retained journal collections before they can exceed domain bounds.
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::{fmt, marker::PhantomData};

pub(super) const MAX_PATCHES: usize = vessel_proto::skills::MAX_SKILLS.saturating_mul(3);
pub(super) const MAX_CHANGES: usize = super::super::bundle::MAX_SKILL_FILES
    .saturating_mul(2)
    .saturating_add(1);

pub(super) fn decode<'de, D, T, const MAX: usize>(source: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const MAX: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for Bounded<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(out, "at most {MAX} journal entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<Vec<T>, A::Error> {
            let mut retained = Vec::new();
            while let Some(item) = items.next_element()? {
                if retained.len() == MAX {
                    return Err(A::Error::custom("skill journal exceeds its entry bound"));
                }
                retained.push(item);
            }
            Ok(retained)
        }
    }
    source.deserialize_seq(Bounded::<T, MAX>(PhantomData))
}
