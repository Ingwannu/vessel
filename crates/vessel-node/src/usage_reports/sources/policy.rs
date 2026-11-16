//! Persist/apply belongs to the actor, so dropping an RPC does not abandon a consent change.
use super::{
    ErrorCode, ExternalLookup, Observation, Owner, ReportSourceGrants, SetUsageSourceLookup,
    Source, UsageSourceDescriptor, WireError, changed, unsupported,
};
use vessel_proto::usage_reports::{UsageDisclosureVersion, UsageSourceGrant};
impl Owner {
    /// Atomically replace the saved grants; the file's state is unknown after
    /// a failure, so the next choice writes whatever memory then holds.
    async fn save(&mut self, grants: ReportSourceGrants) -> Result<(), WireError> {
        self.persisted = None;
        let dir = self.dir.clone();
        let saved = grants.clone();
        tokio::task::spawn_blocking(move || dir.write_usage_sources(&saved))
            .await
            .map_err(|_task| changed())?
            .map_err(|_private| {
                WireError::new(
                    ErrorCode::Internal,
                    "source consent could not be saved; read the current choice before trying again",
                )
            })?;
        self.persisted = Some(grants);
        Ok(())
    }
    pub(super) async fn set(
        &mut self,
        request: SetUsageSourceLookup,
        global: &Observation,
    ) -> Result<UsageSourceDescriptor, WireError> {
        let source = Source::from_id(&request.source).ok_or_else(|| {
            WireError::new(ErrorCode::NotFound, "that usage source is not registered")
        })?;
        if request.disclosure_version != UsageDisclosureVersion::INITIAL {
            return Err(WireError::new(
                ErrorCode::Conflict,
                "the usage source disclosure changed; read it before choosing again",
            ));
        }
        if request.policy == ExternalLookup::On && !self.probe.supports(source.builtin()) {
            return Err(unsupported());
        }
        let mut next = self
            .grants
            .as_slice()
            .iter()
            .filter(|grant| grant.id != request.source)
            .cloned()
            .collect::<Vec<_>>();
        if request.policy == ExternalLookup::On {
            next.push(UsageSourceGrant {
                id: request.source,
                version: request.disclosure_version,
            });
        }
        let next = ReportSourceGrants::try_from(next).map_err(|_bound| changed())?;
        if next != self.grants {
            self.policy_generation.cancel();
            self.policy_generation = self.stop.child_token();
            self.invalidate(source);
            if request.policy == ExternalLookup::Off {
                // Revocation applies in memory at once, even if saving fails.
                self.grants = next.clone();
            }
        }
        // Decide the write against the file, never against memory: after a
        // failed save of Off, memory already reads Off while the file may
        // still grant the source, and a restart would revive that grant.
        if self.persisted.as_ref() != Some(&next) {
            self.save(next.clone()).await?;
            self.grants = next;
        }
        if !global.still_current() {
            return Err(changed());
        }
        source.descriptor(
            global.policy,
            self.consent(source),
            self.probe.supports(source.builtin()),
        )
    }
}
