//! The page's cases (`add-project/model.test.ts`, `paths.test.ts`,
//! `data/git-remote.test.ts`), so both ports answer the same.

use std::collections::HashMap;

use super::git_remote::{self, Location, Transport};
use super::model::{
    CloneProtocol, Direction, Flow, Machine, Origin, PageId, PageKind, Repository, move_index,
    move_selection,
};
use super::options::{
    Method, MethodNote, clone_locations, clone_remote, filter_machines, methods, suggested_parents,
    typed_repositories,
};
use super::paths::{DirectoryOption, directory_options, is_openable, shorten_path, suggestions};
use super::rows::{Empty, Inputs, RowAction, RowNote, RowTitle, empty, rows};

fn machine(name: &str, label: &str) -> Machine {
    Machine {
        name: name.to_owned(),
        label: label.to_owned(),
        can_add_project: true,
        can_browse: false,
        can_clone: true,
        can_search_github: false,
        can_create_directory: true,
    }
}

fn local() -> Machine {
    machine("host-1", "Local")
}

fn remote() -> Machine {
    machine("host-2", "Remote")
}

fn repository() -> Repository {
    Repository {
        id: "manual:ssh:getpaseo/paseo".to_owned(),
        name_with_owner: "getpaseo/paseo".to_owned(),
        clone_url: "getpaseo/paseo".to_owned(),
        protocol: Some(CloneProtocol::Ssh),
        origin: Origin::Typed,
    }
}

fn strings(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

#[test]
fn a_single_machine_is_skipped_without_history() {
    let mut flow = Flow::open(vec![local()], None);
    assert_eq!(
        flow.page().kind,
        PageKind::Method {
            machine: "host-1".to_owned()
        }
    );
    assert!(!flow.back());
}

#[test]
fn it_starts_on_the_preferred_machine_else_on_the_machine_page() {
    let preferred = Flow::open(vec![local(), remote()], Some("host-2"));
    assert_eq!(preferred.page().kind.machine(), Some("host-2"));
    let neither = Flow::open(vec![local(), remote()], None);
    assert_eq!(neither.page().kind.id(), PageId::Host);
}

#[test]
fn the_only_machine_arriving_late_skips_the_machine_page() {
    let mut flow = Flow::open(Vec::new(), None);
    flow.set_machines(vec![local()], None);
    assert_eq!(flow.page().kind.machine(), Some("host-1"));
    assert_eq!(flow.depth(), 1);
    flow.choose_machine("host-1");
    flow.set_machines(vec![local()], None);
    assert_eq!(flow.depth(), 2);
    assert_eq!(flow.machines, vec![local()]);
}

#[test]
fn back_restores_the_field_and_the_row() {
    let mut flow = Flow::open(vec![local(), remote()], None);
    flow.set_field("rem");
    flow.set_active(1);
    flow.choose_machine("host-2");
    flow.open_directory_search("host-2");
    assert!(flow.back());
    assert!(flow.back());
    let page = flow.page();
    assert_eq!(page.kind, PageKind::Host);
    assert_eq!(
        (page.field.as_str(), page.active, &page.error),
        ("rem", 1, &None)
    );
}

#[test]
fn the_keys_wrap_and_skip_disabled_rows() {
    assert_eq!(move_index(2, 3, Direction::Next), 0);
    assert_eq!(move_index(0, 3, Direction::Previous), 2);
    assert_eq!(move_index(5, 3, Direction::Previous), 0);
    assert_eq!(move_selection(0, &[true, false, true], Direction::Next), 2);
    assert_eq!(move_selection(2, &[true, false, true], Direction::Next), 0);
    assert_eq!(move_selection(1, &[false, false], Direction::Next), 0);
}

#[test]
fn a_name_comes_back_with_its_parent() {
    let mut flow = Flow::open(vec![local()], None);
    flow.open_new_directory_parent("host-1");
    flow.open_new_directory_name("host-1", "~/dev");
    flow.set_field("command-center");
    assert!(flow.back());
    flow.open_new_directory_name("host-1", "~/dev");
    assert_eq!(flow.page().field, "command-center");
    flow.open_new_directory_name("host-1", "~/src");
    assert_eq!(flow.page().field, "");
}

#[test]
fn a_destination_comes_back_with_its_field_and_row() {
    let mut flow = Flow::open(vec![local()], None);
    flow.open_github_location("host-1", repository());
    flow.set_field("~/dev");
    flow.set_active(2);
    assert!(flow.back());
    flow.open_github_location("host-1", repository());
    assert_eq!(
        (flow.page().field.as_str(), flow.page().active),
        ("~/dev", 2)
    );
}

#[test]
fn progress_and_errors_land_only_on_the_page_shown() {
    let mut flow = Flow::open(vec![local()], None);
    flow.open_directory_search("host-1");
    flow.begin(PageId::DirectorySearch);
    assert!(flow.page().submitting);
    flow.fail(PageId::DirectorySearch, "Directory not found".to_owned());
    assert_eq!(flow.page().error.as_deref(), Some("Directory not found"));
    assert!(!flow.page().submitting);
    assert!(flow.back());
    let before = flow.clone();
    flow.fail(PageId::DirectorySearch, "late".to_owned());
    assert_eq!(flow, before);
    flow.open_directory_search("host-1");
    flow.fail(PageId::DirectorySearch, "x".to_owned());
    flow.set_field("~/c");
    assert_eq!(
        (flow.page().field.as_str(), &flow.page().error),
        ("~/c", &None)
    );
}

#[test]
fn a_machine_that_cannot_add_has_no_methods() {
    let outdated = Machine {
        can_add_project: false,
        ..local()
    };
    assert!(methods(&outdated).is_empty());
    assert_eq!(empty(PageId::Method, Some(&outdated)), Empty::UpdateMachine);
    assert_eq!(empty(PageId::Method, Some(&local())), Empty::NoMatches);
}

#[test]
fn methods_needing_an_update_stay_listed_and_browse_is_local_only() {
    let old = Machine {
        can_clone: false,
        can_create_directory: false,
        ..local()
    };
    let listed = methods(&old);
    let ids: Vec<Method> = listed.iter().map(|option| option.method).collect();
    assert_eq!(
        ids,
        [
            Method::DirectorySearch,
            Method::Github,
            Method::NewDirectory
        ]
    );
    assert_eq!(
        listed.get(1).map(|option| (&option.note, option.disabled)),
        Some((&MethodNote::CloneNeedsUpdate, true))
    );
    assert_eq!(
        listed.get(2).map(|option| (&option.note, option.disabled)),
        Some((&MethodNote::CreateNeedsUpdate, true))
    );
}

#[test]
fn paseos_four_methods_in_its_order() {
    let here = Machine {
        can_browse: true,
        ..local()
    };
    let listed = methods(&here);
    let ids: Vec<Method> = listed.iter().map(|option| option.method).collect();
    assert_eq!(
        ids,
        [
            Method::DirectorySearch,
            Method::Browse,
            Method::Github,
            Method::NewDirectory
        ]
    );
    let notes: Vec<&MethodNote> = listed.iter().map(|option| &option.note).collect();
    assert_eq!(
        notes,
        [
            &MethodNote::FindOn("Local".to_owned()),
            &MethodNote::BrowseIn,
            &MethodNote::CloneByHand,
            &MethodNote::CreateOn("Local".to_owned()),
        ]
    );
}

#[test]
fn machines_filter_by_label_or_name() {
    let all = [local(), remote()];
    let names = |found: Vec<&Machine>| {
        found
            .into_iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(names(filter_machines(&all, " rem ")), ["host-2"]);
    assert_eq!(names(filter_machines(&all, "host-1")), ["host-1"]);
    assert_eq!(filter_machines(&all, "").len(), 2);
}

#[test]
fn typed_repositories_are_a_remote_or_owner_repo_twice() {
    let remote = typed_repositories("git@github.com:getpaseo/paseo.git");
    assert_eq!(
        remote,
        [Repository {
            id: "manual:git@github.com:getpaseo/paseo.git".to_owned(),
            name_with_owner: "getpaseo/paseo".to_owned(),
            clone_url: "git@github.com:getpaseo/paseo.git".to_owned(),
            protocol: None,
            origin: Origin::Typed,
        }]
    );
    let pair = typed_repositories("getpaseo/paseo");
    let protocols: Vec<Option<CloneProtocol>> = pair.iter().map(|repo| repo.protocol).collect();
    assert_eq!(
        protocols,
        [Some(CloneProtocol::Https), Some(CloneProtocol::Ssh)]
    );
    assert!(pair.iter().all(|repo| repo.clone_url == "getpaseo/paseo"));
    assert!(typed_repositories("paseo").is_empty());
    let urls: Vec<String> = pair.iter().map(clone_remote).collect();
    assert_eq!(
        urls,
        [
            "https://github.com/getpaseo/paseo.git",
            "git@github.com:getpaseo/paseo.git"
        ]
    );
}

#[test]
fn destinations_show_the_clone_and_keep_the_parent() {
    let found = clone_locations(
        &strings(&["~/dev", "~/workspace"]),
        "paseo",
        &strings(&["~/workspace/paseo"]),
    );
    let shown: Vec<(&str, &str, bool)> = found
        .iter()
        .map(|location| {
            (
                location.parent.as_str(),
                location.path.as_str(),
                location.exists,
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            ("~/dev", "~/dev/paseo", false),
            ("~/workspace", "~/workspace/paseo", true)
        ]
    );
    let once = clone_locations(&strings(&["/Users/moboudra/dev", "~/dev"]), "dotfiles", &[]);
    assert_eq!(once.len(), 1);
    assert_eq!(
        once.first().map(|l| l.parent.as_str()),
        Some("/Users/moboudra/dev")
    );
}

#[test]
fn parents_are_the_projects_folders_then_paseos_places() {
    assert_eq!(
        suggested_parents(&strings(&[
            "/home/me/code/shop",
            "/home/me/code/site",
            "/srv/app"
        ])),
        [
            "/home/me/code",
            "/srv",
            "~/dev",
            "~/Developer",
            "~/src",
            "~/projects",
            "~/workspace",
            "~"
        ]
    );
}

#[test]
fn directory_rows_follow_paseos_picker() {
    let recent = strings(&["/home/me/code/shop", "/home/me/notes", "/srv/app"]);
    let mut with_repeats = recent.clone();
    with_repeats.extend(strings(&["/srv/app", " "]));
    assert_eq!(
        suggestions(&with_repeats, &strings(&["/home/me/other"]), ""),
        recent
    );
    assert_eq!(
        suggestions(
            &recent,
            &strings(&["/home/me/code/shop", "/home/me/code/shop-api"]),
            "shop"
        ),
        ["/home/me/code/shop", "/home/me/code/shop-api"]
    );
    assert_eq!(suggestions(&recent, &[], "~/"), recent);
    let option = |typed: bool, path: &str| DirectoryOption {
        typed,
        path: path.to_owned(),
    };
    assert_eq!(
        directory_options(&recent, &[], "/opt/new"),
        [option(true, "/opt/new")]
    );
    assert_eq!(
        directory_options(&recent, &[], "/srv/app"),
        [option(false, "/srv/app")]
    );
    assert_eq!(
        directory_options(&recent, &[], "notes"),
        [option(false, "/home/me/notes")]
    );
    // Separators ignored on the retry: `blank page` finds `blankpage`.
    assert_eq!(
        suggestions(&strings(&["/home/me/blankpage"]), &[], "blank page"),
        ["/home/me/blankpage"]
    );
}

#[test]
fn openable_paths_and_the_home() {
    for path in ["/a", "~", "~/a", "C:\\a", "c:/a", "\\\\server\\s"] {
        assert!(is_openable(path), "{path}");
    }
    assert!(!is_openable("code/shop"));
    assert_eq!(shorten_path("/home/me/code"), "~/code");
    assert_eq!(shorten_path("/Users/me"), "~");
    assert_eq!(shorten_path("/srv/app"), "/srv/app");
}

#[test]
fn remotes_are_read_as_paseo_reads_them() {
    for repo in [
        "https://github.com/owner/repo",
        "http://internal/owner/repo.git",
        "ssh://git@github.com/owner/repo",
        "git@github.com:owner/repo.git",
        "  https://github.com/owner/repo  ",
    ] {
        assert!(git_remote::is_complete(repo), "{repo}");
    }
    for repo in [
        "owner/repo",
        "owner/repo.git",
        "",
        "git://github.com/owner/repo",
        "ftp://host/repo",
        "file:///tmp/repo",
    ] {
        assert!(!git_remote::is_complete(repo), "{repo}");
    }
    let port = |remote: &str| git_remote::location(remote).and_then(|location| location.port);
    assert_eq!(
        port("https://home-git.example.com:60443/team/repo.git").as_deref(),
        Some("60443")
    );
    assert_eq!(
        port("http://internal.example.com:3000/team/repo.git").as_deref(),
        Some("3000")
    );
    assert_eq!(port("https://github.com/acme/repo.git"), None);
    assert_eq!(port("git@host.example.com:team/repo.git"), None);
    assert_eq!(
        git_remote::location("ssh://git@github.com:22/acme/repo.git"),
        git_remote::location("ssh://git@github.com/acme/repo.git")
    );
    assert_eq!(
        git_remote::location("ssh://git@example.com:2222/acme/repo.git"),
        Some(Location {
            transport: Transport::Ssh,
            host: "example.com".to_owned(),
            port: Some("2222".to_owned()),
            path: "acme/repo".to_owned(),
        })
    );
    assert_eq!(
        git_remote::github_repository("git@github.com:getpaseo/paseo.git").as_deref(),
        Some("getpaseo/paseo")
    );
    assert_eq!(
        git_remote::github_repository("https://gitlab.com/a/b.git"),
        None
    );
    assert_eq!(
        git_remote::github_repository("https://github.com/a/b/c"),
        None
    );
}

#[test]
fn rows_are_typed_and_older_machines_get_no_tilde() {
    let mut flow = Flow::open(vec![local()], None);
    flow.open_directory_search("host-1");
    flow.set_field("~/code");
    let shown = HashMap::new();
    let suggested = strings(&["/home/me/code"]);
    let inputs = |tilde: bool| Inputs {
        page: flow.page(),
        machines: &flow.machines,
        machine: flow.machine(),
        recent: &[],
        suggested: &suggested,
        shown: &shown,
        last_clone_parent: None,
        tilde,
        repositories: &[],
    };
    let listed = rows(&inputs(true));
    assert_eq!(
        listed.first().map(|row| &row.action),
        Some(&RowAction::AddPath("~/code".to_owned()))
    );
    assert_eq!(
        listed.first().and_then(|row| row.note.clone()),
        Some(RowNote::OpenThisPath)
    );
    assert_eq!(
        listed.get(1).map(|row| (&row.title, &row.note)),
        Some((
            &RowTitle::Text("~/code".to_owned()),
            &Some(RowNote::Text("/home/me/code".to_owned()))
        ))
    );
    let older = rows(&inputs(false));
    assert_eq!(older.len(), 1);
    assert_eq!(
        older.first().map(|row| &row.action),
        Some(&RowAction::AddPath("/home/me/code".to_owned()))
    );
}
