//! A real bounded Link rejection settles the same pane's pending status.
use vessel_app_link::{Command, Event, Link};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{HostAlias, LayoutKey};
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, PanelAddress, PanelReads, ReadFailure, RepositoryKey, Whitespace,
};
use vessel_app_view::workspace::git::{ReadState, RequestKeys};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{NodeId, ProjectId, SessionId};

#[test]
fn a_full_link_settles_the_exact_status_without_an_endless_spinner() {
    let (link, _inbox) = Link::detached(1).unwrap();
    link.submit(Command::Reload).unwrap();
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(
        RepositoryKey {
            node: context.node(),
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        },
        Comparison::new(ComparisonMode::All, None, Whitespace::Show),
    );
    let ticket = reads.begin_status(keys.issue().unwrap());
    let address = PanelAddress {
        workspace: LayoutKey::Session {
            machine: HostAlias::parse("fixture").unwrap(),
            session: SessionId::random(),
        },
        tab: serde_json::from_str("1").unwrap(),
    };
    let rejected = link
        .submit(Command::GitPanel(vessel_app_link::git_panel::Ask::Status {
            address: address.clone(),
            context: context.clone(),
            ticket,
        }))
        .unwrap_err();
    let crate::unsent::Settle::Answer(answer) = crate::unsent::settle(rejected, "not used") else {
        panic!("a pending read needs its correlated answer");
    };
    let Event::GitPanel(answer) = *answer else {
        panic!("wrong feature answer")
    };
    assert_eq!(answer.address, address);
    assert_eq!(answer.context, context);
    let vessel_app_link::git_panel::Outcome::Status(returned, result) = answer.outcome else {
        panic!("wrong read kind");
    };
    assert_eq!(returned, ticket);
    assert!(reads.finish_status(returned, result));
    assert!(matches!(
        reads.status(),
        ReadState::Failed(ReadFailure::Call(Problem::Unsent))
    ));
    assert!(reads.status().outstanding().is_none());
    assert!(!reads.finish_status(ticket, Err(ReadFailure::DestinationChanged)));
}
