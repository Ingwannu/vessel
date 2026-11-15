//! Reuse GPUI's decoder and asynchronous asset cache for accurate image fit.

use gpui::{
    App, Asset, Image, ImageAssetLoader, ImageCacheError, ImageSource, RenderImage, Window,
};
use std::sync::Arc;

#[derive(Clone)]
struct PreviewDecoder;

impl Asset for PreviewDecoder {
    type Source = Arc<Image>;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move { source.to_image_data(renderer).map_err(Into::into) }
    }
}

pub(super) fn load(
    source: ImageSource,
    window: &mut Window,
    cx: &mut App,
) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
    match source {
        ImageSource::Render(image) => Some(Ok(image)),
        ImageSource::Image(image) => window.use_asset::<PreviewDecoder>(&image, cx),
        ImageSource::Resource(resource) => window.use_asset::<ImageAssetLoader>(&resource, cx),
        ImageSource::Custom(load) => load(window, cx),
    }
}
