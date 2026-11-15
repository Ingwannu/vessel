//! Resolved native font stacks. The app supplies installed/embedded face
//! names; this module never parses preferences or opens a file or URL.
//!
//! A tiny process-wide snapshot matches the existing type scale's lifetime.
//! Font assignment contains no callbacks or I/O; recovering a poisoned lock
//! preserves the last complete snapshot, never a half-applied font stack.
use super::{HANGUL, MONO, SANS};
use gpui::{FontFallbacks, SharedString};
use std::sync::{LazyLock, RwLock};

/// The semantic font axis; content inherits the interface face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Typeface {
    /// Navigation, controls, chat and Markdown prose.
    Interface,
    /// Code, diffs and terminal text.
    Code,
}

/// A resolved primary face and ordered readable fallbacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontStack {
    family: SharedString,
    fallbacks: FontFallbacks,
}

impl FontStack {
    /// Build from available actual face names, never CSS generic aliases.
    /// The embedded Hangul face is retained even for a custom primary face.
    #[must_use]
    pub fn from_names(names: impl IntoIterator<Item = SharedString>, role: Typeface) -> Self {
        let mut names = names.into_iter().filter(|name| !name.is_empty());
        let family = names.next().unwrap_or_else(|| default_family(role).into());
        let mut fallbacks: Vec<String> = names
            .filter(|name| !name.eq_ignore_ascii_case(&family))
            .map(|name| name.to_string())
            .collect();
        if !fallbacks
            .iter()
            .any(|name| name.eq_ignore_ascii_case(HANGUL))
        {
            fallbacks.push(HANGUL.to_owned());
        }
        Self {
            family,
            fallbacks: FontFallbacks::from_fonts(fallbacks),
        }
    }

    /// The available primary family, without CSS quotes or a generic alias.
    #[must_use]
    pub fn family(&self) -> SharedString {
        self.family.clone()
    }

    /// The same fallback list used for shaping, measurement and rendering.
    #[must_use]
    pub fn fallbacks(&self) -> FontFallbacks {
        self.fallbacks.clone()
    }
}

fn default_family(role: Typeface) -> &'static str {
    match role {
        Typeface::Interface => SANS,
        Typeface::Code => MONO,
    }
}

#[derive(PartialEq, Eq)]
struct Stacks {
    interface: FontStack,
    code: FontStack,
}
impl Default for Stacks {
    fn default() -> Self {
        Self {
            interface: FontStack::from_names([], Typeface::Interface),
            code: FontStack::from_names([], Typeface::Code),
        }
    }
}
static STACKS: LazyLock<RwLock<Stacks>> = LazyLock::new(|| RwLock::new(Stacks::default()));

/// Read one complete configured stack; clones share the fallback allocation.
#[must_use]
pub fn active(role: Typeface) -> FontStack {
    let stacks = STACKS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match role {
        Typeface::Interface => stacks.interface.clone(),
        Typeface::Code => stacks.code.clone(),
    }
}

pub(super) fn set(interface: FontStack, code: FontStack) -> bool {
    let next = Stacks { interface, code };
    let mut stacks = STACKS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *stacks == next {
        return false;
    }
    *stacks = next;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_stacks_use_embedded_defaults_and_custom_faces_keep_hangul() {
        let default = FontStack::from_names([], Typeface::Interface);
        assert_eq!(default.family(), SANS);
        let custom = FontStack::from_names(
            ["Example".into(), "Pretendard".into(), "Example".into()],
            Typeface::Code,
        );
        assert_eq!(custom.family(), "Example");
        assert_eq!(custom.fallbacks().fallback_list(), &[HANGUL]);
        assert_eq!(FontStack::from_names([], Typeface::Code).family(), MONO);
    }
}
