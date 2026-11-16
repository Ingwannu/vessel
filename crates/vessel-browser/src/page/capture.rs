//! Capture scope is explicit and never manufactures a painted-frame claim.
use super::{Page, PageError, requests, surface};
use vessel_proto::browser::{ScreenshotExtent, ScreenshotPng};

impl Page {
    pub(crate) async fn capture_extent(
        &mut self,
        extent: ScreenshotExtent,
    ) -> Result<ScreenshotPng, PageError> {
        match extent {
            ScreenshotExtent::Viewport => self.capture().await,
            ScreenshotExtent::Page => self.capture_page().await,
        }
    }
    async fn capture_page(&mut self) -> Result<ScreenshotPng, PageError> {
        let frame = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        let size = self.execute(requests::Metrics {}).await?.size();
        surface(size)?;
        let image = self.execute(requests::CapturePage::new(size)).await?.data;
        let after = self.execute(requests::FrameTree {}).await?.frame_tree.frame;
        if frame.id != after.id || frame.loader_id != after.loader_id {
            let _revision = self.invalidate()?;
            return Err(PageError::Navigation);
        }
        if image.size().width().get() != size.width.get()
            || image.size().height().get() != size.height.get()
        {
            return Err(PageError::Surface);
        }
        Ok(image)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(in crate::page) async fn cases(page: &mut Page) {
        assert!(matches!(
            page.capture_extent(ScreenshotExtent::Page).await,
            Err(PageError::Surface)
        ));
        let full = page.capture_extent(ScreenshotExtent::Page).await.unwrap();
        assert_eq!(full.size().width().get(), 391);
        assert_eq!(full.size().height().get(), 1701);
        assert!(matches!(
            page.capture_extent(ScreenshotExtent::Page).await,
            Err(PageError::Surface)
        ));
        assert!(matches!(
            page.capture_extent(ScreenshotExtent::Page).await,
            Err(PageError::Navigation)
        ));
    }
}
