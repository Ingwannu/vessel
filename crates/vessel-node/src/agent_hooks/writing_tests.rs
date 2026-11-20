//! How the installer writes a person's files: through a link to its target
//! (node review R2), with a backup no second writer overwrites, and one
//! worker that ends on the latest state wanted (R3). Scratch homes only.

use tokio_util::task::TaskTracker;

use super::files::{backup_of, replace};
use super::tests::{commands, read, scratch};
use super::*;

/// A `settings.json` linked into a person's dotfiles stays a link: Vessel
/// writes the file it points at, as Paseo's `writePrivateFileAtomicSync`
/// does, and backs up the target's bytes beside the link.
#[cfg(unix)]
#[test]
fn a_linked_file_stays_a_link_and_its_target_takes_the_hooks() {
    let temp = tempfile::tempdir().unwrap();
    let (home, targets) = scratch(temp.path());
    let dotfiles = temp.path().join("dotfiles");
    std::fs::create_dir_all(&dotfiles).unwrap();
    let target = dotfiles.join("claude-settings.json");
    let original = "{\n  \"model\": \"opus\"\n}\n";
    std::fs::write(&target, original).unwrap();
    let link = home.join(".claude").join("settings.json");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    targets.apply(true);
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link stays a link"
    );
    assert_eq!(std::fs::read_link(&link).unwrap(), target);
    let installed = read(&target);
    assert_eq!(installed["model"], "opus");
    assert_eq!(
        commands(&installed, "Stop"),
        vec![shell_command(Agent::Claude, "Stop")]
    );
    assert_eq!(
        std::fs::read_to_string(backup_of(&link)).unwrap(),
        original,
        "the backup holds the target's bytes, beside the link"
    );
    assert!(!backup_of(&target).exists());

    targets.apply(false);
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    // Off leaves the target without Vessel's entries. As in Paseo, an
    // emptied `hooks` object stays (node review note 4).
    let off = read(&target);
    assert_eq!(off["model"], "opus");
    assert_eq!(off["hooks"], serde_json::json!({}));
}

/// The first backup is the file as it was before Vessel's first change, and
/// a later write never replaces it, even when no backup existed a moment
/// before (`create_new`).
#[test]
fn the_first_backup_is_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("settings.json");
    std::fs::write(&path, "first").unwrap();
    assert!(matches!(
        replace(&path, b"second"),
        files::Outcome::Changed(_)
    ));
    assert_eq!(std::fs::read_to_string(backup_of(&path)).unwrap(), "first");
    assert!(matches!(
        replace(&path, b"third"),
        files::Outcome::Changed(_)
    ));
    assert_eq!(std::fs::read_to_string(backup_of(&path)).unwrap(), "first");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "third");
    // A backup another writer made first is kept as it is.
    let other = temp.path().join("hooks.json");
    std::fs::write(&other, "mine").unwrap();
    std::fs::write(backup_of(&other), "made first").unwrap();
    assert!(matches!(
        replace(&other, b"vessel's"),
        files::Outcome::Changed(_)
    ));
    assert_eq!(
        std::fs::read_to_string(backup_of(&other)).unwrap(),
        "made first"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(backup_of(&path))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "the backup is the owner's only");
    }
}

/// On then off, as fast as a person can click: whatever order the worker
/// sees them in, the hooks end removed, and many changes end on the last.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_last_state_wanted_is_the_one_left() {
    for round in 0_u8..20 {
        let temp = tempfile::tempdir().unwrap();
        let (home, targets) = scratch(temp.path());
        let applier = Applier::new(targets);
        let tasks = TaskTracker::new();
        applier.want(true, &tasks);
        applier.want(false, &tasks);
        let _closed = tasks.close();
        tasks.wait().await;
        let settings = home.join(".claude").join("settings.json");
        let left = std::fs::read_to_string(&settings).unwrap_or_default();
        assert!(!left.contains("VESSEL_HOOK_CLI"), "round {round}: {left}");

        let tasks = TaskTracker::new();
        let last = round.is_multiple_of(2);
        for step in 0_u8..9 {
            applier.want(step.is_multiple_of(2), &tasks);
        }
        applier.want(last, &tasks);
        let _closed = tasks.close();
        tasks.wait().await;
        let installed = std::fs::read_to_string(&settings)
            .unwrap_or_default()
            .contains("VESSEL_HOOK_CLI");
        assert_eq!(installed, last, "round {round}");
    }
}
