//! Candidate frames retain the document and viewport that produced their pixels.

use super::{Page, PageError, TargetId, requests};
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::watch, time::Instant};
use vessel_proto::browser::{BrowserRevision, HumanInput, ScreenshotPng};

/// Keep CDP and the decoder below the package's ten frames per second bound.
const MINIMUM_FRAME_GAP: Duration = Duration::from_millis(100);

/// The page owns one latest frame. Lagging receivers never grow a frame queue.
/// State changes and terminal notices travel separately at the node boundary.
pub(super) struct FrameWatch {
    latest: watch::Sender<Option<Arc<ViewportFrame>>>,
    published_at: Option<Instant>,
}

impl FrameWatch {
    pub(super) fn new() -> Self {
        let (latest, _) = watch::channel(None);
        Self {
            latest,
            published_at: None,
        }
    }

    fn ready_at(&self, now: Instant) -> bool {
        self.latest.receiver_count() > 0
            && self
                .published_at
                .is_none_or(|last| now.saturating_duration_since(last) >= MINIMUM_FRAME_GAP)
    }

    pub(super) fn clear(&self) {
        // Keep the rate limit across invalidation so rapid navigation cannot
        // force a burst of captures. The previous document's pixels are gone.
        let _changed = self.latest.send_if_modified(|frame| frame.take().is_some());
    }
}

pub(crate) struct ViewportFrame {
    target: TargetId,
    revision: BrowserRevision,
    document: requests::Frame,
    image: ScreenshotPng,
}

impl ViewportFrame {
    pub(crate) const fn image(&self) -> &ScreenshotPng {
        &self.image
    }
}

impl Page {
    pub(crate) fn subscribe_frames(&mut self) -> watch::Receiver<Option<Arc<ViewportFrame>>> {
        if self.frame_watch.latest.receiver_count() == 0 {
            // Reopening a hidden target starts with the next live paint.
            self.frame_watch.clear();
        }
        self.frame_watch.latest.subscribe()
    }

    /// The owner calls this on a paint signal. Hidden targets and repeated
    /// signals inside the frame budget incur no capture work.
    pub(crate) async fn refresh_visible_frame(&mut self) -> Result<(), PageError> {
        if !self.frame_watch.ready_at(Instant::now()) {
            return Ok(());
        }
        let frame = self.viewport_frame().await?;
        // A receiver can drop while the capture is in flight. `send` releases
        // the image in that case instead of parking pixels for a hidden pane.
        if self.frame_watch.latest.send(Some(Arc::new(frame))).is_ok() {
            self.frame_watch.published_at = Some(Instant::now());
        }
        Ok(())
    }

    /// Capture transport and the paint target must agree before a frame can
    /// back coordinate input. The native image decoder still checks the pixels.
    pub(crate) async fn viewport_frame(&mut self) -> Result<ViewportFrame, PageError> {
        let document = self.observed_document().await?;
        let image = self.capture().await?;
        if self.observed_document().await? != document {
            let _revision = self.invalidate()?;
            return Err(PageError::Navigation);
        }
        if image.size().width().get() != self.viewport.width.get()
            || image.size().height().get() != self.viewport.height.get()
        {
            return Err(PageError::Surface);
        }
        Ok(ViewportFrame {
            target: self.target.clone(),
            revision: self.revision(),
            document,
            image,
        })
    }

    /// The caller must already own the target. A stale visible frame cannot
    /// direct input even when the replacement tab has the same dimensions.
    pub(crate) async fn input_from_frame(
        &mut self,
        frame: &ViewportFrame,
        input: HumanInput,
    ) -> Result<(), PageError> {
        if frame.revision != self.revision() || frame.target != self.target {
            return Err(PageError::StaleFrame);
        }
        if self.observed_document().await? != frame.document {
            let _revision = self.invalidate()?;
            return Err(PageError::StaleFrame);
        }
        self.human_input(input).await
    }

    async fn observed_document(&mut self) -> Result<requests::Frame, PageError> {
        self.execute(requests::FrameTree {})
            .await
            .map(|result| result.frame_tree.frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hidden_or_throttled_target_does_not_request_capture() {
        let mut watch = FrameWatch::new();
        let now = Instant::now();
        assert!(!watch.ready_at(now));
        let receiver = watch.latest.subscribe();
        assert!(watch.ready_at(now));
        watch.published_at = Some(now);
        assert!(!watch.ready_at(now));
        watch.clear();
        assert!(!watch.ready_at(now));
        assert!(watch.ready_at(now + MINIMUM_FRAME_GAP));
        drop(receiver);
        assert!(!watch.ready_at(now + MINIMUM_FRAME_GAP));
    }
}
