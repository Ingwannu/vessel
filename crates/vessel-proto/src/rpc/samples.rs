//! Typed request fixtures shared by admission tests. The exhaustive method
//! match makes every new registered method supply a decodable sample.

use super::{Call, Method};
use serde_json::json;

/// Decode fictional wire inputs here so tests elsewhere carry only typed calls.
#[expect(
    clippy::too_many_lines,
    reason = "one sample per method makes missing fixtures a compile error"
)]
pub(crate) fn call(method: Method) -> Result<Call, serde_json::Error> {
    let id = crate::SessionId::random();
    let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let root = json!({"kind":"project","project":id});
    let place = root.clone();
    let workspace = json!({"root":"/fixture","language":"rust"});
    let cadence = json!({"kind":"cron","expr":"* * * * *","zone":"UTC"});
    let definition =
        json!({"cadence":cadence,"target":{"kind":"session","session":id},"prompt":"sample"});
    let params = match method {
        Method::NodeInfo
        | Method::ListHarnesses
        | Method::ListSessions
        | Method::ListDevices
        | Method::ReloadProfiles
        | Method::GetManifest
        | Method::ListMembers
        | Method::ListPeers
        | Method::ListNodes
        | Method::ListSecrets
        | Method::ListModels
        | Method::GetPushKey
        | Method::UnsubscribePush => json!(null),
        Method::GetSession
        | Method::OpenSideQuestion
        | Method::KillSession
        | Method::MoveProfile
        | Method::ExportSession
        | Method::SessionOptions
        | Method::GetConversation
        | Method::ResumeSession
        | Method::DetachSession
        | Method::DeleteSession
        | Method::GetPullRequest
        | Method::ParkSession
        | Method::WakeSession
        | Method::ForkSession => json!({"session": id}),
        Method::CreateSession => json!({"spec":{"harness":"codex","mode":"structured"}}),
        Method::PromptSideQuestion => json!({"session":id,"question":"sample"}),
        Method::ResetSideQuestion => json!({"session":id,"side":id}),
        Method::SendInput => json!({"session":id,"input":{"type":"cancel_turn"}}),
        Method::ReadEvents => json!({"session":id,"from":0,"limit":1}),
        Method::Subscribe => json!({"session":id,"from":0}),
        Method::Unsubscribe => json!({"subscription":crate::SubscriptionId::new(1)}),
        Method::StopNode => json!({"when":"now"}),
        Method::InviteDevice => json!({"ttl_secs":1}),
        Method::GetDeviceSelf
        | Method::ListProfiles
        | Method::LeaveFleet
        | Method::SyncFleet
        | Method::GetFleetStatus
        | Method::GetPublished
        | Method::ExportMemory
        | Method::GetRegistry
        | Method::ListWorkspaces
        | Method::ListProjects
        | Method::ListWorktrees
        | Method::ForgeStatus
        | Method::ListPullRequests
        | Method::ListTerminals
        | Method::ListPorts
        | Method::ListAgents
        | Method::GetAgentStatus
        | Method::GetCatalogue
        | Method::ListSchedules
        | Method::ListPrompts
        | Method::GetNodeSettings
        | Method::SetNodeSettings
        | Method::ListInbox => json!({}),
        Method::RevokeDevice => json!({"device":id}),
        Method::ReadBlob => json!({"hash":hash}),
        Method::WatchFleet => json!({"after":0}),
        Method::ReportApplied => json!({"generation":0}),
        Method::JoinFleet => json!({"link":pairing_link(),"grant":[]}),
        Method::Forward => json!({"node":id,"call":{"method":"node.info","params":null}}),
        Method::Relay => {
            json!({"node":id,"call":{"method":"node.info","params":null},"origin":origin()})
        }
        Method::Deliver => json!({"call":{"method":"node.info","params":null},"origin":origin()}),
        Method::GrantHome => json!({"link":pairing_link()}),
        Method::GetCallback => {
            json!({"node":id,"key":crate::NodePublicKey::from_bytes(Default::default())})
        }
        Method::ForgetNode => json!({"node":id}),
        Method::ImportSession => json!({"from":{"node":id,"session":id}}),
        Method::ImportMemory => json!({"from":{"node":id}}),
        Method::ReadTransfer => json!({"transfer":id,"hash":hash}),
        Method::CloseTransfer => json!({"transfer":id}),
        Method::SetSecret => json!({"name":"sample","value":"fictional fixture"}),
        Method::DeleteSecret | Method::RemoveAgent => json!({"name":"sample"}),
        Method::CopySecret => json!({"name":"sample","to":id}),
        Method::LspWorkspaceSymbols => json!({"workspace":workspace,"query":"sample"}),
        Method::LspDocumentSymbols | Method::LspDiagnostics => {
            json!({"workspace":workspace,"file":"sample.rs"})
        }
        Method::LspDefinition
        | Method::LspReferences
        | Method::LspHover
        | Method::LspImplementations
        | Method::LspTypeDefinition
        | Method::LspIncomingCalls
        | Method::LspOutgoingCalls => {
            json!({"workspace":workspace,"file":"sample.rs","position":{"line":1,"column":1}})
        }
        Method::HarnessOptions | Method::GetAgentHistory => json!({"harness":"codex"}),
        Method::SetSessionOption => {
            json!({"session":id,"choice":{"id":"sample","kind":"toggle","on":true}})
        }
        Method::RenameSession | Method::OpenPullRequest => json!({"session":id,"title":"sample"}),
        Method::PromptSession => json!({"session":id,"content":[],"delivery":"start"}),
        Method::ChangeQueue => json!({"session":id,"change":"hold"}),
        Method::MarkSession => json!({"session":id,"mark":"pin"}),
        Method::BeginUpload => {
            json!({"session":id,"name":"sample.txt","mime":"text/plain","size":1})
        }
        Method::WriteUpload => json!({"upload":id,"offset":0,"data":"eA=="}),
        Method::FinishUpload => json!({"upload":id}),
        Method::ReadAttachment => json!({"session":id,"attachment":id,"offset":0}),
        Method::GetWorkspace | Method::RenameWorkspace => json!({"workspace":id}),
        Method::ReleaseWorkspaceAutomation => {
            json!({"workspace":id,"expected":0,"source":{"kind":"change_request","number":1,"head_repository":{"kind":"github","host":"github.com","path":"sample/project"}}})
        }
        Method::PinWorkspace => json!({"workspace":id,"pin":"unpinned"}),
        Method::GetProject
        | Method::RemoveProject
        | Method::ProjectCommands
        | Method::ListSnapshots => json!({"project":id}),
        Method::AddProject | Method::InitProject => json!({"path":"/fixture"}),
        Method::CloneProject => {
            json!({"url":"https://example.invalid/sample.git","parent":"/fixture"})
        }
        Method::ApproveCommands => json!({"project":id,"digest":hash,"approved":true}),
        Method::CreateWorktree => json!({"project":id,"branch":{"kind":"new"}}),
        Method::SetupOutput | Method::RunSetup | Method::RemoveWorktree => json!({"worktree":id}),
        Method::DropSnapshot => json!({"project":id,"worktree":id}),
        Method::GitStatus
        | Method::GitBranches
        | Method::GitLog
        | Method::GitStashes
        | Method::GitFetch
        | Method::GitPush
        | Method::GitPull
        | Method::ListDir => json!({"root":root}),
        Method::GitCompare => {
            json!({"root":root,"comparison":{"kind":"committed"},"whitespace":"show"})
        }
        Method::GitCompareFile => {
            json!({"query":{"root":root,"comparison":{"kind":"all"}},"path":"sample.rs"})
        }
        Method::GitChanges => json!({"root":root,"scope":{"kind":"unstaged"}}),
        Method::GitDiff => json!({"root":root,"scope":{"kind":"unstaged"},"path":"sample.rs"}),
        Method::GitShow => {
            json!({"root":root,"at":{"kind":"ref","name":"HEAD"},"path":"sample.rs"})
        }
        Method::GitStage | Method::GitUnstage | Method::GitDiscard => {
            json!({"root":root,"paths":{"kind":"all"}})
        }
        Method::GitCommit => json!({"root":root,"message":"sample"}),
        Method::GitSwitch => json!({"root":root,"branch":"sample"}),
        Method::GitRenameBranch => json!({"root":root,"to":"sample"}),
        Method::GitStash => json!({"root":root,"action":{"kind":"save"}}),
        Method::ReadFile | Method::CreateDir | Method::DeletePath => {
            json!({"root":root,"path":"sample.rs"})
        }
        Method::WriteFile => {
            json!({"root":root,"path":"sample.rs","text":"sample","expect":{"kind":"any"}})
        }
        Method::RenamePath => json!({"root":root,"from":"sample.rs","to":"renamed.rs"}),
        Method::FindFiles | Method::SearchFiles => json!({"root":root,"query":"sample"}),
        Method::MergePullRequest => {
            json!({"session":id,"number":1,"method":"merge","head":"0123456789abcdef0123456789abcdef01234567"})
        }
        Method::SetFeedback => json!({"session":id,"enabled":true}),
        Method::CreateTerminal | Method::ListScripts => json!({"place":place}),
        Method::GetTerminal
        | Method::KillTerminal
        | Method::CloseTerminal
        | Method::RestartScript => json!({"terminal":id}),
        Method::ReadTerminal => json!({"terminal":id,"from":0,"limit":1}),
        Method::SubscribeTerminal => json!({"terminal":id,"from":0}),
        Method::SendTerminal => {
            json!({"terminal":id,"input":crate::TerminalInput::Resize{size:crate::TerminalSize::default()}})
        }
        Method::RenameTerminal => json!({"terminal":id,"title":"sample"}),
        Method::RunScript => json!({"place":place,"script":"sample"}),
        Method::AddAgent => {
            json!({"name":"sample","agent":{"from":"registry","id":"sample","version":"1.0.0"}})
        }
        Method::InstallAgent => json!({"agent":"codex","part":"harness"}),
        Method::ReadInstall => json!({"job":id}),
        Method::QueryUsage => json!({"scope":{"kind":"machine"},"period":"day"}),
        Method::PreviewSchedule => json!({"cadence":cadence}),
        Method::GetSchedule
        | Method::PauseSchedule
        | Method::ResumeSchedule
        | Method::RunSchedule
        | Method::DeleteSchedule
        | Method::ListRuns => json!({"schedule":id}),
        Method::CreateSchedule => json!({"definition":definition}),
        Method::UpdateSchedule => json!({"schedule":id,"revision":1,"definition":definition}),
        Method::CreatePrompt => json!({"name":"sample","text":"sample"}),
        Method::UpdatePrompt => json!({"prompt":id,"revision":1,"name":"sample","text":"sample"}),
        Method::DeletePrompt => json!({"prompt":id}),
        Method::SearchMessages => json!({"query":"sample"}),
        Method::RewindSession => json!({"session":id,"after":0}),
        Method::SubscribePush => json!({"subscription":push_subscription()}),
        Method::ReportPushPresence => {
            json!({"focus":{"focus":"background"},"activity":"unchanged"})
        }
        Method::GetInboxEntry => json!({"key":{"session":id,"seq":1}}),
        Method::MarkInbox => json!({"key":{"session":id,"seq":1},"read":"read"}),
        Method::GetUsageLimits => {
            json!({"scope":{"kind":"session","session":id},"refresh":"cached"})
        }
        Method::SetUsageLookup => json!({"policy":"off"}),
    };
    serde_json::from_value(json!({"method":method,"params":params}))
}

pub(crate) fn origin() -> crate::Origin {
    crate::Origin {
        node: crate::NodeId::random(),
        by: crate::OriginBy::Owner,
        scopes: Vec::new(),
    }
}

fn push_subscription() -> serde_json::Value {
    // Fictional material of the right shape: an uncompressed-point marker
    // and fixed lengths, base64url without padding. Nothing real is used.
    let key =
        "BAcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";
    json!({"endpoint":"https://push.example/sample","p256dh":key,"auth":"CQkJCQkJCQkJCQkJCQkJCQ"})
}

fn pairing_link() -> crate::PairingLink {
    // Fictional material exists only in memory; no real link is read or logged.
    crate::PairingLink::encode(&crate::PairingOffer {
        v: crate::OfferVersion,
        node: crate::NodeId::random(),
        key: crate::NodePublicKey::from_bytes(Default::default()),
        relays: Vec::new(),
        direct: Vec::new(),
        invite: crate::InvitationSecret::from_bytes(Default::default()),
        expires: chrono::Utc::now(),
    })
    .unwrap()
}
