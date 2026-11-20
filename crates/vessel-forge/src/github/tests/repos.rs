//! Repository search (ADR 0088) against recorded GitHub REST answers on
//! loopback: the account's own repositories, a search in the machine's
//! clone protocol, rows that cannot be wire values, and a spent search
//! quota that holds only search.

use axum::http::StatusCode;
use vessel_proto::forge::{RepoDescription, RepoLimit, Visibility};

use super::stand::{TOKEN, set, stand, stand_with};
use crate::gh::Gh;
use crate::provider::{Forge as _, ForgeError};

/// `GET /user/repos`, as GitHub answers it (fields trimmed to a few).
const OWN: &str = r#"[
{"id":1,"name":"vessel","full_name":"owner/vessel","private":true,"visibility":"private",
"description":"Agents\non every machine","html_url":"https://github.com/owner/vessel",
"ssh_url":"git@github.com:owner/vessel.git","updated_at":"2026-11-18T10:00:00Z",
"pushed_at":"2026-11-19T09:00:00Z"},
{"id":2,"name":"bad name","full_name":"owner/bad name","private":false,
"html_url":"https://github.com/owner/bad%20name","ssh_url":"git@github.com:owner/bad name.git",
"updated_at":"2026-11-17T10:00:00Z"},
{"id":3,"name":"tools","full_name":"owner/tools","private":true,"description":null,
"html_url":"https://github.com/owner/tools","ssh_url":"git@github.com:owner/tools.git",
"updated_at":"2026-11-16T10:00:00Z"}
]"#;

/// `GET /search/repositories`, as GitHub answers it.
const FOUND: &str = r#"{"total_count":2,"incomplete_results":false,"items":[
{"id":7,"full_name":"org/vessel-app","visibility":"internal","description":"The app",
"html_url":"https://github.com/org/vessel-app","ssh_url":"git@github.com:org/vessel-app.git",
"updated_at":"2026-11-19T01:00:00Z"},
{"id":8,"full_name":"someone/vessel","visibility":"public","description":"",
"html_url":"https://github.com/someone/vessel","ssh_url":"git@github.com:someone/vessel.git",
"updated_at":"2026-11-10T01:00:00Z"}]}"#;

const OWN_KEY: &str = "GET /user/repos?affiliation=owner&sort=pushed&direction=desc&per_page=30";
const SEARCH_KEY: &str =
    "GET /search/repositories?q=vessel+app&sort=updated&order=desc&per_page=30";

fn limit() -> RepoLimit {
    RepoLimit::new(30).unwrap()
}

#[tokio::test]
async fn the_account_s_own_repositories_list_in_https_without_bad_rows() {
    let (shared, github, repo) = stand().await;
    set(&shared, OWN_KEY, StatusCode::OK, &[], OWN);
    let rows = github
        .repositories(&repo.host, "  ", limit())
        .await
        .unwrap();
    let paths: Vec<_> = rows.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        ["owner/vessel", "owner/tools"],
        "a bad name is left out"
    );
    assert_eq!(rows[0].visibility, Visibility::Private);
    assert_eq!(
        rows[0].description.as_ref().map(RepoDescription::as_str),
        Some("Agents on every machine")
    );
    assert_eq!(
        rows[0].clone_url.as_str(),
        "https://github.com/owner/vessel"
    );
    assert_eq!(
        rows[1].visibility,
        Visibility::Private,
        "no visibility: private's"
    );
    assert!(rows[1].description.is_none());
    let asked = &shared.lock().unwrap().asked;
    assert_eq!(asked.len(), 1);
    assert_eq!(
        asked[0].1.as_deref(),
        Some(format!("Bearer {TOKEN}").as_str())
    );
}

#[tokio::test]
async fn a_search_clones_in_ssh_when_gh_says_so_and_keeps_the_limit() {
    let folder = tempfile::tempdir().unwrap();
    let program = folder.path().join("gh");
    vessel_testkit::write_executable(
        &program,
        "#!/bin/sh\n[ \"$1 $2 $3\" = \"config get git_protocol\" ] && echo ssh\n",
    )
    .unwrap();
    let (shared, github, repo) =
        stand_with(Some(Gh::new(program, folder.path().to_path_buf()))).await;
    set(&shared, SEARCH_KEY, StatusCode::OK, &[], FOUND);
    let rows = github
        .repositories(&repo.host, " vessel app ", limit())
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].clone_url.as_str(),
        "git@github.com:org/vessel-app.git"
    );
    assert_eq!(rows[0].visibility, Visibility::Internal);
    assert!(
        rows[1].description.is_none(),
        "an empty description is none"
    );
    let one = {
        set(
            &shared,
            "GET /search/repositories?q=vessel+app&sort=updated&order=desc&per_page=1",
            StatusCode::OK,
            &[],
            FOUND,
        );
        github
            .repositories(&repo.host, "vessel app", RepoLimit::new(1).unwrap())
            .await
            .unwrap()
    };
    assert_eq!(one.len(), 1, "never more than asked, whatever GitHub sends");
}

#[tokio::test]
async fn a_spent_search_quota_holds_search_but_not_the_rest() {
    let (shared, github, repo) = stand().await;
    set(
        &shared,
        SEARCH_KEY,
        StatusCode::FORBIDDEN,
        &[
            ("x-ratelimit-resource", "search"),
            ("x-ratelimit-limit", "30"),
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", "4102444800"),
        ],
        r#"{"message":"API rate limit exceeded"}"#,
    );
    set(&shared, OWN_KEY, StatusCode::OK, &[], OWN);
    let spent = github.repositories(&repo.host, "vessel app", limit()).await;
    assert!(matches!(spent, Err(ForgeError::RateLimited { .. })));
    let again = github.repositories(&repo.host, "vessel app", limit()).await;
    assert!(matches!(again, Err(ForgeError::RateLimited { .. })));
    assert!(
        github.repositories(&repo.host, "", limit()).await.is_ok(),
        "the account's list is not a search"
    );
    let searches = shared
        .lock()
        .unwrap()
        .asked
        .iter()
        .filter(|(key, ..)| key.starts_with("GET /search/"))
        .count();
    assert_eq!(searches, 1, "a held search is not sent");
}

#[tokio::test]
async fn a_refusal_or_an_unreadable_answer_is_an_error_not_an_empty_list() {
    let (shared, github, repo) = stand().await;
    set(&shared, OWN_KEY, StatusCode::NOT_FOUND, &[], "{}");
    assert!(matches!(
        github.repositories(&repo.host, "", limit()).await,
        Err(ForgeError::Refused { .. })
    ));
    set(&shared, OWN_KEY, StatusCode::OK, &[], r#"{"not":"a list"}"#);
    assert!(matches!(
        github.repositories(&repo.host, "", limit()).await,
        Err(ForgeError::Unexpected { .. })
    ));
}
