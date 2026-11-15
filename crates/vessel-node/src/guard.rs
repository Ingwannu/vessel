//! The scope check (ADR 0010): every call is checked against the scope its
//! method declares, before it runs.
//!
//! [Decision Log]
//! - Purpose: a paired device may only call what its scopes allow, and no
//!   method may be added without the check.
//! - Constraint: `vessel-proto`'s `Handler` and `dispatch` are frozen for
//!   milestone 3, so the check cannot live in the generated dispatch, and
//!   `vessel-rpc` knows nothing of principals.
//! - Alternatives: a check at the top of every `NodeService` method (one
//!   forgotten line is a hole); a check in `vessel-rpc` before dispatch (a
//!   principal concept in the transport layer).
//! - Choice: [`Guarded`] wraps the service and implements `Handler` itself,
//!   generated from one row per method. Every generated method runs the
//!   same check first, with the method taken from the request type
//!   (`Request::METHOD`), so a row cannot check the wrong scope. A method
//!   added to the protocol is a new `Handler` method, which does not compile
//!   here until it gets its row, and the row cannot leave the check out.
//! - `NodeService`'s calls are inherent methods, not a `Handler`, so the
//!   service cannot be handed to a `Server` without this wrapper.
//! - The rows are not repeated here: `vessel_proto::method_table!` hands
//!   the protocol table's rows to this module's macro (Quality B), so a
//!   new method gets its guarded handler from its one table row.
//!
//! [Decision Log: agents and workspaces] (protocol 10, ADR 0027)
//! - Agents get, of the workspace calls, only what orchestration needs
//!   (`workspace::policy`); the rest are refused right after the scope,
//!   whatever the agent's scopes, by the same generated method, so a new
//!   workspace call is held to the list from its first row.
//!
//! [Decision Log: ownership] (protocol 5, ADR 0013)
//! - Agents may `operate` only on sessions their own session started (or
//!   that those started). Every request type declares which session it
//!   operates on ([`Target`]); the generated method checks ownership right
//!   after the scope, so, as with scopes, a new method does not compile
//!   until it says. Reads (`session.get`, `session.read`,
//!   `session.subscribe`) name a session too but are not ownership-checked:
//!   an agent sees the fleet's sessions, as ADR 0013 grants.

use vessel_proto::{
    CloseTransfer, CopySecret, CreateSession, DeleteSecret, Deliver, ExportMemory, ExportSession,
    ForgetNode, Forward, GetCallback, GetManifest, GetNode, GetSession, GrantHome, Handler,
    ImportMemory, ImportSession, InviteDevice, KillSession, ListDevices, ListHarnesses,
    ListMembers, ListModels, ListNodes, ListPeers, ListProfiles, ListSecrets, ListSessions,
    MoveProfile, ReadBlob, ReadEvents, ReadTransfer, RelayCall, ReloadProfiles, RenameSession,
    ReportApplied, Request, RevokeDevice, SendInput, SessionId, SetSecret, SetSessionOption,
    StopNode, Subscribe, Unsubscribe, WatchFleet, WireError,
};

use crate::service::{Connection, NodeService};

/// The node's service behind the scope check. This is the handler every
/// transport serves.
#[derive(Debug)]
pub(crate) struct Guarded(pub(crate) NodeService);

// The guard's `Handler`, generated from the one method table
// (`vessel_proto::method_table!`), so a new method needs no row here: its
// table row is enough, and the check cannot be left out (Quality B). The
// envelope methods (shape `envelope`) are handed this guard, so the call
// they carry runs through it too (protocol 6, `envelope`); they are not
// held to the agents' workspace list, which applies to what they carry.
macro_rules! guarded {
    ($(
        $(#[$doc:meta])*
        $variant:ident / $handler:ident / $wire:literal
            [$scope:ident, $protocol:tt, $shape:ident] : $request:ty => $response:ty;
    )*) => {
        impl Handler for Guarded {
            type Context = Connection;

            $( guarded_method!($protocol, $shape, $handler, $request, $response); )*
        }
    };
}

// A staged row (`(staged FEATURE)`: values without a runtime) gets nothing
// here: the protocol's `Handler` answers it `Unsupported` by default, and
// no `NodeService` method exists to forward to. The package that wires it
// turns the row into `(pending FEATURE)`, and from then on this macro
// requires its service method like any other row's.
macro_rules! guarded_method {
    ((staged $feature:path), $shape:ident, $handler:ident, $request:ty, $response:ty) => {};
    ($protocol:tt, envelope, $handler:ident, $request:ty, $response:ty) => {
        async fn $handler(
            &self,
            connection: &Connection,
            request: $request,
        ) -> Result<$response, WireError> {
            connection
                .principal()
                .permit(<$request as Request>::METHOD)?;
            if let Some(session) = Target::target(&request) {
                self.0.check_owner(connection.principal(), session).await?;
                self.0.refuse_older(connection, session).await?;
            }
            self.0.$handler(connection, request, self).await
        }
    };
    ($protocol:tt, $shape:ident, $handler:ident, $request:ty, $response:ty) => {
        async fn $handler(
            &self,
            connection: &Connection,
            request: $request,
        ) -> Result<$response, WireError> {
            connection
                .principal()
                .permit(<$request as Request>::METHOD)?;
            crate::workspace::refuse_agents(connection.principal(), <$request as Request>::METHOD)?;
            if let Some(session) = Target::target(&request) {
                self.0.check_owner(connection.principal(), session).await?;
                self.0.refuse_older(connection, session).await?;
            }
            self.0.$handler(connection, request).await
        }
    };
}

/// The session an `operate` call acts on, for the ownership rule.
trait Target {
    fn target(&self) -> Option<SessionId>;
}

macro_rules! targets {
    ($( $request:ty ),* $(,)?) => {
        $(
            impl Target for $request {
                fn target(&self) -> Option<SessionId> {
                    Some(self.session)
                }
            }
        )*
    };
}

macro_rules! untargeted {
    ($( $request:ty ),* $(,)?) => {
        $(
            impl Target for $request {
                fn target(&self) -> Option<SessionId> {
                    None
                }
            }
        )*
    };
}

// `session.export` copies a session's conversation away: an agent may do
// that only to a session it started, like any other `operate` call on one.
// Protocol 9: changing a session's options or its title steers it, like
// sending it input.
// Protocol 10: prompting, the queue, starting again, marks, deleting and
// uploading to a session all act on it, as sending it input does. An
// upload's later pieces name the upload, whose holder check keeps them to
// the caller that began it (`uploads`).
targets!(
    // Protocol 12: parking and waking act on a session as a prompt does;
    // protocol 14: forking reads it and rewinding acts on it (ADR 0052).
    vessel_proto::parking::ParkSession,
    vessel_proto::parking::WakeSession,
    vessel_proto::forking::ForkSession,
    vessel_proto::forking::RewindSession,
    SendInput,
    KillSession,
    MoveProfile,
    ExportSession,
    SetSessionOption,
    RenameSession,
    vessel_proto::conversation::PromptSession,
    vessel_proto::conversation::ChangeQueue,
    vessel_proto::conversation::ResumeSession,
    vessel_proto::conversation::MarkSession,
    vessel_proto::conversation::DeleteSession,
    vessel_proto::conversation::BeginUpload,
    // Protocol 11: opening, merging and the feedback switch act on a
    // session's branch and its conversation.
    vessel_proto::forge::OpenPullRequest,
    vessel_proto::forge::MergePullRequest,
    vessel_proto::forge::SetFeedback,
);

// `session.create` is limited by `agents` instead (count and depth), and so
// is `session.import`, which creates one; the reads are open to every
// agent; the envelope's rows carry a call whose own row is checked where it
// runs (protocol 6, Track X); the rest are not `operate` calls.
// Schedules name schedules, not sessions: who may change one is the
// scheduler's rule (ADR 0033), and a run is checked as its definer.
// Saved prompts name none either: agents may only read them.
untargeted!(
    vessel_proto::GetDeviceSelf,
    vessel_proto::usage::QueryUsage,
    vessel_proto::inbox::ListInbox,
    vessel_proto::inbox::MarkInbox,
    vessel_proto::inbox::GetInboxEntry,
    vessel_proto::push::GetPushKey,
    vessel_proto::push::SubscribePush,
    vessel_proto::push::UnsubscribePush,
    vessel_proto::push::ReportPushPresence,
    vessel_proto::limits::GetUsageLimits,
    vessel_proto::limits::SetUsageLookup,
    // Protocol 12: a search reads, and reads are not ownership-checked.
    vessel_proto::search::SearchMessages,
    // The node's settings: no session's.
    vessel_proto::settings::GetNodeSettings,
    vessel_proto::settings::SetNodeSettings,
    // An agent's own history (ADR 0053): no session's, and refused to
    // agents (`workspace::policy`).
    vessel_proto::history::GetAgentHistory,
    vessel_proto::composer_commands::GetDraftCommands,
    vessel_proto::automation::ListPrompts,
    vessel_proto::automation::CreatePrompt,
    vessel_proto::automation::UpdatePrompt,
    vessel_proto::automation::DeletePrompt,
    vessel_proto::JoinFleet,
    vessel_proto::LeaveFleet,
    vessel_proto::SyncFleet,
    vessel_proto::GetFleetStatus,
    vessel_proto::GetPublished,
    vessel_proto::automation::PreviewSchedule,
    vessel_proto::automation::ListSchedules,
    vessel_proto::automation::GetSchedule,
    vessel_proto::automation::CreateSchedule,
    vessel_proto::automation::UpdateSchedule,
    vessel_proto::automation::PauseSchedule,
    vessel_proto::automation::ResumeSchedule,
    vessel_proto::automation::RunSchedule,
    vessel_proto::automation::DeleteSchedule,
    vessel_proto::automation::ListRuns,
    vessel_proto::conversation::GetConversation,
    vessel_proto::conversation::ReadAttachment,
    vessel_proto::conversation::WriteUpload,
    vessel_proto::conversation::FinishUpload,
    ListModels,
    vessel_proto::GetHarnessOptions,
    vessel_proto::GetSessionOptions,
    GetNode,
    ListHarnesses,
    ListSessions,
    GetSession,
    CreateSession,
    vessel_proto::side_questions::OpenSideQuestion,
    vessel_proto::side_questions::PromptSideQuestion,
    vessel_proto::side_questions::ResetSideQuestion,
    ReadEvents,
    Subscribe,
    Unsubscribe,
    StopNode,
    InviteDevice,
    ListDevices,
    RevokeDevice,
    ListProfiles,
    ReloadProfiles,
    GetManifest,
    ReadBlob,
    WatchFleet,
    ReportApplied,
    ListMembers,
    Forward,
    RelayCall,
    Deliver,
    GrantHome,
    GetCallback,
    ForgetNode,
    ListPeers,
    ListNodes,
    ImportSession,
    ExportMemory,
    ImportMemory,
    ReadTransfer,
    CloseTransfer,
    SetSecret,
    ListSecrets,
    DeleteSecret,
    CopySecret,
    vessel_proto::lsp::WorkspaceSymbols,
    vessel_proto::lsp::DocumentSymbols,
    vessel_proto::lsp::Definition,
    vessel_proto::lsp::References,
    vessel_proto::lsp::GetHover,
    vessel_proto::lsp::Implementations,
    vessel_proto::lsp::TypeDefinition,
    vessel_proto::lsp::IncomingCalls,
    vessel_proto::lsp::OutgoingCalls,
    vessel_proto::lsp::Diagnostics,
    vessel_proto::workspace::ListProjects,
    vessel_proto::workspace::GetProject,
    vessel_proto::workspace::AddProject,
    vessel_proto::workspace::CloneProject,
    vessel_proto::workspace::InitProject,
    vessel_proto::workspace::RemoveProject,
    vessel_proto::workspace::GetProjectCommands,
    vessel_proto::workspace::ApproveCommands,
    vessel_proto::workspace::ListWorktrees,
    vessel_proto::workspace::CreateWorktree,
    vessel_proto::workspace::ReadSetupOutput,
    vessel_proto::workspace::RunSetup,
    vessel_proto::workspace::RemoveWorktree,
    vessel_proto::workspace::ListSnapshots,
    vessel_proto::workspace::DropSnapshot,
    vessel_proto::workspace::GetStatus,
    vessel_proto::workspace::GetChanges,
    vessel_proto::workspace::GetFileDiff,
    vessel_proto::workspace::ShowFile,
    vessel_proto::workspace::ListBranches,
    vessel_proto::workspace::ReadLog,
    vessel_proto::workspace::ListStashes,
    vessel_proto::workspace::Stage,
    vessel_proto::workspace::Unstage,
    vessel_proto::workspace::Discard,
    vessel_proto::workspace::Commit,
    vessel_proto::workspace::Fetch,
    vessel_proto::workspace::Push,
    vessel_proto::workspace::Pull,
    vessel_proto::workspace::Switch,
    vessel_proto::workspace::RenameBranch,
    vessel_proto::workspace::StashChanges,
    vessel_proto::workspace::ListDir,
    vessel_proto::workspace::ReadFile,
    vessel_proto::workspace::WriteFile,
    vessel_proto::workspace::CreateDir,
    vessel_proto::workspace::RenamePath,
    vessel_proto::workspace::DeletePath,
    vessel_proto::workspace::FindFiles,
    vessel_proto::workspace::SearchFiles,
    // Protocol 11: reads; `pr.get`'s refresh is held to ownership by the
    // service (ADR 0029).
    vessel_proto::forge::GetForgeStatus,
    vessel_proto::forge::ListPullRequests,
    vessel_proto::forge::GetPullRequest,
    // Protocol 11 (ADR 0031): a terminal names a session it is beside, not
    // one it operates; who may open one and use it is the terminals' rule
    // (`service::terminals`), which checks an agent's ownership of the
    // session it opens one beside.
    vessel_proto::terminals::CreateTerminal,
    vessel_proto::terminals::ListTerminals,
    vessel_proto::terminals::GetTerminal,
    vessel_proto::terminals::ReadTerminal,
    vessel_proto::terminals::SubscribeTerminal,
    vessel_proto::terminals::SendTerminal,
    vessel_proto::terminals::RenameTerminal,
    vessel_proto::terminals::KillTerminal,
    vessel_proto::terminals::CloseTerminal,
    vessel_proto::scripts::ListScripts,
    vessel_proto::scripts::RunScript,
    vessel_proto::scripts::RestartScript,
    vessel_proto::ports::ListPorts,
    // Protocol 11's agents act on no session; adding and removing one is
    // refused to agents by the service itself (ADR 0032).
    vessel_proto::agents::ListAgents,
    vessel_proto::agents::AddAgent,
    vessel_proto::agents::RemoveAgent,
    vessel_proto::agents::GetAgentStatus,
    vessel_proto::agents::GetCatalogue,
    vessel_proto::agents::InstallAgent,
    vessel_proto::agents::ReadInstall,
);

vessel_proto::method_table!(guarded);

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::*;

    /// Method resolution on a concrete type picks `IsHandler` when the
    /// type implements `Handler`, and falls back to `NotHandler` (one
    /// autoref further) when it does not: a test of a missing impl that
    /// stable Rust can write.
    struct Probe<T>(PhantomData<T>);

    trait IsHandler {
        fn is_handler(&self) -> bool {
            true
        }
    }

    impl<T: Handler> IsHandler for Probe<T> {}

    trait NotHandler {
        fn is_handler(&self) -> bool {
            false
        }
    }

    impl<T> NotHandler for &Probe<T> {}

    /// Only the guarded service can be handed to a `Server`; the service
    /// itself has no `Handler` impl to serve without the scope check.
    #[test]
    fn only_the_guarded_service_is_a_handler() {
        assert!(Probe::<Guarded>(PhantomData).is_handler());
        assert!(!(&Probe::<NodeService>(PhantomData)).is_handler());
    }
}
