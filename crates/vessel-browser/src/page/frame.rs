//! Candidate frames retain the document and viewport that produced their pixels.

use super::{Page, PageError, TargetId, requests};
use vessel_proto::browser::{BrowserRevision, HumanInput, ScreenshotPng};

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
