//! Projection tests include hidden history, chunks and appends across pages.
use super::*;
use vessel_proto::SessionId;
use vessel_proto::prompt_index::PromptIndexLimit;

fn record(seq: u64, event: SessionEvent) -> EventRecord {
    EventRecord {
        seq: EventSeq::new(seq),
        at: "2026-01-01T00:00:00Z".parse().unwrap(),
        event,
    }
}
fn agent(seq: u64, event: AgentEvent) -> EventRecord {
    record(seq, SessionEvent::Agent { event })
}
fn text(value: &str) -> ContentBlock {
    ContentBlock::Text {
        text: value.to_owned(),
    }
}
fn prompt(seq: u64, value: &str) -> EventRecord {
    agent(
        seq,
        AgentEvent::TurnStarted {
            content: vec![text(value)],
        },
    )
}
fn request() -> ListSessionPrompts {
    ListSessionPrompts {
        session: SessionId::random(),
        after: None,
        epoch: None,
        limit: PromptIndexLimit::default(),
    }
}

#[test]
fn chunked_history_steering_and_rewind_produce_only_visible_prompts() {
    let mut index = Index::default();
    for event in [
        prompt(0, "first"),
        agent(
            1,
            AgentEvent::UserMessageChunk {
                content: text("  한"),
            },
        ),
        record(2, SessionEvent::Woken),
        agent(
            3,
            AgentEvent::UserMessageChunk {
                content: text("글\n history"),
            },
        ),
        agent(
            4,
            AgentEvent::MessageChunk {
                content: text("assistant is excluded"),
            },
        ),
        agent(
            5,
            AgentEvent::Steered {
                content: vec![text("steered")],
            },
        ),
        prompt(6, "superseded"),
        record(
            7,
            SessionEvent::Rewound {
                after: EventSeq::new(5),
                harness_session: "fake-history".parse().unwrap(),
                files_changed: 0,
            },
        ),
        prompt(8, "new branch"),
    ] {
        index.take(&event).unwrap();
    }
    let page = index.page(EventSeq::new(0), request()).unwrap();
    assert_eq!(
        page.entries()
            .iter()
            .map(|entry| (entry.seq.get(), entry.preview.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "first"),
            (1, "한글 history"),
            (5, "steered"),
            (8, "new branch")
        ]
    );
    assert_eq!(page.epoch().rewound_at(), Some(EventSeq::new(7)));
}

#[test]
fn append_preserves_revision_and_cursor_while_rewind_or_retention_refuses_it() {
    let mut first = Index::default();
    first.take(&prompt(0, "first")).unwrap();
    first.take(&prompt(1, "second")).unwrap();
    let mut ask = request();
    ask.limit = PromptIndexLimit::new(1).unwrap();
    let page = first.page(EventSeq::new(0), ask.clone()).unwrap();
    assert_eq!(page.next(), Some(EventSeq::new(0)));
    ask.after = page.next();
    ask.epoch = Some(page.epoch());
    let build = || {
        let mut index = Index::default();
        for seq in 0..3 {
            index.take(&prompt(seq, "appended")).unwrap();
        }
        index
    };
    let continued = build().page(EventSeq::new(0), ask.clone()).unwrap();
    assert_eq!(continued.entries().first().unwrap().seq, EventSeq::new(1));
    assert_eq!(continued.epoch(), page.epoch());
    assert!(matches!(
        build().page(EventSeq::new(1), ask.clone()),
        Err(StoreError::PromptIndexChanged)
    ));
    let mut rewound = build();
    rewound
        .take(&record(
            3,
            SessionEvent::Rewound {
                after: EventSeq::new(0),
                harness_session: "history".parse().unwrap(),
                files_changed: 0,
            },
        ))
        .unwrap();
    assert!(matches!(
        rewound.page(EventSeq::new(0), ask),
        Err(StoreError::PromptIndexChanged)
    ));
}
