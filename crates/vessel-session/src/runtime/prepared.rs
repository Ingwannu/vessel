//! A preparation-scoped view of existing runtime operations. It captures the
//! actual runtime's `LaunchAdmission`; foreign handles cannot weaken that fence.
//! No `Deref`, arbitrary command, quota or second launch/claim builder is exposed.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use vessel_harness::{ModelProxy, Profile};
use vessel_proto::HarnessRef;

use super::agent::ProbePurpose;
use super::{LaunchAdmission, ProbeChoices, SessionRuntime};
use crate::{
    Ended, PreparationAdmission, PreparationCancellation, PreparationGate, PreparedAgain, Probed,
    Relaunch, Reopening, SessionError,
};

/// A host-admitted view over the same runtime and original preparation owner.
/// This gates preparation effects only. A node still performs its fresh final
/// device/grant/policy checks before starting a prepared session.
pub struct PreparedRuntime<'r, F> {
    runtime: &'r SessionRuntime,
    gate: Arc<PreparationGate<F>>,
    admission: LaunchAdmission,
}

/// The same retained owners travel through actual physical work. Cloning this
/// value never acquires another slot or creates another host admission owner.
pub(crate) struct RetainedPreparation<T, F> {
    pub(super) owner: Arc<T>,
    pub(super) gate: Arc<PreparationGate<F>>,
    pub(super) admission: LaunchAdmission,
}

impl<T, F> Clone for RetainedPreparation<T, F> {
    fn clone(&self) -> Self {
        Self {
            owner: Arc::clone(&self.owner),
            gate: Arc::clone(&self.gate),
            admission: self.admission.clone(),
        }
    }
}

impl SessionRuntime {
    pub(crate) fn default_preparation_view(
        &self,
    ) -> PreparedRuntime<'_, super::preparation_gate::RuntimeAdmission> {
        self.with_preparation(super::preparation_gate::runtime_gate(self.shutdown.clone()))
    }

    /// Use an existing host arbiter without replacing this runtime's actual
    /// node/runtime shutdown admission. No operation is admitted by construction.
    #[must_use]
    pub fn with_preparation<F>(&self, gate: PreparationGate<F>) -> PreparedRuntime<'_, F>
    where
        F: for<'a> Fn(
                crate::PreparationEffect,
                PreparationCancellation<'a>,
            ) -> PreparationAdmission
            + Send
            + Sync
            + 'static,
    {
        PreparedRuntime {
            runtime: self,
            gate: Arc::new(gate),
            admission: self.launch_admission(),
        }
    }
}

impl<F> fmt::Debug for PreparedRuntime<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparedRuntime { .. }")
    }
}

impl<'r, F> PreparedRuntime<'r, F>
where
    F: for<'a> Fn(crate::PreparationEffect, PreparationCancellation<'a>) -> PreparationAdmission
        + Send
        + Sync
        + 'static,
{
    pub(crate) fn retain<T>(&self, owner: Arc<T>) -> RetainedPreparation<T, F> {
        RetainedPreparation {
            owner,
            gate: Arc::clone(&self.gate),
            admission: self.admission.clone(),
        }
    }

    pub(crate) fn admission(&self) -> LaunchAdmission {
        self.admission.clone()
    }

    /// Prepare and start using the ordinary relaunch builder. Hosts requiring
    /// a fresh grant after preparation use `prepare_again` then their own final
    /// arbiter before `PreparedAgain::start`; this convenience adds no authority.
    ///
    /// # Errors
    /// As [`SessionRuntime::start_again`], including typed preparation refusal.
    pub async fn start_again(
        &self,
        ended: Ended,
        launch: Relaunch,
    ) -> Result<Reopening, SessionError> {
        self.prepare_again(ended, launch).await?.start()
    }

    /// Prepare with the same original gate and actual runtime admission. The
    /// result borrows the runtime, never this potentially temporary view.
    ///
    /// # Errors
    /// As [`SessionRuntime::prepare_again`], including typed preparation refusal.
    pub async fn prepare_again(
        &self,
        ended: Ended,
        launch: Relaunch,
    ) -> Result<PreparedAgain<'r>, SessionError> {
        self.runtime.prepare_again_with(ended, launch, self).await
    }

    /// Read options through the existing prompt-free runtime probe.
    ///
    /// # Errors
    /// As [`SessionRuntime::probe`], including typed preparation refusal.
    pub async fn probe(
        &self,
        harness: HarnessRef,
        profile: Option<Profile>,
        proxy: Option<ModelProxy>,
        cwd: PathBuf,
    ) -> Result<Probed, SessionError> {
        self.runtime
            .probe_selected(
                harness,
                profile,
                proxy,
                cwd,
                (ProbeChoices::default(), ProbePurpose::Options),
                self.retain(Arc::new(())),
            )
            .await
    }

    /// Discover the draft while retaining the original caller resource owner.
    /// The host gate can pause or refuse before a new lookup/version helper.
    ///
    /// # Errors
    /// As [`SessionRuntime::probe_with_choices`], including typed preparation refusal.
    pub async fn probe_with_choices_owned<T: Send + Sync + 'static>(
        &self,
        harness: HarnessRef,
        profile: Option<Profile>,
        proxy: Option<ModelProxy>,
        cwd: PathBuf,
        choices: ProbeChoices,
        owner: T,
    ) -> Result<Probed, SessionError> {
        self.runtime
            .probe_selected(
                harness,
                profile,
                proxy,
                cwd,
                (choices, ProbePurpose::Catalog),
                self.retain(Arc::new(owner)),
            )
            .await
    }
}
