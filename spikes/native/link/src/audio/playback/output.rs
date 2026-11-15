//! Speaker callback: write fenced prepared samples, then publish the actual AEC reference.
//!
//! Neither prepared blocks nor queued segment tails are a playback receipt.
//! Completion carries the callback's predicted DAC drain time to the owner.

use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
    vec::IntoIter,
};

use cpal::Sample as _;
use tokio::sync::mpsc;

use super::{
    packet::Reader,
    receipts::{Lease, Receipt},
};
use crate::{
    audio::{AudioError, dsp::Rendered, format::Format},
    cancel::Fence,
};

const OUTPUT_BLOCKS: usize = 100;
const REFERENCE_BLOCKS: usize = 100;
const COMPLETION_EVENTS: usize = 2;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

pub(in crate::audio) struct Prepared {
    samples: IntoIter<f32>,
    lease: Lease,
    receipt: Arc<Receipt>,
    last: bool,
}
impl fmt::Debug for Prepared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedOutput")
            .field("samples", &self.samples.len())
            .field("lease", &self.lease)
            .finish_non_exhaustive()
    }
}
impl Prepared {
    pub(in crate::audio) fn new(
        audio: Rendered,
        reader: &Reader,
        last: bool,
        format: Format,
    ) -> Result<Self, AudioError> {
        let samples = audio.into_samples();
        if samples.len()
            != format
                .frames()
                .get()
                .saturating_mul(usize::from(format.channels().get()))
        {
            return Err(AudioError::Frame);
        }
        Ok(Self {
            samples: samples.into_iter(),
            lease: reader.lease(),
            receipt: reader.receipt(),
            last,
        })
    }
}

pub(in crate::audio) struct Reference(Vec<f32>);
impl Reference {
    pub(in crate::audio) fn samples(&self) -> &[f32] {
        &self.0
    }
}
impl fmt::Debug for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputReference")
            .field("samples", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::audio) struct Scheduled {
    pub(in crate::audio) lease: Lease,
    pub(in crate::audio) now: Instant,
    pub(in crate::audio) after: Option<Duration>,
}

#[derive(Debug)]
pub(in crate::audio) struct Callback {
    fence: Arc<Fence>,
    format: Format,
    blocks: mpsc::Receiver<Prepared>,
    references: mpsc::Sender<Reference>,
    completions: mpsc::Sender<Scheduled>,
    current: Option<Prepared>,
    partial: Vec<f32>,
}

#[derive(Debug)]
pub(in crate::audio) struct Ports {
    pub(in crate::audio) blocks: mpsc::Sender<Prepared>,
    pub(in crate::audio) references: mpsc::Receiver<Reference>,
    pub(in crate::audio) completions: mpsc::Receiver<Scheduled>,
}

impl Callback {
    pub(in crate::audio) fn channel(format: Format, fence: Arc<Fence>) -> (Self, Ports) {
        let (blocks, receiver) = mpsc::channel(OUTPUT_BLOCKS);
        let (references, reference_rx) = mpsc::channel(REFERENCE_BLOCKS);
        let (completions, completion_rx) = mpsc::channel(COMPLETION_EVENTS);
        let samples = format
            .frames()
            .get()
            .saturating_mul(usize::from(format.channels().get()));
        (
            Self {
                fence,
                format,
                blocks: receiver,
                references,
                completions,
                current: None,
                partial: Vec::with_capacity(samples),
            },
            Ports {
                blocks,
                references: reference_rx,
                completions: completion_rx,
            },
        )
    }

    pub(in crate::audio) fn render<T>(&mut self, output: &mut [T], info: &cpal::OutputCallbackInfo)
    where
        T: cpal::Sample + cpal::FromSample<f32>,
        f32: cpal::FromSample<T>,
    {
        let time = info.timestamp();
        self.write(
            output,
            Instant::now(),
            time.playback.duration_since(&time.callback),
        );
    }

    fn write<T>(&mut self, output: &mut [T], now: Instant, latency: Option<Duration>)
    where
        T: cpal::Sample + cpal::FromSample<f32>,
        f32: cpal::FromSample<T>,
    {
        if !output
            .len()
            .is_multiple_of(usize::from(self.format.channels().get()))
        {
            output.fill(T::EQUILIBRIUM);
            self.fence.close();
            return;
        }
        for (index, target) in output.iter_mut().enumerate() {
            if self.fence.is_closed() {
                *target = T::EQUILIBRIUM;
                self.current = None;
                self.partial.clear();
                continue;
            }
            let (sample, completed) = self.sample();
            *target = sample.clamp(-1.0, 1.0_f32.next_down()).to_sample::<T>();
            self.reference(target.to_sample::<f32>());
            if let Some(lease) = completed {
                let after = latency.and_then(|delay| {
                    self.offset(index)
                        .and_then(|offset| delay.checked_add(offset))
                });
                if self
                    .completions
                    .try_send(Scheduled { lease, now, after })
                    .is_err()
                {
                    self.fence.close();
                }
            }
        }
    }

    fn sample(&mut self) -> (f32, Option<Lease>) {
        // A producer cannot keep a callback busy indefinitely with cancelled blocks.
        for _ in 0..OUTPUT_BLOCKS {
            if self
                .current
                .as_ref()
                .is_some_and(|block| block.receipt.stopped())
            {
                self.current = None;
            }
            if self.current.is_none() {
                self.current = self.blocks.try_recv().ok();
                if self.current.is_none() {
                    return (0.0, None);
                }
                continue;
            }
            let Some(block) = self.current.as_mut() else {
                return (0.0, None);
            };
            let Some(sample) = block.samples.next() else {
                self.current = None;
                continue;
            };
            if block.samples.as_slice().is_empty() {
                let completed = block.last.then_some(block.lease);
                self.current = None;
                return (sample, completed);
            }
            return (sample, None);
        }
        (0.0, None)
    }

    fn reference(&mut self, sample: f32) {
        self.partial.push(sample);
        let block_samples = self
            .format
            .frames()
            .get()
            .saturating_mul(usize::from(self.format.channels().get()));
        if self.partial.len() == block_samples {
            let samples = std::mem::replace(&mut self.partial, Vec::with_capacity(block_samples));
            if self.references.try_send(Reference(samples)).is_err() {
                self.fence.close();
            }
        }
    }

    fn offset(&self, index: usize) -> Option<Duration> {
        let samples = index.checked_add(1)?;
        let channels = usize::from(self.format.channels().get());
        let frames = samples.checked_div(channels)?;
        let rate = u64::from(self.format.rate().get());
        let nanos = u64::try_from(frames)
            .ok()?
            .checked_mul(NANOS_PER_SECOND)?
            .checked_div(rate)?;
        Some(Duration::from_nanos(nanos))
    }
}

#[cfg(test)]
mod tests;
