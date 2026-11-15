//! Metadata is installed only with its original directory ticket.
use super::*;
use crate::areas::workspace::code::{io_tests, scope::Acquisition};
use gpui::TestAppContext;
use vessel_app_view::workspace::file_tree::Listing;
use vessel_proto::workspace::EntryKind;
use vessel_proto::workspace::directory_metadata::InspectedEntry;
use vessel_proto::workspace::panel_files::FileSize;

fn snapshot(path: &str, timestamp: Option<u64>) -> InspectedDirectory {
    InspectedDirectory {
        entries: vec![InspectedEntry {
            path: path.parse().unwrap(),
            kind: EntryKind::File,
            size: Some(FileSize(8)),
            modified: timestamp.map(FileModifiedMillis),
            ignored: IgnoreState::Ignored,
        }],
        more: true,
    }
}
#[test]
fn projection_preserves_unknowns_ignore_and_clipping_and_refuses_other_parents() {
    let root = RelPath::root();
    let (listing, modified) = project(&root, snapshot("a", None)).unwrap();
    assert!(listing.more);
    assert!(listing.entries.first().unwrap().ignored);
    assert_eq!(listing.entries.first().unwrap().size, Some(8));
    assert!(modified.is_empty());
    assert!(project(&root, snapshot("nested/a", Some(1))).is_err());
}
#[gpui::test]
fn old_metadata_cannot_replace_a_newer_same_scope_refresh(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _cx| {
        let root = io_tests::root();
        io_tests::target(shell, root);
        let scope = shell.areas.workspace.code.scope().unwrap();
        // Keep the actual captured acquisition; no invented active lease is used.
        let Acquisition { context, .. } = shell
            .areas
            .workspace
            .code
            .target
            .as_ref()
            .unwrap()
            .admission
            .as_ref()
            .unwrap();
        assert_eq!(scope.connection(), context);
        let old = shell.next_file_ticket().unwrap();
        let latest = shell.next_file_ticket().unwrap();
        let dir = RelPath::root();
        let _previous_listing = shell
            .areas
            .workspace
            .code
            .files
            .listings
            .insert(dir.clone(), Listing::Loading);
        let _previous_ticket = shell
            .areas
            .workspace
            .code
            .files
            .listing_tickets
            .insert(dir, latest);
        let ask = InspectDirectory {
            root,
            directory: RelPath::root(),
            ignored: IgnoredEntries::Include,
        };
        shell.directory_inspected(&scope, &ask, latest, Ok(snapshot("a", Some(9))));
        shell.directory_inspected(&scope, &ask, old, Ok(snapshot("a", Some(1))));
        assert_eq!(
            shell
                .areas
                .workspace
                .code
                .files
                .modified
                .get(&"a".parse().unwrap()),
            Some(&FileModifiedMillis(9))
        );
        // A fresh legacy response removes former metadata instead of inventing it.
        let next = shell.next_file_ticket().unwrap();
        let _previous_ticket = shell
            .areas
            .workspace
            .code
            .files
            .listing_tickets
            .insert(RelPath::root(), next);
        shell.folder_read(
            "node",
            &ListDir {
                root,
                dir: RelPath::root(),
                ignored: true,
            },
            next,
            Ok(DirListing {
                entries: vec![],
                more: false,
            }),
        );
        assert!(shell.areas.workspace.code.files.modified.is_empty());
    });
}
