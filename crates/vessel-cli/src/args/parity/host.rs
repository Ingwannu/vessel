//! Host CLI validates annotations and reference identities before any request is sent.
use crate::error::CliError;
use clap::Subcommand;
use vessel_proto::host_registry::{
    HostLabel, HostMetadata, HostRegistrationId, MetadataKey, MetadataValue,
};

#[derive(Debug, Subcommand)]
pub(crate) enum HostCommand {
    Ls,
    Add {
        connection: String,
        #[arg(long)]
        label: Option<HostLabel>,
        /// Public annotations readable by local agents; never include secrets.
        #[arg(long)]
        meta: Vec<String>,
    },
    Update {
        id: HostRegistrationId,
        #[arg(long)]
        label: Option<HostLabel>,
        #[arg(long)]
        connection: Option<String>,
        #[arg(long)]
        replace_identity: bool,
        /// Public annotations readable by local agents; never include secrets.
        #[arg(long)]
        meta: Vec<String>,
        #[arg(long)]
        remove_meta: Vec<MetadataKey>,
    },
    Inspect {
        id: HostRegistrationId,
    },
    Enable {
        id: HostRegistrationId,
    },
    Disable {
        id: HostRegistrationId,
    },
    Rm {
        id: HostRegistrationId,
    },
}
impl HostCommand {
    pub(crate) fn validate(&self) -> Result<(), CliError> {
        match self {
            Self::Ls
            | Self::Inspect { .. }
            | Self::Enable { .. }
            | Self::Disable { .. }
            | Self::Rm { .. } => Ok(()),
            Self::Add {
                connection, meta, ..
            } => {
                let _destination = destination(connection)?;
                let _metadata = metadata(meta)?;
                Ok(())
            }
            Self::Update {
                connection, meta, ..
            } => {
                if let Some(connection) = connection {
                    let _destination = destination(connection)?;
                }
                let _metadata = metadata(meta)?;
                Ok(())
            }
        }
    }
}
pub(crate) fn destination(
    text: &str,
) -> Result<vessel_proto::host_registry::HostDestination, CliError> {
    use vessel_proto::host_registry::HostDestination;
    if let Some(id) = text.strip_prefix("fleet:") {
        return Ok(HostDestination::Fleet {
            node: id.parse().map_err(|_failure| {
                CliError::Usage("fleet: must be followed by a full node UUID")
            })?,
        });
    }
    if let Some(id) = text.strip_prefix("paired:") {
        return Ok(HostDestination::Paired {
            node: id.parse().map_err(|_failure| {
                CliError::Usage("paired: must be followed by a full node UUID")
            })?,
        });
    }
    Err(CliError::ParityBlocked(
        "resolve direct/SSH/pairing connections through their existing trust authority first; the registry accepts fleet:<NodeId> or paired:<NodeId>",
    ))
}
pub(crate) fn metadata(items: &[String]) -> Result<HostMetadata, CliError> {
    let mut map = std::collections::BTreeMap::new();
    for item in items {
        let (key, value) = item
            .split_once('=')
            .ok_or(CliError::Usage("metadata must be key=value"))?;
        if key.is_empty() {
            return Err(CliError::Usage("metadata key must be nonempty"));
        }
        let key = key
            .parse::<MetadataKey>()
            .map_err(CliError::ParityHostText)?;
        let value = value
            .parse::<MetadataValue>()
            .map_err(CliError::ParityHostText)?;
        let _replaced = map.insert(key, value);
    }
    HostMetadata::try_from(map).map_err(CliError::ParityHostText)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_targets_are_not_silently_added_to_the_local_book() {
        assert!(matches!(
            destination("ssh://build.invalid"),
            Err(CliError::ParityBlocked(_))
        ));
        assert!(destination("fleet:invalid").is_err());
        assert!(destination(&format!("fleet:{}", vessel_proto::NodeId::random())).is_ok());
        assert!(metadata(&["missing-equals".to_owned()]).is_err());
        assert!(metadata(&["=value".to_owned()]).is_err());
        assert!(metadata(&["note=control\u{1b}".to_owned()]).is_err());
    }
}
