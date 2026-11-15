//! A cached policy observation carries the actor's existing cancellation generation.
//! A list spanning off/on must refuse publication even when its final policy is on.
use super::{Command, Limits, busy, stopped};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use vessel_proto::{WireError, limits::ExternalLookup};
#[derive(Debug, Clone)]
pub(crate) struct Observation {
    pub(crate) policy: ExternalLookup,
    pub(super) generation: CancellationToken,
}
impl Observation {
    #[cfg(test)]
    pub(crate) fn fixture(generation: CancellationToken) -> Self {
        Self {
            policy: ExternalLookup::On,
            generation,
        }
    }
    pub(crate) fn still_current(&self) -> bool {
        !self.generation.is_cancelled()
    }
}
impl Limits {
    /// Pure actor/cache read; never touches a credential, policy file or vendor.
    pub(crate) async fn observation(&self) -> Result<Observation, WireError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::Observe { reply })
            .map_err(|_| busy())?;
        response.await.map_err(|_| stopped())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_off_on_cycle_does_not_revalidate_a_prior_on_observation() {
        let generation = CancellationToken::new();
        let before = Observation {
            policy: ExternalLookup::On,
            generation: generation.child_token(),
        };
        assert!(before.still_current());
        generation.cancel();
        let after = Observation {
            policy: ExternalLookup::On,
            generation: CancellationToken::new(),
        };
        assert!(!before.still_current());
        assert!(after.still_current());
    }
}
