//! Cancellation cannot release admission or the serialized writer prematurely.
mod diagnostic;

use super::*;
use vessel_harness::HarnessEnv;
use vessel_proto::skills::SkillNames;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_request_keeps_its_writer_and_shutdown_tracking() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let config = Config::new(data.clone(), HarnessEnv::new(root.path().join("home")));
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let skills = Skills::start(&config, tasks.clone(), stop.clone());
    let _fresh = skills.status().await.unwrap();
    let (begun, started) = tokio::sync::oneshot::channel();
    let (release, waiting) = std::sync::mpsc::channel();
    let writer = Arc::clone(&skills);
    let next = SkillSelection::Custom {
        skills: SkillNames::default(),
    };
    let persisted = next.clone();
    let request = tokio::spawn(async move {
        writer
            .run(move |manager, data, durable| {
                begun.send(()).unwrap();
                waiting.recv().unwrap();
                Ok(
                    manager.save(durable, &next, &SkillNames::default(), |selection| {
                        data.write_skills_selection(selection, chrono::Utc::now())
                    })?,
                )
            })
            .await
    });
    started.await.unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert_eq!(skills.permits.available_permits(), MAX_COMMANDS - 1);
    assert!(
        !tasks.is_empty(),
        "shutdown must still track the detached writer"
    );
    stop.cancel();
    assert!(matches!(skills.status().await, Err(SkillsError::Stopping)));
    release.send(()).unwrap();
    let _closed = tasks.close();
    tasks.wait().await;
    assert_eq!(skills.permits.available_permits(), MAX_COMMANDS);
    assert_eq!(data.read_skills_selection().unwrap(), Some(persisted));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_commands_install_confirm_removal_and_preserve_uninstalled_choice_on_restart() {
    use vessel_proto::skills::{ImportSkillsSelection, SkillImportOutcome, SkillsState};
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let config = Config::new(data.clone(), HarnessEnv::new(root.path().join("home")));
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let skills = Skills::start(&config, tasks.clone(), stop.clone());
    let fresh = skills.status().await.unwrap();
    assert_eq!(fresh.state, SkillsState::NotInstalled);
    assert_eq!(fresh.selection, SkillSelection::All);
    assert!(fresh.installed.is_empty());
    assert!(
        fresh
            .available
            .as_slice()
            .iter()
            .any(|name| name == "vessel")
    );
    let installed = skills.reconcile().await.unwrap();
    assert_eq!(installed.state, SkillsState::UpToDate);
    assert_eq!(installed.installed, fresh.available);
    let selection = SkillSelection::Custom {
        skills: SkillNames::new(vec!["vessel".parse().unwrap()]).unwrap(),
    };
    let first = skills
        .save(SaveSkillsSelection {
            selection: selection.clone(),
            confirmed_removals: SkillNames::default(),
        })
        .await
        .unwrap();
    let removals = first.confirmation_required.unwrap();
    assert!(!removals.is_empty());
    assert!(!removals.contains(&"vessel".parse().unwrap()));
    assert_eq!(first.snapshot.installed, installed.installed);
    assert_eq!(data.read_skills_selection().unwrap(), None);
    let saved = skills
        .save(SaveSkillsSelection {
            selection: selection.clone(),
            confirmed_removals: removals,
        })
        .await
        .unwrap();
    assert!(saved.confirmation_required.is_none());
    assert_eq!(saved.snapshot.selection, selection);
    assert_eq!(saved.snapshot.installed.as_slice().len(), 1);
    assert_eq!(
        data.read_skills_selection().unwrap(),
        Some(selection.clone())
    );
    let removed = skills.uninstall().await.unwrap();
    assert_eq!(removed.state, SkillsState::NotInstalled);
    assert!(removed.installed.is_empty());
    assert_eq!(
        data.read_skills_selection().unwrap(),
        Some(selection.clone())
    );
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
    drop(skills);

    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let restarted = Skills::start(&config, tasks.clone(), stop.clone());
    let status = restarted.status().await.unwrap();
    assert_eq!(status.state, SkillsState::NotInstalled);
    assert_eq!(status.selection, selection);
    let imported = restarted
        .import(ImportSkillsSelection {
            selection: SkillSelection::All,
        })
        .await
        .unwrap();
    assert_eq!(imported.outcome, SkillImportOutcome::AlreadySet);
    assert_eq!(imported.selection, selection);
    let reinstalled = restarted.reconcile().await.unwrap();
    assert_eq!(reinstalled.state, SkillsState::UpToDate);
    assert_eq!(reinstalled.installed.as_slice().len(), 1);
    assert_eq!(reinstalled.selection, selection);
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_import_acknowledges_only_a_valid_initial_choice_and_survives_restart() {
    use vessel_proto::skills::{ImportSkillsSelection, SkillImportOutcome, SkillsState};
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let config = Config::new(data.clone(), HarnessEnv::new(root.path().join("home")));
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let skills = Skills::start(&config, tasks.clone(), stop.clone());
    let unknown = SkillSelection::Custom {
        skills: SkillNames::new(vec!["not-shipped".parse().unwrap()]).unwrap(),
    };
    assert!(
        skills
            .import(ImportSkillsSelection { selection: unknown })
            .await
            .is_err()
    );
    assert_eq!(data.read_skills_selection().unwrap(), None);
    let selection = SkillSelection::Custom {
        skills: SkillNames::default(),
    };
    let imported = skills
        .import(ImportSkillsSelection {
            selection: selection.clone(),
        })
        .await
        .unwrap();
    assert_eq!(imported.outcome, SkillImportOutcome::Imported);
    assert_eq!(imported.selection, selection);
    assert_eq!(
        data.read_skills_selection().unwrap(),
        Some(selection.clone())
    );
    let status = skills.status().await.unwrap();
    assert_eq!(status.state, SkillsState::NotInstalled);
    assert!(
        status.installed.is_empty(),
        "import is not an implicit install"
    );
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
    drop(skills);

    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let restarted = Skills::start(&config, tasks.clone(), stop.clone());
    let retained = restarted
        .import(ImportSkillsSelection {
            selection: SkillSelection::All,
        })
        .await
        .unwrap();
    assert_eq!(retained.outcome, SkillImportOutcome::AlreadySet);
    assert_eq!(retained.selection, selection);
    assert_eq!(data.read_skills_selection().unwrap(), Some(selection));
    assert!(restarted.status().await.unwrap().installed.is_empty());
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}
