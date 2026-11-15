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

#[test]
fn retirement_reports_changed_only_for_the_original_admission() {
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let address = PanelAddress {
        workspace: LayoutKey::Session {
            machine: HostAlias::parse("fixture").unwrap(),
            session: SessionId::random(),
        },
        tab: serde_json::from_str("1").unwrap(),
    };
    let mut entry = super::Entry::new(super::PanelTarget::WorkingDiff, Some(context.clone()));
    let mut keys = RequestKeys::default();
    let mut reads = PanelReads::new(
        RepositoryKey {
            node: context.node(),
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        },
        Comparison::default(),
    );
    let _pending = reads.begin_status(keys.issue().unwrap());
    entry.reads = Some(reads);
    let _old = entry
        .path_places
        .insert("old.rs".parse().unwrap(), Default::default());
    let mut panels = super::Panels::default();
    let _old = panels.entries.insert(address.clone(), entry);
    let another = epochs
        .admit(context.machine().clone(), context.node())
        .unwrap();
    assert!(
        !panels.retire(&another.tag()),
        "same node with another epoch is not this admission"
    );
    assert!(
        panels.retire(&context.tag()),
        "the shell must not take its no-device-page early return"
    );
    let entry = panels.entries.get(&address).unwrap();
    assert_eq!(entry.context, None);
    assert_eq!(entry.problem, Some(ReadFailure::DestinationChanged));
    assert!(entry.path_places.is_empty());
    assert!(matches!(
        entry.reads.as_ref().unwrap().status(),
        ReadState::Failed(ReadFailure::DestinationChanged)
    ));
    assert!(
        !panels.retire(&context.tag()),
        "duplicate retirement needs no second redraw"
    );
}

#[test]
fn empty_mode_callback_refuses_another_root_epoch_comparison_or_commit_target() {
    use super::comparison::ComparisonChange;
    let mut epochs = ConnectionEpochs::default();
    let context = epochs
        .admit(MachineKey::from_book("fixture"), NodeId::random())
        .unwrap();
    let repository = RepositoryKey {
        node: context.node(),
        root: WorkspaceRoot::Project {
            project: ProjectId::random(),
        },
    };
    let comparison = Comparison::new(ComparisonMode::Uncommitted, None, Whitespace::Show);
    let captured = ComparisonChange {
        address: PanelAddress {
            workspace: LayoutKey::Session {
                machine: HostAlias::parse("fixture").unwrap(),
                session: SessionId::random(),
            },
            tab: serde_json::from_str("1").unwrap(),
        },
        context: context.clone(),
        repository,
        comparison: comparison.clone(),
    };
    let mut entry = super::Entry::new(super::PanelTarget::WorkingDiff, Some(context.clone()));
    entry.reads = Some(PanelReads::new(repository, comparison.clone()));
    assert!(captured.is_current(&entry, Some(&context)));
    assert!(!captured.is_current(&entry, None));
    let newer = epochs
        .admit(context.machine().clone(), context.node())
        .unwrap();
    assert!(!captured.is_current(&entry, Some(&newer)));
    entry.context = Some(newer);
    assert!(!captured.is_current(&entry, Some(&context)));
    entry.context = Some(context.clone());
    entry.reads = Some(PanelReads::new(
        RepositoryKey {
            root: WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
            ..repository
        },
        comparison.clone(),
    ));
    assert!(!captured.is_current(&entry, Some(&context)));
    entry.reads = Some(PanelReads::new(repository, Comparison::default()));
    assert!(!captured.is_current(&entry, Some(&context)));
    entry.reads = Some(PanelReads::new(repository, comparison));
    entry.target = super::PanelTarget::CommitDiff("a".repeat(40).parse().unwrap());
    assert!(!captured.is_current(&entry, Some(&context)));
}
