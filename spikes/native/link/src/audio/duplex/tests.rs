use super::*;
use crate::audio::{
    budget::Budget,
    callback::Callback as InputCallback,
    format::{Channels, RateHz},
    owner::Engine,
    playback::{output::Callback as OutputCallback, receipts::State},
    upload_access::Gate,
};
use bytes::Bytes;
use std::{
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::mpsc;
use vessel_app_view::audio::{CapturePurpose, CaptureSlot};

const TEST_WAIT: Duration = Duration::from_secs(2);
struct InputFake {
    format: Format,
    frames: mpsc::Receiver<Samples>,
    starts: Arc<Mutex<Vec<&'static str>>>,
    failure: Option<DeviceFailure>,
    drops: Arc<AtomicUsize>,
}
impl CaptureSource for InputFake {
    fn format(&self) -> Format {
        self.format
    }
    fn start(&self) -> Result<(), DeviceFailure> {
        self.starts.lock().unwrap().push("input");
        self.failure.map_or(Ok(()), Err)
    }
    fn failure(&self) -> Option<DeviceFailure> {
        self.failure
    }
    async fn receive(&mut self) -> Result<Samples, DeviceFailure> {
        self.frames.recv().await.ok_or(DeviceFailure::Interrupted)
    }
}
impl Drop for InputFake {
    fn drop(&mut self) {
        let _old = self.drops.fetch_add(1, Ordering::AcqRel);
    }
}
struct OutputFake {
    format: Format,
    starts: Arc<Mutex<Vec<&'static str>>>,
    failure: Option<DeviceFailure>,
    drops: Arc<AtomicUsize>,
}
impl SpeakerSource for OutputFake {
    fn format(&self) -> Format {
        self.format
    }
    fn start(&self) -> Result<(), DeviceFailure> {
        self.starts.lock().unwrap().push("output");
        self.failure.map_or(Ok(()), Err)
    }
    fn failure(&self) -> Option<DeviceFailure> {
        self.failure
    }
}
impl Drop for OutputFake {
    fn drop(&mut self) {
        let _old = self.drops.fetch_add(1, Ordering::AcqRel);
    }
}
struct Fixture {
    streams: Streams<InputFake, OutputFake>,
    input: InputCallback,
    output: OutputCallback,
    starts: Arc<Mutex<Vec<&'static str>>>,
    drops: Arc<AtomicUsize>,
}
fn setup() -> Fixture {
    let lease = CaptureSlot::default()
        .reserve(CapturePurpose::Conversation)
        .unwrap();
    setup_with(lease, Arc::new(Fence::default()))
}
fn setup_with(lease: CaptureLease, fence: Arc<Fence>) -> Fixture {
    let input_format = Format::capture();
    let output_format = Format::new(RateHz::new(48_000).unwrap(), Channels::Stereo).unwrap();
    let (input, frames) = InputCallback::channel(
        input_format,
        Arc::clone(&fence),
        Budget::default(),
        Gate::new(Arc::clone(&fence)),
    );
    let (output, ports) = OutputCallback::channel(output_format, Arc::clone(&fence));
    let starts = Arc::new(Mutex::new(Vec::new()));
    let drops = Arc::new(AtomicUsize::new(0));
    let streams = Streams::new(
        lease,
        fence,
        InputFake {
            format: input_format,
            frames,
            starts: Arc::clone(&starts),
            failure: None,
            drops: Arc::clone(&drops),
        },
        OutputFake {
            format: output_format,
            starts: Arc::clone(&starts),
            failure: None,
            drops: Arc::clone(&drops),
        },
        ports,
    )
    .unwrap();
    Fixture {
        streams,
        input,
        output,
        starts,
        drops,
    }
}
fn packet() -> Packet {
    Packet::new(Bytes::from(vec![0x20; 480]), RateHz::new(24_000).unwrap()).unwrap()
}
fn timestamp() -> cpal::OutputCallbackInfo {
    cpal::OutputCallbackInfo::new(cpal::OutputStreamTimestamp {
        callback: cpal::StreamInstant::new(1, 0),
        playback: cpal::StreamInstant::new(1, 20_000_000),
    })
}

#[tokio::test]
async fn real_bundle_processes_output_reference_and_drains_only_after_callback_completion() {
    let mut fixture = setup();
    fixture.streams.start().unwrap();
    assert_eq!(*fixture.starts.lock().unwrap(), ["output", "input"]);
    let played = fixture.streams.submit(packet()).unwrap();
    fixture.input.accept(&[0_i16; 160]);
    let first = tokio::time::timeout(TEST_WAIT, fixture.streams.receive())
        .await
        .unwrap()
        .unwrap();
    drop(first);
    assert_eq!(*played.status().borrow(), State::Rendering);
    assert!(fixture.streams.ports.references.is_empty());
    let mut output = [0_i16; 960];
    fixture.output.render(&mut output, &timestamp());
    assert!(output.iter().any(|sample| *sample != 0));
    assert_eq!(*played.status().borrow(), State::Rendering);
    fixture.input.accept(&[0_i16; 160]);
    let second = tokio::time::timeout(TEST_WAIT, fixture.streams.receive())
        .await
        .unwrap()
        .unwrap();
    drop(second);
    assert!(fixture.streams.ports.references.is_empty());
    assert!(fixture.streams.ports.completions.is_empty());
    assert!(matches!(
        *played.status().borrow(),
        State::Draining | State::Played
    ));
    if let Some(deadline) = fixture.streams.pump.next_deadline() {
        fixture.streams.pump.advance(deadline);
    }
    assert_eq!(*played.status().borrow(), State::Played);
}

#[tokio::test]
async fn a_silent_input_and_pending_output_cannot_hold_stop_or_restart_cancelled_audio() {
    let mut fixture = setup();
    let played = fixture.streams.submit(packet()).unwrap();
    fixture.streams.interrupt();
    assert_eq!(*played.status().borrow(), State::Interrupted);
    let next = fixture.streams.submit(packet()).unwrap();
    assert!(!fixture.streams.fence.is_closed());
    fixture.streams.fence.close();
    assert_eq!(
        tokio::time::timeout(TEST_WAIT, fixture.streams.receive())
            .await
            .unwrap()
            .unwrap_err(),
        DeviceFailure::Interrupted
    );
    let mut silence = [1_i16; 960];
    fixture.output.render(&mut silence, &timestamp());
    assert!(silence.iter().all(|sample| *sample == 0));
    assert_eq!(*next.status().borrow(), State::Interrupted);
    assert!(matches!(
        fixture.streams.submit(packet()),
        Err(Refusal::Stopped)
    ));
}

#[test]
fn partial_start_failure_fences_both_streams_and_drops_them_before_ownership_is_free() {
    let engine = Engine::default();
    let (owner, _handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    let mut fixture = setup_with(owner.lease(), owner.fence());
    fixture.streams.input.failure = Some(DeviceFailure::Unavailable);
    assert_eq!(fixture.streams.start(), Err(DeviceFailure::Unavailable));
    assert_eq!(*fixture.starts.lock().unwrap(), ["output", "input"]);
    assert!(fixture.streams.fence.is_closed());
    assert_eq!(fixture.streams.failure(), Some(DeviceFailure::Unavailable));
    let drops = Arc::clone(&fixture.drops);
    drop(fixture);
    assert_eq!(drops.load(Ordering::Acquire), 2);
    assert!(engine.reserve(CapturePurpose::Dictation).is_err());
    drop(owner);
    assert!(engine.reserve(CapturePurpose::Dictation).is_ok());
}

#[test]
fn stopped_native_constructor_never_discovers_or_opens_a_physical_device() {
    let engine = Engine::default();
    let (owner, handle) = engine.reserve(CapturePurpose::Conversation).unwrap();
    handle.stop();
    assert!(matches!(
        Streams::open(&owner),
        Err(DeviceFailure::Interrupted)
    ));
}

#[tokio::test]
async fn generated_cue_uses_the_speech_callback_reference_and_physical_receipt_path() {
    let mut fixture = setup();
    let cue = crate::audio::playback::cue::Cue::new().unwrap();
    let played = fixture.streams.submit(cue.packet().unwrap()).unwrap();
    let mut audible = false;
    for block in 0..298 {
        fixture.input.accept(&[0_i16; 160]);
        let captured = tokio::time::timeout(TEST_WAIT, fixture.streams.receive())
            .await
            .unwrap()
            .unwrap();
        drop(captured);
        assert_eq!(*played.status().borrow(), State::Rendering);
        let mut output = [0_i16; 960];
        fixture.output.render(&mut output, &timestamp());
        audible |= output.iter().any(|sample| *sample != 0);
        if block == 297 {
            assert_eq!(*played.status().borrow(), State::Rendering);
        }
    }
    assert!(audible);
    fixture.input.accept(&[0_i16; 160]);
    let captured = tokio::time::timeout(TEST_WAIT, fixture.streams.receive())
        .await
        .unwrap()
        .unwrap();
    drop(captured);
    assert!(matches!(
        *played.status().borrow(),
        State::Draining | State::Played
    ));
    if let Some(deadline) = fixture.streams.pump.next_deadline() {
        fixture.streams.pump.advance(deadline);
    }
    assert_eq!(*played.status().borrow(), State::Played);
}
