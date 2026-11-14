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
                let binding = associated(self.sessions.limit_association(session));
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
            && associated(self.sessions.limit_association(session)) != before
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
    pub(crate) async fn set_usage_lookup(
        &self,
        _connection: &Connection,
        request: SetUsageLookup,
    ) -> Result<ExternalLookup, WireError> {
        self.limits.policy(request.policy).await
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
