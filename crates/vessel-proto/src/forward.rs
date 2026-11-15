//! Calls across a fleet (protocol 6, ADR 0013 "across nodes through the
//! home"): the envelope a call travels in from its caller's node, through
//! the home, to the node that runs it, and what members learn of the fleet.
//!
//! [Decision Log]
//! - Purpose: an agent (or a person's CLI) reaches only its own node, yet
//!   may need to act on a session on another node of its fleet, with no
//!   more authority there than it has at home.
//! - Constraint: only the home holds credentials on every node; a member
//!   holds one on the home only. A target must be able to tell who a call
//!   speaks for, and the guard is generated from the method table.
//! - Alternatives: a `node` field on every request (every type changes, and
//!   the guard would rediscover the intersection per method); relaying
//!   subscriptions (a stream per waiting agent on each hop, with lag and
//!   resume on both); a mesh of direct links (ADR 0013 option 1, later).
//! - Choice: three rows, each with a real scope for its own hop:
//!   `fleet.forward` (caller to its node), `fleet.relay` (member to home,
//!   scope `relay`, which the home's owner grants per member) and
//!   `fleet.deliver` (home to target, the last hop: never passed on, so at
//!   most two hops and no loops). Each carries an [`Origin`] its first node
//!   vouches for; the target runs the inner call with its delivering
//!   device's grant intersected with `origin.scopes`, and applies ownership
//!   and limits to the origin.
//! - Trade-off: a call to another member takes two hops until a direct
//!   link replaces the home's; waiting across nodes is a long poll
//!   (`session.read` with `wait`), not a stream.

use serde::{Deserialize, Serialize};

use crate::device::Scope;
use crate::id::{DeviceId, NodeId, SessionId};
use crate::key::NodePublicKey;
use crate::node::ProtocolVersion;
use crate::pairing::{PairingLink, RouteUrl};
use crate::relay::RelayTicket;
use crate::rpc::{Call, Method, Reply};

/// Who a forwarded call speaks for, as the node it started on vouches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Origin {
    /// The node the call started on.
    pub node: NodeId,
    /// The principal there.
    pub by: OriginBy,
    /// What that principal holds on its own node. The target never grants
    /// more than these, whatever its own grant to the delivering device.
    pub scopes: Vec<Scope>,
}

/// The principal a call started with, on its own node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OriginBy {
    /// That node's token or local socket.
    Owner,
    /// A device paired with that node.
    Device {
        /// Its id on that node.
        id: DeviceId,
    },
    /// An agent, through its session's credential on that node.
    Agent {
        /// The agent's session.
        session: SessionId,
        /// How many agents deep that session is (the owner's own sessions
        /// are 0), so the target can refuse a session past the depth limit.
        depth: u8,
    },
}

/// `fleet.forward` (protocol 6): run `call` on fleet node `node`. Sent by a
/// client to its own node, which checks `call` against the caller as if it
/// were local, then passes it on with the caller's [`Origin`]. Needs `read`
/// here; the inner call's own scope is checked on every hop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Forward {
    /// The node that runs the call; this node itself runs it locally.
    pub node: NodeId,
    /// The call, which must be [forwardable](Method::forwardable).
    pub call: Box<Call>,
}

/// `fleet.relay` (protocol 6): a member asks its home to pass a call on.
/// The home refuses an `origin.node` other than the member's own (a member
/// speaks only for its own principals). Needs `relay`, which a member gets
/// only when the home's owner asks for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RelayCall {
    /// The node that runs the call; the home runs it itself when it is
    /// the target.
    pub node: NodeId,
    /// The call.
    pub call: Box<Call>,
    /// Who it speaks for.
    pub origin: Origin,
}

/// `fleet.deliver` (protocol 6): the home asks a member to run a call for
/// `origin`. The last hop: a member never passes a delivered call on, and
/// accepts one only from its home's device (the one `fleet.grant` made).
/// Needs `read`; the inner call runs with the delivering device's grant
/// intersected with `origin.scopes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Deliver {
    /// The call.
    pub call: Box<Call>,
    /// Who it speaks for.
    pub origin: Origin,
}

/// Reply to `fleet.forward`, `fleet.relay` and `fleet.deliver`: the inner
/// call's reply. An inner call that fails fails the envelope with its error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Forwarded {
    /// What the target answered.
    pub reply: Box<Reply>,
}

/// `fleet.grant` (protocol 6): a member gives its home a credential on
/// itself, as a single-use pairing link made with its own `device.invite`
/// and the scopes its owner chose. The home redeems it like any device and
/// keeps the credential; revoking it on the member cuts the home off.
/// Needs `sync`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GrantHome {
    /// The link, made on the member.
    pub link: PairingLink,
}

/// `fleet.callback` (protocol 6): what a member with no route of its own
/// needs for its home to call it back: the relays the home registers with,
/// and a ticket that registers this member's own node key there for a while
/// (never the relays' token). The home answers only a member device, for
/// the node and key its record binds that device to, or, before its first
/// grant, for a claim its record would take. Asked again before the ticket
/// expires. Needs `sync`; never forwarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetCallback {
    /// The member's node.
    pub node: NodeId,
    /// Its key, which the ticket names.
    pub key: NodePublicKey,
}

/// What `fleet.callback` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Callback {
    /// The relays the home registers with, in its preference order; none
    /// when the home has no relay (it is reached directly), and then a
    /// member needs a route of its own to be called back.
    pub relays: Vec<RouteUrl>,
    /// A ticket for the member's key, with `relays`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ticket: Option<RelayTicket>,
}

/// `fleet.forget` (protocol 6): on the home, forget member node `node`'s
/// binding (its key and name, kept for good otherwise), for a node
/// reinstalled with a new key or to be renamed. Its grants are deleted;
/// the invitations outstanding on the home are withdrawn, and the node's
/// id and name go next only to a member device paired after this, so the
/// node is invited again and joins from there. Needs `admin`; never
/// forwarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ForgetNode {
    /// The member node.
    pub node: NodeId,
}

/// What `fleet.forget` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct NodeForgotten {
    /// The key the node was bound to.
    pub key: NodePublicKey,
    /// The name it was known by, now released.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The member devices that granted for it; their grants are deleted.
    /// One that should not sync any more is revoked separately.
    pub devices: Vec<DeviceId>,
    /// Invitations that were outstanding on the home and are withdrawn.
    pub withdrawn: u32,
}

/// `fleet.peers` (protocol 6): the home's view of its fleet, for its
/// members. Needs `sync`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListPeers;

/// `fleet.nodes` (protocol 6): what this node knows of its fleet (a member
/// asks its home with `fleet.peers`). Needs `read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListNodes;

/// A node of a fleet, the home included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FleetNode {
    /// Its id.
    pub node: NodeId,
    /// Its name.
    pub name: String,
    /// The protocol its home speaks with it (a protocol 5 member gets the
    /// bundle but no delivered calls).
    pub protocol: ProtocolVersion,
    /// Whether the home can reach it now.
    pub reachable: bool,
}

impl Method {
    /// Whether a call of this method can travel in `fleet.forward`,
    /// `fleet.relay` and `fleet.deliver`. Deny by default: the `match` names
    /// every method, so a new row does not compile until someone decides.
    /// A forwarded call still runs with the target's grant to its home
    /// intersected with the origin's scopes.
    ///
    /// - Forwardable, the work agents and people do on another node: node
    ///   and harness info, sessions (list, read, create, send, kill, move
    ///   between profiles), profiles to choose from, and the transfers F4
    ///   pulls through the home (export, import, read, close), with
    ///   `secret.set`, which `secret.copy` sends to its target, and
    ///   `secret.list` (names only), which it asks the target first, so a
    ///   target that would refuse the value never receives it, and
    ///   `model.list` (protocol 7: route names, providers and model ids,
    ///   never a URL or a secret name), which a picker of profiles for a
    ///   session there needs as much as the profiles, and protocol 9's
    ///   options and titles (`harness.options`, `session.options`,
    ///   `session.set_option`, `session.rename`), which starting and
    ///   steering a session there needs, and protocol 10's conversation
    ///   (`session.prompt`, `session.queue`, `session.conversation`,
    ///   `session.resume`, `session.mark`, `session.delete`, and the
    ///   `attachment.*` calls), which talking to a session there needs as
    ///   much as `session.send` does, and protocol 10's projects,
    ///   worktrees, git and files (`project.*`, `worktree.*`, `git.*`,
    ///   `file.*`), which working in a project there needs; the roots that
    ///   would widen a reader's reach are refused for every caller instead
    ///   (ADR 0027), and protocol 11's `agent.list`, read as `harness.list`
    ///   is.
    /// - Not forwardable, administration of a node done there: stopping it,
    ///   its devices, re-reading its profiles, deleting or copying its
    ///   secrets, adding or removing its agents (ADR 0032).
    /// - Not forwardable, the fleet's own plumbing: an envelope is never
    ///   wrapped in another (at most two hops, no loops), and the bundle,
    ///   grant and membership calls belong to the hop between a member and
    ///   its home.
    /// - Not forwardable, streams: a subscription's events would need a
    ///   stream across hops (`session.read` with `wait` serves instead, and
    ///   `terminal.read` with `wait` for a terminal).
    /// - Forwardable, protocol 11's terminals, scripts and ports
    ///   (`terminal.*` but its subscription, `script.*`, `port.list`): an
    ///   agent opens a shell beside a session it started on another node,
    ///   and a person runs a project's service there (ADR 0031).
    /// - Forwardable, protocol 12's and 14's conversation calls
    ///   (`session.search`, `session.park`, `session.wake`, `session.fork`,
    ///   `session.rewind`) and protocol 17's `agent.history` (ADR 0053),
    ///   which an import there needs as `session.create` does; not
    ///   `node.settings` or `node.set_settings`, a node's administration.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per method, deny by default, so a new method does not compile until \
                  someone decides; the list is the point"
    )]
    pub const fn forwardable(self) -> bool {
        match self {
            Self::NodeInfo
            | Self::ListHarnesses
            | Self::ListSessions
            | Self::GetSession
            | Self::OpenSideQuestion
            | Self::PromptSideQuestion
            | Self::ResetSideQuestion
            | Self::CreateSession
            | Self::SendInput
            | Self::KillSession
            | Self::ReadEvents
            | Self::MoveProfile
            | Self::ListProfiles
            | Self::ExportSession
            | Self::ImportSession
            | Self::ExportMemory
            | Self::ImportMemory
            | Self::ReadTransfer
            | Self::CloseTransfer
            | Self::SetSecret
            | Self::LspWorkspaceSymbols
            | Self::LspDocumentSymbols
            | Self::LspDefinition
            | Self::LspReferences
            | Self::LspHover
            | Self::LspImplementations
            | Self::LspTypeDefinition
            | Self::LspIncomingCalls
            | Self::LspOutgoingCalls
            | Self::LspDiagnostics
            | Self::ListSecrets
            | Self::ListModels
            | Self::HarnessOptions
            | Self::SessionOptions
            | Self::SetSessionOption
            | Self::RenameSession
            | Self::PromptSession
            | Self::ChangeQueue
            | Self::GetConversation
            | Self::ResumeSession
            | Self::MarkSession
            | Self::DeleteSession
            | Self::BeginUpload
            | Self::WriteUpload
            | Self::FinishUpload
            | Self::ReadAttachment
            | Self::ListProjects
            | Self::GetProject
            | Self::AddProject
            | Self::CloneProject
            | Self::InitProject
            | Self::RemoveProject
            | Self::ProjectCommands
            | Self::ApproveCommands
            | Self::ListWorktrees
            | Self::CreateWorktree
            | Self::SetupOutput
            | Self::RunSetup
            | Self::RemoveWorktree
            | Self::ListSnapshots
            | Self::DropSnapshot
            | Self::GitStatus
            | Self::GitChanges
            | Self::GitDiff
            | Self::GitShow
            | Self::GitBranches
            | Self::GitLog
            | Self::GitStashes
            | Self::GitStage
            | Self::GitUnstage
            | Self::GitDiscard
            | Self::GitCommit
            | Self::GitFetch
            | Self::GitPush
            | Self::GitPull
            | Self::GitSwitch
            | Self::GitRenameBranch
            | Self::GitStash
            | Self::ListDir
            | Self::ReadFile
            | Self::WriteFile
            | Self::CreateDir
            | Self::RenamePath
            | Self::DeletePath
            | Self::FindFiles
            | Self::SearchFiles
            // Protocol 11: a session's pull request, on the machine the
            // session runs on.
            | Self::ForgeStatus
            | Self::ListPullRequests
            | Self::GetPullRequest
            | Self::OpenPullRequest
            | Self::MergePullRequest
            | Self::SetFeedback
            // Protocol 11: terminals, scripts and ports, on the machine
            // they run on (a terminal's stream is not forwarded: it is
            // read with `terminal.read` from another node).
            | Self::CreateTerminal
            | Self::ListTerminals
            | Self::GetTerminal
            | Self::ReadTerminal
            | Self::SendTerminal
            | Self::RenameTerminal
            | Self::KillTerminal
            | Self::CloseTerminal
            | Self::ListScripts
            | Self::RunScript
            | Self::RestartScript
            | Self::ListPorts
            // Protocol 11: what a node can run is read like its harnesses.
            | Self::ListAgents
            | Self::GetAgentStatus
            // Schedules (protocol 11): forwardable; who may do what from
            // another node is the target's scheduler's rule (ADR 0033).
            | Self::GetDeviceSelf
            | Self::GetFleetStatus
            | Self::PreviewSchedule
            | Self::ListSchedules
            | Self::GetSchedule
            | Self::CreateSchedule
            | Self::UpdateSchedule
            | Self::PauseSchedule
            | Self::ResumeSchedule
            | Self::RunSchedule
            | Self::DeleteSchedule
            | Self::ListRuns
            // Saved prompts: readable from another node; written only on
            // their own (the service refuses a delegated writer).
            // Lookup policy carries only a choice; credentials stay at the
            // destination, whose admin/operate scope intersection is checked.
            | Self::GetUsageLimits
            | Self::SetUsageLookup
            | Self::QueryUsage
            | Self::ListPrompts
            | Self::CreatePrompt
            | Self::UpdatePrompt
            | Self::DeletePrompt
            | Self::ListInbox
            | Self::GetInboxEntry
            | Self::MarkInbox
            // Protocol 12: a search reads, as `session.read` does.
            | Self::SearchMessages
            // Parking acts on a session as a prompt does.
            | Self::ParkSession
            | Self::WakeSession
            // Forking and rewinding act on a session as a prompt does.
            | Self::ForkSession
            | Self::RewindSession
            // An agent's history there, to import from it there (ADR
            // 0053): it needs `operate`, as the import does.
            | Self::GetAgentHistory => true,
            // Protocol 11: adding and removing an agent is the node's own
            // administration, by its owner or its own devices (ADR 0032).
            Self::GetPushKey
            | Self::SubscribePush
            | Self::UnsubscribePush
            | Self::ReportPushPresence
            | Self::AddAgent
            | Self::GetCatalogue
            | Self::InstallAgent
            | Self::ReadInstall
            | Self::RemoveAgent
            | Self::StopNode
            // The node's settings (protocol 12): its own administration,
            // read and changed by its owner and its own devices.
            | Self::GetNodeSettings
            | Self::SetNodeSettings
            | Self::InviteDevice
            | Self::ListDevices
            | Self::RevokeDevice
            | Self::ReloadProfiles
            | Self::DeleteSecret
            | Self::CopySecret
            | Self::Forward
            | Self::Relay
            | Self::Deliver
            | Self::GrantHome
            | Self::GetCallback
            | Self::ForgetNode
            | Self::ListPeers
            | Self::ListNodes
            | Self::GetManifest
            | Self::ReadBlob
            | Self::WatchFleet
            | Self::ReportApplied
            | Self::ListMembers
            | Self::JoinFleet
            | Self::LeaveFleet
            | Self::SyncFleet
            // Each machine is read over its own connection (ADR 0056).
            | Self::GetPublished
            | Self::Subscribe
            | Self::SubscribeTerminal
            | Self::Unsubscribe => false,
        }
    }
}
