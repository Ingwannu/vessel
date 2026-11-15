//! A real bounded Link rejection settles the same pane's pending status.
use vessel_app_link::{Command, Event, Link};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::layout::{
    AgentRole, HostAlias, Layout, LayoutKey, LayoutMode, OpenIntent, Placement, TabId, Target,
};
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, PanelAddress, PanelReads, ReadFailure, RepositoryKey, Whitespace,
};
use vessel_app_view::workspace::git::{ReadState, RequestKeys};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_proto::{NodeId, ProjectId, SessionId};

// Test identities come from the sole layout allocator, never a raw JSON counter
// or a new serialization dependency just for fixtures.
pub(super) fn tab() -> TabId {
    Layout::default()
        .open(
            Target::WorkingDiff,
            OpenIntent::Reveal,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap()
}

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
        tab: tab(),
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
        tab: tab(),
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
        .insert("old.rs".parse().unwrap(), std::rc::Rc::default());
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
    let entry = &panels.entries[&address];
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
    let mut keys = RequestKeys::default();
    let instance = keys.issue().unwrap();
    let (reads, summary, _) =
        super::target_tests::accepted_reads(repository, comparison.clone(), &mut keys);
    let captured = ComparisonChange {
        target: super::PanelTarget::WorkingDiff,
        instance,
        address: PanelAddress {
            workspace: LayoutKey::Session {
                machine: HostAlias::parse("fixture").unwrap(),
                session: SessionId::random(),
            },
            tab: tab(),
        },
        context: context.clone(),
        repository,
        comparison: comparison.clone(),
        summary,
    };
    let mut entry = super::Entry::new(super::PanelTarget::WorkingDiff, Some(context.clone()));
    entry.instance = Some(instance);
    entry.reads = Some(reads);
    assert!(captured.is_current(&entry, Some(&context)));
    entry.instance = Some(keys.issue().unwrap());
    assert!(
        !captured.is_current(&entry, Some(&context)),
        "Refresh replaced the owner"
    );
    entry.instance = Some(instance);
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

#[test]
fn refresh_preserves_only_same_target_comparison_and_rediscovers_authority() {
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
    let chosen = Comparison::new(ComparisonMode::Uncommitted, None, Whitespace::Ignore);
    let mut previous = super::Entry::new(super::PanelTarget::WorkingDiff, Some(context.clone()));
    previous.reads = Some(PanelReads::new(repository, chosen.clone()));
    let mut refreshed = super::Entry::new(super::PanelTarget::WorkingDiff, Some(context.clone()));
    refreshed.retain_comparison(&previous);
    assert_eq!(refreshed.comparison, chosen);
    assert!(
        refreshed.reads.is_none(),
        "root and tickets must be located again"
    );
    assert!(matches!(refreshed.binding, super::PanelBinding::Idle));
    let next = epochs
        .admit(context.machine().clone(), context.node())
        .unwrap();
    let mut replacement = super::Entry::new(super::PanelTarget::WorkingDiff, Some(next));
    replacement.retain_comparison(&previous);
    assert_eq!(replacement.comparison, Comparison::default());
    let commit: vessel_proto::workspace::CommitId = "a".repeat(40).parse().unwrap();
    let mut immutable = super::Entry::new(
        super::PanelTarget::CommitDiff(commit.clone()),
        Some(context),
    );
    immutable.retain_comparison(&previous);
    assert_eq!(
        immutable.comparison,
        Comparison::at_commit(commit, Whitespace::Show)
    );
}
