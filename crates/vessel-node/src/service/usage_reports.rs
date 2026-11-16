//! Generic report selectors reuse saved-login policy and immutable session proofs.
mod pipeline;
use super::{Connection, NodeService};
use crate::{
    limits::policy::Observation,
    usage_reports::{self, Binding},
};
use futures_util::{StreamExt as _, stream};
use vessel_proto::{
    ErrorCode, HarnessRef, Method, WireError,
    limits::{ExternalLookup, LimitAccount, LimitRefresh, LimitsScope},
    usage::CostVisibility,
    usage_reports::{
        InspectUsageReport, ListUsageReports, MAX_USAGE_REPORTS, ReportEntries,
        ReportSourceFailures, SourceReportEntry, UsageReportLookup, UsageReportScope, UsageReports,
    },
};

const REPORT_REFRESH_BUDGET: std::time::Duration = std::time::Duration::from_secs(90);
const SAVED_REFRESH_CONCURRENCY: usize = 1;

impl NodeService {
    pub(crate) async fn set_usage_source_lookup(
        &self,
        connection: &Connection,
        request: vessel_proto::usage_reports::SetUsageSourceLookup,
    ) -> Result<vessel_proto::usage_reports::UsageSourceDescriptor, WireError> {
        connection.principal().permit(Method::SetUsageLookup)?;
        let global = self.limits.observation().await?;
        self.limits.sources.set(request, &global).await
    }

    pub(crate) async fn list_usage_reports(
        &self,
        connection: &Connection,
        request: ListUsageReports,
    ) -> Result<UsageReports, WireError> {
        self.report_permission(connection, &request.scope, request.refresh)?;
        let refresh = request.refresh == LimitRefresh::Refresh;
        let reading = self.list_reports(connection, request);
        if refresh {
            refresh_within_budget(reading).await
        } else {
            reading.await
        }
    }
    async fn list_reports(
        &self,
        connection: &Connection,
        request: ListUsageReports,
    ) -> Result<UsageReports, WireError> {
        let policy = self.limits.observation().await?;
        let visibility = visibility(connection);
        let machine = matches!(&request.scope, UsageReportScope::Machine);
        let refresh = request.refresh;
        let saved = async {
            if policy.policy == ExternalLookup::Off {
                Ok((Vec::new(), Vec::new()))
            } else {
                self.saved_reports(connection, request, &policy).await
            }
        };
        let source_refresh = async {
            if machine && refresh == LimitRefresh::Refresh && policy.policy == ExternalLookup::On {
                self.limits.sources.refresh_all(&policy).await
            } else {
                Vec::new()
            }
        };
        // The existing owners keep their own actual-work permits; neither group waits
        // for the other's vendor deadlines before it can start.
        let (saved, refreshing) = tokio::join!(saved, source_refresh);
        let (reports, mut failures) = saved?;
        for failure in refreshing {
            upsert_failure(&mut failures, failure);
        }
        let mut additional = self.limits.sources.snapshot(&policy).await?;
        if !machine {
            additional.reports.clear();
            additional.failures.clear();
        }
        for failure in failures {
            upsert_failure(&mut additional.failures, failure);
        }
        if !policy.still_current() || !additional.still_current() {
            return Err(changed());
        }
        complete(reports, policy.policy, visibility, additional)
    }
    async fn saved_reports(
        &self,
        connection: &Connection,
        request: ListUsageReports,
        policy: &Observation,
    ) -> Result<
        (
            Vec<SourceReportEntry>,
            Vec<vessel_proto::usage_reports::UsageSourceFailure>,
        ),
        WireError,
    > {
        let machine = matches!(&request.scope, UsageReportScope::Machine);
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
            return Err(too_large());
        }
        let mut reports = Vec::with_capacity(scopes.len());
        let mut failures = Vec::new();
        let refresh = request.refresh;
        let refreshed = stream::iter(scopes)
            .map(|scope| async move {
                let source = match &scope {
                    LimitsScope::Account { account } => usage_reports::source(&account.harness)
                        .map(vessel_proto::usage_reports::SavedUsageSource::id)
                        .transpose()
                        .map_err(|_invalid| changed())?,
                    LimitsScope::Session { .. } => None,
                };
                let result = self.saved_report(connection, scope, refresh, policy).await;
                Ok::<_, WireError>((source, result))
            })
            .buffered(SAVED_REFRESH_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        for refreshed in refreshed {
            let (source, result) = refreshed?;
            match result {
                Ok(report) => reports.push(report),
                Err(error) if !machine => return Err(error),
                Err(error) => {
                    if let Some(source) = source {
                        let reason = match error.code {
                            ErrorCode::Unavailable => {
                                vessel_proto::usage_reports::UsageFetchFailure::Busy
                            }
                            ErrorCode::Unsupported => {
                                vessel_proto::usage_reports::UsageFetchFailure::SourceRemoved
                            }
                            ErrorCode::InvalidRequest
                            | ErrorCode::Unauthorized
                            | ErrorCode::Forbidden
                            | ErrorCode::NotFound
                            | ErrorCode::Conflict
                            | ErrorCode::ProtocolMismatch
                            | ErrorCode::HarnessNotInstalled
                            | ErrorCode::SpawnFailed
                            | ErrorCode::Internal => {
                                vessel_proto::usage_reports::UsageFetchFailure::ChangedResponse
                            }
                        };
                        upsert_failure(
                            &mut failures,
                            vessel_proto::usage_reports::UsageSourceFailure {
                                source,
                                failure: vessel_proto::usage_reports::UsageAttemptFailure::Error {
                                    reason,
                                },
                            },
                        );
                    }
                }
            }
        }
        Ok((reports, failures))
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
        let refresh = request.refresh == LimitRefresh::Refresh;
        let reading = self.inspect_report(connection, request);
        if refresh {
            refresh_within_budget(reading).await
        } else {
            reading.await
        }
    }
    async fn inspect_report(
        &self,
        connection: &Connection,
        request: InspectUsageReport,
    ) -> Result<UsageReportLookup, WireError> {
        let policy = self.limits.observation().await?;
        if policy.policy == ExternalLookup::Off {
            return Ok(UsageReportLookup::Off);
        }
        let Some(binding) = self.usage_reports.resolve(&request.id).await else {
            let report = self
                .limits
                .sources
                .inspect(&request.id, &policy, request.refresh)
                .await?;
            if !policy.still_current() {
                return Err(changed());
            }
            let found = match report {
                Some(report) => UsageReportLookup::Found {
                    report: Box::new(report),
                    cost_visibility: visibility(connection),
                },
                None => UsageReportLookup::NotFound { id: request.id },
            }
            .for_visibility(visibility(connection));
            let _encoded = found.validate_budget().map_err(|_budget| too_large())?;
            return Ok(found);
        };
        let (current, _) = self.report_binding(binding.scope.clone()).await?;
        if current != binding {
            return Ok(UsageReportLookup::NotFound { id: request.id });
        }
        let report = self
            .saved_report(connection, binding.scope, request.refresh, &policy)
            .await?;
        if !policy.still_current() || report.id() != &request.id {
            return Err(changed());
        }
        let found = UsageReportLookup::Found {
            report: Box::new(report),
            cost_visibility: visibility(connection),
        }
        .for_visibility(visibility(connection));
        let _encoded = found.validate_budget().map_err(|_budget| too_large())?;
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
        pipeline::saved_report(
            &Access {
                service: self,
                connection,
            },
            scope,
            refresh,
            policy,
        )
        .await
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
                        account: account.clone(),
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
                let account = proof.as_ref().map_or(
                    LimitAccount {
                        harness: info.spec.harness,
                        profile: info.profile,
                    },
                    |proof| proof.account().clone(),
                );
                let harness = account.harness.clone();
                Ok((
                    Binding {
                        account,
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
    mut reports: Vec<SourceReportEntry>,
    policy: ExternalLookup,
    visibility: CostVisibility,
    additional: usage_reports::Snapshot,
) -> Result<UsageReports, WireError> {
    reports.extend(additional.reports);
    let reports = ReportEntries::try_from(reports).map_err(|_collection| too_large())?;
    let result = UsageReports {
        at: chrono::Utc::now(),
        policy,
        cost_visibility: visibility,
        reports,
        sources: additional.sources,
        source_failures: ReportSourceFailures::try_from(additional.failures)
            .map_err(|_bound| too_large())?,
    }
    .for_visibility(visibility);
    let _encoded = result.validate_budget().map_err(|_budget| too_large())?;
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

struct Access<'a> {
    service: &'a NodeService,
    connection: &'a Connection,
}
impl pipeline::Bindings for Access<'_> {
    async fn capture(&self, scope: LimitsScope) -> Result<(Binding, HarnessRef), WireError> {
        self.service.report_binding(scope).await
    }
    fn registry(&self) -> &usage_reports::Registry {
        &self.service.usage_reports
    }
    fn limits(&self) -> &crate::limits::Limits {
        &self.service.limits
    }
    fn visibility(&self) -> CostVisibility {
        visibility(self.connection)
    }
}

fn upsert_failure(
    failures: &mut Vec<vessel_proto::usage_reports::UsageSourceFailure>,
    failure: vessel_proto::usage_reports::UsageSourceFailure,
) {
    if let Some(previous) = failures
        .iter_mut()
        .find(|previous| previous.source == failure.source)
    {
        *previous = failure;
    } else {
        failures.push(failure);
    }
}

#[cfg(test)]
mod tests;

async fn refresh_within_budget<T>(
    reading: impl Future<Output = Result<T, WireError>>,
) -> Result<T, WireError> {
    tokio::time::timeout(REPORT_REFRESH_BUDGET, reading)
        .await
        .map_err(|_elapsed| {
            WireError::new(
                ErrorCode::Unavailable,
                "the complete refresh exceeded its time budget; cached reports remain available",
            )
        })?
}
