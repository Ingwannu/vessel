//! Real probe lifecycle against testkit's ACP process, in a synthetic account.

use std::time::Duration;
use vessel_agent::{AgentSetup, Driver, probe_provider_with};
use vessel_harness::providers::ProviderRegistry;
use vessel_proc::CommandSpec;
use vessel_proto::{ResolvedMcpServers, providers::ProviderOverrides};

const SAFETY_NET: Duration = Duration::from_secs(30);
const HANDSHAKE: Duration = Duration::from_secs(10);

#[tokio::test]
async fn native_codex_catalogue_never_creates_a_thread_or_turn() {
    vessel_testkit::within(SAFETY_NET, async {
        let root = tempfile::tempdir().unwrap();
        let audit = root.path().join("requests");
        let command = CommandSpec::new(vessel_testkit::Fake::CodexAppServer.path().unwrap())
            .arg("app-server")
            .env_clear()
            .env(
                if cfg!(windows) { "USERPROFILE" } else { "HOME" },
                root.path(),
            )
            .env("FAKE_CODEX_CATALOGUE_AUDIT", &audit);
        let registry = ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap();
        let definition = registry.get(&"codex".parse().unwrap()).unwrap();
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
        let observed = probe_provider_with(Driver::Codex, &command, setup.clone(), definition)
            .await
            .unwrap();
        assert_eq!(observed.catalogue.models.all().len(), 2);
        assert_eq!(
            observed.catalogue.models.all()[1].spec().default,
            Some(vessel_proto::providers::DefaultChoice::Yes)
        );
        assert_eq!(
            observed.catalogue.models.all()[0]
                .spec()
                .default_thinking
                .as_ref()
                .unwrap()
                .as_str(),
            "high"
        );
        assert_eq!(
            observed.catalogue.models.all()[1]
                .spec()
                .default_thinking
                .as_ref()
                .unwrap()
                .as_str(),
            "low"
        );
        assert!(observed.version.is_some());
        assert_eq!(
            std::fs::read_to_string(&audit).unwrap(),
            "initialize\nmodel/list\nmodel/list\ngetUserSavedConfig\nconfig/read\n"
        );
        let malformed = command.env("FAKE_CODEX_CATALOGUE", "malformed");
        assert!(
            probe_provider_with(Driver::Codex, &malformed, setup, definition)
                .await
                .is_err()
        );
    })
    .await
    .unwrap();
}

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
                .is_some_and(|choices| !choices.all().is_empty())
        );
    })
    .await
    .unwrap();
}
