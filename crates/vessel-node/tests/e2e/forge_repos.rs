//! GitHub repository search end to end (ADR 0088, `forge.repos`): the
//! person's list and search come from the node's forge, bounded by the
//! asked limit; a signed-out forge answers why, with no rows; a device
//! that may only read is refused (the answer holds private repositories).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use vessel_forge::fake::{Failure, FakeForge};
use vessel_proto::forge::{
    ForgeAccess, GhState, RepoLimit, Repository, SearchRepositories, Visibility,
};

use vessel_proto::{ErrorCode, Scope};

use crate::agents::code;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reader_is_refused_and_an_operator_lists() {
    within(async {
        let forge = FakeForge::new();
        forge.list_repository(listed("owner/private", Some("Not for readers"), 1_000));
        let fixture = Fixture::new();
        let forge_for = Arc::clone(&forge);
        let node = fixture
            .start_with(|config| {
                config.forge = vessel_node::ForgeSetup {
                    custom: Some(forge_for),
                    ..vessel_node::ForgeSetup::default()
                };
                crate::secure::advertise_placeholder_route(config);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let reader = crate::side_questions::access::device(&owner, &node, vec![Scope::Read]).await;
        assert_eq!(
            code(reader.call(SearchRepositories::default()).await),
            ErrorCode::Forbidden
        );
        let operator =
            crate::side_questions::access::device(&owner, &node, vec![Scope::Read, Scope::Operate])
                .await;
        let listed = operator.call(SearchRepositories::default()).await.unwrap();
        assert_eq!(listed.repositories.len(), 1);
        node.stop().await.unwrap();
    })
    .await;
}
