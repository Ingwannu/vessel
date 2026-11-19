//! A notice's sound (ADR 0114): the page's two tones, played on the
//! system's default output.
//!
//! [Decision Log]
//! - Purpose: Settings' "Sound only" and "Notifications with sound". The
//!   system's notification is silent on Linux and Windows (the GPUI
//!   patches send no sound hint and mark the toast's audio silent), and
//!   "Sound only" shows no notification at all.
//! - Existing assets: the voice track's `cpal` 0.16 in link (ADR 0080),
//!   with the same exact pin. Link owns the machine's audio there too. The
//!   samples are view's (`notices::samples`), so they are tested without a
//!   device.
//! - Alternatives: the system notification's own sound (it cannot sound
//!   without a notification, and it is the release track's patch); running
//!   `paplay`/`pw-play` (not on every desktop, and not on Windows).
//! - Choice: one short thread per sound. It opens the default output in
//!   its own format, plays the samples once, waits at most two seconds for
//!   them to end, and closes. No device stays open between sounds.
//! - Failures (no device, a format cpal cannot write) are logged and never
//!   shown: a missing sound is not worth a dialog.
//! - Linux, Windows: `cpal`'s ALSA and WASAPI. Elsewhere `play` does
//!   nothing (see `lib.rs`).

use std::sync::mpsc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use vessel_app_view::notices::{NoticeKind, samples};

/// How long the output may take to open.
const OPEN_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a sound may hold its thread: the tones last 0.41 s.
const LONGEST: Duration = Duration::from_secs(2);
/// Time for the device's buffer to drain once the last sample is taken.
const DRAIN: Duration = Duration::from_millis(150);

/// Why a sound did not play.
#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("no output device")]
    NoDevice,
    #[error("the output's format could not be read")]
    Config,
    #[error("the output's sample format is not one cpal writes")]
    Format,
    #[error("the output stream did not open")]
    Open,
    #[error("the output stream did not start")]
    Play,
}

/// Play `kind`'s sound once, on a thread of its own.
pub fn play(kind: NoticeKind) {
    let started = std::thread::Builder::new()
        .name("vessel-notice-sound".to_owned())
        .spawn(move || {
            if let Err(why) = play_now(kind) {
                tracing::debug!(%why, "the notice's sound did not play");
            }
        });
    if let Err(error) = started {
        tracing::debug!(%error, "no thread for the notice's sound");
    }
}

fn play_now(kind: NoticeKind) -> Result<(), Failure> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(Failure::NoDevice)?;
    let config = device
        .default_output_config()
        .map_err(|_private| Failure::Config)?;
    let (done, ended) = mpsc::sync_channel(1);
    let frames = Frames {
        sound: samples(kind, config.sample_rate().0).into_iter(),
        channels: usize::from(config.channels()).max(1),
        done: Some(done),
    };
    let stream = build(&device, &config, frames)?;
    stream.play().map_err(|_private| Failure::Play)?;
    if ended.recv_timeout(LONGEST).is_err() {
        tracing::debug!("the notice's sound did not end in time");
    }
    std::thread::sleep(DRAIN);
    drop(stream);
    Ok(())
}

/// The samples, written to every channel of each frame, then silence.
struct Frames {
    sound: std::vec::IntoIter<f64>,
    channels: usize,
    done: Option<mpsc::SyncSender<()>>,
}

impl Frames {
    fn fill<T: cpal::SizedSample + cpal::FromSample<f64>>(&mut self, out: &mut [T]) {
        for frame in out.chunks_mut(self.channels) {
            let value = self.sound.next();
            if value.is_none()
                && let Some(done) = self.done.take()
            {
                let _sent = done.try_send(());
            }
            frame.fill(<T as cpal::FromSample<f64>>::from_sample_(
                value.unwrap_or(0.0),
            ));
        }
    }
}

fn build(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    mut frames: Frames,
) -> Result<cpal::Stream, Failure> {
    let stream_config = config.config();
    macro_rules! output {
        ($sample:ty) => {
            device.build_output_stream::<$sample, _, _>(
                &stream_config,
                move |out, _info| frames.fill(out),
                |error| tracing::debug!(%error, "the notice's sound stopped"),
                Some(OPEN_TIMEOUT),
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
        _ => return Err(Failure::Format),
    };
    stream.map_err(|_private| Failure::Open)
}
