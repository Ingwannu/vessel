//! ADR 0115's settings documents (`PLUGIN_SETTINGS_PROTOCOL`): three Admin
//! calls after the catalog, never across a fleet hop, the newest protocol.

use super::*;
use crate::plugins::{PLUGIN_CATALOG_PROTOCOL, PLUGIN_SETTINGS_PROTOCOL};

#[test]
fn plugin_settings_are_an_administrators_and_come_after_the_catalog() {
    let methods: Vec<_> = Method::ALL
        .iter()
        .filter(|method| method.since() == PLUGIN_SETTINGS_PROTOCOL)
        .inspect(|method| {
            assert_eq!(method.shape(), MethodShape::Unary, "{method:?}");
            assert!(!method.forwardable(), "{method:?}");
        })
        .map(|method| (method.as_str(), method.scope()))
        .collect();
    assert_eq!(
        methods,
        [
            ("plugin.settings.read", Some(Scope::Admin)),
            ("plugin.settings.write", Some(Scope::Admin)),
            ("plugin.settings.reset", Some(Scope::Admin)),
        ]
    );
    assert!(PLUGIN_CATALOG_PROTOCOL < PLUGIN_SETTINGS_PROTOCOL);
    assert_eq!(PLUGIN_SETTINGS_PROTOCOL, Some(crate::PROTOCOL_VERSION));
}
