//! `fleet.published` (protocol 16, ADR 0056) through real nodes and paired
//! devices: a home, a member and a standalone machine each answer with a
//! value to a device paired as `vessel node pair` pairs one (read, operate,
//! admin, never `sync`), which the member protocol still refuses; a
//! read-only device, an agent and a call through the home are refused.

use std::time::Duration;

use vessel_client::NodeClient;
use vessel_node::fleet::DEFAULT_GRANT;
use vessel_proto::{
    ErrorCode, FleetPublished, GetManifest, GetPublished, PublishedBundle, Scope, SharedFileKind,
};

use crate::agents::{as_agent, code, credential, structured};
use crate::fleet_control::device;
use crate::secure::Relay;
use crate::support::{Fixture, within};

/// `client`'s answer once `ready` holds of it (the fleet loop publishes
/// and applies on its own time).
async fn published_until(
    client: &NodeClient,
    ready: impl Fn(&FleetPublished) -> bool,
) -> FleetPublished {
    loop {
        let published = client.fleet_published().await.unwrap();
        if ready(&published) {
            return published;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn instructions_of(bundle: &PublishedBundle) -> bool {
    bundle.files.iter().any(|file| {
        file.path.as_str() == "instructions.md" && file.kind == SharedFileKind::Instructions
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_role_answers_a_device_paired_as_a_person_pairs_one() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        // What `vessel node pair` grants: read, operate, admin; no `sync`.
        let admin = device(&home.owner, &relay, Scope::DEVICE_DEFAULT.to_vec()).await;
        let at_home = published_until(&admin, |published| {
            matches!(published, FleetPublished::Home { bundle: Some(_) })
        })
        .await;
        let FleetPublished::Home {
            bundle: Some(shared),
        } = at_home
        else {
            panic!("a home's bundle: {at_home:?}");
        };
        assert!(instructions_of(&shared), "{shared:?}");
        // The member protocol stays a member's: the page's earlier read.
        assert_eq!(code(admin.call(GetManifest).await), ErrorCode::Forbidden);

        // A member: what it received, and from where.
        let member = crate::across::member_granting(
            &relay,
            &home,
            "published-member",
            vec![Scope::Sync],
            DEFAULT_GRANT.to_vec(),
        )
        .await;
        let received = published_until(&member.owner, |published| {
            matches!(
                published,
                FleetPublished::Member {
                    bundle: Some(_),
                    applied: Some(_),
                    ..
                }
            )
        })
        .await;
        let FleetPublished::Member {
            home: from,
            bundle: Some(bundle),
            applied: Some(applied),
        } = received
        else {
            panic!("a member's bundle: {received:?}");
        };
        assert_eq!(from, home.id());
        assert_eq!(bundle.generation, applied);
        assert!(instructions_of(&bundle), "{bundle:?}");

        // A machine in no fleet shares nothing, as a value.
        let fixture = Fixture::new();
        let alone = fixture.start().await;
        let owner = fixture.connect(&alone).await;
        assert_eq!(
            owner.fleet_published().await.unwrap(),
            FleetPublished::Standalone
        );
        alone.stop().await.unwrap();
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_read_only_device_an_agent_and_a_call_through_the_home_are_refused() {
    within(async {
        let relay = Relay::start().await;
        let sources = tempfile::tempdir().unwrap();
        let home = crate::across::home(&relay, sources.path()).await;
        let reader = device(&home.owner, &relay, vec![Scope::Read]).await;
        assert_eq!(code(reader.fleet_published().await), ErrorCode::Forbidden);

        let mut session = structured(&home.owner, &home.fixture).await;
        let (_, socket, auth) = credential(&home.owner, &mut session).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(code(agent.fleet_published().await), ErrorCode::Forbidden);
        home.owner.kill(session.id).await.unwrap();

        // Not forwardable: even a member that granted its home everything
        // is read over its own connection only.
        let member = crate::across::member_granting(
            &relay,
            &home,
            "published-delegated",
            vec![Scope::Sync],
            Scope::ALL.to_vec(),
        )
        .await;
        assert_eq!(
            code(home.owner.call_on(member.id(), GetPublished {}).await),
            ErrorCode::InvalidRequest
        );
        member.node.stop().await.unwrap();
        home.node.stop().await.unwrap();
        relay.stop().await;
    })
    .await;
}
