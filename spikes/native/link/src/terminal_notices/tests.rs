//! Following a machine's terminals through the real framed actor: the whole
//! list first, then from the revision read, and only a new attention
//! revision is told. A failed call rests, then asks for the whole list
//! again; a closed connection ends the follower. While ADR 0089's protocol
//! is unassigned, a fresh connection starts no follower.

use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use vessel_app_view::notices::NoticeKind;
use vessel_proto::notifications::{
    ATTENTION_NOTIFICATIONS_PROTOCOL, TerminalActivity, TerminalAttention, TerminalAttentionReason,
    TerminalAttentionRevision,
};
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalList, TerminalPlace};
use vessel_proto::{
    Call, ErrorCode, Outcome as WireOutcome, Reply, SessionId, TerminalId, WireError,
};

use crate::Event;
use crate::testing::{actor::Actor, peer, within};

fn list(
    id: TerminalId,
    attention: u64,
    reason: TerminalAttentionReason,
    revision: u64,
) -> TerminalList {
    let mut info = TerminalInfo::new(
        id,
        TerminalPlace::Session {
            session: SessionId::random(),
        },
        PathBuf::from("/work"),
        TerminalKind::Shell {
            program: PathBuf::from("/bin/sh"),
        },
        Utc::now(),
    );
    info.attention = Some(TerminalAttention {
        terminal: id,
        revision: TerminalAttentionRevision::new(attention),
        activity: TerminalActivity::Attention { reason },
        changed_at: Utc::now(),
    });
    TerminalList {
        terminals: vec![info],
        revision,
    }
}

/// The next call, which must be the whole list from `changed_from`.
async fn listed(actor: &mut Actor, changed_from: Option<u64>) -> peer::Request {
    let request = actor.request().await;
    let Call::ListTerminals(asked) = &request.call else {
        panic!("expected the terminal list, got {:?}", request.call);
    };
    assert_eq!(asked.place, None, "every terminal of the machine");
    assert_eq!(asked.changed_from, changed_from);
    request
}

#[tokio::test]
async fn the_list_is_followed_from_its_revision_and_a_new_attention_is_told() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let id = TerminalId::random();
        let follower = actor.follow_terminals("fixture", crate::terminal_notices::RETRY);
        listed(&mut actor, None)
            .await
            .answer
            .send(WireOutcome::Ok(Reply::ListTerminals(list(
                id,
                3,
                TerminalAttentionReason::Finished,
                10,
            ))))
            .unwrap();
        listed(&mut actor, Some(10))
            .await
            .answer
            .send(WireOutcome::Ok(Reply::ListTerminals(list(
                id,
                4,
                TerminalAttentionReason::NeedsInput,
                11,
            ))))
            .unwrap();
        let Event::TerminalNotices { machine, notices } = actor.next_event().await else {
            panic!("expected the terminal notices");
        };
        assert_eq!(machine, "fixture");
        let told: Vec<(TerminalId, TerminalAttentionRevision, NoticeKind)> = notices
            .iter()
            .map(|notice| (notice.terminal, notice.revision, notice.kind))
            .collect();
        assert_eq!(
            told,
            vec![(id, TerminalAttentionRevision::new(4), NoticeKind::NeedsYou)],
            "the first list's attention is not told"
        );
        let _waiting = listed(&mut actor, Some(11)).await;
        actor.retire().await;
        actor.close_peer().await;
        follower.await.unwrap();
    })
    .await;
}

/// The notices of the next event, as (terminal, revision, kind).
async fn told(actor: &mut Actor) -> Vec<(TerminalId, TerminalAttentionRevision, NoticeKind)> {
    let Event::TerminalNotices { notices, .. } = actor.next_event().await else {
        panic!("expected the terminal notices");
    };
    notices
        .iter()
        .map(|notice| (notice.terminal, notice.revision, notice.kind))
        .collect()
}

#[tokio::test]
async fn a_failed_list_is_asked_again_whole_after_a_rest_and_a_closed_connection_ends_it() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let id = TerminalId::random();
        let follower = actor.follow_terminals("fixture", Duration::from_millis(20));
        let answer = |request: peer::Request, list: TerminalList| {
            request
                .answer
                .send(WireOutcome::Ok(Reply::ListTerminals(list)))
                .unwrap();
        };
        let first = listed(&mut actor, None).await;
        answer(first, list(id, 3, TerminalAttentionReason::Finished, 10));
        listed(&mut actor, Some(10))
            .await
            .answer
            .send(WireOutcome::Err(WireError::new(
                ErrorCode::Unavailable,
                "busy",
            )))
            .unwrap();
        // After the rest the whole list is asked again, and what was told
        // before the failure is kept: only the newer attention is told.
        let again = listed(&mut actor, None).await;
        answer(again, list(id, 5, TerminalAttentionReason::NeedsInput, 12));
        assert_eq!(
            told(&mut actor).await,
            vec![(id, TerminalAttentionRevision::new(5), NoticeKind::NeedsYou)]
        );
        // A call that fails because the connection closed ends the follower
        // rather than resting.
        let _waiting = listed(&mut actor, Some(12)).await;
        actor.retire().await;
        actor.close_peer().await;
        follower.await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn a_fresh_connection_follows_only_a_machine_that_admits_claims() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        assert_eq!(
            actor.terminals_followed_on_connect("fixture"),
            ATTENTION_NOTIFICATIONS_PROTOCOL.is_some()
        );
        assert!(!actor.terminals_followed_on_connect("elsewhere"));
        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}
