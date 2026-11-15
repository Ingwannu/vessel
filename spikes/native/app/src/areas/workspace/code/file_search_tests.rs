//! Repeated queries and stale scopes must not consume the current picker.

use gpui::TestAppContext;
use vessel_proto::ProjectId;

use super::*;

fn receive(
    _: &mut Shell,
    _: FileSearchRequest,
    _: Result<FoundFiles, Problem>,
    _: &mut Context<'_, Shell>,
) {
}

fn request(key: SearchKey) -> FileSearchRequest {
    FileSearchRequest {
        machine: Name::parse("node").unwrap(),
        root: WorkspaceRoot::Project {
            project: ProjectId::new(),
        },
        query: "alpha".parse().unwrap(),
        key,
        limit: NonZeroU32::new(30).unwrap(),
    }
}

fn ask(request: &FileSearchRequest) -> FindFiles {
    FindFiles {
        root: request.root,
        query: request.query.clone(),
        key: Some(request.key),
        limit: Some(request.limit.get()),
    }
}

#[gpui::test]
fn repeated_query_keeps_new_generation_after_old_reply(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _| {
        let key = shell.file_search_key();
        assert_ne!(key, shell.file_search_key());
        let first = request(key);
        shell
            .search_workspace_files(first.clone(), receive)
            .unwrap();
        let old_ticket = shell
            .areas
            .workspace
            .code
            .searches
            .0
            .get(&key)
            .unwrap()
            .ticket;
        let mut middle = first.clone();
        middle.query = "beta".parse().unwrap();
        shell.search_workspace_files(middle, receive).unwrap();
        shell
            .search_workspace_files(first.clone(), receive)
            .unwrap();
        let current = shell
            .areas
            .workspace
            .code
            .searches
            .0
            .get(&key)
            .unwrap()
            .ticket;
        assert_ne!(old_ticket, current);
        assert!(
            shell
                .take_file_search("node", &ask(&first), old_ticket)
                .is_none()
        );
        assert!(
            shell
                .take_file_search("other", &ask(&first), current)
                .is_none()
        );
        let mut wrong_root = ask(&first);
        wrong_root.root = WorkspaceRoot::Project {
            project: ProjectId::new(),
        };
        assert!(
            shell
                .take_file_search("node", &wrong_root, current)
                .is_none()
        );
        assert_eq!(
            shell
                .take_file_search("node", &ask(&first), current)
                .unwrap()
                .0,
            first
        );
        assert!(
            shell
                .take_file_search("node", &ask(&first), current)
                .is_none()
        );
    });
}

#[gpui::test]
fn cancellation_and_capacity_preserve_other_fields(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _| {
        let mut first = None;
        for _ in 0..MAX_FILE_SEARCH_OWNERS {
            let key = shell.file_search_key();
            let request = request(key);
            first.get_or_insert_with(|| request.clone());
            shell.search_workspace_files(request, receive).unwrap();
        }
        let first = first.unwrap();
        shell
            .search_workspace_files(first.clone(), receive)
            .unwrap();
        let key = shell.file_search_key();
        let extra = request(key);
        assert_eq!(
            shell.search_workspace_files(extra.clone(), receive),
            Err(FileSearchRefusal::Capacity)
        );
        shell.cancel_file_search(first.key);
        shell
            .search_workspace_files(extra.clone(), receive)
            .unwrap();
        let ticket = shell
            .areas
            .workspace
            .code
            .searches
            .0
            .get(&key)
            .unwrap()
            .ticket;
        shell.cancel_file_search(key);
        assert!(
            shell
                .take_file_search("node", &ask(&extra), ticket)
                .is_none()
        );
        let mut huge = extra;
        huge.limit = NonZeroU32::new(FILE_SEARCH_LIMIT + 1).unwrap();
        assert_eq!(
            shell.search_workspace_files(huge, receive),
            Err(FileSearchRefusal::Limit)
        );
    });
}
