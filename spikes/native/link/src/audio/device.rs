//! The native stream and DSP remain on one device thread; no GPUI or keys cross it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tokio::sync::mpsc;

use super::callback::{Callback, Samples};
use super::dsp::{Captured, Processor};
use super::format::{Channels, Format, RateHz};
use super::owner::DeviceFailure;
use crate::cancel::Fence;

const DEVICE_OPEN_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) trait CaptureDevice {
    fn start(&self) -> Result<(), DeviceFailure>;
    fn receive(&mut self) -> Result<Option<Captured>, DeviceFailure>;
    fn failure(&self) -> Option<DeviceFailure>;
}

pub(super) struct Native {
    stream: cpal::Stream,
    frames: mpsc::Receiver<Samples>,
    processor: Processor,
    overrun: Arc<AtomicBool>,
    interrupted: Arc<AtomicBool>,
}

impl std::fmt::Debug for Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Native").finish_non_exhaustive()
    }
}

impl Native {
    pub(super) fn open(fence: Arc<Fence>) -> Result<Self, DeviceFailure> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or(DeviceFailure::Unavailable)?;
        let config = device
            .default_input_config()
            .map_err(|_private| DeviceFailure::Unavailable)?;
        let rate = RateHz::new(config.sample_rate().0).map_err(|_private| DeviceFailure::Format)?;
        let channels =
            Channels::try_from(config.channels()).map_err(|_private| DeviceFailure::Format)?;
        let format = Format::new(rate, channels).map_err(|_private| DeviceFailure::Format)?;
        let processor =
            Processor::new(format, format).map_err(|_private| DeviceFailure::Processing)?;
        let (callback, frames) = Callback::channel(format, Arc::clone(&fence));
        let overrun = callback.overrun();
        let interrupted = Arc::new(AtomicBool::new(false));
        let stream = build(&device, &config, callback, fence, Arc::clone(&interrupted))?;
        Ok(Self {
            stream,
            frames,
            processor,
            overrun,
            interrupted,
        })
    }
}

impl CaptureDevice for Native {
    fn start(&self) -> Result<(), DeviceFailure> {
        self.stream
            .play()
            .map_err(|_private| DeviceFailure::Unavailable)
    }

    fn receive(&mut self) -> Result<Option<Captured>, DeviceFailure> {
        match self.frames.try_recv() {
            Ok(frame) => self
                .processor
                .capture(frame.as_slice())
                .map(Some)
                .map_err(|_private| DeviceFailure::Processing),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => Err(DeviceFailure::Interrupted),
        }
    }

    fn failure(&self) -> Option<DeviceFailure> {
        if self.interrupted.load(Ordering::Acquire) {
            Some(DeviceFailure::Interrupted)
        } else if self.overrun.load(Ordering::Acquire) {
            Some(DeviceFailure::Overrun)
        } else {
            None
        }
    }
}

fn build(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    mut callback: Callback,
    fence: Arc<Fence>,
    interrupted: Arc<AtomicBool>,
) -> Result<cpal::Stream, DeviceFailure> {
    let on_error = move |_error| {
        interrupted.store(true, Ordering::Release);
        fence.close();
    };
    // No foreign device/error Debug text is retained. The same normalized
    // converter handles each supported hardware sample representation.
    macro_rules! input {
        ($sample:ty) => {
            device.build_input_stream::<$sample, _, _>(
                &config.config(),
                move |samples, _info| callback.accept(samples),
                on_error,
                Some(DEVICE_OPEN_TIMEOUT),
            )
        };
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => input!(i8),
        cpal::SampleFormat::I16 => input!(i16),
        cpal::SampleFormat::I24 => input!(cpal::I24),
        cpal::SampleFormat::I32 => input!(i32),
        cpal::SampleFormat::I64 => input!(i64),
        cpal::SampleFormat::U8 => input!(u8),
        cpal::SampleFormat::U16 => input!(u16),
        cpal::SampleFormat::U32 => input!(u32),
        cpal::SampleFormat::U64 => input!(u64),
        cpal::SampleFormat::F32 => input!(f32),
        cpal::SampleFormat::F64 => input!(f64),
        _ => return Err(DeviceFailure::Format),
    };
    stream.map_err(|_private| DeviceFailure::Unavailable)
}
