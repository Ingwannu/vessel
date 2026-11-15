//! Collection caps apply before decoding the first excess item, not after allocation.

use serde::de::{self, DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::marker::PhantomData;

use super::result::{ConsoleEntry, NetworkEntry};
use super::state::BrowserTab;

/// One engine never owns an unbounded target/profile inventory.
pub const MAX_BROWSER_TABS: u32 = 32;
/// Match Paseo's maximum retained/requested log count per ring.
pub const MAX_BROWSER_LOGS: u32 = super::MAX_LOG_ENTRIES;

/// A constructor and the serde edge share each collection's cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidBrowserList {
    /// Never drop other workspace records merely to fit a partial inventory.
    #[error("browser inventory exceeds {MAX_BROWSER_TABS} targets")]
    Tabs,
    /// Console entries must remain within the retained ring.
    #[error("browser console tail exceeds {MAX_BROWSER_LOGS} entries")]
    Console,
    /// Network entries have the same bounded retention contract.
    #[error("browser network tail exceeds {MAX_BROWSER_LOGS} entries")]
    Network,
}

macro_rules! limited_list {
    ($name:ident,$item:ty,$max:ident,$reason:ident,$doc:literal) => {
        #[doc=$doc]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(transparent)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        pub struct $name(Vec<$item>);
        impl $name {
            /// Borrow the complete bounded collection; no silent partial result.
            #[must_use]
            pub fn as_slice(&self) -> &[$item] {
                &self.0
            }
        }
        impl TryFrom<Vec<$item>> for $name {
            type Error = InvalidBrowserList;
            fn try_from(items: Vec<$item>) -> Result<Self, Self::Error> {
                if u32::try_from(items.len()).map_or(true, |length| length > $max) {
                    Err(InvalidBrowserList::$reason)
                } else {
                    Ok(Self(items))
                }
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                decode::<D, $item, $max>(deserializer).map(Self)
            }
        }
    };
}
limited_list!(
    BrowserTabs,
    BrowserTab,
    MAX_BROWSER_TABS,
    Tabs,
    "A complete workspace-filtered inventory, never a silently truncated one."
);
limited_list!(
    ConsoleEntries,
    ConsoleEntry,
    MAX_BROWSER_LOGS,
    Console,
    "A console tail that fits the retained ring before it is decoded."
);
limited_list!(
    NetworkEntries,
    NetworkEntry,
    MAX_BROWSER_LOGS,
    Network,
    "A network tail with no arbitrary extra response bodies or headers."
);

fn decode<'de, D, T, const MAX: u32>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Sequence<T, const MAX: u32>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const MAX: u32> Visitor<'de> for Sequence<T, MAX> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "at most {MAX} browser entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut source: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            let limit = usize::try_from(MAX).map_err(de::Error::custom)?;
            while values.len() < limit {
                match source.next_element()? {
                    Some(value) => values.push(value),
                    None => return Ok(values),
                }
            }
            if source.next_element_seed(Reject::<MAX>)?.is_some() {
                return Err(de::Error::custom("browser entry limit exceeded"));
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Sequence::<T, MAX>(PhantomData))
}
struct Reject<const MAX: u32>;
impl<'de, const MAX: u32> DeserializeSeed<'de> for Reject<MAX> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _deserializer: D) -> Result<(), D::Error> {
        Err(de::Error::custom(format_args!(
            "browser collection exceeds {MAX} entries"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excess_item_is_refused_before_its_payload_is_decoded() {
        // The first excess item is deliberately not a console entry. The cap,
        // rather than that item's schema, must be the first rejected boundary.
        let limit = usize::try_from(MAX_BROWSER_LOGS).unwrap();
        let mut items = vec![r#"{"level":"log","text":""}"#; limit];
        items.push(r#"{"untrusted_payload":"do not decode"}"#);
        let input = format!("[{}]", items.join(","));
        let error = serde_json::from_str::<ConsoleEntries>(&input).unwrap_err();
        assert!(error.to_string().contains("collection exceeds"));
        assert!(serde_json::from_str::<ConsoleEntries>("[]").is_ok());
    }
}
