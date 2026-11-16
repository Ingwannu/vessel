//! Fixed typed claims are validated again when a durable receipt is decoded.

use crate::StoreError;
use rusqlite::{Connection, OptionalExtension as _, params};
use vessel_proto::creation::{CreationError, Receipt, Reservation, Resource, WorkspaceClaim};
use vessel_proto::{SessionId, TerminalId, WorkspaceId};

#[derive(Clone, Copy)]
enum Claim {
    Workspace(WorkspaceId),
    Agent(SessionId),
    Terminal(TerminalId),
}
impl Claim {
    fn parts(self) -> (&'static str, String) {
        match self {
            Self::Workspace(id) => ("workspace", id.to_string()),
            Self::Agent(id) => ("agent", id.to_string()),
            Self::Terminal(id) => ("terminal", id.to_string()),
        }
    }
}
fn required(reserved: &Reservation) -> impl Iterator<Item = Claim> {
    let workspace = match reserved.workspace_claim {
        WorkspaceClaim::Create => Some(Claim::Workspace(reserved.workspace)),
        WorkspaceClaim::Existing => None,
    };
    let resource = reserved.resource.map(|resource| match resource {
        Resource::Agent { session } => Claim::Agent(session),
        Resource::Terminal { terminal } => Claim::Terminal(terminal),
    });
    [workspace, resource].into_iter().flatten()
}
fn owner(db: &Connection, claim: Claim) -> Result<Option<String>, StoreError> {
    let (kind, resource) = claim.parts();
    db.query_row(
        "SELECT operation FROM creation_claims WHERE kind=?1 AND resource=?2",
        params![kind, resource],
        |row| row.get(0),
    )
    .optional()
    .map_err(StoreError::from)
}
pub(super) fn reserve(db: &Connection, reserved: &Reservation) -> Result<(), StoreError> {
    for claim in required(reserved) {
        if owner(db, claim)?.is_some() {
            return Err(CreationError::ResourceClaimed.into());
        }
        if let Claim::Agent(session) = claim
            && db
                .query_row(
                    "SELECT 1 FROM sessions WHERE id=?1",
                    [session.to_string()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some()
        {
            return Err(CreationError::ResourceClaimed.into());
        }
        let (kind, resource) = claim.parts();
        let _changed = db.execute(
            "INSERT INTO creation_claims(kind,resource,operation) VALUES (?1,?2,?3)",
            params![kind, resource, reserved.operation.to_string()],
        )?;
    }
    Ok(())
}
pub(super) fn validate(db: &Connection, receipt: &Receipt) -> Result<(), StoreError> {
    let reserved = &receipt.snapshot().reserved;
    for claim in required(reserved) {
        if owner(db, claim)?.as_deref() != Some(reserved.operation.to_string().as_str()) {
            return Err(CreationError::ClaimsCorrupt.into());
        }
    }
    Ok(())
}
