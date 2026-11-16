//! One pin selection serves settings, cards and the sidebar summary.
use serde::{Deserialize, Serialize};

/// A pin matches every account of its source; human account labels never identify it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin<S, W> {
    /// Validated source identity supplied by the node contract.
    #[serde(alias = "sourceId")]
    pub source: S,
    /// Validated window identity, independent of label and account.
    #[serde(alias = "windowId")]
    pub window: W,
}

/// None follows source defaults; Some(empty) explicitly pins nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PinSelection<S, W>(Option<Vec<Pin<S, W>>>);
impl<S, W> Default for PinSelection<S, W> {
    fn default() -> Self {
        Self(None)
    }
}
impl<S: Clone + Eq, W: Clone + Eq> PinSelection<S, W> {
    /// Migrate a source namespace while preserving default/explicit-empty and first occurrence.
    #[must_use]
    pub fn map_sources(mut self, mut map: impl FnMut(S) -> S) -> Self {
        let Some(pins) = self.0.take() else {
            return self;
        };
        let mut mapped = Vec::new();
        for Pin { source, window } in pins {
            let pin = Pin {
                source: map(source),
                window,
            };
            if !mapped.contains(&pin) {
                mapped.push(pin);
            }
        }
        self.0 = Some(mapped);
        self
    }
    /// Import an explicit selection, including the intentionally empty one.
    #[must_use]
    pub fn explicit(pins: Vec<Pin<S, W>>) -> Self {
        Self(Some(pins))
    }
    /// Defaults are calculated only from currently available source reports.
    #[must_use]
    pub fn effective<'a>(&'a self, defaults: &'a [Pin<S, W>]) -> &'a [Pin<S, W>] {
        self.0.as_deref().unwrap_or(defaults)
    }
    /// The first edit snapshots displayed defaults before toggling exactly one pin.
    pub fn toggle(&mut self, pin: Pin<S, W>, defaults: &[Pin<S, W>]) {
        let pins = self.0.get_or_insert_with(|| defaults.to_vec());
        if let Some(index) = pins.iter().position(|existing| existing == &pin) {
            drop(pins.remove(index));
        } else {
            pins.push(pin);
        }
    }
    /// Pinning is based on typed source/window identity, never on translated text.
    #[must_use]
    pub fn contains(&self, pin: &Pin<S, W>, defaults: &[Pin<S, W>]) -> bool {
        self.effective(defaults).contains(pin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    enum Source {
        First,
        Second,
    }
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    enum Window {
        Session,
        Week,
    }
    fn pin(source: Source, window: Window) -> Pin<Source, Window> {
        Pin { source, window }
    }
    #[test]
    fn first_edit_snapshots_defaults_and_empty_is_not_default() {
        let first = pin(Source::First, Window::Session);
        let second = pin(Source::Second, Window::Week);
        let defaults = [first.clone(), second.clone()];
        let mut selected = PinSelection::default();
        assert_eq!(selected.effective(&defaults), &defaults);
        selected.toggle(first.clone(), &defaults);
        assert_eq!(selected.effective(&[]), std::slice::from_ref(&second));
        selected.toggle(second, &defaults);
        assert!(selected.effective(&defaults).is_empty());
        assert!(!selected.contains(&first, &defaults));
    }
    #[test]
    fn serialized_null_and_empty_have_different_meanings() {
        type Selection = PinSelection<Source, Window>;
        let defaults = [pin(Source::First, Window::Session)];
        let implicit: Selection = serde_json::from_str("null").unwrap();
        let empty: Selection = serde_json::from_str("[]").unwrap();
        assert_eq!(implicit.effective(&defaults), &defaults);
        assert!(empty.effective(&defaults).is_empty());
        assert_eq!(serde_json::to_string(&implicit).unwrap(), "null");
        assert_eq!(serde_json::to_string(&empty).unwrap(), "[]");
    }
    #[test]
    fn same_window_of_another_source_is_a_distinct_pin() {
        let first = pin(Source::First, Window::Week);
        let second = pin(Source::Second, Window::Week);
        let mut selected = PinSelection::explicit(vec![first.clone()]);
        selected.toggle(second.clone(), &[]);
        assert!(selected.contains(&first, &[]));
        assert!(selected.contains(&second, &[]));
    }
}
