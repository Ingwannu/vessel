use super::super::capture::Held;
use super::super::dsp::Processor;
use super::super::format::{Channels, Format, RateHz};
use super::super::owner::{Engine, State};
use super::*;
use std::sync::mpsc as blocking;
use std::time::Duration;
use vessel_app_view::audio::CapturePurpose;

struct Device {
    fence: Arc<Fence>,
    dropped: blocking::SyncSender<()>,
    release: blocking::Receiver<()>,
}

impl CaptureDevice for Device {
    fn start(&self) -> Result<(), DeviceFailure> {
        Ok(())
    }
    async fn receive(&mut self) -> Result<Held, DeviceFailure> {
        self.fence.close();
        std::future::pending().await
    }
    fn failure(&self) -> Option<DeviceFailure> {
        None
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.dropped.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}

fn processor() -> Processor {
    let format = Format::new(RateHz::new(16_000).unwrap(), Channels::Mono).unwrap();
    Processor::new(format, format).unwrap()
}

#[test]
fn native_start_after_local_stop_never_opens_a_device_or_publishes_frames() {
    let (owner, handle) = Engine::process()
        .reserve(CapturePurpose::Dictation)
        .unwrap();
    let mut status = handle.status();
    handle.stop();
    let mut frames = start(owner).unwrap();
    assert!(frames.blocking_recv().is_none());
    assert_eq!(*status.borrow_and_update(), State::Released);
}

#[test]
fn a_slow_device_destructor_keeps_the_slot_after_callbacks_are_fenced() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    let status = handle.status();
    let (dropped, destroying) = blocking::sync_channel(1);
    let (release, released) = blocking::sync_channel(1);
    let (sender, _receiver) = mpsc::channel(1);
    let task = std::thread::spawn(move || {
        run(owner, &sender, |fence, _budget, _upload| {
            Ok(Device {
                fence,
                dropped,
                release: released,
            })
        });
    });
    destroying.recv_timeout(Duration::from_secs(2)).unwrap();
    handle.stop();
    assert!(engine.reserve(CapturePurpose::Dictation).is_err());
    assert_eq!(*status.borrow(), State::Running);
    release.send(()).unwrap();
    task.join().unwrap();
    assert_eq!(*status.borrow(), State::Released);
    assert!(engine.reserve(CapturePurpose::Dictation).is_ok());
}

struct Frames(Processor, Budget, Gate);
impl CaptureDevice for Frames {
    fn start(&self) -> Result<(), DeviceFailure> {
        Ok(())
    }
    fn receive(&mut self) -> impl Future<Output = Result<Held, DeviceFailure>> {
        std::future::ready(
            self.1
                .reserve()
                .map_err(|_private| DeviceFailure::Overrun)
                .and_then(|permit| {
                    self.0
                        .capture(&[0.0; 160])
                        .map(|audio| Held::new(audio, permit, self.2.snapshot()))
                        .map_err(|_private| DeviceFailure::Processing)
                }),
        )
    }
    fn failure(&self) -> Option<DeviceFailure> {
        None
    }
}

#[test]
fn normalized_queue_overrun_disposes_the_device_and_cannot_reuse_a_generation() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Dictation).unwrap();
    let status = handle.status();
    let (sender, mut receiver) = mpsc::channel(1);
    run(owner, &sender, |_fence, budget, upload| {
        Ok(Frames(processor(), budget, upload))
    });
    assert_eq!(*status.borrow(), State::Failed(DeviceFailure::Overrun));
    let frame = receiver.try_recv().unwrap();
    assert_eq!(frame.lease(), handle.lease());
    assert_eq!(frame.pcm().len(), 320);
    assert!(!format!("{frame:?}").contains("[0, 0]"));
    let (next, next_handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    assert_ne!(frame.lease(), next_handle.lease());
    assert!(!next.stopped());
}

#[test]
fn opening_failure_releases_reservation_without_any_running_state() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Dictation).unwrap();
    let status = handle.status();
    let (sender, _receiver) = mpsc::channel(1);
    run::<Frames>(owner, &sender, |_fence, _budget, _upload| {
        Err(DeviceFailure::Unavailable)
    });
    assert_eq!(*status.borrow(), State::Failed(DeviceFailure::Unavailable));
    assert!(engine.reserve(CapturePurpose::Conversation).is_ok());
}

struct Silent {
    started: blocking::SyncSender<()>,
}
impl CaptureDevice for Silent {
    fn start(&self) -> Result<(), DeviceFailure> {
        self.started
            .send(())
            .map_err(|_private| DeviceFailure::Thread)
    }
    async fn receive(&mut self) -> Result<Held, DeviceFailure> {
        std::future::pending().await
    }
    fn failure(&self) -> Option<DeviceFailure> {
        None
    }
}

#[test]
fn stop_wakes_a_silent_device_without_waiting_for_another_callback() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    let status = handle.status();
    let (started, running) = blocking::sync_channel(1);
    let (done, finished) = blocking::sync_channel(1);
    let (sender, _receiver) = mpsc::channel(1);
    let task = std::thread::spawn(move || {
        run(owner, &sender, |_fence, _budget, _upload| {
            Ok(Silent { started })
        });
        done.send(()).unwrap();
    });
    running.recv_timeout(Duration::from_secs(2)).unwrap();
    handle.stop();
    finished.recv_timeout(Duration::from_secs(2)).unwrap();
    task.join().unwrap();
    assert_eq!(*status.borrow(), State::Released);
    assert!(engine.reserve(CapturePurpose::Dictation).is_ok());
}
