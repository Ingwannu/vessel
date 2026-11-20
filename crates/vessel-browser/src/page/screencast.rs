//! Probe the real compositor before any watch transport can be offered.

use std::time::Duration;

use super::{Page, PageError, requests};
use crate::cdp::{CdpError, Event, Events};
use vessel_proto::browser::{ScreenshotPng, ViewportSide, ViewportSize};

const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(4);
const SIGNAL_IMAGE_SIDE: u32 = 64;

pub(super) enum PaintSignals {
    Idle,
    Watching,
}

impl Page {
    pub(crate) async fn start_paint_signals(&mut self) -> Result<(), PageError> {
        if matches!(self.paint_signals, PaintSignals::Idle) {
            let side =
                ViewportSide::try_from(SIGNAL_IMAGE_SIDE).map_err(|_bound| PageError::Surface)?;
            let bounds = ViewportSize {
                width: side,
                height: side,
            };
            let _focused = self.execute(requests::BringToFront {}).await?;
            let _started = self
                .execute(requests::StartScreencast::with_viewport(bounds))
                .await?;
            self.paint_signals = PaintSignals::Watching;
        }
        Ok(())
    }

    pub(crate) async fn on_paint_signal(&mut self, event: &Event) -> Result<(), PageError> {
        if !matches!(self.paint_signals, PaintSignals::Watching) {
            return Ok(());
        }
        if !self.frame_watch.visible() {
            let _stopped = self.execute(requests::StopScreencast {}).await?;
            self.paint_signals = PaintSignals::Idle;
            self.frame_watch.clear();
        } else if let Some(frame) = event.screencast().map_err(PageError::Cdp)? {
            let _acknowledged = self.execute(frame.acknowledgement()).await?;
            self.frame_watch.painted(tokio::time::Instant::now());
        }
        Ok(())
    }
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
