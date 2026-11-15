//! A single actual CDP target, below authenticated workspace/profile admission.
//!
//! No constructor qualifies a runtime or authorizes a URL. The future profile
//! actor must enforce those boundaries before using this test-staged controller.
//! Mutable access serializes target operations. After a lost/failed CDP command,
//! only close is allowed: an uncertain mutation must not be silently replayed.

mod input;
mod snapshot;

use crate::cdp::{CdpError, Client, SessionId, TargetId, requests};
use vessel_proto::browser::{
    BrowserHistory, BrowserRevision, BrowserUrl, HistoryDirection, MAX_RGBA_BYTES, RequiredInput,
    ScreenshotPng, ViewportSize,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum PageError {
    #[error("browser target command failed")]
    Cdp(#[source] CdpError),
    #[error("browser target must be closed after an uncertain command")]
    Uncertain,
    #[error("browser history has no permitted entry in that direction")]
    NoHistory,
    #[error("browser history contains an unsupported destination")]
    Destination,
    #[error("browser surface exceeds the raster budget")]
    Surface,
    #[error("browser revision namespace is exhausted")]
    Exhausted,
    #[error("browser refused target closure")]
    CloseRefused,
    #[error("browser input operation is not implemented by this adapter")]
    Unsupported,
    #[error("browser rejected navigation")]
    Navigation,
    #[error("browser snapshot could not be produced")]
    Snapshot(#[from] crate::snapshot::SnapshotError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeliveryState {
    Active,
    Uncertain,
}

/// Backend identity stays inside the execution crate; public `BrowserId` is owned above it.
pub(crate) struct Page {
    client: Client,
    target: TargetId,
    session: SessionId,
    revision: BrowserRevision,
    delivery: DeliveryState,
    viewport: ViewportSize,
    pointer: requests::PointerState,
    pointer_position: Option<vessel_proto::browser::CssPoint>,
    references: crate::references::References<crate::snapshot::Binding>,
}
impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserPage")
            .field("revision", &self.revision)
            .field("delivery", &self.delivery)
            .finish_non_exhaustive()
    }
}
impl Page {
    /// The calling profile already owns the policy proxy, runtime and creation admission.
    pub(crate) async fn open(
        client: Client,
        url: BrowserUrl,
        viewport: ViewportSize,
    ) -> Result<Self, PageError> {
        surface(viewport)?;
        let created = client
            .execute(
                None,
                requests::CreateTarget {
                    url,
                    background: true,
                },
            )
            .await
            .map_err(PageError::Cdp)?;
        let setup = async {
            let attached = client
                .execute(None, requests::Attach::new(created.target_id.clone()))
                .await?;
            let _enabled_page = client
                .execute(Some(attached.session_id.clone()), requests::EnablePage {})
                .await?;
            let _enabled_runtime = client
                .execute(
                    Some(attached.session_id.clone()),
                    requests::EnableRuntime {},
                )
                .await?;
            let _enabled_network = client
                .execute(
                    Some(attached.session_id.clone()),
                    requests::EnableNetwork {},
                )
                .await?;
            let _viewport = client
                .execute(
                    Some(attached.session_id.clone()),
                    requests::SetViewport::new(viewport),
                )
                .await?;
            Ok::<_, CdpError>(attached.session_id)
        }
        .await;
        match setup {
            Ok(session) => Ok(Self {
                client,
                target: created.target_id,
                session,
                revision: BrowserRevision::ZERO,
                delivery: DeliveryState::Active,
                viewport,
                pointer: requests::PointerState::default(),
                pointer_position: None,
                references: crate::references::References::default(),
            }),
            Err(source) => {
                // Never report a ready target after partial setup. The profile
                // still retains the child and must clean up if this close fails.
                let _close = client
                    .execute(
                        None,
                        requests::CloseTarget {
                            target_id: created.target_id,
                        },
                    )
                    .await;
                Err(PageError::Cdp(source))
            }
        }
    }

    pub(crate) const fn revision(&self) -> BrowserRevision {
        self.revision
    }

    /// The owner invalidates its element references at this same boundary.
    pub(crate) fn invalidate(&mut self) -> Result<BrowserRevision, PageError> {
        self.references
            .invalidate()
            .map_err(crate::snapshot::SnapshotError::from)?;
        self.revision = self.revision.next().ok_or(PageError::Exhausted)?;
        Ok(self.revision)
    }

    async fn execute<R: requests::Request>(
        &mut self,
        request: R,
    ) -> Result<R::Response, PageError> {
        if self.delivery == DeliveryState::Uncertain {
            return Err(PageError::Uncertain);
        }
        match self
            .client
            .execute(Some(self.session.clone()), request)
            .await
        {
            Ok(result) => Ok(result),
            Err(source) => {
                self.delivery = DeliveryState::Uncertain;
                Err(PageError::Cdp(source))
            }
        }
    }

    pub(crate) async fn navigate(&mut self, url: BrowserUrl) -> Result<(), PageError> {
        let _revision = self.invalidate()?;
        let result = self.execute(requests::Navigate { url }).await?;
        if result.error_text.is_some() {
            return Err(PageError::Navigation);
        }
        Ok(())
    }
    pub(crate) async fn reload(&mut self) -> Result<(), PageError> {
        let _revision = self.invalidate()?;
        self.execute(requests::Reload {}).await.map(|_| ())
    }
    pub(crate) async fn stop(&mut self) -> Result<(), PageError> {
        self.execute(requests::Stop {}).await.map(|_| ())
    }
    pub(crate) async fn history(&mut self) -> Result<BrowserHistory, PageError> {
        let history = self.execute(requests::History {}).await?;
        let Some(index) = history
            .current_index
            .position()
            .filter(|index| *index < history.entries.len())
        else {
            return Err(PageError::NoHistory);
        };
        let back = index > 0;
        let forward = index
            .checked_add(1)
            .is_some_and(|next| next < history.entries.len());
        Ok(match (back, forward) {
            (false, false) => BrowserHistory::Empty,
            (true, false) => BrowserHistory::Back,
            (false, true) => BrowserHistory::Forward,
            (true, true) => BrowserHistory::Both,
        })
    }
    pub(crate) async fn traverse(&mut self, direction: HistoryDirection) -> Result<(), PageError> {
        let history = self.execute(requests::History {}).await?;
        let index = history
            .current_index
            .position()
            .filter(|index| *index < history.entries.len())
            .ok_or(PageError::NoHistory)?;
        let next = match direction {
            HistoryDirection::Back => index.checked_sub(1),
            HistoryDirection::Forward => index.checked_add(1),
        }
        .ok_or(PageError::NoHistory)?;
        let entry = history.entries.get(next).ok_or(PageError::NoHistory)?;
        // Never turn an engine history item into file/debug/external-handler authority.
        let _url = entry
            .url
            .as_str()
            .parse::<BrowserUrl>()
            .map_err(|_invalid| PageError::Destination)?;
        let id = entry.id;
        let _revision = self.invalidate()?;
        self.execute(requests::NavigateHistory { entry_id: id })
            .await
            .map(|_| ())
    }
    pub(crate) async fn resize(&mut self, viewport: ViewportSize) -> Result<(), PageError> {
        surface(viewport)?;
        let _revision = self.invalidate()?;
        let _viewport = self.execute(requests::SetViewport::new(viewport)).await?;
        self.viewport = viewport;
        Ok(())
    }
    pub(crate) async fn capture(&mut self) -> Result<ScreenshotPng, PageError> {
        self.execute(requests::Capture {})
            .await
            .map(|capture| capture.data)
    }
    pub(crate) async fn evaluate(
        &mut self,
        expression: RequiredInput,
    ) -> Result<requests::Evaluated, PageError> {
        self.execute(requests::Evaluate::new(expression)).await
    }
    pub(crate) async fn close(self) -> Result<(), PageError> {
        let result = self
            .client
            .execute(
                None,
                requests::CloseTarget {
                    target_id: self.target,
                },
            )
            .await
            .map_err(PageError::Cdp)?;
        if result.success {
            Ok(())
        } else {
            Err(PageError::CloseRefused)
        }
    }
}

fn surface(size: ViewportSize) -> Result<(), PageError> {
    const RGBA_BYTES_PER_PIXEL: u64 = 4;
    let bytes = u64::from(size.width.get())
        .checked_mul(u64::from(size.height.get()))
        .and_then(|pixels| pixels.checked_mul(RGBA_BYTES_PER_PIXEL));
    if bytes.is_some_and(|bytes| bytes <= MAX_RGBA_BYTES) {
        Ok(())
    } else {
        Err(PageError::Surface)
    }
}

#[cfg(test)]
mod tests;
