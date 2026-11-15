//! Parse typed whole-field configuration selection; no connection or file access.

use clap::Subcommand;
use vessel_proto::settings::NodeSettingKey;

#[derive(Debug, Subcommand)]
pub(crate) enum ConfigCommand {
    /// Print all live settings, or the JSON value of one whole field.
    Get { key: Option<NodeSettingKey> },
    /// Remove whole saved overrides so launch/default values apply again.
    Unset {
        #[arg(num_args = vessel_proto::settings::SettingKeys::MINIMUM..=vessel_proto::settings::SettingKeys::MAXIMUM)]
        keys: Vec<NodeSettingKey>,
    },
    /// Save one typed JSON value and apply it through `node.set_settings`.
    Set {
        key: NodeSettingKey,
        /// Complete JSON value, quoted for your shell; dotted paths are not used.
        value: String,
    },
}
