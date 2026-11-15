//! Derive one service environment suffix from its already validated name.
//! Files and runtime injection share this rule; it reads no process environment.

use std::fmt;
use std::sync::Arc;

use super::ScriptName;

/// The normalized suffix shared by peer-port and peer-URL variables.
/// Construct it only from `ScriptName`, whose ASCII grammar prevents an empty key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceEnvName(Arc<str>);

impl ServiceEnvName {
    /// Derive Paseo's uppercase/collapsed form without revalidating a name.
    #[must_use]
    pub fn of(name: &ScriptName) -> Self {
        let mut text = String::with_capacity(name.as_str().len());
        let mut separator = false;
        for byte in name.as_str().bytes() {
            if byte.is_ascii_alphanumeric() {
                if separator {
                    text.push('_');
                }
                text.push(char::from(byte.to_ascii_uppercase()));
                separator = false;
            } else {
                // Leading separators cannot occur in ScriptName; a trailing run
                // is only emitted when another alphanumeric byte follows.
                separator = true;
            }
        }
        Self(Arc::from(text))
    }

    /// Compose an environment key without recovering or parsing the source name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ServiceEnvName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_uppercase_and_separator_runs_match_the_reference() {
        for (input, expected) in [
            ("web", "WEB"),
            ("ApiV2", "APIV2"),
            ("web-app", "WEB_APP"),
            ("web__..--app", "WEB_APP"),
            ("web-", "WEB"),
            ("7-api._", "7_API"),
        ] {
            let name = input.parse().unwrap();
            let derived = ServiceEnvName::of(&name);
            assert_eq!(derived.as_str(), expected);
            assert_eq!(derived.to_string(), expected);
        }
        for text in ["web-app", "web_app", "web.app"] {
            assert_eq!(
                ServiceEnvName::of(&text.parse().unwrap()).as_str(),
                "WEB_APP"
            );
        }
    }
}
