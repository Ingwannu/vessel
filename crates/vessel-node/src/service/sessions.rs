//! Sessions' calls (protocols 2 to 10): list, read, create (an import's
//! path too, and a place resolved into its folder), send, kill, read the
//! log, subscribe. Routing only, as the rest of the service.

use std::time::Duration;

use vessel_proto::workspace::SessionPlace;
use vessel_proto::{
    Ack, CreateSession, EventPage, GetSession, KillSession, ListSessions, ReadEvents, SendInput,
    SessionId, SessionInfo, SessionOrigin, SessionSpec, Subscribe, Subscribed, Unsubscribe,
    WireError,
};

use crate::error::ServiceError;
use crate::principal::Principal;
use crate::subscribe;
use crate::waits::Waiter;

use super::{Connection, NodeService, wire};

impl NodeService {
    pub(crate) async fn list_sessions(
        &self,
        _: &Connection,
        request: ListSessions,
    ) -> Result<Vec<SessionInfo>, WireError> {
        let list = match request.selection() {
            Some(selection) => self.sessions.selected(selection).await,
            None => self.sessions.list().await,
        };
        list.map(|list| {
            list.into_iter()
                .map(|info| self.with_workspace(info))
                .collect()
        })
        .map_err(|error| wire(&error))
    }

    pub(crate) async fn get_session(
        &self,
        _: &Connection,
        request: GetSession,
    ) -> Result<SessionInfo, WireError> {
        self.sessions
            .get(request.session)
            .await
            .map(|info| self.with_workspace(info))
            .map_err(|error| wire(&error))
    }

    /// `session.create`: the spec's profile resolved, the caller recorded
    /// as its creator, and the session's agent given its credential within
    /// the caller's grant and the agent limits (ADRs 0011, 0013).
    pub(crate) async fn create_session(
        &self,
        connection: &Connection,
        request: CreateSession,
    ) -> Result<SessionInfo, WireError> {
        crate::label_ingress::spec(&request.spec)?;
        self.create(connection, request.spec, None).await
    }

    /// A session of `spec` for `connection`'s caller: `session.create`'s
    /// path, also an import's (`origin` says where it came from).
    pub(super) async fn create(
        &self,
        connection: &Connection,
        spec: SessionSpec,
        origin: Option<SessionOrigin>,
    ) -> Result<SessionInfo, WireError> {
        self.create_as(connection.principal(), spec, origin, None)
            .await
    }

    /// A session of `spec` for `principal`: `create`'s path, also a
    /// schedule's run (protocol 11), which acts as its definer with nobody
    /// watching: `unattended` is the line its log starts with, and its
    /// agent is offered no questions (ADR 0033).
    pub(crate) async fn create_as(
        &self,
        principal: &Principal,
        mut spec: SessionSpec,
        origin: Option<SessionOrigin>,
        unattended: Option<String>,
    ) -> Result<SessionInfo, WireError> {
        if origin.is_some() {
            spec.side_question_of = None;
        }
        if let Some(parent) = spec.side_question_of {
            // This path inherits a spec or reuses a side; never discard an override.
            if !spec.labels.is_empty() {
                return Err(wire(&ServiceError::SideCreationLabels));
            }
            if spec.listing.is_some() {
                return Err(wire(&ServiceError::SideCreationListing));
            }
            if unattended.is_some() {
                return Err(crate::side_questions::SideError::Template.wire());
            }
            return self.open_side_as(principal, parent).await;
        }
        self.sessions
            .inherit_listing(&mut spec, self.local_creator(principal))
            .await
            .map_err(|error| wire(&error))?;
        self.create_core_as(principal, spec, origin, unattended, None)
            .await
    }

    /// Resolved side creation retains its serialization lease through launch.
    pub(super) async fn create_core_as(
        &self,
        principal: &Principal,
        spec: SessionSpec,
        origin: Option<SessionOrigin>,
        unattended: Option<String>,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) -> Result<SessionInfo, WireError> {
        self.create_starting(principal, spec, (origin, unattended), None, lease)
            .await
    }

    /// `create_as`'s path, for a session that starts as a fork of another's
    /// conversation too (protocol 14, `forking`): the same refusals,
    /// grants, credentials and model token as any session its caller
    /// creates.
    pub(super) async fn create_starting(
        &self,
        principal: &Principal,
        mut spec: SessionSpec,
        (origin, unattended): (Option<SessionOrigin>, Option<String>),
        fork: Option<vessel_session::ForkStart>,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) -> Result<SessionInfo, WireError> {
        if self.lifecycle.is_draining() {
            return Err(wire(&ServiceError::Draining));
        }
        self.refuse_installing(&spec.harness)
            .map_err(|error| wire(&error))?;
        // An imported session's place named its own machine's project; it
        // runs in the folder it brings (carrying worktrees is wave 2's).
        if origin.is_some() {
            spec.place = None;
        }
        // A place resolves to the folder `cwd` names (protocol 10). In a
        // worktree, `_starting` keeps a removal out until the session is
        // live, or failed to start (ADR 0030); it is dropped on return.
        let (notices, _starting) = self.place(principal, &mut spec).await?;
        let mut options = self
            .launcher
            .options(&self.profiles, &spec, principal)
            .map_err(|error| wire(&error))?;
        if let Some(line) = unattended {
            options.questions = false;
            options.notices.push(line);
        }
        options.title_origin = title_origin(&spec, origin.as_ref(), fork.as_ref());
        // An import says where it came from; otherwise the launcher's
        // (a call delivered from another node records `remote`).
        if origin.is_some() {
            options.origin = origin;
        }
        let id = SessionId::random();
        let secret = self
            .agents
            .reserve(id, principal, self.launcher.has_tools())
            .map_err(|error| wire(&error))?;
        options.id = Some(id);
        options.notices.extend(notices);
        if let Some(secret) = &secret
            && let Err(error) = self
                .launcher
                .attach_tools(&mut options.mcp_servers, id, secret)
        {
            self.agents.forget(id);
            return Err(wire(&error));
        }
        // A routed profile's harness calls the node's proxy with a token of
        // the session's own (ADR 0015).
        match self.models.launch(options.profile.as_ref(), id) {
            Ok(Some((model, proxy))) => {
                options.model = Some(model);
                options.model_proxy = Some(proxy);
            }
            Ok(None) => {}
            Err(error) => {
                self.agents.forget(id);
                return Err(wire(&error));
            }
        }
        options.fork = fork.map(Box::new);
        let created = self.sessions.create(spec, options, lease).await;
        if created.is_err() {
            self.agents.forget(id);
            self.models.retire(id);
        }
        let info = created.map_err(|error| wire(&error))?;
        if let Some(SessionPlace::Worktree { worktree }) = info.spec.place.as_deref() {
            self.workspaces.claim(*worktree, info.id).await;
        }
        // Its pull request, when it is placed (protocol 11).
        if let Some(handle) = self.sessions.live(info.id) {
            self.watch_session(&handle, principal);
        }
        Ok(self.with_workspace(info))
    }

    pub(crate) async fn send_input(
        &self,
        _: &Connection,
        request: SendInput,
    ) -> Result<Ack, WireError> {
        self.sessions
            .send(request.session, request.input)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn kill_session(
        &self,
        _: &Connection,
        request: KillSession,
    ) -> Result<Ack, WireError> {
        self.sessions
            .kill(request.session)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error))
    }

    pub(crate) async fn read_events(
        &self,
        connection: &Connection,
        mut request: ReadEvents,
    ) -> Result<EventPage, WireError> {
        let here = self.links.node();
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.wait_slot(connection, shares, &mut request.wait);
        let read = match request.wait {
            Some(seconds) => {
                let wait = Duration::from_secs(seconds.min(ReadEvents::MAX_WAIT).into());
                self.sessions
                    .read_waiting(request.session, request.from, request.limit, wait)
                    .await
            }
            None => {
                self.sessions
                    .read(request.session, request.from, request.limit)
                    .await
            }
        };
        read.map_err(|error| wire(&error))
    }

    pub(crate) async fn subscribe(
        &self,
        connection: &Connection,
        request: Subscribe,
    ) -> Result<Subscribed, WireError> {
        // Before it opens: an older client must not be subscribed to what
        // it cannot decode (protocol 11's added agents).
        self.refuse_older(connection, request.session).await?;
        subscribe::open(
            &self.sessions,
            &connection.outbound,
            &self.forwarders,
            request,
        )
        .await
        .map(|mut subscribed| {
            subscribed.info = self.with_workspace(subscribed.info);
            subscribed
        })
        .map_err(|error| wire(&error))
    }

    pub(crate) async fn unsubscribe(
        &self,
        connection: &Connection,
        request: Unsubscribe,
    ) -> Result<Ack, WireError> {
        connection
            .outbound
            .close_subscription(request.subscription)
            .await
            .map(|()| Ack)
            .map_err(|error| wire(&error.into()))
    }
}

/// A copied specification cannot attest who wrote its title. Locally
/// derived side/fork names remain given; explicit names (including a
/// person's schedule name) keep `SessionInfo`'s typed default.
fn title_origin(
    spec: &SessionSpec,
    origin: Option<&SessionOrigin>,
    fork: Option<&vessel_session::ForkStart>,
) -> Option<vessel_proto::TitleOrigin> {
    match origin {
        Some(SessionOrigin::Imported { .. }) => Some(vessel_proto::TitleOrigin::Unknown),
        Some(SessionOrigin::Remote { .. }) | None => (spec.side_question_of.is_some()
            || fork.is_some())
        .then_some(vessel_proto::TitleOrigin::Given),
    }
}
