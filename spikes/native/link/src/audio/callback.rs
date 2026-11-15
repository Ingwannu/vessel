//! Device callbacks batch exact 10 ms frames without waiting on the consumer.
//!
//! Each callback keeps at most one partial block. Overflow closes the same
//! privacy fence as local Stop, independently of the saturated data channel.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc;

use super::budget::{Budget, Permit};
use super::format::Format;
use super::upload_access::{Access, Gate};
use crate::cancel::Fence;

const CALLBACK_QUEUE_BLOCKS: usize = 100;

pub(super) struct Samples {
    values: Vec<f32>,
    permit: Permit,
    access: Access,
}

impl Samples {
    #[cfg(test)]
    pub(super) fn as_slice(&self) -> &[f32] {
        &self.values
    }
    pub(super) fn into_parts(self) -> (Vec<f32>, Permit, Access) {
        (self.values, self.permit, self.access)
    }
}

impl fmt::Debug for Samples {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Samples")
            .field("count", &self.values.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(super) struct Callback {
    fence: Arc<Fence>,
    overrun: Arc<AtomicBool>,
    sender: mpsc::Sender<Samples>,
    partial: Vec<f32>,
    block_samples: usize,
    budget: Budget,
    permit: Option<Permit>,
    upload: Gate,
    access: Option<Access>,
}

impl Callback {
    pub(super) fn channel(
        format: Format,
        fence: Arc<Fence>,
        budget: Budget,
        upload: Gate,
    ) -> (Self, mpsc::Receiver<Samples>) {
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
                budget,
                permit: None,
                upload,
                access: None,
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
                drop(self.permit.take());
                drop(self.access.take());
                return;
            }
            if self.permit.is_none() && !self.begin_block() {
                return;
            }
            self.partial.push(sample.to_sample::<f32>());
            if self.partial.len() == self.block_samples && !self.send_block() {
                return;
            }
        }
    }

    fn begin_block(&mut self) -> bool {
        match self.budget.reserve() {
            Ok(permit) => {
                self.permit = Some(permit);
                self.access = Some(self.upload.snapshot());
                true
            }
            Err(_capacity) => {
                self.overrun.store(true, Ordering::Release);
                self.fence.close();
                false
            }
        }
    }

    fn send_block(&mut self) -> bool {
        let values = std::mem::replace(&mut self.partial, Vec::with_capacity(self.block_samples));
        let Some(permit) = self.permit.take() else {
            self.fence.close();
            return false;
        };
        let Some(access) = self.access.take() else {
            self.fence.close();
            return false;
        };
        if self
            .sender
            .try_send(Samples {
                values,
                permit,
                access,
            })
            .is_err()
        {
            self.overrun.store(true, Ordering::Release);
            self.fence.close();
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests;
