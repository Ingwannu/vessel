//! The kit's own drawings, served to GPUI as assets: the window controls'
//! four glyphs, as SVG paths on a 10 by 10 grid (Windows 11's shapes:
//! a line, a square, two offset squares, a cross).
//!
//! Embedded as text, not files: four short paths need no bundle, and an
//! app built from the kit needs no asset directory to draw its chrome.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

/// A glyph the kit draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// The window's minimise button.
    Minimize,
    /// Maximise.
    Maximize,
    /// Restore, when maximised.
    Restore,
    /// Close.
    Close,
}

impl Glyph {
    /// Its asset path.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Minimize => "kit/minimize.svg",
            Self::Maximize => "kit/maximize.svg",
            Self::Restore => "kit/restore.svg",
            Self::Close => "kit/close.svg",
        }
    }

    const fn svg(self) -> &'static str {
        match self {
            Self::Minimize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 5.5h10" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Maximize => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Restore => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="2.5" width="7" height="7" fill="none" stroke="black" stroke-width="1"/><path d="M2.5 2.5V0.5h7v7h-2" fill="none" stroke="black" stroke-width="1"/></svg>"#
            }
            Self::Close => {
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="black" stroke-width="1"/></svg>"#
            }
        }
    }
}

/// The kit's assets, for `Application::with_assets`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let glyph = [
            Glyph::Minimize,
            Glyph::Maximize,
            Glyph::Restore,
            Glyph::Close,
        ]
        .into_iter()
        .find(|glyph| glyph.path() == path);
        Ok(glyph.map(|glyph| Cow::Borrowed(glyph.svg().as_bytes())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok([
            Glyph::Minimize,
            Glyph::Maximize,
            Glyph::Restore,
            Glyph::Close,
        ]
        .into_iter()
        .map(Glyph::path)
        .filter(|asset| asset.starts_with(path))
        .map(SharedString::from)
        .collect())
    }
}
