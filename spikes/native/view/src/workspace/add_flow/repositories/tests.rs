//! The GitHub page's search rows, from recorded answers.

use vessel_proto::forge::{ForgeAccess, GhState, Repository as Found, RepositoryList, Visibility};

use super::*;
use crate::workspace::add_flow::model::CloneProtocol;

fn row(path: &str, description: Option<&str>) -> Found {
    Found {
        path: path.parse().unwrap(),
        description: description.map(|text| text.parse().unwrap()),
        visibility: Visibility::Public,
        updated_at: chrono::DateTime::UNIX_EPOCH,
        clone_url: format!("https://github.com/{path}").parse().unwrap(),
    }
}

fn host() -> vessel_proto::forge::ForgeHost {
    "github.com".parse().unwrap()
}

#[test]
fn only_a_node_with_forge_repos_is_searched() {
    let needs = REPOSITORY_SEARCH_PROTOCOL.unwrap();
    assert!(can_search(Some(needs)));
    assert!(!can_search(Some(ProtocolVersion::new(needs.get() - 1))));
    assert!(!can_search(None), "a machine not read yet");
}

#[test]
fn a_found_row_keeps_the_node_s_clone_url_and_its_description() {
    let chosen = found(&row("acme/widgets", Some("The widgets")));
    assert_eq!(chosen.id, "repository:acme/widgets");
    assert_eq!(chosen.name_with_owner, "acme/widgets");
    assert_eq!(chosen.clone_url, "https://github.com/acme/widgets");
    assert_eq!(chosen.protocol, None, "no transport word on a found row");
    assert_eq!(
        chosen.origin,
        Origin::Found {
            description: Some("The widgets".to_owned())
        }
    );
}

#[test]
fn the_node_s_access_is_said_in_paseo_s_words() {
    let signed_out = |gh| ForgeAccess::SignedOut { host: host(), gh };
    assert_eq!(access_error(&ForgeAccess::Ready { login: None }), None);
    assert_eq!(
        access_error(&signed_out(GhState::Missing)),
        Some(SearchError::GhMissing)
    );
    assert_eq!(
        access_error(&signed_out(GhState::SignedOut)),
        Some(SearchError::SignedOut)
    );
    assert_eq!(
        access_error(&signed_out(GhState::Failed)),
        Some(SearchError::Unavailable)
    );
    assert_eq!(
        access_error(&ForgeAccess::Unknown),
        Some(SearchError::Unavailable)
    );
    assert_eq!(
        searched(None),
        Searched {
            repositories: Vec::new(),
            loading: false,
            error: Some(SearchError::Failed),
        },
        "a call that failed"
    );
    let list = RepositoryList {
        access: signed_out(GhState::Missing),
        repositories: Vec::new(),
    };
    assert_eq!(searched(Some(&list)).error, Some(SearchError::GhMissing));
}

#[test]
fn typed_rows_come_first_unless_the_search_has_that_repository() {
    let found_rows = vec![found(&row("acme/widgets", None))];
    let typed_first = github_choices("acme/gadgets", &found_rows);
    let protocols: Vec<Option<CloneProtocol>> =
        typed_first.iter().map(|choice| choice.protocol).collect();
    assert_eq!(
        protocols,
        [Some(CloneProtocol::Https), Some(CloneProtocol::Ssh), None]
    );
    let exact = github_choices(" ACME/Widgets ", &found_rows);
    assert_eq!(exact, found_rows, "the search already has it");
    assert_eq!(github_choices("", &found_rows), found_rows);
}
