//! Startup admission belongs to the retained task until registration settles.
use crate::workspace::{StartGuard, Starting, WorkspaceError};

#[derive(Debug)]
pub(crate) enum Admission {
    Held {
        root: StartGuard,
        _placement: Option<Starting>,
    },
    // Session persistence tests exercise the runtime without a workspace owner.
    #[cfg(test)]
    Detached,
}
impl Admission {
    pub(crate) const fn new(root: StartGuard, placement: Option<Starting>) -> Self {
        Self::Held {
            root,
            _placement: placement,
        }
    }
    pub(crate) async fn revalidate(&self) -> Result<(), WorkspaceError> {
        match self {
            Self::Held { root, .. } => root.revalidate().await,
            #[cfg(test)]
            Self::Detached => Ok(()),
        }
    }
}

// A conversion keeps platform/test absence explicit without pretending every
// production Linux admission can be unbound; the whole admission retains RAII.
impl From<&Admission> for Option<vessel_proto::checkpoint::PhysicalRoot> {
    fn from(admission: &Admission) -> Self {
        match admission {
            #[cfg(target_os = "linux")]
            Admission::Held { root, .. } => Some(root.physical()),
            #[cfg(not(target_os = "linux"))]
            Admission::Held { .. } => None,
            #[cfg(test)]
            Admission::Detached => None,
        }
    }
}
