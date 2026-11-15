//! One eligibility decision feeds toolbar buttons and keyboard actions alike.

/// History facts are separate from loading: an empty stack is not a disabled engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum History {
    /// A new tab has no history in either direction.
    #[default]
    Empty,
    /// Only a previous entry exists.
    Back,
    /// Only a later entry exists.
    Forward,
    /// Both entries exist.
    Both,
}

/// Refresh and Stop are mutually exclusive states of one toolbar control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadState {
    /// Page loading has settled, successfully or with an error.
    #[default]
    Settled,
    /// The page may receive Stop until the engine acknowledges completion.
    Loading,
}

/// Node-ready history/loading facts; unavailable engines never construct a ready pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Navigation {
    /// Eligibility comes from the engine, not guessed from URL changes.
    pub history: History,
    /// The same state drives toolbar icon, status bucket and shortcut behavior.
    pub load: LoadState,
}

/// A prepared toolbar intent, independent of a node or pane identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationAction {
    /// Request an existing backward entry.
    Back,
    /// Request an existing forward entry.
    Forward,
    /// Reload the current page.
    Reload,
    /// Stop the current loading attempt.
    Stop,
}

impl Navigation {
    /// Buttons and keyboard share these rules so empty history sends no no-op.
    #[must_use]
    pub const fn allows(self, action: NavigationAction) -> bool {
        match action {
            NavigationAction::Back => matches!(self.history, History::Back | History::Both),
            NavigationAction::Forward => matches!(self.history, History::Forward | History::Both),
            NavigationAction::Reload => matches!(self.load, LoadState::Settled),
            NavigationAction::Stop => matches!(self.load, LoadState::Loading),
        }
    }

    /// Loading has exactly one primary action; a stale toolbar never sends both.
    #[must_use]
    pub const fn primary(self) -> NavigationAction {
        match self.load {
            LoadState::Settled => NavigationAction::Reload,
            LoadState::Loading => NavigationAction::Stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_tab_can_reload_but_cannot_send_empty_history_or_stop() {
        let navigation = Navigation::default();
        assert_eq!(navigation.primary(), NavigationAction::Reload);
        assert!(navigation.allows(NavigationAction::Reload));
        assert!(!navigation.allows(NavigationAction::Stop));
        assert!(!navigation.allows(NavigationAction::Back));
        assert!(!navigation.allows(NavigationAction::Forward));
    }

    #[test]
    fn loading_does_not_destroy_the_nodes_history_facts() {
        for (history, back, forward) in [
            (History::Empty, false, false),
            (History::Back, true, false),
            (History::Forward, false, true),
            (History::Both, true, true),
        ] {
            let navigation = Navigation {
                history,
                load: LoadState::Loading,
            };
            assert_eq!(navigation.primary(), NavigationAction::Stop);
            assert!(navigation.allows(NavigationAction::Stop));
            assert!(!navigation.allows(NavigationAction::Reload));
            assert_eq!(navigation.allows(NavigationAction::Back), back);
            assert_eq!(navigation.allows(NavigationAction::Forward), forward);
        }
    }
}
