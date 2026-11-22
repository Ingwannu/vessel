//! The method table's later rows, from the forge's (ADR 0029) on
//! (`table.rs`'s Decision Log: two files). New methods, pending ones
//! included, are added here, at the end.

/// The rows after `file.grep`: `method_table!` passes its own rows, as
/// tokens, after the callback; these follow them, and the callback gets
/// every row in one invocation. Not called directly.
#[doc(hidden)]
#[macro_export]
macro_rules! method_table_rest {
    ($callback:ident; $($first:tt)*) => {
        $callback! {
            $($first)*
            // Protocol 11, track B2 (ADR 0029): pull requests on a forge.
            /// How the node is signed in to each forge.
            ForgeStatus / forge_status / "forge.status" [Read, 11, unary]: $crate::forge::GetForgeStatus => $crate::forge::ForgeStatus;
            /// Every watched session's pull request.
            ListPullRequests / list_pull_requests / "pr.list" [Read, 11, unary]: $crate::forge::ListPullRequests => $crate::forge::PullRequestList;
            /// A session's pull request, whole.
            GetPullRequest / get_pull_request / "pr.get" [Read, 11, unary]: $crate::forge::GetPullRequest => $crate::forge::PullRequestView;
            /// Open a pull request from a session's branch.
            OpenPullRequest / open_pull_request / "pr.open" [Operate, 11, unary]: $crate::forge::OpenPullRequest => $crate::forge::OpenOutcome;
            /// Merge a session's pull request, at the head the caller saw.
            MergePullRequest / merge_pull_request / "pr.merge" [Operate, 11, unary]: $crate::forge::MergePullRequest => $crate::forge::MergeOutcome;
            /// Turn feedback to the session on or off.
            SetFeedback / set_feedback / "pr.feedback" [Operate, 11, unary]: $crate::forge::SetFeedback => $crate::forge::SessionPullRequest;
            /// Open a shell beside a session or in a project (protocol 11).
            CreateTerminal / create_terminal / "terminal.create" [Operate, 11, unary]: $crate::terminals::CreateTerminal => $crate::terminals::TerminalInfo;
            /// The terminals (protocol 11).
            ListTerminals / list_terminals / "terminal.list" [Read, 11, unary]: $crate::terminals::ListTerminals => $crate::terminals::TerminalList;
            /// One terminal, or its next change (protocol 11).
            GetTerminal / get_terminal / "terminal.get" [Read, 11, unary]: $crate::terminals::GetTerminal => $crate::terminals::TerminalInfo;
            /// A page of a terminal's log (protocol 11).
            ReadTerminal / read_terminal / "terminal.read" [Read, 11, unary]: $crate::terminals::ReadTerminal => $crate::EventPage;
            /// Subscribe to a terminal's log (protocol 11).
            SubscribeTerminal / subscribe_terminal / "terminal.subscribe" [Read, 11, stream]: $crate::terminals::SubscribeTerminal => $crate::terminals::TerminalSubscribed;
            /// Keystrokes or a resize, for a shell (protocol 11).
            SendTerminal / send_terminal / "terminal.send" [Operate, 11, unary]: $crate::terminals::SendTerminal => $crate::Ack;
            /// Give a terminal a title (protocol 11).
            RenameTerminal / rename_terminal / "terminal.rename" [Operate, 11, unary]: $crate::terminals::RenameTerminal => $crate::terminals::TerminalInfo;
            /// End a terminal's process, keeping its output (protocol 11).
            KillTerminal / kill_terminal / "terminal.kill" [Operate, 11, unary]: $crate::terminals::KillTerminal => $crate::Ack;
            /// End a terminal and forget it (protocol 11).
            CloseTerminal / close_terminal / "terminal.close" [Operate, 11, unary]: $crate::terminals::CloseTerminal => $crate::Ack;
            /// A place's scripts, their approval and their runs (protocol 11).
            ListScripts / list_scripts / "script.list" [Read, 11, unary]: $crate::scripts::ListScripts => $crate::scripts::ScriptList;
            /// Run a project's script or start a service (protocol 11).
            RunScript / run_script / "script.run" [Operate, 11, unary]: $crate::scripts::RunScript => $crate::scripts::ScriptRunOutcome;
            /// Run a script's run again (protocol 11).
            RestartScript / restart_script / "script.restart" [Operate, 11, unary]: $crate::scripts::RestartScript => $crate::scripts::ScriptRunOutcome;
            /// The ports the node's sessions and terminals listen on (protocol 11).
            ListPorts / list_ports / "port.list" [Read, 11, unary]: $crate::ports::ListPorts => $crate::ports::PortList;
            /// The agents the node can run (protocol 11, ADR 0032).
            ListAgents / list_agents / "agent.list" [Read, 11, unary]: $crate::agents::ListAgents => Vec<$crate::agents::AgentInfo>;
            /// Add an agent by its command (protocol 11).
            AddAgent / add_agent / "agent.add" [Admin, 11, unary]: $crate::agents::AddAgent => $crate::agents::AgentInfo;
            /// Forget an added agent (protocol 11).
            RemoveAgent / remove_agent / "agent.remove" [Admin, 11, unary]: $crate::agents::RemoveAgent => $crate::Ack;
            /// Whether agents are installed and signed in (protocol 11).
            GetAgentStatus / agent_status / "agent.status" [Read, 11, unary]: $crate::agents::GetAgentStatus => Vec<$crate::agents::AgentReadiness>;
            /// The ACP registry, as the node's copy (protocol 11).
            GetCatalogue / agent_catalogue / "agent.catalogue" [Read, 11, unary]: $crate::agents::GetCatalogue => $crate::agents::Catalogue;
            /// Run one of the four's documented installers (protocol 11).
            InstallAgent / install_agent / "agent.install" [Admin, 11, unary]: $crate::agents::InstallAgent => $crate::agents::InstallJob;
            /// A page of an install's output (protocol 11).
            ReadInstall / install_output / "agent.install_output" [Admin, 11, unary]: $crate::agents::ReadInstall => $crate::agents::InstallOutput;
            /// Reported tokens and permitted costs (protocol 21, ADR 0042).
            QueryUsage / query_usage / "usage.query" [Read, {$crate::usage::USAGE_PROTOCOL_VERSION.get()}, unary]: $crate::usage::QueryUsage => $crate::usage::UsageReport;
            /// Device-scoped push.key (protocol 22, ADR 0041).
            GetPushKey / get_push_key / "push.key" [Read, 22, unary]: $crate::push::GetPushKey => $crate::push::PushKey;
            /// Device-scoped push.subscribe (protocol 22, ADR 0041).
            SubscribePush / subscribe_push / "push.subscribe" [Read, 22, unary]: $crate::push::SubscribePush => $crate::push::PushLease;
            /// Device-scoped push.unsubscribe (protocol 22, ADR 0041).
            UnsubscribePush / unsubscribe_push / "push.unsubscribe" [Read, 22, unary]: $crate::push::UnsubscribePush => $crate::Ack;
            /// Device-scoped push.presence (protocol 22, ADR 0041).
            ReportPushPresence / report_push_presence / "push.presence" [Read, 22, unary]: $crate::push::ReportPushPresence => $crate::Ack;
            /// A bounded page of this node's durable attention (protocol 22).
            ListInbox / list_inbox / "inbox.list" [Read, 22, unary]: $crate::inbox::ListInbox => $crate::inbox::InboxPage;
            /// Recheck exactly one inbox source after a delayed answer (protocol 22).
            GetInboxEntry / get_inbox_entry / "inbox.get" [Read, 22, unary]: $crate::inbox::GetInboxEntry => Option<$crate::inbox::InboxEntry>;
            /// Mark current attention read or unread without answering it (protocol 22).
            MarkInbox / mark_inbox / "inbox.mark" [Read, 22, unary]: $crate::inbox::MarkInbox => $crate::Ack;
            /// Latest saved-sign-in limits (protocol 23, ADR 0042); explicit refresh additionally needs operate.
            GetUsageLimits / get_usage_limits / "usage.limits" [Read, {$crate::limits::LIMITS_PROTOCOL_VERSION.get()}, unary]: $crate::limits::GetUsageLimits => $crate::limits::UsageLimits;
            /// Persist this machine's credential-use policy (protocol 23); administrator only.
            SetUsageLookup / set_usage_lookup / "usage.lookup" [Admin, {$crate::limits::LIMITS_PROTOCOL_VERSION.get()}, unary]: $crate::limits::SetUsageLookup => $crate::limits::ExternalLookup;
            // Protocol 11, track D (ADR 0033): schedules and saved prompts.
            /// Compute a bounded schedule preview without saving or starting work.
            PreviewSchedule / preview_schedule / "schedule.preview" [Read, 13, unary]: $crate::automation::PreviewSchedule => $crate::automation::SchedulePreview;
            /// Every schedule on the node (protocol 11).
            ListSchedules / list_schedules / "schedule.list" [Read, 11, unary]: $crate::automation::ListSchedules => $crate::automation::ScheduleList;
            /// One schedule (protocol 11).
            GetSchedule / get_schedule / "schedule.get" [Read, 11, unary]: $crate::automation::GetSchedule => $crate::automation::ScheduleInfo;
            /// Make a schedule (protocol 11).
            CreateSchedule / create_schedule / "schedule.create" [Operate, 11, unary]: $crate::automation::CreateSchedule => $crate::automation::ScheduleInfo;
            /// Replace a schedule's definition (protocol 11).
            UpdateSchedule / update_schedule / "schedule.update" [Operate, 11, unary]: $crate::automation::UpdateSchedule => $crate::automation::ScheduleInfo;
            /// Change an existing-session schedule's cadence while retaining its owner.
            UpdateScheduleCadence / update_schedule_cadence / "schedule.update_cadence" [Operate, (pending $crate::automation::HEARTBEAT_CADENCE_PROTOCOL), unary]: $crate::automation::UpdateScheduleCadence => $crate::automation::ScheduleInfo;
            /// Stop a schedule's runs (protocol 11).
            PauseSchedule / pause_schedule / "schedule.pause" [Operate, 11, unary]: $crate::automation::PauseSchedule => $crate::automation::ScheduleInfo;
            /// Let a paused schedule run again (protocol 11).
            ResumeSchedule / resume_schedule / "schedule.resume" [Operate, 11, unary]: $crate::automation::ResumeSchedule => $crate::automation::ScheduleInfo;
            /// Run a schedule once now (protocol 11).
            RunSchedule / run_schedule / "schedule.run" [Operate, 11, unary]: $crate::automation::RunSchedule => $crate::automation::ScheduleRun;
            /// Remove a schedule and its runs (protocol 11).
            DeleteSchedule / delete_schedule / "schedule.delete" [Operate, 11, unary]: $crate::automation::DeleteSchedule => $crate::Ack;
            /// A schedule's newest runs (protocol 11).
            ListRuns / list_runs / "schedule.runs" [Read, 11, unary]: $crate::automation::ListRuns => $crate::automation::RunList;
            /// The saved prompts (protocol 11).
            ListPrompts / list_prompts / "prompt.list" [Read, 11, unary]: $crate::automation::ListPrompts => $crate::automation::PromptList;
            /// Save a prompt (protocol 11).
            CreatePrompt / create_prompt / "prompt.create" [Operate, 11, unary]: $crate::automation::CreatePrompt => $crate::automation::SavedPrompt;
            /// Change a saved prompt (protocol 11).
            UpdatePrompt / update_prompt / "prompt.update" [Operate, 11, unary]: $crate::automation::UpdatePrompt => $crate::automation::SavedPrompt;
            /// Remove a saved prompt (protocol 11).
            DeletePrompt / delete_prompt / "prompt.delete" [Operate, 11, unary]: $crate::automation::DeletePrompt => $crate::Ack;
            // Protocol 12, track A2: the conversation, further.
            /// Search every session's messages (protocol 12, ADR 0037).
            SearchMessages / search_messages / "session.search" [Read, 12, unary]: $crate::search::SearchMessages => $crate::search::SearchPage;
            /// Stop an idle session's agent, keeping the session (protocol 12, ADR 0038).
            ParkSession / park_session / "session.park" [Operate, 12, unary]: $crate::parking::ParkSession => $crate::SessionInfo;
            /// Start a parked session's agent again (protocol 12).
            WakeSession / wake_session / "session.wake" [Operate, 12, unary]: $crate::parking::WakeSession => $crate::SessionInfo;
            /// The node's settings in force (protocol 12, ADR 0038).
            GetNodeSettings / node_settings / "node.settings" [Read, 12, unary]: $crate::settings::GetNodeSettings => $crate::settings::NodeSettings;
            /// Change the node's settings; kept across restarts (protocol 12).
            SetNodeSettings / set_node_settings / "node.set_settings" [Admin, 12, unary]: $crate::settings::SetNodeSettings => $crate::settings::NodeSettings;
            /// Read the host's saved system prompt supplement (Admin only).
            GetAppendSystemPrompt / get_append_system_prompt / "node.append_system_prompt" [Admin, (pending $crate::settings::APPEND_SYSTEM_PROMPT_PROTOCOL), unary]: $crate::settings::GetAppendSystemPrompt => $crate::settings::AppendSystemPrompt;
            /// A new session continuing another's conversation from a turn's end (protocol 14, ADR 0052).
            ForkSession / fork_session / "session.fork" [Operate, 14, unary]: $crate::forking::ForkSession => $crate::SessionInfo;
            /// Take a session back to a turn's end (protocol 14, ADR 0052).
            RewindSession / rewind_session / "session.rewind" [Operate, 14, unary]: $crate::forking::RewindSession => $crate::SessionInfo;
            /// A page of an agent's own conversations, to import one (protocol 17, ADR 0053).
            GetAgentHistory / agent_history / "agent.history" [Operate, 17, unary]: $crate::history::GetAgentHistory => $crate::history::AgentHistoryPage;
            /// Make a managed child independent without changing its process or conversation.
            DetachSession / detach_session / "session.detach" [Operate, (pending $crate::subagents::SUBAGENT_RELATIONS), unary]: $crate::subagents::DetachSession => $crate::subagents::DetachedSession;
            /// Read an atomic independent-workspace registry snapshot.
            GetRegistry / get_registry / "workspace.registry" [Read, (pending $crate::registry::PROJECT_WORKSPACE_REGISTRY), unary]: $crate::registry::GetRegistry => $crate::registry::RegistrySnapshot;
            /// List active/archived workspace descriptors independently of sessions.
            ListWorkspaces / list_workspaces / "workspace.list" [Read, (pending $crate::registry::PROJECT_WORKSPACE_REGISTRY), unary]: $crate::registry::ListWorkspaces => Vec<$crate::registry::ServedWorkspace>;
            /// Inspect one workspace including its archived lifecycle.
            GetWorkspace / get_workspace / "workspace.get" [Read, (pending $crate::registry::PROJECT_WORKSPACE_REGISTRY), unary]: $crate::registry::GetWorkspace => $crate::registry::ServedWorkspace;
            /// Rename one active workspace without changing its filesystem placement.
            RenameWorkspace / rename_workspace / "workspace.rename" [Operate, (pending $crate::registry::PROJECT_WORKSPACE_REGISTRY), unary]: $crate::registry::RenameWorkspace => $crate::registry::WorkspaceDescriptor;
            /// Persist one workspace pin without changing client-local ordering.
            PinWorkspace / pin_workspace / "workspace.pin" [Operate, (pending $crate::registry::PROJECT_WORKSPACE_REGISTRY), unary]: $crate::registry::PinWorkspace => $crate::registry::WorkspaceDescriptor;
            /// Release automation for one explicitly inspected external source and revision.
            ReleaseWorkspaceAutomation / release_workspace_automation / "workspace.release_automation" [Operate, (pending $crate::registry::PROJECT_WORKSPACE_REGISTRY), unary]: $crate::registry::ReleaseWorkspaceAutomation => $crate::registry::WorkspaceDescriptor;

            /// Distinct committed/All comparison summary (ADR 0087, staged values).
            GitCompare / git_compare / "git.compare" [Read, (staged $crate::workspace::GIT_PANEL), unary]: $crate::workspace::CompareGit => $crate::workspace::GitCompared<$crate::workspace::DiffSummary>;
            /// A bounded file in the same comparison (ADR 0087, staged values).
            GitCompareFile / git_compare_file / "git.compare_file" [Read, (staged $crate::workspace::GIT_PANEL), unary]: $crate::workspace::CompareGitFile => $crate::workspace::GitCompared<$crate::workspace::FileDiff>;
            /// Collect bounded content-free diagnostics (protocol 24).
            Diagnostics / diagnostics / "node.diagnostics" [Read, (pending $crate::diagnostics::DIAGNOSTICS_PROTOCOL), unary]: $crate::diagnostics::GetDiagnostics => $crate::diagnostics::DiagnosticReport;
            /// Rich explorer listing with honest modification metadata (ADR 0083, protocol 27).
            InspectDirectory / inspect_directory / "file.inspect" [Read, (pending $crate::workspace::panel_files::FILE_PANELS_PROTOCOL), unary]: $crate::workspace::directory_metadata::InspectDirectory => $crate::workspace::directory_metadata::InspectedDirectory;
            /// Exact, version-bound binary page (ADR 0083, protocol 27).
            ReadFileBytes / read_file_bytes / "file.read_bytes" [Read, (pending $crate::workspace::panel_files::FILE_PANELS_PROTOCOL), unary]: $crate::workspace::panel_files::ReadFileBytes => $crate::workspace::panel_files::FileBytePage;
            /// Exclusive sibling duplication, never an overwrite (ADR 0083, protocol 27).
            DuplicateFileEntry / duplicate_file_entry / "file.duplicate" [Operate, (pending $crate::workspace::panel_files::FILE_PANELS_PROTOCOL), unary]: $crate::workspace::panel_files::DuplicateFileEntry => $crate::workspace::panel_files::DuplicatedFileEntry;
            /// Finite parent-directory observation of one saved file (ADR 0083, protocol 27).
            ObserveFile / observe_file / "file.observe" [Read, (pending $crate::workspace::panel_files::FILE_PANELS_PROTOCOL), unary]: $crate::workspace::panel_files::ObserveFile => $crate::workspace::panel_files::ObservedFile;
            /// Bounded root-scoped file/directory discovery (ADR 0083, protocol 27).
            SuggestEntries / suggest_entries / "file.suggest" [Read, (pending $crate::workspace::panel_files::FILE_PANELS_PROTOCOL), unary]: $crate::workspace::suggestions::SuggestEntries => $crate::workspace::suggestions::EntrySuggestions;

            /// Host orchestration skills (staged ADR 0065, Admin only).
            GetSkillsStatus / skills_status / "skills.status" [Admin, (staged $crate::mcp::MCP_PARITY), unary]: $crate::skills::GetSkillsStatus => $crate::skills::SkillsSnapshot;
            /// Host orchestration skills (staged ADR 0065, Admin only).
            ReconcileSkills / reconcile_skills / "skills.reconcile" [Admin, (staged $crate::mcp::MCP_PARITY), unary]: $crate::skills::ReconcileSkills => $crate::skills::SkillsSnapshot;
            /// Host orchestration skills (staged ADR 0065, Admin only).
            UninstallSkills / uninstall_skills / "skills.uninstall" [Admin, (staged $crate::mcp::MCP_PARITY), unary]: $crate::skills::UninstallSkills => $crate::skills::SkillsSnapshot;
            /// Host orchestration skills (staged ADR 0065, Admin only).
            SaveSkillsSelection / save_skills_selection / "skills.save_selection" [Admin, (staged $crate::mcp::MCP_PARITY), unary]: $crate::skills::SaveSkillsSelection => $crate::skills::SkillsSaved;
            /// Host orchestration skills (staged ADR 0065, Admin only).
            ImportSkillsSelection / import_skills_selection / "skills.import_selection" [Admin, (staged $crate::mcp::MCP_PARITY), unary]: $crate::skills::ImportSkillsSelection => $crate::skills::SkillsImported;

            /// Complete source/account reports (protocol 26, ADR 0042).
            ListUsageReports / list_usage_reports / "usage.reports" [Read, (pending $crate::usage_reports::ACCOUNT_REPORTS_PROTOCOL), unary]: $crate::usage_reports::ListUsageReports => $crate::usage_reports::UsageReports;
            /// Resolve exactly one opaque report id without widening its scope.
            InspectUsageReport / inspect_usage_report / "usage.inspect" [Read, (pending $crate::usage_reports::ACCOUNT_REPORTS_PROTOCOL), unary]: $crate::usage_reports::InspectUsageReport => $crate::usage_reports::UsageReportLookup;
            /// Accept or revoke exactly the displayed source disclosure; never fetches.
            SetUsageSourceLookup / set_usage_source_lookup / "usage.source_lookup" [Admin, (pending $crate::usage_reports::USAGE_SOURCE_CONSENT_PROTOCOL), unary]: $crate::usage_reports::SetUsageSourceLookup => $crate::usage_reports::UsageSourceDescriptor;
            /// Observe the node-local catalog with a bounded optional wait.
            ListWorkspaceLabels / list_workspace_labels / "workspace.label.list" [Read, (pending $crate::workspace_labels::WORKSPACE_LABELS), unary]: $crate::workspace_labels::ListWorkspaceLabels => $crate::workspace_labels::CatalogList;
            /// Set one active workspace assignment and optional new definition atomically.
            AssignWorkspaceLabel / assign_workspace_label / "workspace.label.assign" [Operate, (pending $crate::workspace_labels::WORKSPACE_LABELS), unary]: $crate::workspace_labels::AssignWorkspaceLabel => $crate::workspace_labels::WorkspaceLabelAssigned;
            /// Atomically rename and recolor one host definition.
            UpdateWorkspaceLabel / update_workspace_label / "workspace.label.update" [Operate, (pending $crate::workspace_labels::WORKSPACE_LABELS), unary]: $crate::workspace_labels::UpdateWorkspaceLabel => $crate::workspace_labels::WorkspaceLabelUpdated;
            /// Inspect an advisory affected count without locking later deletion.
            InspectWorkspaceLabelDelete / inspect_workspace_label_delete / "workspace.label.inspect_delete" [Read, (pending $crate::workspace_labels::WORKSPACE_LABELS), unary]: $crate::workspace_labels::InspectWorkspaceLabelDelete => $crate::workspace_labels::WorkspaceLabelDeleted;
            /// Delete a definition and every assignment in one commit.
            DeleteWorkspaceLabel / delete_workspace_label / "workspace.label.delete" [Operate, (pending $crate::workspace_labels::WORKSPACE_LABELS), unary]: $crate::workspace_labels::DeleteWorkspaceLabel => $crate::workspace_labels::WorkspaceLabelDeleted;

            /// Workspace service observations only; unavailable until the qualified runtime wiring.
            ListWorkspaceScripts / workspace_scripts / "workspace.script.list" [Read, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::WorkspaceScriptQuery => $crate::workspace_services::WorkspaceScriptQueryResult;
            /// A person's exact workspace script start; no default runtime implementation.
            StartWorkspaceScript / start_workspace_script / "workspace.script.start" [Operate, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::StartWorkspaceScript => $crate::workspace_services::WorkspaceScriptOutcome;
            /// Stop requires qualified actual cleanup, not a metadata acknowledgement.
            StopWorkspaceScript / stop_workspace_script / "workspace.script.stop" [Operate, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::StopWorkspaceScript => $crate::workspace_services::WorkspaceScriptOutcome;
            /// Restart requires the canonical predecessor lease and actual joined run.
            RestartWorkspaceScript / restart_workspace_script / "workspace.script.restart" [Operate, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::RestartWorkspaceScript => $crate::workspace_services::WorkspaceScriptOutcome;
            /// Read only a cached committed generated environment plan; never prepare it.
            GetWorkspaceServiceEnv / workspace_service_env / "workspace.service.env" [Read, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::GetWorkspaceServiceEnv => $crate::workspace_services::ServiceEnvPlan;
            /// Private service settings are direct Admin data, never generic node Read.
            GetServiceSettings / service_settings / "service.settings" [Admin, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::GetServiceSettings => $crate::workspace_services::ServicesAdminSettings;
            /// Gateway choice grants no per-service publication consent.
            SetServiceGateway / set_service_gateway / "service.set_gateway" [Admin, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::SetServiceGateway => $crate::workspace_services::ServiceSettingsOutcome;
            /// Explicit anonymous consent still needs canonical incarnation before any write.
            SetServiceExposure / set_service_exposure / "service.set_exposure" [Admin, (staged $crate::workspace_services::WORKSPACE_SERVICES), unary]: $crate::workspace_services::SetServiceExposure => $crate::workspace_services::ServiceSettingsOutcome;
            /// Start a session's agent again on its conversation, as Paseo's "Reload agent" (TRACKER 803, ADR 0110).
            ReloadSession / reload_session / "session.reload" [Operate, (pending $crate::reload::RELOAD_PROTOCOL), unary]: $crate::reload::ReloadSession => $crate::SessionInfo;
            /// Folders under the node user's home for Add project, as Paseo's (ADR 0112).
            SuggestFolders / suggest_folders / "folder.suggest" [Operate, (pending $crate::workspace::host_folders::HOST_FOLDERS_PROTOCOL), unary]: $crate::workspace::host_folders::SuggestFolders => $crate::workspace::host_folders::FolderSuggestions;
            /// A project's own icon, inside its root, as Paseo's (ADR 0112).
            ProjectIcon / project_icon / "project.icon" [Read, (pending $crate::workspace::host_folders::HOST_FOLDERS_PROTOCOL), unary]: $crate::workspace::host_folders::GetProjectIcon => $crate::workspace::host_folders::ProjectIcon;
            /// One agent's diagnostic with a new probe of its catalogue, as Paseo's provider diagnostic (ADR 0111).
            GetAgentDiagnostic / agent_diagnostic / "agent.diagnostic" [Read, (pending $crate::diagnostics::DIAGNOSTICS_WIRE_PROTOCOL), unary]: $crate::agents::GetAgentDiagnostic => $crate::agents::AgentDiagnostic;
            /// The last lines of the node's own log, for its administrators (ADR 0111).
            ReadNodeLog / node_log / "node.log" [Admin, (pending $crate::diagnostics::DIAGNOSTICS_WIRE_PROTOCOL), unary]: $crate::diagnostics::ReadNodeLog => $crate::diagnostics::NodeLog;
            /// One new empty folder made a project, as Paseo's New directory (ADR 0113).
            CreateProjectDirectory / create_project_directory / "project.create_directory" [Operate, (pending $crate::registry::PROJECT_DIRECTORY_PROTOCOL), unary]: $crate::registry::CreateProjectDirectory => $crate::workspace::ProjectInfo;
            /// The signed-in account's GitHub repositories, or a search, for Add project (ADR 0088).
            SearchRepositories / search_repositories / "forge.repos" [Operate, (pending $crate::forge::REPOSITORY_SEARCH_PROTOCOL), unary]: $crate::forge::SearchRepositories => $crate::forge::RepositoryList;
            /// This node's plugin installations, as Paseo's plugin list (ADR 0115).
            ListPlugins / list_plugins / "plugin.list" [Read, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::ListPlugins => $crate::plugins::PluginList;
            /// One installation's retained log tail, for administrators (ADR 0115).
            GetPluginLogs / plugin_logs / "plugin.logs" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::GetPluginLogs => $crate::plugins::PluginLogs;
            /// Read a plugin directory's manifest without installing it (ADR 0115).
            InspectPlugin / inspect_plugin / "plugin.inspect" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::InspectPlugin => $crate::plugins::PluginInspection;
            /// Install a plugin from Paseo's source string; directories only for now (ADR 0115).
            InstallPlugin / install_plugin / "plugin.install" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::InstallPlugin => $crate::plugins::PluginListItem;
            /// Save an installation as enabled and start it when plugins are on (ADR 0115).
            EnablePlugin / enable_plugin / "plugin.enable" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::EnablePlugin => $crate::plugins::PluginListItem;
            /// Save an installation as disabled and stop it (ADR 0115).
            DisablePlugin / disable_plugin / "plugin.disable" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::DisablePlugin => $crate::plugins::PluginListItem;
            /// Stop an installation and start it again from disk (ADR 0115).
            ReloadPlugin / reload_plugin / "plugin.reload" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::ReloadPlugin => $crate::plugins::PluginListItem;
            /// Stop and forget an installation, its settings and logs (ADR 0115).
            RemovePlugin / remove_plugin / "plugin.remove" [Admin, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::RemovePlugin => $crate::Ack;
            /// Wait for the plugin generation to move, a long poll as fleet.watch (ADR 0115).
            WatchPlugins / watch_plugins / "plugin.watch" [Read, (pending $crate::plugins::PLUGINS_PROTOCOL), unary]: $crate::plugins::WatchPlugins => $crate::plugins::PluginGeneration;
            /// Claim one local delivery opportunity for this authenticated connection.
            ClaimNotification / claim_notification / "notification.claim" [Read, (pending $crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL), unary]: $crate::notifications::ClaimNotification => $crate::notifications::NotificationClaim;
            /// Clear only the terminal attention revision actually observed.
            ClearTerminalAttention / clear_terminal_attention / "terminal.attention_clear" [Read, (pending $crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL), unary]: $crate::notifications::ClearTerminalAttention => $crate::notifications::TerminalAttention;
            /// Report explicit activity for an owned or capability-bound terminal.
            ReportTerminalActivity / report_terminal_activity / "terminal.activity" [Operate, (pending $crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL), unary]: $crate::notifications::ReportTerminalActivity => $crate::notifications::TerminalAttention;
            /// Authoritative per-session inbox badges, independent of the loaded page.
            GetInboxAttention / inbox_attention / "inbox.attention" [Read, (pending $crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL), unary]: $crate::inbox_attention::GetInboxAttention => Vec<$crate::inbox_attention::SessionAttention>;
            /// Mark observed attention read without granting general session operations.
            ReadInboxThrough / read_inbox_through / "inbox.read" [Read, (pending $crate::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL), unary]: $crate::inbox_attention::ReadInboxThrough => $crate::Ack;
            /// A session's pull request's reviews, comments and review threads, as Paseo's pane (ADR 0088).
            GetPullTimeline / pull_timeline / "pr.timeline" [Read, (pending $crate::forge::PULL_REQUEST_PANE_PROTOCOL), unary]: $crate::forge::GetPullTimeline => $crate::forge::PullTimeline;
            /// One check of a session's pull request, with its failed jobs' log tails (ADR 0088).
            GetCheckDetails / check_details / "pr.check" [Read, (pending $crate::forge::PULL_REQUEST_PANE_PROTOCOL), unary]: $crate::forge::GetCheckDetails => $crate::forge::CheckDetails;
            /// The running installations' client declarations, as Paseo's plugin catalog (ADR 0115).
            GetPluginCatalog / plugin_catalog / "plugin.catalog" [Read, (pending $crate::plugins::PLUGIN_CATALOG_PROTOCOL), unary]: $crate::plugins::GetPluginCatalog => $crate::plugins::PluginCatalog;
            /// Call a method a running installation declared, as Paseo's plugin RPC (ADR 0115).
            InvokePlugin / invoke_plugin / "plugin.invoke" [Admin, (pending $crate::plugins::PLUGIN_CATALOG_PROTOCOL), unary]: $crate::plugins::InvokePlugin => $crate::plugins::PluginOutput;
            /// One settings document of a running installation, as Paseo's `settings.<id>.read` (ADR 0115, "Settings").
            ReadPluginSettings / read_plugin_settings / "plugin.settings.read" [Admin, (pending $crate::plugins::PLUGIN_SETTINGS_PROTOCOL), unary]: $crate::plugins::ReadPluginSettings => $crate::plugins::PluginSettingsRead;
            /// Save a complete settings document at the revision read, as Paseo's `settings.<id>.write` (ADR 0115, "Settings").
            WritePluginSettings / write_plugin_settings / "plugin.settings.write" [Admin, (pending $crate::plugins::PLUGIN_SETTINGS_PROTOCOL), unary]: $crate::plugins::WritePluginSettings => $crate::plugins::PluginSettingsWrite;
            /// Replace a settings document with its defaults, as Paseo's `settings.<id>.reset` (ADR 0115, "Settings").
            ResetPluginSettings / reset_plugin_settings / "plugin.settings.reset" [Admin, (pending $crate::plugins::PLUGIN_SETTINGS_PROTOCOL), unary]: $crate::plugins::ResetPluginSettings => $crate::plugins::PluginSettingsWrite;
        }
    };
}
