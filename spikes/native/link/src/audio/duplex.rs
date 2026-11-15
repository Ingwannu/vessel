//! Both streams and their single capture/render processor live on the device thread.
//!
//! The bundle implements the existing capture-device boundary. The enclosing
//! worker retains its microphone guard until this entire bundle is dropped.
//! Conversation entry/commands remain staged; these sources open no test device.

mod output;

use super::{
    callback::Samples,
    capture::{DeviceFailure, Held},
    device::{CaptureDevice, Input},
    dsp::Processor,
    format::Format,
    owner::Owner,
    playback::{
        output::Ports,
        packet::Packet,
        pump::{Failure, Pump},
        receipts::{Handle, Refusal},
    },
};
use crate::cancel::Fence;
use output::Output;
use std::{fmt, sync::Arc, time::Instant};
use vessel_app_view::audio::CaptureLease;

trait CaptureSource {
    fn format(&self) -> Format;
    fn start(&self) -> Result<(), DeviceFailure>;
    fn failure(&self) -> Option<DeviceFailure>;
    async fn receive(&mut self) -> Result<Samples, DeviceFailure>;
}
impl CaptureSource for Input {
    fn format(&self) -> Format {
        Input::format(self)
    }
    fn start(&self) -> Result<(), DeviceFailure> {
        Input::start(self)
    }
    fn failure(&self) -> Option<DeviceFailure> {
        Input::failure(self)
    }
    async fn receive(&mut self) -> Result<Samples, DeviceFailure> {
        Input::receive(self).await
    }
}
trait SpeakerSource {
    fn format(&self) -> Format;
    fn start(&self) -> Result<(), DeviceFailure>;
    fn failure(&self) -> Option<DeviceFailure>;
}

struct Streams<I: CaptureSource, O: SpeakerSource> {
    // Keep both physical streams ahead of processor/book fields in drop order;
    // the enclosing thread drops the whole bundle before retiring its guard.
    input: I,
    output: O,
    processor: Processor,
    pump: Pump,
    ports: Ports,
    fence: Arc<Fence>,
}
impl<I: CaptureSource, O: SpeakerSource> fmt::Debug for Streams<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DuplexStreams")
            .field("pump", &self.pump)
            .finish_non_exhaustive()
    }
}
impl Streams<Input, Output> {
    fn open(owner: &Owner) -> Result<Self, DeviceFailure> {
        if owner.stopped() {
            return Err(DeviceFailure::Interrupted);
        }
        let fence = owner.fence();
        let input = Input::open(Arc::clone(&fence), owner.budget(), owner.upload_gate())?;
        let (output, ports) = Output::open(Arc::clone(&fence))?;
        Self::new(owner.lease(), fence, input, output, ports)
    }
}
impl<I: CaptureSource, O: SpeakerSource> Streams<I, O> {
    fn new(
        lease: CaptureLease,
        fence: Arc<Fence>,
        input: I,
        output: O,
        ports: Ports,
    ) -> Result<Self, DeviceFailure> {
        let processor = Processor::new(input.format(), output.format())
            .map_err(|_private| DeviceFailure::Processing)?;
        let pump = Pump::new(lease, Arc::clone(&fence), output.format())
            .map_err(|_private| DeviceFailure::Processing)?;
        Ok(Self {
            input,
            output,
            processor,
            pump,
            ports,
            fence,
        })
    }
    fn submit(&mut self, packet: Packet) -> Result<Handle, Refusal> {
        self.pump.submit(packet)
    }
    fn interrupt(&mut self) {
        self.pump.interrupt();
    }
    async fn next(&mut self) -> Result<Held, DeviceFailure> {
        loop {
            self.pump.advance(Instant::now());
            self.pump
                .prepare(&mut self.processor, &self.ports.blocks)
                .map_err(playback_failure)?;
            let deadline = self.pump.next_deadline();
            tokio::select! { biased;
                () = self.fence.closed() => return Err(DeviceFailure::Interrupted),
                reference = self.ports.references.recv() => {
                    let reference = reference.ok_or(DeviceFailure::Interrupted)?;
                    self.pump.reference(&mut self.processor, &reference).map_err(playback_failure)?;
                },
                completed = self.ports.completions.recv() => {
                    let completed = completed.ok_or(DeviceFailure::Interrupted)?;
                    // Cancelled/retired tails may arrive after a barge-in. Their
                    // receipt is already fenced and must not fail a new stream.
                    match self.pump.scheduled(completed) {
                        Ok(()) | Err(Refusal::Stopped) => {},
                        Err(_private) => return Err(DeviceFailure::Interrupted),
                    }
                },
                () = wait_deadline(deadline) => {},
                samples = self.input.receive() => {
                    let (samples, permit, access) = samples?.into_parts();
                    let audio = self.processor.capture(&samples).map_err(|_private| DeviceFailure::Processing)?;
                    return Ok(Held::new(audio, permit, access));
                },
            }
        }
    }
}
impl<I: CaptureSource, O: SpeakerSource> CaptureDevice for Streams<I, O> {
    fn start(&self) -> Result<(), DeviceFailure> {
        if self.fence.is_closed() {
            return Err(DeviceFailure::Interrupted);
        }
        self.output
            .start()
            .inspect_err(|_private| self.fence.close())?;
        if self.fence.is_closed() {
            return Err(DeviceFailure::Interrupted);
        }
        self.input
            .start()
            .inspect_err(|_private| self.fence.close())
    }
    async fn receive(&mut self) -> Result<Held, DeviceFailure> {
        self.next().await
    }
    fn failure(&self) -> Option<DeviceFailure> {
        self.output.failure().or_else(|| self.input.failure())
    }
}
impl<I: CaptureSource, O: SpeakerSource> Drop for Streams<I, O> {
    fn drop(&mut self) {
        self.fence.close();
        self.pump.interrupt();
    }
}
async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await,
        None => std::future::pending().await,
    }
}
fn playback_failure(failure: Failure) -> DeviceFailure {
    match failure {
        Failure::Audio(_) => DeviceFailure::Processing,
        Failure::Receipt(_) | Failure::Disconnected => DeviceFailure::Interrupted,
    }
}

#[cfg(test)]
mod tests;
