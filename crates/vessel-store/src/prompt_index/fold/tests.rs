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
            AgentEvent::MessageChunk {
                content: text("first reply"),
            },
        ),
        agent(
            2,
            AgentEvent::UserMessageChunk {
                content: text("  한"),
            },
        ),
        record(3, SessionEvent::Woken),
        agent(
            4,
            AgentEvent::UserMessageChunk {
                content: text("글\n history"),
            },
        ),
        agent(
            5,
            AgentEvent::MessageChunk {
                content: text("assistant is excluded"),
            },
        ),
        agent(
            6,
            AgentEvent::Steered {
                content: vec![text("steered")],
            },
        ),
        prompt(7, "superseded"),
        record(
            8,
            SessionEvent::Rewound {
                after: EventSeq::new(6),
                harness_session: "fake-history".parse().unwrap(),
                files_changed: 0,
            },
        ),
        prompt(9, "new branch"),
    ] {
        index.take(&event).unwrap();
    }
    let page = index.page(EventSeq::new(0), &request()).unwrap();
    assert_eq!(
        page.entries()
            .iter()
            .map(|entry| (entry.seq.get(), entry.preview.as_str()))
            .collect::<Vec<_>>(),
        [
            (0, "first"),
            (2, "한글 history"),
            (6, "steered"),
            (9, "new branch")
        ]
    );
    assert_eq!(page.epoch().rewound_at(), Some(EventSeq::new(8)));
}

#[test]
fn chunks_after_a_turn_start_extend_its_one_prompt_instead_of_duplicating_it() {
    let mut index = Index::default();
    index.take(&prompt(0, "한")).unwrap();
    index
        .take(&agent(
            1,
            AgentEvent::UserMessageChunk {
                content: text("글"),
            },
        ))
        .unwrap();
    let page = index.page(EventSeq::ZERO, &request()).unwrap();
    assert_eq!(page.entries().len(), 1);
    assert_eq!(page.entries()[0].seq, EventSeq::ZERO);
    assert_eq!(page.entries()[0].preview.as_str(), "한글");
}

#[test]
fn previews_keep_links_and_replayed_file_names_as_the_existing_timeline_does() {
    use vessel_proto::AttachmentId;
    use vessel_proto::conversation::{AttachmentKind, AttachmentRef};

    let file = ContentBlock::Attachment {
        attachment: AttachmentRef {
            id: AttachmentId::random(),
            name: "diagram.png".parse().unwrap(),
            kind: AttachmentKind::Image,
            mime: "image/png".parse().unwrap(),
            size: 1,
        },
    };
    let mut index = Index::default();
    index
        .take(&agent(
            0,
            AgentEvent::TurnStarted {
                content: vec![
                    text("Read "),
                    ContentBlock::ResourceLink {
                        name: "design".to_owned(),
                        uri: "file:///design.md".to_owned(),
                    },
                    file.clone(),
                ],
            },
        ))
        .unwrap();
    index
        .take(&agent(
            1,
            AgentEvent::MessageChunk {
                content: text("reply"),
            },
        ))
        .unwrap();
    index
        .take(&agent(2, AgentEvent::UserMessageChunk { content: file }))
        .unwrap();
    let page = index.page(EventSeq::ZERO, &request()).unwrap();
    assert_eq!(
        page.entries()[0].preview.as_str(),
        "Read [design](file:///design.md)"
    );
    assert_eq!(page.entries()[1].preview.as_str(), "[diagram.png]");
}

#[test]
fn work_and_metadata_budgets_refuse_without_silently_omitting_prompts() {
    let mut scanned = Index {
        scanned: SCAN_RECORDS,
        ..Default::default()
    };
    assert!(matches!(
        scanned.take(&prompt(0, "excess")),
        Err(StoreError::PromptIndexRecords)
    ));
    let mut index = Index::default();
    let record = prompt(0, "bounded");
    for seq in 0..=INDEX_ENTRIES {
        let mut entry = record.clone();
        entry.seq = EventSeq::new(u64::try_from(seq).unwrap());
        index.take(&entry).unwrap();
    }
    assert!(matches!(
        index.page(EventSeq::ZERO, &request()),
        Err(StoreError::PromptIndexPrompts)
    ));
}

#[test]
fn append_preserves_revision_and_cursor_while_rewind_or_retention_refuses_it() {
    let mut first = Index::default();
    first.take(&prompt(0, "first")).unwrap();
    first.take(&prompt(1, "second")).unwrap();
    let mut ask = request();
    ask.limit = PromptIndexLimit::new(1).unwrap();
    let page = first.page(EventSeq::new(0), &ask).unwrap();
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
    let continued = build().page(EventSeq::new(0), &ask).unwrap();
    assert_eq!(continued.entries().first().unwrap().seq, EventSeq::new(1));
    assert_eq!(continued.epoch(), page.epoch());
    assert!(matches!(
        build().page(EventSeq::new(1), &ask),
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
        rewound.page(EventSeq::new(0), &ask),
        Err(StoreError::PromptIndexChanged)
    ));
}
