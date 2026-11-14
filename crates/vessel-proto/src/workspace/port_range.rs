//! Inclusive service port ranges with private, ordered, validated endpoints.
//! Their canonical text is what the project approval will cover.

use std::fmt;
use std::str::FromStr;

use super::{InvalidServicePort, ServicePort};

/// An inclusive service port range whose order cannot be bypassed by serde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PortRange {
    start: ServicePort,
    end: ServicePort,
}

/// Why a range cannot be used for allocation or shown as an approved range.
#[derive(Debug, thiserror::Error)]
pub enum InvalidPortRange {
    /// Match the project's schema rather than accepting a more permissive number grammar.
    #[error("a port range is `start-end`, with 1 to 5 ASCII digits at each end")]
    Format,
    /// Preserve the endpoint's numeric error instead of turning it into text.
    #[error("a port range endpoint is invalid: {0}")]
    Endpoint(#[source] InvalidServicePort),
    /// An inclusive range cannot run backwards.
    #[error("a port range's start must not exceed its end")]
    Reversed,
}

impl PortRange {
    /// Keep order checked even when endpoints came from another validated source.
    ///
    /// # Errors
    /// [`InvalidPortRange::Reversed`] when `start` exceeds `end`.
    pub fn new(start: ServicePort, end: ServicePort) -> Result<Self, InvalidPortRange> {
        if start > end {
            return Err(InvalidPortRange::Reversed);
        }
        Ok(Self { start, end })
    }

    /// The first candidate; included in allocation.
    #[must_use]
    pub const fn start(self) -> ServicePort {
        self.start
    }

    /// The last candidate; included in allocation.
    #[must_use]
    pub const fn end(self) -> ServicePort {
        self.end
    }
}

impl FromStr for PortRange {
    type Err = InvalidPortRange;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // Paseo trims the whole string as JavaScript does, not either endpoint.
        let text = text.trim_matches(|character: char| {
            (character.is_whitespace() && character != '\u{0085}') || character == '\u{feff}'
        });
        let (start, end) = text.split_once('-').ok_or(InvalidPortRange::Format)?;
        if !endpoint_shape(start) || !endpoint_shape(end) {
            return Err(InvalidPortRange::Format);
        }
        let start = start.parse().map_err(InvalidPortRange::Endpoint)?;
        let end = end.parse().map_err(InvalidPortRange::Endpoint)?;
        Self::new(start, end)
    }
}

fn endpoint_shape(text: &str) -> bool {
    (1..=5).contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit())
}

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

crate::key::serde_via_text!(PortRange);

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn ranges_are_inclusive_and_canonical_without_losing_the_bounds() {
        let range: PortRange = "\u{feff} 00001-65535\n".parse().unwrap();
        assert_eq!(range.start().get(), 1);
        assert_eq!(range.end().get(), 65_535);
        assert_eq!(range.to_string(), "1-65535");
        assert_eq!(serde_json::to_string(&range).unwrap(), "\"1-65535\"");
        assert_eq!(
            serde_json::from_str::<PortRange>("\"1-65535\"").unwrap(),
            range
        );
        assert_eq!("7-7".parse::<PortRange>().unwrap().to_string(), "7-7");
    }

    #[test]
    fn text_and_wire_cannot_bypass_range_validation() {
        for text in [
            "",
            "0-1",
            "1-65536",
            "2-1",
            "1 -2",
            "1- 2",
            "+1-2",
            "1-2-3",
            "1.0-2",
            "000001-2",
            "１-2",
            "\u{0085}1-2",
            "1-2\u{0085}",
        ] {
            assert!(text.parse::<PortRange>().is_err(), "{text:?}");
            assert!(
                serde_json::from_str::<PortRange>(&serde_json::to_string(text).unwrap()).is_err()
            );
        }
        let one = ServicePort::try_from(1_u16).unwrap();
        let two = ServicePort::try_from(2_u16).unwrap();
        assert!(matches!(
            PortRange::new(two, one),
            Err(InvalidPortRange::Reversed)
        ));
        assert!(serde_json::from_str::<PortRange>(r#"{"start":2,"end":1}"#).is_err());
    }

    #[test]
    fn an_invalid_endpoint_retains_the_numeric_source_chain() {
        let error = "1-65536".parse::<PortRange>().unwrap_err();
        let endpoint = error.source().unwrap();
        assert!(endpoint.is::<InvalidServicePort>());
        assert!(endpoint.source().unwrap().is::<std::num::ParseIntError>());
    }
}
