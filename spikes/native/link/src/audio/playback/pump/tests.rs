use super::super::{
    output::{Callback, Ports},
    receipts::State,
};
use super::*;
use crate::audio::format::{Channels, RateHz};
use bytes::Bytes;
use std::time::Duration;
use vessel_app_view::audio::{CapturePurpose, CaptureSlot};

fn setup() -> (Pump, Processor, Callback, Ports, Arc<Fence>) {
    let lease = CaptureSlot::default()
        .reserve(CapturePurpose::Conversation)
        .unwrap();
    let format = Format::new(RateHz::new(48_000).unwrap(), Channels::Stereo).unwrap();
    let fence = Arc::new(Fence::default());
    let (callback, ports) = Callback::channel(format, Arc::clone(&fence));
    (
        Pump::new(lease, Arc::clone(&fence), format).unwrap(),
        Processor::new(format, format).unwrap(),
        callback,
        ports,
        fence,
    )
}
fn packet(blocks: usize) -> Packet {
    Packet::new(
        Bytes::from(vec![0x20; blocks.checked_mul(480).unwrap()]),
        RateHz::new(24_000).unwrap(),
    )
    .unwrap()
}
fn info() -> cpal::OutputCallbackInfo {
    cpal::OutputCallbackInfo::new(cpal::OutputStreamTimestamp {
        callback: cpal::StreamInstant::new(1, 0),
        playback: cpal::StreamInstant::new(1, 20_000_000),
    })
}

#[test]
fn preparation_callback_reference_and_dac_drain_are_distinct_steps() {
    let (mut pump, mut processor, mut callback, mut ports, _fence) = setup();
    let handle = pump.submit(packet(2)).unwrap();
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(*handle.status().borrow(), State::Rendering);
    assert_eq!(
        ports
            .blocks
            .max_capacity()
            .saturating_sub(ports.blocks.capacity()),
        2
    );
    assert!(ports.references.try_recv().is_err());
    assert!(ports.completions.try_recv().is_err());
    callback.render(&mut [0_i16; 1_920], &info());
    for _ in 0..2 {
        pump.reference(&mut processor, &ports.references.try_recv().unwrap())
            .unwrap();
    }
    let event = ports.completions.try_recv().unwrap();
    pump.scheduled(event).unwrap();
    assert_eq!(*handle.status().borrow(), State::Draining);
    let deadline = pump.next_deadline().unwrap();
    pump.advance(deadline.checked_sub(Duration::from_nanos(1)).unwrap());
    assert_eq!(*handle.status().borrow(), State::Draining);
    pump.advance(deadline);
    assert_eq!(*handle.status().borrow(), State::Played);
    assert!(pump.next_deadline().is_none());
}

#[test]
fn saturated_output_does_not_hold_interrupt_or_leak_cancelled_blocks_into_new_speech() {
    let (mut pump, mut processor, mut callback, mut ports, fence) = setup();
    let old = pump.submit(packet(101)).unwrap();
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(ports.blocks.capacity(), 0);
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    pump.interrupt();
    assert_eq!(*old.status().borrow(), State::Interrupted);
    let next = pump.submit(packet(1)).unwrap();
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(*next.status().borrow(), State::Queued);
    // The callback's bounded scan may spend one silent sample retiring cancelled
    // blocks; it must never replay the obsolete prepared segment.
    let mut silence = [1_i16; 960];
    callback.render(&mut silence, &info());
    assert!(silence.iter().all(|sample| *sample == 0));
    assert!(ports.completions.try_recv().is_err());
    pump.reference(&mut processor, &ports.references.try_recv().unwrap())
        .unwrap();
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(*next.status().borrow(), State::Rendering);
    fence.close();
    pump.advance(Instant::now());
    assert_eq!(*next.status().borrow(), State::Interrupted);
    assert!(matches!(
        pump.prepare(&mut processor, &ports.blocks),
        Err(Failure::Receipt(Refusal::Stopped))
    ));
}

#[test]
fn abandoned_prefetch_cannot_accumulate_stale_work_or_complete_after_device_loss() {
    let (mut pump, mut processor, callback, ports, _fence) = setup();
    for _ in 0..300 {
        let dropped = pump.submit(packet(1)).unwrap();
        drop(dropped);
    }
    assert!(pump.pending.len() <= 2);
    let first = pump.submit(packet(1)).unwrap();
    let second = pump.submit(packet(1)).unwrap();
    assert!(matches!(pump.submit(packet(1)), Err(Refusal::Capacity)));
    drop(callback);
    assert!(matches!(
        pump.prepare(&mut processor, &ports.blocks),
        Err(Failure::Disconnected)
    ));
    drop(pump);
    assert_eq!(*first.status().borrow(), State::Failed);
    assert_eq!(*second.status().borrow(), State::Failed);
}

#[test]
fn a_queued_handle_cancelled_between_worker_steps_does_not_fail_the_new_segment() {
    let (mut pump, mut processor, mut callback, mut ports, fence) = setup();
    let cancelled = pump.submit(packet(1)).unwrap();
    let next = pump.submit(packet(1)).unwrap();
    cancelled.interrupt();
    // An external handle can close its receipt after the worker's advance;
    // preparation must skip that generation rather than fail the device owner.
    let prepared = pump.next(&mut processor).unwrap().unwrap();
    ports.blocks.try_send(prepared).unwrap();
    callback.render(&mut [0_i16; 960], &info());
    let event = ports.completions.try_recv().unwrap();
    assert_eq!(event.lease, next.lease());
    pump.scheduled(event).unwrap();
    assert_eq!(*cancelled.status().borrow(), State::Interrupted);
    assert_eq!(*next.status().borrow(), State::Draining);
    assert!(!fence.is_closed());
}
