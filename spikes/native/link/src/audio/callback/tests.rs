use super::*;
use crate::audio::format::{Channels, RateHz};

fn format() -> Format {
    Format::new(RateHz::new(16_000).unwrap(), Channels::Mono).unwrap()
}

#[test]
fn arbitrary_callback_boundaries_preserve_pcm_order_and_stop_fences_the_partial_tail() {
    let fence = Arc::new(Fence::default());
    let (mut callback, mut frames) = Callback::channel(
        format(),
        Arc::clone(&fence),
        Budget::default(),
        Gate::new(Arc::clone(&fence)),
    );
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
    fence.close();
    callback.accept(&[8_192_i16; 320]);
    assert!(callback.partial.is_empty());
    assert!(frames.try_recv().is_err());
}

#[test]
fn full_data_queue_closes_capture_out_of_band() {
    let fence = Arc::new(Fence::default());
    let (mut callback, mut frames) = Callback::channel(
        format(),
        Arc::clone(&fence),
        Budget::default(),
        Gate::new(Arc::clone(&fence)),
    );
    let overrun = callback.overrun();
    for _ in 0..CALLBACK_QUEUE_BLOCKS {
        callback.accept(&[0_i16; 160]);
    }
    assert!(!fence.is_closed());
    callback.accept(&[0_i16; 160]);
    assert!(fence.is_closed());
    assert!(overrun.load(Ordering::Acquire));
    while frames.try_recv().is_ok() {}
    callback.accept(&[0_i16; 160]);
    assert!(frames.try_recv().is_err());
}

#[test]
fn owner_stop_immediately_fences_a_partial_device_callback_while_slot_is_held() {
    use crate::audio::owner::Engine;
    use vessel_app_view::audio::CapturePurpose;
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    owner.started().unwrap();
    let (mut callback, mut frames) =
        Callback::channel(format(), owner.fence(), owner.budget(), owner.upload_gate());
    callback.accept(&[1_i16; 159]);
    handle.stop();
    callback.accept(&[1_i16; 320]);
    assert!(callback.partial.is_empty());
    assert!(frames.try_recv().is_err());
    assert!(engine.reserve(CapturePurpose::Dictation).is_err());
    drop(owner);
    assert!(engine.reserve(CapturePurpose::Dictation).is_ok());
}

#[test]
fn partial_blocks_keep_their_capture_interval_across_mute_and_unmute() {
    use vessel_app_view::voice::Mute;
    let fence = Arc::new(Fence::default());
    let gate = Gate::new(Arc::clone(&fence));
    let (mut callback, mut frames) = Callback::channel(
        format(),
        Arc::clone(&fence),
        Budget::default(),
        gate.clone(),
    );
    callback.accept(&[1_i16; 159]);
    gate.set_mute(Mute::Muted).unwrap();
    callback.accept(&[1_i16; 2]);
    let before_mute = frames.try_recv().unwrap();
    gate.set_mute(Mute::Open).unwrap();
    callback.accept(&[1_i16; 159]);
    let during_mute = frames.try_recv().unwrap();
    callback.accept(&[1_i16; 160]);
    let after_unmute = frames.try_recv().unwrap();
    assert!(!before_mute.access.live());
    assert!(!during_mute.access.live());
    assert!(after_unmute.access.live());
    assert!(!fence.is_closed());
}
