//! Decode policy and skill lists without allocating an excess item.

use std::fmt;
use std::marker::PhantomData;

use serde::de::{DeserializeSeed, Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

struct Excess<const MAX: usize>;
impl<'de, const MAX: usize> DeserializeSeed<'de> for Excess<MAX> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(D::Error::custom(format_args!("list exceeds {MAX} entries")))
    }
}

pub(crate) fn decode<'de, D, T, const MAX: usize>(input: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Items<T, const MAX: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for Items<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(out, "at most {MAX} entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            if seq.size_hint().is_some_and(|size| size > MAX) {
                return Err(A::Error::custom(format_args!("list exceeds {MAX} entries")));
            }
            let mut items = Vec::new();
            loop {
                if items.len() == MAX {
                    return match seq.next_element_seed(Excess::<MAX>)? {
                        None => Ok(items),
                        Some(()) => Err(A::Error::custom("excess entry")),
                    };
                }
                match seq.next_element()? {
                    Some(item) => items.push(item),
                    None => return Ok(items),
                }
            }
        }
    }
    input.deserialize_seq(Items::<T, MAX>(PhantomData))
}
