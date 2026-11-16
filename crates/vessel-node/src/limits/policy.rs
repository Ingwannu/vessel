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
    /// Actual admitted work observes the same revocation as cached publication.
    pub(crate) async fn cancelled(&self) {
        self.generation.cancelled().await;
    }
}
impl Limits {
    /// Pure actor/cache read; never touches a credential, policy file or vendor.
    pub(crate) async fn observation(&self) -> Result<Observation, WireError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::Observe { reply })
            .map_err(|_backpressure| busy())?;
        response.await.map_err(|_closed| stopped())
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
    #[tokio::test]
    async fn both_off_and_on_have_fresh_observations_and_revoke_the_previous_generation() {
        let root = tempfile::tempdir().unwrap();
        let stop = CancellationToken::new();
        let tasks = tokio_util::task::TaskTracker::new();
        let limits = Limits::start(
            vessel_store::DataDir::new(root.path()),
            vessel_harness::LimitProbe::Disabled,
            stop.clone(),
            &tasks,
            vessel_harness::LimitContext::new(),
        )
        .await;
        let initial = limits.observation().await.unwrap();
        assert_eq!(initial.policy, ExternalLookup::Off);
        assert!(initial.still_current());
        assert_eq!(
            limits.policy(ExternalLookup::On).await.unwrap(),
            ExternalLookup::On
        );
        assert!(!initial.still_current());
        let on = limits.observation().await.unwrap();
        assert!(on.still_current());
        assert_eq!(
            limits.policy(ExternalLookup::Off).await.unwrap(),
            ExternalLookup::Off
        );
        assert!(!on.still_current());
        let off = limits.observation().await.unwrap();
        assert_eq!(off.policy, ExternalLookup::Off);
        assert!(off.still_current());
        assert_eq!(
            limits.policy(ExternalLookup::On).await.unwrap(),
            ExternalLookup::On
        );
        assert!(!off.still_current());
        stop.cancel();
        let _closed = tasks.close();
        tasks.wait().await;
    }
}
