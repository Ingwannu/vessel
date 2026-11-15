//! Actual owned metadata and worker failures stay private in every error layer.
use super::*;
use std::error::Error;

const MANIFEST_CANARY: &str = "PRIVATE_SKILL_MANIFEST_FIELD";
const JOURNAL_CANARY: &str = "PRIVATE_SKILL_JOURNAL_OWNER";
const PANIC_CANARY: &str = "PRIVATE_SKILL_TASK_PAYLOAD";
const MAX_ERROR_CHAIN: usize = 16;

#[cfg(test)]
fn assert_closed(error: &(dyn Error + 'static), canary: &str) {
    let mut at = error;
    for _level in 0..MAX_ERROR_CHAIN {
        assert!(!format!("{at}").contains(canary));
        assert!(!format!("{at:?}").contains(canary));
        let Some(next) = at.source() else {
            return;
        };
        at = next;
    }
    panic!("diagnostic source chain did not terminate");
}

#[tokio::test]
async fn actual_manifest_and_journal_failures_seal_enclosing_sources() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let home = root.path().join("home");
    let config = Config::new(data.clone(), HarnessEnv::new(home.clone()));
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let skills = Skills::start(&config, tasks.clone(), stop.clone());
    let _installed = skills.reconcile().await.unwrap();
    let manifest = home.join(".agents/skills/vessel/.vessel-managed-files.json");
    let original = std::fs::read(&manifest).unwrap();
    let malformed = format!(r#"{{"version":1,"files":{{}},"{MANIFEST_CANARY}":true}}"#);
    std::fs::write(&manifest, &malformed).unwrap();
    let error = skills.status().await.unwrap_err();
    assert!(matches!(error, SkillsError::Skill(SkillError::Manifest(_))));
    assert_closed(&error, MANIFEST_CANARY);
    let error = skills
        .save(SaveSkillsSelection {
            selection: SkillSelection::All,
            confirmed_removals: SkillNames::default(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        SkillsError::Selection(SelectionError::Skill(SkillError::Manifest(_)))
    ));
    assert_closed(&error, MANIFEST_CANARY);
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), malformed);
    std::fs::write(&manifest, original).unwrap();
    let selection = format!(r#"{{"selection":{{"mode":"{MANIFEST_CANARY}"}}}}"#);
    let selection_file = data.root().join("skills.json");
    std::fs::write(&selection_file, &selection).unwrap();
    let error = skills.status().await.unwrap_err();
    assert!(matches!(error, SkillsError::Store(_)));
    assert_closed(&error, MANIFEST_CANARY);
    assert_eq!(std::fs::read_to_string(&selection_file).unwrap(), selection);
    std::fs::remove_file(selection_file).unwrap();

    let journal = data
        .root()
        .join("skill-transactions/transaction-0000000000000001");
    std::fs::create_dir_all(&journal).unwrap();
    let document = format!(r#"{{"owner":"{JOURNAL_CANARY}"}}"#);
    std::fs::write(journal.join("transaction.json"), &document).unwrap();
    let error = skills.status().await.unwrap_err();
    assert!(matches!(error, SkillsError::Skill(SkillError::Manifest(_))));
    assert_closed(&error, JOURNAL_CANARY);
    assert_eq!(
        std::fs::read_to_string(journal.join("transaction.json")).unwrap(),
        document
    );
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}

#[tokio::test]
async fn actual_worker_panic_and_cancelled_task_expose_only_closed_categories() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let config = Config::new(data, HarnessEnv::new(root.path().join("home")));
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let skills = Skills::start(&config, tasks.clone(), stop.clone());
    let _opened = skills.status().await.unwrap();
    let error = skills
        .run::<()>(|_manager, _data, _durable| panic!("{PANIC_CANARY}"))
        .await
        .unwrap_err();
    assert!(matches!(error, SkillsError::Lost(_)));
    assert_closed(&error, PANIC_CANARY);
    assert!(format!("{error:?}").contains("Panicked"));
    assert_eq!(skills.permits.available_permits(), MAX_COMMANDS);
    // This checks the returned diagnostic chain. It does not suppress or
    // assert sanitization of the process-wide Rust panic hook.
    let cancelled = tokio::spawn(std::future::pending::<()>());
    cancelled.abort();
    let error = SkillsError::from(cancelled.await.unwrap_err());
    assert_closed(&error, PANIC_CANARY);
    assert!(format!("{error:?}").contains("Cancelled"));
    stop.cancel();
    let _closed = tasks.close();
    tasks.wait().await;
}
