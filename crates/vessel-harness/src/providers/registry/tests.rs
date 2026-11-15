//! Registry generations are checked without constructing a provider client.

use super::*;
use vessel_proto::{
    ConfigValue,
    providers::{Arguments, CommandEdit, ProviderEnvironment},
};

fn config(rows: &[(&str, ProviderOverride)]) -> ProviderOverrides {
    ProviderOverrides::new(
        rows.iter()
            .map(|(id, config)| (id.parse().unwrap(), config.clone()))
            .collect(),
    )
    .unwrap()
}

fn plugin(id: &str) -> PluginDefinition {
    PluginDefinition {
        id: id.parse().unwrap(),
        label: "Local plugin".parse().unwrap(),
        configuration: ConfigurationRequirement::Optional,
        revision: PluginRevision::new(1),
        launch: LaunchSettings::default(),
    }
}

#[test]
fn a_generation_preserves_manifest_order_and_disabled_omp_does_not_mean_missing() {
    let registry = ProviderRegistry::build(&ProviderOverrides::default(), &[]).unwrap();
    assert_eq!(
        registry
            .definitions()
            .iter()
            .map(|d| d.id.as_str())
            .collect::<Vec<_>>(),
        ["claude", "codex", "copilot", "opencode", "pi", "omp"]
    );
    assert_eq!(
        registry.get(&"omp".parse().unwrap()).unwrap().enabled,
        Enabled::Off
    );
    assert!(registry.unresolved().is_empty());
    assert!(registry.shadowed_plugins().is_empty());
}

#[test]
fn derived_providers_inherit_policy_and_launch_but_have_distinct_identities() {
    let base = ProviderOverride {
        enabled: Some(Enabled::Off),
        env: ProviderEnvironment::new(BTreeMap::from([(
            "BASE".parse().unwrap(),
            ConfigValue::Text("synthetic".into()),
        )]))
        .unwrap(),
        ..ProviderOverride::default()
    };
    let custom = ProviderOverride {
        extends: Some(ProviderKind::Claude),
        label: Some("Other Claude".parse().unwrap()),
        ..ProviderOverride::default()
    };
    let registry =
        ProviderRegistry::build(&config(&[("claude", base), ("other-claude", custom)]), &[])
            .unwrap();
    let custom = registry.get(&"other-claude".parse().unwrap()).unwrap();
    let base = registry.get(&"claude".parse().unwrap()).unwrap();
    assert_eq!(custom.kind, base.kind);
    assert_eq!(custom.create_policy, base.create_policy);
    assert_eq!(custom.launch, base.launch);
    assert_eq!(custom.enabled, Enabled::On);
    assert_eq!(custom.derived_from, Some(base.id.clone()));
    assert_eq!(custom.source, ProviderSource::Configured);
}

#[test]
fn custom_acp_requires_a_label_and_replacement_command_before_any_client_exists() {
    let mut custom = ProviderOverride {
        extends: Some(ProviderKind::Acp),
        ..ProviderOverride::default()
    };
    assert!(matches!(
        ProviderRegistry::build(&config(&[("custom", custom.clone())]), &[]),
        Err(RegistryError::MissingLabel(_))
    ));
    custom.label = Some("Custom".parse().unwrap());
    assert!(matches!(
        ProviderRegistry::build(&config(&[("custom", custom.clone())]), &[]),
        Err(RegistryError::MissingCommand(_))
    ));
    custom.command = Some(CommandEdit::Replace {
        program: "fake-acp".parse().unwrap(),
        args: Arguments::default(),
    });
    let registry = ProviderRegistry::build(&config(&[("custom", custom)]), &[]).unwrap();
    let entry = registry.get(&"custom".parse().unwrap()).unwrap();
    assert_eq!(entry.create_policy, CreatePolicy::Acp);
    assert!(entry.modes.modes().is_empty());
    assert_eq!(entry.modes.default_mode(), None);
}

#[test]
fn plugin_collisions_duplicates_configuration_and_shadowing_are_explicit() {
    assert!(matches!(
        ProviderRegistry::build(&ProviderOverrides::default(), &[plugin("claude")]),
        Err(RegistryError::BuiltinCollision(_))
    ));
    let mut registered = plugin("custom-plugin");
    assert!(matches!(
        ProviderRegistry::build(
            &ProviderOverrides::default(),
            &[registered.clone(), registered.clone()]
        ),
        Err(RegistryError::DuplicatePlugin(_))
    ));
    registered.configuration = ConfigurationRequirement::Required;
    assert!(
        ProviderRegistry::build(&ProviderOverrides::default(), &[registered.clone()])
            .unwrap()
            .get(&registered.id)
            .is_none()
    );
    let settings = config(&[
        ("custom-plugin", ProviderOverride::default()),
        ("unregistered", ProviderOverride::default()),
    ]);
    let registry = ProviderRegistry::build(&settings, &[registered.clone()]).unwrap();
    assert_eq!(
        registry.get(&registered.id).unwrap().source,
        ProviderSource::Plugin(registered.revision)
    );
    assert_eq!(
        registry.unresolved(),
        ["unregistered".parse::<ProviderId>().unwrap()]
    );
    let shadows = ProviderOverride {
        extends: Some(ProviderKind::Codex),
        label: Some("Configured wins".parse().unwrap()),
        ..ProviderOverride::default()
    };
    let registry = ProviderRegistry::build(
        &config(&[("custom-plugin", shadows)]),
        &[registered.clone()],
    )
    .unwrap();
    assert_eq!(registry.shadowed_plugins(), [registered.id.clone()]);
    assert_eq!(
        registry.get(&registered.id).unwrap().kind,
        ProviderKind::Codex
    );
}

#[test]
fn a_replacement_registration_changes_the_generation_even_when_metadata_matches() {
    let mut registration = plugin("local");
    let before =
        ProviderRegistry::build(&ProviderOverrides::default(), &[registration.clone()]).unwrap();
    registration.revision = PluginRevision::new(2);
    let after = ProviderRegistry::build(&ProviderOverrides::default(), &[registration]).unwrap();
    assert_ne!(before, after);
}
