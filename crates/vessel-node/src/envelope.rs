//! Calls across the fleet (protocol 6, Track X, ADR 0013 "across nodes
//! through the home"): `fleet.forward`, `fleet.relay` and `fleet.deliver`,
//! and the fleet's `grant`, `peers` and `nodes`.
//!
//! ```text
//!  caller ─forward─▶ its node ─relay─▶ home ─deliver─▶ target
//!                    (checks the       (checks the     (runs it as the
//!                     caller, builds    member speaks   delivering device
//!                     the origin)       for itself)     ∩ the origin)
//! ```
//!
//! [Decision Log]
//! - Purpose: an agent or a person's client reaches only its own node, yet
//!   acts on other nodes of its fleet, with no more authority there than
//!   it has at home and no more than the target's owner granted the home.
//! - Each hop checks what it can know: the first node checks the caller
//!   against the inner call's row as if it were local (so nobody gets a
//!   hop for what they may not do) and vouches for the caller in the
//!   `Origin`; the home checks that a relaying member speaks only for its
//!   own node (by the node its grant came from); the target accepts a
//!   delivery only from its home's device and runs the inner call as
//!   `Principal::Delegated`, the delivering device's scopes intersected
//!   with the origin's, with ownership and the agent limits applied to
//!   the origin's agent. Calls the home runs for a member are delegated the
//!   same way, from the member's device on the home.
//! - Running a call here goes through the guarded handlers, so the scope
//!   and ownership checks run again for the delegated principal. The
//!   dispatch below names every forwardable method (`Method::forwardable`,
//!   pinned by a test); the envelope's own are not among them, so a hop is
//!   never wrapped twice and the dispatch cannot recurse.
//! - Alternatives: a `node` field on every request (every type changes,
//!   and the guard would rediscover the intersection per method); trusting
//!   a delivered `Origin` without the home's check (any member could claim
//!   to be another node's agent).
//! - Trade-off: a call to another member takes two hops; a long poll
//!   (`session.read` with `wait`) waits on the target, holding one call on
//!   each hop.

use vessel_proto::{
    Ack, Call, Callback, Deliver, ErrorCode, FleetNode, ForgetNode, Forward, Forwarded,
    GetCallback, GrantHome, ListNodes, ListPeers, Method, NodeForgotten, NodeId, Origin, OriginBy,
    RelayCall, Reply, Scope, WireError,
};

use crate::guard::Guarded;
use crate::principal::Principal;
use crate::service::{Connection, NodeService};
use crate::waits::{Share, WaitSlot, Waiter};

fn forbidden(message: &str) -> WireError {
    WireError::new(ErrorCode::Forbidden, message.to_owned())
}

fn not_forwardable(method: Method) -> WireError {
    WireError::new(
        ErrorCode::InvalidRequest,
        format!(
            "`{}` cannot be sent to another node (only work on sessions, profiles, \
             transfers, and secret.set and secret.list can)",
            method.as_str()
        ),
    )
}

fn forwardable(method: Method) -> Result<(), WireError> {
    if method.forwardable() {
        Ok(())
    } else {
        Err(not_forwardable(method))
    }
}

impl NodeService {
    /// `fleet.forward`: check the call against the caller, then run it here
    /// or pass it on with the caller's origin.
    pub(crate) async fn forward(
        &self,
        connection: &Connection,
        request: Forward,
        guard: &Guarded,
    ) -> Result<Forwarded, WireError> {
        let Forward { node, mut call } = request;
        if node == self.links.node() {
            forwardable(call.method())?;
            connection.principal().permit(call.method())?;
            return forwarded(run_here(guard, connection, *call).await);
        }
        let here = self.links.node();
        // An agent of this node opens a terminal elsewhere only if its own
        // session's profile allows agent terminals, which only this node
        // can tell (ADR 0031); the other node checks the session it is
        // beside.
        if let (Some((agent_node, own)), Call::CreateTerminal(_)) =
            (connection.principal().agent(), call.as_ref())
            && agent_node == here
        {
            self.profile_allows(own)
                .await
                .map_err(crate::terminals::TerminalsError::wire)?;
        }
        let shares = Waiter::of(connection.principal(), here).shares(here);
        let _slot = self.passed_wait(connection, shares, &mut call);
        self.pass_on(connection.principal(), node, call).await
    }

    /// Run `call` on fleet node `node` (another node) for `principal`: the
    /// route `fleet.forward` takes, checks included (forwardable, the
    /// caller's own scope for it). For node code that acts on another node
    /// for its caller, such as an import pulling a transfer (F4): the
    /// source node sees the caller's origin, so its ownership rules and
    /// its grant to the home apply there.
    pub(crate) async fn pass_on(
        &self,
        principal: &Principal,
        node: NodeId,
        call: Box<Call>,
    ) -> Result<Forwarded, WireError> {
        forwardable(call.method())?;
        principal.permit(call.method())?;
        let origin = self.origin_of(principal)?;
        if self.links.is_home() {
            self.links.call_member(node, Deliver { call, origin }).await
        } else if self.links.is_member() {
            self.links.call_home(RelayCall { node, call, origin }).await
        } else {
            Err(WireError::new(
                ErrorCode::NotFound,
                format!("this node is in no fleet, so it cannot reach node {node}"),
            ))
        }
    }

    /// `fleet.relay` (the home): pass a member's call on, or run it here.
    pub(crate) async fn relay(
        &self,
        connection: &Connection,
        request: RelayCall,
        guard: &Guarded,
    ) -> Result<Forwarded, WireError> {
        let RelayCall { node, call, origin } = request;
        forwardable(call.method())?;
        let Principal::Device { id, scopes } = connection.principal() else {
            return Err(forbidden("only a fleet member's device relays calls"));
        };
        if !self.links.is_home() {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "this node is not a fleet's home".to_owned(),
            ));
        }
        let grant = self.links.grant_of(*id).await.ok_or_else(|| {
            forbidden(
                "this member has not granted the home access yet (its node does when it \
                 connects, if it joined with `--grant`)",
            )
        })?;
        if origin.node != grant.node {
            return Err(forbidden("a member relays only calls of its own node"));
        }
        if node == self.links.node() {
            let delegated = Principal::delegated(*id, scopes, origin, self.links.node());
            return forwarded(run_here(guard, &connection.acting_as(delegated), *call).await);
        }
        let mut call = call;
        // The relaying device's share too: it names its origins itself.
        let mut shares = Waiter::origin(&origin).shares(self.links.node());
        shares.push(Share::Relay(*id));
        let _slot = self.passed_wait(connection, shares, &mut call);
        self.links.call_member(node, Deliver { call, origin }).await
    }

    /// `fleet.deliver` (a member): run a call the home delivered, as the
    /// home's device cut down to the origin.
    pub(crate) async fn deliver(
        &self,
        connection: &Connection,
        request: Deliver,
        guard: &Guarded,
    ) -> Result<Forwarded, WireError> {
        let Deliver { call, origin } = request;
        forwardable(call.method())?;
        let Principal::Device { id, scopes } = connection.principal() else {
            return Err(forbidden("only this node's fleet home delivers calls here"));
        };
        let Some(home) = self.links.home().await else {
            return Err(forbidden("this node is not a member of a fleet"));
        };
        if self.links.home_device(home).await != Some(*id) {
            return Err(forbidden("only this node's fleet home delivers calls here"));
        }
        let delegated = Principal::delegated(*id, scopes, origin, home);
        forwarded(run_here(guard, &connection.acting_as(delegated), *call).await)
    }

    /// A waiting read passed on to another node holds a call of this
    /// connection until it is answered: within this connection's and the
    /// waiter's share (`waits`), or it goes without its wait.
    fn passed_wait(
        &self,
        connection: &Connection,
        shares: Vec<Share>,
        call: &mut Call,
    ) -> Option<WaitSlot> {
        if let Call::ReadEvents(read) = call {
            self.wait_slot(connection, shares, &mut read.wait)
        } else {
            None
        }
    }

    /// Who a caller here is, for a call passed on to another node.
    fn origin_of(&self, principal: &Principal) -> Result<Origin, WireError> {
        let node = self.links.node();
        Ok(match principal {
            Principal::Owner => Origin {
                node,
                by: OriginBy::Owner,
                scopes: Scope::ALL.to_vec(),
            },
            Principal::Device { id, scopes } => Origin {
                node,
                by: OriginBy::Device { id: *id },
                scopes: scopes.clone(),
            },
            Principal::Agent {
                session, scopes, ..
            } => Origin {
                node,
                by: OriginBy::Agent {
                    session: *session,
                    depth: self
                        .agents
                        .depth(*session)
                        .ok_or_else(|| forbidden("the agent's session has ended"))?,
                },
                scopes: scopes.clone(),
            },
            Principal::Delegated { .. } => {
                return Err(forbidden(
                    "a call delivered from another node is not passed on again",
                ));
            }
        })
    }

    /// `fleet.grant` (the home): keep a member's credential.
    pub(crate) async fn grant_home(
        &self,
        connection: &Connection,
        request: GrantHome,
    ) -> Result<Ack, WireError> {
        let Principal::Device { id, .. } = connection.principal() else {
            return Err(forbidden(
                "only a fleet member's device grants its home access",
            ));
        };
        self.links
            .accept_grant(*id, &request.link)
            .await
            .map(|()| Ack)
    }

    /// `fleet.callback` (the home): its relays and a ticket, for a member's
    /// device.
    pub(crate) async fn get_callback(
        &self,
        connection: &Connection,
        request: GetCallback,
    ) -> Result<Callback, WireError> {
        let Principal::Device { id, .. } = connection.principal() else {
            return Err(forbidden(
                "only a fleet member's device asks its home for its relays",
            ));
        };
        self.links.callback(*id, request).await
    }

    /// `fleet.forget` (the home's owner; `admin`).
    pub(crate) async fn forget_node(
        &self,
        _: &Connection,
        request: ForgetNode,
    ) -> Result<NodeForgotten, WireError> {
        self.links.forget_node(request.node).await
    }

    /// `fleet.peers` (the home).
    pub(crate) async fn list_peers(
        &self,
        _: &Connection,
        _: ListPeers,
    ) -> Result<Vec<FleetNode>, WireError> {
        if !self.links.is_home() {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "this node is not a fleet's home".to_owned(),
            ));
        }
        Ok(self.links.peers(&self.lifecycle.info().name).await)
    }

    /// `fleet.nodes`: the home's view, from the home itself or asked of
    /// it; a node in no fleet is a fleet of one.
    pub(crate) async fn list_nodes(
        &self,
        _: &Connection,
        _: ListNodes,
    ) -> Result<Vec<FleetNode>, WireError> {
        let me = self.lifecycle.info();
        if self.links.is_home() {
            Ok(self.links.peers(&me.name).await)
        } else if self.links.is_member() {
            self.links.call_home(ListPeers).await
        } else {
            Ok(vec![FleetNode {
                node: me.id,
                name: me.name,
                protocol: vessel_proto::PROTOCOL_VERSION,
                reachable: true,
            }])
        }
    }
}

fn forwarded(reply: Result<Reply, WireError>) -> Result<Forwarded, WireError> {
    reply.map(|reply| Forwarded {
        reply: Box::new(reply),
    })
}

/// Run a forwardable call here through the guard. Every forwardable method
/// has its arm; every other is refused (and was refused before any hop).
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive forwarding match keeps every typed RPC visible to review"
)]
async fn run_here(
    guard: &Guarded,
    connection: &Connection,
    call: Call,
) -> Result<Reply, WireError> {
    use vessel_proto::Handler as _;
    Ok(match call {
        Call::LspWorkspaceSymbols(request) => {
            Reply::LspWorkspaceSymbols(guard.lsp_workspace_symbols(connection, request).await?)
        }
        Call::LspDocumentSymbols(request) => {
            Reply::LspDocumentSymbols(guard.lsp_document_symbols(connection, request).await?)
        }
        Call::LspDefinition(request) => {
            Reply::LspDefinition(guard.lsp_definition(connection, request).await?)
        }
        Call::LspReferences(request) => {
            Reply::LspReferences(guard.lsp_references(connection, request).await?)
        }
        Call::LspHover(request) => Reply::LspHover(guard.lsp_hover(connection, request).await?),
        Call::LspImplementations(request) => {
            Reply::LspImplementations(guard.lsp_implementations(connection, request).await?)
        }
        Call::LspTypeDefinition(request) => {
            Reply::LspTypeDefinition(guard.lsp_type_definition(connection, request).await?)
        }
        Call::LspIncomingCalls(request) => {
            Reply::LspIncomingCalls(guard.lsp_incoming_calls(connection, request).await?)
        }
        Call::LspOutgoingCalls(request) => {
            Reply::LspOutgoingCalls(guard.lsp_outgoing_calls(connection, request).await?)
        }
        Call::LspDiagnostics(request) => {
            Reply::LspDiagnostics(guard.lsp_diagnostics(connection, request).await?)
        }
        Call::NodeInfo(request) => Reply::NodeInfo(guard.node_info(connection, request).await?),
        Call::ListHarnesses(request) => {
            Reply::ListHarnesses(guard.list_harnesses(connection, request).await?)
        }
        Call::ListSessions(request) => {
            Reply::ListSessions(guard.list_sessions(connection, request).await?)
        }
        Call::GetSession(request) => {
            Reply::GetSession(guard.get_session(connection, request).await?)
        }
        Call::CreateSession(request) => {
            Reply::CreateSession(guard.create_session(connection, request).await?)
        }
        Call::OpenSideQuestion(request) => {
            Reply::OpenSideQuestion(guard.open_side_question(connection, request).await?)
        }
        Call::PromptSideQuestion(request) => {
            Reply::PromptSideQuestion(guard.prompt_side_question(connection, request).await?)
        }
        Call::ResetSideQuestion(request) => {
            Reply::ResetSideQuestion(guard.reset_side_question(connection, request).await?)
        }
        Call::SendInput(request) => Reply::SendInput(guard.send_input(connection, request).await?),
        Call::KillSession(request) => {
            Reply::KillSession(guard.kill_session(connection, request).await?)
        }
        Call::ReadEvents(request) => {
            Reply::ReadEvents(guard.read_events(connection, request).await?)
        }
        Call::MoveProfile(request) => {
            Reply::MoveProfile(guard.move_profile(connection, request).await?)
        }
        Call::ListProfiles(request) => {
            Reply::ListProfiles(guard.list_profiles(connection, request).await?)
        }
        Call::ListAgents(request) => {
            Reply::ListAgents(guard.list_agents(connection, request).await?)
        }
        Call::GetAgentStatus(request) => {
            Reply::GetAgentStatus(guard.agent_status(connection, request).await?)
        }
        Call::ExportSession(request) => {
            Reply::ExportSession(guard.export_session(connection, request).await?)
        }
        Call::ImportSession(request) => {
            Reply::ImportSession(guard.import_session(connection, request).await?)
        }
        Call::ExportMemory(request) => {
            Reply::ExportMemory(guard.export_memory(connection, request).await?)
        }
        Call::ImportMemory(request) => {
            Reply::ImportMemory(guard.import_memory(connection, request).await?)
        }
        Call::ReadTransfer(request) => {
            Reply::ReadTransfer(guard.read_transfer(connection, request).await?)
        }
        Call::CloseTransfer(request) => {
            Reply::CloseTransfer(guard.close_transfer(connection, request).await?)
        }
        Call::SetSecret(request) => Reply::SetSecret(guard.set_secret(connection, request).await?),
        Call::ListSecrets(request) => {
            Reply::ListSecrets(guard.list_secrets(connection, request).await?)
        }
        Call::ListModels(request) => {
            Reply::ListModels(guard.list_models(connection, request).await?)
        }
        Call::HarnessOptions(request) => {
            Reply::HarnessOptions(guard.harness_options(connection, request).await?)
        }
        Call::SessionOptions(request) => {
            Reply::SessionOptions(guard.session_options(connection, request).await?)
        }
        Call::SetSessionOption(request) => {
            Reply::SetSessionOption(guard.set_session_option(connection, request).await?)
        }
        Call::RenameSession(request) => {
            Reply::RenameSession(guard.rename_session(connection, request).await?)
        }
        Call::PromptSession(request) => {
            Reply::PromptSession(guard.prompt_session(connection, request).await?)
        }
        Call::ChangeQueue(request) => {
            Reply::ChangeQueue(guard.change_queue(connection, request).await?)
        }
        Call::GetConversation(request) => {
            Reply::GetConversation(guard.get_conversation(connection, request).await?)
        }
        Call::ResumeSession(request) => {
            Reply::ResumeSession(guard.resume_session(connection, request).await?)
        }
        Call::MarkSession(request) => {
            Reply::MarkSession(guard.mark_session(connection, request).await?)
        }
        Call::DeleteSession(request) => {
            Reply::DeleteSession(guard.delete_session(connection, request).await?)
        }
        Call::BeginUpload(request) => {
            Reply::BeginUpload(guard.begin_upload(connection, request).await?)
        }
        Call::WriteUpload(request) => {
            Reply::WriteUpload(guard.write_upload(connection, request).await?)
        }
        Call::FinishUpload(request) => {
            Reply::FinishUpload(guard.finish_upload(connection, request).await?)
        }
        Call::ReadAttachment(request) => {
            Reply::ReadAttachment(guard.read_attachment(connection, request).await?)
        }
        Call::ListProjects(request) => {
            Reply::ListProjects(guard.list_projects(connection, request).await?)
        }
        Call::GetProject(request) => {
            Reply::GetProject(guard.get_project(connection, request).await?)
        }
        Call::AddProject(request) => {
            Reply::AddProject(guard.add_project(connection, request).await?)
        }
        Call::CloneProject(request) => {
            Reply::CloneProject(guard.clone_project(connection, request).await?)
        }
        Call::InitProject(request) => {
            Reply::InitProject(guard.init_project(connection, request).await?)
        }
        Call::RemoveProject(request) => {
            Reply::RemoveProject(guard.remove_project(connection, request).await?)
        }
        Call::ProjectCommands(request) => {
            Reply::ProjectCommands(guard.project_commands(connection, request).await?)
        }
        Call::ApproveCommands(request) => {
            Reply::ApproveCommands(guard.approve_commands(connection, request).await?)
        }
        Call::ListWorktrees(request) => {
            Reply::ListWorktrees(guard.list_worktrees(connection, request).await?)
        }
        Call::CreateWorktree(request) => {
            Reply::CreateWorktree(guard.create_worktree(connection, request).await?)
        }
        Call::SetupOutput(request) => {
            Reply::SetupOutput(guard.setup_output(connection, request).await?)
        }
        Call::RunSetup(request) => Reply::RunSetup(guard.run_setup(connection, request).await?),
        Call::RemoveWorktree(request) => {
            Reply::RemoveWorktree(guard.remove_worktree(connection, request).await?)
        }
        Call::ListSnapshots(request) => {
            Reply::ListSnapshots(guard.list_snapshots(connection, request).await?)
        }
        Call::DropSnapshot(request) => {
            Reply::DropSnapshot(guard.drop_snapshot(connection, request).await?)
        }
        Call::GitStatus(request) => Reply::GitStatus(guard.git_status(connection, request).await?),
        Call::GitChanges(request) => {
            Reply::GitChanges(guard.git_changes(connection, request).await?)
        }
        Call::GitDiff(request) => Reply::GitDiff(guard.git_diff(connection, request).await?),
        Call::GitShow(request) => Reply::GitShow(guard.git_show(connection, request).await?),
        Call::GitBranches(request) => {
            Reply::GitBranches(guard.git_branches(connection, request).await?)
        }
        Call::GitLog(request) => Reply::GitLog(guard.git_log(connection, request).await?),
        Call::GitStashes(request) => {
            Reply::GitStashes(guard.git_stashes(connection, request).await?)
        }
        Call::GitStage(request) => Reply::GitStage(guard.git_stage(connection, request).await?),
        Call::GitUnstage(request) => {
            Reply::GitUnstage(guard.git_unstage(connection, request).await?)
        }
        Call::GitDiscard(request) => {
            Reply::GitDiscard(guard.git_discard(connection, request).await?)
        }
        Call::GitCommit(request) => Reply::GitCommit(guard.git_commit(connection, request).await?),
        Call::GitFetch(request) => Reply::GitFetch(guard.git_fetch(connection, request).await?),
        Call::GitPush(request) => Reply::GitPush(guard.git_push(connection, request).await?),
        Call::GitPull(request) => Reply::GitPull(guard.git_pull(connection, request).await?),
        Call::GitSwitch(request) => Reply::GitSwitch(guard.git_switch(connection, request).await?),
        Call::GitRenameBranch(request) => {
            Reply::GitRenameBranch(guard.git_rename_branch(connection, request).await?)
        }
        Call::GitStash(request) => Reply::GitStash(guard.git_stash(connection, request).await?),
        Call::ListDir(request) => Reply::ListDir(guard.list_dir(connection, request).await?),
        Call::ReadFile(request) => Reply::ReadFile(guard.read_file(connection, request).await?),
        Call::WriteFile(request) => Reply::WriteFile(guard.write_file(connection, request).await?),
        Call::CreateDir(request) => Reply::CreateDir(guard.create_dir(connection, request).await?),
        Call::RenamePath(request) => {
            Reply::RenamePath(guard.rename_path(connection, request).await?)
        }
        Call::DeletePath(request) => {
            Reply::DeletePath(guard.delete_path(connection, request).await?)
        }
        Call::FindFiles(request) => Reply::FindFiles(guard.find_files(connection, request).await?),
        Call::SearchFiles(request) => {
            Reply::SearchFiles(guard.search_files(connection, request).await?)
        }
        Call::ForgeStatus(request) => {
            Reply::ForgeStatus(guard.forge_status(connection, request).await?)
        }
        Call::ListPullRequests(request) => {
            Reply::ListPullRequests(guard.list_pull_requests(connection, request).await?)
        }
        Call::GetPullRequest(request) => {
            Reply::GetPullRequest(guard.get_pull_request(connection, request).await?)
        }
        Call::OpenPullRequest(request) => {
            Reply::OpenPullRequest(guard.open_pull_request(connection, request).await?)
        }
        Call::MergePullRequest(request) => {
            Reply::MergePullRequest(guard.merge_pull_request(connection, request).await?)
        }
        Call::SetFeedback(request) => {
            Reply::SetFeedback(guard.set_feedback(connection, request).await?)
        }
        Call::CreateTerminal(request) => {
            Reply::CreateTerminal(guard.create_terminal(connection, request).await?)
        }
        Call::ListTerminals(request) => {
            Reply::ListTerminals(guard.list_terminals(connection, request).await?)
        }
        Call::GetTerminal(request) => {
            Reply::GetTerminal(guard.get_terminal(connection, request).await?)
        }
        Call::ReadTerminal(request) => {
            Reply::ReadTerminal(guard.read_terminal(connection, request).await?)
        }
        Call::SendTerminal(request) => {
            Reply::SendTerminal(guard.send_terminal(connection, request).await?)
        }
        Call::RenameTerminal(request) => {
            Reply::RenameTerminal(guard.rename_terminal(connection, request).await?)
        }
        Call::KillTerminal(request) => {
            Reply::KillTerminal(guard.kill_terminal(connection, request).await?)
        }
        Call::CloseTerminal(request) => {
            Reply::CloseTerminal(guard.close_terminal(connection, request).await?)
        }
        Call::ListScripts(request) => {
            Reply::ListScripts(guard.list_scripts(connection, request).await?)
        }
        Call::RunScript(request) => Reply::RunScript(guard.run_script(connection, request).await?),
        Call::RestartScript(request) => {
            Reply::RestartScript(guard.restart_script(connection, request).await?)
        }
        Call::ListPorts(request) => Reply::ListPorts(guard.list_ports(connection, request).await?),
        Call::JoinFleet(request) => Reply::JoinFleet(guard.join_fleet(connection, request).await?),
        Call::LeaveFleet(request) => {
            Reply::LeaveFleet(guard.leave_fleet(connection, request).await?)
        }
        Call::SyncFleet(request) => Reply::SyncFleet(guard.sync_fleet(connection, request).await?),
        Call::GetDeviceSelf(request) => {
            Reply::GetDeviceSelf(guard.device_self(connection, request).await?)
        }
        Call::GetFleetStatus(request) => {
            Reply::GetFleetStatus(guard.fleet_status(connection, request).await?)
        }
        Call::GetPublished(request) => {
            Reply::GetPublished(guard.fleet_published(connection, request).await?)
        }
        Call::PreviewSchedule(request) => {
            Reply::PreviewSchedule(guard.preview_schedule(connection, request).await?)
        }
        Call::ListSchedules(request) => {
            Reply::ListSchedules(guard.list_schedules(connection, request).await?)
        }
        Call::GetSchedule(request) => {
            Reply::GetSchedule(guard.get_schedule(connection, request).await?)
        }
        Call::CreateSchedule(request) => {
            Reply::CreateSchedule(guard.create_schedule(connection, request).await?)
        }
        Call::UpdateSchedule(request) => {
            Reply::UpdateSchedule(guard.update_schedule(connection, request).await?)
        }
        Call::PauseSchedule(request) => {
            Reply::PauseSchedule(guard.pause_schedule(connection, request).await?)
        }
        Call::ResumeSchedule(request) => {
            Reply::ResumeSchedule(guard.resume_schedule(connection, request).await?)
        }
        Call::RunSchedule(request) => {
            Reply::RunSchedule(guard.run_schedule(connection, request).await?)
        }
        Call::DeleteSchedule(request) => {
            Reply::DeleteSchedule(guard.delete_schedule(connection, request).await?)
        }
        Call::ListRuns(request) => Reply::ListRuns(guard.list_runs(connection, request).await?),
        Call::GetUsageLimits(request) => {
            Reply::GetUsageLimits(guard.get_usage_limits(connection, request).await?)
        }
        Call::SetUsageLookup(request) => {
            Reply::SetUsageLookup(guard.set_usage_lookup(connection, request).await?)
        }
        Call::QueryUsage(request) => {
            Reply::QueryUsage(guard.query_usage(connection, request).await?)
        }
        Call::ListPrompts(request) => {
            Reply::ListPrompts(guard.list_prompts(connection, request).await?)
        }
        Call::CreatePrompt(request) => {
            Reply::CreatePrompt(guard.create_prompt(connection, request).await?)
        }
        Call::UpdatePrompt(request) => {
            Reply::UpdatePrompt(guard.update_prompt(connection, request).await?)
        }
        Call::ListInbox(request) => Reply::ListInbox(guard.list_inbox(connection, request).await?),
        Call::GetInboxEntry(request) => {
            Reply::GetInboxEntry(guard.get_inbox_entry(connection, request).await?)
        }
        Call::MarkInbox(request) => Reply::MarkInbox(guard.mark_inbox(connection, request).await?),
        Call::GetPushKey(request) => {
            Reply::GetPushKey(guard.get_push_key(connection, request).await?)
        }
        Call::SubscribePush(request) => {
            Reply::SubscribePush(guard.subscribe_push(connection, request).await?)
        }
        Call::UnsubscribePush(request) => {
            Reply::UnsubscribePush(guard.unsubscribe_push(connection, request).await?)
        }
        Call::ReportPushPresence(request) => {
            Reply::ReportPushPresence(guard.report_push_presence(connection, request).await?)
        }
        Call::DeletePrompt(request) => {
            Reply::DeletePrompt(guard.delete_prompt(connection, request).await?)
        }
        Call::SearchMessages(request) => {
            Reply::SearchMessages(guard.search_messages(connection, request).await?)
        }
        Call::ParkSession(request) => {
            Reply::ParkSession(guard.park_session(connection, request).await?)
        }
        Call::WakeSession(request) => {
            Reply::WakeSession(guard.wake_session(connection, request).await?)
        }
        Call::ForkSession(request) => {
            Reply::ForkSession(guard.fork_session(connection, request).await?)
        }
        Call::RewindSession(request) => {
            Reply::RewindSession(guard.rewind_session(connection, request).await?)
        }
        Call::GetAgentHistory(request) => {
            Reply::GetAgentHistory(guard.agent_history(connection, request).await?)
        }
        Call::StopNode(_)
        | Call::GetNodeSettings(_)
        | Call::SetNodeSettings(_)
        | Call::AddAgent(_)
        | Call::GetCatalogue(_)
        | Call::InstallAgent(_)
        | Call::ReadInstall(_)
        | Call::RemoveAgent(_)
        | Call::InviteDevice(_)
        | Call::ListDevices(_)
        | Call::RevokeDevice(_)
        | Call::ReloadProfiles(_)
        | Call::DeleteSecret(_)
        | Call::CopySecret(_)
        | Call::Forward(_)
        | Call::Relay(_)
        | Call::Deliver(_)
        | Call::GrantHome(_)
        | Call::GetCallback(_)
        | Call::ForgetNode(_)
        | Call::ListPeers(_)
        | Call::ListNodes(_)
        | Call::GetManifest(_)
        | Call::ReadBlob(_)
        | Call::WatchFleet(_)
        | Call::ReportApplied(_)
        | Call::ListMembers(_)
        | Call::Subscribe(_)
        | Call::SubscribeTerminal(_)
        | Call::Unsubscribe(_) => return Err(not_forwardable(call.method())),
    })
}
