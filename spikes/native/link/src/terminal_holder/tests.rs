//! Locating a notice's terminal through the real framed actor: each
//! candidate folder is listed as a folder place, a failed or empty one is
//! passed over, and the first that lists the terminal names its session.

use std::path::PathBuf;

use chrono::Utc;
use vessel_proto::terminals::{TerminalInfo, TerminalKind, TerminalList, TerminalPlace};
use vessel_proto::{
    Call, ErrorCode, Outcome as WireOutcome, Reply, SessionId, TerminalId, WireError,
};

use super::{Locate, Located};
use crate::testing::{actor::Actor, peer, within};
use crate::{Command, Event};

/// The next call, which must list `path` as a folder place, whole.
async fn listed(actor: &mut Actor, path: &str) -> peer::Request {
    let request = actor.request().await;
    let Call::ListTerminals(asked) = &request.call else {
        panic!("expected a terminal list, got {:?}", request.call);
    };
    assert_eq!(
        asked.place,
        Some(TerminalPlace::Folder {
            path: PathBuf::from(path),
        })
    );
    assert_eq!(asked.changed_from, None);
    request
}

/// A list holding `terminals`, kept in the canonical `/real`.
fn holding(terminals: &[TerminalId]) -> WireOutcome {
    let terminals = terminals
        .iter()
        .map(|id| {
            TerminalInfo::new(
                *id,
                TerminalPlace::Folder {
                    path: PathBuf::from("/real"),
                },
                PathBuf::from("/real"),
                TerminalKind::Shell {
                    program: PathBuf::from("/bin/sh"),
                },
                Utc::now(),
            )
        })
        .collect();
    WireOutcome::Ok(Reply::ListTerminals(TerminalList {
        terminals,
        revision: 1,
    }))
}

#[tokio::test]
async fn the_first_folder_the_node_matches_names_the_holder() {
    within(async {
        let mut actor = Actor::new("fixture").await;
        let terminal = TerminalId::random();
        let (failing, other, holder) = (
            SessionId::random(),
            SessionId::random(),
            SessionId::random(),
        );
        actor
            .dispatch(Command::LocateTerminal(Locate {
                machine: "fixture".to_owned(),
                terminal,
                candidates: vec![
                    (failing, PathBuf::from("/gone")),
                    (other, PathBuf::from("/other")),
                    (holder, PathBuf::from("/link")),
                ],
            }))
            .await;
        listed(&mut actor, "/gone")
            .await
            .answer
            .send(WireOutcome::Err(WireError::new(
                ErrorCode::InvalidRequest,
                "not a folder",
            )))
            .unwrap();
        listed(&mut actor, "/other")
            .await
            .answer
            .send(holding(&[TerminalId::random()]))
            .unwrap();
        listed(&mut actor, "/link")
            .await
            .answer
            .send(holding(&[terminal]))
            .unwrap();
        let Event::TerminalLocated(located) = actor.next_event().await else {
            panic!("expected the terminal's holder");
        };
        assert_eq!(
            located,
            Located {
                machine: "fixture".to_owned(),
                terminal,
                session: holder,
            }
        );
        actor.retire().await;
        actor.close_peer().await;
    })
    .await;
}
