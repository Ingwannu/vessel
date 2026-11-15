//! Real probe lifecycle against testkit's ACP process, in a synthetic account.

use std::time::Duration;
use vessel_agent::{AgentSetup, Driver, probe_provider_with};
use vessel_harness::providers::ProviderRegistry;
use vessel_proc::CommandSpec;
use vessel_proto::{ResolvedMcpServers, providers::ProviderOverrides};

const SAFETY_NET: Duration = Duration::from_secs(30);
const HANDSHAKE: Duration = Duration::from_secs(10);

#[tokio::test]
async fn one_prompt_free_fake_probe_yields_models_modes_effort_and_version() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let command = CommandSpec::new(vessel_testkit::Fake::Acp.path().unwrap())
            .env_clear()
            .env(
                if cfg!(windows) { "USERPROFILE" } else { "HOME" },
                root.path(),
            );
        let registry = ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap();
        let definition = registry.get(&"claude".parse().unwrap()).unwrap();
        let setup = AgentSetup {
            cwd: root.path().to_path_buf(),
            model: None,
            resume: None,
            fork: None,
            options: Vec::new(),
            mcp_servers: ResolvedMcpServers::new(),
            handshake_timeout: HANDSHAKE,
            questions: false,
            private_dir: None,
        };
        let observed = probe_provider_with(Driver::Acp, &command, setup, definition)
            .await
            .unwrap();
        assert!(!observed.catalogue.models.all().is_empty());
        assert!(!observed.catalogue.modes.modes().is_empty());
        assert!(observed.version.is_some());
        let default_model = observed
            .catalogue
            .models
            .all()
            .iter()
            .find(|m| m.spec().default == Some(vessel_proto::providers::DefaultChoice::Yes))
            .unwrap();
        assert!(
            default_model
                .spec()
                .thinking
                .as_ref()
                .is_some_and(|choices| !choices.is_empty())
        );
    })
    .await
    .unwrap();
}
