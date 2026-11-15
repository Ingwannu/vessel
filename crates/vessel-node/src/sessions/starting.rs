//! Startup admission belongs to the retained task until registration settles.
use crate::workspace::{StartGuard, Starting, WorkspaceError};

#[derive(Debug)]
pub(crate) enum Admission {
    Held {
        root: StartGuard,
        placement: Option<Starting>,
    },
    // Session persistence tests exercise the runtime without a workspace owner.
    #[cfg(test)]
    Detached,
}
impl Admission {
    pub(crate) fn physical(&self) -> Option<vessel_proto::checkpoint::PhysicalRoot> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Held { root, .. } => Some(root.physical()),
            #[cfg(not(target_os = "linux"))]
            Self::Held { .. } => None,
            #[cfg(test)]
            Self::Detached => None,
        }
    }
    pub(crate) const fn new(root: StartGuard, placement: Option<Starting>) -> Self {
        Self::Held { root, placement }
    }
    pub(crate) async fn revalidate(&self) -> Result<(), WorkspaceError> {
        match self {
            Self::Held { root, placement } => {
                // Borrow the count alongside the root: both survive caller loss.
                let _placement = placement;
                root.revalidate().await
            }
            #[cfg(test)]
            Self::Detached => Ok(()),
        }
    }
}
