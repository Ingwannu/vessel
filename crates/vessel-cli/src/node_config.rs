//! Whole-field settings output and mutation through the authenticated node.
//! Decode before resolving a connection; the node alone owns persistence.

use vessel_proto::Method;
use vessel_proto::settings::GetNodeSettings;

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
                let needs = key.required_protocol().ok_or(
                    vessel_client::CallError::UnassignedProtocol {
                        method: Method::GetNodeSettings,
                    },
                )?;
                if client.protocol() < needs {
                    return Err(vessel_client::CallError::NotInProtocol {
                        method: Method::GetNodeSettings,
                        needs,
                        negotiated: client.protocol(),
                    }
                    .into());
                }
            }
            let settings = client.call(GetNodeSettings {}).await?;
            match key {
                Some(key) => output(&key.value(&settings)?).await,
                None => output(&settings).await,
            }
        }
        ConfigCommand::Set { key, value } => {
            let change = key.change(&value)?;
            let client = connect(target).await?.client;
            let settings = client.call(change).await?;
            output(&key.value(&settings)?).await
        }
    }
}

async fn output(value: &impl serde::Serialize) -> Result<(), CliError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(CliError::SettingsEncode)?;
    bytes.push(b'\n');
    Ok(io::out(&bytes).await?)
}
