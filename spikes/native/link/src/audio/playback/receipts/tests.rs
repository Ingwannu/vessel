use super::*;
use crate::audio::format::RateHz;
use bytes::Bytes;
use vessel_app_view::audio::CaptureSlot;

fn setup() -> (Book, Arc<Fence>) {
    let lease = CaptureSlot::default()
        .reserve(CapturePurpose::Conversation)
        .unwrap();
    let fence = Arc::new(Fence::default());
    (Book::new(lease, Arc::clone(&fence)).unwrap(), fence)
}
fn packet(bytes: &'static [u8]) -> Packet {
    Packet::new(Bytes::from_static(bytes), RateHz::new(24_000).unwrap()).unwrap()
}

#[test]
fn final_callback_copy_drains_until_the_predicted_last_sample_time() {
    let (mut book, _fence) = setup();
    let handle = book.submit(packet(&[0, 32])).unwrap();
    let status = handle.status();
    let lease = handle.lease();
    let mut reader = book.begin(lease).unwrap().unwrap();
    let tail = reader.next().unwrap().unwrap();
    assert_eq!(tail.valid_frames(), 1);
    assert_eq!(tail.mono().len(), 240);
    assert_eq!(tail.mono()[0].to_bits(), 0.25_f32.to_bits());
    assert!(tail.mono()[1..].iter().all(|sample| *sample == 0.0));
    assert!(tail.last());
    assert!(reader.next().unwrap().is_none());
    assert_eq!(reader.rate().get(), 24_000);
    assert!(!reader.stopped());
    assert!(!format!("{reader:?}{tail:?}").contains("0.25"));
    let now = Instant::now();
    let drain = Duration::from_millis(50);
    book.scheduled(lease, now, Some(drain)).unwrap();
    assert_eq!(*status.borrow(), State::Draining);
    assert_eq!(book.next_deadline(), Some(now + drain));
    book.advance((now + drain).checked_sub(Duration::from_nanos(1)).unwrap());
    assert_eq!(*status.borrow(), State::Draining);
    book.advance(now + drain);
    assert_eq!(*status.borrow(), State::Played);
    assert_eq!(book.next_deadline(), None);
    handle.interrupt();
    assert_eq!(*status.borrow(), State::Played);
}

#[test]
fn full_prefetch_and_draining_segments_cancel_outside_the_queue() {
    let (mut book, fence) = setup();
    let first = book.submit(packet(&[0, 32])).unwrap();
    let second = book.submit(packet(&[0, 64])).unwrap();
    assert!(matches!(
        book.submit(packet(&[0, 0])),
        Err(Refusal::Capacity)
    ));
    let mut active = book.begin(first.lease()).unwrap().unwrap();
    let now = Instant::now();
    book.scheduled(first.lease(), now, Some(Duration::from_millis(10)))
        .unwrap();
    fence.close();
    assert!(matches!(active.next(), Err(Refusal::Stopped)));
    assert!(matches!(book.begin(second.lease()), Err(Refusal::Stopped)));
    book.advance(now + Duration::from_secs(1));
    assert_eq!(*first.status().borrow(), State::Interrupted);
    assert_eq!(*second.status().borrow(), State::Interrupted);
    assert!(matches!(
        book.submit(packet(&[0, 0])),
        Err(Refusal::Stopped)
    ));
    book.scheduled(first.lease(), now, Some(Duration::ZERO))
        .unwrap();
    assert_eq!(*first.status().borrow(), State::Interrupted);
}

#[test]
fn barge_in_cancels_prepared_readers_and_accepts_only_a_fresh_group() {
    let (mut book, _fence) = setup();
    let first = book.submit(packet(&[0, 32])).unwrap();
    let mut reader = book.begin(first.lease()).unwrap().unwrap();
    book.interrupt_all();
    assert!(reader.stopped());
    assert!(matches!(reader.next(), Err(Refusal::Stopped)));
    let next = book.submit(packet(&[0, 32])).unwrap();
    assert_ne!(first.lease(), next.lease());
    assert!(book.begin(first.lease()).unwrap().is_none());
    assert!(book.begin(next.lease()).unwrap().is_some());
}

#[test]
fn bad_clock_or_lost_device_is_failed_and_never_played() {
    let (mut book, _fence) = setup();
    for after in [None, Some(MAX_DEVICE_DRAIN + Duration::from_nanos(1))] {
        let handle = book.submit(packet(&[0, 0])).unwrap();
        assert!(book.begin(handle.lease()).unwrap().is_some());
        assert_eq!(
            book.scheduled(handle.lease(), Instant::now(), after),
            Err(Refusal::Clock)
        );
        assert_eq!(*handle.status().borrow(), State::Failed);
        book.advance(Instant::now());
    }
    let handle = book.submit(packet(&[0, 0])).unwrap();
    let status = handle.status();
    drop(book);
    assert_eq!(*status.borrow(), State::Failed);
}

#[test]
fn handle_drop_and_another_capture_cannot_complete_old_output() {
    let (mut book, _fence) = setup();
    let handle = book.submit(packet(&[0, 0])).unwrap();
    let lease = handle.lease();
    let status = handle.status();
    let mut reader = book.begin(lease).unwrap().unwrap();
    drop(handle);
    assert_eq!(*status.borrow(), State::Interrupted);
    assert!(matches!(reader.next(), Err(Refusal::Stopped)));
    let mut slot = CaptureSlot::default();
    let previous = slot.reserve(CapturePurpose::Conversation).unwrap();
    assert_eq!(
        slot.stopping(previous).unwrap(),
        vessel_app_view::audio::CaptureCompletion::Current
    );
    assert_eq!(
        slot.released(previous).unwrap(),
        vessel_app_view::audio::CaptureCompletion::Current
    );
    let other = slot.reserve(CapturePurpose::Conversation).unwrap();
    let mut other_book = Book::new(other, Arc::new(Fence::default())).unwrap();
    assert!(other_book.begin(lease).unwrap().is_none());
    other_book
        .scheduled(lease, Instant::now(), Some(Duration::ZERO))
        .unwrap();
    assert_eq!(*status.borrow(), State::Interrupted);
    let dictation = CaptureSlot::default()
        .reserve(CapturePurpose::Dictation)
        .unwrap();
    assert!(matches!(
        Book::new(dictation, Arc::new(Fence::default())),
        Err(Refusal::Purpose)
    ));
}

#[test]
fn invalid_pcm_and_exhausted_ticket_are_refused_before_insertion() {
    let rate = RateHz::new(24_000).unwrap();
    for pcm in [
        Bytes::new(),
        Bytes::from_static(&[1]),
        Bytes::from(vec![0; 8 * 1024 * 1024 + 2]),
    ] {
        assert!(matches!(Packet::new(pcm, rate), Err(Refusal::Format)));
    }
    let (mut book, _fence) = setup();
    book.next = u64::MAX;
    assert!(matches!(
        book.submit(packet(&[0, 0])),
        Err(Refusal::Exhausted)
    ));
    assert!(book.entries.is_empty());
}

#[test]
fn a_silent_device_and_never_started_segment_have_finite_failure_deadlines() {
    let (mut book, _fence) = setup();
    let queued = book.submit(packet(&[0, 0])).unwrap();
    let deadline = book.next_deadline().unwrap();
    book.advance(deadline.checked_sub(Duration::from_nanos(1)).unwrap());
    assert_eq!(*queued.status().borrow(), State::Queued);
    book.advance(deadline);
    assert_eq!(*queued.status().borrow(), State::Failed);
    let active = book.submit(packet(&[0, 0])).unwrap();
    assert!(book.begin(active.lease()).unwrap().is_some());
    book.advance(book.next_deadline().unwrap());
    assert_eq!(*active.status().borrow(), State::Failed);
    let slow_pcm = Bytes::from(vec![0; 8 * 1024 * 1024]);
    assert!(matches!(
        Packet::new(slow_pcm, RateHz::new(8_000).unwrap()),
        Err(Refusal::Format)
    ));
}

mod lock_probe;
