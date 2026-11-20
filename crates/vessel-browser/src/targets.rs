//! The browser owner binds each live tab to one previously admitted workspace.

use std::collections::{BTreeMap, btree_map::Entry};
use std::sync::Arc;

use tokio::sync::{broadcast, watch};
use vessel_proto::WorkspaceId;
use vessel_proto::browser::{
    BrowserFailure, BrowserId, BrowserLoad, BrowserRevision, BrowserUrl, MAX_BROWSER_TABS,
    ViewportSize,
};

use crate::cdp::{Client, SessionId};
use crate::page::{Page, PageError, ViewportFrame};

const NOTICE_CAPACITY: usize = 16;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetState {
    pub(crate) revision: BrowserRevision,
    pub(crate) load: BrowserLoad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetNotice {
    State(TargetState),
    Ended(BrowserFailure),
}

pub(crate) struct TargetWatch {
    pub(crate) initial: TargetState,
    pub(crate) notices: broadcast::Receiver<TargetNotice>,
    pub(crate) frames: watch::Receiver<Option<Arc<ViewportFrame>>>,
}

pub(crate) struct Binding {
    workspace: WorkspaceId,
    pub(crate) page: Page,
    state: TargetState,
    notices: broadcast::Sender<TargetNotice>,
}

impl Binding {
    fn new(workspace: WorkspaceId, page: Page) -> Self {
        let (notices, _) = broadcast::channel(NOTICE_CAPACITY);
        let state = TargetState { revision: page.revision(), load: BrowserLoad::Loading };
        Self { workspace, page, state, notices }
    }

    pub(crate) fn watch(&mut self) -> TargetWatch {
        TargetWatch {
            initial: self.state,
            notices: self.notices.subscribe(),
            frames: self.page.subscribe_frames(),
        }
    }

    pub(crate) fn announce(&mut self, load: BrowserLoad) {
        let latest = TargetState { revision: self.page.revision(), load };
        if self.state != latest {
            self.state = latest;
            let _receivers = self.notices.send(TargetNotice::State(latest));
        }
    }

    fn end(&self, reason: BrowserFailure) {
        let _receivers = self.notices.send(TargetNotice::Ended(reason));
    }
}

/// One engine owns this inventory. Callers supply a workspace already resolved
/// by the node, and never pass in a browser id for a newly created tab.
#[derive(Default)]
pub(crate) struct Targets(BTreeMap<BrowserId, Binding>);

impl Targets {
    pub(crate) fn observing(&mut self, session: &SessionId) -> Option<&mut Binding> {
        self.0
            .values_mut()
            .find(|binding| binding.page.uses_session(session))
    }

    pub(crate) fn end(self, reason: BrowserFailure) {
        for binding in self.0.into_values() {
            binding.end(reason);
        }
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
        let _binding = entry.insert(Binding::new(workspace, page));
        Ok(browser)
    }

    pub(crate) fn owned(
        &mut self,
        workspace: WorkspaceId,
        browser: &BrowserId,
    ) -> Result<&mut Page, TargetError> {
        Ok(&mut self.binding(workspace, browser)?.page)
    }

    pub(crate) fn binding(
        &mut self,
        workspace: WorkspaceId,
        browser: &BrowserId,
    ) -> Result<&mut Binding, TargetError> {
        self.0
            .get_mut(browser)
            .filter(|binding| binding.workspace == workspace)
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
        if let Some(closed) = self.0.remove(browser) {
            closed.end(BrowserFailure::Closed);
        }
        Ok(())
    }
}
