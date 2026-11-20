//! Which built-in panel shows a file in the Files tab, Paseo's
//! `file-panels/catalog.ts` and `resolve.ts` (pinned 1c00b365c, 0.15.4),
//! ported from the page's `data/workspace/file-panels.ts` with its cases
//! (F-F38, F-F40; docs/plans/wp-file-viewers.md).
//!
//! [Decision Log]
//! - Purpose: Markdown with a Preview beside its Source, images drawn,
//!   every other file its Source, as Paseo's built-in panels.
//! - Constraint: the app has no web engine (ADR 0020 waits), so HTML stays
//!   Source only here (coordinator's ruling 2, 2026-11-19); Paseo also
//!   resolves plugin panels, which Vessel has not yet (area M).
//! - Choice: Paseo's catalog as data, matched as its resolver matches: the
//!   extension after the base name's last dot, lower case.
//! - Cost: a file whose content does not match its name (a `.png` that is
//!   text) opens in the panel its name says, as in Paseo.

/// An image the image panel draws, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// `png`.
    Png,
    /// `jpg`, `jpeg`.
    Jpeg,
    /// `gif`.
    Gif,
    /// `webp`.
    Webp,
    /// `svg`.
    Svg,
}

/// The built-in panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePanel {
    /// `md`, `markdown`: a Preview beside the Source.
    Markdown,
    /// `html`, `htm`: Paseo's sandboxed page; Source only in the app.
    Html,
    /// An image, drawn and zoomable.
    Image(ImageKind),
    /// Every other file: its Source.
    Source,
}

impl FilePanel {
    /// Whether the app draws a Preview beside the file's Source: Markdown
    /// (HTML's Preview needs a web engine, ruling 2).
    #[must_use]
    pub const fn has_preview(self) -> bool {
        matches!(self, Self::Markdown)
    }
}

/// Which side of a file with a Preview is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The rendered document.
    Preview,
    /// Its lines.
    Source,
}

impl Side {
    /// Paseo's `useTextFileView`: Preview, or Source when the file was
    /// opened at a line.
    #[must_use]
    pub const fn opening(at_line: bool) -> Self {
        if at_line { Self::Source } else { Self::Preview }
    }
}

/// The largest file a Preview draws: Paseo's native display budget
/// (`source/readonly/presentation.ts`, 10 MiB), past which its text panel
/// says the file is too large to display.
pub const PREVIEW_LIMIT: u64 = 10 * 1024 * 1024;

/// Whether a file of `size` bytes is drawn in its Preview.
#[must_use]
pub const fn preview_fits(size: u64) -> bool {
    size <= PREVIEW_LIMIT
}

/// The extension after the base name's last dot, lower case (Paseo's
/// `filePanelExtension`); empty when there is none.
#[must_use]
pub fn extension(path: &str) -> String {
    let name = path.trim().rsplit(['/', '\\']).next().unwrap_or_default();
    name.rsplit_once('.')
        .map(|(_, extension)| extension.to_lowercase())
        .unwrap_or_default()
}

/// The panel that shows `path`.
#[must_use]
pub fn panel_of(path: &str) -> FilePanel {
    match extension(path).as_str() {
        "md" | "markdown" => FilePanel::Markdown,
        "html" | "htm" => FilePanel::Html,
        "png" => FilePanel::Image(ImageKind::Png),
        "jpg" | "jpeg" => FilePanel::Image(ImageKind::Jpeg),
        "gif" => FilePanel::Image(ImageKind::Gif),
        "webp" => FilePanel::Image(ImageKind::Webp),
        "svg" => FilePanel::Image(ImageKind::Svg),
        _ => FilePanel::Source,
    }
}

/// An image file's kind; `None` for a file the image panel does not claim.
#[must_use]
pub fn image_kind(path: &str) -> Option<ImageKind> {
    match panel_of(path) {
        FilePanel::Image(kind) => Some(kind),
        FilePanel::Markdown | FilePanel::Html | FilePanel::Source => None,
    }
}

#[cfg(test)]
mod tests;
