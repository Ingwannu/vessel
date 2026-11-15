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
