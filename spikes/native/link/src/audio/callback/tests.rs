use super::*;
use crate::audio::format::{Channels, RateHz};

fn format() -> Format {
    Format::new(RateHz::new(16_000).unwrap(), Channels::Mono).unwrap()
}

#[test]
fn arbitrary_callback_boundaries_preserve_pcm_order_and_stop_fences_the_partial_tail() {
    let fence = Arc::new(AtomicBool::new(false));
    let (mut callback, mut frames) = Callback::channel(format(), Arc::clone(&fence));
    callback.accept(&[8_192_i16; 159]);
    assert!(frames.try_recv().is_err());
    callback.accept(&[8_192_i16; 2]);
    let frame = frames.try_recv().unwrap();
    assert_eq!(frame.as_slice().len(), 160);
    assert!(
        frame
            .as_slice()
            .iter()
            .all(|sample| sample.to_bits() == 0.25_f32.to_bits())
    );
    assert!(!format!("{frame:?}").contains("0.25"));
    fence.store(true, Ordering::Release);
    callback.accept(&[8_192_i16; 320]);
    assert!(callback.partial.is_empty());
    assert!(frames.try_recv().is_err());
}

#[test]
fn full_data_queue_closes_capture_out_of_band() {
    let fence = Arc::new(AtomicBool::new(false));
    let (mut callback, mut frames) = Callback::channel(format(), Arc::clone(&fence));
    let overrun = callback.overrun();
    for _ in 0..CALLBACK_QUEUE_BLOCKS {
        callback.accept(&[0_i16; 160]);
    }
    assert!(!fence.load(Ordering::Acquire));
    callback.accept(&[0_i16; 160]);
    assert!(fence.load(Ordering::Acquire));
    assert!(overrun.load(Ordering::Acquire));
    while frames.try_recv().is_ok() {}
    callback.accept(&[0_i16; 160]);
    assert!(frames.try_recv().is_err());
}
