//! Normalized versions never retain a suffix that could contain private text.
use super::InvalidDiagnostic;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// Input bytes inspected when normalizing an observed version.
pub const MAX_DIAGNOSTIC_VERSION_BYTES: usize = 64;
/// Number of numeric semantic-version components.
pub const VERSION_COMPONENTS: usize = 3;
/// Minimum digits in a numeric version component.
pub const MIN_DIAGNOSTIC_VERSION_DIGITS: usize = 1;
/// Largest numeric component, also used in validation errors.
pub const MAX_DIAGNOSTIC_VERSION_COMPONENT: u32 = u32::MAX;

// Decimal syntax belongs here, not in a caller that interprets raw output.
const DECIMAL_RADIX: u32 = 10;

/// Numeric version information; arbitrary prerelease/build words are absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(deny_unknown_fields)]
pub struct DiagnosticVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
    /// A prerelease suffix existed; its words were discarded.
    pub prerelease: bool,
    /// Build metadata existed; its words were discarded.
    pub build_metadata: bool,
}
impl FromStr for DiagnosticVersion {
    type Err = InvalidDiagnostic;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.is_empty() || text.len() > MAX_DIAGNOSTIC_VERSION_BYTES {
            return Err(InvalidDiagnostic::Version);
        }
        let (without_build, build_metadata) = match text.split_once('+') {
            Some((core, suffix)) if suffix_valid(suffix, Suffix::Build) => (core, true),
            Some(_) => return Err(InvalidDiagnostic::Version),
            None => (text, false),
        };
        let (core, prerelease) = match without_build.split_once('-') {
            Some((core, suffix)) if suffix_valid(suffix, Suffix::Prerelease) => (core, true),
            Some(_) => return Err(InvalidDiagnostic::Version),
            None => (without_build, false),
        };
        let mut fields = core.split('.');
        // The named semantic fields consume exactly VERSION_COMPONENTS;
        // count validation precedes parsing without allocating components.
        if core.split('.').count() != VERSION_COMPONENTS {
            return Err(InvalidDiagnostic::Version);
        }
        let major = component(fields.next())?;
        let minor = component(fields.next())?;
        let patch = component(fields.next())?;
        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
            build_metadata,
        })
    }
}
fn component(text: Option<&str>) -> Result<u32, InvalidDiagnostic> {
    let text = text.ok_or(InvalidDiagnostic::Version)?;
    if !numeric(text) {
        return Err(InvalidDiagnostic::Version);
    }
    // Check the next decimal digit against the named ceiling before
    // multiplication. No intermediate parser error (or input text) exists
    // to be discarded; a bound violation is this type's own rule.
    let quotient = MAX_DIAGNOSTIC_VERSION_COMPONENT
        .checked_div(DECIMAL_RADIX)
        .ok_or(InvalidDiagnostic::Version)?;
    let remainder = MAX_DIAGNOSTIC_VERSION_COMPONENT
        .checked_rem(DECIMAL_RADIX)
        .ok_or(InvalidDiagnostic::Version)?;
    text.bytes().try_fold(0_u32, |value, byte| {
        let digit = u32::from(byte.saturating_sub(b'0'));
        if value > quotient || (value == quotient && digit > remainder) {
            return Err(InvalidDiagnostic::Version);
        }
        value
            .checked_mul(DECIMAL_RADIX)
            .and_then(|value| value.checked_add(digit))
            .ok_or(InvalidDiagnostic::Version)
    })
}
fn numeric(text: &str) -> bool {
    text.len() >= MIN_DIAGNOSTIC_VERSION_DIGITS
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && (text.len() == MIN_DIAGNOSTIC_VERSION_DIGITS || !text.starts_with('0'))
}
#[derive(Clone, Copy)]
enum Suffix {
    Prerelease,
    Build,
}
fn suffix_valid(text: &str, suffix: Suffix) -> bool {
    text.split('.').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && match suffix {
                Suffix::Build => true,
                Suffix::Prerelease => {
                    !part.bytes().all(|byte| byte.is_ascii_digit()) || numeric(part)
                }
            }
    })
}
impl fmt::Display for DiagnosticVersion {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if self.prerelease {
            output.write_str(" (prerelease)")?;
        }
        if self.build_metadata {
            output.write_str(" (build metadata omitted)")?;
        }
        Ok(())
    }
}
