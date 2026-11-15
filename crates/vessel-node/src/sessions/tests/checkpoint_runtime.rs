//! Real node bridge + fake ACP + managed worktree before wire-number assignment.
use super::*;
use crate::workspace::{Opening, Workspaces};
use std::process::Command;
use vessel_proto::{
    AgentEvent, ContentBlock, Creator, Delivery, EventSeq, SessionEvent,
    checkpoint::{
        Boundary, CheckpointOwner, CheckpointState, PreviewCheckpoint, ReadCheckpointChanges,
    },
    workspace::{CreateWorktree, CreateWorktreeOutcome, SessionPlace, SetupChoice, WorktreeBranch},
};
use vessel_session::{CheckpointHooks, SessionHandle};

fn repository(root: &std::path::Path, config: &std::path::Path) {
    for args in [
        &["init", "-q"][..],
        &["add", "README.md"][..],
        &["commit", "-q", "-m", "first"][..],
    ] {
        if args.first() == Some(&"add") {
            std::fs::write(root.join("README.md"), b"checkpoint bytes\n").unwrap();
        }
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .env("GIT_CONFIG_GLOBAL", config)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .status()
                .unwrap()
                .success()
        );
    }
}
async fn turn(handle: &SessionHandle, text: &str) -> EventSeq {
    let mut follow = handle.follow(handle.info().next_seq).unwrap();
    handle
        .prompt(
            vec![ContentBlock::Text {
                text: text.to_owned(),
            }],
            Delivery::Queue,
            Creator::Owner,
        )
        .await
        .unwrap();
    loop {
        let record = follow.live.recv().await.unwrap();
        if matches!(
            record.event,
            SessionEvent::Agent {
                event: AgentEvent::TurnEnded { .. }
            }
        ) {
            return record.seq;
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_node_runtime_bridge_persists_before_and_after_real_fake_provider_turns() {
    vessel_testkit::within(std::time::Duration::from_secs(120),async {
        let (hooks,requests)=CheckpointHooks::channel();
        let fixture=Fixture::with_checkpoints(Some(hooks)).await;
        let config=fixture.data.path().join("gitconfig");
        let excludes=fixture.data.path().join("ignore");
        std::fs::write(&excludes,b"").unwrap();
        std::fs::write(&config,format!("[user]\nname = Test\nemail = test@example.invalid\n[init]\ndefaultBranch = main\n[commit]\ngpgsign = false\n[core]\nexcludesFile = {}\n",excludes.display())).unwrap();
        let repo=tempfile::tempdir().unwrap();
        repository(repo.path(),&config);
        let data=DataDir::new(fixture.data.path());
        let stop=CancellationToken::new();
        let workspaces=Workspaces::start(Opening {data:data.root().to_path_buf(),workspace:data.prepare_workspace_dir().unwrap(),home:fixture.data.path().to_path_buf(),git:Some(which::which("git").unwrap()),
            git_env:vec![("GIT_CONFIG_GLOBAL".to_owned(),config.into_os_string()),("GIT_CONFIG_NOSYSTEM".to_owned(),"1".into())],removed:crate::TerminalsConfig::default().removed.into()},
            fixture.store.clone(),fixture.tasks.clone(),stop.clone()).await.unwrap();
        let project=workspaces.add_project(repo.path().to_path_buf(),None).await.unwrap();
        let CreateWorktreeOutcome::Created {worktree}=workspaces.create_worktree(Creator::Owner,CreateWorktree {project:project.id,branch:WorktreeBranch::New {name:None,base:None},setup:SetupChoice::Run},None).await.unwrap()
        else {panic!("no managed worktree");};
        let closed = crate::checkpoint::start(Arc::clone(&fixture.sessions), Arc::clone(&workspaces), fixture.store.clone(), fixture.tasks.clone(), stop.clone(), requests);
        let controller = closed.clone().no_terminal_fixture();
        let mut spec=fixture.structured().with_cwd(worktree.path.clone());
        spec.place=Some(Box::new(SessionPlace::Worktree {worktree:worktree.id}));
        let root=workspaces.admit_start(&worktree.path).await.unwrap();
        let session=fixture.sessions.create(spec,LaunchOptions::default(),None,super::super::Admission::new(root,None)).await.unwrap();
        let handle=fixture.runtime.get(session.id).unwrap();
        let _ready=ready(&handle).await.unwrap();
        let ended=turn(&handle,"first checkpoint turn").await;
        let _second=turn(&handle,"second checkpoint turn").await;
        fixture.runtime.shutdown().await;
        let checkpoint = loop {
            let page = fixture.store.checkpoint_page(session.id, None).await.unwrap();
            if let Some(checkpoint) = page.marks.iter().find_map(|mark| match mark.state {
                CheckpointState::Ready { checkpoint } if mark.boundary == Boundary::AfterTurn { ended } => Some(checkpoint),
                _ => None,
            }) { break checkpoint; }
            tokio::task::yield_now().await;
        };
        std::fs::write(worktree.path.join("README.md"), b"preview must not overwrite this edit\n").unwrap();
        let preview = controller.preview(CheckpointOwner::LocalOwner, PreviewCheckpoint { session: session.id, checkpoint }).await.unwrap();
        assert_eq!(preview.changed.get(), 1);
        assert_eq!(preview.page.changes.len(), 1);
        assert!(preview.page.next.is_none());
        assert_eq!(std::fs::read(worktree.path.join("README.md")).unwrap(), b"preview must not overwrite this edit\n");
        assert!(matches!(controller.changes(CheckpointOwner::Device { device: vessel_proto::DeviceId::random() }, ReadCheckpointChanges { plan: preview.page.plan, after: None }).await,
            Err(vessel_proto::checkpoint::CheckpointError::Forbidden)));
        let (frozen, consumed) = fixture.store.checkpoint_plan(preview.page.plan, CheckpointOwner::LocalOwner).await.unwrap();
        assert!(consumed.is_none());
        assert_ne!(frozen.current, *frozen.desired.image());
        let physical = vessel_workspace::physical::Directory::open(&worktree.path).unwrap().identity();
        fixture.sessions.checkpoint_ended_target(session.id, physical).await.unwrap();
        let rejected = vessel_proto::checkpoint::RestoreCheckpoint { session: session.id, plan: preview.page.plan, restore: vessel_proto::checkpoint::RestoreId::random() };
        assert!(matches!(closed.restore(CheckpointOwner::LocalOwner, rejected).await, Err(vessel_proto::checkpoint::CheckpointError::Busy)), "runtime terminal admission must stay closed");
        std::fs::write(worktree.path.join("README.md"), b"edit after confirmation was shown\n").unwrap();
        assert!(matches!(controller.restore(CheckpointOwner::LocalOwner, rejected).await, Err(vessel_proto::checkpoint::CheckpointError::StalePreview)));
        assert!(fixture.store.checkpoint_operation(session.id, rejected.restore, CheckpointOwner::LocalOwner).await.is_err(), "stale confirmation is refused before restore admission");
        assert_eq!(std::fs::read(worktree.path.join("README.md")).unwrap(), b"edit after confirmation was shown\n");
        let preview = controller.preview(CheckpointOwner::LocalOwner, PreviewCheckpoint { session: session.id, checkpoint }).await.unwrap();
        let before = fixture.store.session(session.id).await.unwrap().unwrap();
        let restore = vessel_proto::checkpoint::RestoreCheckpoint { session: session.id, plan: preview.page.plan, restore: vessel_proto::checkpoint::RestoreId::random() };
        let result = controller.restore(CheckpointOwner::LocalOwner, restore).await.unwrap();
        assert!(matches!(result, vessel_proto::checkpoint::RestoreOutcome::Restored { changed } if changed.get() == 1));
        assert_eq!(std::fs::read(worktree.path.join("README.md")).unwrap(), b"checkpoint bytes\n");
        let after = fixture.store.session(session.id).await.unwrap().unwrap();
        assert_eq!(before.status, after.status);
        assert_eq!(before.ended_at, after.ended_at);
        let history = fixture.sessions.read(session.id, before.next_seq, 10).await.unwrap();
        assert_eq!(history.records.len(), 1);
        assert!(matches!(history.records[0].event, SessionEvent::Checkpoint { event: vessel_proto::checkpoint::CheckpointEvent::FilesRestored { restore: id, .. } } if id == restore.restore));
        std::fs::write(worktree.path.join("README.md"), b"external edit after completed restore\n").unwrap();
        assert_eq!(controller.restore(CheckpointOwner::LocalOwner, restore).await.unwrap(), result);
        assert_eq!(std::fs::read(worktree.path.join("README.md")).unwrap(), b"external edit after completed restore\n");
        assert_eq!(fixture.store.session(session.id).await.unwrap().unwrap().next_seq, after.next_seq);

        stop.cancel();
        fixture.settle().await;
        let page=fixture.store.checkpoint_page(session.id,None).await.unwrap();
        assert!(page.marks.iter().any(|mark| matches!(mark.boundary,Boundary::BeforeTurn {..}) && matches!(mark.state,CheckpointState::Ready {..})),"actual before image: {page:?}");
        assert!(page.marks.iter().any(|mark| mark.boundary==Boundary::AfterTurn {ended} && matches!(mark.state,CheckpointState::Ready {..})),"actual first after image: {page:?}");
        fixture.stop().await;
    }).await.unwrap();
}
