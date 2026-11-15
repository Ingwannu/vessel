//! Whole-field settings output and mutation through the authenticated node.
//! Decode before resolving a connection; the node alone owns persistence.

use vessel_proto::settings::{GetNodeSettings, ReloadNodeSettings, SettingKeys, UnsetNodeSettings};

use crate::args::{ConfigCommand, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::io;

/// Keep file ownership on the node and validate a mutation before connecting.
pub(crate) async fn run(target: &Target, command: ConfigCommand) -> Result<(), CliError> {
    match command {
        ConfigCommand::Get { key } => {
            let client = connect(target).await?.client;
            if let Some(key) = key {
                client.check_call(&key.read_requirement()?)?;
            }
            let settings = client.call(GetNodeSettings {}).await?;
            match key {
                Some(key) => output(&key.value(&settings)?).await,
                None => output(&settings).await,
            }
        }
        ConfigCommand::Unset { keys } => {
            let keys = SettingKeys::new(keys)?;
            let client = connect(target).await?.client;
            output(&client.call(UnsetNodeSettings { keys }).await?).await
        }
        ConfigCommand::Set { key, value } => {
            let change = key.change(&value)?;
            let client = connect(target).await?.client;
            let settings = client.call(change).await?;
            output(&key.value(&settings)?).await
        }
    }
}

/// Reload through the same assigned-protocol admission as every typed call.
pub(crate) async fn reload(target: &Target) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    output(&client.call(ReloadNodeSettings {}).await?).await
}

async fn output(value: &impl serde::Serialize) -> Result<(), CliError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(CliError::SettingsEncode)?;
    bytes.push(b'\n');
    Ok(io::out(&bytes).await?)
}
