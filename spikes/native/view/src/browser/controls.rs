//! One General tab supplies the authoritative toolbar facts and target.

use vessel_proto::browser::{
    BrowserCommand, BrowserControl, BrowserControlCommand, BrowserHistory, BrowserLoad,
    BrowserProfile, BrowserRevision, BrowserTab, BrowserUrl, InvalidBrowserUrl, NavigateArgs,
    ResizeArgs, TabArgs, ViewportSide, ViewportSize,
};

use super::{History, LoadState, Navigation, NavigationAction, Viewport};

/// An admitted General tab and the revision that supplies its displayed facts.
#[derive(Debug)]
pub struct GeneralPane {
    tab: BrowserTab,
    revision: BrowserRevision,
}

impl GeneralPane {
    /// A service Preview never enters the General browser's resident state.
    #[must_use]
    pub fn new(tab: BrowserTab, revision: BrowserRevision) -> Option<Self> {
        (tab.profile == BrowserProfile::General).then_some(Self { tab, revision })
    }

    /// The current server descriptor. Editing the URL field cannot replace it.
    #[must_use]
    pub const fn tab(&self) -> &BrowserTab {
        &self.tab
    }

    /// Frames and controls are tied to the watched document generation.
    #[must_use]
    pub const fn revision(&self) -> BrowserRevision {
        self.revision
    }

    /// Admit only a descriptor from this pane's own current resource.
    pub fn observe(&mut self, incoming: BrowserTab, revision: BrowserRevision) -> bool {
        if incoming.profile != BrowserProfile::General
            || incoming.workspace != self.tab.workspace
            || incoming.browser_id != self.tab.browser_id
            || revision < self.revision
        {
            return false;
        }
        self.tab = incoming;
        self.revision = revision;
        true
    }

    /// Toolbar and shortcuts consume the same observed history and loading state.
    #[must_use]
    pub const fn navigation(&self) -> Navigation {
        Navigation {
            history: match self.tab.history {
                BrowserHistory::Empty => History::Empty,
                BrowserHistory::Back => History::Back,
                BrowserHistory::Forward => History::Forward,
                BrowserHistory::Both => History::Both,
            },
            load: match self.tab.load {
                BrowserLoad::Loading => LoadState::Loading,
                BrowserLoad::Ready | BrowserLoad::Failed(_) => LoadState::Settled,
            },
        }
    }

    /// An unavailable toolbar control never sends a mutation.
    #[must_use]
    pub fn navigation_request(&self, action: NavigationAction) -> Option<BrowserControl> {
        if !self.navigation().allows(action) {
            return None;
        }
        let browser_id = self.tab.browser_id.clone();
        let args = TabArgs { browser_id };
        let command = match action {
            NavigationAction::Back => BrowserControlCommand::Drive(BrowserCommand::Back(args)),
            NavigationAction::Forward => {
                BrowserControlCommand::Drive(BrowserCommand::Forward(args))
            }
            NavigationAction::Reload => BrowserControlCommand::Drive(BrowserCommand::Reload(args)),
            NavigationAction::Stop => BrowserControlCommand::Stop {
                browser_id: args.browser_id,
            },
        };
        Some(self.request(command))
    }

    /// Parse at the address field boundary before sending to the node's policy.
    ///
    /// # Errors
    /// Refuses unsupported schemes, user information and invalid addresses.
    pub fn address_request(&self, draft: &str) -> Result<BrowserControl, InvalidBrowserUrl> {
        let url = BrowserUrl::from_address(draft)?;
        let args = NavigateArgs {
            browser_id: self.tab.browser_id.clone(),
            url,
        };
        Ok(self.request(BrowserControlCommand::Drive(BrowserCommand::Navigate(args))))
    }

    /// Responsive uses current pane bounds. A device uses the table's exact CSS size.
    #[must_use]
    pub fn resize_request(
        &self,
        viewport: Viewport,
        responsive: ViewportSize,
    ) -> Option<BrowserControl> {
        let size = match viewport {
            Viewport::Responsive => responsive,
            Viewport::Device(device) => {
                let size = device.size();
                ViewportSize {
                    width: ViewportSide::try_from(u32::from(size.width().get())).ok()?,
                    height: ViewportSide::try_from(u32::from(size.height().get())).ok()?,
                }
            }
        };
        let args = ResizeArgs {
            browser_id: self.tab.browser_id.clone(),
            size,
        };
        Some(self.request(BrowserControlCommand::Drive(BrowserCommand::Resize(args))))
    }

    fn request(&self, command: BrowserControlCommand) -> BrowserControl {
        BrowserControl {
            workspace: self.tab.workspace,
            opening_session: None,
            command,
        }
    }
}

#[cfg(test)]
mod tests;
