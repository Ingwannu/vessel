//! Validated provider metadata, without network, model loading or settings I/O.
//!
//! URL parsing is delegated to the existing workspace parser so normalised IP
//! addresses cannot bypass the cleartext restriction (ADR 0067).

use super::DictationDecodeError;
use crate::key::serde_via_text;
use std::{cell::Cell, fmt, str::FromStr};

// Shared with error formatting: changing a check must change its diagnostic.
pub(super) const MIN_TEXT_CHARS: usize = 1;
pub(super) const MAX_LANGUAGE_CHARS: usize = 16;
pub(super) const MAX_MODEL_CHARS: usize = 64;
const MAX_MODEL_BYTES: usize = MAX_MODEL_CHARS * char::MAX.len_utf8();
pub(super) const MAX_URL_BYTES: usize = 256;
const MAX_URL_INPUT_BYTES: usize = MAX_URL_BYTES * char::MAX.len_utf8();

/// ASCII letters and `-`, bounded by `MIN_TEXT_CHARS` and `MAX_LANGUAGE_CHARS`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct Language(String);

impl Language {
    /// English, the default.
    #[must_use]
    pub fn english() -> Self {
        Self(String::from("en"))
    }

    /// Encode the validated language only at the provider boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Language {
    type Err = DictationDecodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let ok = (MIN_TEXT_CHARS..=MAX_LANGUAGE_CHARS).contains(&text.len())
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic() || byte == b'-');
        if ok {
            Ok(Self(text.to_owned()))
        } else {
            Err(DictationDecodeError::Language)
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

serde_via_text!(Language);

/// The endpoint's model name, bounded by `MAX_MODEL_CHARS`, without whitespace.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct EndpointModel(String);

impl EndpointModel {
    /// `whisper-1`, the default.
    #[must_use]
    pub fn whisper() -> Self {
        Self(String::from("whisper-1"))
    }

    /// Encode the validated model only at the provider boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for EndpointModel {
    type Err = DictationDecodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let ok = text.len() <= MAX_MODEL_BYTES
            && (MIN_TEXT_CHARS..=MAX_MODEL_CHARS).contains(&text.chars().count())
            && !text.chars().any(char::is_whitespace)
            && !text.chars().any(char::is_control);
        if ok {
            Ok(Self(text.to_owned()))
        } else {
            Err(DictationDecodeError::EndpointModel)
        }
    }
}

impl fmt::Display for EndpointModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

serde_via_text!(EndpointModel);

/// The endpoint's base URL. `https`, or `http` only when the host is
/// loopback, so the key is not sent in cleartext off this machine.
/// A host without userinfo, query or fragment, bounded by `MAX_URL_BYTES`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct EndpointBase(BaseAddress);

// The default is static, so Default needs no fallible parse or panic. Custom
// addresses retain their parsed URL instead of being decoded on every request.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BaseAddress {
    OpenAi,
    Custom(url::Url),
}

impl EndpointBase {
    /// The default OpenAI-compatible base.
    const OPENAI: &str = "https://api.openai.com/v1";

    /// The default base.
    #[must_use]
    pub fn openai() -> Self {
        Self(BaseAddress::OpenAi)
    }

    /// Encode the canonical base at the settings or HTTP boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            BaseAddress::OpenAi => Self::OPENAI,
            BaseAddress::Custom(url) => url.as_str().trim_end_matches('/'),
        }
    }

    /// Append the transcription path without reparsing an admin's address.
    ///
    /// # Errors
    /// An address cannot carry hierarchical paths, or the static default is invalid.
    pub fn transcription_url(&self) -> Result<url::Url, DictationDecodeError> {
        let mut url = match &self.0 {
            BaseAddress::OpenAi => Self::OPENAI
                .parse()
                .map_err(DictationDecodeError::EndpointUrl)?,
            BaseAddress::Custom(url) => url.clone(),
        };
        let _ = url
            .path_segments_mut()
            .map_err(|()| DictationDecodeError::EndpointBase)?
            .pop_if_empty()
            .push("audio")
            .push("transcriptions");
        Ok(url)
    }
}

impl FromStr for EndpointBase {
    type Err = DictationDecodeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > MAX_URL_INPUT_BYTES {
            return Err(DictationDecodeError::EndpointBase);
        }
        if text.chars().any(char::is_whitespace)
            || text.contains('\\')
            || text.chars().any(char::is_control)
        {
            return Err(DictationDecodeError::EndpointBase);
        }
        let mut url = decode_url(text)?;
        let loopback = match url.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
            None => false,
        };
        let transport = match url.scheme() {
            "https" => true,
            "http" => loopback,
            _ => false,
        };
        let canonical = url.as_str().trim_end_matches('/');
        if !transport
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url.query().is_some()
            || url.port() == Some(0)
            || canonical.len() > MAX_URL_BYTES
        {
            return Err(DictationDecodeError::EndpointBase);
        }
        if canonical == Self::OPENAI {
            return Ok(Self::openai());
        }
        // Trim only terminal separators; preserve escaped path bytes.
        let path = url.path().trim_end_matches('/').to_owned();
        url.set_path(&path);
        Ok(Self(BaseAddress::Custom(url)))
    }
}

impl fmt::Display for EndpointBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

serde_via_text!(EndpointBase);

// WHATWG parsing normalises an empty userinfo away. Observe the parser's
// syntax event so even `https://@host` obeys the no-userinfo contract.
#[derive(Clone, Copy, PartialEq, Eq)]
enum UserInfo {
    Absent,
    Present,
}

fn decode_url(text: &str) -> Result<url::Url, DictationDecodeError> {
    let credentials = Cell::new(UserInfo::Absent);
    let note = |violation| {
        if violation == url::SyntaxViolation::EmbeddedCredentials {
            credentials.set(UserInfo::Present);
        }
    };
    let url = url::Url::options()
        .syntax_violation_callback(Some(&note))
        .parse(text)
        .map_err(DictationDecodeError::EndpointUrl)?;
    if credentials.get() == UserInfo::Present {
        Err(DictationDecodeError::EndpointBase)
    } else {
        Ok(url)
    }
}
