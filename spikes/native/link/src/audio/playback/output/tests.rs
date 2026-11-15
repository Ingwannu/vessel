use super::*;
use crate::audio::{
    dsp::Processor,
    format::{Channels, RateHz},
    playback::{
        packet::Packet,
        receipts::{Book, Refusal, State},
    },
};
use bytes::Bytes;
use vessel_app_view::audio::{CapturePurpose, CaptureSlot};

fn setup() -> (Book, Callback, Ports, Format, Processor, Arc<Fence>) {
    let fence = Arc::new(Fence::default());
    let lease = CaptureSlot::default()
        .reserve(CapturePurpose::Conversation)
        .unwrap();
    let format = Format::new(RateHz::new(48_000).unwrap(), Channels::Stereo).unwrap();
    let (callback, ports) = Callback::channel(format, Arc::clone(&fence));
    (
        Book::new(lease, Arc::clone(&fence)).unwrap(),
        callback,
        ports,
        format,
        Processor::new(format, format).unwrap(),
        fence,
    )
}

#[test]
fn only_written_samples_reach_the_reference_and_tail_event_contains_device_latency() {
    let (mut book, mut callback, mut ports, format, mut processor, _fence) = setup();
    let packet = Packet::new(Bytes::from(vec![0; 480]), RateHz::new(24_000).unwrap()).unwrap();
    let handle = book.submit(packet).unwrap();
    let mut reader = book.begin(handle.lease()).unwrap().unwrap();
    let block = reader.next().unwrap().unwrap();
    let rendered = processor
        .prepare_render(block.mono(), reader.rate())
        .unwrap();
    ports
        .blocks
        .try_send(Prepared::new(rendered, &reader, block.last(), format).unwrap())
        .unwrap();
    assert!(ports.references.try_recv().is_err());
    let mut first = [1_i16; 958];
    let info = cpal::OutputCallbackInfo::new(cpal::OutputStreamTimestamp {
        callback: cpal::StreamInstant::new(1, 0),
        playback: cpal::StreamInstant::new(1, 20_000_000),
    });
    callback.render(&mut first, &info);
    assert!(first.iter().all(|sample| *sample == 0));
    assert!(ports.completions.try_recv().is_err());
    assert!(ports.references.try_recv().is_err());
    let mut tail = [1_i16; 2];
    callback.render(&mut tail, &info);
    let reference = ports.references.try_recv().unwrap();
    assert_eq!(reference.samples().len(), 960);
    assert!(reference.samples().iter().all(|sample| *sample == 0.0));
    processor.reference_render(reference.samples()).unwrap();
    let event = ports.completions.try_recv().unwrap();
    assert_eq!(event.lease, handle.lease());
    // The tail's one stereo frame adds 1/48000 second after DAC start.
    assert_eq!(
        event.after,
        Some(Duration::from_millis(20) + Duration::from_nanos(20_833))
    );
    book.scheduled(event.lease, event.now, event.after).unwrap();
    assert_eq!(*handle.status().borrow(), State::Draining);
    assert!(!format!("{reference:?}").contains("[0.0"));
}

#[test]
fn per_segment_interrupt_clears_a_partially_consumed_block_without_a_played_event() {
    let (mut book, mut callback, mut ports, format, mut processor, _fence) = setup();
    let pcm: Vec<_> = std::iter::repeat_n(8_192_i16.to_le_bytes(), 240)
        .flatten()
        .collect();
    let handle = book
        .submit(Packet::new(pcm.into(), RateHz::new(24_000).unwrap()).unwrap())
        .unwrap();
    let mut reader = book.begin(handle.lease()).unwrap().unwrap();
    let block = reader.next().unwrap().unwrap();
    ports
        .blocks
        .try_send(
            Prepared::new(
                processor
                    .prepare_render(block.mono(), reader.rate())
                    .unwrap(),
                &reader,
                true,
                format,
            )
            .unwrap(),
        )
        .unwrap();
    let now = Instant::now();
    callback.write(&mut [0_i16; 20], now, Some(Duration::ZERO));
    handle.interrupt();
    let mut rest = [1_i16; 940];
    callback.write(&mut rest, now, Some(Duration::ZERO));
    assert!(rest.iter().all(|sample| *sample == 0));
    assert!(ports.completions.try_recv().is_err());
    assert_eq!(*handle.status().borrow(), State::Interrupted);
    // AEC sees the initial audible part followed by actual silence, not prepared speech.
    let reference = ports.references.try_recv().unwrap();
    assert!(
        reference.samples()[20..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn global_stop_full_reference_queue_and_malformed_frame_never_block_the_callback() {
    let (_book, mut callback, mut ports, _format, _processor, fence) = setup();
    for _ in 0..REFERENCE_BLOCKS {
        callback.write(&mut [0_f32; 960], Instant::now(), None);
    }
    assert!(!fence.is_closed());
    callback.write(&mut [0_f32; 960], Instant::now(), None);
    assert!(fence.is_closed());
    let mut late = [1_f32; 960];
    callback.write(&mut late, Instant::now(), Some(Duration::ZERO));
    assert!(late.iter().all(|sample| *sample == 0.0));
    assert_eq!(ports.references.len(), REFERENCE_BLOCKS);
    while ports.references.try_recv().is_ok() {}
    let (_book, mut malformed, _ports, _format, _processor, fence) = setup();
    malformed.write(&mut [1_i16; 3], Instant::now(), None);
    assert!(fence.is_closed());
}

#[test]
fn a_backwards_device_clock_is_not_invented_as_success() {
    let (mut book, mut callback, mut ports, format, mut processor, _fence) = setup();
    let handle = book
        .submit(Packet::new(Bytes::from(vec![0; 480]), RateHz::new(24_000).unwrap()).unwrap())
        .unwrap();
    let mut reader = book.begin(handle.lease()).unwrap().unwrap();
    let block = reader.next().unwrap().unwrap();
    ports
        .blocks
        .try_send(
            Prepared::new(
                processor
                    .prepare_render(block.mono(), reader.rate())
                    .unwrap(),
                &reader,
                true,
                format,
            )
            .unwrap(),
        )
        .unwrap();
    let info = cpal::OutputCallbackInfo::new(cpal::OutputStreamTimestamp {
        callback: cpal::StreamInstant::new(2, 0),
        playback: cpal::StreamInstant::new(1, 0),
    });
    callback.render(&mut [0_f32; 960], &info);
    let event = ports.completions.try_recv().unwrap();
    assert_eq!(event.after, None);
    assert!(book.scheduled(event.lease, event.now, event.after).is_err());
    assert_eq!(*handle.status().borrow(), State::Failed);
}

#[test]
fn a_failed_receipt_also_fences_already_prepared_output_and_its_reader() {
    let (mut book, mut callback, mut ports, format, mut processor, _fence) = setup();
    let packet = Packet::new(Bytes::from(vec![0x20; 960]), RateHz::new(24_000).unwrap()).unwrap();
    let handle = book.submit(packet).unwrap();
    let mut reader = book.begin(handle.lease()).unwrap().unwrap();
    let block = reader.next().unwrap().unwrap();
    ports
        .blocks
        .try_send(
            Prepared::new(
                processor
                    .prepare_render(block.mono(), reader.rate())
                    .unwrap(),
                &reader,
                block.last(),
                format,
            )
            .unwrap(),
        )
        .unwrap();
    book.advance(book.next_deadline().unwrap());
    assert_eq!(*handle.status().borrow(), State::Failed);
    assert!(reader.stopped());
    assert!(matches!(reader.next(), Err(Refusal::Stopped)));
    let mut output = [1_i16; 960];
    callback.write(&mut output, Instant::now(), Some(Duration::ZERO));
    assert!(output.iter().all(|sample| *sample == 0));
    assert!(ports.completions.try_recv().is_err());
}
