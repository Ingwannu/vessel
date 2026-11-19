//! Seeded repositories are what `forge.repos` lists and searches (ADR
//! 0088), through the real fake's trait call.
use super::{Listing, Repository, decode, document, repo};
use crate::Forge as _;
use crate::fake::Failure;
use vessel_proto::forge::{RepoLimit, Visibility};

fn entry(path: &str, description: Option<&str>, day: u32) -> Repository {
    let mut entry = document().repositories.remove(0);
    entry.repo.path = path.parse().unwrap();
    entry.pulls.clear();
    entry.listing = Listing {
        description: description.map(|text| text.parse().unwrap()),
        visibility: Some(Visibility::Private),
        updated_at: Some(format!("2026-11-{day:02}T00:00:00Z").parse().unwrap()),
    };
    entry
}

#[tokio::test]
async fn seeded_repositories_list_newest_first_and_search_by_path_or_description() {
    let mut raw = document();
    raw.repositories = vec![
        entry("fixture/older", Some("A Vessel fixture"), 1),
        entry("fixture/newer", None, 9),
        entry("fixture/vessel-app", None, 5),
    ];
    let forge = decode(&raw).into_forge();
    let host = repo().host;
    let all = forge
        .repositories(&host, "", RepoLimit::DEFAULT)
        .await
        .unwrap();
    let paths: Vec<_> = all.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(
        paths,
        ["fixture/newer", "fixture/vessel-app", "fixture/older"]
    );
    assert_eq!(all[0].visibility, Visibility::Private);
    assert_eq!(
        all[0].clone_url.as_str(),
        "https://forge.example.invalid/fixture/newer"
    );
    let found = forge
        .repositories(&host, "  VESSEL ", RepoLimit::DEFAULT)
        .await
        .unwrap();
    let paths: Vec<_> = found.iter().map(|row| row.path.as_str()).collect();
    assert_eq!(paths, ["fixture/vessel-app", "fixture/older"]);
    let one = forge
        .repositories(&host, "", RepoLimit::new(1).unwrap())
        .await
        .unwrap();
    assert_eq!(one.len(), 1);
    forge.fail(Some(Failure::SignedOut));
    assert!(forge.repositories(&host, "", RepoLimit::DEFAULT).await.is_err());
}
