//! Protocol 12's node settings (track A2, ADR 0038): `node.settings` reads
//! how long an idle agent runs before it is parked, `node.set_settings`
//! changes it; the change is kept across a restart and wins over the flag,
//! a choice the node does not know is refused, and an agent (no `admin`)
//! may read but not change. Protocol 18's drivers (ADR 0057): every agent
//! over ACP until a person chooses, `direct` refused while this build
//! offers no agent's own protocol, and a newer build's saved choice kept
//! in the file but run as ACP.

use std::time::Duration;

use vessel_proto::ErrorCode;
use vessel_proto::parking::{IdleMinutes, ParkAfter};
use vessel_proto::settings::{
    AgentDriver, AgentDrivers, GetNodeSettings, SetAgentDrivers, SetNodeSettings,
};

use crate::agents::{as_agent, code, credential, structured, vessel_mcp};
use crate::support::{Fixture, within};

#[cfg(test)]
mod aside;

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
            owner
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            idle(30),
            "nothing saved: the flag's value"
        );

        let changed = owner
            .call(SetNodeSettings {
                park_after: Some(ParkAfter::Never),
                drivers: None,
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(changed.park_after, ParkAfter::Never);
        assert_eq!(
            owner
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            ParkAfter::Never
        );
        assert_eq!(
            code(
                owner
                    .call(SetNodeSettings {
                        park_after: Some(ParkAfter::Unknown),
                        drivers: None,
                        ..SetNodeSettings::default()
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
            owner
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            ParkAfter::Never
        );
        let changed = owner
            .call(SetNodeSettings {
                park_after: Some(idle(5)),
                drivers: None,
                ..SetNodeSettings::default()
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
            agent
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            ParkAfter::DEFAULT
        );
        assert_eq!(
            code(
                agent
                    .call(SetNodeSettings {
                        park_after: Some(ParkAfter::Never),
                        drivers: None,
                        ..SetNodeSettings::default()
                    })
                    .await
            ),
            ErrorCode::Forbidden
        );
        assert_eq!(
            owner
                .call(GetNodeSettings::default())
                .await
                .unwrap()
                .park_after,
            ParkAfter::DEFAULT,
            "nothing changed"
        );
        node.stop().await.unwrap();
    })
    .await;
}

/// A change naming `drivers` only.
fn drivers(drivers: SetAgentDrivers) -> SetNodeSettings {
    SetNodeSettings {
        park_after: None,
        drivers: Some(drivers),
        ..SetNodeSettings::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drivers_are_acp_until_an_agents_own_protocol_is_offered() {
    within(async {
        let fixture = Fixture::new();
        // As a newer build would have saved them: Claude Code over its own
        // protocol, and a choice this build does not know.
        std::fs::write(
            fixture.data_dir().settings_path(),
            r#"{"drivers":{"claude_code":"direct","open_code":"later"}}"#,
        )
        .unwrap();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let in_force = owner.call(GetNodeSettings::default()).await.unwrap();
        assert_eq!(in_force.drivers, AgentDrivers::default(), "run as ACP");

        // An agent whose own protocol this build does not offer yet:
        // refused, and nothing changes.
        let direct = SetAgentDrivers {
            open_code: Some(AgentDriver::Direct),
            ..SetAgentDrivers::default()
        };
        assert_eq!(
            code(owner.call(drivers(direct)).await),
            ErrorCode::Unsupported
        );
        // Codex's is offered (its permissions slice has landed).
        let codex = SetAgentDrivers {
            codex: Some(AgentDriver::Direct),
            ..SetAgentDrivers::default()
        };
        let changed = owner.call(drivers(codex)).await.unwrap();
        assert_eq!(changed.drivers.codex, AgentDriver::Direct);
        let unknown = SetAgentDrivers {
            codex: Some(AgentDriver::Unknown),
            ..SetAgentDrivers::default()
        };
        assert_eq!(
            code(owner.call(drivers(unknown)).await),
            ErrorCode::InvalidRequest
        );

        // ACP is accepted; the newer build's choices stay in the file.
        let acp = SetAgentDrivers {
            codex: Some(AgentDriver::Acp),
            ..SetAgentDrivers::default()
        };
        let changed = owner.call(drivers(acp)).await.unwrap();
        assert_eq!(changed.drivers, AgentDrivers::default());
        let kept: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.data_dir().settings_path()).unwrap())
                .unwrap();
        assert_eq!(
            kept["drivers"],
            serde_json::json!({"claude_code": "direct", "codex": "acp", "open_code": "unknown"})
        );
        node.stop().await.unwrap();
    })
    .await;
}
