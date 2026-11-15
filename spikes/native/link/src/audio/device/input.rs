//! The one native input-stream constructor, shared by capture and future duplex.
//!
//! It owns raw callback frames only; the enclosing device thread owns the DSP
//! and destroys this stream before retiring its process-wide microphone guard.

use super::{DEVICE_OPEN_TIMEOUT, DeviceFailure, validated_format};
use crate::{
    audio::{
        budget::Budget,
        callback::{Callback, Samples},
        format::Format,
        upload_access::Gate,
    },
    cancel::Fence,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

pub(in crate::audio) struct Input {
    stream: cpal::Stream,
    frames: mpsc::Receiver<Samples>,
    format: Format,
    overrun: Arc<AtomicBool>,
    interrupted: Arc<AtomicBool>,
}
impl Input {
    pub(in crate::audio) fn open(
        fence: Arc<Fence>,
        budget: Budget,
        upload: Gate,
    ) -> Result<Self, DeviceFailure> {
        if fence.is_closed() {
            return Err(DeviceFailure::Interrupted);
        }
        let device = cpal::default_host()
            .default_input_device()
            .ok_or(DeviceFailure::Unavailable)?;
        let config = device
            .default_input_config()
            .map_err(|_private| DeviceFailure::Unavailable)?;
        let format = validated_format(&config)?;
        let (callback, frames) = Callback::channel(format, Arc::clone(&fence), budget, upload);
        let overrun = callback.overrun();
        let interrupted = Arc::new(AtomicBool::new(false));
        let stream = build(&device, &config, callback, fence, Arc::clone(&interrupted))?;
        Ok(Self {
            stream,
            frames,
            format,
            overrun,
            interrupted,
        })
    }
    pub(in crate::audio) const fn format(&self) -> Format {
        self.format
    }
    pub(in crate::audio) fn start(&self) -> Result<(), DeviceFailure> {
        self.stream
            .play()
            .map_err(|_private| DeviceFailure::Unavailable)
    }
    pub(in crate::audio) async fn receive(&mut self) -> Result<Samples, DeviceFailure> {
        self.frames.recv().await.ok_or(DeviceFailure::Interrupted)
    }
    pub(in crate::audio) fn failure(&self) -> Option<DeviceFailure> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_already_stopped_acquisition_refuses_before_native_device_discovery() {
        let fence = Arc::new(Fence::default());
        fence.close();
        assert!(matches!(
            Input::open(Arc::clone(&fence), Budget::default(), Gate::new(fence)),
            Err(DeviceFailure::Interrupted)
        ));
    }
}
