//! Canonical metadata remains usable only while its original weak read guard is live.
//! No independent lease, client, epoch or registry identity is minted here.
use std::collections::{HashMap, VecDeque};
use vessel_app_link::DeliveryGuard;
use vessel_app_view::{
    connection::ConnectionContext,
    layout::{LookupProblem, WorkspaceLocation, WorkspaceLookup},
    machine_book::Name,
    workspace::registry_projection::{
        ProjectionRefusal, RegistryData, RegistryPhase, RegistryProjection, RegistryReadFailure,
        RegistryReadKey, WorkspaceRow,
    },
};
use vessel_proto::{NodeId, SessionId, WorkspaceId};

#[derive(Debug)]
struct Read {
    context: ConnectionContext,
    guard: DeliveryGuard,
}

/// The app retains canonical display values and the exact weak observation that admitted them.
#[derive(Debug, Default)]
pub(crate) struct RegistryCache {
    model: RegistryProjection,
    reads: HashMap<Name, Read>,
    queued: VecDeque<ConnectionContext>,
    active: Vec<RegistryReadKey>,
}
impl RegistryCache {
    fn begin(&mut self, context: ConnectionContext) -> Result<RegistryReadKey, ProjectionRefusal> {
        let key = self.model.begin(context.clone())?;
        if let Ok(host) = Name::parse(context.machine().as_str()) {
            if self
                .reads
                .get(&host)
                .is_some_and(|read| read.context != context)
            {
                let _old = self.reads.remove(&host);
            }
        }
        Ok(key)
    }

    fn finish(
        &mut self,
        key: &RegistryReadKey,
        guard: Option<DeliveryGuard>,
        result: Result<RegistryData, RegistryReadFailure>,
    ) -> Result<bool, ProjectionRefusal> {
        let context = key.context();
        let (guard, result) = match guard {
            Some(guard) if guard.validate(context).is_ok() => (Some(guard), result),
            Some(_) => (None, Err(RegistryReadFailure::Offline)),
            None if result.is_ok() => (None, Err(RegistryReadFailure::Refused)),
            None => (None, result),
        };
        let applied = self.model.finish(key, result);
        match applied {
            Ok(false) => Ok(false),
            Ok(true) => {
                let host = Name::parse(context.machine().as_str())
                    .map_err(|_invalid| ProjectionRefusal::Invalid)?;
                if self.model.phase(context) == Some(RegistryPhase::Ready) {
                    if let Some(guard) = guard {
                        let _old = self.reads.insert(
                            host,
                            Read {
                                context: context.clone(),
                                guard,
                            },
                        );
                    } else {
                        self.remove_read(context);
                    }
                } else {
                    self.remove_read(context);
                }
                Ok(true)
            }
            Err(why) => {
                self.remove_read(context);
                Err(why)
            }
        }
    }

    fn remove_read(&mut self, context: &ConnectionContext) {
        if let Ok(host) = Name::parse(context.machine().as_str()) {
            if self
                .reads
                .get(&host)
                .is_some_and(|read| read.context == *context)
            {
                let _old = self.reads.remove(&host);
            }
        }
    }

    fn require_read(&self, context: &ConnectionContext) -> Result<(), LookupProblem> {
        let host = Name::parse(context.machine().as_str())
            .map_err(|_invalid| LookupProblem::Unavailable)?;
        let read = self.reads.get(&host).ok_or(LookupProblem::Unavailable)?;
        if read.context != *context {
            return Err(LookupProblem::Unavailable);
        }
        read.guard
            .validate(context)
            .map_err(|_retired| LookupProblem::Unavailable)
    }

    pub(crate) fn phase(&self, context: &ConnectionContext) -> Option<RegistryPhase> {
        match self.model.phase(context) {
            Some(RegistryPhase::Ready) if self.require_read(context).is_err() => {
                Some(RegistryPhase::Offline)
            }
            phase => phase,
        }
    }

    pub(crate) fn rows(
        &self,
        context: &ConnectionContext,
    ) -> Result<impl Iterator<Item = WorkspaceRow<'_>>, LookupProblem> {
        let rows = self.model.rows(context)?;
        self.require_read(context)?;
        Ok(rows)
    }

    pub(crate) fn reconcile(&mut self, current: &[ConnectionContext]) {
        self.model.reconcile(current);
        self.reads.retain(|_, read| current.contains(&read.context));
        self.active.retain(|key| current.contains(key.context()));
        self.queued.retain(|context| current.contains(context));
        for context in current
            .iter()
            .take(vessel_app_view::workspace::registry_projection::MAX_REGISTRY_HOSTS)
        {
            if !self.active.iter().any(|key| key.context() == context)
                && !self.queued.contains(context)
            {
                self.queued.push_back(context.clone());
            }
        }
    }

    /// Rotate admitted hosts rather than letting four long polls starve initial reads.
    pub(crate) fn next_read(
        &mut self,
    ) -> Option<vessel_app_link::workspace::registry_reads::Request> {
        use vessel_app_view::workspace::registry_projection::MAX_REGISTRY_READS;
        if self.active.len() >= MAX_REGISTRY_READS {
            return None;
        }
        while let Some(context) = self.queued.pop_front() {
            let changed_from = self
                .rows(&context)
                .ok()
                .and_then(|_rows| self.model.changed_from(&context));
            if let Ok(key) = self.begin(context) {
                self.active.push(key.clone());
                return Some(vessel_app_link::workspace::registry_reads::Request {
                    key,
                    changed_from,
                });
            }
        }
        None
    }

    pub(crate) fn receive(
        &mut self,
        response: vessel_app_link::workspace::registry_reads::Response,
    ) {
        response.receive(|key, guard, result| {
            let Some(index) = self.active.iter().position(|pending| *pending == key) else {
                return;
            };
            let _settled = self.active.remove(index);
            let _outcome = self.finish(&key, guard, result);
            // Errors settle this host. A subsequent fleet read or explicit refresh
            // may retry; an immediate refusal must not become an unbounded loop.
            if self.phase(key.context()) == Some(RegistryPhase::Ready)
                && !self.queued.contains(key.context())
            {
                self.queued.push_back(key.context().clone());
            }
        });
    }
    pub(crate) fn forget(&mut self, host: &Name) {
        self.model.forget(host);
        let _old = self.reads.remove(host);
        self.active
            .retain(|key| key.context().machine().as_str() != host.as_str());
        self.queued
            .retain(|context| context.machine().as_str() != host.as_str());
    }
}
impl WorkspaceLookup for RegistryCache {
    fn session_workspace(
        &self,
        node: NodeId,
        session: SessionId,
    ) -> Result<Option<WorkspaceId>, LookupProblem> {
        self.require_read(self.model.read_context(node)?)?;
        self.model.session_workspace(node, session)
    }

    fn workspace_location(
        &self,
        node: NodeId,
        workspace: WorkspaceId,
    ) -> Result<WorkspaceLocation, LookupProblem> {
        self.require_read(self.model.read_context(node)?)?;
        self.model.workspace_location(node, workspace)
    }
}

#[cfg(test)]
mod tests;
