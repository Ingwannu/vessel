//! The device worker prepares finite output without waiting behind its data queue.
//!
//! One worker borrows its existing processor for capture, preparation and actual
//! render references; callbacks never own a second AEC processor. Readiness and
//! native stream construction remain the enclosing owner's responsibility.

use std::{collections::VecDeque, sync::Arc, time::Instant};

use tokio::sync::mpsc;
use vessel_app_view::audio::CaptureLease;

use super::{
    output::{Prepared, Reference, Scheduled},
    packet::{Packet, Reader},
    receipts::{Book, Handle, Lease, MAX_SEGMENTS, Refusal},
};
use crate::{
    audio::{AudioError, dsp::Processor, format::Format},
    cancel::Fence,
};

// Yield to capture/control after a bounded amount of preparation, even if a
// callback consumes blocks concurrently and continuously creates more space.
const PREPARE_BLOCKS_PER_TURN: usize = 100;

#[derive(Debug, thiserror::Error)]
pub(in crate::audio) enum Failure {
    #[error(transparent)]
    Receipt(#[from] Refusal),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error("the speaker callback is unavailable")]
    Disconnected,
}

#[derive(Debug)]
pub(in crate::audio) struct Pump {
    book: Book,
    pending: VecDeque<Lease>,
    current: Option<Reader>,
    fence: Arc<Fence>,
    output: Format,
}
impl Pump {
    pub(in crate::audio) fn new(
        lease: CaptureLease,
        fence: Arc<Fence>,
        output: Format,
    ) -> Result<Self, Refusal> {
        Ok(Self {
            book: Book::new(lease, Arc::clone(&fence))?,
            pending: VecDeque::new(),
            current: None,
            fence,
            output,
        })
    }
    pub(in crate::audio) fn submit(&mut self, packet: Packet) -> Result<Handle, Refusal> {
        self.advance(Instant::now());
        let handle = self.book.submit(packet)?;
        self.pending.push_back(handle.lease());
        Ok(handle)
    }
    pub(in crate::audio) fn interrupt(&mut self) {
        self.book.interrupt_all();
        self.current = None;
        self.pending.clear();
    }
    pub(in crate::audio) fn advance(&mut self, now: Instant) {
        if self.fence.is_closed() {
            self.interrupt();
        } else {
            self.book.advance(now);
            self.pending.retain(|lease| self.book.queued(*lease));
            if self.current.as_ref().is_some_and(Reader::stopped) {
                self.current = None;
            }
        }
    }
    pub(in crate::audio) fn next_deadline(&self) -> Option<Instant> {
        self.book.next_deadline()
    }
    pub(in crate::audio) fn reference(
        &mut self,
        processor: &mut Processor,
        reference: &Reference,
    ) -> Result<(), Failure> {
        if self.fence.is_closed() {
            return Err(Refusal::Stopped.into());
        }
        processor.reference_render(reference.samples())?;
        Ok(())
    }
    pub(in crate::audio) fn scheduled(&mut self, event: Scheduled) -> Result<(), Refusal> {
        self.book.scheduled(event.lease, event.now, event.after)
    }
    /// A full data queue yields immediately; the worker can still stop/capture.
    pub(in crate::audio) fn prepare(
        &mut self,
        processor: &mut Processor,
        blocks: &mpsc::Sender<Prepared>,
    ) -> Result<(), Failure> {
        self.advance(Instant::now());
        if self.fence.is_closed() {
            return Err(Refusal::Stopped.into());
        }
        for _ in 0..PREPARE_BLOCKS_PER_TURN {
            let permit = match blocks.try_reserve() {
                Ok(permit) => permit,
                Err(mpsc::error::TrySendError::Full(())) => return Ok(()),
                Err(mpsc::error::TrySendError::Closed(())) => return Err(Failure::Disconnected),
            };
            let Some(prepared) = self.next(processor)? else {
                return Ok(());
            };
            permit.send(prepared);
        }
        Ok(())
    }
    fn next(&mut self, processor: &mut Processor) -> Result<Option<Prepared>, Failure> {
        for _ in 0..MAX_SEGMENTS {
            if self.current.as_ref().is_some_and(Reader::stopped) {
                self.current = None;
            }
            if self.current.is_none() {
                let Some(lease) = self.pending.pop_front() else {
                    return Ok(None);
                };
                match self.book.begin(lease) {
                    Ok(Some(reader)) => self.current = Some(reader),
                    Ok(None) | Err(Refusal::Stopped) => continue,
                    Err(error) => return Err(error.into()),
                }
            }
            let Some(reader) = self.current.as_mut() else {
                return Ok(None);
            };
            let block = match reader.next() {
                Ok(Some(block)) => block,
                Err(Refusal::Stopped) => {
                    self.current = None;
                    continue;
                }
                Err(error) => return Err(error.into()),
                // Last blocks retire readers below, so exhaustion here would
                // indicate broken packet/worker ordering.
                Ok(None) => return Err(Refusal::Order.into()),
            };
            let audio = processor.prepare_render(block.mono(), reader.rate())?;
            let prepared = Prepared::new(audio, reader, block.last(), self.output)?;
            if block.last() {
                self.current = None;
            }
            return Ok(Some(prepared));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
