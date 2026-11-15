//! Builders for the search index's tests (`search.rs`, `search_measure.rs`,
//! by `#[path]`): records that say things, a started session, a request,
//! and a page as text. Included as a `cfg(test)` module so the lints' test
//! allowances apply to it.

use std::sync::Arc;

use vessel_proto::search::{SearchMessages, SearchPage};
use vessel_proto::{
    AgentEvent, ContentBlock, EventRecord, SessionEvent, SessionId, SessionStatus, StopReason,
};
use vessel_store::Store;

use super::fixtures::{new_session, record, status};
pub(crate) fn text(text: &str) -> ContentBlock {
    ContentBlock::Text {
        text: text.to_owned(),
    }
}

pub(crate) fn agent(seq: u64, event: AgentEvent) -> Arc<EventRecord> {
    record(seq, SessionEvent::Agent { event })
}

pub(crate) fn prompt(seq: u64, words: &str) -> Arc<EventRecord> {
    agent(
        seq,
        AgentEvent::TurnStarted {
            provenance: None,
            content: vec![text(words)],

            client_message_id: None,
        },
    )
}

pub(crate) fn chunk(seq: u64, words: &str) -> Arc<EventRecord> {
    agent(
        seq,
        AgentEvent::MessageChunk {
            content: text(words),
        },
    )
}

pub(crate) fn turn_ended(seq: u64) -> Arc<EventRecord> {
    agent(
        seq,
        AgentEvent::TurnEnded {
            reason: StopReason::EndTurn,
            usage: None,
            point: None,
        },
    )
}

pub(crate) async fn started(store: &Store, created: i64) -> SessionId {
    let session = new_session(created);
    store.create_session(session.clone()).await.unwrap();
    store
        .append(session.id, vec![status(0, SessionStatus::Running)])
        .await
        .unwrap();
    session.id
}

pub(crate) fn request(query: &str) -> SearchMessages {
    SearchMessages {
        query: query.parse().unwrap(),
        session: None,
        before: None,
        limit: None,
    }
}

/// Search once the writer's idle steps have taken everything in.
pub(crate) async fn search(store: &Store, query: &str) -> SearchPage {
    store.indexed().await;
    store.search(request(query)).await.unwrap()
}

/// Each hit's text, its matches in brackets.
pub(crate) fn shown(page: &SearchPage) -> Vec<String> {
    page.hits
        .iter()
        .map(|hit| {
            hit.snippet
                .parts
                .iter()
                .map(|part| {
                    if part.matched {
                        format!("[{}]", part.text)
                    } else {
                        part.text.clone()
                    }
                })
                .collect()
        })
        .collect()
}
