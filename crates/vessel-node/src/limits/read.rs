//! Generic reports retain their captured policy through actual actor admission.
//! Direct legacy reads deliberately observe policy when the actor admits them.
use super::{Command, Limits, busy, policy::Observation, stopped};
use tokio::sync::oneshot;
use vessel_harness::LimitBinding;
use vessel_proto::{
    WireError,
    limits::{LimitAccount, LimitFailure, LimitRefresh, UsageLimits},
};
impl Limits {
    pub(crate) async fn read(
        &self,
        account: LimitAccount,
        binding: Result<LimitBinding, LimitFailure>,
        refresh: LimitRefresh,
    ) -> Result<UsageLimits, WireError> {
        self.read_captured(account, binding, refresh, None).await
    }
    pub(crate) async fn read_observed(
        &self,
        account: LimitAccount,
        binding: Result<LimitBinding, LimitFailure>,
        refresh: LimitRefresh,
        observed: &Observation,
    ) -> Result<UsageLimits, WireError> {
        self.read_captured(account, binding, refresh, Some(observed.clone()))
            .await
    }
    async fn read_captured(
        &self,
        account: LimitAccount,
        binding: Result<LimitBinding, LimitFailure>,
        refresh: LimitRefresh,
        observed: Option<Observation>,
    ) -> Result<UsageLimits, WireError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::Read {
                account,
                binding,
                refresh,
                observed,
                reply,
            })
            .map_err(|_full| busy())?;
        response.await.map_err(|_closed| stopped())?
    }
}
