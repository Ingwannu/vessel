//! Capture-only processing built on the shared native input-stream owner.
//!
//! Raw device construction and callback delivery live in input. Keeping DSP
//! outside that stream permits duplex to share its single processor with the
//! actual output reference, without opening a second microphone engine.

mod input;
pub(super) use input::Input;

use super::{
    budget::Budget,
    capture::Held,
    dsp::Processor,
    format::{Channels, Format, RateHz},
    owner::DeviceFailure,
    upload_access::Gate,
};
use crate::cancel::Fence;
use std::sync::Arc;

pub(super) trait CaptureDevice {
    fn start(&self) -> Result<(), DeviceFailure>;
    async fn receive(&mut self) -> Result<Held, DeviceFailure>;
    fn failure(&self) -> Option<DeviceFailure>;
}

pub(super) struct Native {
    input: Input,
    processor: Processor,
}
impl std::fmt::Debug for Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Native").finish_non_exhaustive()
    }
}
impl Native {
    pub(super) fn open(
        fence: Arc<Fence>,
        budget: Budget,
        upload: Gate,
    ) -> Result<Self, DeviceFailure> {
        let input = Input::open(fence, budget, upload)?;
        let processor = Processor::capture_only(input.format())
            .map_err(|_private| DeviceFailure::Processing)?;
        Ok(Self { input, processor })
    }
}
impl CaptureDevice for Native {
    fn start(&self) -> Result<(), DeviceFailure> {
        self.input.start()
    }
    async fn receive(&mut self) -> Result<Held, DeviceFailure> {
        let frame = self.input.receive().await?;
        let (samples, permit, access) = frame.into_parts();
        self.processor
            .capture(&samples)
            .map(|audio| Held::new(audio, permit, access))
            .map_err(|_private| DeviceFailure::Processing)
    }
    fn failure(&self) -> Option<DeviceFailure> {
        self.input.failure()
    }
}

pub(super) fn validated_format(
    config: &cpal::SupportedStreamConfig,
) -> Result<Format, DeviceFailure> {
    let rate = RateHz::new(config.sample_rate().0).map_err(|_private| DeviceFailure::Format)?;
    let channels =
        Channels::try_from(config.channels()).map_err(|_private| DeviceFailure::Format)?;
    Format::new(rate, channels).map_err(|_private| DeviceFailure::Format)
}
