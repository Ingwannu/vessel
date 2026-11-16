//! Protocol 26 over the wire (ADR 0083, ADR 0105): the file panels' five
//! calls answered by a real node for a person, a reader refused the one
//! that changes the disk, and an agent refused all five, as it is for
//! `file.read`.

use std::path::PathBuf;

use vessel_client::NodeClient;
use vessel_proto::workspace::directory_metadata::{IgnoredEntries, InspectDirectory};
use vessel_proto::workspace::panel_files::{
    DuplicateFileEntry, FileOffset, FileSize, ObserveFile, ObservedFile, ReadFileBytes,
};
use vessel_proto::workspace::suggestions::{
    SuggestEntries, SuggestionKinds, SuggestionMatch, SuggestionQuery,
};
use vessel_proto::workspace::{EntryKind, RelPath, WorkspaceRoot};
use vessel_proto::{ErrorCode, ProtocolVersion, Scope};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

/// The protocol that assigned the five calls.
const NEEDS: ProtocolVersion = ProtocolVersion::new(26);

fn inspect(root: WorkspaceRoot) -> InspectDirectory {
    InspectDirectory {
        root,
        directory: RelPath::root(),
        ignored: IgnoredEntries::Include,
    }
}

fn first_page(root: WorkspaceRoot, path: RelPath) -> ReadFileBytes {
    ReadFileBytes {
        root,
        path,
        offset: FileOffset::ZERO,
        expected: None,
    }
}

fn suggest(root: WorkspaceRoot, query: &str) -> SuggestEntries {
    SuggestEntries {
        root,
        directory: RelPath::root(),
        query: SuggestionQuery::try_from(query.to_owned()).unwrap(),
        kinds: SuggestionKinds::Files,
        matching: SuggestionMatch::Fuzzy,
        limit: None,
    }
}

fn readme() -> RelPath {
    "README.md".parse().unwrap()
}

/// The owner's project: its workspace root and its folder on disk.
async fn project_on(owner: &NodeClient) -> (tempfile::TempDir, WorkspaceRoot, PathBuf) {
    let (folder, project) = crate::tidying::project(owner).await;
    let root = WorkspaceRoot::Project {
        project: project.id,
    };
    (folder, root, project.root)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_lists_reads_duplicates_finds_and_observes_files() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.harness_lookup = crate::workspace::lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        assert!(owner.protocol() >= NEEDS);
        let (_folder, root, disk) = project_on(&owner).await;

        let listed = owner
            .inspect_workspace_directory(inspect(root))
            .await
            .unwrap();
        let entry = listed
            .entries
            .iter()
            .find(|entry| entry.path == readme())
            .unwrap();
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size, Some(FileSize(6)));
        assert!(entry.modified.is_some(), "{entry:?}");

        let page = owner
            .read_workspace_byte_page(first_page(root, readme()))
            .await
            .unwrap();
        assert_eq!(page.bytes.bytes().as_ref(), b"hello\n");
        assert_eq!(page.revision.size, FileSize(6));
        assert_eq!(page.next, None);

        let copy = owner
            .duplicate_workspace_entry(DuplicateFileEntry {
                root,
                path: readme(),
            })
            .await
            .unwrap();
        assert_eq!(copy.path.as_str(), "README copy.md");
        assert_eq!(
            std::fs::read(disk.join("README copy.md")).unwrap(),
            b"hello\n"
        );

        let found = owner
            .suggest_workspace_entries(suggest(root, "READ"))
            .await
            .unwrap();
        assert!(
            found.entries.iter().any(|entry| entry.path == readme()),
            "{found:?}"
        );

        let observe = |after| ObserveFile {
            root,
            path: readme(),
            after,
        };
        let first = owner.observe_workspace_file(observe(None)).await.unwrap();
        assert_eq!(
            first,
            ObservedFile::Ready {
                revision: page.revision.clone()
            }
        );
        // The write may land before or after the node starts watching: it
        // registers before its first read, so either way the wait ends with
        // the new revision, never the old one.
        let waiting = owner.observe_workspace_file(observe(Some(first.clone())));
        let write = async {
            std::fs::write(disk.join("README.md"), "hello again\n").unwrap();
        };
        let (changed, ()) = tokio::join!(waiting, write);
        let changed = changed.unwrap();
        let ObservedFile::Ready { revision } = &changed else {
            panic!("expected the changed file, got {changed:?}");
        };
        assert_eq!(revision.size, FileSize(12));
        assert_ne!(revision, &page.revision);

        std::fs::remove_file(disk.join("README.md")).unwrap();
        assert_eq!(
            owner
                .observe_workspace_file(observe(Some(changed)))
                .await
                .unwrap(),
            ObservedFile::Missing
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reader_observes_and_reads_but_cannot_duplicate() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = crate::workspace::lookup(&fixture);
                crate::secure::advertise_placeholder_route(config);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, root, disk) = project_on(&owner).await;
        let reader = crate::side_questions::access::device(&owner, &node, vec![Scope::Read]).await;

        assert!(matches!(
            reader
                .call(ObserveFile {
                    root,
                    path: readme(),
                    after: None,
                })
                .await
                .unwrap(),
            ObservedFile::Ready { .. }
        ));
        let page = reader.call(first_page(root, readme())).await.unwrap();
        assert_eq!(page.bytes.bytes().as_ref(), b"hello\n");
        let listed = reader.call(inspect(root)).await.unwrap();
        assert!(listed.entries.iter().any(|entry| entry.path == readme()));
        let found = reader.call(suggest(root, "")).await.unwrap();
        assert!(
            found.entries.iter().any(|entry| entry.path == readme()),
            "{found:?}"
        );
        assert_eq!(
            code(
                reader
                    .call(DuplicateFileEntry {
                        root,
                        path: readme(),
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert!(!disk.join("README copy.md").exists());
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_is_refused_every_file_panel_call() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.vessel_mcp = Some(vessel_mcp(&fixture));
                config.harness_lookup = crate::workspace::lookup(&fixture);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, root, disk) = project_on(&owner).await;
        let mut running = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;

        let refusals = [
            code(agent.call(inspect(root)).await),
            code(agent.call(first_page(root, readme())).await),
            code(
                agent
                    .call(DuplicateFileEntry {
                        root,
                        path: readme(),
                    })
                    .await,
            ),
            code(
                agent
                    .call(ObserveFile {
                        root,
                        path: readme(),
                        after: None,
                    })
                    .await,
            ),
            code(agent.call(suggest(root, "")).await),
        ];
        assert_eq!(refusals, [ErrorCode::Forbidden; 5]);
        assert!(!disk.join("README copy.md").exists());
        node.stop().await.unwrap();
    })
    .await;
}
