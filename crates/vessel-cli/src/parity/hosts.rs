//! Host CLI metadata edits compare revisions and never mutate the client-local node book.
use super::output::{self, Row};
use crate::{
    args::{
        Target,
        parity::{HostCommand, Presentation, host_destination, host_metadata},
    },
    connect::connect,
    error::CliError,
};
use serde::Serialize;
use vessel_client::NodeClient;
use vessel_proto::{
    NodeId, WireError,
    host_registry::{
        AddHostRegistration, HostEnabled, HostLabel, HostLabelChange, HostMetadata,
        HostRegistration, HostRegistrationId, HostSource, HostStatus, HostSummary, IdentityChange,
        RemoveHostRegistration, UpdateHostRegistration,
    },
};

pub(super) async fn run(
    target: &Target,
    presentation: Presentation,
    command: HostCommand,
) -> Result<(), CliError> {
    command.validate()?;
    if vessel_proto::host_registry::CLI_PARITY_PROTOCOL.is_none() {
        return Err(CliError::ParityBlocked(
            "host registrations await the integrator's CLI_PARITY_PROTOCOL assignment",
        ));
    }
    let client = connect(target).await?.client;
    match command {
        HostCommand::Ls => {
            let rows = client
                .host_registrations()
                .await?
                .hosts
                .into_iter()
                .map(HostRow::from)
                .collect::<Vec<_>>();
            output::list(presentation, &rows).await
        }
        HostCommand::Add {
            connection,
            label,
            meta,
        } => {
            let input = AddHostRegistration {
                destination: host_destination(&connection)?,
                label,
                metadata: host_metadata(&meta)?,
            };
            let record = client.add_host_registration(input).await?;
            show_record(&client, presentation, record).await
        }
        HostCommand::Inspect { id } => {
            // Paseo's inspect deliberately supplies a JSON human renderer, unlike mutations.
            let row = HostRow::from(find(&client, id).await?);
            output::single(presentation, &row.registration_id.to_string(), &row).await
        }
        HostCommand::Rm { id } => {
            let old = find(&client, id).await?;
            client
                .remove_host_registration(RemoveHostRegistration {
                    registration_id: id,
                    expected_revision: old.registration.revision,
                })
                .await?;
            show(presentation, old).await
        }
        HostCommand::Enable { id } => {
            enabled(&client, presentation, id, HostEnabled::Enabled).await
        }
        HostCommand::Disable { id } => {
            enabled(&client, presentation, id, HostEnabled::Disabled).await
        }
        HostCommand::Update {
            id,
            label,
            connection,
            replace_identity,
            meta,
            remove_meta,
        } => {
            let old = find(&client, id).await?.registration;
            let metadata = if meta.is_empty() && remove_meta.is_empty() {
                None
            } else {
                let mut merged = old.metadata.entries().clone();
                merged.extend(
                    host_metadata(&meta)?
                        .entries()
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone())),
                );
                for key in remove_meta {
                    let _removed = merged.remove(&key);
                }
                Some(HostMetadata::try_from(merged).map_err(CliError::ParityHostText)?)
            };
            let label = match label {
                None => HostLabelChange::Keep,
                Some(label) if label.as_str().is_empty() => HostLabelChange::Clear,
                Some(label) => HostLabelChange::Set(label),
            };
            let record = client
                .update_host_registration(UpdateHostRegistration {
                    registration_id: id,
                    expected_revision: old.revision,
                    label,
                    metadata,
                    enabled: None,
                    destination: connection.as_deref().map(host_destination).transpose()?,
                    identity_change: if replace_identity {
                        IdentityChange::Replace
                    } else {
                        IdentityChange::KeepPinned
                    },
                })
                .await?;
            show_record(&client, presentation, record).await
        }
    }
}
async fn enabled(
    client: &NodeClient,
    presentation: Presentation,
    id: HostRegistrationId,
    enabled: HostEnabled,
) -> Result<(), CliError> {
    let old = find(client, id).await?.registration;
    let record = client
        .update_host_registration(UpdateHostRegistration {
            registration_id: id,
            expected_revision: old.revision,
            label: HostLabelChange::Keep,
            metadata: None,
            enabled: Some(enabled),
            destination: None,
            identity_change: IdentityChange::KeepPinned,
        })
        .await?;
    show_record(client, presentation, record).await
}
async fn find(client: &NodeClient, id: HostRegistrationId) -> Result<HostSummary, CliError> {
    find_optional(client, id)
        .await?
        .ok_or(CliError::Usage("unknown host registration ID"))
}
async fn find_optional(
    client: &NodeClient,
    id: HostRegistrationId,
) -> Result<Option<HostSummary>, CliError> {
    Ok(client
        .host_registrations()
        .await?
        .hosts
        .into_iter()
        .find(|host| host.registration.registration_id == id))
}
async fn show_record(
    client: &NodeClient,
    presentation: Presentation,
    record: HostRegistration,
) -> Result<(), CliError> {
    // Connectivity is a later observation. A concurrent edit must not replace
    // the acknowledged mutation's metadata in the success receipt.
    let observed = find_optional(client, record.registration_id).await?;
    show(presentation, checked_summary(&record, observed)?).await
}
fn checked_summary(
    acknowledged: &HostRegistration,
    observed: Option<HostSummary>,
) -> Result<HostSummary, CliError> {
    let observed = observed.ok_or(CliError::ParityHostObservationChanged(
        acknowledged.registration_id,
    ))?;
    if observed.registration.registration_id != acknowledged.registration_id
        || observed.registration.source != acknowledged.source
    {
        return Err(CliError::ParityInvalidReceipt(
            "host observation target/source differ from mutation acknowledgement",
        ));
    }
    if observed.registration != *acknowledged {
        return Err(CliError::ParityHostObservationChanged(
            acknowledged.registration_id,
        ));
    }
    Ok(observed)
}
async fn show(presentation: Presentation, host: HostSummary) -> Result<(), CliError> {
    let row = HostRow::from(host);
    output::record(presentation, &row).await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostRow {
    registration_id: HostRegistrationId,
    server_id: NodeId,
    label: Option<HostLabel>,
    source: HostSource,
    metadata: HostMetadata,
    enabled: Enabled,
    status: HostStatus,
    target_description: String,
    error: Option<WireError>,
}
struct Enabled(HostEnabled);
impl Serialize for Enabled {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.0 == HostEnabled::Enabled)
    }
}
impl From<HostSummary> for HostRow {
    fn from(summary: HostSummary) -> Self {
        let record = summary.registration;
        let target_description = match record.destination {
            vessel_proto::host_registry::HostDestination::Fleet { node } => format!("fleet:{node}"),
            vessel_proto::host_registry::HostDestination::Paired { node } => {
                format!("paired:{node}")
            }
        };
        Self {
            registration_id: record.registration_id,
            server_id: record.destination.node(),
            label: record.label.or(summary.destination_name),
            source: record.source,
            metadata: record.metadata,
            enabled: Enabled(record.enabled),
            status: summary.status,
            target_description,
            error: summary.error,
        }
    }
}
impl Row for HostRow {
    fn id(&self) -> String {
        self.registration_id.to_string()
    }
    fn headers() -> &'static [&'static str] {
        &["ID", "LABEL", "SERVER", "ENABLED", "STATUS", "SOURCE"]
    }
    fn cells(&self) -> Vec<String> {
        vec![
            self.registration_id.to_string(),
            self.label
                .as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string),
            self.server_id.to_string(),
            (self.enabled.0 == HostEnabled::Enabled).to_string(),
            format!("{:?}", self.status).to_ascii_lowercase(),
            match self.source {
                HostSource::Owner => "owner".to_owned(),
                HostSource::Device { device } => format!("device:{device}"),
            },
        ]
    }
}

#[cfg(test)]
mod tests;
