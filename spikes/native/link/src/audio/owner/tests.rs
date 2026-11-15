use super::*;
use vessel_app_view::audio::CaptureRefusal;

#[test]
fn stop_and_handle_drop_gate_callbacks_but_never_release_a_live_owner() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    owner.started().unwrap();
    assert_eq!(handle.lease(), owner.lease());
    let late_callback = owner.fence();
    let status = handle.status();
    handle.stop();
    assert!(late_callback.load(Ordering::Acquire));
    assert_eq!(*status.borrow(), State::Running);
    assert!(matches!(
        engine.reserve(CapturePurpose::Dictation),
        Err(AudioError::Reservation(CaptureRefusal::Busy(
            CapturePurpose::Conversation
        )))
    ));
    drop(handle);
    assert!(engine.reserve(CapturePurpose::Dictation).is_err());
    // The device thread drops its streams before dropping this guard.
    drop(owner);
    assert_eq!(*status.borrow(), State::Released);
    let (next, handle) = engine.reserve(CapturePurpose::Dictation).unwrap();
    assert!(!next.stopped());
    assert!(late_callback.load(Ordering::Acquire));
    assert_eq!(next.lease(), handle.lease());
}

#[test]
fn stop_before_start_never_reopens_the_callback_fence() {
    let engine = Engine::default();
    let (mut owner, handle) = engine.reserve(CapturePurpose::Dictation).unwrap();
    handle.stop();
    assert!(matches!(owner.started(), Err(AudioError::Stopped)));
    let status = handle.status();
    owner.fail(DeviceFailure::Thread);
    drop(owner);
    assert_eq!(*status.borrow(), State::Failed(DeviceFailure::Thread));
    assert!(engine.reserve(CapturePurpose::Conversation).is_ok());
}
