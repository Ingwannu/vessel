//! A build's label (`0.0.0.0.1`), and the order in which labels follow
//! one another.
//!
//! The owner numbers the builds they test as `0.0.0.0.1`, `0.0.0.0.2`,
//! ...: five numbers, not semver's three, and not the crate's version
//! (which stays `0.0.0` while the app is a spike). So a label is its own
//! type: dot-separated decimal numbers, compared number by number, where
//! a missing number counts as zero (`0.1` and `0.1.0` are the same
//! build). Semver was rejected because it cannot hold five numbers, and
//! because a label is never a pre-release or carries build metadata.
//!
//! Must stay true: parsing is the only way to make one, and a label
//! never compares by its text (`0.0.0.0.10` follows `0.0.0.0.9`).

use std::{cmp::Ordering, fmt, str::FromStr};

/// The most numbers a label holds; the owner's labels use five.
const MAX_PARTS: usize = 8;

/// The longest label text accepted, so a hostile feed cannot make one
/// that is costly to keep or show.
const MAX_LEN: usize = 64;

/// A build's label: one to eight dot-separated decimal numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Label {
    parts: Vec<u32>,
}

/// Why a text is not a label.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LabelError {
    /// Empty, longer than 64 characters, or more than eight numbers.
    #[error("a label is one to eight numbers in at most 64 characters")]
    Length,
    /// A part is empty, not decimal digits, has a leading zero or does not
    /// fit in 32 bits.
    #[error("a label's parts are decimal numbers without leading zeros")]
    Part,
}

impl Label {
    /// The numbers, most significant first.
    #[must_use]
    pub fn parts(&self) -> &[u32] {
        &self.parts
    }

    /// The number at `index`, zero past the end (see the module's note on
    /// missing numbers).
    fn part(&self, index: usize) -> u32 {
        self.parts.get(index).copied().unwrap_or(0)
    }
}

impl FromStr for Label {
    type Err = LabelError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() || text.len() > MAX_LEN {
            return Err(LabelError::Length);
        }
        let mut parts = Vec::new();
        for piece in text.split('.') {
            if parts.len() == MAX_PARTS {
                return Err(LabelError::Length);
            }
            let digits_only = !piece.is_empty() && piece.bytes().all(|byte| byte.is_ascii_digit());
            let leading_zero = piece.len() > 1 && piece.starts_with('0');
            if !digits_only || leading_zero {
                return Err(LabelError::Part);
            }
            let number = piece.parse::<u32>().map_err(|_overflow| LabelError::Part)?;
            parts.push(number);
        }
        Ok(Self { parts })
    }
}

impl TryFrom<String> for Label {
    type Error = LabelError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Label> for String {
    fn from(label: Label) -> Self {
        label.to_string()
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for part in &self.parts {
            if !first {
                f.write_str(".")?;
            }
            first = false;
            write!(f, "{part}")?;
        }
        Ok(())
    }
}

impl Ord for Label {
    fn cmp(&self, other: &Self) -> Ordering {
        let longest = self.parts.len().max(other.parts.len());
        (0..longest)
            .map(|index| self.part(index).cmp(&other.part(index)))
            .find(|ordering| ordering.is_ne())
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Label {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(text: &str) -> Label {
        text.parse().unwrap()
    }

    #[test]
    fn the_owners_labels_follow_one_another() {
        assert!(label("0.0.0.0.2") > label("0.0.0.0.1"));
        assert!(label("0.0.0.0.10") > label("0.0.0.0.9"));
        assert!(label("0.0.0.1.0") > label("0.0.0.0.99"));
        assert_eq!(label("0.0.0.0.1").to_string(), "0.0.0.0.1");
    }

    #[test]
    fn a_missing_number_is_zero() {
        assert_eq!(label("0.1").cmp(&label("0.1.0")), Ordering::Equal);
        assert!(label("0.1.0.1") > label("0.1"));
    }

    #[test]
    fn what_is_not_a_label() {
        for text in [
            "",
            ".",
            "1.",
            ".1",
            "1..2",
            "01",
            "1.02",
            "a",
            "1.-2",
            "1 .2",
            "4294967296",
        ] {
            assert!(text.parse::<Label>().is_err(), "{text:?} was accepted");
        }
        assert_eq!(
            "1.2.3.4.5.6.7.8.9".parse::<Label>(),
            Err(LabelError::Length)
        );
        assert_eq!("1".repeat(65).parse::<Label>(), Err(LabelError::Length));
        assert!("0".parse::<Label>().is_ok());
        assert!("4294967295".parse::<Label>().is_ok());
    }
}
