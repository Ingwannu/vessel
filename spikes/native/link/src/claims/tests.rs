//! Asking a machine about its notices, through the real framed actor:
//! one first inbox page, then a claim per notice found on it. While ADR
//! 0089's protocol is unassigned the client refuses the claim before the
//! wire, so a found notice comes back failed and shows nothing. A page that
//! fails sends every notice by ADR 0114's path.

use crate::claims::Ask;
use crate::testing::{actor::Actor, within};
use crate::{Command, Event};
use vessel_app_view::notices::claims::Asked;
use vessel_app_view::notices::{Announce, Notice, NoticeKind};
use vessel_proto::inbox::{
    InboxCount, InboxCounts, InboxCursor, InboxEntry, InboxItem, InboxKey, InboxPage, InboxRead,
};
use vessel_proto::notifications::{ATTENTION_NOTIFICATIONS_PROTOCOL, NotificationClaim};
use vessel_proto::{
    Call, ErrorCode, EventSeq, Outcome as WireOutcome, Reply, SessionId, WireError,
};

fn told(session: SessionId) -> Announce {
    Announce {
        notice: Notice {
            kind: NoticeKind::Finished,
            machine: "fixture".to_owned(),
            session,
            title: String::new(),
        },
        system: true,
        sound: true,
    }
}

fn page(session: SessionId) -> InboxPage {
    InboxPage {
        counts: InboxCounts {
            total: InboxCount::new(1),
            waiting: InboxCount::new(0),
            unread: InboxCount::new(1),
        },
        entries: vec![InboxEntry {
            cursor: InboxCursor::new(7),
            key: InboxKey {
                session,
                seq: EventSeq::new(20),
            },
            title: None,
            project: None,
            project_name: None,
            at: chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
            read: InboxRead::Unread,
            item: InboxItem::Finished,
        }],
        next: None,
    }
}

#[tokio::test]
async fn one_first_page_then_a_claim_per_notice_found_on_it() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let found = SessionId::random();
        let missing = SessionId::random();
        actor
            .dispatch(Command::Claims(Ask {
                machine: "fixture".to_owned(),
                told: vec![told(found), told(missing)],
            }))
            .await;
        let request = actor.request().await;
        let Call::ListInbox(list) = request.call else {
            panic!("expected the first inbox page, got {:?}", request.call);
        };
        assert_eq!(list.before, None, "the first page");
        request
            .answer
            .send(WireOutcome::Ok(Reply::ListInbox(page(found))))
            .unwrap();
        // Once the protocol is assigned, the found notice's claim reaches
        // the peer and is admitted; while it is not, the client refuses it
        // before the wire.
        let expected = if ATTENTION_NOTIFICATIONS_PROTOCOL.is_some() {
            let claim = actor.request().await;
            assert!(matches!(claim.call, Call::ClaimNotification(_)));
            claim
                .answer
                .send(WireOutcome::Ok(Reply::ClaimNotification(
                    NotificationClaim::Admitted,
                )))
                .unwrap();
            Asked::Answered(NotificationClaim::Admitted)
        } else {
            Asked::Failed
        };
        let Event::Claims(answer) = actor.next_event().await else {
            panic!("expected the claims' answer");
        };
        assert_eq!(answer.machine, "fixture");
        let outcomes: Vec<(SessionId, Asked)> = answer
            .asked
            .iter()
            .map(|(one, asked)| (one.notice.session, *asked))
            .collect();
        assert_eq!(
            outcomes,
            vec![(found, expected), (missing, Asked::NotOnFirstPage)]
        );
        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn a_machine_not_connected_answers_every_notice_failed() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        actor
            .dispatch(Command::Claims(Ask {
                machine: "elsewhere".to_owned(),
                told: vec![told(SessionId::random())],
            }))
            .await;
        let Event::Claims(answer) = actor.next_event().await else {
            panic!("expected the claims' answer");
        };
        assert!(
            answer
                .asked
                .iter()
                .all(|(_one, asked)| *asked == Asked::Failed)
        );
        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}

#[tokio::test]
async fn a_failed_first_page_tells_every_notice_by_the_old_path_and_claims_none() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let (one, two) = (SessionId::random(), SessionId::random());
        actor
            .dispatch(Command::Claims(Ask {
                machine: "fixture".to_owned(),
                told: vec![told(one), told(two)],
            }))
            .await;
        let request = actor.request().await;
        assert!(matches!(request.call, Call::ListInbox(_)));
        request
            .answer
            .send(WireOutcome::Err(WireError::new(
                ErrorCode::Unavailable,
                "busy",
            )))
            .unwrap();
        let Event::Claims(answer) = actor.next_event().await else {
            panic!("expected the claims' answer");
        };
        let outcomes: Vec<(SessionId, Asked)> = answer
            .asked
            .iter()
            .map(|(notice, asked)| (notice.notice.session, *asked))
            .collect();
        assert_eq!(
            outcomes,
            vec![(one, Asked::NotOnFirstPage), (two, Asked::NotOnFirstPage)],
            "no claim goes, and nothing is silenced"
        );
        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}
