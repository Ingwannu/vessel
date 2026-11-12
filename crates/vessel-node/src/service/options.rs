//! Protocol 9's calls (milestone 10, ADR 0022): what an agent offers
//! before a session, what a session runs with and changing it, and a
//! session's title. Routing only, as the rest of the service: the session
//! runtime keeps the options and titles (`vessel-session`), `probes` keeps
//! what agents offer.

use chrono::Utc;
use vessel_proto::{
    GetHarnessOptions, GetSessionOptions, HarnessOptions, HarnessRef, RenameSession, SessionId,
    SessionInfo, SessionOption, SetSessionOption, StructuredSupport, WireError,
};

use super::{Connection, NodeService, wire};
use crate::error::ServiceError;
use crate::probes::ProbeKey;
use crate::transfers::export::named;

impl NodeService {
    /// `harness.options`: what `harness`'s agent offers under `profile`,
    /// from a recent probe or a new one (`probes`). A harness that cannot
    /// run structured sessions here offers nothing: an empty list, not an
    /// error, since that is the truth a form shows. One that is not
    /// installed, or whose agent does not start (not signed in, say), is
    /// the probe's error, with the reason.
    pub(crate) async fn harness_options(
        &self,
        _: &Connection,
        request: GetHarnessOptions,
    ) -> Result<HarnessOptions, WireError> {
        let GetHarnessOptions { harness, profile } = request;
        let kind = match &harness {
            HarnessRef::Known(kind) => *kind,
            HarnessRef::Added(name) => {
                return Err(wire(&ServiceError::UnknownAgent(name.clone())));
            }
        };
        let resolved =
            named(&self.profiles, Some(kind), profile.as_ref()).map_err(|error| wire(&error))?;
        let installed = vessel_harness::discover(&self.lookup)
            .await
            .into_iter()
            .find(|found| found.kind == harness);
        let Some(installed) = installed else {
            // Not installed: the probe's launch says so, naming the binary.
            return self.probe(harness, resolved).await;
        };
        match installed.structured {
            StructuredSupport::Unavailable { reason } => {
                tracing::debug!(%harness, %reason, "no structured sessions here; it offers nothing");
                Ok(HarnessOptions {
                    options: Vec::new(),
                    probed_at: Utc::now(),
                    abilities: None,
                })
            }
            StructuredSupport::Available => {
                let key = ProbeKey {
                    harness: harness.clone(),
                    profile,
                };
                self.probes
                    .options(key, installed.version, async || {
                        self.probe(harness, resolved).await
                    })
                    .await
            }
        }
    }

    /// Ask `harness`'s agent now. A routed profile's agent gets a model
    /// token of its own for the probe, retired with it: it makes no model
    /// call (no prompt is sent), but it is started as a session would be.
    async fn probe(
        &self,
        harness: HarnessRef,
        profile: vessel_harness::Profile,
    ) -> Result<HarnessOptions, WireError> {
        let probe = SessionId::random();
        let proxy = self
            .models
            .launch(Some(&profile), probe)
            .map_err(|error| wire(&error))?
            .map(|(_model, proxy)| proxy);
        let dir = self.probes.dir().clone();
        let probed = self.sessions.probe(harness, profile, proxy, dir).await;
        self.models.retire(probe);
        let probed = probed.map_err(|error| wire(&error))?;
        Ok(HarnessOptions {
            options: probed.options,
            probed_at: Utc::now(),
            abilities: Some(probed.abilities),
        })
    }

    /// `session.options`: a live session's options as its agent last
    /// reported them.
    pub(crate) async fn session_options(
        &self,
        _: &Connection,
        request: GetSessionOptions,
    ) -> Result<Vec<SessionOption>, WireError> {
        self.sessions
            .options(request.session)
            .await
            .map_err(|error| wire(&error))
    }

    /// `session.set_option`: change one of a live session's options; the
    /// agent's refusal is the call's error.
    pub(crate) async fn set_session_option(
        &self,
        _: &Connection,
        request: SetSessionOption,
    ) -> Result<Vec<SessionOption>, WireError> {
        self.sessions
            .set_option(request.session, request.choice)
            .await
            .map_err(|error| wire(&error))
    }

    /// `session.rename`: give a live session a title.
    pub(crate) async fn rename_session(
        &self,
        _: &Connection,
        request: RenameSession,
    ) -> Result<SessionInfo, WireError> {
        self.sessions
            .rename(request.session, request.title)
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
    }
}
