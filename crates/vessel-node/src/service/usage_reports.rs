//! Generic report selectors reuse saved-login policy and immutable session proofs.
use super::{Connection, NodeService};
use crate::{
    limits::policy::Observation,
    usage_reports::{self, Binding},
};
use vessel_proto::{
    ErrorCode, HarnessRef, Method, WireError,
    limits::{ExternalLookup, GetUsageLimits, LimitAccount, LimitRefresh, LimitsScope},
    usage::CostVisibility,
    usage_reports::{
        InspectUsageReport, ListUsageReports, MAX_USAGE_REPORTS, ReportEntries,
        ReportSourceFailures, SourceReportEntry, UsageReportLookup, UsageReportScope, UsageReports,
    },
};

impl NodeService {
    pub(crate) async fn list_usage_reports(
        &self,
        connection: &Connection,
        request: ListUsageReports,
    ) -> Result<UsageReports, WireError> {
        self.report_permission(connection, &request.scope, request.refresh)?;
        let policy = self.limits.observation().await?;
        let visibility = visibility(connection);
        if policy.policy == ExternalLookup::Off {
            return complete(Vec::new(), ExternalLookup::Off, visibility);
        }
        let scopes = match request.scope {
            UsageReportScope::Session { session } => vec![LimitsScope::Session { session }],
            UsageReportScope::Machine => self
                .profiles
                .list(None)
                .into_iter()
                .filter(|profile| usage_reports::source(&profile.harness).is_some())
                .map(|profile| LimitsScope::Account {
                    account: LimitAccount {
                        harness: profile.harness,
                        profile: Some(profile.name),
                    },
                })
                .collect(),
        };
        if scopes.len() > MAX_USAGE_REPORTS {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "there are too many configured usage accounts; none were omitted",
            ));
        }
        let mut reports = Vec::with_capacity(scopes.len());
        for scope in scopes {
            reports.push(
                self.saved_report(connection, scope, request.refresh, &policy)
                    .await?,
            );
        }
        if !policy.still_current() {
            return Err(changed());
        }
        complete(reports, policy.policy, visibility)
    }
    pub(crate) async fn inspect_usage_report(
        &self,
        connection: &Connection,
        request: InspectUsageReport,
    ) -> Result<UsageReportLookup, WireError> {
        // An opaque id must not expand an agent's local-session authority.
        if connection.principal().agent().is_some() {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "an agent must select usage by its own local session",
            ));
        }
        if request.refresh == LimitRefresh::Refresh {
            connection.principal().permit(Method::CreateSession)?;
        }
        let policy = self.limits.observation().await?;
        if policy.policy == ExternalLookup::Off {
            return Ok(UsageReportLookup::Off);
        }
        let Some(binding) = self.usage_reports.resolve(&request.id).await else {
            return Ok(UsageReportLookup::NotFound { id: request.id });
        };
        let (current, _) = self.report_binding(binding.scope.clone()).await?;
        if current != binding {
            return Ok(UsageReportLookup::NotFound { id: request.id });
        }
        let report = self
            .saved_report(connection, binding.scope, request.refresh, &policy)
            .await?;
        if !policy.still_current() || report.id != request.id {
            return Err(changed());
        }
        let found = UsageReportLookup::Found {
            report: Box::new(report),
            cost_visibility: visibility(connection),
        }
        .for_visibility(visibility(connection));
        found.validate_budget().map_err(|_| too_large())?;
        Ok(found)
    }
    fn report_permission(
        &self,
        connection: &Connection,
        scope: &UsageReportScope,
        refresh: LimitRefresh,
    ) -> Result<(), WireError> {
        if let Some((node, own)) = connection.principal().agent()
            && (node != self.lifecycle.info().id
                || !matches!(scope,UsageReportScope::Session {session} if *session==own))
        {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "an agent may read only usage of its own local session",
            ));
        }
        if refresh == LimitRefresh::Refresh {
            connection.principal().permit(Method::CreateSession)?;
        }
        Ok(())
    }
    async fn saved_report(
        &self,
        connection: &Connection,
        scope: LimitsScope,
        refresh: LimitRefresh,
        policy: &Observation,
    ) -> Result<SourceReportEntry, WireError> {
        let (binding, harness) = self.report_binding(scope).await?;
        let (source, _, _) = usage_reports::source(&harness).ok_or_else(|| {
            WireError::new(
                ErrorCode::Unsupported,
                "this saved login has no usage report source",
            )
        })?;
        let id = self
            .usage_reports
            .identify(
                source.parse().map_err(|_| changed())?,
                binding.clone(),
                policy,
            )
            .await?;
        let limits = self
            .get_usage_limits(
                connection,
                GetUsageLimits {
                    scope: binding.scope.clone(),
                    refresh,
                },
            )
            .await?;
        let (after, _) = self.report_binding(binding.scope.clone()).await?;
        if after != binding || !policy.still_current() {
            return Err(changed());
        }
        let label = limits.account.profile.as_ref().map(|name| name.as_str());
        let label = label.map(str::to_owned);
        usage_reports::entry(id, label.as_deref(), limits).map_err(|_| {
            WireError::new(
                ErrorCode::Internal,
                "the complete saved account report could not be represented safely",
            )
        })
    }
    async fn report_binding(&self, scope: LimitsScope) -> Result<(Binding, HarnessRef), WireError> {
        match scope {
            LimitsScope::Account { account } => {
                let profile = match account.profile {
                    Some(name) => self.profiles.get(&name),
                    None => self.profiles.default_of(&account.harness),
                }
                .filter(|profile| profile.harness() == account.harness)
                .ok_or_else(|| {
                    WireError::new(
                        ErrorCode::NotFound,
                        "that usage account is no longer configured",
                    )
                })?;
                let account = LimitAccount {
                    harness: profile.harness(),
                    profile: Some(profile.name().clone()),
                };
                let harness = account.harness.clone();
                let proof = self.launcher.account_binding(&self.profiles, &profile);
                Ok((
                    Binding {
                        scope: LimitsScope::Account { account },
                        proof,
                    },
                    harness,
                ))
            }
            LimitsScope::Session { session } => {
                let info = self
                    .sessions
                    .get(session)
                    .await
                    .map_err(|error| super::wire(&error))?;
                let proof = self.session_limit_binding(session).await;
                let harness = info.spec.harness;
                Ok((
                    Binding {
                        scope: LimitsScope::Session { session },
                        proof,
                    },
                    harness,
                ))
            }
        }
    }
}
fn visibility(connection: &Connection) -> CostVisibility {
    if connection.principal().permit(Method::CreateSession).is_ok() {
        CostVisibility::Permitted
    } else {
        CostVisibility::Withheld
    }
}
fn complete(
    reports: Vec<SourceReportEntry>,
    policy: ExternalLookup,
    visibility: CostVisibility,
) -> Result<UsageReports, WireError> {
    let reports = ReportEntries::try_from(reports).map_err(|_| too_large())?;
    let result = UsageReports {
        at: chrono::Utc::now(),
        policy,
        cost_visibility: visibility,
        reports,
        source_failures: ReportSourceFailures::default(),
    }
    .for_visibility(visibility);
    result.validate_budget().map_err(|_| too_large())?;
    Ok(result)
}
fn changed() -> WireError {
    WireError::new(
        ErrorCode::Conflict,
        "usage report bindings changed; read reports again",
    )
}
fn too_large() -> WireError {
    WireError::new(
        ErrorCode::Unavailable,
        "the complete usage report exceeds its supported size; no accounts were omitted",
    )
}
