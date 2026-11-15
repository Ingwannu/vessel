//! Assistant-authored file references are parsed without touching the filesystem.
//! The Files area still authorizes roots and performs every lookup/open operation.

mod location;
mod percent;
pub mod recognize;

use std::num::NonZeroU32;
use std::sync::Arc;

use vessel_proto::HttpUrl;

use crate::matcher::whitespace;

/// A path parsed from assistant prose; it grants no authority to read that path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathToken(Arc<str>);

impl PathToken {
    /// The Files owner validates this against its authorized workspace roots.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One-based inclusive source lines, validated before navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lines {
    start: NonZeroU32,
    end: NonZeroU32,
}

impl Lines {
    /// The first source line.
    #[must_use]
    pub const fn start(self) -> NonZeroU32 {
        self.start
    }
    /// The last source line, equal to start for a point target.
    #[must_use]
    pub const fn end(self) -> NonZeroU32 {
        self.end
    }

    fn parse(start: &str, end: Option<&str>) -> Option<Self> {
        if !digits(start) || end.is_some_and(|end| !digits(end)) {
            return None;
        }
        let start = start.parse::<NonZeroU32>().ok()?;
        let end = end.map_or(Some(start), |end| end.parse::<NonZeroU32>().ok())?;
        (start <= end).then_some(Self { start, end })
    }
}

/// Authored hrefs are encoded; text/inline-code tokens retain literal percent signs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A markdown link's destination identifies a file explicitly.
    Href,
    /// Prose and inline code need a file-looking suffix before becoming links.
    Inferred,
}

/// Whether the caller can resolve a relative path in a known workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    /// Relative references can be resolved by the Files area.
    Known,
    /// Only absolute or home-relative paths can be classified.
    Unknown,
}

/// A parsed target still needs current-workspace authorization and possibly lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTarget {
    /// Path without its line suffix.
    pub path: PathToken,
    /// Source navigation, if the assistant supplied it.
    pub lines: Option<Lines>,
}

/// An inferred relative path needs suffix search before being opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// An HTTP link is dispatched through the host's external opener.
    External(HttpUrl),
    /// Explicit or absolute file target; its root must still be authorized.
    File(FileTarget),
    /// A file-looking relative token requiring lookup within the current root.
    Lookup(FileTarget),
}

/// Classify a target, preserving raw percent escapes outside authored hrefs.
#[must_use]
pub fn classify(value: &str, origin: Origin, workspace: Workspace) -> Option<Link> {
    let value = value.trim_matches(whitespace);
    if let Some(url) = http_url(value) {
        return Some(Link::External(url));
    }
    let target = parse_file(value, origin)?;
    let absolute = absolute(target.path.as_str()) || target.path.as_str().starts_with("~/");
    if !absolute && workspace == Workspace::Unknown {
        return None;
    }
    if origin == Origin::Inferred && !absolute {
        if !looks_like_file(target.path.as_str()) {
            return None;
        }
        Some(Link::Lookup(target))
    } else {
        Some(Link::File(target))
    }
}

fn http_url(value: &str) -> Option<HttpUrl> {
    let (scheme, rest) = value.split_once("://")?;
    let normalized = if scheme.eq_ignore_ascii_case("http") {
        "http"
    } else if scheme.eq_ignore_ascii_case("https") {
        "https"
    } else {
        return None;
    };
    if scheme == normalized {
        value.parse().ok()
    } else {
        format!("{normalized}://{rest}").parse().ok()
    }
}

/// Parse the location alone, without performing navigation or filesystem access.
#[must_use]
pub fn parse_file(value: &str, origin: Origin) -> Option<FileTarget> {
    let value = value
        .trim_matches(whitespace)
        .trim_matches(['\'', '"', '`']);
    if value.is_empty() || value.contains('?') || value.chars().any(char::is_control) {
        return None;
    }
    let (value, file_url) = match value.strip_prefix("file://") {
        Some(value) if value.starts_with('/') => (value, true),
        Some(_) => return None,
        None => (value, false),
    };
    if value.contains("://") {
        return None;
    }
    let (path, lines) = location::extract(value)?;
    // A raw colon is either the Windows drive or a valid location delimiter.
    // Encoded colons are decoded afterwards and remain filename bytes.
    let colon_path = if file_url && path.get(2..3) == Some(":") {
        path.strip_prefix('/').unwrap_or(path)
    } else {
        path
    };
    if colon_path.match_indices(':').any(|(index, _)| {
        index != 1
            || !colon_path
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
            || !matches!(colon_path.as_bytes().get(2), Some(b'/' | b'\\'))
    }) {
        return None;
    }
    let decoded = if origin == Origin::Href || file_url {
        percent::decode(path).unwrap_or_else(|| path.to_owned())
    } else {
        path.to_owned()
    };
    let normalized = decoded.replace('\\', "/");
    let normalized = if file_url && normalized.get(2..3) == Some(":") {
        normalized.strip_prefix('/').unwrap_or(&normalized)
    } else {
        normalized.as_str()
    };
    if normalized.is_empty() || normalized.chars().any(char::is_control) {
        return None;
    }
    Some(FileTarget {
        path: PathToken(Arc::from(normalized)),
        lines,
    })
}

fn absolute(path: &str) -> bool {
    path.starts_with('/')
        || (path.as_bytes().get(1) == Some(&b':')
            && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            && path.as_bytes().get(2) == Some(&b'/'))
}

fn looks_like_file(path: &str) -> bool {
    if path.starts_with("./") || path.starts_with("../") {
        return true;
    }
    if let Some((first, _)) = path.split_once('/')
        && domain_like(first)
    {
        return false;
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    if name.starts_with('.') && name.len() > 1 {
        return true;
    }
    let Some((_, extension)) = name.rsplit_once('.') else {
        return false;
    };
    EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

fn domain_like(segment: &str) -> bool {
    segment.contains('.')
        && segment.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

/// Reference extension allowlist; inferred tokens alone use it, never authored links.
const EXTENSIONS: &[&str] = &[
    "astro", "bash", "c", "cc", "cjs", "cpp", "cs", "css", "cts", "cxx", "env", "fish", "go",
    "gql", "gradle", "graphql", "h", "hpp", "htm", "html", "ini", "java", "js", "json", "jsonc",
    "jsx", "kt", "kts", "less", "lock", "lua", "md", "mdx", "mjs", "mts", "php", "proto", "py",
    "rb", "rs", "sass", "scss", "sh", "sql", "svelte", "swift", "toml", "ts", "tsx", "txt", "vue",
    "xml", "yaml", "yml", "zsh",
];

#[cfg(test)]
mod tests;

fn digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}
