//! Per-source presence is applied before generic burst coalescing, including
//! candidates beyond the first bounded scan page.
use super::super::{PresenceId, Push};
use std::sync::Arc;
use vessel_proto::inbox::{InboxCursor, InboxRead, MarkInbox};
use vessel_proto::push::{PushActivity, PushFocus, ReportPushPresence};
use vessel_proto::{
    AgentEvent, EventRecord, EventSeq, HarnessKind, SessionEvent, SessionId, SessionInfo,
    SessionMode, SessionSpec, StopReason, TerminalSize,
};
use vessel_store::{DataDir, Store, StoreConfig};
pub(super) async fn session(store: &Store) -> SessionId {
    let info = SessionInfo::new(
        SessionId::random(),
        SessionSpec::new(
            HarnessKind::Codex,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        ),
        chrono::Utc::now(),
    );
    let id = info.id;
    store.create_session(info).await.unwrap();
    id
}
pub(super) async fn finished(store: &Store, id: SessionId, sequence: u64) {
    store
        .append(
            id,
            vec![Arc::new(EventRecord {
                seq: EventSeq::new(sequence),
                at: chrono::Utc::now(),
                event: SessionEvent::Agent {
                    event: AgentEvent::TurnEnded {
                        reason: StopReason::EndTurn,
                        usage: None,
                        point: None,
                    },
                },
            })],
        )
        .await
        .unwrap();
}
#[tokio::test]
async fn visible_first_or_last_does_not_drop_background_and_scan_continues_after_sixty_four() {
    for visible_first in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let dir = DataDir::new(temp.path());
        dir.prepare().unwrap();
        let store = Store::open(&dir, StoreConfig::default())
            .await
            .unwrap()
            .store;
        let visible = session(&store).await;
        let hidden = session(&store).await;
        if !visible_first {
            finished(&store, hidden, 0).await;
        }
        for sequence in 0..65 {
            finished(&store, visible, sequence).await;
        }
        if visible_first {
            finished(&store, hidden, 0).await;
        }
        let push = Push::new(store.clone(), dir);
        let presence = PresenceId::new();
        assert!(
            push.present(
                presence,
                ReportPushPresence {
                    focus: PushFocus::Session { session: visible },
                    activity: PushActivity::Active
                }
            )
            .unwrap()
        );
        let through = store.inbox_head().await.unwrap();
        // Visible A suppresses only itself. Since a recent client is present,
        // B routes locally under Paseo; no source is accidentally consumed by A.
        assert_eq!(push.pick(InboxCursor::new(0), through).await.unwrap(), None);
        // With nobody present the same durable sources become push eligible;
        // the prior local/suppressed pass did not remove their read state.
        push.forget(presence);
        let candidate = push
            .pick(InboxCursor::new(0), through)
            .await
            .unwrap()
            .unwrap();
        assert!(candidate.key.session == visible || candidate.key.session == hidden);
        store
            .mark_inbox(MarkInbox {
                key: candidate.key,
                read: InboxRead::Read,
            })
            .await
            .unwrap();
        assert_ne!(
            push.pick(InboxCursor::new(0), through)
                .await
                .unwrap()
                .unwrap()
                .key,
            candidate.key
        );
        store.close().await;
    }
}
