//! Pure candidate validation leaves the committed directory untouched on every refusal.
use super::{Change, RegistryError, TrustedDestinations};
use vessel_proto::host_registry::{
    HostDestination, HostEnabled, HostLabelChange, HostRegistration, HostRegistrationId,
    HostRevision, HostSource, IdentityChange, MAX_HOST_REGISTRATIONS, SharedHostRegistration,
    SharedHostRegistrations,
};
use vessel_store::host_registry::HostRegistryFile;

pub(super) struct State {
    entries: Vec<HostRegistration>,
}
impl State {
    pub(super) fn shared(&self) -> Result<SharedHostRegistrations, RegistryError> {
        // Explicit projection excludes authentication source, revisions and private state.
        self.entries
            .iter()
            .map(|record| SharedHostRegistration {
                registration_id: record.registration_id,
                destination: record.destination,
                destination_name: None,
                label: record.label.clone(),
                annotations: record.metadata.clone(),
            })
            .collect::<Vec<_>>()
            .try_into()
            .map_err(RegistryError::SharedInventory)
    }
    pub(super) fn new(file: HostRegistryFile) -> Self {
        Self {
            entries: file.into_entries(),
        }
    }
    pub(super) fn list(&self, source: HostSource) -> Vec<HostRegistration> {
        self.entries
            .iter()
            .filter(|e| e.source == source)
            .cloned()
            .collect()
    }
    pub(super) fn prepare(
        &self,
        source: HostSource,
        change: Change,
    ) -> Result<(Vec<HostRegistration>, Option<HostRegistration>), RegistryError> {
        let mut entries = self.entries.clone();
        let result = match change {
            Change::Add(input, trusted) => {
                trusted.require(input.destination)?;
                if entries.len() >= MAX_HOST_REGISTRATIONS {
                    return Err(RegistryError::Full);
                }
                let record = HostRegistration {
                    registration_id: HostRegistrationId::random(),
                    source,
                    destination: input.destination,
                    label: input.label,
                    metadata: input.metadata,
                    enabled: HostEnabled::Enabled,
                    revision: HostRevision::default(),
                };
                entries.push(record.clone());
                Some(record)
            }
            Change::Update(input, trusted) => {
                let record = entries
                    .iter_mut()
                    .find(|e| e.registration_id == input.registration_id)
                    .ok_or(RegistryError::NotFound)?;
                authority(record, source, input.expected_revision)?;
                if let Some(destination) = input.destination {
                    trusted.require(destination)?;
                    if destination.node() != record.destination.node()
                        && input.identity_change != IdentityChange::Replace
                    {
                        return Err(RegistryError::IdentityChanged);
                    }
                    record.destination = destination;
                }
                match input.label {
                    HostLabelChange::Keep => {}
                    HostLabelChange::Clear => record.label = None,
                    HostLabelChange::Set(label) => record.label = Some(label),
                }
                if let Some(metadata) = input.metadata {
                    record.metadata = metadata;
                }
                if let Some(enabled) = input.enabled {
                    record.enabled = enabled;
                }
                record.revision = record
                    .revision
                    .next()
                    .ok_or(RegistryError::RevisionExhausted)?;
                Some(record.clone())
            }
            Change::Remove(input) => {
                let record = entries
                    .iter()
                    .find(|e| e.registration_id == input.registration_id)
                    .ok_or(RegistryError::NotFound)?;
                authority(record, source, input.expected_revision)?;
                entries.retain(|e| e.registration_id != input.registration_id);
                None
            }
        };
        Ok((entries, result))
    }
}
fn authority(
    record: &HostRegistration,
    source: HostSource,
    revision: HostRevision,
) -> Result<(), RegistryError> {
    if record.source != source {
        return Err(RegistryError::Forbidden);
    }
    if record.revision != revision {
        return Err(RegistryError::Stale);
    }
    Ok(())
}
impl TrustedDestinations {
    fn require(&self, destination: HostDestination) -> Result<(), RegistryError> {
        if self.0.contains(&destination) {
            Ok(())
        } else {
            Err(RegistryError::Untrusted)
        }
    }
}
