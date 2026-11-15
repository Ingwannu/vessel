//! Destination scope checks and launch-bound account selection. No
//! credential, vendor schema or endpoint knowledge crosses this boundary.
use super::{Connection, NodeService};
use vessel_harness::LimitBinding;
use vessel_proto::{
    ErrorCode, Method, WireError,
    limits::{
        ExternalLookup, GetUsageLimits, LimitAccount, LimitFailure, LimitRefresh, LimitsScope,
        SetUsageLookup, UsageLimits,
    },
    usage::CostVisibility,
};
use vessel_session::LimitAssociation;
impl NodeService {
    pub(crate) async fn get_usage_limits(
        &self,
        connection: &Connection,
        request: GetUsageLimits,
    ) -> Result<UsageLimits, WireError> {
        let principal = connection.principal();
        if let Some((node, own)) = principal.agent()
            && (node != self.lifecycle.info().id
                || !matches!(request.scope,LimitsScope::Session { session } if session==own))
        {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "an agent may read only limits of its own local session",
            ));
        }
        if request.refresh == LimitRefresh::Refresh {
            principal.permit(Method::CreateSession)?;
        }
        let (account, binding, session) = match request.scope {
            LimitsScope::Account { account } => {
                let profile = match &account.profile {
                    Some(name) => self.profiles.get(name),
                    None => self.profiles.default_of(&account.harness),
                }
                .filter(|profile| profile.harness() == account.harness)
                .ok_or_else(|| {
                    WireError::new(
                        ErrorCode::NotFound,
                        "that agent profile is no longer configured; choose a current profile",
                    )
                })?;
                let binding = self.launcher.account_binding(&self.profiles, &profile);
                (
                    LimitAccount {
                        harness: profile.harness(),
                        profile: Some(profile.name().clone()),
                    },
                    binding,
                    None,
                )
            }
            LimitsScope::Session { session } => {
                let info = self
                    .sessions
                    .get(session)
                    .await
                    .map_err(|error| super::wire(&error))?;
                let binding = self.session_limit_binding(session).await;
                let account = binding.as_ref().map_or(
                    LimitAccount {
                        harness: info.spec.harness,
                        profile: info.profile,
                    },
                    |binding| binding.account().clone(),
                );
                (account, binding, Some(session))
            }
        };
        let before = binding.clone();
        let mut report = self.limits.read(account, binding, request.refresh).await?;
        if let Some(session) = session
            && self.session_limit_binding(session).await != before
        {
            return Err(WireError::new(
                ErrorCode::Conflict,
                "this session changed accounts during the check; refresh its limits again",
            ));
        }
        if principal.permit(Method::CreateSession).is_err() {
            report.cost_visibility = CostVisibility::Withheld;
            redact(&mut report.state);
        }
        Ok(report)
    }
    // The saved OpenAI store of a multi-provider agent belongs to a session
    // only while its current model identifies OpenAI. Recheck on both sides
    // of awaiting a probe, just as for a concurrently moved profile.
    pub(super) async fn session_limit_binding(
        &self,
        session: vessel_proto::SessionId,
    ) -> Result<LimitBinding, LimitFailure> {
        let binding = associated(self.sessions.limit_association(session));
        if let Ok(chosen) = &binding
            && chosen.requires_model_evidence()
        {
            // An absent/ended/moving options snapshot cannot prove a provider;
            // use the typed unavailable association instead of a default model.
            let options = match self.sessions.options(session).await {
                Ok(options) => options,
                Err(_unavailable) => return Err(LimitFailure::Unassociated),
            };
            chosen.check_session_model(&options)?;
        }
        binding
    }
    pub(crate) async fn set_usage_lookup(
        &self,
        _connection: &Connection,
        request: SetUsageLookup,
    ) -> Result<ExternalLookup, WireError> {
        let result = self.limits.policy(request.policy).await;
        self.usage_reports.forget_cancelled().await;
        let sources = if request.policy == ExternalLookup::Off {
            self.limits.sources.clear().await
        } else {
            Ok(())
        };
        let policy = result?;
        sources?;
        Ok(policy)
    }
}
fn associated(association: LimitAssociation) -> Result<LimitBinding, LimitFailure> {
    match association {
        LimitAssociation::Bound(binding) => Ok(binding),
        LimitAssociation::Refused(reason) => Err(reason),
        LimitAssociation::Unavailable => Err(LimitFailure::Unassociated),
    }
}
fn redact(state: &mut vessel_proto::limits::AccountLimitState) {
    use vessel_proto::limits::AccountLimitState;
    let snapshot = match state {
        AccountLimitState::Ready { snapshot } => Some(snapshot),
        AccountLimitState::Refreshing { retained } | AccountLimitState::Failed { retained, .. } => {
            retained.as_mut()
        }
        AccountLimitState::Off | AccountLimitState::NotRequested => None,
    };
    if let Some(snapshot) = snapshot
        && let Some(credits) = &mut snapshot.credits
    {
        credits.balance = None;
    }
}
