//! ADR 0113 over the wire: Add project's New directory answered by a real
//! node (a folder under the home, written `~`, made a project without
//! Git); an agent is refused, as for every project write.

use vessel_proto::ErrorCode;
use vessel_proto::registry::{CreateProjectDirectory, DirectoryName, PROJECT_DIRECTORY_PROTOCOL};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

fn new_directory(parent: &str, name: &str) -> CreateProjectDirectory {
    CreateProjectDirectory {
        parent: parent.into(),
        name: DirectoryName::parse(name).unwrap(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_person_makes_a_new_directory_under_the_home() {
    within(async {
        let fixture = Fixture::new();
        let home = std::fs::canonicalize(fixture.user_home()).unwrap();
        std::fs::create_dir_all(home.join("dev")).unwrap();
        let node = fixture
            .start_with(|config| config.harness_lookup = crate::workspace::lookup(&fixture))
            .await;
        let owner = fixture.connect(&node).await;
        assert!(Some(owner.protocol()) >= PROJECT_DIRECTORY_PROTOCOL);

        let made = owner.call(new_directory("~/dev", "notes")).await.unwrap();
        assert_eq!(made.root, home.join("dev").join("notes"));
        assert!(!made.git);
        assert!(home.join("dev/notes").is_dir());
        assert!(!home.join("dev/notes/.git").exists());
        let exists = owner.call(new_directory("~/dev", "notes")).await;
        assert_eq!(code(exists), ErrorCode::Conflict);
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_cannot_make_a_new_directory() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| {
                config.vessel_mcp = Some(vessel_mcp(&fixture));
                config.harness_lookup = crate::workspace::lookup(&fixture);
            })
            .await;
        let owner = fixture.connect(&node).await;
        let mut running = structured(&owner, &fixture).await;
        let (_, socket, auth) = credential(&owner, &mut running).await;
        let agent = as_agent(&socket, &auth).await;
        let refused = agent.call(new_directory("~", "agent-made")).await;
        assert_eq!(code(refused), ErrorCode::Forbidden);
        let home = std::fs::canonicalize(fixture.user_home()).unwrap();
        assert!(!home.join("agent-made").exists());
        node.stop().await.unwrap();
    })
    .await;
}
