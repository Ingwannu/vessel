//! A project's own icon (ADR 0112), as Paseo's `utils/project-icon.ts`
//! finds it: its names in its order (SVG and PNG before ICO), first in
//! `public`, `static`, `priv/static`, `assets`, `images`, `img` (two levels
//! under each), then each package of `packages/*` and `apps/*`, then the
//! root itself; the first file found is the answer or there is none.
//!
//! [Decision Log]
//! - Purpose: projects drawn with their own mark, as Paseo's sidebar and
//!   projects screen draw them.
//! - Constraint: ADR 0112's conditions: inside the project's root only, no
//!   link followed, square, at most [`MAX_ICON_BYTES`], the type decided by
//!   the bytes (never the name), and only types every client draws as an
//!   image.
//! - Alternatives: Paseo's name-based fallback type and its JPEG, GIF and
//!   WebP sniffing (the names it looks for are SVG, PNG and ICO; a
//!   renamed JPEG would be the only gain).
//! - Choice: PNG (signature, square by its header), ICO (header; its
//!   largest PNG frame sent as PNG when it has one, as Paseo's), SVG (text
//!   whose first element is `<svg`; trusted square, as Paseo's). Links are
//!   never read or walked; entries are taken in name order (Paseo's
//!   `readdir` order is the disk's) and the walk reads at most
//!   [`MAX_ICON_SCAN`] entries.
//! - Cost: a project whose icon is reached only through a link draws the
//!   folder mark.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use vessel_proto::workspace::host_folders::{
    IconBytes, IconImage, IconType, MAX_ICON_BYTES, ProjectIcon,
};

use super::{FileError, blocking};
use crate::paths::Root;

/// Paseo's `ICON_PATTERNS`, in its order; `*` stands for any text.
const PATTERNS: &[&str] = &[
    "favicon.svg",
    "favicon.png",
    "favicon-*.svg",
    "favicon-*.png",
    "favico.svg",
    "favico.png",
    "icon.svg",
    "icon.png",
    "app-icon.svg",
    "app-icon.png",
    "apple-touch-icon.png",
    "apple-touch-icon-*.png",
    "icon-*.png",
    "android-chrome-*.png",
    "safari-pinned-tab.svg",
    "mstile-*.png",
    "logo.svg",
    "logo.png",
    "favicon.ico",
    "favico.ico",
];
/// Paseo's `PRIORITY_DIRS`.
const PRIORITY: &[&str] = &["public", "static", "priv/static", "assets", "images", "img"];
/// Paseo's `MONOREPO_PACKAGE_DIRS`.
const PACKAGES: &[&str] = &["packages", "apps"];
/// Paseo's `IGNORED_DIRS`.
const IGNORED: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".output",
    "coverage",
    ".cache",
    "vendor",
    "src",
    "lib",
    "test",
    "tests",
    "__tests__",
];
/// Levels under a priority folder (Paseo's `maxDepth - 1`).
const PRIORITY_DEPTH: usize = 2;
/// Entries read in one search at most (Paseo's has no bound).
pub const MAX_ICON_SCAN: usize = 5_000;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The icon of the project at `root`, or none.
///
/// # Errors
/// Only a stopped blocking task: an unreadable folder or file, as a file
/// of no allowed type, is no icon.
pub async fn project_icon(root: &Root) -> Result<ProjectIcon, FileError> {
    let root = root.path().to_path_buf();
    blocking(move || {
        let icon = Search::default()
            .find(&root)
            .and_then(|path| read(&path))
            .and_then(decide);
        Ok(ProjectIcon { icon })
    })
    .await
}

#[derive(Default)]
struct Search {
    scanned: usize,
}

impl Search {
    fn find(&mut self, root: &Path) -> Option<PathBuf> {
        if let Some(found) = self.priority(root) {
            return Some(found);
        }
        for packages in PACKAGES {
            if !real_dir(root, packages) {
                continue;
            }
            for package in self.folders(&root.join(packages), &[]) {
                if let Some(found) = self.priority(&package).or_else(|| self.in_dir(&package)) {
                    return Some(found);
                }
            }
        }
        self.in_dir(root)
    }

    /// Entries of `dir` in name order: (path, whether a real folder,
    /// whether a real file). Links are neither.
    fn entries(&mut self, dir: &Path) -> Vec<(String, bool, bool)> {
        let Ok(read) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        for entry in read.flatten() {
            if self.scanned >= MAX_ICON_SCAN {
                break;
            }
            self.scanned = self.scanned.saturating_add(1);
            let (Ok(kind), Some(name)) = (
                entry.file_type(),
                entry.file_name().to_str().map(str::to_owned),
            ) else {
                continue;
            };
            entries.push((name, kind.is_dir(), kind.is_file()));
        }
        entries.sort();
        entries
    }

    fn folders(&mut self, dir: &Path, skip: &[&str]) -> Vec<PathBuf> {
        self.entries(dir)
            .into_iter()
            .filter(|(name, folder, _)| *folder && !skip.contains(&name.as_str()))
            .map(|(name, _, _)| dir.join(name))
            .collect()
    }

    /// The first of Paseo's names in `dir` (pattern order, then name).
    fn in_dir(&mut self, dir: &Path) -> Option<PathBuf> {
        let files: Vec<String> = self
            .entries(dir)
            .into_iter()
            .filter(|(_, _, file)| *file)
            .map(|(name, _, _)| name)
            .collect();
        PATTERNS.iter().find_map(|pattern| {
            files
                .iter()
                .find(|name| matches(name, pattern))
                .map(|name| dir.join(name))
        })
    }

    fn priority(&mut self, base: &Path) -> Option<PathBuf> {
        PRIORITY.iter().find_map(|dir| {
            if real_dir(base, dir) {
                self.under(&base.join(dir), 0)
            } else {
                None
            }
        })
    }

    /// `dir`, then its folders to [`PRIORITY_DEPTH`] (Paseo's
    /// `searchDirRecursively`).
    fn under(&mut self, dir: &Path, depth: usize) -> Option<PathBuf> {
        if let Some(found) = self.in_dir(dir) {
            return Some(found);
        }
        if depth >= PRIORITY_DEPTH {
            return None;
        }
        for folder in self.folders(dir, IGNORED) {
            if let Some(found) = self.under(&folder, depth.saturating_add(1)) {
                return Some(found);
            }
        }
        None
    }
}

/// Whether `relative` (`/` between parts) under `base` is a folder with
/// no link on the way: `read_dir` would follow one out of the root.
fn real_dir(base: &Path, relative: &str) -> bool {
    let mut path = base.to_path_buf();
    relative.split('/').all(|part| {
        path.push(part);
        fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir())
    })
}

/// Paseo's `matchesPattern`: `*` is any text, the rest exact.
fn matches(name: &str, pattern: &str) -> bool {
    match pattern.split_once('*') {
        Some((head, tail)) => {
            name.len() >= head.len().saturating_add(tail.len())
                && name.starts_with(head)
                && name.ends_with(tail)
        }
        None => name == pattern,
    }
}

/// The file's bytes when it is a regular file of at most
/// [`MAX_ICON_BYTES`], opened without following a link in its last part
/// and judged on the opened handle (`crate::regular`). A folder on the way
/// swapped for a link after the search is the race ADR 0027 accepts for
/// every root check (a process of the same user can race them).
fn read(path: &Path) -> Option<Vec<u8>> {
    let file = crate::regular::open(path).ok()??;
    let budget = u64::try_from(MAX_ICON_BYTES).ok()?;
    if file.metadata().ok()?.len() > budget {
        return None;
    }
    let mut bytes = Vec::new();
    let _read = file
        .take(budget.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= MAX_ICON_BYTES).then_some(bytes)
}

/// The icon the bytes make, by their signature.
fn decide(bytes: Vec<u8>) -> Option<IconImage> {
    let (kind, bytes) = if bytes.starts_with(PNG_SIGNATURE) {
        square_png(&bytes).then_some((IconType::Png, bytes))?
    } else if is_ico(&bytes) {
        match ico_png_frame(&bytes) {
            Some(frame) => square_png(&frame).then_some((IconType::Png, frame))?,
            None => (IconType::Ico, bytes),
        }
    } else if is_svg(&bytes) {
        (IconType::Svg, bytes)
    } else {
        return None;
    };
    Some(IconImage {
        kind,
        bytes: IconBytes::new(bytes.into()).ok()?,
    })
}

fn u16_le(bytes: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    Some(u16::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

fn u32_le(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_le_bytes(bytes.get(at..end)?.try_into().ok()?))
}

fn u32_be(bytes: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    Some(u32::from_be_bytes(bytes.get(at..end)?.try_into().ok()?))
}

/// A PNG's header says it is square (Paseo's `isSquareImage`).
fn square_png(bytes: &[u8]) -> bool {
    matches!((u32_be(bytes, 16), u32_be(bytes, 20)), (Some(width), Some(height)) if width == height && width > 0)
}

/// An ICO header: reserved 0, type 1, at least one image.
fn is_ico(bytes: &[u8]) -> bool {
    u16_le(bytes, 0) == Some(0)
        && u16_le(bytes, 2) == Some(1)
        && u16_le(bytes, 4).is_some_and(|count| count > 0)
}

/// The widest PNG frame of an ICO (Paseo's `extractIcoPngFrame`).
fn ico_png_frame(bytes: &[u8]) -> Option<Vec<u8>> {
    let count = usize::from(u16_le(bytes, 4)?);
    let mut best: Option<(u16, &[u8])> = None;
    for index in 0..count {
        let entry = index.checked_mul(16)?.checked_add(6)?;
        let Some(&width) = bytes.get(entry) else {
            break;
        };
        let width = if width == 0 { 256 } else { u16::from(width) };
        let (Some(size), Some(offset)) = (
            u32_le(bytes, entry.checked_add(8)?),
            u32_le(bytes, entry.checked_add(12)?),
        ) else {
            break;
        };
        let start = usize::try_from(offset).ok()?;
        let end = start.checked_add(usize::try_from(size).ok()?)?;
        let Some(frame) = bytes.get(start..end) else {
            continue;
        };
        if frame.starts_with(PNG_SIGNATURE) && best.is_none_or(|(widest, _)| width > widest) {
            best = Some((width, frame));
        }
    }
    best.map(|(_, frame)| frame.to_vec())
}

/// SVG text: after an optional byte order mark, XML declaration, comments
/// and doctype, the first element is `svg`.
fn is_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    loop {
        let skipped = if rest.starts_with("<?") {
            rest.split_once("?>").map(|(_, after)| after)
        } else if rest.starts_with("<!--") {
            rest.split_once("-->").map(|(_, after)| after)
        } else if rest
            .get(..9)
            .is_some_and(|head| head.eq_ignore_ascii_case("<!doctype"))
        {
            rest.split_once('>').map(|(_, after)| after)
        } else {
            break;
        };
        let Some(after) = skipped else {
            return false;
        };
        rest = after.trim_start();
    }
    rest.strip_prefix("<svg")
        .and_then(|after| after.chars().next())
        .is_some_and(|next| next.is_whitespace() || next == '>' || next == '/')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    fn ico_with(frame: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0, 0, 1, 0, 1, 0];
        bytes.extend_from_slice(&[32, 32, 0, 0, 1, 0, 32, 0]);
        bytes.extend_from_slice(&u32::try_from(frame.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(&22_u32.to_le_bytes());
        bytes.extend_from_slice(frame);
        bytes
    }

    async fn icon_of(dir: &Path) -> Option<IconImage> {
        project_icon(&Root::open(dir).unwrap()).await.unwrap().icon
    }

    #[test]
    fn the_type_comes_from_the_bytes() {
        assert_eq!(
            decide(png(16, 16)).map(|icon| icon.kind),
            Some(IconType::Png)
        );
        assert_eq!(decide(png(16, 8)), None);
        let svg = br#"<?xml version="1.0"?><!-- mark --><svg xmlns="http://www.w3.org/2000/svg"/>"#;
        assert_eq!(
            decide(svg.to_vec()).map(|icon| icon.kind),
            Some(IconType::Svg)
        );
        assert_eq!(decide(b"<html><svg></svg></html>".to_vec()), None);
        assert_eq!(decide(b"GIF89a....".to_vec()), None);
        let framed = decide(ico_with(&png(32, 32))).unwrap();
        assert_eq!(framed.kind, IconType::Png);
        assert!(framed.bytes.bytes().starts_with(PNG_SIGNATURE));
        let bitmap = decide(ico_with(&[0; 40])).unwrap();
        assert_eq!(bitmap.kind, IconType::Ico);
        assert!(matches("favicon-32x32.png", "favicon-*.png"));
        assert!(!matches("favicon.png", "favicon-*.png"));
    }

    #[tokio::test]
    async fn paseos_places_and_order_inside_the_root_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        // The root's own icon is the last place looked.
        fs::write(root.join("logo.png"), png(8, 8)).unwrap();
        fs::create_dir_all(root.join("public/icons")).unwrap();
        fs::write(root.join("public/icons/favicon.ico"), ico_with(&[0; 40])).unwrap();
        fs::write(
            root.join("public/icons/icon.svg"),
            b"<svg viewBox='0 0 1 1'/>",
        )
        .unwrap();
        // Under `public`, SVG before ICO whatever the names' order.
        let icon = icon_of(root).await.unwrap();
        assert_eq!(icon.kind, IconType::Svg);
        fs::remove_dir_all(root.join("public")).unwrap();
        assert_eq!(icon_of(root).await.unwrap().kind, IconType::Png);
        // Too large: none.
        fs::write(root.join("logo.png"), vec![0; MAX_ICON_BYTES + 1]).unwrap();
        assert_eq!(icon_of(root).await, None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_link_is_never_read_or_walked() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("favicon.png"), png(8, 8)).unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("public")).unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("packages")).unwrap();
        fs::create_dir(temp.path().join("priv")).unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("priv/static")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("favicon.png"),
            temp.path().join("icon.png"),
        )
        .unwrap();
        assert_eq!(icon_of(temp.path()).await, None);
    }
}
