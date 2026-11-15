//! Protocol 9's calls (milestone 10, ADR 0022): what an agent offers
//! before a session, what a session runs with and changing it, and a
//! session's title. Routing only, as the rest of the service: the session
//! runtime keeps the options and titles (`vessel-session`), `probes` keeps
//! what agents offer.

use chrono::Utc;
use vessel_proto::{
    GetHarnessOptions, GetSessionOptions, HarnessOptions, HarnessRef, ProfileName, RenameSession,
    SessionId, SessionInfo, SessionOption, SetSessionOption, StructuredSupport, WireError,
};

use super::{Connection, NodeService, wire};
use crate::probes::{Answer, ProbeKey};
use crate::readiness::HandshakeFailure;

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
        self.refuse_installing(&harness)
            .map_err(|error| wire(&error))?;
        let resolved = self
            .resolve(&harness, profile.as_ref())
            .map_err(|error| wire(&error))?;
        self.probed(harness, resolved, profile).await.options
    }

    /// What a probe of `harness` under `resolved` (the profile `profile`,
    /// its default when `None`) found: a recent one's, or a new one's.
    pub(crate) async fn probed(
        &self,
        harness: HarnessRef,
        resolved: vessel_harness::Profile,
        profile: Option<ProfileName>,
    ) -> Answer {
        let installed = self
            .harnesses()
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
                Answer {
                    options: Ok(HarnessOptions {
                        options: Vec::new(),
                        probed_at: Utc::now(),
                        abilities: None,
                    }),
                    // No adapter: nothing was started to say anything.
                    said: Err(HandshakeFailure::Spawn),
                }
            }
            StructuredSupport::Available => {
                let key = ProbeKey {
                    harness: harness.clone(),
                    profile,
                };
                self.probes
                    .answer(key, installed.version, async || {
                        self.probe(harness, resolved).await
                    })
                    .await
            }
        }
    }

    /// Ask `harness`'s agent now. A routed profile's agent gets a model
    /// token of its own for the probe, retired with it: it makes no model
    /// call (no prompt is sent), but it is started as a session would be.
    async fn probe(&self, harness: HarnessRef, profile: vessel_harness::Profile) -> Answer {
        let probe = SessionId::random();
        let proxy = match self.models.launch(Some(&profile), probe) {
            Ok(launched) => launched.map(|(_model, proxy)| proxy),
            Err(error) => {
                return Answer {
                    options: Err(wire(&error)),
                    said: Err(HandshakeFailure::Spawn),
                };
            }
        };
        let dir = self.probes.dir().clone();
        let probed = self.sessions.probe(harness, profile, proxy, dir).await;
        self.models.retire(probe);
        match probed {
            Ok(probed) => Answer {
                options: Ok(HarnessOptions {
                    options: probed.options,
                    probed_at: Utc::now(),
                    abilities: Some(probed.abilities),
                }),
                said: Ok(probed.introduction),
            },
            Err(error) => Answer {
                said: Err(crate::readiness::failure_of(&error)),
                options: Err(wire(&error)),
            },
        }
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
