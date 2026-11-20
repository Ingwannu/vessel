//! Authority comes from the current registry and hardened git remote,
//! never a cursor. Credential selection precedes every cache lookup and
//! is checked again after network work before a page can be published.
use super::super::hosts::{Hosts, denied};
use super::{Error, Request, cache::Key};
use crate::workspace::Workspaces;
use std::sync::Arc;
use vessel_forge::{Forge, Priority, remote};
use vessel_proto::{
    forge::{ForgeAccess, ForgeKind, OpenPullPage, RepoRef},
    workspace::WorkspaceRoot,
};

#[derive(Debug)]
pub(super) struct Target {
    pub(super) key: Key,
    // Forge is the existing provider extension point, not a test-only trait.
    forge: Arc<dyn Forge>,
}

pub(super) async fn prepare(
    workspaces: &Workspaces,
    hosts: &Hosts,
    request: &Request,
) -> Result<Target, Error> {
    let resolved = workspaces
        .resolve_git_guarded(
            WorkspaceRoot::Project {
                project: request.project,
            },
            Arc::clone(&request.keep),
        )
        .await
        .map_err(Error::Workspace)?;
    let folder = resolved.root.path();
    let checkout = workspaces
        .checkout(folder)
        .await
        .map_err(Error::Workspace)?;
    let (_, address) = workspaces
        .project_remote_address(folder, checkout.branch.as_ref())
        .await
        .map_err(Error::Workspace)?
        .ok_or(Error::Access(ForgeAccess::NoRemote))?;
    let parsed = remote::parse(address.expose()).ok_or(Error::Access(ForgeAccess::NoRemote))?;
    let repo = RepoRef {
        kind: ForgeKind::Github,
        host: parsed.host,
        path: parsed.path,
    };
    if request
        .cursor
        .as_ref()
        .is_some_and(|cursor| cursor.repo != repo)
    {
        return Err(Error::CursorRepository);
    }
    let access = hosts.access(&repo.host).await;
    if !matches!(access, ForgeAccess::Ready { .. }) {
        return Err(Error::Access(access));
    }
    let credential = hosts
        .project_identity(&repo.host)
        .await
        .ok_or(Error::CredentialChanged)?;
    let forge = hosts.forge().ok_or_else(|| {
        Error::Access(ForgeAccess::NotSupported {
            host: repo.host.clone(),
        })
    })?;
    Ok(Target {
        key: Key {
            repo,
            after: request.cursor.as_ref().map(|cursor| cursor.after.clone()),
            credential,
        },
        forge,
    })
}

pub(super) async fn load(hosts: &Hosts, target: &Target) -> Result<OpenPullPage, Error> {
    let repo = &target.key.repo;
    let page = target
        .forge
        .list_open(repo, target.key.after.as_ref(), Priority::Person)
        .await
        .map_err(|source| Error::Provider {
            access: denied(&repo.host, &source, None),
            source,
        })?;
    if hosts.project_identity(&repo.host).await.as_ref() != Some(&target.key.credential) {
        return Err(Error::CredentialChanged);
    }
    if page.repo() != repo
        || page
            .continuation()
            .as_ref()
            .is_some_and(|next| Some(&next.after) == target.key.after.as_ref())
    {
        return Err(Error::InconsistentPage);
    }
    Ok(page)
}
