//! Protocol 12's node settings (track A2, ADR 0038): `node.settings` reads
//! how long an idle agent runs before it is parked, `node.set_settings`
//! changes it; the change is kept across a restart and wins over the flag,
//! a choice the node does not know is refused, and an agent (no `admin`)
//! may read but not change.

use std::time::Duration;

use vessel_proto::ErrorCode;
use vessel_proto::parking::{IdleMinutes, ParkAfter};
use vessel_proto::settings::{GetNodeSettings, SetNodeSettings};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

fn idle(minutes: u16) -> ParkAfter {
    ParkAfter::Idle {
        minutes: IdleMinutes::clamped(minutes),
    }
}

/// `--park-after 30`, as an operator's service would pass it.
fn flag(config: &mut vessel_node::Config) {
    config.park_after = Some(Duration::from_secs(30 * 60));
    config.park_after_given = true;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_change_is_kept_across_a_restart_and_wins_over_the_flag() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start_with(flag).await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner.call(GetNodeSettings {}).await.unwrap().park_after,
            idle(30),
            "nothing saved: the flag's value"
        );

        let changed = owner
            .call(SetNodeSettings {
                park_after: Some(ParkAfter::Never),
            })
            .await
            .unwrap();
        assert_eq!(changed.park_after, ParkAfter::Never);
        assert_eq!(
            owner.call(GetNodeSettings {}).await.unwrap().park_after,
            ParkAfter::Never
        );
        assert_eq!(
            code(
                owner
                    .call(SetNodeSettings {
                        park_after: Some(ParkAfter::Unknown),
                    })
                    .await
            ),
            ErrorCode::InvalidRequest,
            "a choice the node does not know"
        );
        // A change naming nothing answers what is in force.
        assert_eq!(
            owner
                .call(SetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            ParkAfter::Never
        );
        node.stop().await.unwrap();

        // Started again with the same flag: the saved value wins.
        let node = fixture.start_with(flag).await;
        let owner = fixture.connect(&node).await;
        assert_eq!(
            owner.call(GetNodeSettings {}).await.unwrap().park_after,
            ParkAfter::Never
        );
        let changed = owner
            .call(SetNodeSettings {
                park_after: Some(idle(5)),
            })
            .await
            .unwrap();
        assert_eq!(changed.park_after, idle(5));
        node.stop().await.unwrap();
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_agent_reads_the_settings_but_may_not_change_them() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture
            .start_with(|config| config.vessel_mcp = Some(vessel_mcp(&fixture)))
            .await;
        let owner = fixture.connect(&node).await;
        let mut parent = structured(&owner, &fixture).await;
        let (_program, socket, auth) = credential(&owner, &mut parent).await;
        let agent = as_agent(&socket, &auth).await;
        assert_eq!(
            agent.call(GetNodeSettings {}).await.unwrap().park_after,
            ParkAfter::DEFAULT
        );
        assert_eq!(
            code(
                agent
                    .call(SetNodeSettings {
                        park_after: Some(ParkAfter::Never),
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert_eq!(
            owner.call(GetNodeSettings {}).await.unwrap().park_after,
            ParkAfter::DEFAULT,
            "nothing changed"
        );
        node.stop().await.unwrap();
    })
    .await;
}
