//! Validated bounded capability text, with redacted diagnostics. Crypto
//! point validation belongs to the node's cryptographic boundary.
use crate::key::{base64url, base64url_array};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Invalid subscription material; never echoes a capability or key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidPushMaterial {
    /// HTTPS, no user information/fragment, default HTTPS port, <= 2,048 bytes.
    Endpoint,
    /// A 65-byte uncompressed P-256 point encoded as unpadded base64url.
    PublicKey,
    /// A 16-byte authentication secret encoded as unpadded base64url.
    Auth,
}
impl fmt::Display for InvalidPushMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Endpoint => "subscribe again with a valid HTTPS push endpoint (at most 2048 bytes, no user information, fragment or custom port)",
            Self::PublicKey => "subscribe again with a valid uncompressed P-256 push public key",
            Self::Auth => "subscribe again with a valid 16-byte push authentication secret",
        })
    }
}
impl std::error::Error for InvalidPushMaterial {}

/// A browser-issued HTTPS capability. Never log or navigate to it.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PushEndpoint(String);
impl PushEndpoint {
    /// The exact bounded endpoint, only for authenticated delivery/storage.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl FromStr for PushEndpoint {
    type Err = InvalidPushMaterial;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() > 2048 || value.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(InvalidPushMaterial::Endpoint);
        }
        let url = url::Url::parse(value).map_err(|_url| InvalidPushMaterial::Endpoint)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url.port_or_known_default() != Some(443)
        {
            return Err(InvalidPushMaterial::Endpoint);
        }
        let normalized = url.to_string();
        if normalized.len() > 2048 {
            return Err(InvalidPushMaterial::Endpoint);
        }
        Ok(Self(normalized))
    }
}
impl fmt::Debug for PushEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PushEndpoint(<redacted>)")
    }
}
impl Serialize for PushEndpoint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.expose())
    }
}
impl<'de> Deserialize<'de> for PushEndpoint {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        String::deserialize(decoder)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// The device's encryption point, or the node's public VAPID identity.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PushPublicKey([u8; 65]);
impl PushPublicKey {
    /// Parse a SEC1 uncompressed point's shape; the node checks its curve.
    ///
    /// # Errors
    /// Wrong byte count or missing uncompressed-point marker.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidPushMaterial> {
        let bytes =
            <[u8; 65]>::try_from(bytes).map_err(|_length| InvalidPushMaterial::PublicKey)?;
        if bytes.first() != Some(&4) {
            return Err(InvalidPushMaterial::PublicKey);
        }
        Ok(Self(bytes))
    }
    /// Public SEC1 bytes for the cryptographic library.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 65] {
        &self.0
    }
}
impl FromStr for PushPublicKey {
    type Err = InvalidPushMaterial;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 87 {
            return Err(InvalidPushMaterial::PublicKey);
        }
        let bytes = base64url_array::<65>(text).map_err(|_key| InvalidPushMaterial::PublicKey)?;
        Self::from_bytes(&bytes)
    }
}
impl fmt::Debug for PushPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PushPublicKey(<redacted>)")
    }
}
impl Serialize for PushPublicKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base64url(&self.0))
    }
}
impl<'de> Deserialize<'de> for PushPublicKey {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        String::deserialize(decoder)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Web Push authentication material; Debug cannot print its secret bytes.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct PushAuth([u8; 16]);
impl PushAuth {
    /// Wrap the browser's fixed-length authentication secret.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    /// Secret bytes for encryption. Never log them.
    #[must_use]
    pub const fn expose(&self) -> &[u8; 16] {
        &self.0
    }
}
impl FromStr for PushAuth {
    type Err = InvalidPushMaterial;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != 22 {
            return Err(InvalidPushMaterial::Auth);
        }
        base64url_array::<16>(text)
            .map(Self)
            .map_err(|_key| InvalidPushMaterial::Auth)
    }
}
impl fmt::Debug for PushAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PushAuth(<redacted>)")
    }
}
impl Serialize for PushAuth {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&base64url(self.expose()))
    }
}
impl<'de> Deserialize<'de> for PushAuth {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        String::deserialize(decoder)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
