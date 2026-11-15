//! Whole-field settings output and mutation through the authenticated node.
//! Decode before resolving a connection; the node alone owns persistence.

use vessel_proto::Method;
use vessel_proto::settings::{
    GetNodeSettings, NodeSettingKey, ReloadNodeSettings, SettingKeys, UnsetNodeSettings,
};

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
                check_read(key, client.protocol())?;
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

// A missing field must be refused before a typed default can masquerade as data.
fn check_read(
    key: NodeSettingKey,
    negotiated: vessel_proto::ProtocolVersion,
) -> Result<(), vessel_client::CallError> {
    let needs = key
        .required_protocol()
        .ok_or(vessel_client::CallError::UnassignedProtocol {
            method: Method::GetNodeSettings,
        })?;
    if negotiated < needs {
        return Err(vessel_client::CallError::NotInProtocol {
            method: Method::GetNodeSettings,
            needs,
            negotiated,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_read_refuses_older_or_unassigned_admission_before_rpc() {
        let key = NodeSettingKey::Drivers;
        let needs = key.required_protocol().unwrap();
        let older = vessel_proto::ProtocolVersion::new(needs.get().checked_sub(1).unwrap());
        assert!(
            matches!(check_read(key, older), Err(vessel_client::CallError::NotInProtocol { needs: actual, negotiated, .. }) if actual == needs && negotiated == older)
        );
        assert_eq!(check_read(key, needs), Ok(()));
        assert!(matches!(
            check_read(NodeSettingKey::Unknown, vessel_proto::PROTOCOL_VERSION),
            Err(vessel_client::CallError::UnassignedProtocol {
                method: Method::GetNodeSettings
            })
        ));
    }
}
