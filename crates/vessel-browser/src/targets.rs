//! The browser owner binds each live tab to one previously admitted workspace.

use std::collections::{BTreeMap, btree_map::Entry};

use vessel_proto::WorkspaceId;
use vessel_proto::browser::{BrowserId, BrowserUrl, MAX_BROWSER_TABS, ViewportSize};

use crate::cdp::{Client, SessionId};
use crate::page::{Page, PageError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum TargetError {
    #[error("browser target capacity is occupied")]
    Busy,
    #[error("browser tab is unavailable in this workspace")]
    Missing,
    #[error("browser target id is already in use")]
    Collision,
    #[error("browser target did not complete its operation")]
    Page(#[from] PageError),
}

struct Binding {
    workspace: WorkspaceId,
    page: Page,
}

/// One engine owns this inventory. Callers supply a workspace already resolved
/// by the node, and never pass in a browser id for a newly created tab.
#[derive(Default)]
pub(crate) struct Targets(BTreeMap<BrowserId, Binding>);

impl Targets {
    pub(crate) fn observing(&mut self, session: &SessionId) -> Option<&mut Page> {
        self.0
            .values_mut()
            .find(|binding| binding.page.uses_session(session))
            .map(|binding| &mut binding.page)
    }
    pub(crate) async fn open(
        &mut self,
        client: &Client,
        workspace: WorkspaceId,
        url: BrowserUrl,
        viewport: ViewportSize,
    ) -> Result<BrowserId, TargetError> {
        if !u32::try_from(self.0.len()).is_ok_and(|count| count < MAX_BROWSER_TABS) {
            return Err(TargetError::Busy);
        }
        // Reserve privately so a collision cannot replace a live binding.
        // The caller learns the ID only after the real creation succeeds.
        let browser = BrowserId::random();
        let Entry::Vacant(entry) = self.0.entry(browser.clone()) else {
            return Err(TargetError::Collision);
        };
        let page = Page::open(client.clone(), url, viewport).await?;
        let _binding = entry.insert(Binding { workspace, page });
        Ok(browser)
    }

    pub(crate) fn owned(
        &mut self,
        workspace: WorkspaceId,
        browser: &BrowserId,
    ) -> Result<&mut Page, TargetError> {
        self.0
            .get_mut(browser)
            .filter(|binding| binding.workspace == workspace)
            .map(|binding| &mut binding.page)
            .ok_or(TargetError::Missing)
    }

    pub(crate) async fn close(
        &mut self,
        workspace: WorkspaceId,
        browser: &BrowserId,
    ) -> Result<(), TargetError> {
        // Preserve the target and its allocation if a close has uncertain
        // effects. The owner can still clean up the actual engine in that case.
        self.owned(workspace, browser)?.close().await?;
        let _closed = self.0.remove(browser);
        Ok(())
    }
}
