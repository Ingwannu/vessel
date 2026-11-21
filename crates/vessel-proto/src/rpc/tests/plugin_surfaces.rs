//! ADR 0115's surfaces (`PLUGIN_SURFACES_PROTOCOL`): new declaration kinds
//! in the catalog and no method, after the settings documents they bind; the
//! newest protocol.

use super::*;
use crate::plugins::{PLUGIN_SETTINGS_PROTOCOL, PLUGIN_SURFACES_PROTOCOL};

#[test]
fn the_surfaces_add_no_method_and_come_after_the_settings() {
    assert!(
        Method::ALL
            .iter()
            .all(|method| method.since() != PLUGIN_SURFACES_PROTOCOL),
        "the surfaces change only what plugin.catalog may hold"
    );
    assert!(PLUGIN_SETTINGS_PROTOCOL < PLUGIN_SURFACES_PROTOCOL);
    assert_eq!(PLUGIN_SURFACES_PROTOCOL, Some(crate::PROTOCOL_VERSION));
}
