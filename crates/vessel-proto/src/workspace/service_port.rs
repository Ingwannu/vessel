//! Validated declared or allocated service ports; not observed listeners.
//! No decoder can turn port zero into an instruction to allocate a port.

use std::fmt;
use std::num::{NonZeroU16, ParseIntError, TryFromIntError};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A service's configured or allocated TCP port, never the ephemeral sentinel zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "number"))]
#[serde(try_from = "u16", into = "u16")]
pub struct ServicePort(NonZeroU16);

/// Why a declaration or allocator output cannot identify a service port.
#[derive(Debug, thiserror::Error)]
pub enum InvalidServicePort {
    /// Zero asks the operating system to allocate; it is not an allocated port.
    #[error("a service port is between {min} and {max}, not zero", min = ServicePort::MIN, max = ServicePort::MAX)]
    Zero,
    /// Allocator output accepts decimal digits only, without a sign or a fraction.
    #[error("a service port must contain only ASCII decimal digits")]
    Digits,
    /// The integer parser's cause is retained, including overflow.
    #[error("a service port must fit between {min} and {max}", min = ServicePort::MIN, max = ServicePort::MAX)]
    Number(#[source] ParseIntError),
    /// A file's signed integer is outside the TCP port representation.
    #[error("a service port must fit between {min} and {max}", min = ServicePort::MIN, max = ServicePort::MAX)]
    Range(#[source] TryFromIntError),
}

impl ServicePort {
    /// The first nonzero TCP port, shared by validation and refusal messages.
    pub const MIN: Self = Self(NonZeroU16::MIN);
    /// The largest TCP port, tied to the representation's bound.
    pub const MAX: Self = Self(NonZeroU16::MAX);

    /// The port number for a socket API, after validation at the input edge.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

impl TryFrom<u16> for ServicePort {
    type Error = InvalidServicePort;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        NonZeroU16::new(value)
            .map(Self)
            .ok_or(InvalidServicePort::Zero)
    }
}

impl TryFrom<i64> for ServicePort {
    type Error = InvalidServicePort;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::try_from(u16::try_from(value).map_err(InvalidServicePort::Range)?)
    }
}

impl From<ServicePort> for u16 {
    fn from(port: ServicePort) -> Self {
        port.get()
    }
}

impl FromStr for ServicePort {
    type Err = InvalidServicePort;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(InvalidServicePort::Digits);
        }
        Self::try_from(text.parse::<u16>().map_err(InvalidServicePort::Number)?)
    }
}

impl fmt::Display for ServicePort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn construction_and_wire_decode_enforce_the_same_tcp_bounds() {
        for boundary in [ServicePort::MIN, ServicePort::MAX] {
            let number = boundary.get();
            let port = ServicePort::try_from(number).unwrap();
            assert_eq!(port.get(), number);
            let json = serde_json::to_string(&port).unwrap();
            assert_eq!(json, number.to_string());
            assert_eq!(serde_json::from_str::<ServicePort>(&json).unwrap(), port);
        }
        let below = ServicePort::MIN
            .get()
            .checked_sub(ServicePort::MIN.get())
            .unwrap();
        let above = i64::from(ServicePort::MAX.get()) + i64::from(ServicePort::MIN.get());
        assert!(ServicePort::try_from(below).is_err());
        for invalid in [
            below.to_string(),
            above.to_string(),
            "-7".into(),
            "7.7".into(),
            "\"7\"".into(),
        ] {
            assert!(serde_json::from_str::<ServicePort>(&invalid).is_err());
        }
    }

    #[test]
    fn allocator_text_is_decimal_and_canonical_without_a_leading_sign() {
        let padded = format!("000000{}", ServicePort::MIN);
        assert_eq!(padded.parse::<ServicePort>().unwrap(), ServicePort::MIN);
        for invalid in ["", "+7", "-7", "7.0", "７", " 7", "7 "] {
            assert!(invalid.parse::<ServicePort>().is_err(), "{invalid:?}");
        }
        let below = ServicePort::MIN
            .get()
            .checked_sub(ServicePort::MIN.get())
            .unwrap();
        let above = i64::from(ServicePort::MAX.get()) + i64::from(ServicePort::MIN.get());
        for invalid in [below.to_string(), above.to_string()] {
            assert!(invalid.parse::<ServicePort>().is_err());
        }
    }

    #[test]
    fn numeric_conversion_errors_keep_their_causes() {
        let above = i64::from(ServicePort::MAX.get()) + i64::from(ServicePort::MIN.get());
        for number in [-i64::from(ServicePort::MIN.get()), above] {
            let error = ServicePort::try_from(number).unwrap_err();
            assert!(error.source().unwrap().is::<TryFromIntError>());
        }
        let error = above.to_string().parse::<ServicePort>().unwrap_err();
        assert!(error.source().unwrap().is::<ParseIntError>());
    }
}
