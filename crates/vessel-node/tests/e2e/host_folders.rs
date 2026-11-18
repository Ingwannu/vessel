//! ADR 0112 over the wire: Add project's folder suggestions from the node
//! user's home (Operate) and a project's own icon (Read), answered by a
//! real node; a reader gets icons but no folders, an agent neither.

use vessel_client::NodeClient;
use vessel_proto::workspace::host_folders::{
    GetProjectIcon, HOST_FOLDERS_PROTOCOL, IconType, SuggestFolders,
};
use vessel_proto::workspace::suggestions::SuggestionQuery;
use vessel_proto::{ErrorCode, ProjectId, Scope};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

fn folders(query: &str) -> SuggestFolders {
    SuggestFolders {
        query: SuggestionQuery::try_from(query.to_owned()).unwrap(),
        limit: None,
    }
}

const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"/>"#;

/// The owner's project with Paseo's favicon under `public`.
async fn project_with_icon(owner: &NodeClient) -> (tempfile::TempDir, ProjectId) {
    let (folder, project) = crate::tidying::project(owner).await;
    std::fs::create_dir(project.root.join("public")).unwrap();
    std::fs::write(project.root.join("public/favicon.svg"), SVG).unwrap();
    (folder, project.id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_gets_the_homes_folders_and_a_projects_icon() {
    within(async {
        let fixture = Fixture::new();
        let home = std::fs::canonicalize(fixture.user_home()).unwrap();
        for folder in ["code/shop", "code/.secret-shop", "notes"] {
            std::fs::create_dir_all(home.join(folder)).unwrap();
        }
        std::fs::write(home.join("code/shop-notes.txt"), "x").unwrap();
        let node = fixture
            .start_with(|config| config.harness_lookup = crate::workspace::lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        assert!(Some(owner.protocol()) >= HOST_FOLDERS_PROTOCOL);

        let found = owner.suggest_host_folders(folders("shop")).await.unwrap();
        let shown: Vec<&str> = found
            .folders
            .iter()
            .map(|folder| folder.shown.as_str())
            .collect();
        assert_eq!(shown, ["~/code/shop"], "{found:?}");
        assert_eq!(
            found.folders.first().map(|folder| folder.path.clone()),
            Some(home.join("code/shop"))
        );
        let listed = owner.suggest_host_folders(folders("~")).await.unwrap();
        let shown: Vec<&str> = listed
            .folders
            .iter()
            .map(|folder| folder.shown.as_str())
            .collect();
        assert!(
            shown.contains(&"~/code") && shown.contains(&"~/notes"),
            "{listed:?}"
        );
        assert!(
            owner
                .suggest_host_folders(folders("/"))
                .await
                .unwrap()
                .folders
                .is_empty()
        );

        let (_folder, project) = project_with_icon(&owner).await;
        let icon = owner
            .project_icon(GetProjectIcon { project })
            .await
            .unwrap()
            .icon
            .unwrap();
        assert_eq!(icon.kind, IconType::Svg);
        assert_eq!(icon.bytes.bytes().as_ref(), SVG);
        let (_bare, bare) = crate::tidying::project(&owner).await;
        assert_eq!(
            owner
                .project_icon(GetProjectIcon { project: bare.id })
                .await
                .unwrap()
                .icon,
            None
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reader_gets_icons_but_not_the_homes_folders() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.harness_lookup = crate::workspace::lookup(&fixture);
                crate::secure::advertise_placeholder_route(config);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project_with_icon(&owner).await;
        let reader = crate::side_questions::access::device(&owner, &node, vec![Scope::Read]).await;
        let icon = reader.call(GetProjectIcon { project }).await.unwrap();
        assert_eq!(icon.icon.map(|icon| icon.kind), Some(IconType::Svg));
        assert_eq!(
            code(reader.call(folders("code")).await),
            ErrorCode::Forbidden
        );
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_gets_neither() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.vessel_mcp = Some(vessel_mcp(&fixture));
                config.harness_lookup = crate::workspace::lookup(&fixture);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let (_folder, project) = project_with_icon(&owner).await;
        let mut running = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        let refusals = [
            code(agent.call(folders("code")).await),
            code(agent.call(GetProjectIcon { project }).await),
        ];
        assert_eq!(refusals, [ErrorCode::Forbidden; 2]);
        node.stop().await.unwrap();
    })
    .await;
}
