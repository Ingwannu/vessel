use super::*;
use crate::conversation::{Delivery, PromptSession};
use crate::{AgentEvent, Call, SessionId};

fn context_json() -> serde_json::Value {
    serde_json::json!({
        "type": "context",
        "kind": "pull_check",
        "title": "test (linux)",
        "text": "GitHub pull request check\nName: test (linux)\nConclusion: failure",
        "url": "https://github.com/acme-labs/fleet-agent/runs/106",
    })
}

fn context() -> ContentBlock {
    serde_json::from_value(context_json()).unwrap()
}

fn review_json() -> serde_json::Value {
    serde_json::json!({
        "type": "review",
        "root": "/work/fleet-agent",
        "mode": "uncommitted",
        "comments": [{
            "path": "src/upload/retry.rs",
            "side": "new",
            "line": 48,
            "body": "Cap this at 30 seconds.",
            "hunk_header": "@@ -40,6 +40,9 @@ fn backoff",
            "target": { "new": 48, "kind": "add", "content": "sleep(delay);" },
            "lines": [
                { "old": 46, "new": 46, "kind": "context", "content": "let delay = base * 2;" },
                { "new": 47, "kind": "add", "content": "let delay = delay.min(max);" },
                { "new": 48, "kind": "add", "content": "sleep(delay);" },
            ],
        }],
    })
}

fn review() -> ContentBlock {
    serde_json::from_value(review_json()).unwrap()
}

fn before() -> ProtocolVersion {
    ProtocolVersion::new(PROMPT_CONTEXT_PROTOCOL.unwrap().get().saturating_sub(1))
}

#[test]
fn a_context_and_a_review_keep_their_wire_shape() {
    let block = context();
    assert_eq!(serde_json::to_value(&block).unwrap(), context_json());
    let ContentBlock::Context(read) = &block else {
        panic!("a context block");
    };
    assert_eq!(read.kind, ContextKind::PullCheck);
    assert_eq!(read.title.as_str(), "test (linux)");
    let block = review();
    assert_eq!(serde_json::to_value(&block).unwrap(), review_json());
}

#[test]
fn a_value_past_its_rule_does_not_decode() {
    for (field, value) in [
        ("title", serde_json::json!("")),
        ("title", serde_json::json!("two\nlines")),
        ("title", serde_json::json!("t".repeat(201))),
        ("text", serde_json::json!("   ")),
        ("text", serde_json::json!("nul\0byte")),
        ("text", serde_json::json!("x".repeat(BLOCK_BYTES.saturating_add(1)))),
        ("url", serde_json::json!("ftp://example.com/log")),
        ("url", serde_json::json!("https://exa mple.com")),
    ] {
        let mut json = context_json();
        json[field] = value;
        assert!(
            serde_json::from_value::<ContentBlock>(json).is_err(),
            "{field}"
        );
    }
    let mut json = review_json();
    json["comments"][0]["body"] = serde_json::json!(" ");
    assert!(serde_json::from_value::<ContentBlock>(json).is_err());
    let mut json = review_json();
    json["comments"][0]["lines"][0]["content"] = serde_json::json!("a\nb");
    assert!(serde_json::from_value::<ContentBlock>(json).is_err());
}

#[test]
fn a_review_reads_as_paseo_s_review_text() {
    assert_eq!(
        agent_text(&review()).unwrap(),
        "Vessel review attachment (uncommitted)\n\
         CWD: /work/fleet-agent\n\
         \n\
         Comment 1: src/upload/retry.rs:new:48\n\
         Cap this at 30 seconds.\n\
         @@ -40,6 +40,9 @@ fn backoff\n  \
         46 46  let delay = base * 2;\n   \
         - 47 +let delay = delay.min(max);\n\
         >  - 48 +sleep(delay);"
    );
    let ContentBlock::Context(read) = context() else {
        panic!("a context block");
    };
    assert_eq!(
        agent_text(&ContentBlock::Context(read.clone())).unwrap(),
        read.text.as_str()
    );
    assert_eq!(
        agent_text(&ContentBlock::Text {
            text: "plain".to_owned()
        }),
        None
    );
}

#[test]
fn a_review_keeps_its_bounds_as_a_whole() {
    let ContentBlock::Review(one) = review() else {
        panic!("a review block");
    };
    assert_eq!(check(&[ContentBlock::Review(one.clone())]), Ok(()));
    let mut none = one.clone();
    none.comments.clear();
    assert_eq!(none.check(), Err(ContextProblem::Comments(0)));
    let mut many = one.clone();
    let comment = many.comments.first().unwrap().clone();
    many.comments = vec![comment.clone(); MAX_COMMENTS.saturating_add(1)];
    assert_eq!(many.check(), Err(ContextProblem::Comments(MAX_COMMENTS.saturating_add(1))));
    let mut long = one.clone();
    let line = comment.lines.first().unwrap().clone();
    long.comments.first_mut().unwrap().lines = vec![line; MAX_LINES.saturating_add(1)];
    assert_eq!(long.check(), Err(ContextProblem::Lines(MAX_LINES.saturating_add(1))));
    let mut large = one;
    let mut wordy = comment;
    wordy.body = "w".repeat(BODY_BYTES).parse().unwrap();
    large.comments = vec![wordy; 5];
    assert!(matches!(large.check(), Err(ContextProblem::TooLarge(_))));
}

#[test]
fn an_older_peer_reads_the_blocks_as_the_text_the_agent_read() {
    let words = ContentBlock::Text {
        text: "Fix these.".to_owned(),
    };
    let started = AgentEvent::TurnStarted {
        content: vec![words.clone(), context(), review()],
    };
    assert_eq!(started.required_protocol(), PROMPT_CONTEXT_PROTOCOL);
    assert_eq!(started.for_protocol(PROMPT_CONTEXT_PROTOCOL.unwrap()), None);
    let older = started.for_protocol(before()).unwrap();
    let AgentEvent::TurnStarted { content } = older else {
        panic!("a turn start");
    };
    assert_eq!(content.first(), Some(&words));
    assert_eq!(
        content.get(1),
        Some(&ContentBlock::Text {
            text: agent_text(&context()).unwrap()
        })
    );
    assert_eq!(
        content.get(2),
        Some(&ContentBlock::Text {
            text: agent_text(&review()).unwrap()
        })
    );
    // A steer stays a steer from 10, and a log line before it.
    let steered = AgentEvent::Steered {
        content: vec![context()],
    };
    assert!(matches!(
        steered.for_protocol(before()),
        Some(AgentEvent::Steered { .. })
    ));
    assert!(matches!(
        steered.for_protocol(ProtocolVersion::new(9)),
        Some(AgentEvent::HarnessLog { .. })
    ));
}

#[test]
fn a_prompt_holding_a_block_needs_its_protocol() {
    let call = |content: Vec<ContentBlock>| {
        Call::PromptSession(PromptSession {
            session: SessionId::random(),
            content,
            delivery: Delivery::Start,
            notify: false,
        })
    };
    let plain = call(vec![ContentBlock::Text {
        text: "hello".to_owned(),
    }]);
    let with = call(vec![context()]);
    assert!(plain.since() < with.since());
    assert_eq!(with.since(), PROMPT_CONTEXT_PROTOCOL);
}
