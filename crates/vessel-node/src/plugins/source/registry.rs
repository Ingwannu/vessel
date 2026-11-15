//! Registry origin identity and public slugs only; no credential lookup/fetch.
use super::{SourceError, reference::SourceText};
use std::fmt;
use url::{Host, Url};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct RegistryId(SourceText);
impl fmt::Debug for RegistryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RegistryId(<private>)")
    }
}
impl RegistryId {
    pub(super) fn parse(raw: &str) -> Result<Self, SourceError> {
        let (owner, slug) = raw.split_once('/').ok_or(SourceError::Registry)?;
        if !component(owner) || !component(slug) {
            return Err(SourceError::Registry);
        }
        Ok(Self(SourceText::new(raw)?))
    }
    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
fn component(raw: &str) -> bool {
    !raw.is_empty()
        && raw.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct RegistryEndpoint(Url);
impl fmt::Debug for RegistryEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RegistryEndpoint(<private>)")
    }
}
impl RegistryEndpoint {
    pub(super) fn parse(raw: &str) -> Result<Self, SourceError> {
        let _bounded = SourceText::new(raw)?;
        if raw.chars().any(char::is_whitespace) {
            return Err(SourceError::Registry);
        }
        let endpoint = Url::parse(raw).map_err(SourceError::Url)?;
        let authority = raw
            .split_once("://")
            .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
            .ok_or(SourceError::Registry)?;
        if authority.is_empty()
            || authority.contains('@')
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.host().is_none()
        {
            return Err(SourceError::Registry);
        }
        let loopback = match endpoint.host() {
            Some(Host::Domain("localhost")) => true,
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            Some(Host::Domain(_)) | None => false,
        };
        if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && loopback) {
            return Err(SourceError::Registry);
        }
        Ok(Self(endpoint))
    }
    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone)]
pub(super) struct RegistrySource {
    pub(super) endpoint: RegistryEndpoint,
    pub(super) id: RegistryId,
}

pub(super) fn explicit(raw: &str) -> Result<Option<RegistrySource>, SourceError> {
    let Some((host, id)) = raw.split_once('/') else {
        return Ok(None);
    };
    if !host.contains(['.', ':']) || host.contains(['@', '?', '#', '\\']) {
        return Ok(None);
    }
    let Ok(id) = RegistryId::parse(id) else {
        return Ok(None);
    };
    let endpoint = RegistryEndpoint::parse(&format!("https://{host}"))?;
    Ok(Some(RegistrySource { endpoint, id }))
}
