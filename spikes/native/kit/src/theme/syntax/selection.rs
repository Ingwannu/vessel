//! One syntax choice for existing code consumers, separate from app palettes.
//! A complete Copy enum is assigned under a short lock; poison recovery keeps
//! that complete last choice. No preference I/O or renderer state lives here.
use super::SyntaxPalette;
use std::sync::RwLock;
static CHOSEN: RwLock<SyntaxPalette> = RwLock::new(SyntaxPalette::One);

/// The selected syntax brand; its light/dark colours resolve at draw time.
#[must_use]
pub fn selected() -> SyntaxPalette {
    *CHOSEN
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
pub(crate) fn select(palette: SyntaxPalette) -> bool {
    let mut chosen = CHOSEN
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *chosen == palette {
        return false;
    }
    *chosen = palette;
    true
}
