//! Plans bind the destination's complete authenticated human, never a request field.
use super::Principal;
use vessel_proto::{
    ErrorCode, ErrorDetail, OriginBy, WireError,
    checkpoint::{CheckpointError, CheckpointHumanOrigin, CheckpointOwner},
};

impl Principal {
    pub(crate) fn checkpoint_owner(&self) -> Result<CheckpointOwner, WireError> {
        match self {
            Self::Owner => Ok(CheckpointOwner::LocalOwner),
            Self::Device { id, .. } => Ok(CheckpointOwner::Device { device: *id }),
            Self::Agent { .. } => Err(forbidden()),
            Self::Delegated { device, origin, .. } => {
                let human = match origin.by {
                    OriginBy::Owner => CheckpointHumanOrigin::Owner,
                    OriginBy::Device { id } => CheckpointHumanOrigin::Device { device: id },
                    OriginBy::Agent { .. } => return Err(forbidden()),
                };
                Ok(CheckpointOwner::DelegatedHuman {
                    delivering: *device,
                    origin_node: origin.node,
                    origin: human,
                })
            }
        }
    }
}
fn forbidden() -> WireError {
    WireError {
        code: ErrorCode::Forbidden,
        message: "only an authenticated human principal may confirm file restoration".to_owned(),
        detail: Some(ErrorDetail::Checkpoint {
            reason: CheckpointError::Forbidden,
        }),
    }
}

#[cfg(test)]
mod tests;
