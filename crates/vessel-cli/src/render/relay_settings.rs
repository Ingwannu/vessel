//! A settings report describes registration lifetimes, never credentials/routes.
use vessel_proto::relay_settings::{CallbackRelays, ConfiguredRelays, RelayHealth, RelaySettings};

pub(super) fn render(settings: Option<RelaySettings>) -> String {
    let Some(settings) = settings else {
        return String::from(
            "configured relays: not reported\nfleet callback relays: not reported\n",
        );
    };
    let configured = match settings.configured {
        ConfiguredRelays::Unconfigured => "not configured",
        ConfiguredRelays::Disabled => "disabled",
        ConfiguredRelays::Disabling => "stopping registrations",
        ConfiguredRelays::Unknown => "a newer node's state",
        ConfiguredRelays::Enabled { health } => match health {
            RelayHealth::Connecting => "enabled, connecting",
            RelayHealth::Online => "enabled, registered",
            RelayHealth::Retrying => "enabled, retrying registration",
            RelayHealth::Stopped => "enabled, registrations stopped",
            RelayHealth::Unknown => "enabled, registration health unknown",
        },
    };
    let callback = match settings.fleet_callback {
        CallbackRelays::Absent => "absent",
        CallbackRelays::Present => "present (independent of configured relays)",
        CallbackRelays::Unknown => "a newer node's state",
    };
    format!("configured relays: {configured}\nfleet callback relays: {callback}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_absence_never_means_disabled() {
        assert_eq!(
            render(None),
            "configured relays: not reported\nfleet callback relays: not reported\n"
        );
    }
    #[test]
    fn configured_and_callback_lifetimes_stay_separate() {
        for (configured, label) in [
            (ConfiguredRelays::Unconfigured, "not configured"),
            (ConfiguredRelays::Disabled, "disabled"),
            (ConfiguredRelays::Disabling, "stopping registrations"),
            (ConfiguredRelays::Unknown, "a newer node's state"),
            (
                ConfiguredRelays::Enabled {
                    health: RelayHealth::Connecting,
                },
                "enabled, connecting",
            ),
            (
                ConfiguredRelays::Enabled {
                    health: RelayHealth::Online,
                },
                "enabled, registered",
            ),
            (
                ConfiguredRelays::Enabled {
                    health: RelayHealth::Retrying,
                },
                "enabled, retrying registration",
            ),
            (
                ConfiguredRelays::Enabled {
                    health: RelayHealth::Stopped,
                },
                "enabled, registrations stopped",
            ),
            (
                ConfiguredRelays::Enabled {
                    health: RelayHealth::Unknown,
                },
                "enabled, registration health unknown",
            ),
        ] {
            for (fleet_callback, callback) in [
                (CallbackRelays::Absent, "absent"),
                (
                    CallbackRelays::Present,
                    "present (independent of configured relays)",
                ),
                (CallbackRelays::Unknown, "a newer node's state"),
            ] {
                assert_eq!(
                    render(Some(RelaySettings {
                        configured,
                        fleet_callback
                    })),
                    format!("configured relays: {label}\nfleet callback relays: {callback}\n")
                );
            }
        }
    }
}
