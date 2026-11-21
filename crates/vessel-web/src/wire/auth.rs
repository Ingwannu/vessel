//! Fixed-size authentication values. Parse once at the wire boundary so
//! transcripts have one unambiguous representation and bounded size.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A malformed nonce or launch identifier.
#[derive(Debug, thiserror::Error)]
#[error("expected 32 bytes as lowercase hex")]
pub struct InvalidAuthValue;

macro_rules! hex_value {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS))]
        #[cfg_attr(feature = "ts", ts(type = "string"))]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            pub(crate) fn from_random(value: crate::LaunchToken) -> Self {
                Self(value.expose().to_owned())
            }
            /// The canonical hexadecimal representation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidAuthValue;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.len() == 64
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    Ok(Self(value))
                } else {
                    Err(InvalidAuthValue)
                }
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
hex_value!(Nonce, "A fresh 256-bit connection challenge.");
hex_value!(LaunchId, "The public identity of one UI process launch.");

/// Which key proves the connection: never send the key itself in a hello.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    /// Redeem the URL's code once, receiving an independent session key.
    Bootstrap,
    /// Reconnect using the session key already held by this tab.
    Resume,
}

/// A token as a browser presented it: not yet checked, never printed.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct PresentedToken(String);

impl PresentedToken {
    /// The token as given.
    #[must_use]
    pub fn new(token: String) -> Self {
        Self(token)
    }

    /// The text, for comparing.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PresentedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PresentedToken(..)")
    }
}

/// Text from the page that may be a credential: a pairing link (to
/// inspect or pair with), or the text of a QR code (a pairing link to
/// show). A link holds a single-use invitation until it is redeemed, so
/// `Debug` never prints it (an `Op` or a `ClientFrame` in a log line, a
/// test failure or a panic message shows a placeholder), and there is no
/// `Display`. It is not checked here: explaining a bad link is
/// `inspect`'s job, with the book's own parser.
///
/// [`SecretText::expose`] is called only where the text is parsed or
/// drawn (`ops`: inspect, pair, qr; the source's pairing); nothing on
/// those paths logs it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(transparent)]
pub struct SecretText(String);

impl SecretText {
    /// The text as the page sent it.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self(text)
    }

    /// The text, to parse or draw; never to log or format into a message.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretText(<redacted>)")
    }
}
