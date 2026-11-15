//! Kill a real child at the public persistence boundary, then reopen its journal.
//!
//! The callback uses the existing atomic file writer as a synthetic preference
//! owner. This proves process-interruption recovery of fleet's real mechanism,
//! not the node Store join, power-loss durability or every journal crash edge.
use std::io::{BufRead as _, Read as _, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use vessel_proto::skills::{SkillNames, SkillSelection};

use super::manager::{SkillLocations, SkillManager};
use super::tests::{catalogue, service};
use crate::files::{self, Mode};

const CHILD_TEST: &str = "orchestration::staged::crash_tests::skill_transaction_child";
const CHILD_ROOT: &str = "VESSEL_TEST_SKILL_CRASH_ROOT";
const CHILD_OPERATION: &str = "VESSEL_TEST_SKILL_CRASH_OPERATION";
const CHILD_EDGE: &str = "VESSEL_TEST_SKILL_CRASH_EDGE";
const READY: &str = "skill-transaction-held";
const CHILD_READY_BUDGET: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
enum Operation {
    Update,
    Delete,
}
impl Operation {
    fn argument(self) -> &'static str {
        match self {
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
    fn selection(self) -> SkillSelection {
        match self {
            Self::Update => SkillSelection::All,
            Self::Delete => SkillSelection::Custom {
                skills: SkillNames::default(),
            },
        }
    }
}
#[derive(Clone, Copy)]
enum Edge {
    BeforePreference,
    AfterPreference,
}
impl Edge {
    fn argument(self) -> &'static str {
        match self {
            Self::BeforePreference => "before",
            Self::AfterPreference => "after",
        }
    }
}

fn manager(root: &Path, operation: Operation) -> SkillManager {
    let bundles = match operation {
        Operation::Update => catalogue("two", Some(("nested/new.md", b"new"))),
        Operation::Delete => catalogue("one", Some(("old.md", b"old"))),
    };
    SkillManager::new(
        bundles,
        SkillLocations::new(
            root.join("agents/skills"),
            root.join("claude/skills"),
            root.join("codex/skills"),
            root.join("transactions"),
        )
        .unwrap(),
    )
}

// A failed assertion must not leave this test's isolated subprocess behind.
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _killed = self.0.kill();
        let _reaped = self.0.wait();
    }
}

fn kill_at_boundary(root: &Path, operation: Operation, edge: Edge) {
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
            .env_clear()
            .env(CHILD_ROOT, root)
            .env(CHILD_OPERATION, operation.argument())
            .env(CHILD_EDGE, edge.argument())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let output = child.0.stdout.take().unwrap();
    let (ready, received) = std::sync::mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let reached = std::io::BufReader::new(output)
            .lines()
            .any(|line| line.unwrap().ends_with(READY));
        let _sent = ready.send(reached);
    });
    let reached = received.recv_timeout(CHILD_READY_BUDGET);
    child.0.kill().unwrap();
    let status = child.0.wait().unwrap();
    assert!(!status.success());
    #[cfg(unix)]
    assert!(status.code().is_none(), "child must die by signal");
    reader.join().unwrap();
    assert!(
        reached.unwrap(),
        "child exited before the exact save boundary"
    );
}

#[test]
fn skill_transaction_child() {
    // Only the isolated subprocess receives these test-only parameters. The
    // product manager still accepts explicit roots and never reads environment.
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let operation = match std::env::var(CHILD_OPERATION).unwrap().as_str() {
        "update" => Operation::Update,
        "delete" => Operation::Delete,
        value => panic!("unexpected child operation: {value}"),
    };
    let edge = match std::env::var(CHILD_EDGE).unwrap().as_str() {
        "before" => Edge::BeforePreference,
        "after" => Edge::AfterPreference,
        value => panic!("unexpected child edge: {value}"),
    };
    let root = Path::new(&root);
    let confirmed = SkillNames::new(vec!["vessel".parse().unwrap()]).unwrap();
    let _saved = manager(root, operation)
        .save(None, &operation.selection(), &confirmed, |next| {
            if matches!(edge, Edge::AfterPreference) {
                files::replace(
                    &root.join("preference.json"),
                    Mode::Private,
                    &serde_json::to_vec(next).unwrap(),
                )?;
            }
            let mut output = std::io::stdout().lock();
            writeln!(output, "{READY}")?;
            output.flush()?;
            // No destructor or success acknowledgement can run before parent kill.
            std::io::stdin().read_exact(&mut [0_u8])
        })
        .unwrap();
    panic!("the parent did not kill its held child");
}

#[test]
fn killed_skill_save_recovers_updates_and_deletions_on_both_preference_edges() {
    for operation in [Operation::Update, Operation::Delete] {
        for edge in [Edge::BeforePreference, Edge::AfterPreference] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            let mut initial = service(root, catalogue("one", Some(("old.md", b"old"))));
            let _installed = initial.reconcile(&SkillSelection::All).unwrap();
            for home in ["agents/skills", "claude/skills"] {
                std::fs::write(root.join(home).join("vessel/user.md"), b"user").unwrap();
                std::fs::create_dir(root.join(home).join("unrelated")).unwrap();
                std::fs::write(root.join(home).join("unrelated/keep.md"), b"keep").unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    std::fs::set_permissions(
                        root.join(home).join("vessel/SKILL.md"),
                        std::fs::Permissions::from_mode(0o750),
                    )
                    .unwrap();
                }
            }
            kill_at_boundary(root, operation, edge);
            let durable = match edge {
                Edge::BeforePreference => {
                    assert!(!root.join("preference.json").exists());
                    None
                }
                Edge::AfterPreference => Some(
                    serde_json::from_slice::<SkillSelection>(
                        &std::fs::read(root.join("preference.json")).unwrap(),
                    )
                    .unwrap(),
                ),
            };
            if matches!(edge, Edge::AfterPreference) {
                assert_eq!(durable, Some(operation.selection()));
            }
            let mut reopened = manager(root, operation);
            let snapshot = reopened.status(durable.as_ref()).unwrap();
            assert_eq!(reopened.status(durable.as_ref()).unwrap(), snapshot);
            assert_eq!(
                std::fs::read_dir(root.join("transactions"))
                    .unwrap()
                    .count(),
                0
            );
            verify_files(root, operation, edge);
        }
    }
}

fn verify_files(root: &Path, operation: Operation, edge: Edge) {
    for home in ["agents/skills", "claude/skills"] {
        let home = root.join(home);
        assert_eq!(
            std::fs::read(home.join("unrelated/keep.md")).unwrap(),
            b"keep"
        );
        let expected_entries = match (operation, edge) {
            (Operation::Delete, Edge::AfterPreference) => 1,
            (Operation::Update, Edge::BeforePreference | Edge::AfterPreference)
            | (Operation::Delete, Edge::BeforePreference) => 2,
        };
        assert_eq!(std::fs::read_dir(&home).unwrap().count(), expected_entries);
        let skill = home.join("vessel");
        match (operation, edge) {
            (Operation::Delete, Edge::AfterPreference) => assert!(!skill.exists()),
            (Operation::Update | Operation::Delete, Edge::BeforePreference) => {
                assert_eq!(std::fs::read(skill.join("old.md")).unwrap(), b"old");
                assert!(!skill.join("nested/new.md").exists());
                assert!(
                    std::fs::read_to_string(skill.join("SKILL.md"))
                        .unwrap()
                        .ends_with("one\n")
                );
                assert_eq!(std::fs::read(skill.join("user.md")).unwrap(), b"user");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    assert_eq!(
                        std::fs::metadata(skill.join("SKILL.md"))
                            .unwrap()
                            .permissions()
                            .mode()
                            & 0o777,
                        0o750
                    );
                }
            }
            (Operation::Update, Edge::AfterPreference) => {
                assert!(!skill.join("old.md").exists());
                assert_eq!(std::fs::read(skill.join("nested/new.md")).unwrap(), b"new");
                assert!(
                    std::fs::read_to_string(skill.join("SKILL.md"))
                        .unwrap()
                        .ends_with("two\n")
                );
                assert_eq!(std::fs::read(skill.join("user.md")).unwrap(), b"user");
            }
        }
    }
}
