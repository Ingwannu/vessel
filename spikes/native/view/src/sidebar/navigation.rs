//! Sidebar header/footer order and visibility, following Paseo's
//! `sidebar-nav/model.ts`. Keys identify UI contributions, never a node
//! installation or RPC target; those remain the plugin owner's addresses.
//!
//! [Decision Log]
//! Retain unavailable keys through edits so disconnecting a plugin does not
//! erase its place. A bounded opaque UI key also retains future builtins;
//! interpreting it as a plugin id would invent another identity contract.

mod key;
#[cfg(test)]
mod tests;
mod words;

use serde::{Deserialize, Serialize};

pub use key::{Builtin, InvalidKey, Key, MAX_KEY_BYTES};

/// Maximum persisted rows in each band, including unavailable contributions.
pub const MAX_ENTRIES: usize = 128;

/// Independent preference bands; a plugin's local id may occur in both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// Rows above the scrolling workspace body.
    Header,
    /// Optional rows above the fixed footer icons.
    Footer,
}

/// Visibility at call sites, stored as Paseo's boolean field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
pub enum Visibility {
    /// The row participates in rendering.
    Shown,
    /// Retain its place but omit its rendering.
    Hidden,
}
impl From<bool> for Visibility {
    fn from(value: bool) -> Self {
        if value { Self::Shown } else { Self::Hidden }
    }
}
impl From<Visibility> for bool {
    fn from(value: Visibility) -> Self {
        value == Visibility::Shown
    }
}

/// One stored or resolved row. The array order is the display order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Stable UI contribution key, independent of online hosts.
    pub key: Key,
    /// Whether the row is shown.
    pub visible: Visibility,
}

/// A bounded band of stored preferences; duplicate keys are read first-wins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Item>", into = "Vec<Item>")]
pub struct Entries(Vec<Item>);

/// Refusing growth retains the complete previous preference document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a sidebar band holds at most {MAX_ENTRIES} items")]
pub struct TooMany;

impl TryFrom<Vec<Item>> for Entries {
    type Error = TooMany;
    fn try_from(items: Vec<Item>) -> Result<Self, Self::Error> {
        if items.len() > MAX_ENTRIES {
            Err(TooMany)
        } else {
            Ok(Self(items))
        }
    }
}
impl From<Entries> for Vec<Item> {
    fn from(entries: Entries) -> Self {
        entries.0
    }
}

/// Native persisted bands; empty means Paseo's current default order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Navigation {
    #[serde(deserialize_with = "crate::prefs::lenient")]
    header: Entries,
    #[serde(deserialize_with = "crate::prefs::lenient")]
    footer: Entries,
}
impl Navigation {
    /// Whether no choice has been stored, preserving older document bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.header.0.is_empty() && self.footer.0.is_empty()
    }

    /// Stored choices for one independent band.
    #[must_use]
    pub const fn entries(&self, section: Section) -> &Entries {
        match section {
            Section::Header => &self.header,
            Section::Footer => &self.footer,
        }
    }

    /// Whether a builtin is requested, independent of current data availability.
    /// Usage fetch policy reads this same choice as the footer renderer.
    #[must_use]
    pub fn visibility(&self, builtin: Builtin) -> Visibility {
        self.entries(builtin.section())
            .items()
            .iter()
            .find(|item| item.key.builtin() == Some(builtin))
            .map_or_else(|| builtin.default_visibility(), |item| item.visible)
    }

    /// Replace only one band through the existing preference writer.
    pub(crate) fn set(&mut self, section: Section, entries: Entries) {
        match section {
            Section::Header => self.header = entries,
            Section::Footer => self.footer = entries,
        }
    }
}

/// A settings row moves one place in the current available inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Toward the first row.
    Up,
    /// Toward the last row.
    Down,
}

impl Entries {
    /// Retained choices, including unavailable keys.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.0
    }

    /// Resolve the band's current inventory. Contribution keys come from the
    /// plugin owner after placement grouping; their order is authoritative.
    #[must_use]
    pub fn resolve(&self, section: Section, contributions: &[Key]) -> Vec<Item> {
        let available = |key: &Key| match key.builtin() {
            Some(builtin) => builtin.section() == section,
            None => contributions.contains(key),
        };
        let mut items = Vec::new();
        for item in &self.0 {
            if available(&item.key) && !items.iter().any(|known: &Item| known.key == item.key) {
                items.push(item.clone());
            }
        }
        for builtin in Builtin::ALL
            .into_iter()
            .filter(|builtin| builtin.section() == section)
        {
            let key = Key::of(builtin);
            if !items.iter().any(|item| item.key == key) {
                items.push(Item {
                    key,
                    visible: builtin.default_visibility(),
                });
            }
        }
        for key in contributions.iter().filter(|key| key.builtin().is_none()) {
            if !items.iter().any(|item| &item.key == key) {
                items.push(Item {
                    key: key.clone(),
                    visible: Visibility::Shown,
                });
            }
        }
        items
    }

    /// Change visibility while preserving every unavailable preference.
    /// # Errors
    /// Refuses a band that would exceed the named persistence bound.
    pub fn visibility(
        &self,
        items: &[Item],
        key: &Key,
        visible: Visibility,
    ) -> Result<Self, TooMany> {
        let items: Vec<_> = items
            .iter()
            .map(|item| Item {
                key: item.key.clone(),
                visible: if &item.key == key {
                    visible
                } else {
                    item.visible
                },
            })
            .collect();
        self.rewrite(&items)
    }

    /// Move one current row. Moving the first up or the last down is a no-op.
    /// # Errors
    /// Refuses growth past the named persistence bound.
    pub fn moved(&self, items: &[Item], key: &Key, direction: Direction) -> Result<Self, TooMany> {
        let mut next = items.to_vec();
        if let Some(from) = next.iter().position(|item| &item.key == key) {
            let to = match direction {
                Direction::Up => from.checked_sub(1),
                Direction::Down => from.checked_add(1).filter(|to| *to < next.len()),
            };
            if let Some(to) = to {
                next.swap(from, to);
            }
        }
        self.rewrite(&next)
    }

    fn rewrite(&self, items: &[Item]) -> Result<Self, TooMany> {
        let mut remaining = items.iter();
        let mut kept = Vec::new();
        let mut seen = Vec::new();
        for previous in &self.0 {
            if seen.contains(&previous.key) {
                continue;
            }
            seen.push(previous.key.clone());
            if items.iter().any(|item| item.key == previous.key) {
                kept.extend(remaining.next().cloned());
            } else {
                kept.push(previous.clone());
            }
        }
        kept.extend(remaining.cloned());
        Self::try_from(kept)
    }
}
