use super::*;
use crate::audio::{Channels, RateHz, dsp::Processor, owner::Engine};

#[tokio::test]
async fn public_controls_cancel_a_pending_reader_but_cannot_publish_physical_release() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Dictation).unwrap();
    owner.started().unwrap();
    let (_sender, frames) = mpsc::channel(1);
    let mut capture = Capture {
        lease: owner.lease(),
        fence: owner.fence(),
        frames,
        status: handle.status(),
        _owner: handle,
    };
    let control = capture.control();
    assert_eq!(capture.lease(), control.lease());
    let task = tokio::spawn(async move { capture.next().await });
    tokio::task::yield_now().await;
    control.stop();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap()
            .is_none()
    );
    assert_eq!(*control.status().borrow(), CaptureState::Running);
    assert!(engine.reserve(CapturePurpose::Dictation).is_err());
    drop(owner);
    assert_eq!(*control.status().borrow(), CaptureState::Released);
}

#[tokio::test]
async fn a_queued_frame_has_typed_fixed_metadata_and_stop_discards_it() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Dictation).unwrap();
    let format = Format::new(RateHz::new(16_000).unwrap(), Channels::Mono).unwrap();
    let mut processor = Processor::capture_only(format).unwrap();
    let frame = Frame::new(
        owner.lease(),
        Held::new(
            processor.capture(&[0.0; 160]).unwrap(),
            owner.budget().reserve().unwrap(),
        ),
    );
    assert_eq!(frame.format().rate().get(), 16_000);
    assert_eq!(frame.format().channels(), Channels::Mono);
    assert_eq!(frame.format().frames().get(), 160);
    assert_eq!(frame.pcm().len(), 320);
    assert_eq!(frame.lease(), handle.lease());
    assert!(!format!("{frame:?}").contains("[0, 0]"));
    let (sender, frames) = mpsc::channel(1);
    sender.try_send(frame).unwrap();
    let mut capture = Capture {
        lease: owner.lease(),
        fence: owner.fence(),
        frames,
        status: handle.status(),
        _owner: handle,
    };
    capture.stop();
    assert!(capture.next().await.is_none());
    assert!(capture.frames.try_recv().is_err());
}

#[test]
fn conversation_acquisition_refuses_before_opening_hardware() {
    assert!(matches!(
        Capture::acquire(CapturePurpose::Conversation),
        Err(AcquireError::ConversationUnavailable)
    ));
}
