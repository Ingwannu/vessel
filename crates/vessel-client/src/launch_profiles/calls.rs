//! Thin typed calls; only the new live bundle uses the source's RPC budget.

use crate::{CallError, NodeClient};
use std::{future::Future, time::Duration};
use vessel_proto::SessionId;
use vessel_proto::launch_profiles::{
    ApplicableLaunchProfiles, ApplySessionOptions, ApplySessionOptionsResult,
    GetApplicableLaunchProfiles, GetLaunchProfileOptions, LaunchCatalogue, LaunchChoices,
    LaunchProfileRevision, LaunchProfiles, LaunchProfilesSnapshot, LaunchSelection,
    ListLaunchProfiles, ReplaceLaunchProfiles,
};

/// The pinned reference's session RPC budget. Existing calls are unchanged.
pub const APPLY_OPTIONS_TIMEOUT: Duration = Duration::from_millis(60_000);

/// Typed uncertainty and validation at the new launch-profile client boundary.
#[derive(Debug, thiserror::Error)]
pub enum LaunchProfileClientError {
    /// Normal local admission, connection or remote refusal; no blind replay.
    #[error(transparent)]
    Call(#[from] CallError),
    /// Provider work can remain fenced after this caller stops waiting.
    #[error("configuration acknowledgement timed out; reread the session before retrying")]
    Timeout(#[source] tokio::time::error::Elapsed),
    /// A pure transition exceeded a validated launch bound.
    #[error("the prepared launch selection is invalid")]
    Values(#[from] vessel_proto::launch_profiles::InvalidLaunchValue),
}

/// Node-reported protocol support; an unsupported state is not an empty list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchProfileSupport {
    /// The shared feature is not assigned; no call may be sent.
    Unassigned,
    /// The connection cannot decode both CRUD and live application.
    Older {
        /// Required table version.
        needed: vessel_proto::ProtocolVersion,
        /// Negotiated peer version.
        negotiated: vessel_proto::ProtocolVersion,
    },
    /// Both capabilities meet the connection's negotiated version.
    Available,
}

impl NodeClient {
    /// Migration requires both host CRUD and live-apply capability, as the
    /// source picker does. Requirements come only from the method table.
    #[must_use]
    pub fn launch_profile_support(&self) -> LaunchProfileSupport {
        let crud = vessel_proto::Method::ListLaunchProfiles.since();
        let apply = vessel_proto::Method::ApplySessionOptions.since();
        let (Some(crud), Some(apply)) = (crud, apply) else {
            return LaunchProfileSupport::Unassigned;
        };
        let needed = crud.max(apply);
        if self.protocol() < needed {
            LaunchProfileSupport::Older {
                needed,
                negotiated: self.protocol(),
            }
        } else {
            LaunchProfileSupport::Available
        }
    }

    /// Complete editable host list; successful empty is distinct from unavailable.
    ///
    /// # Errors
    /// Normal Read, pending/older protocol, connection and storage refusals.
    pub async fn launch_profiles(&self) -> Result<LaunchProfilesSnapshot, CallError> {
        self.call(ListLaunchProfiles {}).await
    }

    /// Whole-list ordered CAS. Conflict/lost acknowledgement requires rereading.
    ///
    /// # Errors
    /// Admin/human, revision, validation, storage and protocol refusals.
    pub async fn replace_launch_profiles(
        &self,
        expected: LaunchProfileRevision,
        profiles: LaunchProfiles,
    ) -> Result<LaunchProfilesSnapshot, CallError> {
        self.call(ReplaceLaunchProfiles { expected, profiles })
            .await
    }

    /// Actual host-wide options after selected choices; discard stale request keys.
    ///
    /// # Errors
    /// Read/human, unavailable account/driver, busy, configuration changed,
    /// probe failure and pending/older protocol refusals.
    pub async fn launch_profile_options(
        &self,
        selection: LaunchSelection,
    ) -> Result<LaunchCatalogue, CallError> {
        self.call(GetLaunchProfileOptions { selection }).await
    }

    /// Presets eligible for the session's actual harness and canonical account.
    ///
    /// # Errors
    /// Read, missing/ended/terminal session, storage and protocol refusals.
    pub async fn applicable_launch_profiles(
        &self,
        session: SessionId,
    ) -> Result<ApplicableLaunchProfiles, CallError> {
        self.call(GetApplicableLaunchProfiles { session }).await
    }

    /// One node-owned bundle, preserving actual prefix/notice/first refusal.
    /// A timeout never authorizes retry or says that applied work was rolled back.
    ///
    /// # Errors
    /// Normal Operate/ownership/state/admission/protocol refusals, or typed
    /// acknowledgement timeout while the same process's fence remains held.
    pub async fn apply_session_options(
        &self,
        session: SessionId,
        options: LaunchChoices,
    ) -> Result<ApplySessionOptionsResult, LaunchProfileClientError> {
        apply_budget(self.call(ApplySessionOptions { session, options })).await
    }
}

async fn apply_budget<T>(
    call: impl Future<Output = Result<T, CallError>>,
) -> Result<T, LaunchProfileClientError> {
    tokio::time::timeout(APPLY_OPTIONS_TIMEOUT, call)
        .await
        .map_err(LaunchProfileClientError::Timeout)?
        .map_err(LaunchProfileClientError::Call)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn only_the_new_call_budget_expires_at_the_pinned_reference_limit() {
        let phase = Duration::from_secs(5);
        let safety = APPLY_OPTIONS_TIMEOUT.saturating_add(phase);
        vessel_testkit::within(
            safety,
            Box::pin(async {
                let mut call =
                    Box::pin(apply_budget(std::future::pending::<Result<(), CallError>>()));
                assert!(futures_util::poll!(&mut call).is_pending());
                tokio::time::advance(APPLY_OPTIONS_TIMEOUT.saturating_sub(Duration::from_secs(1)))
                    .await;
                assert!(futures_util::poll!(&mut call).is_pending());
                tokio::time::advance(Duration::from_secs(1)).await;
                let error = tokio::time::timeout(phase, call)
                    .await
                    .expect("named phase: new call budget expires")
                    .unwrap_err();
                assert!(matches!(error, LaunchProfileClientError::Timeout(_)));
                assert!(std::error::Error::source(&error).is_some());
            }),
        )
        .await
        .unwrap();
    }
}
