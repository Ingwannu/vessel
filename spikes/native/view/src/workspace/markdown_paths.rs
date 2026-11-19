//! Where a Markdown file's link or image points, as Paseo's preview
//! resolves it (`components/markdown/image-source.ts`, pinned 1c00b365c,
//! 0.15.4), ported from the page's `data/workspace/markdown-paths.ts`
//! with its cases: a relative target from the Markdown file's folder, an
//! absolute one as it is; a web or `data:` address is left to the caller.
//! The node serves files under the workspace root only, so a target is
//! kept as a path relative to that root, and one outside it is not a file
//! the preview can show (plan difference 7).

use std::str::FromStr as _;

use vessel_proto::workspace::RelPath;

/// What a target is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// An absolute http(s) address.
    Web(String),
    /// An image's own `data:` URL.
    Data(String),
    /// A file of the workspace.
    File(RelPath),
    /// Nothing the preview follows.
    None,
}

/// `text` with its `%XX` escapes decoded (`decodeURIComponent`); `None`
/// when an escape is malformed or the bytes are not UTF-8, where the
/// browser's call throws and Paseo keeps the text as written.
fn decoded(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        if byte == b'%' {
            let high = rest.next().and_then(hex)?;
            let low = rest.next().and_then(hex)?;
            bytes.push(high.wrapping_shl(4) | low);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// `parts` with `.` dropped and `..` taking the folder before it; `None`
/// when a `..` would leave the root.
fn normalized<'a>(parts: impl IntoIterator<Item = &'a str>) -> Option<Vec<&'a str>> {
    let mut kept = Vec::new();
    for part in parts {
        match part {
            "" | "." => {}
            ".." => {
                kept.pop()?;
            }
            _ => kept.push(part),
        }
    }
    Some(kept)
}

/// Whether `source` starts with a URL scheme of two or more characters
/// (so a drive letter, `C:`, is not one).
fn has_scheme(source: &str) -> bool {
    source.split_once(':').is_some_and(|(scheme, _)| {
        let mut chars = scheme.chars();
        scheme.len() >= 2
            && chars.next().is_some_and(|first| first.is_ascii_alphabetic())
            && chars.all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
            })
    })
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

/// The file `target` names, written in the Markdown file at `from`
/// (relative to the root, whose folder on the node is `folder` when
/// known).
#[must_use]
pub fn target(target: &str, from: &RelPath, folder: Option<&str>) -> Target {
    let source = target.trim();
    if source.is_empty() || source.starts_with('#') {
        return Target::None;
    }
    if starts_with_ignore_case(source, "http:") || starts_with_ignore_case(source, "https:") {
        return Target::Web(source.to_owned());
    }
    if starts_with_ignore_case(source, "data:image/") {
        return Target::Data(source.to_owned());
    }
    let file_url = starts_with_ignore_case(source, "file://");
    if has_scheme(source) && !starts_with_ignore_case(source, "file:") {
        // `blob:` names memory of the page that wrote it; any other
        // scheme is not a file.
        return Target::None;
    }
    let bare = source.split(['?', '#']).next().unwrap_or_default();
    let bare = if file_url {
        // `file://host/path`: the path from the first `/` after the host.
        bare.get("file://".len()..)
            .and_then(|rest| rest.find('/').and_then(|at| rest.get(at..)))
            .unwrap_or_default()
    } else {
        bare
    };
    let path = decoded(bare)
        .unwrap_or_else(|| bare.to_owned())
        .replace('\\', "/");
    if path.starts_with('~') {
        return Target::None;
    }
    let drive = path.get(1..3) == Some(":/")
        && path.chars().next().is_some_and(|first| first.is_ascii_alphabetic());
    let parts = if path.starts_with('/') || drive {
        let Some(folder) = folder else {
            return Target::None;
        };
        let folder = folder.replace('\\', "/");
        let (Some(base), Some(full)) = (normalized(folder.split('/')), normalized(path.split('/')))
        else {
            return Target::None;
        };
        if full.len() <= base.len() || !base.iter().zip(&full).all(|(a, b)| a == b) {
            return Target::None;
        }
        full.into_iter().skip(base.len()).map(str::to_owned).collect::<Vec<_>>()
    } else {
        let directory = from.as_str().split('/').collect::<Vec<_>>();
        let directory = directory
            .split_last()
            .map(|(_, folders)| folders.to_vec())
            .unwrap_or_default();
        let Some(joined) = normalized(directory.into_iter().chain(path.split('/'))) else {
            return Target::None;
        };
        joined.into_iter().map(str::to_owned).collect()
    };
    if parts.is_empty() {
        return Target::None;
    }
    RelPath::from_str(&parts.join("/")).map_or(Target::None, Target::File)
}

#[cfg(test)]
mod tests;
