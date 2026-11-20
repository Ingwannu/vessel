//! Browser tab ids and element refs, parsed once at the edge.
//!
//! Paseo accepts a UUID version 4 or `<13 or more digits>-<hex>`
//! (`rpc-schemas.ts`, `BROWSER_AUTOMATION_BROWSER_ID_PATTERN`, the `i`
//! flag). An element ref is `@e` and a decimal (`^@e\d+$`).

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use super::InvalidInput;

/// A tab id. The test double mints a UUID version 4. A build with no
/// engine mints nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct BrowserId {
    kind: IdKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum IdKind {
    Uuid(Uuid),
    Stamp { digits: String, hex: String },
}

impl BrowserId {
    /// A new UUID version 4. Only a real engine, or the test double, calls
    /// this. The product absence does not.
    #[must_use]
    pub fn random() -> Self {
        Self {
            kind: IdKind::Uuid(Uuid::new_v4()),
        }
    }
}

impl fmt::Display for BrowserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            IdKind::Uuid(id) => write!(f, "{id}"),
            IdKind::Stamp { digits, hex } => write!(f, "{digits}-{hex}"),
        }
    }
}

impl FromStr for BrowserId {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if let Ok(id) = Uuid::parse_str(text)
            && id.get_version() == Some(uuid::Version::Random)
        {
            return Ok(Self {
                kind: IdKind::Uuid(id),
            });
        }
        let Some((digits, hex)) = text.split_once('-') else {
            return Err(InvalidInput::BrowserId);
        };
        if hex.contains('-')
            || digits.len() < 13
            || !digits.chars().all(|c| c.is_ascii_digit())
            || hex.is_empty()
            || !hex.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(InvalidInput::BrowserId);
        }
        Ok(Self {
            kind: IdKind::Stamp {
                digits: digits.to_owned(),
                hex: hex.to_ascii_lowercase(),
            },
        })
    }
}

crate::key::serde_via_text!(BrowserId);

/// An element ref from the latest snapshot: `@e` and a decimal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct ElementRef(String);

impl ElementRef {
    /// `@e` and `n`, including `@e0`.
    #[must_use]
    pub fn numbered(n: u32) -> Self {
        Self(format!("@e{n}"))
    }
}

impl fmt::Display for ElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ElementRef {
    type Err = InvalidInput;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let Some(digits) = text.strip_prefix("@e") else {
            return Err(InvalidInput::ElementRef);
        };
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(InvalidInput::ElementRef);
        }
        Ok(Self(text.to_owned()))
    }
}

crate::key::serde_via_text!(ElementRef);
