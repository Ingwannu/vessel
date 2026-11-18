//! Usage authorization is evaluated at the destination, including a
//! delegated principal's intersection. No credential lookup here.

use super::{Connection, NodeService, wire};
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageReport, UsageSampleKind, UsageScope};
use vessel_proto::{ErrorCode, Method, WireError};

impl NodeService {
    pub(crate) async fn query_usage(
        &self,
        connection: &Connection,
        request: QueryUsage,
    ) -> Result<UsageReport, WireError> {
        let principal = connection.principal();
        if let Some((node, own)) = principal.agent()
            && (node != self.lifecycle.info().id
                || !matches!(request.scope, UsageScope::Session { session } if session == own))
        {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "an agent may read only usage of its own session on this machine",
            ));
        }
        if let UsageScope::Session { session } = request.scope {
            let info = self
                .sessions
                .get(session)
                .await
                .map_err(|error| wire(&error))?;
            // Match session.read's durability barrier: a received turn-end
            // must not race an immediate total query.
            let _page = self
                .sessions
                .read(session, info.next_seq, 1)
                .await
                .map_err(|error| wire(&error))?;
        } else if request.period == UsagePeriod::Session || request.after.is_some() {
            return Err(WireError::new(
                ErrorCode::InvalidRequest,
                "choose a day or week for machine or agent usage; only session usage has a sequence cursor",
            ));
        } else {
            // The same barrier across every live session, bounded
            // (`sessions::settled`): a turn end a person has seen must not
            // race a machine or agent total.
            let late = self.sessions.settled().await;
            if !late.is_empty() {
                tracing::warn!(
                    ?late,
                    "usage read before these sessions' stores caught up (waited {:?})",
                    crate::sessions::settled::SETTLE
                );
            }
        }
        let mut report = self
            .sessions
            .store()
            .usage(request, chrono::Utc::now())
            .await
            .map_err(|error| wire(&error.into()))?;
        // The new aggregate does not widen cost visibility. Existing log
        // reads keep their original contract (ADR 0042).
        if principal.permit(Method::CreateSession).is_err() {
            report.cost_visibility = vessel_proto::usage::CostVisibility::Withheld;
            report.current_costs.clear();
            report.totals.costs.clear();
            report.totals.unaccounted_costs = vessel_proto::usage::ReportCount::new(0);
            for agent in &mut report.agents {
                agent.totals.costs.clear();
                agent.totals.unaccounted_costs = vessel_proto::usage::ReportCount::new(0);
            }
            report
                .samples
                .retain(|sample| matches!(sample.kind, UsageSampleKind::Turn { .. }));
        }
        Ok(report)
    }
}
