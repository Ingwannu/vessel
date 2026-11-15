//! Dedup/order, preferred-host loss/return, persistence and override isolation.
use super::*;
use crate::{
    appearance::FontFamily,
    prefs::{PreferenceChange, Preferences},
};
use vessel_proto::{NodeId, plugins::RuntimeGeneration};

fn installation(node: NodeId, plugin: &str) -> PluginAddress {
    PluginAddress {
        node,
        plugin: plugin.parse().unwrap(),
        generation: RuntimeGeneration::random(),
    }
}
fn theme(id: &str, name: &str) -> ThemeContribution {
    let mut theme: ThemeContribution = serde_json::from_str(r##"{"id":"dark","name":"Fixture","appearance":"dark","colors":{"background":"#12345678","foreground":"#fff","raised":"#222","control":"#333","border":"#444","mutedForeground":"#aaa","ring":"#777"}}"##).unwrap();
    theme.id = id.parse().unwrap();
    theme.name = name.parse().unwrap();
    theme
}
#[test]
fn logical_order_and_preferred_host_do_not_sort_by_key_or_runtime_generation() {
    let first = installation(NodeId::random(), "zulu");
    let second = installation(NodeId::random(), "zulu");
    let another = installation(first.node, "alpha");
    let a = theme("dark", "First host");
    let b = theme("dark", "Preferred host");
    let c = theme("light", "Another plugin");
    let targets = [
        Candidate {
            address: &first,
            theme: &a,
        },
        Candidate {
            address: &another,
            theme: &c,
        },
        Candidate {
            address: &second,
            theme: &b,
        },
    ];
    let mut hosts = PreferredHosts::default();
    hosts.choose(targets[0].key(), second.node);
    let catalog = Catalog::new(targets, &hosts);
    assert_eq!(catalog.entries().len(), 2);
    assert_eq!(catalog.entries()[0].key().to_string(), "zulu/theme/dark");
    assert_eq!(catalog.entries()[0].candidate().address, &second);
    assert_eq!(
        catalog.entries()[0].candidate().theme.name.as_str(),
        "Preferred host"
    );
    assert_eq!(catalog.entries()[1].key().to_string(), "alpha/theme/light");
    let absent = Catalog::new([targets[0]], &hosts);
    assert_eq!(absent.entries()[0].candidate().address, &first);
    assert_eq!(hosts.get(&targets[0].key()), Some(&second.node));
    let returned = Catalog::new(targets, &hosts);
    assert_eq!(returned.entries()[0].candidate().address, &second);
}
#[test]
fn unavailable_selection_falls_back_without_deleting_identity_and_recovers_after_restart() {
    let address = installation(NodeId::random(), "custom");
    let colors = theme("night", "Night");
    let target = Candidate {
        address: &address,
        theme: &colors,
    };
    let original = Preferences::default()
        .changed(&PreferenceChange::MonoFamily(
            FontFamily::parse("'Embedded Face', monospace").unwrap(),
        ))
        .unwrap();
    let selected = original
        .changed(&PreferenceChange::ContributedTheme(target.key()))
        .unwrap();
    let document = serde_json::to_string(&selected).unwrap();
    let restored: Preferences = serde_json::from_str(&document).unwrap();
    assert_eq!(restored, selected);
    let hosts = PreferredHosts::default();
    assert!(matches!(
        Catalog::new([], &hosts).resolve(&restored.appearance),
        Resolution::Builtin(Theme::System)
    ));
    assert_eq!(serde_json::to_string(&restored).unwrap(), document);
    let present = Catalog::new([target], &hosts);
    assert!(
        matches!(present.resolve(&restored.appearance), Resolution::Contributed(found) if found.address == &address)
    );
    assert_eq!(restored.appearance.next_theme(), Theme::Light);
    let builtin = restored
        .changed(&PreferenceChange::Theme(Theme::Midnight))
        .unwrap();
    assert!(
        matches!(&builtin.appearance.plugin_theme, Some(ContributedChoice::Remembered(key)) if key == &target.key())
    );
    assert!(matches!(
        present.resolve(&builtin.appearance),
        Resolution::Builtin(Theme::Midnight)
    ));
    assert_eq!(
        builtin.appearance.mono_family,
        original.appearance.mono_family
    );
    let again = builtin
        .changed(&PreferenceChange::ContributedTheme(target.key()))
        .unwrap();
    assert_eq!(again.appearance.theme, Theme::Midnight);
    assert_eq!(
        again.appearance.mono_family,
        original.appearance.mono_family
    );
}
#[test]
fn malformed_keys_are_not_choices_and_lenient_file_read_retains_valid_siblings() {
    for raw in [
        "",
        "vendor/dark",
        "Vendor/theme/dark",
        "vendor/theme/",
        "vendor/theme/a/b",
        "vendor/theme/dark/theme/x",
        "vendor/theme/../dark",
    ] {
        assert!(raw.parse::<PluginThemeId>().is_err(), "{raw}");
    }
    let read: Preferences = serde_json::from_str(r#"{"appearance":{"theme":"dark","code_size":19,"plugin_theme":{"state":"selected","theme":"bad"}}}"#).unwrap();
    assert_eq!(read.appearance.theme, Theme::Dark);
    assert_eq!(read.appearance.code_size.get(), 19);
    assert_eq!(read.appearance.plugin_theme, None);
    let key: PluginThemeId = "vendor/theme/night".parse().unwrap();
    assert_eq!(
        serde_json::to_string(&key).unwrap(),
        "\"vendor/theme/night\""
    );
    assert_eq!(
        serde_json::from_str::<PluginThemeId>("\"vendor/theme/night\"").unwrap(),
        key
    );
}
