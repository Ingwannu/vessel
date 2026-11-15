//! Reuse canonical relative paths and real URL validation; do not grant I/O.
use serde::Deserialize;
use std::fmt;
use vessel_proto::{
    RelPath,
    plugins::{AttachmentError, AttachmentUrl},
};

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(super) struct PackagePath(RelPath);

#[derive(Debug, thiserror::Error)]
pub(super) enum InvalidPackagePath {
    #[error("plugin asset is not a valid package-relative path")]
    Path(#[source] vessel_proto::workspace::InvalidRelPath),
    #[error("plugin asset must name a confined package-relative file")]
    Policy,
    #[error("plugin media URL is invalid")]
    Url(#[source] AttachmentError),
}

impl TryFrom<String> for PackagePath {
    type Error = InvalidPackagePath;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let path: RelPath = raw.parse().map_err(InvalidPackagePath::Path)?;
        if path.is_root()
            || raw.contains([':', '?', '#'])
            || raw != vessel_proto::workspace::trim_project_text(&raw)
        {
            return Err(InvalidPackagePath::Policy);
        }
        Ok(Self(path))
    }
}
impl PackagePath {
    pub(super) fn relative(&self) -> RelPath {
        self.0.clone()
    }

    pub(super) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Debug for PackagePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PackagePath(<private>)")
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub(super) enum Media {
    Package(PackagePath),
    Https(AttachmentUrl),
}
impl fmt::Debug for Media {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Package(_) => "Package(<private>)",
            Self::Https(_) => "Https(<private>)",
        })
    }
}
impl TryFrom<String> for Media {
    type Error = InvalidPackagePath;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        if raw
            .split_once(':')
            .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("https"))
        {
            raw.parse()
                .map(Self::Https)
                .map_err(InvalidPackagePath::Url)
        } else {
            PackagePath::try_from(raw).map(Self::Package)
        }
    }
}
