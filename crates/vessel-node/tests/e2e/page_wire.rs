//! Two things a page carries exactly over the wire (protocol 11, the join
//! of wave 2's first half): a file's write guarded by the content hash it
//! read, and a project's set of commands with the set last approved.

use vessel_proto::ErrorCode;
use vessel_proto::workspace::{
    AddProject, ApproveCommands, GetProjectCommands, ProjectCommands, ProjectInfo, ReadFile,
    WorkspaceRoot, WriteExpect, WriteFile,
};

use crate::agents::code;
use crate::support::{Fixture, within};
use crate::workspace::{lookup, repository};

/// A project in a fresh repository, with `vessel.toml` holding `file`.
async fn project_with(
    client: &vessel_client::NodeClient,
    file: &str,
) -> (tempfile::TempDir, ProjectInfo) {
    let folder = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(folder.path()).unwrap().join("app");
    std::fs::create_dir(&root).unwrap();
    repository(&root);
    std::fs::write(root.join("vessel.toml"), file).unwrap();
    let project = client
        .call(AddProject {
            path: root,
            name: None,
        })
        .await
        .unwrap();
    (folder, project)
}

/// A page reads a file, holds its version exactly (a hash, and a time in
/// milliseconds a JavaScript number holds), and writes with that hash: the
/// write lands; one guarded by a content no longer there is `conflict`
/// and changes nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_write_is_guarded_by_the_content_hash_a_page_holds() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project_with(&owner, "").await;
        let root = WorkspaceRoot::Project {
            project: project.id,
        };
        let path: vessel_proto::workspace::RelPath = "README.md".parse().unwrap();
        let read = owner
            .call(ReadFile {
                root,
                path: path.clone(),
                offset: 0,
                limit: None,
            })
            .await
            .unwrap();
        assert!(read.version.modified < 1 << 53, "{:?}", read.version);
        let seen = read.version.hash.clone().unwrap();
        let write = |text: &str, expect| WriteFile {
            root,
            path: path.clone(),
            text: text.to_owned(),
            expect,
        };
        let saved = owner
            .call(write(
                "edited\n",
                WriteExpect::Content { hash: seen.clone() },
            ))
            .await
            .unwrap();
        assert_ne!(saved.hash.as_ref(), Some(&seen));
        // The same guard again: the content it names is gone.
        assert_eq!(
            code(
                owner
                    .call(write("lost\n", WriteExpect::Content { hash: seen }))
                    .await
            ),
            ErrorCode::Conflict
        );
        assert_eq!(
            std::fs::read_to_string(project.root.join("README.md")).unwrap(),
            "edited\n"
        );
        // Guarded by what it wrote, the next save lands.
        let _again = owner
            .call(write(
                "again\n",
                WriteExpect::Content {
                    hash: saved.hash.unwrap(),
                },
            ))
            .await
            .unwrap();
        node.stop().await.unwrap();
    })
    .await;
}

/// Once a set is approved and the file changes, `project.commands` shows
/// the set not approved beside the one that was.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_commands_carry_the_set_last_approved() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project_with(&owner, "[worktree]\nsetup = [\"echo one\"]\n").await;
        let commands = || GetProjectCommands {
            project: project.id,
            base: None,
        };
        let ProjectCommands::Listed { digest, .. } = owner.call(commands()).await.unwrap() else {
            panic!("the file names a setup");
        };
        let _approved = owner
            .call(ApproveCommands {
                project: project.id,
                digest: digest.clone(),
                approved: true,
            })
            .await
            .unwrap();
        std::fs::write(
            project.root.join("vessel.toml"),
            "[worktree]\nsetup = [\"echo two\"]\n",
        )
        .unwrap();
        let ProjectCommands::Listed {
            approved,
            setup,
            last_approved,
            ..
        } = owner.call(commands()).await.unwrap()
        else {
            panic!("the file names a setup");
        };
        assert!(!approved);
        assert_eq!(setup[0].as_str(), "echo two");
        let last = last_approved.expect("the set last approved");
        assert_eq!(last.digest, digest);
        assert_eq!(last.commands.setup[0].as_str(), "echo one");
        node.stop().await.unwrap();
    })
    .await;
}
