//! Native speaker stream construction; playback callback owns actual samples/reference.

use super::{DeviceFailure, Format, SpeakerSource};
use crate::{
    audio::{
        device::validated_format,
        playback::output::{Callback, Ports},
    },
    cancel::Fence,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(super) struct Output {
    stream: cpal::Stream,
    format: Format,
    interrupted: Arc<AtomicBool>,
}
impl fmt::Debug for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeSpeaker")
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}
impl Output {
    pub(super) fn open(fence: Arc<Fence>) -> Result<(Self, Ports), DeviceFailure> {
        if fence.is_closed() {
            return Err(DeviceFailure::Interrupted);
        }
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(DeviceFailure::Unavailable)?;
        let config = device
            .default_output_config()
            .map_err(|_private| DeviceFailure::Unavailable)?;
        let format = validated_format(&config)?;
        let (callback, ports) = Callback::channel(format, Arc::clone(&fence));
        let interrupted = Arc::new(AtomicBool::new(false));
        let stream = build(&device, &config, callback, fence, Arc::clone(&interrupted))?;
        Ok((
            Self {
                stream,
                format,
                interrupted,
            },
            ports,
        ))
    }
}
impl SpeakerSource for Output {
    fn format(&self) -> Format {
        self.format
    }
    fn start(&self) -> Result<(), DeviceFailure> {
        self.stream
            .play()
            .map_err(|_private| DeviceFailure::Unavailable)
    }
    fn failure(&self) -> Option<DeviceFailure> {
        self.interrupted
            .load(Ordering::Acquire)
            .then_some(DeviceFailure::Interrupted)
    }
}
fn build(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    mut callback: Callback,
    fence: Arc<Fence>,
    interrupted: Arc<AtomicBool>,
) -> Result<cpal::Stream, DeviceFailure> {
    let on_error = move |_private| {
        interrupted.store(true, Ordering::Release);
        fence.close();
    };
    macro_rules! output {
        ($sample:ty) => {
            device.build_output_stream::<$sample, _, _>(
                &config.config(),
                move |samples, info| callback.render(samples, info),
                on_error,
                Some(crate::audio::device::DEVICE_OPEN_TIMEOUT),
            )
        };
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => output!(i8),
        cpal::SampleFormat::I16 => output!(i16),
        cpal::SampleFormat::I24 => output!(cpal::I24),
        cpal::SampleFormat::I32 => output!(i32),
        cpal::SampleFormat::I64 => output!(i64),
        cpal::SampleFormat::U8 => output!(u8),
        cpal::SampleFormat::U16 => output!(u16),
        cpal::SampleFormat::U32 => output!(u32),
        cpal::SampleFormat::U64 => output!(u64),
        cpal::SampleFormat::F32 => output!(f32),
        cpal::SampleFormat::F64 => output!(f64),
        _ => return Err(DeviceFailure::Format),
    };
    stream.map_err(|_private| DeviceFailure::Unavailable)
}
