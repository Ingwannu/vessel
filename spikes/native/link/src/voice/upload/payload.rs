//! A typed request's shared bytes retain the original capture allocation and permit.
//!
//! Dropping an upload future does not necessarily destroy its queued request.
//! Keep the device's budget owner inside each queued raw-PCM byte allocation.
//! Encoding may release that owner before sink admission; the resulting JSON
//! frame has the RPC transport's separate bound, not this capture permit.

use super::{CaptureError, Frame};
use bytes::Bytes;

impl Frame {
    /// Check the capture interval at this boundary, without copying raw PCM or
    /// detaching its budget. The adapter must still cancel its actual RPC future
    /// on mute/Stop; this allocation cannot revoke already committed bytes.
    pub(in crate::voice) fn into_pcm(self) -> Result<Bytes, CaptureError> {
        if !self.permitted() {
            return Err(CaptureError::Muted);
        }
        Ok(Bytes::from_owner(PcmOwner(self)))
    }
}

struct PcmOwner(Frame);

impl AsRef<[u8]> for PcmOwner {
    fn as_ref(&self) -> &[u8] {
        self.0.pcm()
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests {
    use super::*;
    use crate::audio::{CAPTURE_QUEUE_BYTES, testing::CaptureFixture};
    use vessel_app_view::{audio::CapturePurpose, voice::Mute};

    #[test]
    fn queued_payload_aliases_retain_capture_budget_after_caller_and_stop() {
        let mut device = CaptureFixture::new(CapturePurpose::Conversation);
        let lease = device.lease();
        let frame = device.frame();
        let count = frame.pcm().len();
        let address = frame.pcm().as_ptr();
        let payload = Frame::from_capture(lease, frame)
            .unwrap()
            .into_pcm()
            .unwrap();
        assert_eq!(payload.as_ptr(), address, "PCM was not copied");
        let queued = payload.clone();
        let alias = queued.slice(..2);
        drop(payload);
        device.fence().close();
        assert_eq!(device.available(), CAPTURE_QUEUE_BYTES - count);
        // Stop fences future sends, but must not count retained request memory
        // as free. Even a short alias keeps the complete captured owner alive.
        drop(queued);
        assert_eq!(device.available(), CAPTURE_QUEUE_BYTES - count);
        drop(alias);
        assert_eq!(device.available(), CAPTURE_QUEUE_BYTES);
    }

    #[test]
    fn mute_before_payload_conversion_discards_audio_and_its_budget() {
        let mut device = CaptureFixture::new(CapturePurpose::Conversation);
        let lease = device.lease();
        let frame = Frame::from_capture(lease, device.frame()).unwrap();
        device.mute(Mute::Muted);
        device.mute(Mute::Open);
        assert!(matches!(frame.into_pcm(), Err(CaptureError::Muted)));
        assert_eq!(device.available(), CAPTURE_QUEUE_BYTES);
    }
}
