//! No microphone/speaker is opened: the real bounded Pump and callback use synthetic PCM.

use super::*;
use crate::audio::{
    dsp::Processor,
    format::{Channels, Format, RateHz},
    playback::{
        output::Callback,
        receipts::{Book, State},
    },
};
use bytes::Bytes;
use std::time::{Duration, Instant};
use vessel_app_view::audio::{CapturePurpose, CaptureSlot};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
fn format() -> Format {
    Format::new(RateHz::new(48_000).unwrap(), Channels::Stereo).unwrap()
}
fn setup() -> (Control, Inbox, Pump, Arc<Fence>) {
    let lease = CaptureSlot::default()
        .reserve(CapturePurpose::Conversation)
        .unwrap();
    let fence = Arc::new(Fence::default());
    let (control, inbox) = Control::channel(Arc::clone(&fence));
    let pump = Pump::new(lease, Arc::clone(&fence), format()).unwrap();
    (control, inbox, pump, fence)
}
fn packet(blocks: usize) -> Packet {
    Packet::new(
        Bytes::from(vec![0x20; blocks.checked_mul(480).unwrap()]),
        RateHz::new(24_000).unwrap(),
    )
    .unwrap()
}
async fn ready(pending: Pending) -> Handle {
    tokio::time::timeout(TEST_TIMEOUT, pending.ready())
        .await
        .unwrap()
        .unwrap()
}
fn timestamp() -> cpal::OutputCallbackInfo {
    cpal::OutputCallbackInfo::new(cpal::OutputStreamTimestamp {
        callback: cpal::StreamInstant::new(1, 0),
        playback: cpal::StreamInstant::new(1, 20_000_000),
    })
}

#[tokio::test]
async fn admission_covers_the_queue_and_book_together_and_terminal_receipts_hold_no_pcm_permit() {
    let (control, mut inbox, mut pump, _fence) = setup();
    let first = control.submit(packet(1)).unwrap();
    let second = control.submit(packet(1)).unwrap();
    assert!(matches!(control.submit(packet(1)), Err(Refusal::Capacity)));
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    let first = ready(first).await;
    let second = ready(second).await;
    // Dequeue alone did not free either retained PCM admission.
    assert!(matches!(control.submit(packet(1)), Err(Refusal::Capacity)));
    control.interrupt().unwrap();
    pump.advance(Instant::now());
    assert_eq!(*first.status().borrow(), State::Interrupted);
    assert_eq!(*second.status().borrow(), State::Interrupted);
    let _next = control.submit(packet(1)).unwrap();
    let _another = control.submit(packet(1)).unwrap();
    assert!(matches!(control.submit(packet(1)), Err(Refusal::Capacity)));
}

#[tokio::test]
async fn dropping_a_full_queue_request_fences_it_before_the_worker_reads_it() {
    let (control, mut inbox, mut pump, _fence) = setup();
    let abandoned = control.submit(packet(1)).unwrap();
    let kept = control.submit(packet(1)).unwrap();
    drop(abandoned);
    assert!(matches!(control.submit(packet(1)), Err(Refusal::Capacity)));
    let stale = inbox.receiver.try_recv().unwrap();
    assert!(stale.packet.stopped());
    stale.apply(&mut pump);
    let fresh = control.submit(packet(1)).unwrap();
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    let kept = ready(kept).await;
    let fresh = ready(fresh).await;
    assert_eq!(*kept.status().borrow(), State::Queued);
    assert_eq!(*fresh.status().borrow(), State::Queued);
}

#[tokio::test]
async fn barge_in_silences_a_full_prepared_queue_and_late_wake_preserves_new_speech() {
    let (control, mut inbox, mut pump, fence) = setup();
    let output = format();
    let (mut callback, mut ports) = Callback::channel(output, fence);
    let mut processor = Processor::new(output, output).unwrap();
    let pending = control.submit(packet(101)).unwrap();
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    let old = ready(pending).await;
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(ports.blocks.capacity(), 0);
    control.interrupt().unwrap();
    // The actor has not pruned the old Book yet; callback atomics already silence it.
    let mut silence = [1_i16; 960];
    callback.render(&mut silence, &timestamp());
    assert!(silence.iter().all(|sample| *sample == 0));
    assert!(ports.completions.try_recv().is_err());
    let pending = control.submit(packet(1)).unwrap();
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    let new = ready(pending).await;
    assert_eq!(*old.status().borrow(), State::Interrupted);
    // The old wake arrives after fresh admission. It is pruning, never interrupt-all.
    assert!(
        tokio::time::timeout(TEST_TIMEOUT, inbox.next())
            .await
            .unwrap()
            .is_none()
    );
    pump.advance(Instant::now());
    assert_eq!(*new.status().borrow(), State::Queued);
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(*new.status().borrow(), State::Rendering);
}

#[tokio::test]
async fn abandoning_a_delivered_reply_cannot_leave_prepared_audio_playable() {
    let (control, mut inbox, mut pump, fence) = setup();
    let output = format();
    let (mut callback, ports) = Callback::channel(output, fence);
    let mut processor = Processor::new(output, output).unwrap();
    let pending = control.submit(packet(101)).unwrap();
    inbox.receiver.try_recv().unwrap().apply(&mut pump);
    pump.prepare(&mut processor, &ports.blocks).unwrap();
    assert_eq!(ports.blocks.capacity(), 0);
    // Reply holds a Handle, but the future has not accepted ownership of it.
    drop(pending);
    let mut silence = [1_i16; 960];
    callback.render(&mut silence, &timestamp());
    assert!(silence.iter().all(|sample| *sample == 0));
    pump.advance(Instant::now());
    assert!(pump.next_deadline().is_none());
}

#[tokio::test]
async fn stop_wakes_full_commands_without_needing_the_worker_or_a_reply() {
    let (control, mut inbox, _pump, fence) = setup();
    let first = control.submit(packet(1)).unwrap();
    let second = control.submit(packet(1)).unwrap();
    control.stop();
    assert!(fence.is_closed());
    assert!(matches!(
        tokio::time::timeout(TEST_TIMEOUT, first.ready())
            .await
            .unwrap(),
        Err(Refusal::Stopped)
    ));
    assert!(matches!(
        tokio::time::timeout(TEST_TIMEOUT, second.ready())
            .await
            .unwrap(),
        Err(Refusal::Stopped)
    ));
    assert!(matches!(control.submit(packet(1)), Err(Refusal::Stopped)));
    assert!(matches!(control.interrupt(), Err(Refusal::Stopped)));
    assert!(inbox.receiver.try_recv().unwrap().packet.stopped());
}

#[tokio::test]
async fn a_cancelled_reader_retains_the_admission_until_its_raw_pcm_is_dropped() {
    let (control, mut inbox, _pump, fence) = setup();
    let pending = control.submit(packet(1)).unwrap();
    let Command {
        packet: admitted,
        response,
    } = inbox.receiver.try_recv().unwrap();
    let lease = CaptureSlot::default()
        .reserve(CapturePurpose::Conversation)
        .unwrap();
    let mut book = Book::new(lease, fence).unwrap();
    let handle = book.submit(admitted).unwrap();
    let reader = book.begin(handle.lease()).unwrap().unwrap();
    response.send(Ok(handle)).unwrap();
    let handle = ready(pending).await;
    book.interrupt_all();
    assert_eq!(*handle.status().borrow(), State::Interrupted);
    assert!(reader.stopped());
    let _other = control.submit(packet(1)).unwrap();
    assert!(matches!(control.submit(packet(1)), Err(Refusal::Capacity)));
    drop(reader);
    let _next = control.submit(packet(1)).unwrap();
}
