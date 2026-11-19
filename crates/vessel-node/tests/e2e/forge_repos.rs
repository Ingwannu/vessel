//! GitHub repository search end to end (ADR 0088, `forge.repos`): the
//! person's list and search come from the node's forge, bounded by the
//! asked limit; a signed-out forge answers why, with no rows.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use vessel_forge::fake::{Failure, FakeForge};
use vessel_proto::forge::{
    ForgeAccess, GhState, RepoLimit, Repository, SearchRepositories, Visibility,
};

use crate::support::{Fixture, within};

fn listed(path: &str, description: Option<&str>, seconds: i64) -> Repository {
    Repository {
        path: path.parse().unwrap(),
        description: description.map(|text| text.parse().unwrap()),
        visibility: Visibility::Public,
        updated_at: DateTime::<Utc>::from_timestamp(seconds, 0).unwrap(),
        clone_url: format!("https://github.com/{path}").parse().unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_lists_and_searches_the_account_s_repositories() {
    within(async {
        let forge = FakeForge::new();
        forge.list_repository(listed("owner/older", Some("Vessel docs"), 1_000));
        forge.list_repository(listed("owner/vessel", None, 3_000));
        forge.list_repository(listed("owner/other", None, 2_000));
        let fixture = Fixture::new();
        let forge_for = Arc::clone(&forge);
        let node = fixture
            .start_with(|config| {
                config.forge = vessel_node::ForgeSetup {
                    custom: Some(forge_for),
                    ..vessel_node::ForgeSetup::default()
                };
            })
            .await;
        let owner = fixture.connect(&node).await;
        let all = owner.call(SearchRepositories::default()).await.unwrap();
        assert!(matches!(all.access, ForgeAccess::Ready { .. }), "{all:?}");
        let paths: Vec<_> = all
            .repositories
            .iter()
            .map(|row| row.path.as_str())
            .collect();
        assert_eq!(paths, ["owner/vessel", "owner/other", "owner/older"]);
        let found = owner
            .call(SearchRepositories {
                query: "vessel".parse().unwrap(),
                limit: RepoLimit::new(1).unwrap(),
            })
            .await
            .unwrap();
        let paths: Vec<_> = found
            .repositories
            .iter()
            .map(|row| row.path.as_str())
            .collect();
        assert_eq!(
            paths,
            ["owner/vessel"],
            "the newest match, as many as asked"
        );
        forge.fail(Some(Failure::SignedOut));
        let out = owner.call(SearchRepositories::default()).await.unwrap();
        assert!(out.repositories.is_empty());
        assert!(
            matches!(
                out.access,
                ForgeAccess::SignedOut {
                    gh: GhState::SignedOut | GhState::Missing,
                    ..
                }
            ),
            "{:?}",
            out.access
        );
    })
    .await;
}
