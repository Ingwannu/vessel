//! An owner guard is retired only after its stream bundle has been destroyed.
//!
//! Stop uses the atomic callback fence, not this thread's data queue. A stuck
//! driver can retain ownership but cannot authorize another acquisition.

use std::sync::Arc;

use tokio::sync::mpsc;
use vessel_app_view::audio::CaptureLease;

use super::device::{CaptureDevice, Native};
use super::dsp::Captured;
use super::owner::{DeviceFailure, Owner};
use crate::cancel::Fence;

const NORMALIZED_QUEUE_BLOCKS: usize = 100;

#[derive(Debug)]
pub(super) struct Frame {
    lease: CaptureLease,
    audio: Captured,
}

impl Frame {
    pub(super) const fn lease(&self) -> CaptureLease {
        self.lease
    }
    pub(super) fn pcm(&self) -> &[u8] {
        self.audio.pcm()
    }
}

pub(super) fn start(owner: Owner) -> Result<mpsc::Receiver<Frame>, DeviceFailure> {
    let (sender, receiver) = mpsc::channel(NORMALIZED_QUEUE_BLOCKS);
    let _thread = std::thread::Builder::new()
        .name("vessel-audio".to_owned())
        .spawn(move || run(owner, &sender, Native::open))
        .map_err(|_private| DeviceFailure::Thread)?;
    Ok(receiver)
}

fn run<D: CaptureDevice>(
    mut owner: Owner,
    sender: &mpsc::Sender<Frame>,
    open: impl FnOnce(Arc<Fence>) -> Result<D, DeviceFailure>,
) {
    if owner.stopped() {
        return;
    }
    match open(owner.fence()) {
        Ok(mut device) => {
            let result = match tokio::runtime::Builder::new_current_thread().build() {
                Ok(runtime) => runtime.block_on(capture(&owner, sender, &mut device)),
                Err(_private) => Err(DeviceFailure::Thread),
            };
            if let Err(failure) = result {
                owner.fail(failure);
            } else if let Some(failure) = device.failure() {
                owner.fail(failure);
            }
            // A slow destructor keeps the global slot held. Never publish a
            // release just because the actor has exited its capture loop.
            drop(device);
        }
        Err(failure) => owner.fail(failure),
    }
    drop(owner);
}

async fn capture(
    owner: &Owner,
    sender: &mpsc::Sender<Frame>,
    device: &mut impl CaptureDevice,
) -> Result<(), DeviceFailure> {
    if owner.stopped() {
        return Ok(());
    }
    device.start()?;
    if owner.started().is_err() {
        return Ok(());
    }
    let fence = owner.fence();
    loop {
        let audio = tokio::select! {
            biased;
            () = fence.closed() => break,
            () = sender.closed() => break,
            received = device.receive() => received?,
        };
        if owner.stopped() {
            break;
        }
        sender
            .try_send(Frame {
                lease: owner.lease(),
                audio,
            })
            .map_err(|_private| DeviceFailure::Overrun)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
