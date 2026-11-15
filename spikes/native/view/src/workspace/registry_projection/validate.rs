//! Reject incoherent peer metadata before exposing lookup authority.
use super::{ConnectionContext, ProjectionRefusal, RegistryData};
use std::collections::HashSet;

pub(super) fn data(
    context: &ConnectionContext,
    data: &RegistryData,
) -> Result<(), ProjectionRefusal> {
    if data.registry.node != context.node() {
        return Err(ProjectionRefusal::Invalid);
    }
    let mut workspaces = HashSet::new();
    for workspace in &data.registry.workspaces {
        if !workspaces.insert(workspace.id) || workspace.revision > data.registry.revision {
            return Err(ProjectionRefusal::Invalid);
        }
    }
    let mut resources = HashSet::new();
    for member in &data.registry.memberships {
        if !workspaces.contains(&member.workspace) || !resources.insert(member.resource) {
            return Err(ProjectionRefusal::Invalid);
        }
    }
    let mut projects = HashSet::new();
    for project in &data.projects.projects {
        if !projects.insert(project.id) {
            return Err(ProjectionRefusal::Invalid);
        }
    }
    let mut worktrees = HashSet::new();
    for worktree in &data.worktrees.worktrees {
        if !worktrees.insert(worktree.id) {
            return Err(ProjectionRefusal::Invalid);
        }
    }
    Ok(())
}

/// Measure bounded encoded content without allocating a second serialized snapshot.
pub(super) fn bytes(data: &RegistryData, limit: usize) -> Result<usize, ProjectionRefusal> {
    let mut budget = Budget {
        bytes: 0,
        limit,
        full: false,
    };
    let result = serde_json::to_writer(
        &mut budget,
        &(&data.registry, &data.projects, &data.worktrees),
    );
    match result {
        Ok(()) => Ok(budget.bytes),
        Err(_encode) if budget.full => Err(ProjectionRefusal::Full),
        Err(_encode) => Err(ProjectionRefusal::Invalid),
    }
}

struct Budget {
    bytes: usize,
    limit: usize,
    full: bool,
}
impl std::io::Write for Budget {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let Some(next) = self
            .bytes
            .checked_add(input.len())
            .filter(|next| *next <= self.limit)
        else {
            self.full = true;
            return Err(std::io::Error::other(
                "workspace metadata exceeds its cache budget",
            ));
        };
        self.bytes = next;
        Ok(input.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
