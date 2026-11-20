//! The one method table (`method_table!`): every RPC method's row, in two
//! files. This one holds the earlier rows, through the workspace's file
//! methods (`file.grep`); `table/rest.rs` holds the rest, from the forge's
//! (ADR 0029) on, and new methods, pending ones included, are added at its
//! end. The rest of the protocol (requests, frames, the generated code) is
//! declared in the parent module, which invokes this table; see its
//! Decision Logs.
//!
//! Kept in its own file so that rows added by many packages do not push
//! the parent past its size budget (Quality B follow-up, milestone 12).
//!
//! [Decision Log: two files] (TRACKER 815)
//! - Purpose: the rows passed the 500-line file budget (docs/QUALITY.md)
//!   when ADR 0089's five methods were added after the plugin manager's.
//! - Constraint: one callback must receive every row in one invocation, in
//!   the order the rows are written: `Method::ALL`, the generated enums and
//!   the pinned lists in `rpc/tests.rs` follow it.
//! - Alternatives: listing the file in `scripts/rust-size-allow.txt` (the
//!   list takes no new files); moving the row documentation out (it is what
//!   the generated docs show, and the table would pass the budget again a
//!   few methods later); one table per feature, combined by a third macro
//!   (every feature's rows would need their own file and order rule).
//! - Choice: `method_table!` hands its rows, as tokens, to the hidden
//!   `method_table_rest!`, which appends the later rows and calls the
//!   callback once with all of them. Nothing outside this module changes:
//!   callers still write `vessel_proto::method_table!(callback)`.
//! - Must stay true: rows are only appended, at the end of `table/rest.rs`;
//!   when that file reaches the budget, a third file joins the chain the
//!   same way.

mod rest;

/// The one method table: one row per RPC method (the Decision Log above).
///
/// A row is `Variant / handler / "wire.name" [Scope, protocol, shape]:
/// Request => Response;`.
/// - `Scope` is what a paired device must hold (`Authenticated`: no extra
///   scope).
/// - `protocol` is the first protocol version with the method: a number,
///   `{expression}` for a named version constant, or `(pending FEATURE)`
///   for a method whose number the integrator has not assigned yet, where
///   `FEATURE` is the feature's `Option<ProtocolVersion>` constant (None
///   until assigned; both ends refuse the method while it is None).
///   `(staged FEATURE)` is a pending method whose values landed before
///   any runtime serves it: its `Handler` method answers `Unsupported` by
///   default and the node's guard needs no service method for it. The
///   package that serves it changes the row to `(pending FEATURE)` in the
///   commit that adds its handler; a staged row's constant must stay None.
/// - `shape` is how the method is answered: `unary` (one reply),
///   `stream` (the reply opens a stream of frames) or `envelope` (it
///   carries another call to a node and answers once; what it carries is
///   the call that succeeds or fails).
///
/// The table is a macro that hands its rows to `$callback`, so that code
/// which must repeat something for every method (this crate's generated
/// enums and `Handler`, the node's guard and envelope, the client's unary
/// marker) is generated from these rows instead of kept as a second list.
/// Types are written from the crate root (`$crate::`) for that reason.
#[macro_export]
macro_rules! method_table {
    ($callback:ident) => {
        $crate::method_table_rest! {
            $callback;
            /// Describe the node.
            NodeInfo / node_info / "node.info" [Read, 2, unary]: $crate::GetNode => $crate::NodeInfo;
            /// List installed harnesses.
            ListHarnesses / list_harnesses / "harness.list" [Read, 2, unary]: $crate::ListHarnesses => Vec<$crate::Harness>;
            /// List sessions.
            ListSessions / list_sessions / "session.list" [Read, 2, unary]: $crate::ListSessions => Vec<$crate::SessionInfo>;
            /// Get one session.
            GetSession / get_session / "session.get" [Read, 2, unary]: $crate::GetSession => $crate::SessionInfo;
            /// Start a session.
            CreateSession / create_session / "session.create" [Operate, 2, unary]: $crate::CreateSession => $crate::SessionInfo;
            /// Open or reuse a separate side conversation (protocol 19).
            OpenSideQuestion / open_side_question / "session.side_open" [Operate, 19, unary]: $crate::side_questions::OpenSideQuestion => $crate::SessionInfo;
            /// Ask a side question with refreshed parent context (protocol 19).
            PromptSideQuestion / prompt_side_question / "session.side_prompt" [Operate, 19, unary]: $crate::side_questions::PromptSideQuestion => $crate::Delivered;
            /// Replace the expected side conversation (protocol 19).
            ResetSideQuestion / reset_side_question / "session.side_reset" [Operate, 19, unary]: $crate::side_questions::ResetSideQuestion => $crate::SessionInfo;
            /// Send input to a session.
            SendInput / send_input / "session.send" [Operate, 2, unary]: $crate::SendInput => $crate::Ack;
            /// Kill a session.
            KillSession / kill_session / "session.kill" [Operate, 2, unary]: $crate::KillSession => $crate::Ack;
            /// Read a page of a session's log.
            ReadEvents / read_events / "session.read" [Read, 2, unary]: $crate::ReadEvents => $crate::EventPage;
            /// Subscribe to a session's events.
            Subscribe / subscribe / "session.subscribe" [Read, 2, stream]: $crate::Subscribe => $crate::Subscribed;
            /// End a subscription.
            Unsubscribe / unsubscribe / "session.unsubscribe" [Read, 2, unary]: $crate::Unsubscribe => $crate::Ack;
            /// Stop the node (protocol 3).
            StopNode / stop_node / "node.stop" [Admin, 3, unary]: $crate::StopNode => $crate::Ack;
            /// Make a pairing invitation (protocol 4).
            InviteDevice / invite_device / "device.invite" [Admin, 4, unary]: $crate::InviteDevice => $crate::Invitation;
            /// Inspect this authenticated connection's own effective grant (protocol 20).
            GetDeviceSelf / device_self / "device.self" [Authenticated, 20, unary]: $crate::GetDeviceSelf => $crate::DeviceSelf;
            /// List paired devices (protocol 4).
            ListDevices / list_devices / "device.list" [Admin, 4, unary]: $crate::ListDevices => Vec<$crate::DeviceInfo>;
            /// Revoke a device and close its connections (protocol 4).
            RevokeDevice / revoke_device / "device.revoke" [Admin, 4, unary]: $crate::RevokeDevice => $crate::Ack;
            /// List profiles (protocol 5).
            ListProfiles / list_profiles / "profile.list" [Read, 5, unary]: $crate::ListProfiles => Vec<$crate::ProfileInfo>;
            /// Read `profiles.toml` again (protocol 5).
            ReloadProfiles / reload_profiles / "profile.reload" [Admin, 5, unary]: $crate::ReloadProfiles => Vec<$crate::ProfileInfo>;
            /// Move a structured session to another profile (protocol 5).
            MoveProfile / move_profile / "session.move_profile" [Operate, 5, unary]: $crate::MoveProfile => $crate::SessionInfo;
            /// The fleet's current bundle (protocol 5).
            GetManifest / get_manifest / "fleet.manifest" [Sync, 5, unary]: $crate::GetManifest => $crate::Manifest;
            /// Part of a bundle blob (protocol 5).
            ReadBlob / read_blob / "fleet.blob" [Sync, 5, unary]: $crate::ReadBlob => $crate::BlobChunk;
            /// Wait for a newer bundle generation (protocol 5).
            WatchFleet / watch_fleet / "fleet.watch" [Sync, 5, unary]: $crate::WatchFleet => $crate::Generation;
            /// Report an applied generation (protocol 5).
            ReportApplied / report_applied / "fleet.report" [Sync, 5, unary]: $crate::ReportApplied => $crate::Ack;
            /// Join this machine to an invited fleet home (protocol 15).
            JoinFleet / join_fleet / "fleet.join" [Admin, 15, unary]: $crate::JoinFleet => $crate::JoinedFleet;
            /// Leave this machine's fleet, keeping materialized files (protocol 15).
            LeaveFleet / leave_fleet / "fleet.leave" [Admin, 15, unary]: $crate::LeaveFleet => $crate::Ack;
            /// Accept a manual sync on this machine (protocol 15).
            SyncFleet / sync_fleet / "fleet.sync" [Admin, 15, unary]: $crate::SyncFleet => $crate::Ack;
            /// Read safe local apply history and permitted member reports (protocol 15).
            GetFleetStatus / fleet_status / "fleet.status" [Read, 15, unary]: $crate::GetFleetStatus => $crate::FleetSyncStatus;
            /// What this machine shares through its fleet, by name (protocol 16, ADR 0056).
            GetPublished / fleet_published / "fleet.published" [Admin, 16, unary]: $crate::GetPublished => $crate::FleetPublished;
            /// List fleet members (protocol 5).
            ListMembers / list_members / "fleet.members" [Admin, 5, unary]: $crate::ListMembers => Vec<$crate::FleetMember>;
            /// Run a call on another node of the fleet (protocol 6).
            Forward / forward / "fleet.forward" [Read, 6, envelope]: $crate::Forward => $crate::Forwarded;
            /// A member asks its home to pass a call on (protocol 6).
            Relay / relay / "fleet.relay" [Relay, 6, envelope]: $crate::RelayCall => $crate::Forwarded;
            /// The home has a member run a call (protocol 6).
            Deliver / deliver / "fleet.deliver" [Read, 6, envelope]: $crate::Deliver => $crate::Forwarded;
            /// A member grants its home a credential on itself (protocol 6).
            GrantHome / grant_home / "fleet.grant" [Sync, 6, unary]: $crate::GrantHome => $crate::Ack;
            /// The home's relays, for a member to be called back through (protocol 6).
            GetCallback / get_callback / "fleet.callback" [Sync, 6, unary]: $crate::GetCallback => $crate::Callback;
            /// The home's view of the fleet, for members (protocol 6).
            ListPeers / list_peers / "fleet.peers" [Sync, 6, unary]: $crate::ListPeers => Vec<$crate::FleetNode>;
            /// The fleet as this node knows it (protocol 6).
            ListNodes / list_nodes / "fleet.nodes" [Read, 6, unary]: $crate::ListNodes => Vec<$crate::FleetNode>;
            /// The home forgets a member node's binding (protocol 6).
            ForgetNode / forget_node / "fleet.forget" [Admin, 6, unary]: $crate::ForgetNode => $crate::NodeForgotten;
            /// Snapshot a session's harness files (protocol 6).
            ExportSession / export_session / "session.export" [Operate, 6, unary]: $crate::ExportSession => $crate::TransferManifest;
            /// Copy a session here from a node of the fleet (protocol 6).
            ImportSession / import_session / "session.import" [Operate, 6, unary]: $crate::ImportSession => $crate::SessionImported;
            /// Snapshot a profile's memory (protocol 6).
            ExportMemory / export_memory / "memory.export" [Admin, 6, unary]: $crate::ExportMemory => $crate::TransferManifest;
            /// Copy memory here from a node of the fleet (protocol 6).
            ImportMemory / import_memory / "memory.import" [Admin, 6, unary]: $crate::ImportMemory => $crate::MemoryImported;
            /// Part of a file of an open transfer (protocol 6).
            ReadTransfer / read_transfer / "transfer.read" [Read, 6, unary]: $crate::ReadTransfer => $crate::BlobChunk;
            /// Drop an open transfer (protocol 6).
            CloseTransfer / close_transfer / "transfer.close" [Read, 6, unary]: $crate::CloseTransfer => $crate::Ack;
            /// Store a secret (protocol 6).
            SetSecret / set_secret / "secret.set" [Admin, 6, unary]: $crate::SetSecret => $crate::Ack;
            /// List secrets by name (protocol 6).
            ListSecrets / list_secrets / "secret.list" [Admin, 6, unary]: $crate::ListSecrets => Vec<$crate::SecretName>;
            /// Remove a secret (protocol 6).
            DeleteSecret / delete_secret / "secret.delete" [Admin, 6, unary]: $crate::DeleteSecret => $crate::Ack;
            /// Copy a secret to a node of the fleet (protocol 6).
            CopySecret / copy_secret / "secret.copy" [Admin, 6, unary]: $crate::CopySecret => $crate::Ack;
            /// Saved-file workspace symbols (protocol 7).
            LspWorkspaceSymbols / lsp_workspace_symbols / "lsp.workspace_symbols" [Read, 7, unary]: $crate::lsp::WorkspaceSymbols => $crate::lsp::Page<$crate::lsp::Symbol>;
            /// Saved-file document symbols (protocol 7).
            LspDocumentSymbols / lsp_document_symbols / "lsp.document_symbols" [Read, 7, unary]: $crate::lsp::DocumentSymbols => $crate::lsp::Page<$crate::lsp::Symbol>;
            /// Saved-file definition (protocol 7).
            LspDefinition / lsp_definition / "lsp.definition" [Read, 7, unary]: $crate::lsp::Definition => $crate::lsp::Page<$crate::lsp::Location>;
            /// Saved-file references (protocol 7).
            LspReferences / lsp_references / "lsp.references" [Read, 7, unary]: $crate::lsp::References => $crate::lsp::Page<$crate::lsp::Location>;
            /// Saved-file hover (protocol 7).
            LspHover / lsp_hover / "lsp.hover" [Read, 7, unary]: $crate::lsp::GetHover => $crate::lsp::Hover;
            /// Saved-file implementations (protocol 7).
            LspImplementations / lsp_implementations / "lsp.implementations" [Read, 7, unary]: $crate::lsp::Implementations => $crate::lsp::Page<$crate::lsp::Location>;
            /// Saved-file type definition (protocol 7).
            LspTypeDefinition / lsp_type_definition / "lsp.type_definition" [Read, 7, unary]: $crate::lsp::TypeDefinition => $crate::lsp::Page<$crate::lsp::Location>;
            /// Saved-file incoming calls (protocol 7).
            LspIncomingCalls / lsp_incoming_calls / "lsp.incoming_calls" [Read, 7, unary]: $crate::lsp::IncomingCalls => $crate::lsp::Page<$crate::lsp::Call>;
            /// Saved-file outgoing calls (protocol 7).
            LspOutgoingCalls / lsp_outgoing_calls / "lsp.outgoing_calls" [Read, 7, unary]: $crate::lsp::OutgoingCalls => $crate::lsp::Page<$crate::lsp::Call>;
            /// Saved-file diagnostics (protocol 7).
            LspDiagnostics / lsp_diagnostics / "lsp.diagnostics" [Read, 7, unary]: $crate::lsp::Diagnostics => $crate::lsp::Page<$crate::lsp::Diagnostic>;
            /// The model routes this node offers (protocol 7).
            ListModels / list_models / "model.list" [Read, 7, unary]: $crate::ListModels => Vec<$crate::ModelRoute>;
            /// What a harness's agent offers before a session exists (protocol 9).
            HarnessOptions / harness_options / "harness.options" [Read, 9, unary]: $crate::GetHarnessOptions => $crate::HarnessOptions;
            /// A structured session's config options (protocol 9).
            SessionOptions / session_options / "session.options" [Read, 9, unary]: $crate::GetSessionOptions => Vec<$crate::SessionOption>;
            /// Change one of a running session's config options (protocol 9).
            SetSessionOption / set_session_option / "session.set_option" [Operate, 9, unary]: $crate::SetSessionOption => Vec<$crate::SessionOption>;
            /// Give a session a title (protocol 9).
            RenameSession / rename_session / "session.rename" [Operate, 9, unary]: $crate::RenameSession => $crate::SessionInfo;
            /// Send a prompt, delivered as asked (protocol 10).
            PromptSession / prompt_session / "session.prompt" [Operate, 10, unary]: $crate::conversation::PromptSession => $crate::Delivered;
            /// Change a session's queue (protocol 10).
            ChangeQueue / change_queue / "session.queue" [Operate, 10, unary]: $crate::conversation::ChangeQueue => $crate::conversation::QueueChanged;
            /// A session's conversation as it stands (protocol 10).
            GetConversation / get_conversation / "session.conversation" [Read, 10, unary]: $crate::conversation::GetConversation => $crate::Conversation;
            /// Start an ended session again with its conversation (protocol 10).
            ResumeSession / resume_session / "session.resume" [Operate, 10, unary]: $crate::conversation::ResumeSession => $crate::SessionInfo;
            /// Pin, archive or mark a session read (protocol 10).
            MarkSession / mark_session / "session.mark" [Operate, 10, unary]: $crate::conversation::MarkSession => $crate::SessionInfo;
            /// Remove an ended session and everything kept with it (protocol 10).
            DeleteSession / delete_session / "session.delete" [Operate, 10, unary]: $crate::conversation::DeleteSession => $crate::conversation::SessionDeleted;
            /// Declare a file to upload to a session (protocol 10).
            BeginUpload / begin_upload / "attachment.begin" [Operate, 10, unary]: $crate::conversation::BeginUpload => $crate::conversation::Upload;
            /// The next piece of an upload (protocol 10).
            WriteUpload / write_upload / "attachment.write" [Operate, 10, unary]: $crate::conversation::WriteUpload => $crate::conversation::Written;
            /// Check an upload and keep it with its session (protocol 10).
            FinishUpload / finish_upload / "attachment.finish" [Operate, 10, unary]: $crate::conversation::FinishUpload => $crate::AttachmentRef;
            /// A piece of a session's attachment (protocol 10).
            ReadAttachment / read_attachment / "attachment.read" [Read, 10, unary]: $crate::conversation::ReadAttachment => $crate::BlobChunk;
            /// The projects, and the git the node found (protocol 10).
            ListProjects / list_projects / "project.list" [Read, 10, unary]: $crate::workspace::ListProjects => $crate::workspace::ProjectList;
            /// One project, or its next change (protocol 10).
            GetProject / get_project / "project.get" [Read, 10, unary]: $crate::workspace::GetProject => $crate::workspace::ProjectInfo;
            /// Add a folder as a project (protocol 10).
            AddProject / add_project / "project.add" [Operate, 10, unary]: $crate::workspace::AddProject => $crate::workspace::ProjectInfo;
            /// Clone a repository as a project (protocol 10).
            CloneProject / clone_project / "project.clone" [Operate, 10, unary]: $crate::workspace::CloneProject => $crate::workspace::ProjectInfo;
            /// Start a new repository as a project (protocol 10).
            InitProject / init_project / "project.init" [Operate, 10, unary]: $crate::workspace::InitProject => $crate::workspace::ProjectInfo;
            /// Forget a project (protocol 10).
            RemoveProject / remove_project / "project.remove" [Operate, 10, unary]: $crate::workspace::RemoveProject => $crate::Ack;
            /// What a project's file would run (protocol 10).
            ProjectCommands / project_commands / "project.commands" [Read, 10, unary]: $crate::workspace::GetProjectCommands => $crate::workspace::ProjectCommands;
            /// Approve a project's commands (protocol 10).
            ApproveCommands / approve_commands / "project.approve" [Operate, 10, unary]: $crate::workspace::ApproveCommands => $crate::workspace::ProjectCommands;
            /// The worktrees (protocol 10).
            ListWorktrees / list_worktrees / "worktree.list" [Read, 10, unary]: $crate::workspace::ListWorktrees => $crate::workspace::WorktreeList;
            /// Make a worktree (protocol 10).
            CreateWorktree / create_worktree / "worktree.create" [Operate, 10, unary]: $crate::workspace::CreateWorktree => $crate::workspace::CreateWorktreeOutcome;
            /// A page of a worktree's setup output (protocol 10).
            SetupOutput / setup_output / "worktree.output" [Read, 10, unary]: $crate::workspace::ReadSetupOutput => $crate::workspace::SetupOutput;
            /// Run a worktree's setup again (protocol 10).
            RunSetup / run_setup / "worktree.setup" [Operate, 10, unary]: $crate::workspace::RunSetup => $crate::workspace::WorktreeInfo;
            /// Remove a worktree (protocol 10).
            RemoveWorktree / remove_worktree / "worktree.remove" [Operate, 10, unary]: $crate::workspace::RemoveWorktree => $crate::workspace::RemoveWorktreeOutcome;
            /// A project's worktree snapshots (protocol 10).
            ListSnapshots / list_snapshots / "worktree.snapshots" [Read, 10, unary]: $crate::workspace::ListSnapshots => $crate::workspace::SnapshotList;
            /// Delete a worktree snapshot (protocol 10).
            DropSnapshot / drop_snapshot / "worktree.drop_snapshot" [Operate, 10, unary]: $crate::workspace::DropSnapshot => $crate::Ack;
            /// A root's git status (protocol 10).
            GitStatus / git_status / "git.status" [Read, 10, unary]: $crate::workspace::GetStatus => $crate::workspace::GitStatus;
            /// The changed files of a diff scope (protocol 10).
            GitChanges / git_changes / "git.changes" [Read, 10, unary]: $crate::workspace::GetChanges => $crate::workspace::DiffSummary;
            /// One file's diff (protocol 10).
            GitDiff / git_diff / "git.diff" [Read, 10, unary]: $crate::workspace::GetFileDiff => $crate::workspace::FileDiff;
            /// A page of a file at a commit (protocol 10).
            GitShow / git_show / "git.show" [Read, 10, unary]: $crate::workspace::ShowFile => $crate::workspace::FileRead;
            /// A root's branches (protocol 10).
            GitBranches / git_branches / "git.branches" [Read, 10, unary]: $crate::workspace::ListBranches => $crate::workspace::BranchList;
            /// A page of history (protocol 10).
            GitLog / git_log / "git.log" [Read, 10, unary]: $crate::workspace::ReadLog => $crate::workspace::CommitPage;
            /// The stash (protocol 10).
            GitStashes / git_stashes / "git.stashes" [Read, 10, unary]: $crate::workspace::ListStashes => $crate::workspace::StashList;
            /// Stage files (protocol 10).
            GitStage / git_stage / "git.stage" [Operate, 10, unary]: $crate::workspace::Stage => $crate::workspace::GitStatus;
            /// Unstage files (protocol 10).
            GitUnstage / git_unstage / "git.unstage" [Operate, 10, unary]: $crate::workspace::Unstage => $crate::workspace::GitStatus;
            /// Drop local changes (protocol 10).
            GitDiscard / git_discard / "git.discard" [Operate, 10, unary]: $crate::workspace::Discard => $crate::workspace::GitStatus;
            /// Commit (protocol 10).
            GitCommit / git_commit / "git.commit" [Operate, 10, unary]: $crate::workspace::Commit => $crate::workspace::CommitOutcome;
            /// Fetch from a remote (protocol 10).
            GitFetch / git_fetch / "git.fetch" [Operate, 10, unary]: $crate::workspace::Fetch => $crate::workspace::FetchOutcome;
            /// Push the current branch (protocol 10).
            GitPush / git_push / "git.push" [Operate, 10, unary]: $crate::workspace::Push => $crate::workspace::PushOutcome;
            /// Fast-forward from the upstream (protocol 10).
            GitPull / git_pull / "git.pull" [Operate, 10, unary]: $crate::workspace::Pull => $crate::workspace::PullOutcome;
            /// Switch to a branch (protocol 10).
            GitSwitch / git_switch / "git.switch" [Operate, 10, unary]: $crate::workspace::Switch => $crate::workspace::SwitchOutcome;
            /// Rename a branch (protocol 10).
            GitRenameBranch / git_rename_branch / "git.rename_branch" [Operate, 10, unary]: $crate::workspace::RenameBranch => $crate::workspace::RenameOutcome;
            /// Save, apply or drop a stash entry (protocol 10).
            GitStash / git_stash / "git.stash" [Operate, 10, unary]: $crate::workspace::StashChanges => $crate::workspace::StashOutcome;
            /// A folder's entries (protocol 10).
            ListDir / list_dir / "file.list" [Read, 10, unary]: $crate::workspace::ListDir => $crate::workspace::DirListing;
            /// A page of a file (protocol 10).
            ReadFile / read_file / "file.read" [Read, 10, unary]: $crate::workspace::ReadFile => $crate::workspace::FileRead;
            /// Write a text file (protocol 10).
            WriteFile / write_file / "file.write" [Operate, 10, unary]: $crate::workspace::WriteFile => $crate::workspace::FileVersion;
            /// Create a folder (protocol 10).
            CreateDir / create_dir / "file.create_dir" [Operate, 10, unary]: $crate::workspace::CreateDir => $crate::Ack;
            /// Move a file or folder (protocol 10).
            RenamePath / rename_path / "file.rename" [Operate, 10, unary]: $crate::workspace::RenamePath => $crate::Ack;
            /// Delete a file or folder (protocol 10).
            DeletePath / delete_path / "file.delete" [Operate, 10, unary]: $crate::workspace::DeletePath => $crate::Ack;
            /// Find files by name (protocol 10).
            FindFiles / find_files / "file.find" [Read, 10, unary]: $crate::workspace::FindFiles => $crate::workspace::FoundFiles;
            /// Search inside files (protocol 10).
            SearchFiles / search_files / "file.grep" [Read, 10, unary]: $crate::workspace::SearchFiles => $crate::workspace::GrepResult;
        }
    };
}
