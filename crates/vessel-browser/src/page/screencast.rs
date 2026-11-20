//! Probe the real compositor before any watch transport can be offered.

use std::time::Duration;

use super::{Page, PageError, requests};
use crate::cdp::{CdpError, Events};
use vessel_proto::browser::ScreenshotPng;

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(4);

impl Page {
    /// The candidate has a single selected target and consumes its actual first
    /// compositor frame. A product watch needs routing and separate frame backpressure.
    pub(crate) async fn probe_compositor(
        &mut self,
        events: &mut Events,
    ) -> Result<ScreenshotPng, PageError> {
        let _focused = self.execute(requests::BringToFront {}).await?;
        let _started = self
            .execute(requests::StartScreencast::with_viewport(self.viewport))
            .await?;
        let frame = tokio::time::timeout(FIRST_FRAME_TIMEOUT, self.first_frame(events)).await;
        let stopped = self.execute(requests::StopScreencast {}).await;
        let _stopped = stopped?;
        frame.map_err(PageError::NoFrame)?
    }

    async fn first_frame(&mut self, events: &mut Events) -> Result<ScreenshotPng, PageError> {
        while let Some(event) = events.next().await {
            if event.session() != Some(&self.session) {
                continue;
            }
            if let Some(frame) = event.screencast().map_err(PageError::Cdp)? {
                let _acknowledged = self.execute(frame.acknowledgement()).await?;
                return Ok(frame.into_image());
            }
        }
        Err(PageError::Cdp(CdpError::Closed))
    }
}
