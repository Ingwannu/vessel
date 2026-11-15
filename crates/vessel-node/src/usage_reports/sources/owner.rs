//! Cache reads and policy commands never invoke a source; only Refresh starts jobs.
use super::{
    Command, ExternalLookup, Observation, Owner, Snapshot, Source, WireError, changed, mpsc, spec,
};
use vessel_proto::usage_reports::{UsageDisclosureVersion, UsageSourceConsent};
impl Owner {
    pub(super) async fn run(mut self, mut commands: mpsc::Receiver<Command>) {
        loop {
            let observed_global = self.observed_global.clone();
            tokio::select! {biased;
                ()=self.stop.cancelled()=>break,
                ()=revoked(observed_global)=>{
                    self.invalidate_all();
                    self.observed_global=None;
                },
                result=self.jobs.join_next(),if !self.jobs.is_empty()=>if let Some(result)=result{match result{Ok(done)=>self.completed(done),Err(_panic)=>{self.invalidate_all();tracing::error!("a source usage worker ended unexpectedly");}}},
                command=commands.recv()=>match command{
                    #[cfg(test)]
                    Some(Command::CachedEntries{reply})=>{let _closed=reply.send(self.entries.len());},
                    Some(Command::Snapshot{global,reply})=>{let result=if global.still_current(){self.prune(&global);self.snapshot(&global)}else{Err(changed())};let _closed=reply.send(result);},
                    Some(Command::Refresh{source,global,reply})=>self.refresh(source,global,reply),
                    Some(Command::Set{request,global,reply})=>{let result=self.set(request,&global).await;let _closed=reply.send(result);},
                    Some(Command::Clear{reply})=>{self.invalidate_all();let _closed=reply.send(());},
                    None=>break,
                }
            }
        }
        self.invalidate_all();
        while let Some(result) = self.jobs.join_next().await {
            if result.is_err() {
                tracing::error!("a source usage worker ended unexpectedly during shutdown");
            }
        }
        self.context.drain().await;
    }
    pub(super) fn consent(&self, source: Source) -> UsageSourceConsent {
        self.grants
            .as_slice()
            .iter()
            .find(|grant| grant.id.as_str() == source.name())
            .map_or(UsageSourceConsent::Off, |grant| {
                UsageSourceConsent::Granted {
                    version: grant.version,
                }
            })
    }
    pub(super) fn enabled(&self, source: Source) -> bool {
        self.probe.supports(source.builtin())
            && matches!(self.consent(source),UsageSourceConsent::Granted{version} if version==UsageDisclosureVersion::INITIAL)
    }
    fn snapshot(&self, global: &Observation) -> Result<Snapshot, WireError> {
        let mut sources = spec::legacy(global.policy, self.legacy_supported)?;
        for source in Source::ALL {
            sources.push(source.descriptor(
                global.policy,
                self.consent(source),
                self.probe.supports(source.builtin()),
            )?);
        }
        let visible = self.entries.iter().filter(|entry| {
            global.policy == ExternalLookup::On
                && global.still_current()
                && entry.global.still_current()
                && self.enabled(entry.source)
        });
        let reports = visible
            .clone()
            .filter_map(|entry| entry.report.clone())
            .collect();
        let failures = visible.filter_map(|entry| entry.failure.clone()).collect();
        Ok(Snapshot {
            sources: sources.try_into().map_err(|_bound| changed())?,
            reports,
            failures,
            generation: self.policy_generation.child_token(),
            global: global.clone(),
        })
    }
    pub(super) fn prune(&mut self, global: &Observation) {
        if self
            .observed_global
            .as_ref()
            .is_some_and(|old| !old.still_current())
        {
            self.invalidate_all();
        }
        self.observed_global = Some(global.clone());
        self.entries.retain(|entry| {
            let keep = global.policy == ExternalLookup::On
                && global.still_current()
                && entry.global.still_current();
            if !keep {
                entry.stop.cancel();
            }
            keep
        });
    }
    pub(super) fn invalidate(&mut self, source: Source) {
        self.entries.retain(|entry| {
            if entry.source == source {
                entry.stop.cancel();
                false
            } else {
                true
            }
        });
    }
    pub(super) fn invalidate_all(&mut self) {
        self.policy_generation.cancel();
        self.policy_generation = self.stop.child_token();
        for entry in &self.entries {
            entry.stop.cancel();
        }
        self.entries.clear();
    }
}
// Observe the canonical policy owner, never a second policy engine or an RPC task.
async fn revoked(global: Option<Observation>) {
    match global {
        Some(global) => global.cancelled().await,
        None => std::future::pending().await,
    }
}
