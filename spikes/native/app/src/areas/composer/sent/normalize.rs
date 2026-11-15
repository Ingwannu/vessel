//! Remote image bytes stay in Reading until bounded first-frame normalization ends.
use super::{Arc, Context, Sent, Shell};
impl Shell {
    pub(super) fn sent_image_received(
        &mut self,
        sent: Sent,
        bytes: Option<Arc<[u8]>>,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(bytes) = bytes else {
            self.areas.composer.sent.borrow_mut().read(&sent, None);
            self.sent_reads();
            cx.notify();
            return;
        };
        let normalized = cx.background_executor().spawn(async move {
            vessel_app_kit::image_input::normalize(
                bytes.to_vec(),
                vessel_app_kit::image_input::InputLimits {
                    source: vessel_proto::AttachmentLimits::DEFAULT.image,
                    result: vessel_proto::AttachmentLimits::DEFAULT.image,
                },
            )
            .ok()
            .map(|image| Arc::from(image.bytes))
        });
        cx.spawn(async move |this, cx| {
            let bytes = normalized.await;
            let _closed = this.update(cx, |shell, cx| {
                shell.areas.composer.sent.borrow_mut().read(&sent, bytes);
                shell.sent_reads();
                cx.notify();
            });
        })
        .detach();
    }
}
