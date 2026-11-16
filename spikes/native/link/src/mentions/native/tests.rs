//! No node, filesystem, agent, or operator state is used by this boundary.
use super::*;
use std::sync::Mutex;
use vessel_app_view::{
    composer::mentions::{Menu, Scope},
    machine_book::Name,
};
use vessel_proto::{
    ProjectId,
    workspace::{DirEntry, FoundFile, WorkspaceRoot},
};

struct Fake {
    list: Result<DirListing, Failure>,
    files: Result<FoundFiles, Failure>,
    listed: Mutex<Vec<ListDir>>,
    found: Mutex<Vec<FindFiles>>,
}
impl Source for Fake {
    fn list(&self, ask: ListDir) -> impl Future<Output = Result<DirListing, Failure>> + Send {
        self.listed.lock().unwrap().push(ask);
        std::future::ready(self.list.clone())
    }
    fn find(&self, ask: FindFiles) -> impl Future<Output = Result<FoundFiles, Failure>> + Send {
        self.found.lock().unwrap().push(ask);
        std::future::ready(self.files.clone())
    }
}
fn fake() -> Fake {
    Fake {
        list: Ok(DirListing {
            entries: vec![
                entry("folder", EntryKind::Dir),
                entry("symbolic", EntryKind::Symlink),
                entry("readme.md", EntryKind::File),
            ],
            more: false,
        }),
        files: Ok(FoundFiles {
            matches: vec![FoundFile {
                path: "src/found.rs".parse().unwrap(),
                positions: vec![],
            }],
            indexed: 1,
            complete: true,
            superseded: false,
        }),
        listed: Mutex::new(vec![]),
        found: Mutex::new(vec![]),
    }
}
fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size: None,
        ignored: false,
    }
}
fn request(text: &str) -> Request {
    Menu::default()
        .observe(
            Some(Scope {
                connection: vessel_app_view::connection::ConnectionEpochs::default()
                    .admit(
                        vessel_app_view::connection::MachineKey::from_book("fixture"),
                        vessel_proto::NodeId::random(),
                    )
                    .unwrap(),
                draft: DraftKey::New,
                machine: Name::parse("fixture").unwrap(),
                root: WorkspaceRoot::Project {
                    project: ProjectId::random(),
                },
            }),
            text,
            text.len(),
            Duration::ZERO,
        )
        .unwrap()
        .unwrap()
}
#[tokio::test]
async fn blank_mention_lists_only_authorized_children_and_preserves_kind() {
    let source = fake();
    let ask = request("@");
    let rows = lookup(&source, &ask).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].kind, SuggestionKind::Directory);
    assert_eq!(rows[0].path.as_str(), "folder");
    assert!(source.found.lock().unwrap().is_empty());
    assert_eq!(source.listed.lock().unwrap()[0].root, ask.scope.root);
    assert_eq!(source.listed.lock().unwrap()[0].dir, RelPath::root());
}
#[tokio::test]
async fn parent_local_directories_join_the_requested_parent_without_following_links() {
    let source = fake();
    let ask = request("@src/fo");
    let rows = lookup(&source, &ask).await.unwrap();
    assert_eq!(
        rows.iter().map(|row| row.path.as_str()).collect::<Vec<_>>(),
        ["src/found.rs", "src/folder"]
    );
    let listed = source.listed.lock().unwrap();
    assert_eq!(listed[0].dir.as_str(), "src");
    assert!(!listed[0].ignored);
    let found = source.found.lock().unwrap();
    assert_eq!(found[0].query.as_str(), "src/fo");
    assert_eq!(found[0].limit, Some(LIMIT));
}
#[tokio::test]
async fn failed_directory_query_is_not_presented_as_complete_empty_success() {
    let mut source = fake();
    source.list = Err(Failure::Refused(vessel_proto::ErrorCode::Forbidden));
    assert_eq!(
        lookup(&source, &request("@file")).await,
        Err(Failure::Refused(vessel_proto::ErrorCode::Forbidden))
    );
}
#[test]
fn malformed_child_names_cannot_escape_the_captured_parent_and_bounds_apply() {
    let parent = "src".parse().unwrap();
    for name in [
        "../other",
        "/absolute",
        "nested/child",
        "..",
        "",
        "C:\\secret",
    ] {
        assert_eq!(
            project_listing(
                DirListing {
                    entries: vec![entry(name, EntryKind::File)],
                    more: false
                },
                &parent
            ),
            Err(Failure::InvalidReply)
        );
    }
    let entries = (0..RESULT_LIMIT.saturating_add(1))
        .map(|index| entry(&format!("{index}.rs"), EntryKind::File))
        .collect();
    assert_eq!(
        project_listing(
            DirListing {
                entries,
                more: true
            },
            &parent
        )
        .unwrap()
        .len(),
        RESULT_LIMIT
    );
}
