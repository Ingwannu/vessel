//! A demo fleet for the browser build: two machines, their sessions on the
//! board, and conversations made of the wire's own records, answered as a
//! node would (a prompt starts a turn and is echoed, an answer resolves a
//! permission).
//!
//! [Decision Log]
//! - Purpose: the browser track asks whether the app's own screens draw
//!   and take typing in a browser (docs/plans/backend-m12-browser.md,
//!   questions 1 and 2). That needs data on the screens, and the browser
//!   has no `vessel-client` yet (question 8 says what it would take).
//! - Constraint: the screens must not know which they are given; the data
//!   has to arrive through the same `Event`s, as `EventRecord`s folded by
//!   `view`, so what is drawn is what a real stream would draw.
//! - Alternatives: records recorded from a real node into a file (the CLI
//!   has no JSON output for a log, so that is a tool to write first); a
//!   websocket client in the browser (question 8: more than a day).
//! - Choice: records built here from `vessel-proto`'s types, with the
//!   texts the questions need (Hangul, Markdown, a long paragraph that
//!   wraps, a permission waiting, a long conversation to scroll).
//! - Cost: nothing here is a node's behaviour beyond the echo; a session's
//!   board phrase is written, not derived. It is compiled for wasm32 and
//!   for tests only.

use std::collections::HashMap;
use std::str::FromStr as _;

use chrono::{DateTime, Duration, Utc};
use vessel_app_view::board::CardDetail;
use vessel_app_view::{Agent, Attention, Card, Fleet, Lane, Machine, Phrase, Reach, agent_name};
use vessel_proto::{
    AgentEvent, ContentBlock, DisplayStatus, EventRecord, EventSeq, HarnessKind, HarnessRef,
    OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId, PROTOCOL_VERSION,
    PermissionId, PermissionOption, PermissionOptionId, PermissionOptionKind, PermissionOutcome,
    PermissionRequest, SessionEvent, SessionId, SessionOption, StopReason, ToolCall, ToolCallId,
    ToolCallStatus, ToolCallUpdate, ToolKind,
};

/// The demo's sessions, each with its log.
#[derive(Debug)]
pub(crate) struct Demo {
    cards: Vec<Card>,
    logs: HashMap<SessionId, Vec<EventRecord>>,
    /// Ids for sessions started from the app, in order (fixed, as above).
    spare: Vec<SessionId>,
}

/// The fixed ids (UUIDs written out: a random one needs the browser's
/// crypto through `getrandom`, which nothing else here needs).
const IDS: [&str; 5] = [
    "6f1c2a3e-0000-4000-8000-000000000001",
    "6f1c2a3e-0000-4000-8000-000000000002",
    "6f1c2a3e-0000-4000-8000-000000000003",
    "6f1c2a3e-0000-4000-8000-000000000004",
    "6f1c2a3e-0000-4000-8000-000000000005",
];
/// Ids for conversations started in the demo (one page's worth).
const SPARE: [&str; 8] = [
    "6f1c2a3e-0000-4000-8000-0000000000b1",
    "6f1c2a3e-0000-4000-8000-0000000000b2",
    "6f1c2a3e-0000-4000-8000-0000000000b3",
    "6f1c2a3e-0000-4000-8000-0000000000b4",
    "6f1c2a3e-0000-4000-8000-0000000000b5",
    "6f1c2a3e-0000-4000-8000-0000000000b6",
    "6f1c2a3e-0000-4000-8000-0000000000b7",
    "6f1c2a3e-0000-4000-8000-0000000000b8",
];
const PERMISSION: &str = "6f1c2a3e-0000-4000-8000-0000000000a1";

/// The paragraph the first conversation answers with: Markdown, Hangul
/// and Latin, long enough to wrap at any width a phone has.
const SUMMARY: &str = "이 저장소는 **Vessel**입니다. 여러 머신에서 코딩 에이전트 세션을 \
돌리고, 어디서든 붙어서 보고 답할 수 있게 하는 노드(`vesseld`)와 클라이언트로 되어 \
있습니다. The node keeps every session's log as numbered records, so a client that \
drops its connection resumes where it left off and nothing is lost or repeated.\n\n\
- `crates/vessel-node`: 데몬 조립\n- `crates/vessel-client`: 타입이 있는 클라이언트\n\
- `web/`: 브라우저용 페이지 (React)\n\n다음으로 테스트를 돌려 볼까요?";

/// How many entries the long conversation has (a phone scrolls it).
const LONG: u64 = 400;

impl Demo {
    /// The fleet and every session's log, as of `now`.
    pub(crate) fn new(now: DateTime<Utc>) -> Self {
        let mut demo = Self {
            cards: Vec::new(),
            logs: HashMap::new(),
            spare: SPARE
                .iter()
                .rev()
                .filter_map(|text| SessionId::from_str(text).ok())
                .collect(),
        };
        // The ids are constants the tests parse; were one to stop parsing,
        // the demo would come up empty rather than panic.
        let parsed: Vec<SessionId> = IDS
            .iter()
            .filter_map(|text| SessionId::from_str(text).ok())
            .collect();
        let Ok([first, hangul, long, docs, ready]) = <[SessionId; 5]>::try_from(parsed) else {
            return demo;
        };
        demo.add(
            card(
                "laptop",
                &first,
                "vessel-client",
                Lane::Building,
                DisplayStatus::Blocked,
                now,
                1,
            ),
            first_conversation(now),
        );
        demo.add(
            card(
                "laptop",
                &hangul,
                "한글 세션: 문서 다듬기",
                Lane::InReview,
                DisplayStatus::NeedsHumanReview,
                now,
                7,
            ),
            hangul_conversation(now),
        );
        demo.add(
            card(
                "build-server",
                &long,
                "ci-fixes",
                Lane::Validating,
                DisplayStatus::FixingCi,
                now,
                2,
            ),
            long_conversation(now),
        );
        demo.add(
            card(
                "build-server",
                &docs,
                "docs",
                Lane::Building,
                DisplayStatus::Working,
                now,
                0,
            ),
            Vec::new(),
        );
        demo.add(
            card(
                "build-server",
                &ready,
                "release notes",
                Lane::Ready,
                DisplayStatus::Mergeable,
                now,
                30,
            ),
            Vec::new(),
        );
        demo
    }

    fn add(&mut self, card: Card, records: Vec<EventRecord>) {
        let _earlier = self.logs.insert(card.session, records);
        self.cards.push(card);
    }

    /// The machines and their sessions.
    pub(crate) fn fleet(&self, now: DateTime<Utc>) -> Fleet {
        let on = |machine: &str| -> Vec<Card> {
            self.cards
                .iter()
                .filter(|card| card.machine == machine)
                .cloned()
                .collect()
        };
        // The two reachable machines differ only by name.
        let online = |name: &str| Machine {
            node: None,
            name: name.to_owned(),
            reach: Reach::Online,
            sessions: on(name),
            agents: agents(),
            protocol: Some(PROTOCOL_VERSION),
            version: None,
        };
        Fleet {
            machines: vec![
                online("laptop"),
                online("build-server"),
                Machine::unreachable("old-box".to_owned(), "no route: this is the browser's demo"),
            ],
            read_at: Some(now),
        }
    }

    /// Start a conversation on `machine` with `prompt` as its first turn:
    /// its card (to open), or `None` when the demo's spare ids are spent.
    pub(crate) fn start(
        &mut self,
        machine: &str,
        harness: &HarnessRef,
        prompt: &str,
        now: DateTime<Utc>,
    ) -> Option<Card> {
        let session = self.spare.pop()?;
        let title = prompt.lines().next().unwrap_or(prompt).to_owned();
        let mut started = card(
            machine,
            &session,
            &title,
            Lane::Building,
            DisplayStatus::Idle,
            now,
            0,
        );
        started.agent = agent_name(harness);
        self.add(started.clone(), Vec::new());
        let _added = self.prompt(&session, prompt, now);
        Some(started)
    }

    /// A session's whole log so far.
    pub(crate) fn records(&self, session: &SessionId) -> Vec<EventRecord> {
        self.logs.get(session).cloned().unwrap_or_default()
    }

    /// A prompt: the turn starts with it and the agent echoes it. Returns
    /// the records added.
    pub(crate) fn prompt(
        &mut self,
        session: &SessionId,
        text: &str,
        now: DateTime<Utc>,
    ) -> Vec<EventRecord> {
        let reply = format!("받았습니다 (echo): {text}");
        self.append(
            session,
            now,
            vec![
                AgentEvent::TurnStarted {
                    content: vec![ContentBlock::Text {
                        text: text.to_owned(),
                    }],
                },
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text { text: reply },
                },
                AgentEvent::TurnEnded {
                    point: None,
                    reason: StopReason::EndTurn,
                    usage: None,
                },
            ],
        )
    }

    /// An answer to the waiting permission: resolved, the tool call done
    /// (or failed when refused), the turn over.
    pub(crate) fn answer(
        &mut self,
        session: &SessionId,
        id: PermissionId,
        option: PermissionOptionId,
        now: DateTime<Utc>,
    ) -> Vec<EventRecord> {
        let allowed = option.as_str() != "reject";
        let status = if allowed {
            ToolCallStatus::Completed
        } else {
            ToolCallStatus::Failed
        };
        let words = if allowed {
            "테스트 412개가 모두 통과했습니다. All 412 tests passed."
        } else {
            "Not run: the permission was refused."
        };
        self.append(
            session,
            now,
            vec![
                AgentEvent::PermissionResolved {
                    id,
                    outcome: PermissionOutcome::Selected { option },
                },
                AgentEvent::ToolCallUpdate(update(TEST_CALL, status)),
                AgentEvent::MessageChunk {
                    content: ContentBlock::Text {
                        text: words.to_owned(),
                    },
                },
                AgentEvent::TurnEnded {
                    point: None,
                    reason: StopReason::EndTurn,
                    usage: None,
                },
            ],
        )
    }

    fn append(
        &mut self,
        session: &SessionId,
        now: DateTime<Utc>,
        events: Vec<AgentEvent>,
    ) -> Vec<EventRecord> {
        let Some(log) = self.logs.get_mut(session) else {
            return Vec::new();
        };
        let start = u64::try_from(log.len()).unwrap_or(u64::MAX);
        let added = records(start, now, events);
        log.extend(added.iter().cloned());
        added
    }
}

const TEST_CALL: &str = "call-test";

/// What every demo agent offers before a session: a model, an effort and
/// a permission mode (the new chat's pickers), as an ACP agent lists them.
pub(crate) fn options() -> Vec<SessionOption> {
    let select =
        |id: &str, name: &str, category, current: &str, values: &[(&str, &str)]| SessionOption {
            id: OptionId::new(id),
            name: name.to_owned(),
            description: None,
            category,
            kind: OptionKind::Select {
                current: OptionValueId::new(current),
                values: values
                    .iter()
                    .map(|(value, words)| OptionValue {
                        id: OptionValueId::new(*value),
                        name: (*words).to_owned(),
                        description: None,
                        group: None,
                    })
                    .collect(),
            },
        };
    vec![
        select(
            "model",
            "Model",
            OptionCategory::Model,
            "opus",
            &[
                ("opus", "Opus 5.5"),
                ("sonnet", "Sonnet 5.5"),
                ("haiku", "Haiku 5.5"),
            ],
        ),
        select(
            "effort",
            "Effort",
            OptionCategory::ThoughtLevel,
            "high",
            &[("low", "Low"), ("medium", "Medium"), ("high", "High")],
        ),
        select(
            "mode",
            "Permissions",
            OptionCategory::Mode,
            "default",
            &[
                ("default", "Ask"),
                ("acceptEdits", "Accept edits"),
                ("plan", "Plan"),
            ],
        ),
    ]
}

fn agents() -> Vec<Agent> {
    [HarnessKind::ClaudeCode, HarnessKind::Codex]
        .into_iter()
        .map(|kind| {
            let harness = HarnessRef::Known(kind);
            Agent {
                name: agent_name(&harness),
                harness,
                unavailable: None,
                not_installed: false,
            }
        })
        .collect()
}

fn card(
    machine: &str,
    session: &SessionId,
    title: &str,
    lane: Lane,
    status: DisplayStatus,
    now: DateTime<Utc>,
    minutes_ago: i64,
) -> Card {
    Card {
        session: *session,
        structured: true,
        live: true,
        title: title.to_owned(),
        machine: machine.to_owned(),
        agent: agent_name(&HarnessRef::Known(HarnessKind::ClaudeCode)),
        lane: Some(lane),
        phrase: Phrase::Node(status),
        attention: if matches!(status, DisplayStatus::Blocked) {
            Attention::NeedsYou
        } else if matches!(status, DisplayStatus::Working | DisplayStatus::FixingCi) {
            Attention::Working
        } else {
            Attention::AtRest
        },
        at: now
            .checked_sub_signed(Duration::minutes(minutes_ago))
            .unwrap_or(now),
        detail: CardDetail {
            harness: Some(HarnessRef::Known(HarnessKind::ClaudeCode)),
            ..CardDetail::default()
        },
    }
}

/// Records numbered from `start`, all at `now`.
fn records(start: u64, now: DateTime<Utc>, events: Vec<AgentEvent>) -> Vec<EventRecord> {
    let mut seq = EventSeq::new(start);
    let mut out = Vec::with_capacity(events.len());
    for event in events {
        out.push(EventRecord {
            seq,
            at: now,
            event: SessionEvent::Agent { event },
        });
        match seq.next() {
            Some(next) => seq = next,
            None => break,
        }
    }
    out
}

fn text(words: &str) -> ContentBlock {
    ContentBlock::Text {
        text: words.to_owned(),
    }
}

fn call(id: &str, title: &str, kind: ToolKind, status: ToolCallStatus) -> ToolCall {
    ToolCall {
        id: ToolCallId::new(id),
        title: title.to_owned(),
        kind,
        status,
        content: Vec::new(),
        locations: Vec::new(),
        raw_input: None,
    }
}

fn update(id: &str, status: ToolCallStatus) -> ToolCallUpdate {
    ToolCallUpdate {
        id: ToolCallId::new(id),
        title: None,
        kind: None,
        status: Some(status),
        content: None,
        locations: None,
        raw_input: None,
        raw_output: None,
    }
}

fn option(id: &str, name: &str, kind: PermissionOptionKind) -> PermissionOption {
    PermissionOption {
        id: PermissionOptionId::new(id),
        name: name.to_owned(),
        kind,
    }
}

fn first_conversation(now: DateTime<Utc>) -> Vec<EventRecord> {
    let permission = PermissionId::from_str(PERMISSION).ok();
    let mut events = vec![
        AgentEvent::TurnStarted {
            content: vec![text("README를 읽고 이 저장소가 무엇인지 요약해 줘")],
        },
        AgentEvent::ThoughtChunk {
            content: text("Reading the README first, then the crate map."),
        },
        AgentEvent::ToolCall(call(
            "call-read",
            "Read README.md",
            ToolKind::Read,
            ToolCallStatus::Completed,
        )),
        AgentEvent::MessageChunk {
            content: text(SUMMARY),
        },
        AgentEvent::TurnEnded {
            point: None,
            reason: StopReason::EndTurn,
            usage: None,
        },
        AgentEvent::TurnStarted {
            content: vec![text("응, 테스트 돌려 줘")],
        },
        AgentEvent::ToolCall(call(
            TEST_CALL,
            "cargo test --workspace",
            ToolKind::Execute,
            ToolCallStatus::Pending,
        )),
    ];
    if let Some(id) = permission {
        events.push(AgentEvent::PermissionRequested(PermissionRequest {
            id,
            tool_call: ToolCallId::new(TEST_CALL),
            options: vec![
                option("allow-once", "Allow once", PermissionOptionKind::AllowOnce),
                option(
                    "allow-always",
                    "Always allow",
                    PermissionOptionKind::AllowAlways,
                ),
                option("reject", "Reject", PermissionOptionKind::RejectOnce),
            ],
        }));
    }
    records(0, now, events)
}

fn hangul_conversation(now: DateTime<Utc>) -> Vec<EventRecord> {
    records(
        0,
        now,
        vec![
            AgentEvent::TurnStarted {
                content: vec![text("설치 안내 문서를 읽기 쉽게 고쳐 줘. 문장은 짧게.")],
            },
            AgentEvent::MessageChunk {
                content: text(
                    "고친 안내입니다.\n\n1. 노드를 설치합니다: `vessel init`\n2. 휴대폰이나 다른 \
                     컴퓨터에서 짝을 맺습니다: `vessel node pair`\n3. 나온 링크를 앱에 붙여 넣습니다.\n\n\
                     문장마다 한 가지 일만 말하도록 나눴고, 명령은 모두 코드로 표시했습니다.",
                ),
            },
            AgentEvent::TurnEnded {
                point: None,
                reason: StopReason::EndTurn,
                usage: None,
            },
        ],
    )
}

fn long_conversation(now: DateTime<Utc>) -> Vec<EventRecord> {
    let mut events = Vec::new();
    for index in 0..LONG {
        let entry = if index % 4 == 0 {
            AgentEvent::TurnStarted {
                content: vec![text(&format!("step {index}: 다음 실패를 고쳐 줘"))],
            }
        } else if index % 4 == 3 {
            AgentEvent::TurnEnded {
                point: None,
                reason: StopReason::EndTurn,
                usage: None,
            }
        } else {
            AgentEvent::MessageChunk {
                content: text(&format!(
                    "Entry {index}. 실패한 검사를 다시 돌리고, 원인을 찾아 고쳤습니다. \
                     The fix keeps the stream's order."
                )),
            }
        };
        events.push(entry);
    }
    records(0, now, events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::Conversation;

    fn fold(records: &[EventRecord]) -> Conversation {
        let mut conversation = Conversation::default();
        for record in records {
            conversation.apply(record);
        }
        conversation
    }

    #[test]
    fn every_fixed_id_parses() {
        for id in IDS.iter().chain(&SPARE).chain(std::iter::once(&PERMISSION)) {
            assert!(uuid_ok(id), "{id}");
        }
    }

    fn uuid_ok(text: &str) -> bool {
        SessionId::from_str(text).is_ok()
    }

    fn id(index: usize) -> SessionId {
        IDS.get(index)
            .and_then(|text| SessionId::from_str(text).ok())
            .unwrap_or_else(SessionId::random)
    }

    #[test]
    fn the_first_conversation_waits_on_a_permission() {
        let demo = Demo::new(Utc::now());
        let first = id(0);
        let conversation = fold(&demo.records(&first));
        assert_eq!(conversation.asked().len(), 1);
        assert!(conversation.turn_running());
    }

    #[test]
    fn a_prompt_continues_the_log_and_is_echoed() {
        let mut demo = Demo::new(Utc::now());
        let session = id(1);
        let before = demo.records(&session).len();
        let added = demo.prompt(&session, "안녕", Utc::now());
        let seq = added.first().map(|record| record.seq.get());
        assert_eq!(seq, Some(u64::try_from(before).unwrap_or(u64::MAX)));
        let conversation = fold(&demo.records(&session));
        let last = conversation
            .entries()
            .last()
            .map(|entry| entry.text.clone());
        assert_eq!(last.as_deref(), Some("받았습니다 (echo): 안녕"));
    }

    #[test]
    fn an_answer_clears_the_permission() {
        let mut demo = Demo::new(Utc::now());
        let session = id(0);
        let asked = fold(&demo.records(&session)).asked().first().cloned();
        let Some(asked) = asked else {
            return;
        };
        let option = asked.options.first().map(|option| option.id.clone());
        let Some(option) = option else {
            return;
        };
        let _added = demo.answer(&session, asked.id, option, Utc::now());
        let conversation = fold(&demo.records(&session));
        assert!(conversation.asked().is_empty());
        assert!(!conversation.turn_running());
    }

    #[test]
    fn the_new_chat_gets_a_model_an_effort_and_a_mode() {
        let categories: Vec<OptionCategory> =
            options().iter().map(|option| option.category).collect();
        assert_eq!(
            categories,
            [
                OptionCategory::Model,
                OptionCategory::ThoughtLevel,
                OptionCategory::Mode
            ]
        );
    }

    #[test]
    fn the_fleet_has_every_card_once() {
        let demo = Demo::new(Utc::now());
        let fleet = demo.fleet(Utc::now());
        let cards: usize = fleet
            .machines
            .iter()
            .map(|machine| machine.sessions.len())
            .sum();
        assert_eq!(cards, IDS.len());
    }

    #[test]
    fn a_started_conversation_is_on_the_board_with_its_prompt() {
        let mut demo = Demo::new(Utc::now());
        let harness = HarnessRef::Known(HarnessKind::Codex);
        let started = demo.start("laptop", &harness, "새 대화\n둘째 줄", Utc::now());
        let Some(card) = started else {
            return;
        };
        assert_eq!(card.title, "새 대화");
        let fleet = demo.fleet(Utc::now());
        assert!(fleet.machines.iter().any(|machine| {
            machine
                .sessions
                .iter()
                .any(|other| other.session == card.session)
        }));
        let conversation = fold(&demo.records(&card.session));
        let first = conversation
            .entries()
            .first()
            .map(|entry| entry.text.clone());
        assert_eq!(first.as_deref(), Some("새 대화\n둘째 줄"));
    }
}
