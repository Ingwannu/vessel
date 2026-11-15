//! Finite source loading retries and append-only CAS, with no blind replay.

use super::super::{LaunchProfileClientError, LaunchProfileSupport};
use super::{LegacyCatalogueState, MigrationCatalogue, MigrationCatalogues, build_migration_plan};
use crate::{CallError, NodeClient};
use std::{collections::HashSet, future::Future, sync::Arc, time::Duration};
use vessel_proto::launch_profiles::{
    LaunchProfileRefusal, LaunchProfileRevision, LaunchProfiles, LaunchProfilesSnapshot,
    LegacyFavorites, LegacyProviderKey,
};
use vessel_proto::{ErrorCode, ErrorDetail, LaunchProfileId, Method, NodeId};

/// Source delays, only for explicit loading/busy states; one final read follows.
const CATALOGUE_LOADING_DELAYS: [Duration; 6] = [
    Duration::from_millis(100),
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_millis(1_000),
    Duration::from_millis(2_000),
    Duration::from_millis(4_000),
];
/// Conflicts rebuild from a fresh whole list, with a finite contention budget.
const MAX_CAS_ATTEMPTS: usize = 4;

/// Known host completion; none of these states imply an app marker was persisted.
#[derive(Debug, Clone)]
pub enum LegacyMigrationCompletion {
    /// No compatibility inputs: no host read, catalogue read or host write.
    NoFavorites,
    /// All inputs already have ids or model selections on this host.
    AlreadyPresent {
        /// Revision actually read while reconciling.
        revision: LaunchProfileRevision,
    },
    /// The append-only whole list was acknowledged by the host.
    Saved {
        /// Acknowledged revision, not a guessed local increment.
        revision: LaunchProfileRevision,
        /// Identities appended by this attempt.
        added: Arc<[LaunchProfileId]>,
    },
}

/// The app may persist its completion marker for this actual responding node.
/// There is no public constructor that fabricates a successful host receipt.
#[derive(Debug, Clone)]
#[must_use]
pub struct LegacyMigrationReceipt {
    node: NodeId,
    completion: LegacyMigrationCompletion,
}
impl LegacyMigrationReceipt {
    /// Actual authenticated welcome identity, never a mutable machine/book label.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
    /// Known completion; app marker write failure must remain retryable.
    #[must_use]
    pub const fn completion(&self) -> &LegacyMigrationCompletion {
        &self.completion
    }
}

impl NodeClient {
    /// Convert validated app inputs through real catalogue mappings and ordinary
    /// host list/CAS. The app retains its per-node marker and in-flight owner;
    /// this helper never reads a preference, credential or account directory.
    ///
    /// # Errors
    /// Missing capabilities, ordinary catalogue/auth/provider errors, bounded
    /// loading/contended CAS exhaustion, unresolved choices and unknown write
    /// acknowledgements. A marker must remain unset after any error.
    pub async fn migrate_legacy_favorites<Load, Loaded>(
        &self,
        favorites: LegacyFavorites,
        load: Load,
    ) -> Result<LegacyMigrationReceipt, LaunchProfileClientError>
    where
        Load: FnMut(LegacyProviderKey) -> Loaded,
        Loaded: Future<Output = Result<LegacyCatalogueState, LaunchProfileClientError>>,
    {
        match self.launch_profile_support() {
            LaunchProfileSupport::Unassigned => {
                return Err(CallError::UnassignedProtocol {
                    method: Method::ListLaunchProfiles,
                }
                .into());
            }
            LaunchProfileSupport::Older { needed, negotiated } => {
                return Err(CallError::NotInProtocol {
                    method: Method::ListLaunchProfiles,
                    needs: needed,
                    negotiated,
                }
                .into());
            }
            LaunchProfileSupport::Available => {}
        }
        execute(
            self.node().id,
            favorites,
            load,
            || self.launch_profiles(),
            |revision, list| self.replace_launch_profiles(revision, list),
        )
        .await
    }
}

// Generic closures exercise the same mutation state machine without adding a
// single-implementation host trait or a production transport/version bypass.
async fn execute<Load, Loaded, Read, ReadFuture, Write, WriteFuture>(
    node: NodeId,
    favorites: LegacyFavorites,
    mut load: Load,
    mut read: Read,
    mut write: Write,
) -> Result<LegacyMigrationReceipt, LaunchProfileClientError>
where
    Load: FnMut(LegacyProviderKey) -> Loaded,
    Loaded: Future<Output = Result<LegacyCatalogueState, LaunchProfileClientError>>,
    Read: FnMut() -> ReadFuture,
    ReadFuture: Future<Output = Result<LaunchProfilesSnapshot, CallError>>,
    Write: FnMut(LaunchProfileRevision, LaunchProfiles) -> WriteFuture,
    WriteFuture: Future<Output = Result<LaunchProfilesSnapshot, CallError>>,
{
    if favorites.as_slice().is_empty() {
        return Ok(LegacyMigrationReceipt {
            node,
            completion: LegacyMigrationCompletion::NoFavorites,
        });
    }
    let catalogues = load_catalogues(&favorites, &mut load).await?;
    let mut last_stale = None;
    for _ in 0..MAX_CAS_ATTEMPTS {
        let current = read().await?;
        let plan = build_migration_plan(node, &current, &favorites, &catalogues)?;
        if plan.added().is_empty() {
            return Ok(LegacyMigrationReceipt {
                node,
                completion: LegacyMigrationCompletion::AlreadyPresent {
                    revision: current.revision(),
                },
            });
        }
        match write(plan.expected(), plan.profiles().clone()).await {
            Ok(saved) => {
                // Added records require this exact CAS increment. Matching
                // content with a stale/ahead revision is not its write ack.
                if plan.expected().next() != Some(saved.revision())
                    || saved.profiles() != plan.profiles()
                {
                    return Err(LaunchProfileClientError::LegacyWriteNotConfirmed);
                }
                return Ok(LegacyMigrationReceipt {
                    node,
                    completion: LegacyMigrationCompletion::Saved {
                        revision: saved.revision(),
                        added: Arc::from(plan.added()),
                    },
                });
            }
            Err(error) if stale(&error) => {
                last_stale = Some(error);
            }
            Err(error @ (CallError::Disconnected | CallError::UnexpectedReply(_))) => {
                return reconcile(node, &favorites, &catalogues, error, &mut read).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(LaunchProfileClientError::LegacyConflictBudget { last: last_stale })
}

async fn reconcile<Read, ReadFuture>(
    node: NodeId,
    favorites: &LegacyFavorites,
    catalogues: &MigrationCatalogues,
    write: CallError,
    read: &mut Read,
) -> Result<LegacyMigrationReceipt, LaunchProfileClientError>
where
    Read: FnMut() -> ReadFuture,
    ReadFuture: Future<Output = Result<LaunchProfilesSnapshot, CallError>>,
{
    // A lost ack is not a failed transaction. Read once to prove every
    // pair/id is present; otherwise return uncertainty without another write.
    // Preserve both the original write error and any reconciliation failure.
    let current = match read().await {
        Ok(current) => current,
        Err(error) => {
            return Err(LaunchProfileClientError::LegacyUnknownWrite {
                write,
                reconciliation: Some(Box::new(error.into())),
            });
        }
    };
    match build_migration_plan(node, &current, favorites, catalogues) {
        Ok(remaining) if remaining.added().is_empty() => Ok(LegacyMigrationReceipt {
            node,
            completion: LegacyMigrationCompletion::AlreadyPresent {
                revision: current.revision(),
            },
        }),
        Ok(_remaining) => Err(LaunchProfileClientError::LegacyUnknownWrite {
            write,
            reconciliation: None,
        }),
        Err(error) => Err(LaunchProfileClientError::LegacyUnknownWrite {
            write,
            reconciliation: Some(Box::new(error)),
        }),
    }
}

fn stale(error: &CallError) -> bool {
    matches!(error, CallError::Remote(error) if error.code == ErrorCode::Conflict && matches!(error.detail, Some(ErrorDetail::LaunchProfile { problem: LaunchProfileRefusal::StaleRevision { .. } })))
}

async fn load_catalogues<Load, Loaded>(
    favorites: &LegacyFavorites,
    load: &mut Load,
) -> Result<MigrationCatalogues, LaunchProfileClientError>
where
    Load: FnMut(LegacyProviderKey) -> Loaded,
    Loaded: Future<Output = Result<LegacyCatalogueState, LaunchProfileClientError>>,
{
    let mut providers = HashSet::new();
    let mut ready = Vec::new();
    for favorite in favorites.as_slice() {
        if !providers.insert(favorite.provider().clone()) {
            continue;
        }
        let catalogue = load_one(favorite.provider(), load).await?;
        if catalogue.provider() != favorite.provider() {
            return Err(LaunchProfileClientError::LegacyCatalogueMismatch);
        }
        ready.push(catalogue);
    }
    ready.try_into()
}

async fn load_one<Load, Loaded>(
    provider: &LegacyProviderKey,
    load: &mut Load,
) -> Result<MigrationCatalogue, LaunchProfileClientError>
where
    Load: FnMut(LegacyProviderKey) -> Loaded,
    Loaded: Future<Output = Result<LegacyCatalogueState, LaunchProfileClientError>>,
{
    for delay in CATALOGUE_LOADING_DELAYS {
        match load(provider.clone()).await? {
            LegacyCatalogueState::Ready(catalogue) => return Ok(catalogue),
            // This is the reference's external readiness backoff, never a
            // sleep used to coordinate internal task completion.
            LegacyCatalogueState::Loading | LegacyCatalogueState::Busy => {
                tokio::time::sleep(delay).await;
            }
        }
    }
    match load(provider.clone()).await? {
        LegacyCatalogueState::Ready(catalogue) => Ok(catalogue),
        LegacyCatalogueState::Loading | LegacyCatalogueState::Busy => {
            Err(LaunchProfileClientError::LegacyCatalogueLoading)
        }
    }
}

#[cfg(test)]
mod tests;
