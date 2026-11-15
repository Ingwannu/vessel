//! Selected-client admission and bounded local rejection use the real actor.
use super::*;
use crate::connections::Connected;
use crate::testing::{Home, peer, phase, state, within};
use vessel_app_view::connection::{ConnectionContext, MachineKey};
use vessel_app_view::layout::{HostAlias, LayoutKey};
use vessel_app_view::workspace::git::RequestKeys;
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, PanelReads, RepositoryKey, Whitespace,
};
use vessel_client::book::BookEntry;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{Call, NodeId, ProjectId, SessionId};

pub(super) fn install(
    state: &mut crate::State,
    client: vessel_client::NodeClient,
) -> ConnectionContext {
    let context = state
        .admissions
        .admit(MachineKey::from_book("fixture"), client.node().id)
        .unwrap();
    let _old = state.clients.insert(
        "fixture".into(),
        Connected::new(
            client,
            Vec::new(),
            BookEntry {
                address: "ws://127.0.0.1:0/rpc".parse().unwrap(),
                token: None,
                credential: None,
                id: None,
            },
            context.clone(),
        ),
    );
    context
}
pub(super) fn status(context: ConnectionContext, node: NodeId, machine: &str) -> Ask {
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(
        RepositoryKey {
            node,
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        },
        Comparison::new(ComparisonMode::All, None, Whitespace::Show),
    );
    Ask::Status {
        address: PanelAddress {
            workspace: LayoutKey::Session {
                machine: HostAlias::parse(machine).unwrap(),
                session: SessionId::random(),
            },
            tab: serde_json::from_str("1").unwrap(),
        },
        context,
        ticket: reads.begin_status(keys.issue().unwrap()),
    }
}
pub(super) async fn answer(events: &mut tokio::sync::mpsc::Receiver<crate::Event>) -> Answer {
    let crate::Event::GitPanel(answer) =
        phase("git panel settlement", events.recv()).await.unwrap()
    else {
        panic!("expected the correlated git answer");
    };
    answer
}
#[tokio::test]
async fn another_alias_node_or_retired_generation_never_polls_a_request() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::connect("fixture").await;
        let context = install(&mut state, peer.client.take().unwrap());
        for ask in [
            status(context.clone(), context.node(), "other"),
            status(context.clone(), NodeId::random(), "fixture"),
        ] {
            let expected = ask.address().clone();
            state.git_panel(ask).await;
            let result = answer(&mut events).await;
            assert_eq!(result.address, expected);
            assert_eq!(result.context, context);
            assert!(matches!(
                result.outcome,
                Outcome::Status(
                    _,
                    Err(ReadFailure::DestinationChanged | ReadFailure::UnexpectedNode { .. })
                )
            ));
        }
        state.clients["fixture"].retire();
        state
            .git_panel(status(context.clone(), context.node(), "fixture"))
            .await;
        assert!(matches!(
            answer(&mut events).await.outcome,
            Outcome::Status(_, Err(ReadFailure::DestinationChanged))
        ));
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}
#[tokio::test]
async fn retirement_after_the_real_request_frame_settles_the_original_ticket() {
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::controlled("fixture").await;
        let context = install(&mut state, peer.client.take().unwrap());
        let ask = status(context.clone(), context.node(), "fixture");
        let expected = ask.address().clone();
        state.git_panel(ask).await;
        let request = phase(
            "actual original-client status frame",
            peer.requests.as_mut().unwrap().recv(),
        )
        .await
        .unwrap();
        assert!(matches!(request.call, Call::GitStatus(_)));
        state.clients["fixture"].retire();
        let result = answer(&mut events).await;
        assert_eq!(result.address, expected);
        assert_eq!(result.context, context);
        assert!(matches!(
            result.outcome,
            Outcome::Status(_, Err(ReadFailure::DestinationChanged))
        ));
        assert_eq!(peer.calls.lock().unwrap().len(), 1);
    })
    .await;
}

#[tokio::test]
async fn staged_comparison_summary_and_file_settle_their_tickets_without_a_peer_frame() {
    use vessel_app_view::workspace::problem::Problem;
    within(async {
        let home = Home::new();
        let (mut state, mut events) = state(&home);
        let mut peer = peer::connect("fixture").await;
        let context = install(&mut state, peer.client.take().unwrap());
        let Ask::Status {
            address, ticket, ..
        } = status(context.clone(), context.node(), "fixture")
        else {
            panic!("fixture produces status identity");
        };
        let mut keys = RequestKeys::default();
        for mode in [ComparisonMode::All, ComparisonMode::Committed] {
            let mut reads = PanelReads::new(
                ticket.repository(),
                Comparison::new(mode, None, Whitespace::Show),
            );
            let ticket = reads.begin_summary(keys.issue().unwrap());
            state
                .git_panel(Ask::Summary {
                    address: address.clone(),
                    context: context.clone(),
                    ticket: ticket.clone(),
                })
                .await;
            let returned = answer(&mut events).await;
            assert_eq!(returned.address, address);
            assert_eq!(returned.context, context);
            let Outcome::Summary(got, result) = returned.outcome else {
                panic!("summary outcome");
            };
            assert_eq!(got, ticket);
            assert_eq!(result, Err(ReadFailure::Call(Problem::Older)));
            let ticket = reads.begin_file("source.rs".parse().unwrap(), keys.issue().unwrap());
            state
                .git_panel(Ask::File {
                    address: address.clone(),
                    context: context.clone(),
                    ticket: ticket.clone(),
                })
                .await;
            let returned = answer(&mut events).await;
            assert_eq!(returned.address, address);
            assert_eq!(returned.context, context);
            let Outcome::File(got, result) = returned.outcome else {
                panic!("file outcome");
            };
            assert_eq!(got, ticket);
            assert_eq!(result, Err(ReadFailure::Call(Problem::Older)));
        }
        assert!(peer.calls.lock().unwrap().is_empty());
    })
    .await;
}
