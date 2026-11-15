//! npm owns version/tag/range interpretation. These values validate only the
//! source grammar before a future argv/lockfile adapter may use them.
use super::{SourceError, reference::SourceText};
use std::{fmt, sync::Arc};
use vessel_proto::workspace::trim_project_text;

const MAX_PACKAGE_BYTES: usize = 214;
const MAX_SELECTOR_BYTES: usize = 256;
#[derive(Clone, PartialEq, Eq)]
pub(super) struct PackageName(Arc<str>);
impl fmt::Debug for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PackageName(<private>)")
    }
}
#[derive(Debug, Clone)]
pub(super) struct NpmSource {
    pub(super) package: PackageName,
    pub(super) selector: SourceText,
}

impl PackageName {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(super) fn npm(raw: &str) -> Result<NpmSource, SourceError> {
    let _bounded = SourceText::new(raw)?;
    let (name, selector) = if let Some(scoped) = raw.strip_prefix('@') {
        let (scope, tail) = scoped.split_once('/').ok_or(SourceError::Npm)?;
        if !component(scope) {
            return Err(SourceError::Npm);
        }
        let (name, selector) = tail
            .split_once('@')
            .map_or((tail, None), |(name, selector)| (name, Some(selector)));
        if !component(name) {
            return Err(SourceError::Npm);
        }
        (format!("@{scope}/{name}"), selector)
    } else {
        let (name, selector) = raw
            .split_once('@')
            .map_or((raw, None), |(name, selector)| (name, Some(selector)));
        if !component(name) {
            return Err(SourceError::Npm);
        }
        (name.to_owned(), selector)
    };
    if name.len() > MAX_PACKAGE_BYTES {
        return Err(SourceError::Npm);
    }
    let selector = selector.unwrap_or("latest");
    if selector.is_empty()
        || selector.len() > MAX_SELECTOR_BYTES
        || selector != trim_project_text(selector)
        || selector.contains([':', '@', '/', '\\'])
    {
        return Err(SourceError::Npm);
    }
    Ok(NpmSource {
        package: PackageName(Arc::from(name)),
        selector: SourceText::new(selector)?,
    })
}
fn component(raw: &str) -> bool {
    raw.bytes()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && raw.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}
