//! The workloads: sessions created and followed through the tab, then
//! driven, with every record the tab reads counted and timed.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bytes::Bytes;
use chrono::Utc;
use vessel_proto::{
    AgentEvent, Call, ContentBlock, CreateSession, GetNode, HarnessKind, Reply, SendInput,
    SessionEvent, SessionId, SessionInput, SessionMode, SessionSpec, StopReason, TerminalBytes,
    TerminalInput, TerminalSize,
};
use vessel_web::wire::{NodeKey, ServerFrame, StreamId};

use crate::fleet::Fleet;
use crate::tab::{Incoming, Tab};

/// One workload's figures, as the tab saw them.
pub(crate) struct Phase {
    pub(crate) records: u64,
    /// Terminal bytes, or for agent records their text's bytes.
    pub(crate) bytes: u64,
    pub(crate) elapsed: Duration,
    /// Each record's latency in microseconds, from its `at` on the node to
    /// the tab having decoded it.
    latencies: Vec<i64>,
}

impl Phase {
    /// The latency at `permille` (0 to 1000: the median is 500), in
    /// microseconds.
    pub(crate) fn latency(&self, permille: usize) -> i64 {
        let mut sorted = self.latencies.clone();
        sorted.sort_unstable();
        let last = sorted.len().saturating_sub(1);
        let index = last
            .checked_mul(permille.min(1000))
            .and_then(|scaled| scaled.checked_div(1000))
            .unwrap_or(last);
        sorted.get(index).copied().unwrap_or_default()
    }
}

/// The tab and the sessions it follows.
pub(crate) struct Run {
    tab: Tab,
    incoming: Incoming,
    node: NodeKey,
    terminals: Vec<SessionId>,
    agents: Vec<SessionId>,
}

impl Run {
    /// A tab on the fleet's UI, once the node is connected.
    pub(crate) async fn open(fleet: &Fleet) -> Self {
        let (mut tab, mut incoming) = Tab::open(&fleet.launch).await;
        let node: NodeKey = "load".parse().unwrap();
        // The bridge dials the node on its first call; until that call's
        // connection is up, a call can fail. Each attempt waits on the
        // bridge's answer, never on a timer.
        let mut attempts = 0_u32;
        loop {
            let id = tab.call(&node, &Call::NodeInfo(GetNode {})).await;
            if incoming.reply(id).await.is_ok() {
                break;
            }
            attempts = attempts.checked_add(1).unwrap();
            assert!(attempts < 100, "the node never connected");
            loop {
                if let ServerFrame::Node { .. } = incoming.next().await {
                    break;
                }
            }
        }
        Self {
            tab,
            incoming,
            node,
            terminals: Vec::new(),
            agents: Vec::new(),
        }
    }

    async fn create(&mut self, mode: SessionMode) -> SessionId {
        let spec = SessionSpec::new(HarnessKind::ClaudeCode, mode).with_cwd(std::env::temp_dir());
        let call = Call::CreateSession(CreateSession { spec });
        let id = self.tab.call(&self.node, &call).await;
        match self.incoming.reply(id).await {
            Ok(Reply::CreateSession(info)) => info.id,
            other => panic!("session.create: {other:?}"),
        }
    }

    /// `count` terminal sessions, followed, each up and prompting.
    pub(crate) async fn terminals(&mut self, count: usize) {
        for _session in 0..count {
            let mode = SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize {
                    cols: 120,
                    rows: 40,
                },
            };
            let session = self.create(mode).await;
            self.terminals.push(session);
        }
        let streams = self.follow(Kind::Terminal).await;
        let _setup = self.read(streams, Marker::Ready).await;
    }

    /// `count` structured sessions, followed, each ready.
    pub(crate) async fn structured(&mut self, count: usize) {
        for _session in 0..count {
            let mode = SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            };
            let session = self.create(mode).await;
            self.agents.push(session);
        }
        let streams = self.follow(Kind::Agent).await;
        let _setup = self.read(streams, Marker::AgentReady).await;
    }

    /// Follow every session of `kind`; how many streams that is.
    async fn follow(&mut self, kind: Kind) -> usize {
        let (sessions, base) = match kind {
            Kind::Terminal => (&self.terminals, 0_u64),
            Kind::Agent => (&self.agents, 1_000),
        };
        for (index, session) in sessions.iter().enumerate() {
            let index = u64::try_from(index).unwrap();
            let stream = StreamId::new(base.checked_add(index).unwrap());
            self.tab.follow(stream, &self.node, *session).await;
        }
        sessions.len()
    }

    /// Every terminal session writes `lines` lines of `width` bytes at once.
    pub(crate) async fn burst(&mut self, lines: u32, width: u32) -> Phase {
        let start = Instant::now();
        for session in self.terminals.clone() {
            self.keys(session, &flood(lines, width)).await;
        }
        let streams = self.terminals.len();
        let mut phase = self.read(streams, Marker::Flooded).await;
        phase.elapsed = start.elapsed();
        phase
    }

    /// For `seconds`, every 100 ms, every terminal session writes `lines`
    /// lines of `width` bytes; the phase ends when every session has
    /// finished every one of them.
    pub(crate) async fn paced(&mut self, lines: u32, width: u32, seconds: u32) -> Phase {
        let start = Instant::now();
        let ticks = seconds.checked_mul(10).unwrap();
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        let mut counter = Counter::new(Marker::Flooded, self.terminals.len(), ticks);
        let mut phase = Phase::empty();
        let mut sent = 0_u32;
        loop {
            tokio::select! {
                _now = tick.tick(), if sent < ticks => {
                    for session in self.terminals.clone() {
                        self.keys(session, &flood(lines, width)).await;
                    }
                    sent = sent.checked_add(1).unwrap();
                }
                frame = self.incoming.next() => {
                    phase.observe(&frame);
                    if counter.see(&frame) {
                        break;
                    }
                }
            }
        }
        phase.elapsed = start.elapsed();
        phase
    }

    /// Every structured session floods message chunks for `seconds`, then
    /// is cancelled; the phase ends when every turn has ended.
    pub(crate) async fn agent_flood(&mut self, seconds: u64) -> Phase {
        let start = Instant::now();
        for session in self.agents.clone() {
            self.input(
                session,
                SessionInput::Prompt {
                    content: vec![ContentBlock::Text {
                        text: "flood".to_owned(),
                    }],
                },
            )
            .await;
        }
        let deadline = tokio::time::Instant::now()
            .checked_add(Duration::from_secs(seconds))
            .unwrap();
        let mut phase = Phase::empty();
        while tokio::time::Instant::now() < deadline {
            tokio::select! {
                frame = self.incoming.next() => phase.observe(&frame),
                () = tokio::time::sleep_until(deadline) => {}
            }
        }
        for session in self.agents.clone() {
            self.input(session, SessionInput::CancelTurn { turn: None })
                .await;
        }
        let streams = self.agents.len();
        let mut tail = self.read(streams, Marker::Cancelled).await;
        tail.absorb(phase);
        tail.elapsed = start.elapsed();
        tail
    }

    async fn keys(&mut self, session: SessionId, text: &str) {
        let input = SessionInput::Terminal {
            input: TerminalInput::Data {
                data: TerminalBytes::new(Bytes::copy_from_slice(text.as_bytes())),
            },
        };
        self.input(session, input).await;
    }

    /// Send `input`; its reply is read (and ignored) with the records.
    async fn input(&mut self, session: SessionId, input: SessionInput) {
        let call = Call::SendInput(SendInput { session, input });
        let _id = self.tab.call(&self.node, &call).await;
    }

    /// Read until each of `streams` streams has shown `marker` once.
    async fn read(&mut self, streams: usize, marker: Marker) -> Phase {
        let mut counter = Counter::new(marker, streams, 1);
        let mut phase = Phase::empty();
        loop {
            let frame = self.incoming.next().await;
            phase.observe(&frame);
            if counter.see(&frame) {
                return phase;
            }
        }
    }
}

/// The sessions of one kind.
#[derive(Clone, Copy)]
enum Kind {
    Terminal,
    Agent,
}

fn flood(lines: u32, width: u32) -> String {
    format!("flood {lines} {}\r", width.saturating_sub(2))
}

/// What ends a wait on a stream.
#[derive(Clone, Copy)]
enum Marker {
    /// The terminal printed `ready` (`fake-tui` is up).
    Ready,
    /// The terminal printed `flooded ` (`fake-tui` finished a flood).
    Flooded,
    /// The agent is ready.
    AgentReady,
    /// The agent's turn ended cancelled.
    Cancelled,
}

impl Marker {
    const fn text(self) -> Option<&'static [u8]> {
        match self {
            Self::Ready => Some(b"ready"),
            Self::Flooded => Some(b"flooded "),
            Self::AgentReady | Self::Cancelled => None,
        }
    }

    fn agent(self, event: &AgentEvent) -> bool {
        match self {
            Self::AgentReady => matches!(event, AgentEvent::Ready { .. }),
            Self::Cancelled => matches!(
                event,
                AgentEvent::TurnEnded {
                    reason: StopReason::Cancelled,
                    usage: _,
                    ..
                }
            ),
            Self::Ready | Self::Flooded => false,
        }
    }
}

/// Counts a marker on each stream until every stream has shown it `want`
/// times. A terminal's text is searched across records (a marker can be
/// split between two reads), each occurrence counted once.
struct Counter {
    marker: Marker,
    want: u32,
    streams: usize,
    finished: usize,
    seen: HashMap<StreamId, (u32, Vec<u8>)>,
}

impl Counter {
    fn new(marker: Marker, streams: usize, want: u32) -> Self {
        Self {
            marker,
            want,
            streams,
            finished: 0,
            seen: HashMap::new(),
        }
    }

    /// Count `frame`; whether every stream is done.
    fn see(&mut self, frame: &ServerFrame) -> bool {
        let ServerFrame::Event { stream, record } = frame else {
            return false;
        };
        let (count, tail) = self.seen.entry(*stream).or_default();
        let before = *count;
        match (&record.event, self.marker.text()) {
            (SessionEvent::Terminal(output), Some(needle)) => {
                tail.extend_from_slice(output.data.bytes());
                while let Some(at) = tail.windows(needle.len()).position(|w| w == needle) {
                    drop(tail.drain(..at.checked_add(needle.len()).unwrap()));
                    *count = count.saturating_add(1);
                }
                let keep = tail.len().saturating_sub(needle.len());
                drop(tail.drain(..keep));
            }
            (SessionEvent::Agent { event }, None) if self.marker.agent(event) => {
                *count = count.saturating_add(1);
            }
            _ => {}
        }
        if before < self.want && *count >= self.want {
            self.finished = self.finished.checked_add(1).unwrap();
        }
        self.finished == self.streams
    }
}

impl Phase {
    fn empty() -> Self {
        Self {
            records: 0,
            bytes: 0,
            elapsed: Duration::ZERO,
            latencies: Vec::new(),
        }
    }

    /// Count `frame` if it is a record.
    fn observe(&mut self, frame: &ServerFrame) {
        let ServerFrame::Event { record, .. } = frame else {
            return;
        };
        let latency = Utc::now()
            .signed_duration_since(record.at)
            .num_microseconds()
            .unwrap_or(i64::MAX);
        self.latencies.push(latency);
        self.records = self.records.checked_add(1).unwrap();
        let bytes = match &record.event {
            SessionEvent::Terminal(output) => output.data.bytes().len(),
            SessionEvent::Agent {
                event:
                    AgentEvent::MessageChunk {
                        content: ContentBlock::Text { text },
                    },
            } => text.len(),
            SessionEvent::Status { .. }
            | SessionEvent::Agent { .. }
            | SessionEvent::ProfileChanged { .. }
            | SessionEvent::ModelCall(_)
            | SessionEvent::OptionsChanged { .. }
            | SessionEvent::Renamed { .. }
            | SessionEvent::QueueChanged { .. }
            | SessionEvent::CommandsChanged { .. }
            | SessionEvent::UsageChanged { .. }
            | SessionEvent::Parked { .. }
            | SessionEvent::Woken
            | SessionEvent::Forked { .. }
            | SessionEvent::Rewound { .. }
            | SessionEvent::Reloaded { .. } => 0,
        };
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(bytes).unwrap())
            .unwrap();
    }

    fn absorb(&mut self, other: Self) {
        self.records = self.records.checked_add(other.records).unwrap();
        self.bytes = self.bytes.checked_add(other.bytes).unwrap();
        self.latencies.extend(other.latencies);
    }
}
