use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{mpsc, oneshot};
use vessel_proto::voice::{PlaybackPcm, VoiceEpoch, VoiceId};

const TEST_WAIT: Duration = Duration::from_secs(2);
#[derive(Default)]
struct Counts {
    next: AtomicUsize,
    active: AtomicUsize,
    maximum: AtomicUsize,
}
struct Active(Arc<Counts>);
impl Drop for Active {
    fn drop(&mut self) {
        let _old = self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Request {
    index: usize,
    cancel: CancellationToken,
    reply: oneshot::Sender<Result<Synthesized, TtsError>>,
}
struct Provider {
    counts: Arc<Counts>,
    requests: mpsc::Sender<Request>,
}
impl Synthesizer for Provider {
    async fn generate(
        &self,
        _text: &SpeechText,
        cancel: &CancellationToken,
    ) -> Result<Synthesized, TtsError> {
        let index = self.counts.next.fetch_add(1, Ordering::AcqRel);
        let active = self
            .counts
            .active
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        let _previous = self.counts.maximum.fetch_max(active, Ordering::AcqRel);
        let _active = Active(Arc::clone(&self.counts));
        let (reply, result) = oneshot::channel();
        self.requests
            .send(Request {
                index,
                cancel: cancel.clone(),
                reply,
            })
            .await
            .map_err(|_private| TtsError::Cancelled)?;
        result.await.map_err(|_private| TtsError::Cancelled)?
    }
}
struct Audible {
    lease: VoiceLease,
    packet: Playback,
    reply: oneshot::Sender<PlaybackOutcome>,
}
struct Speaker {
    packets: mpsc::Sender<Audible>,
}
impl Output for Speaker {
    async fn play(
        &mut self,
        lease: VoiceLease,
        packet: Playback,
        _cancel: &CancellationToken,
    ) -> Result<PlaybackOutcome, Failure> {
        let (reply, result) = oneshot::channel();
        self.packets
            .send(Audible {
                lease,
                packet,
                reply,
            })
            .await
            .map_err(|_private| Failure::Output)?;
        result.await.map_err(|_private| Failure::Output)
    }
}
fn setup() -> (
    Provider,
    Speaker,
    mpsc::Receiver<Request>,
    mpsc::Receiver<Audible>,
    Arc<Counts>,
) {
    let (requests, request_rx) = mpsc::channel(8);
    let (packets, packet_rx) = mpsc::channel(2);
    let counts = Arc::new(Counts::default());
    (
        Provider {
            counts: Arc::clone(&counts),
            requests,
        },
        Speaker { packets },
        request_rx,
        packet_rx,
        counts,
    )
}
fn lease() -> VoiceLease {
    VoiceLease {
        id: VoiceId::random(),
        epoch: VoiceEpoch::random(),
    }
}
fn text() -> SpeechText {
    SpeechText::try_from(String::from("One. Two. Three. Four.")).unwrap()
}
fn audio(index: usize) -> Synthesized {
    Synthesized {
        format: "audio/pcm;rate=24000;bits=16".parse().unwrap(),
        pcm: PlaybackPcm::new(vec![u8::try_from(index).unwrap(), 0]).unwrap(),
    }
}
async fn request(rx: &mut mpsc::Receiver<Request>) -> Request {
    tokio::time::timeout(TEST_WAIT, rx.recv())
        .await
        .unwrap()
        .unwrap()
}
async fn audible(rx: &mut mpsc::Receiver<Audible>) -> Audible {
    tokio::time::timeout(TEST_WAIT, rx.recv())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn prefetch_runs_during_a_held_receipt_but_audio_and_groups_remain_ordered() {
    let (provider, mut speaker, mut requests, mut packets, counts) = setup();
    let generation = CancellationToken::new();
    let running = generation.clone();
    let lease = lease();
    let job =
        tokio::spawn(async move { speak(&text(), lease, &provider, &mut speaker, &running).await });
    let first = request(&mut requests).await;
    let second = request(&mut requests).await;
    assert_eq!((first.index, second.index), (0, 1));
    let child = first.cancel.clone();
    second.reply.send(Ok(audio(1))).unwrap();
    assert!(packets.try_recv().is_err());
    first.reply.send(Ok(audio(0))).unwrap();
    let first = audible(&mut packets).await;
    assert_eq!(first.lease, lease);
    assert_eq!(first.packet.pcm.as_slice(), &[0, 0]);
    let third = request(&mut requests).await;
    assert_eq!(third.index, 2);
    third.reply.send(Ok(audio(2))).unwrap();
    assert_eq!(counts.next.load(Ordering::Acquire), 3);
    assert!(packets.try_recv().is_err());
    first.reply.send(PlaybackOutcome::Played).unwrap();
    let second = audible(&mut packets).await;
    assert_eq!(second.packet.pcm.as_slice(), &[1, 0]);
    assert_ne!(first.packet.group, second.packet.group);
    assert_ne!(first.packet.id, second.packet.id);
    let fourth = request(&mut requests).await;
    assert_eq!(fourth.index, 3);
    fourth.reply.send(Ok(audio(3))).unwrap();
    second.reply.send(PlaybackOutcome::Played).unwrap();
    for expected in [2, 3] {
        let next = audible(&mut packets).await;
        assert_eq!(next.packet.pcm.as_slice(), &[expected, 0]);
        next.reply.send(PlaybackOutcome::Played).unwrap();
    }
    assert_eq!(job.await.unwrap().unwrap(), PlaybackOutcome::Played);
    assert_eq!(counts.active.load(Ordering::Acquire), 0);
    assert!(counts.maximum.load(Ordering::Acquire) <= PREFETCH_SEGMENTS);
    assert!(child.is_cancelled());
    assert!(!generation.is_cancelled());
}

#[tokio::test]
async fn cancelled_synthesis_drops_every_owned_request_and_emits_no_audio() {
    let (provider, mut speaker, mut requests, mut packets, counts) = setup();
    let generation = CancellationToken::new();
    let running = generation.clone();
    let job =
        tokio::spawn(
            async move { speak(&text(), lease(), &provider, &mut speaker, &running).await },
        );
    let first = request(&mut requests).await;
    let second = request(&mut requests).await;
    generation.cancel();
    assert_eq!(job.await.unwrap().unwrap(), PlaybackOutcome::Interrupted);
    assert!(first.cancel.is_cancelled() && second.cancel.is_cancelled());
    assert_eq!(counts.active.load(Ordering::Acquire), 0);
    assert!(
        tokio::time::timeout(TEST_WAIT, packets.recv())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn failed_physical_playback_never_reports_the_whole_speech_as_played() {
    let (provider, mut speaker, mut requests, mut packets, counts) = setup();
    let generation = CancellationToken::new();
    let running = generation.clone();
    let job =
        tokio::spawn(
            async move { speak(&text(), lease(), &provider, &mut speaker, &running).await },
        );
    let first = request(&mut requests).await;
    let _second = request(&mut requests).await;
    first.reply.send(Ok(audio(0))).unwrap();
    let first = audible(&mut packets).await;
    first.reply.send(PlaybackOutcome::Failed).unwrap();
    assert_eq!(job.await.unwrap().unwrap(), PlaybackOutcome::Failed);
    assert_eq!(counts.active.load(Ordering::Acquire), 0);
    assert!(
        tokio::time::timeout(TEST_WAIT, packets.recv())
            .await
            .unwrap()
            .is_none()
    );
    assert!(!generation.is_cancelled());
}

#[tokio::test(start_paused = true)]
async fn provider_and_device_receipt_waits_have_finite_independent_deadlines() {
    let (provider, mut speaker, mut requests, _packets, counts) = setup();
    let running = CancellationToken::new();
    let job =
        tokio::spawn(
            async move { speak(&text(), lease(), &provider, &mut speaker, &running).await },
        );
    let _first = request(&mut requests).await;
    let _second = request(&mut requests).await;
    tokio::time::advance(PROVIDER_TIMEOUT).await;
    assert!(matches!(job.await.unwrap(), Err(Failure::TimedOut)));
    assert_eq!(counts.active.load(Ordering::Acquire), 0);
    let (provider, mut speaker, mut requests, mut packets, counts) = setup();
    let running = CancellationToken::new();
    let job =
        tokio::spawn(
            async move { speak(&text(), lease(), &provider, &mut speaker, &running).await },
        );
    let first = request(&mut requests).await;
    let _second = request(&mut requests).await;
    first.reply.send(Ok(audio(0))).unwrap();
    let first = audible(&mut packets).await;
    let wait = receipts::wait_duration(&first.packet).unwrap();
    tokio::time::advance(wait).await;
    assert!(matches!(job.await.unwrap(), Err(Failure::TimedOut)));
    assert_eq!(counts.active.load(Ordering::Acquire), 0);
}
