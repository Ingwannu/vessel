//! Colon subpaths are parsed independently of host filesystem syntax. No path
//! here is canonicalized, opened or granted as a launch capability.
use super::SourceError;
use std::{fmt, sync::Arc};
use vessel_proto::workspace::{RelPath, trim_project_text};

pub(super) const MAX_SOURCE_BYTES: usize = 4096;
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SourceText(Arc<str>);
impl SourceText {
    pub(super) fn new(raw: &str) -> Result<Self, SourceError> {
        if raw.is_empty()
            || raw.len() > MAX_SOURCE_BYTES
            || raw != trim_project_text(raw)
            || raw.chars().any(char::is_control)
        {
            return Err(SourceError::Text);
        }
        Ok(Self(Arc::from(raw)))
    }
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for SourceText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SourceText(<private>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Prefix {
    Npm,
    Github,
    Git,
    None,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    Directory,
    Managed,
}
#[derive(Debug)]
pub(super) struct Reference {
    pub(super) prefix: Prefix,
    pub(super) shape: Shape,
    pub(super) source: SourceText,
    pub(super) subpath: Option<RelPath>,
}

pub(super) fn reference(raw: &str) -> Result<Reference, SourceError> {
    let _bounded = SourceText::new(raw)?;
    let (prefix, source) = if let Some(source) = raw.strip_prefix("npm:") {
        (Prefix::Npm, source)
    } else if let Some(source) = raw.strip_prefix("github:") {
        (Prefix::Github, source)
    } else if let Some(source) = raw
        .strip_prefix("git:")
        .filter(|rest| !rest.starts_with("//"))
    {
        (Prefix::Git, source)
    } else {
        (Prefix::None, raw)
    };
    let (source, subpath) = split_subpath(source, prefix);
    let shape = if prefix == Prefix::None && directory_shape(source) {
        Shape::Directory
    } else {
        Shape::Managed
    };
    Ok(Reference {
        prefix,
        shape,
        source: SourceText::new(source)?,
        subpath,
    })
}

fn split_subpath(raw: &str, prefix: Prefix) -> (&str, Option<RelPath>) {
    let Some((before, tail)) = raw.rsplit_once(':') else {
        return (raw, None);
    };
    let Some(subpath) = portable(tail) else {
        return (raw, None);
    };
    if let Some((_, authority)) = raw.split_once("://") {
        let path = authority.find('/');
        let separator = before.len();
        let authority_start = raw.len().saturating_sub(authority.len());
        if path.is_none_or(|path| separator < authority_start.saturating_add(path)) {
            return (raw, None);
        }
    } else if prefix != Prefix::Npm && scp_separator(raw) == Some(before.len()) {
        return (raw, None);
    }
    (before, Some(subpath))
}

pub(super) fn scp_separator(raw: &str) -> Option<usize> {
    let (user, host_path) = raw.split_once('@')?;
    if user.is_empty() || user.contains('/') || user.chars().any(char::is_whitespace) {
        return None;
    }
    let (host, _path) = if host_path.starts_with('[') {
        let (inside, path) = host_path.split_once("]:")?;
        (host_path.get(..inside.len().saturating_add(1))?, path)
    } else {
        host_path.split_once(':')?
    };
    if host.is_empty() || host.chars().any(char::is_whitespace) {
        return None;
    }
    Some(user.len().saturating_add(1).saturating_add(host.len()))
}

fn portable(raw: &str) -> Option<RelPath> {
    if raw == "." {
        return Some(RelPath::root());
    }
    if raw.contains(':') {
        return None;
    }
    raw.replace('\\', "/")
        .parse()
        .ok()
        .filter(|path: &RelPath| !path.is_root())
}

fn directory_shape(raw: &str) -> bool {
    raw.starts_with(['/', '~'])
        || raw.starts_with("\\\\")
        || raw == "."
        || raw == ".."
        || raw.starts_with("./")
        || raw.starts_with("../")
        || raw.starts_with(".\\")
        || raw.starts_with("..\\")
        || raw.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && raw
                .get(1..)
                .is_some_and(|rest| rest.starts_with(":/") || rest.starts_with(":\\"))
}
