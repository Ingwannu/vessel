//! Launch-domain decoding and counting serialization; no file/network I/O.

use std::{fmt, io, marker::PhantomData};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, DeserializeSeed, SeqAccess, Visitor},
};

use super::InvalidLaunchValue;

/// Byte budget for a complete launch request/reply body, after JSON escaping.
#[derive(Debug)]
pub struct LaunchPayload;

impl LaunchPayload {
    /// One bound for the complete canonical JSON body, not the RPC envelope.
    pub const MAX_BYTES: usize = 262_144;

    pub(super) fn check(value: &impl Serialize) -> Result<(), InvalidLaunchValue> {
        let mut count = Count {
            used: 0,
            exceeded: false,
        };
        match serde_json::to_writer(&mut count, value) {
            Ok(()) => Ok(()),
            Err(_) if count.exceeded => Err(InvalidLaunchValue::Payload),
            Err(error) => Err(InvalidLaunchValue::Encoding(error)),
        }
    }
}

struct Count {
    used: usize,
    exceeded: bool,
}
impl io::Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Overflow also means too large; never wrap the admission counter.
        match self.used.checked_add(bytes.len()) {
            Some(next) if next <= LaunchPayload::MAX_BYTES => {
                self.used = next;
                Ok(bytes.len())
            }
            Some(_) | None => {
                self.exceeded = true;
                Err(io::Error::other("launch payload limit"))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Decode strict DTOs without returning their untrusted field names/text.
pub(super) fn decode<'de, D, T>(input: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(input).map_err(|_untrusted| de::Error::custom(InvalidLaunchValue::Malformed))
}

// A missing optional field clears it. A present field must contain its
// actual type; null must not silently introduce another reset spelling.
pub(super) fn optional<'de, D, T>(input: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(input).map(Some)
}

struct Reject;
impl<'de> DeserializeSeed<'de> for Reject {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(de::Error::custom(InvalidLaunchValue::Malformed))
    }
}

// Reject's seed never constructs or scans an extra domain item. The outer
// type maps this generic decoder failure to its own structural rule.
pub(super) fn bounded<'de, D, T, const MAX: usize>(input: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Rows<T, const MAX: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for Rows<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {MAX} launch items")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            if seq.size_hint().is_some_and(|n| n > MAX) {
                return Err(de::Error::custom(InvalidLaunchValue::Malformed));
            }
            let mut rows = Vec::new();
            while rows.len() < MAX {
                let Some(row) = seq.next_element()? else {
                    return Ok(rows);
                };
                rows.push(row);
            }
            if seq.next_element_seed(Reject)?.is_some() {
                return Err(de::Error::custom(InvalidLaunchValue::Malformed));
            }
            Ok(rows)
        }
    }
    input
        .deserialize_seq(Rows::<T, MAX>(PhantomData))
        .map_err(|_untrusted| de::Error::custom(InvalidLaunchValue::Malformed))
}
