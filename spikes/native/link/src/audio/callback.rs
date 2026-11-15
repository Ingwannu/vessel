//! Device callbacks batch exact 10 ms frames without waiting on the consumer.
//!
//! Each callback keeps at most one partial block. Overflow closes the same
//! privacy fence as local Stop, independently of the saturated data channel.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc;

use super::format::Format;
use crate::cancel::Fence;

const CALLBACK_QUEUE_BLOCKS: usize = 100;

pub(super) struct Samples(Vec<f32>);

impl Samples {
    pub(super) fn as_slice(&self) -> &[f32] {
        &self.0
    }
}

impl fmt::Debug for Samples {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Samples")
            .field("count", &self.0.len())
            .finish()
    }
}

#[derive(Debug)]
pub(super) struct Callback {
    fence: Arc<Fence>,
    overrun: Arc<AtomicBool>,
    sender: mpsc::Sender<Samples>,
    partial: Vec<f32>,
    block_samples: usize,
}

impl Callback {
    pub(super) fn channel(format: Format, fence: Arc<Fence>) -> (Self, mpsc::Receiver<Samples>) {
        let (sender, receiver) = mpsc::channel(CALLBACK_QUEUE_BLOCKS);
        let block_samples = format
            .frames()
            .get()
            .saturating_mul(usize::from(format.channels().get()));
        (
            Self {
                fence,
                overrun: Arc::new(AtomicBool::new(false)),
                sender,
                partial: Vec::with_capacity(block_samples),
                block_samples,
            },
            receiver,
        )
    }

    pub(super) fn overrun(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.overrun)
    }

    pub(super) fn accept<T>(&mut self, samples: &[T])
    where
        T: cpal::Sample + Copy,
        f32: cpal::FromSample<T>,
    {
        for sample in samples {
            if self.fence.is_closed() {
                self.partial.clear();
                return;
            }
            self.partial.push(sample.to_sample::<f32>());
            if self.partial.len() == self.block_samples {
                let block =
                    std::mem::replace(&mut self.partial, Vec::with_capacity(self.block_samples));
                if self.sender.try_send(Samples(block)).is_err() {
                    self.overrun.store(true, Ordering::Release);
                    self.fence.close();
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
