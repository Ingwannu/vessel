// Generated from the Rust wire types (ADR 0017). Do not edit: running
// `VESSEL_WRITE_TS=1 cargo test --features ts typescript` in the crate that
// generates this file rewrites it, and CI fails while it differs.

/**
 * Refresh state, separate from the successful observation's age.
 */
export type AccountLimitState = { "state": "off" } | { "state": "not_requested" } | { "state": "refreshing", 
/**
 * Last safe response, if any.
 */
retained: LimitSnapshot | null, } | { "state": "ready", 
/**
 * Latest successful observation.
 */
snapshot: LimitSnapshot, } | { "state": "failed", 
/**
 * Safe action category.
 */
reason: LimitFailure, 
/**
 * Previous same-login response only.
 */
retained: LimitSnapshot | null, 
/**
 * Actual cooldown when known.
 */
retry_at: string | null, };

/**
 * Empty success.
 */
export type Ack = null;

/**
 * An ACP protocol observation, never a Vessel `ProtocolVersion`.
 */
export type AcpProtocolVersion = number;

/**
 * The latest state of a session's agent, as the node observed it.
 */
export type Activity = "active" | "idle" | "waiting_input" | "exited";

/**
 * `agent.add`: define an agent on the node by its command (`agents.toml`),
 * and answer with it as now in effect. Starts and fetches nothing. Needs
 * `admin`, and is refused to an agent and to a call from another node.
 */
export type AddAgent = { 
/**
 * Its name; agents' and profiles' names are one space, compared
 * ignoring case.
 */
name: AgentName, 
/**
 * What to add.
 */
agent: AgentToAdd, 
/**
 * Replace an agent of that name (sessions running keep what they
 * started with).
 */
replace?: boolean, };

/**
 * `project.add`: a folder on the node.
 */
export type AddProject = { 
/**
 * An existing folder, absolute.
 */
path: string, 
/**
 * What to call it; its folder's name otherwise.
 */
name?: ProjectName | null, };

/**
 * An added agent as clients see it: its command whole, its environment by
 * name. Values never leave the node (as for profiles, ADR 0011): a text
 * value may be a credential someone wrote there.
 */
export type AddedAgent = { 
/**
 * What a person calls it.
 */
label?: AgentLabel | null, 
/**
 * Its program as written.
 */
program: AgentProgram, 
/**
 * Its arguments as written (`~` expanded).
 */
args: Array<string>, 
/**
 * The variables it sets to text, by name.
 */
env: Array<EnvName>, 
/**
 * The variables it takes from the node's secrets.
 */
secrets: Array<SecretEnv>, 
/**
 * The variables passed on from the node's own environment.
 */
inherit: Array<EnvName>, 
/**
 * The fleet's MCP servers it gets, by name (ADR 0012); it gets no
 * other but the `vessel` server.
 */
mcp: Array<McpServerName>, 
/**
 * Where its definition came from.
 */
source: AgentSource, };

/**
 * What becomes of a run's new session once its first turn has ended.
 */
export type AfterRun = "archive" | "keep" | "unknown";

/**
 * What an agent can do, from its ACP `initialize` answer.
 *
 * [Decision Log]
 * - Purpose: steering, starting again and attaching images are offered
 *   only where the agent takes them, and a client must know before it
 *   offers the control, not from a failure after.
 * - Choice: read once per agent start (`initialize`), logged with the
 *   session's `ready` and kept by the store, so an ended session still
 *   says whether it can start again. Files are not here: every
 *   structured agent takes a resource link, ACP's baseline content.
 */
export type AgentAbilities = { 
/**
 * A message can be added to a running turn: the agent's answer
 * carries `_meta.steering.supported: true` (the ACP steering
 * extension, `_session/steering`).
 */
steer: boolean, 
/**
 * An ended conversation can be loaded again (`loadSession`).
 */
resume: boolean, 
/**
 * A prompt may hold images (`promptCapabilities.image`).
 */
image: boolean, 
/**
 * A prompt may hold audio (`promptCapabilities.audio`).
 */
audio: boolean, 
/**
 * The conversation can be forked (`sessionCapabilities.fork`, ACP's
 * unstable `session/fork`; protocol 14, ADR 0052). Absent from older
 * nodes and sessions started before 14: a fork's own handshake
 * decides, not this.
 */
fork: boolean, };

/**
 * One command the agent offers (ACP `available_commands_update`). A
 * client uses it by sending a prompt that starts with `/name`.
 */
export type AgentCommand = { 
/**
 * Its name, without the `/`.
 */
name: CommandName, 
/**
 * What it does, in the agent's words (cut at
 * [`AgentCommand::MAX_DESCRIPTION`] bytes by the node).
 */
description: string, 
/**
 * What to type after it, when it takes input (cut at
 * [`AgentCommand::MAX_HINT`] bytes).
 */
hint?: string | null, 
/**
 * Explicit command classification (ADR 0085); absent from older reports.
 * Only a Skill may be completed inline; unknown and absent remain root-only.
 */
kind?: CommandKind | null, };

/**
 * What `agent.add` writes when a person gives the command.
 */
export type AgentDefinition = { 
/**
 * What a person calls it.
 */
label?: AgentLabel | null, 
/**
 * Its program: a name found on the node's search path, or an absolute
 * path.
 */
program: AgentProgram, 
/**
 * Its arguments. Every client sees them, and the process table shows
 * them: a secret belongs in `env` as a secret's name.
 */
args?: Array<string>, 
/**
 * The variables it sets: text, or a secret of the node by name.
 */
env?: Array<AgentVariable>, 
/**
 * The variables passed on from the node's own environment.
 */
inherit?: Array<EnvName>, 
/**
 * The fleet's MCP servers it gets, by name.
 */
mcp?: Array<McpServerName>, };

/**
 * What a node knows of one agent.
 */
export type AgentDetail = { "kind": "known", 
/**
 * Its vendor's documented installs: the agent, and its ACP adapter
 * when it needs one.
 */
installs: Array<InstallOffer>, 
/**
 * How a person signs in.
 */
sign_in: SignInHelp, } | { "kind": "added", 
/**
 * Its definition, values left out.
 */
added: AddedAgent, };

/**
 * Which protocol drives an agent's structured sessions (protocol 18, ADR
 * 0057).
 */
export type AgentDriver = "acp" | "direct" | "unknown";

/**
 * The driver of each agent that has a protocol of its own (protocol 18).
 * Other agents run over ACP.
 */
export type AgentDrivers = { 
/**
 * Claude Code's.
 */
claude_code: AgentDriver, 
/**
 * Codex's.
 */
codex: AgentDriver, 
/**
 * `OpenCode`'s.
 */
open_code: AgentDriver, };

/**
 * One entry in a structured session's timeline.
 */
export type AgentEvent = { "event": "ready", 
/**
 * The harness's session id.
 */
harness_session: HarnessSessionId, 
/**
 * What the agent said it can do in its handshake (protocol 10);
 * absent from older nodes.
 */
abilities?: AgentAbilities | null, } | { "event": "turn_started", 
/**
 * The prompt.
 */
content: Array<ContentBlock>, } | { "event": "user_message_chunk", 
/**
 * The chunk.
 */
content: ContentBlock, } | { "event": "message_chunk", 
/**
 * The chunk.
 */
content: ContentBlock, } | { "event": "thought_chunk", 
/**
 * The chunk.
 */
content: ContentBlock, } | { "event": "tool_call" } & ToolCall | { "event": "tool_call_update" } & ToolCallUpdate | { "event": "plan", 
/**
 * Plan entries in order.
 */
entries: Array<PlanEntry>, } | { "event": "permission_requested" } & PermissionRequest | { "event": "permission_resolved", 
/**
 * Which request.
 */
id: PermissionId, 
/**
 * How it was settled.
 */
outcome: PermissionOutcome, } | { "event": "turn_ended", 
/**
 * Why it stopped.
 */
reason: StopReason, 
/**
 * The turn's tokens, when the agent reports them with its end
 * (protocol 10).
 */
usage?: TurnUsage | null, 
/**
 * Where a fork or a rewind can start (protocol 14, ADR 0052):
 * set when the turn's last agent message carried an id and nothing
 * but the turn's end followed it. Absent otherwise, and from older
 * nodes.
 */
point?: TurnPoint | null, } | { "event": "turn_failed", 
/**
 * Human-readable reason.
 */
message: string, } | { "event": "harness_log", 
/**
 * The line, without its newline.
 */
line: string, } | { "event": "steered", 
/**
 * The message.
 */
content: Array<ContentBlock>, } | { "event": "question_asked" } & Question | { "event": "question_resolved", 
/**
 * Which question.
 */
id: QuestionId, 
/**
 * How it was settled.
 */
outcome: QuestionOutcome, };

/**
 * Reply to `agent.history`.
 */
export type AgentHistoryPage = { 
/**
 * Newest first, as the agent gives them; at most
 * [`AgentHistoryPage::MAX_ENTRIES`].
 */
conversations: Array<HistoryEntry>, 
/**
 * The next page's cursor, opaque; absent on the last page. A page
 * can be empty and still have one (an agent that filters by folder
 * after paging), after the node has asked several of the agent's.
 */
next?: string | null, };

/**
 * One agent as `agent.list` reports it.
 */
export type AgentInfo = { 
/**
 * Which agent.
 */
agent: HarnessRef, 
/**
 * What the node knows of it.
 */
detail: AgentDetail, };

/**
 * What a person calls an added agent (`Goose`), shown on the page.
 */
export type AgentLabel = string;

/**
 * The id an agent gives one of its messages (ACP's `messageId` on a
 * message chunk): what a fork at a turn names (protocol 14, ADR 0052).
 * At most [`AgentMessageId::MAX_BYTES`]; the node drops a longer one.
 */
export type AgentMessageId = string & { readonly __brand: "AgentMessageId" };

/**
 * The name of an agent added on a node by its command (protocol 11, ADR
 * 0032): the profile name rule (`[A-Za-z0-9._-]`, 1 to 64), and never a
 * name of one of the four known agents, in any case or spelling
 * (`claude`, `Claude-Code`, `opencode`).
 *
 * Why the profile rule: an added agent's default profile is named after
 * it, as the four's are (ADR 0011), so every agent name must be a profile
 * name. Why the four are refused: a word a person types is read as one of
 * the four first ([`HarnessRef::from_str`]), so an added agent called
 * `codex` could never be named.
 */
export type AgentName = string;

/**
 * An added agent's program as written: a name looked up on the node's
 * search path (`goose`, `npx`), or an absolute path.
 */
export type AgentProgram = string;

/**
 * One agent under one profile.
 */
export type AgentReadiness = { 
/**
 * Which agent.
 */
agent: HarnessRef, 
/**
 * Which profile (the agent's default has the agent's name).
 */
profile: ProfileName, 
/**
 * Whether it is installed.
 */
installed: Installed, 
/**
 * Whether it is signed in.
 */
signed_in: SignedIn, 
/**
 * When the older of the two facts was found.
 */
checked_at: string, };

/**
 * Where an added agent's definition came from.
 */
export type AgentSource = { "from": "command" } | { "from": "registry", 
/**
 * The entry's id.
 */
id: RegistryId, 
/**
 * The entry's version.
 */
version: RegistryVersion, };

/**
 * What `agent.add` adds.
 */
export type AgentToAdd = { "from": "command", 
/**
 * The definition.
 */
definition: AgentDefinition, } | { "from": "registry", 
/**
 * The entry.
 */
id: RegistryId, 
/**
 * Its version, as the catalogue showed it.
 */
version: RegistryVersion, 
/**
 * The fleet's MCP servers it gets.
 */
mcp?: Array<McpServerName>, };

/**
 * One agent/profile's totals on the queried machine.
 */
export type AgentUsage = { 
/**
 * Which agent.
 */
harness: HarnessRef, 
/**
 * Profile at the time of the sample, not its current assignment.
 */
profile: ProfileName | null, 
/**
 * Its reported usage.
 */
totals: UsageTotals, };

/**
 * One variable an added agent sets.
 */
export type AgentVariable = { 
/**
 * The variable.
 */
name: EnvName, 
/**
 * Its value: text, or `{ secret = "NAME" }`.
 */
value: ConfigValue, };

/**
 * An agent's version as it reports it (ACP `agentInfo.version`, or the
 * first line of a known agent's `--version`).
 */
export type AgentVersion = string;

/**
 * The value given for one field.
 */
export type AnswerValue = { "type": "text", "value": string } | { "type": "number", "value": FiniteNumber } | { "type": "integer", "value": number } | { "type": "boolean", "value": boolean } | { "type": "choice", "value": string } | { "type": "choices", "value": Array<string> };

/**
 * `project.approve`: approve (or withdraw) a set of commands.
 */
export type ApproveCommands = { 
/**
 * Which project.
 */
project: ProjectId, 
/**
 * The set, as `project.commands` gave it.
 */
digest: CommandsDigest, 
/**
 * Approve, or withdraw an approval.
 */
approved: boolean, };

/**
 * A set a person approved for a project, and when.
 */
export type ApprovedCommands = { 
/**
 * What was approved.
 */
commands: CommandSet, 
/**
 * Its digest, as `project.approve` was given it.
 */
digest: CommandsDigest, 
/**
 * When (the newest approval of it).
 */
approved_at: string, };

/**
 * Shared fields for a query at a saved source position.
 */
export type AtPosition = { 
/**
 * Authorized server selection.
 */
workspace: Workspace, 
/**
 * Relative saved file.
 */
file: FilePath, 
/**
 * One-based Unicode scalar position.
 */
position: Position, 
/**
 * Sorted result page (unused by hover).
 */
page: PageRequest, };

/**
 * A file sent with a prompt, kept with its session (protocol 10).
 */
export type AttachmentId = string & { readonly __brand: "AttachmentId" };

/**
 * What an attachment is to the agent.
 */
export type AttachmentKind = "image" | "audio" | "file";

/**
 * How large attachments may be, as a node enforces it; a client reads
 * them (`session.conversation`) before it uploads.
 */
export type AttachmentLimits = { 
/**
 * Largest image, in bytes.
 */
image: number, 
/**
 * Largest audio file, in bytes.
 */
audio: number, 
/**
 * Largest other file, in bytes.
 */
file: number, 
/**
 * Most attachments one prompt names.
 */
per_prompt: number, 
/**
 * Most bytes of images and audio one prompt carries to the agent.
 */
inline_per_prompt: number, 
/**
 * Most bytes of attachments one session keeps.
 */
per_session: number, 
/**
 * Largest piece `attachment.write` takes, in bytes.
 */
chunk: number, };

/**
 * An attachment's file name as the person's file had it. The node
 * stores the bytes under it (in a folder of the attachment's own id),
 * so it is one path component on every platform.
 */
export type AttachmentName = string;

/**
 * An attachment a prompt names, as `attachment.finish` returned it.
 */
export type AttachmentRef = { 
/**
 * Its id within its session.
 */
id: AttachmentId, 
/**
 * Its file name.
 */
name: AttachmentName, 
/**
 * What it is to the agent.
 */
kind: AttachmentKind, 
/**
 * Its type.
 */
mime: MimeType, 
/**
 * Its size in bytes.
 */
size: number, };

/**
 * A sign-in method an agent offers in its handshake (ACP `authMethods`, of
 * the `agent` type: Vessel advertises no other, so it is offered no
 * other). The node keeps at most [`AuthMethod::MAX`], each cut.
 */
export type AuthMethod = { 
/**
 * Its id, as the agent gave it.
 */
id: AuthMethodId, 
/**
 * Its name, in the agent's words (cut at [`AuthMethod::MAX_NAME`]
 * characters).
 */
name: string, 
/**
 * What it does, in the agent's words (cut at
 * [`AuthMethod::MAX_DESCRIPTION`] characters).
 */
description?: string | null, };

/**
 * The id of a sign-in method an agent offers (ACP `authMethods[].id`),
 * echoed back as given.
 */
export type AuthMethodId = string & { readonly __brand: "AuthMethodId" };

/**
 * `attachment.begin`: declare a file to upload to a live structured
 * session. Needs `operate`.
 */
export type BeginUpload = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * The file's name.
 */
name: AttachmentName, 
/**
 * Its type.
 */
mime: MimeType, 
/**
 * Its size in bytes.
 */
size: number, };

/**
 * Reply to `fleet.blob`.
 */
export type BlobChunk = { 
/**
 * Bytes from the requested offset: at most `MAX_LEN`, fewer at the end.
 */
data: string, 
/**
 * The blob's whole size, so the member knows when it is done.
 */
total: number, };

/**
 * The SHA-256 of a blob, lowercase hex on the wire. Blobs are fetched and
 * stored by it, so equal content is fetched once.
 */
export type BlobHash = string & { readonly __brand: "BlobHash" };

/**
 * Exact external source that caused automatic setup/naming to be withheld.
 */
export type BlockedSource = { "kind": "change_request", 
/**
 * Forge repository includes its validated host/kind/path, not a free URL.
 */
head_repository: RepoRef, 
/**
 * Stable request number, positive and exactly representable on the wire.
 */
number: PullNumber, };

/**
 * A branch.
 */
export type Branch = { 
/**
 * Its name: `main`, or `origin/main` for a remote-tracking one.
 */
name: RefName, 
/**
 * The remote, for a remote-tracking branch.
 */
remote?: RemoteName | null, 
/**
 * Whether it is checked out in this root.
 */
current: boolean, 
/**
 * Whether it is the project's default branch.
 */
default: boolean, 
/**
 * Its commit.
 */
head: CommitId, 
/**
 * When that commit was made.
 */
time: string, 
/**
 * Against its upstream, for a local branch that has one.
 */
upstream?: Distance | null, 
/**
 * The folder that has it checked out, when one does.
 */
worktree?: string | null, };

/**
 * A root's branches.
 */
export type BranchList = { 
/**
 * Local branches first, then remote-tracking ones, each by name; at
 * most 1,000.
 */
branches: Array<Branch>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * A git branch name (`refs/heads/<name>`).
 */
export type BranchName = string;

/**
 * A file in a bundle.
 */
export type BundleFile = { 
/**
 * Its content's hash.
 */
hash: BlobHash, 
/**
 * Its size in bytes.
 */
size: number, 
/**
 * Whether it is executable (scripts in skills). Ignored on Windows.
 * There is no other mode: members always write fleet files owner-only
 * (0600, 0700 if executable), so no manifest can widen access.
 */
executable?: boolean, };

/**
 * A file's place in a bundle: relative, `/`-separated, and valid as a file
 * name on every OS Vessel runs on, so joining it under a target directory
 * can never leave that directory.
 */
export type BundlePath = string;

/**
 * When a schedule is due.
 */
export type Cadence = { "kind": "every", 
/**
 * The interval.
 */
seconds: EverySeconds, 
/**
 * Where the count starts; the moment the cadence was set when
 * left out.
 */
anchor?: string | null, } | { "kind": "cron", 
/**
 * The times.
 */
expr: CronExpr, 
/**
 * The zone they are read in.
 */
zone: TimeZoneName, } | { "kind": "unknown" };

/**
 * A request of any method, as sent on the wire.
 */
export type Call = { "method": "node.info", "params": GetNode } | { "method": "harness.list", "params": ListHarnesses } | { "method": "session.list", "params": ListSessions } | { "method": "session.get", "params": GetSession } | { "method": "session.create", "params": CreateSession } | { "method": "session.side_open", "params": OpenSideQuestion } | { "method": "session.side_prompt", "params": PromptSideQuestion } | { "method": "session.side_reset", "params": ResetSideQuestion } | { "method": "session.send", "params": SendInput } | { "method": "session.kill", "params": KillSession } | { "method": "session.read", "params": ReadEvents } | { "method": "session.subscribe", "params": Subscribe } | { "method": "session.unsubscribe", "params": Unsubscribe } | { "method": "node.stop", "params": StopNode } | { "method": "device.invite", "params": InviteDevice } | { "method": "device.self", "params": GetDeviceSelf } | { "method": "device.list", "params": ListDevices } | { "method": "device.revoke", "params": RevokeDevice } | { "method": "profile.list", "params": ListProfiles } | { "method": "profile.reload", "params": ReloadProfiles } | { "method": "session.move_profile", "params": MoveProfile } | { "method": "fleet.manifest", "params": GetManifest } | { "method": "fleet.blob", "params": ReadBlob } | { "method": "fleet.watch", "params": WatchFleet } | { "method": "fleet.report", "params": ReportApplied } | { "method": "fleet.join", "params": JoinFleet } | { "method": "fleet.leave", "params": LeaveFleet } | { "method": "fleet.sync", "params": SyncFleet } | { "method": "fleet.status", "params": GetFleetStatus } | { "method": "fleet.published", "params": GetPublished } | { "method": "fleet.members", "params": ListMembers } | { "method": "fleet.forward", "params": Forward } | { "method": "fleet.relay", "params": RelayCall } | { "method": "fleet.deliver", "params": Deliver } | { "method": "fleet.grant", "params": GrantHome } | { "method": "fleet.callback", "params": GetCallback } | { "method": "fleet.peers", "params": ListPeers } | { "method": "fleet.nodes", "params": ListNodes } | { "method": "fleet.forget", "params": ForgetNode } | { "method": "session.export", "params": ExportSession } | { "method": "session.import", "params": ImportSession } | { "method": "memory.export", "params": ExportMemory } | { "method": "memory.import", "params": ImportMemory } | { "method": "transfer.read", "params": ReadTransfer } | { "method": "transfer.close", "params": CloseTransfer } | { "method": "secret.set", "params": SetSecret } | { "method": "secret.list", "params": ListSecrets } | { "method": "secret.delete", "params": DeleteSecret } | { "method": "secret.copy", "params": CopySecret } | { "method": "lsp.workspace_symbols", "params": WorkspaceSymbols } | { "method": "lsp.document_symbols", "params": DocumentSymbols } | { "method": "lsp.definition", "params": Definition } | { "method": "lsp.references", "params": References } | { "method": "lsp.hover", "params": GetHover } | { "method": "lsp.implementations", "params": Implementations } | { "method": "lsp.type_definition", "params": TypeDefinition } | { "method": "lsp.incoming_calls", "params": IncomingCalls } | { "method": "lsp.outgoing_calls", "params": OutgoingCalls } | { "method": "lsp.diagnostics", "params": Diagnostics } | { "method": "model.list", "params": ListModels } | { "method": "harness.options", "params": GetHarnessOptions } | { "method": "session.options", "params": GetSessionOptions } | { "method": "session.set_option", "params": SetSessionOption } | { "method": "session.rename", "params": RenameSession } | { "method": "session.prompt", "params": PromptSession } | { "method": "session.queue", "params": ChangeQueue } | { "method": "session.conversation", "params": GetConversation } | { "method": "session.resume", "params": ResumeSession } | { "method": "session.mark", "params": MarkSession } | { "method": "session.delete", "params": DeleteSession } | { "method": "attachment.begin", "params": BeginUpload } | { "method": "attachment.write", "params": WriteUpload } | { "method": "attachment.finish", "params": FinishUpload } | { "method": "attachment.read", "params": ReadAttachment } | { "method": "project.list", "params": ListProjects } | { "method": "project.get", "params": GetProject } | { "method": "project.add", "params": AddProject } | { "method": "project.clone", "params": CloneProject } | { "method": "project.init", "params": InitProject } | { "method": "project.remove", "params": RemoveProject } | { "method": "project.commands", "params": GetProjectCommands } | { "method": "project.approve", "params": ApproveCommands } | { "method": "worktree.list", "params": ListWorktrees } | { "method": "worktree.create", "params": CreateWorktree } | { "method": "worktree.output", "params": ReadSetupOutput } | { "method": "worktree.setup", "params": RunSetup } | { "method": "worktree.remove", "params": RemoveWorktree } | { "method": "worktree.snapshots", "params": ListSnapshots } | { "method": "worktree.drop_snapshot", "params": DropSnapshot } | { "method": "git.status", "params": GetStatus } | { "method": "git.changes", "params": GetChanges } | { "method": "git.diff", "params": GetFileDiff } | { "method": "git.show", "params": ShowFile } | { "method": "git.branches", "params": ListBranches } | { "method": "git.log", "params": ReadLog } | { "method": "git.stashes", "params": ListStashes } | { "method": "git.stage", "params": Stage } | { "method": "git.unstage", "params": Unstage } | { "method": "git.discard", "params": Discard } | { "method": "git.commit", "params": Commit } | { "method": "git.fetch", "params": Fetch } | { "method": "git.push", "params": Push } | { "method": "git.pull", "params": Pull } | { "method": "git.switch", "params": Switch } | { "method": "git.rename_branch", "params": RenameBranch } | { "method": "git.stash", "params": StashChanges } | { "method": "file.list", "params": ListDir } | { "method": "file.read", "params": ReadFile } | { "method": "file.write", "params": WriteFile } | { "method": "file.create_dir", "params": CreateDir } | { "method": "file.rename", "params": RenamePath } | { "method": "file.delete", "params": DeletePath } | { "method": "file.find", "params": FindFiles } | { "method": "file.grep", "params": SearchFiles } | { "method": "forge.status", "params": GetForgeStatus } | { "method": "pr.list", "params": ListPullRequests } | { "method": "pr.get", "params": GetPullRequest } | { "method": "pr.open", "params": OpenPullRequest } | { "method": "pr.merge", "params": MergePullRequest } | { "method": "pr.feedback", "params": SetFeedback } | { "method": "terminal.create", "params": CreateTerminal } | { "method": "terminal.list", "params": ListTerminals } | { "method": "terminal.get", "params": GetTerminal } | { "method": "terminal.read", "params": ReadTerminal } | { "method": "terminal.subscribe", "params": SubscribeTerminal } | { "method": "terminal.send", "params": SendTerminal } | { "method": "terminal.rename", "params": RenameTerminal } | { "method": "terminal.kill", "params": KillTerminal } | { "method": "terminal.close", "params": CloseTerminal } | { "method": "script.list", "params": ListScripts } | { "method": "script.run", "params": RunScript } | { "method": "script.restart", "params": RestartScript } | { "method": "port.list", "params": ListPorts } | { "method": "agent.list", "params": ListAgents } | { "method": "agent.add", "params": AddAgent } | { "method": "agent.remove", "params": RemoveAgent } | { "method": "agent.status", "params": GetAgentStatus } | { "method": "agent.catalogue", "params": GetCatalogue } | { "method": "agent.install", "params": InstallAgent } | { "method": "agent.install_output", "params": ReadInstall } | { "method": "usage.query", "params": QueryUsage } | { "method": "push.key", "params": GetPushKey } | { "method": "push.subscribe", "params": SubscribePush } | { "method": "push.unsubscribe", "params": UnsubscribePush } | { "method": "push.presence", "params": ReportPushPresence } | { "method": "inbox.list", "params": ListInbox } | { "method": "inbox.get", "params": GetInboxEntry } | { "method": "inbox.mark", "params": MarkInbox } | { "method": "usage.limits", "params": GetUsageLimits } | { "method": "usage.lookup", "params": SetUsageLookup } | { "method": "schedule.preview", "params": PreviewSchedule } | { "method": "schedule.list", "params": ListSchedules } | { "method": "schedule.get", "params": GetSchedule } | { "method": "schedule.create", "params": CreateSchedule } | { "method": "schedule.update", "params": UpdateSchedule } | { "method": "schedule.pause", "params": PauseSchedule } | { "method": "schedule.resume", "params": ResumeSchedule } | { "method": "schedule.run", "params": RunSchedule } | { "method": "schedule.delete", "params": DeleteSchedule } | { "method": "schedule.runs", "params": ListRuns } | { "method": "prompt.list", "params": ListPrompts } | { "method": "prompt.create", "params": CreatePrompt } | { "method": "prompt.update", "params": UpdatePrompt } | { "method": "prompt.delete", "params": DeletePrompt } | { "method": "session.search", "params": SearchMessages } | { "method": "session.park", "params": ParkSession } | { "method": "session.wake", "params": WakeSession } | { "method": "node.settings", "params": GetNodeSettings } | { "method": "node.set_settings", "params": SetNodeSettings } | { "method": "session.fork", "params": ForkSession } | { "method": "session.rewind", "params": RewindSession } | { "method": "agent.history", "params": GetAgentHistory } | { "method": "session.detach", "params": DetachSession } | { "method": "workspace.registry", "params": GetRegistry } | { "method": "workspace.list", "params": ListWorkspaces } | { "method": "workspace.get", "params": GetWorkspace } | { "method": "workspace.rename", "params": RenameWorkspace } | { "method": "workspace.pin", "params": PinWorkspace } | { "method": "workspace.release_automation", "params": ReleaseWorkspaceAutomation } | { "method": "git.compare", "params": CompareGit } | { "method": "git.compare_file", "params": CompareGitFile } | { "method": "node.diagnostics", "params": GetDiagnostics } | { "method": "file.inspect", "params": InspectDirectory } | { "method": "file.read_bytes", "params": ReadFileBytes } | { "method": "file.duplicate", "params": DuplicateFileEntry } | { "method": "file.observe", "params": ObserveFile } | { "method": "file.suggest", "params": SuggestEntries } | { "method": "skills.status", "params": GetSkillsStatus } | { "method": "skills.reconcile", "params": ReconcileSkills } | { "method": "skills.uninstall", "params": UninstallSkills } | { "method": "skills.save_selection", "params": SaveSkillsSelection } | { "method": "skills.import_selection", "params": ImportSkillsSelection };

/**
 * What `fleet.callback` answers.
 */
export type Callback = { 
/**
 * The relays the home registers with, in its preference order; none
 * when the home has no relay (it is reached directly), and then a
 * member needs a route of its own to be called back.
 */
relays: Array<RouteUrl>, 
/**
 * A ticket for the member's key, with `relays`.
 */
ticket?: RelayTicket | null, };

/**
 * The node's copy of the registry.
 */
export type Catalogue = { 
/**
 * When it was fetched; absent before the first fetch since the node
 * started.
 */
fetched_at?: string | null, 
/**
 * The registry's own version.
 */
version?: RegistryVersion | null, 
/**
 * Its entries Vessel reads, in the registry's order.
 */
entries: Array<CatalogueEntry>, 
/**
 * Entries left out: unreadable, past a bound, or pinning no exact
 * version.
 */
skipped: number, };

/**
 * One registry entry.
 */
export type CatalogueEntry = { 
/**
 * Its id.
 */
id: RegistryId, 
/**
 * Its name.
 */
name: string, 
/**
 * The version the registry pins.
 */
version: RegistryVersion, 
/**
 * What it says of itself.
 */
description: string, 
/**
 * Its site.
 */
website?: HttpUrl | null, 
/**
 * Its source.
 */
repository?: HttpUrl | null, 
/**
 * Its licence, as the registry says.
 */
license?: string | null, 
/**
 * What would run on this node.
 */
runs: CatalogueRun, };

/**
 * What a registry entry would run on this node.
 */
export type CatalogueRun = { "kind": "known", 
/**
 * Which.
 */
harness: HarnessKind, } | { "kind": "command", 
/**
 * The runner.
 */
program: AgentProgram, 
/**
 * Its arguments, the pinned package among them.
 */
args: Array<string>, 
/**
 * The variables the entry sets, as text.
 */
env: Array<CatalogueVariable>, 
/**
 * Whether the runner is on the node's search path.
 */
runner_found: boolean, } | { "kind": "download", 
/**
 * The archive.
 */
archive: HttpUrl, 
/**
 * Its SHA-256, when the registry gives one.
 */
sha256?: Sha256Hex | null, } | { "kind": "unavailable" };

/**
 * A variable a registry entry sets.
 */
export type CatalogueVariable = { 
/**
 * Its name.
 */
name: EnvName, 
/**
 * Its value, as the registry gives it.
 */
value: string, };

/**
 * What a schedule does about due times it missed (the node stopped, the
 * machine slept, the clock jumped).
 */
export type CatchUp = "once" | "skip" | "unknown";

/**
 * `session.queue`: change a session's queue. Needs `operate`.
 */
export type ChangeQueue = { 
/**
 * Which session.
 */
session: SessionId, } & ({ "change": "edit", 
/**
 * Which entry.
 */
entry: QueueEntryId, 
/**
 * The revision the edit was made from; another is `conflict`.
 */
revision: number, 
/**
 * Its new content.
 */
content: Array<ContentBlock>, } | { "change": "move", 
/**
 * Which entry.
 */
entry: QueueEntryId, 
/**
 * The entry it goes before; absent is the end.
 */
before?: QueueEntryId | null, } | { "change": "remove", 
/**
 * Which entry.
 */
entry: QueueEntryId, } | { "change": "send_now", 
/**
 * Which entry.
 */
entry: QueueEntryId, 
/**
 * How: `start`, `steer` or `interrupt` (not `queue`).
 */
delivery: Delivery, } | { "change": "hold" } | { "change": "resume" } | { "change": "clear" });

/**
 * How a file changed.
 */
export type ChangeStatus = "added" | "modified" | "deleted" | "renamed" | "copied" | "type_changed" | "conflicted" | "unknown";

/**
 * A check's or a status context's name.
 */
export type CheckName = string;

/**
 * One check, as a card names it.
 */
export type CheckRun = { 
/**
 * Its name.
 */
name: CheckName, 
/**
 * Its result.
 */
state: CheckState, 
/**
 * Its page on the forge, when it gave an `https` one.
 */
url?: ForgeUrl | null, };

/**
 * One check's result (agent-orchestrator's table).
 */
export type CheckState = "pending" | "passed" | "failed" | "cancelled" | "skipped" | "unknown";

/**
 * Where a check failed.
 */
export type CheckStep = "spawn" | "initialize" | "session_new" | "exited";

/**
 * A head commit's checks.
 */
export type Checks = { 
/**
 * All of them together.
 */
state: ChecksState, 
/**
 * How many passed (neutral and skipped included).
 */
passed: number, 
/**
 * How many failed.
 */
failed: number, 
/**
 * How many run or wait.
 */
pending: number, 
/**
 * More were reported than one read takes (100).
 */
truncated: boolean, 
/**
 * The failing ones, at most [`Checks::MAX_FAILING`].
 */
failing: Array<CheckRun>, };

/**
 * All of a head commit's checks together.
 */
export type ChecksState = "none" | "pending" | "passing" | "failing" | "cancelled" | "unknown";

/**
 * One option of a choice.
 */
export type ChoiceOption = { 
/**
 * What is sent back when it is chosen.
 */
value: string, 
/**
 * Its label.
 */
title: string, 
/**
 * A longer explanation.
 */
description?: string | null, };

/**
 * Why a clone failed.
 */
export type CloneFailure = "needs_credential" | "not_found" | "network" | "timed_out" | "other" | "unknown";

/**
 * `project.clone`: a repository into a new folder.
 */
export type CloneProject = { 
/**
 * What to clone.
 */
url: CloneUrl, 
/**
 * The existing folder, absolute, to clone into.
 */
parent: string, 
/**
 * The new folder in it; the repository's name otherwise.
 */
folder?: FolderName | null, 
/**
 * What to call the project; the folder's name otherwise.
 */
name?: ProjectName | null, };

/**
 * A clone's stage, from git's progress lines.
 */
export type CloneStage = "connecting" | "counting" | "compressing" | "receiving" | "resolving" | "checking_out" | "unknown";

/**
 * A repository URL `git clone` may fetch.
 */
export type CloneUrl = string;

/**
 * `terminal.close`: end it if it runs, and forget it.
 */
export type CloseTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, };

/**
 * `transfer.close` (protocol 6): drop the snapshot. Only the caller that
 * opened it may close it. Needs `read`.
 */
export type CloseTransfer = { 
/**
 * Which transfer.
 */
transfer: TransferId, };

/**
 * A provider's explicit distinction decides whether a command is useful inline.
 */
export type CommandKind = "command" | "skill" | "unknown";

/**
 * A command the agent offers, without its `/` (`compact`, `review`).
 * Sent back as the start of a prompt, `/name`, so it has no white
 * space: what follows the first space is the command's input.
 */
export type CommandName = string;

/**
 * What a project's file names to run: setup and teardown, and its
 * scripts and services (protocol 11). Bounded by the file it was parsed
 * from: at most 64 KiB, 32 commands of each kind, 32 scripts.
 */
export type CommandSet = { 
/**
 * Run when a worktree is created.
 */
setup: Array<ProjectCommand>, 
/**
 * Run before a worktree is removed.
 */
teardown: Array<ProjectCommand>, 
/**
 * Its scripts and services, in name order; left out when none.
 */
scripts?: Array<ScriptEntry>, 
/**
 * Terminal commands covered by this approval (ADR 0066).
 */
terminals?: Array<FileTerminal>, 
/**
 * Retain every allocation choice covered by the approval's digest.
 */
service_ports?: ServicePorts | null, };

/**
 * SHA-256 of a project's setup and teardown commands, hexadecimal: what
 * a person approves (ADR 0027).
 */
export type CommandsDigest = string & { readonly __brand: "CommandsDigest" };

/**
 * `git.commit`.
 */
export type Commit = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * The message.
 */
message: CommitMessage, 
/**
 * What it takes.
 */
stage: CommitStage, };

/**
 * A commit's id, in full or abbreviated, lower case.
 */
export type CommitId = string;

/**
 * A commit.
 */
export type CommitInfo = { 
/**
 * Its id.
 */
id: CommitId, 
/**
 * Its parents' ids.
 */
parents: Array<CommitId>, 
/**
 * Its author's name.
 */
author: string, 
/**
 * When it was authored.
 */
time: string, 
/**
 * Its message's first line.
 */
subject: string, };

/**
 * A commit's message.
 */
export type CommitMessage = string;

/**
 * `git.commit`'s answer.
 */
export type CommitOutcome = { "outcome": "committed", 
/**
 * The new commit.
 */
commit: CommitInfo, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * A page of history.
 */
export type CommitPage = { 
/**
 * Newest first.
 */
commits: Array<CommitInfo>, 
/**
 * Whether older ones follow.
 */
more: boolean, };

/**
 * What a commit takes.
 */
export type CommitStage = "staged" | "tracked" | "everything";

/**
 * Pending `git.compare`: no arbitrary directory or command text is accepted.
 */
export type CompareGit = { 
/**
 * Re-resolved and authorized by the node, including on a cache hit.
 */
root: WorkspaceRoot, 
/**
 * Exactly the selected comparison, never an approximate legacy scope.
 */
comparison: GitComparison, 
/**
 * Included in the cache key so filtered output cannot poison other views.
 */
whitespace: GitWhitespace, 
/**
 * Bypassing freshness requires an explicit reason.
 */
reason?: GitReadReason, };

/**
 * Pending `git.compare_file`: one bounded file body in the same comparison.
 */
export type CompareGitFile = { 
/**
 * Repository/comparison/whitespace identity retained across pages.
 */
query: CompareGit, 
/**
 * A validated path relative to the root.
 */
path: RelPath, 
/**
 * Hunk offset, not a byte offset or an array index in the client.
 */
from_hunk: GitHunkOffset, };

/**
 * A key of a repository's git configuration (`core.sshcommand`), named
 * when a run is refused for it (protocol 11, ADR 0030); never its
 * value.
 */
export type ConfigKey = string;

/**
 * A value in an MCP server definition: literal text, or the name of a
 * secret each node resolves from its own store. On the wire, a string or
 * `{"secret": "NAME"}`.
 */
export type ConfigValue = string | { secret: SecretName };

/**
 * A piece of message content.
 */
export type ContentBlock = { "type": "text", 
/**
 * The text.
 */
text: string, } | { "type": "resource_link", 
/**
 * URI of the resource.
 */
uri: string, 
/**
 * Display name.
 */
name: string, } | { "type": "opaque", 
/**
 * The block as the transport sent it.
 */
raw: RawJson, } | { "type": "attachment", 
/**
 * Which, and what it is.
 */
attachment: AttachmentRef, };

/**
 * SHA-256 of a file's content, hexadecimal.
 */
export type ContentHash = string & { readonly __brand: "ContentHash" };

/**
 * Scope and age of every answer, including empty answers.
 */
export type Context = { 
/**
 * Files represented by this result/cache, never a promise of all files.
 */
files: Array<FilePath>, 
/**
 * Query observation time, or the oldest contributing cached report;
 * absent when nothing was reported.
 */
observed_at: string | null, 
/**
 * Progress actually observed on this server generation.
 */
indexing: Indexing, 
/**
 * Saved query, cached report, or unknown.
 */
freshness: Freshness, 
/**
 * True only for a server report explicitly covering the whole workspace.
 */
workspace_complete: boolean, };

/**
 * How full the session's context is, from the agent's last
 * `usage_update`.
 */
export type ContextUsage = { 
/**
 * Whether the runtime observed this while loading an existing
 * conversation (protocol 21). Context occupancy remains useful;
 * a replayed cumulative cost is not new spending.
 */
observation?: UsageObservation, 
/**
 * Tokens in the context now.
 */
used: TokenCount, 
/**
 * The context window's size.
 */
size: TokenCount, 
/**
 * What the session has cost so far, when the agent says.
 */
cost?: Cost | null, };

/**
 * A session's conversation as it stands (`session.conversation`): what
 * its log says now, without reading the log.
 */
export type Conversation = { 
/**
 * The prompts waiting.
 */
queue: PromptQueue, 
/**
 * The commands the agent offers now (none for an ended session).
 */
commands: Array<AgentCommand>, 
/**
 * The agent's last report of its context.
 */
usage?: ContextUsage | null, 
/**
 * Permission requests waiting for an answer.
 */
permissions: Array<PermissionRequest>, 
/**
 * Questions waiting for an answer.
 */
questions: Array<Question>, 
/**
 * What the agent can do, once its handshake said.
 */
abilities?: AgentAbilities | null, 
/**
 * How large attachments may be.
 */
limits: AttachmentLimits, };

/**
 * `secret.copy` (protocol 6): send this node's value for `name` to fleet
 * node `to`, which stores it under the same name. Needs `admin` here, and
 * the delivering grant on `to` must hold `admin` too.
 */
export type CopySecret = { 
/**
 * Which secret.
 */
name: SecretName, 
/**
 * The node to copy it to.
 */
to: NodeId, };

/**
 * A cost, as the agent counts it.
 */
export type Cost = { 
/**
 * How much.
 */
amount: FiniteNumber, 
/**
 * In what.
 */
unit: CostUnit, };

/**
 * What a cost is counted in: an ISO 4217 code (`USD`) or the agent's
 * own unit (`credits`).
 */
export type CostUnit = string;

/**
 * Cost absence and scope redaction are different user-visible states.
 */
export type CostVisibility = "permitted" | "withheld";

/**
 * `file.create_dir`.
 */
export type CreateDir = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * The new folder (its parent must exist).
 */
path: RelPath, };

/**
 * `prompt.create`: save a prompt. Needs `operate`; agents are refused.
 */
export type CreatePrompt = { 
/**
 * Its name.
 */
name: PromptName, 
/**
 * The text.
 */
text: PromptText, 
/**
 * Its project; absent: the machine's.
 */
project?: ProjectId | null, 
/**
 * What a session started from it runs with.
 */
session?: SessionSpec | null, };

/**
 * `schedule.create`: make a schedule. Needs `operate`.
 */
export type CreateSchedule = { 
/**
 * What it does.
 */
definition: ScheduleDefinition, 
/**
 * Paused from the start.
 */
paused: boolean, };

/**
 * `session.create`: start a harness.
 */
export type CreateSession = { 
/**
 * What to run.
 */
spec: SessionSpec, };

/**
 * `terminal.create`: open the user's shell in a place.
 */
export type CreateTerminal = { 
/**
 * Where it runs.
 */
place: TerminalPlace, 
/**
 * Its window; 80×24 when absent.
 */
size?: TerminalSize | null, 
/**
 * What to call it.
 */
title?: SessionTitle | null, };

/**
 * `worktree.create`: a worktree with no session yet.
 */
export type CreateWorktree = { 
/**
 * Of which project.
 */
project: ProjectId, 
/**
 * On which branch.
 */
branch: WorktreeBranch, 
/**
 * Whether setup runs.
 */
setup: SetupChoice, };

/**
 * `worktree.create`'s answer.
 */
export type CreateWorktreeOutcome = { "outcome": "created", 
/**
 * It.
 */
worktree: WorktreeInfo, } | { "outcome": "needs_approval", 
/**
 * What a person would approve.
 */
commands: ProjectCommands, } | { "outcome": "refused", 
/**
 * Why.
 */
reason: WorktreeRefusal, };

/**
 * Who created a session (ADR 0013): the owner, a paired device, or an agent
 * running in another session.
 */
export type Creator = { "kind": "owner" } | { "kind": "device", 
/**
 * The device.
 */
id: DeviceId, } | { "kind": "agent", 
/**
 * The node the agent's session runs on.
 */
node: NodeId, 
/**
 * The agent's session.
 */
session: SessionId, };

/**
 * A schedule's cron expression: five fields, minute, hour, day of the
 * month, month and day of the week. Kept as written, with runs of white
 * space made one.
 */
export type CronExpr = string;

/**
 * A custom agent's slot in this report, never its person-chosen name.
 */
export type CustomAgentSlot = number;

/**
 * Definitions at a saved source position.
 */
export type Definition = AtPosition;

/**
 * `file.delete`.
 */
export type DeletePath = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * What.
 */
path: RelPath, 
/**
 * A folder with everything in it; only an empty one otherwise.
 */
recursive: boolean, };

/**
 * `prompt.delete`: remove a saved prompt. Needs `operate`; agents are
 * refused.
 */
export type DeletePrompt = { 
/**
 * Which.
 */
prompt: SavedPromptId, };

/**
 * `schedule.delete`: remove a schedule and its runs. Needs `operate`.
 */
export type DeleteSchedule = { 
/**
 * Which.
 */
schedule: ScheduleId, };

/**
 * `secret.delete` (protocol 6): remove a secret. Needs `admin`.
 */
export type DeleteSecret = { 
/**
 * Which secret.
 */
name: SecretName, };

/**
 * `session.delete`: remove an ended session, its log, its marks, its
 * queue and its attachments. Needs `operate`.
 */
export type DeleteSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * `fleet.deliver` (protocol 6): the home asks a member to run a call for
 * `origin`. The last hop: a member never passes a delivered call on, and
 * accepts one only from its home's device (the one `fleet.grant` made).
 * Needs `read`; the inner call runs with the delivering device's grant
 * intersected with `origin.scopes`.
 */
export type Deliver = { 
/**
 * The call.
 */
call: Call, 
/**
 * Who it speaks for.
 */
origin: Origin, };

/**
 * What became of a delivered prompt.
 */
export type Delivered = { "delivered": "started" } | { "delivered": "steered" } | { "delivered": "queued", 
/**
 * Its entry.
 */
entry: QueueEntryId, };

/**
 * How a prompt goes to a session (`session.prompt`).
 */
export type Delivery = "start" | "queue" | "steer" | "interrupt";

/**
 * A retry distinguishes a committed detach from a new mutation.
 */
export type DetachDisposition = "detached" | "already_detached";

/**
 * Stored once per managed child, surviving lost replies, resume and restart.
 */
export type DetachReceipt = { former_parent: SessionRef, detached_at: string, };

/**
 * Planned session.detach input; only the target id comes from the caller.
 * Method registration awaits the central pending-method contract.
 */
export type DetachSession = { 
/**
 * The child on the receiving node; parent and workspace are authoritative.
 */
session: SessionId, };

/**
 * Planned session.detach result; session carries the unchanged workspace.
 */
export type DetachedSession = { 
/**
 * Current authoritative session with identical process/history identity.
 */
session: SessionInfo, 
/**
 * Original typed parent and commit time, including on a repeat.
 */
receipt: DetachReceipt, 
/**
 * Whether this request changed current delegation.
 */
disposition: DetachDisposition, };

/**
 * A paired device: one client machine holding its own credential for a
 * node (ADR 0010). Minted by the node at pairing.
 */
export type DeviceId = string & { readonly __brand: "DeviceId" };

/**
 * A paired device as the node reports it.
 */
export type DeviceInfo = { 
/**
 * Its id.
 */
id: DeviceId, 
/**
 * The name given at pairing (the client machine's name by default).
 */
label: string, 
/**
 * When it was paired.
 */
created_at: string, 
/**
 * When it last authenticated.
 */
last_seen?: string | null, 
/**
 * What it may do.
 */
scopes: Array<Scope>, 
/**
 * When it was revoked, if it was.
 */
revoked_at?: string | null, };

/**
 * Why a pairing invitation could not be issued (protocol 20, ADR 0044).
 */
export type DeviceInviteError = "no_routes" | "unknown";

/**
 * This connection's effective grant, not a promise of resource ownership.
 * Unavailable queries remain unknown to a client; there is no default grant.
 */
export type DeviceSelf = { 
/**
 * Only the directly paired caller's id. Owner, agent and delegated
 * callers have none; the delivering device is never disclosed.
 */
device?: DeviceId | null, 
/**
 * Canonical, duplicate-free scopes used by this call's guard. Agent
 * limits, ownership and profile policy still apply to later operations.
 */
scopes: Array<Scope>, };

/**
 * A bounded diagnostic with only authorized source locations.
 */
export type Diagnostic = { 
/**
 * Diagnostic location.
 */
location: Location, 
/**
 * Server severity.
 */
severity: Severity, 
/**
 * Bounded explanatory text.
 */
message: string, 
/**
 * Optional bounded server/source label.
 */
source: string | null, };

/**
 * One agent with no path, arbitrary name or output field.
 */
export type DiagnosticAgent = { 
/**
 * Safe identity.
 */
id: DiagnosticAgentId, 
/**
 * Metadata-only executable observation.
 */
executable: DiagnosticPresence, 
/**
 * Conversation availability.
 */
structured: DiagnosticAvailability, 
/**
 * Terminal availability.
 */
terminal: DiagnosticAvailability, 
/**
 * Cached sign-in of this agent's default profile only.
 */
sign_in: DiagnosticSignIn, 
/**
 * Numeric cached version; free-form output is absent.
 */
version: DiagnosticVersion | null, 
/**
 * When a cached protocol observation was made, if one exists.
 */
checked_at: string | null, 
/**
 * Whether the cached observation has expired.
 */
stale: boolean, };

/**
 * A safe agent identifier in this report.
 */
export type DiagnosticAgentId = { "kind": "builtin", 
/**
 * The builtin kind.
 */
harness: HarnessKind, } | { "kind": "custom", 
/**
 * Ordinal valid only for this report.
 */
slot: CustomAgentSlot, };

/**
 * A capability refusal has no executable/path/output text.
 */
export type DiagnosticAgentRefusal = { "reason": "missing_executable" } | { "reason": "missing_adapter" } | { "reason": "too_old", 
/**
 * Reported numeric protocol.
 */
speaks: AcpProtocolVersion, 
/**
 * Required numeric protocol.
 */
needs: AcpProtocolVersion, } | { "reason": "too_new", 
/**
 * Reported numeric protocol.
 */
speaks: AcpProtocolVersion, 
/**
 * Required numeric protocol.
 */
needs: AcpProtocolVersion, } | { "reason": "unsupported" };

/**
 *A complete, bounded inventory with distinct safe agent identities.
 */
export type DiagnosticAgents = Array<DiagnosticAgent>;

/**
 * CPU architecture classes from the build target.
 */
export type DiagnosticArch = "x86_64" | "aarch64" | "x86" | "arm" | "other";

/**
 * Known availability or the exact safe reason it cannot be asserted.
 */
export type DiagnosticAvailability = { "state": "available" } | { "state": "unavailable", 
/**
 * Never a raw reason string.
 */
reason: DiagnosticAgentRefusal, } | { "state": "unknown", 
/**
 * No refresh/probe is performed to fill this fact.
 */
reason: UnknownReason, };

/**
 * Observed binding state, never a claim of outside reachability.
 */
export type DiagnosticBinding = "disabled" | "starting" | "bound";

/**
 * A count whose arithmetic never wraps or silently loses overflow.
 */
export type DiagnosticCount = { value: number, saturated: boolean, };

/**
 * Explicit error observation coverage; never "all node errors".
 */
export type DiagnosticErrorCoverage = "local_rpc_completions";

/**
 * One observed failed local RPC completion; no request/method/message data.
 */
export type DiagnosticFailure = { 
/**
 * Existing closed error class.
 */
code: ErrorCode, 
/**
 * When the failed completion was observed.
 */
at: string, };

/**
 * A bounded failure window with explicit observation/loss coverage.
 */
export type DiagnosticFailureWindow = { 
/**
 * What is and is not observed.
 */
coverage: DiagnosticErrorCoverage, 
/**
 * A new node instance starts a new window; history is not persisted.
 */
observed_since: string, 
/**
 * Retained failures from the collector's rolling time window.
 */
entries: RecentFailures, 
/**
 * Observations lost/evicted since this node instance started.
 */
omitted: DiagnosticCount, };

/**
 * The home is represented by a fixed marker, never by its path.
 */
export type DiagnosticHome = "redacted";

/**
 * Listener metadata never contains a route/address.
 */
export type DiagnosticListeners = { 
/**
 * Local transport kind.
 */
local_kind: DiagnosticLocalKind, 
/**
 * Local bind state.
 */
local: DiagnosticBinding, 
/**
 * Direct TCP bind state; not an advertised route.
 */
direct: DiagnosticBinding, };

/**
 * Local transport kinds, without a socket/pipe name.
 */
export type DiagnosticLocalKind = "socket" | "pipe";

/**
 * Memory from one observation. Counts expose JSON-safe saturation explicitly.
 */
export type DiagnosticMemory = { 
/**
 * Resident memory in bytes.
 */
resident_bytes: DiagnosticCount, 
/**
 * Virtual address-space size in bytes.
 */
virtual_bytes: DiagnosticCount, };

/**
 * Public process facts, projected rather than cloning `NodeInfo`.
 */
export type DiagnosticNode = { 
/**
 * Stable machine identity; no key/hostname.
 */
id: NodeId, 
/**
 * Normalized package version or explicit unknown.
 */
version: DiagnosticVersion | null, 
/**
 * Build platform.
 */
os: DiagnosticOs, 
/**
 * Build architecture.
 */
arch: DiagnosticArch, 
/**
 * Node lifecycle, not transport reachability.
 */
state: NodeState, 
/**
 * Elapsed monotonic node-instance time.
 */
uptime: DurationMs, 
/**
 * Fixed privacy marker.
 */
home: DiagnosticHome, };

/**
 * Platform classes contain no operating-system release/environment text.
 */
export type DiagnosticOs = "linux" | "mac_os" | "windows" | "other";

/**
 * Executable presence says nothing about sign-in or successful turns.
 */
export type DiagnosticPresence = "found" | "missing" | "unknown";

/**
 * Only this daemon's process; no remote process-list or signalling handle.
 */
export type DiagnosticProcess = { 
/**
 * Process identity at observation time.
 */
pid: DiagnosticProcessId, 
/**
 * Parent, absent when the operating system supplies no parent.
 */
parent: DiagnosticProcessId | null, 
/**
 * Missing memory is explicit; no zero substitution.
 */
memory: DiagnosticSection<DiagnosticMemory>, };

/**
 * Nonzero process identity, distinct from node and session identities.
 */
export type DiagnosticProcessId = number & { readonly __brand: "DiagnosticProcessId" };

/**
 * Safe typed explanation of resource admission refusal.
 */
export type DiagnosticRefusal = "busy" | "stopping";

/**
 * Counts from relay watches; no route, refusal text or credential.
 */
export type DiagnosticRelayCounts = { 
/**
 * Connecting or registering, including reconnect attempts.
 */
connecting: DiagnosticCount, 
/**
 * Registered and accepting relayed channels.
 */
registered: DiagnosticCount, 
/**
 * Registration refused; normal retry policy is unchanged.
 */
refused: DiagnosticCount, 
/**
 * Link stopped.
 */
stopped: DiagnosticCount, 
/**
 * Future non-exhaustive link states, without invented meaning.
 */
unrecognized: DiagnosticCount, };

/**
 * Configured links and fleet callbacks remain distinct.
 */
export type DiagnosticRelays = { 
/**
 * Links from startup configuration.
 */
configured: DiagnosticRelayCounts, 
/**
 * Current home's callback links.
 */
callback: DiagnosticRelayCounts, };

/**
 * A complete canonical report that fits its byte cap; no mutable access.
 */
export type DiagnosticReport = DiagnosticReportFields;

/**
 * Inputs to the validated report; the public root owns an immutable copy.
 */
export type DiagnosticReportFields = { 
/**
 * When this collection completed.
 */
collected_at: string, 
/**
 * Safe node identity/platform/lifecycle.
 */
node: DiagnosticNode, 
/**
 * Numeric metadata about this daemon process only.
 */
process: DiagnosticSection<DiagnosticProcess>, 
/**
 * Readability/schema and stored session counts, not integrity proof.
 */
store: DiagnosticSection<DiagnosticStore>, 
/**
 * Complete bounded inventory or an explicit reason it is unavailable.
 */
agents: DiagnosticSection<DiagnosticAgents>, 
/**
 * Current runtime counts, separate from durable state.
 */
sessions: DiagnosticSection<DiagnosticRuntimeSessions>, 
/**
 * Bound state without routes.
 */
listeners: DiagnosticSection<DiagnosticListeners>, 
/**
 * Relay watch counts without endpoint data.
 */
relays: DiagnosticSection<DiagnosticRelays>, 
/**
 * Explicitly scoped failed-call observations, not a log tail.
 */
recent_failures: DiagnosticSection<DiagnosticFailureWindow>, };

/**
 * Runtime sessions are distinct from durable state during publication.
 */
export type DiagnosticRuntimeSessions = { 
/**
 * Live session actors, including parked ones.
 */
live: DiagnosticCount, 
/**
 * Live sessions holding no running harness process.
 */
parked: DiagnosticCount, };

/**
 * A schema version, not a wire protocol number.
 */
export type DiagnosticSchemaVersion = number;

/**
 * One source succeeded at a stated time, or failed for a closed reason.
 */
export type DiagnosticSection<T> = { "state": "available", 
/**
 * When these facts were observed.
 */
at: string, 
/**
 * Allowlisted facts.
 */
value: T, } | { "state": "unavailable", 
/**
 * A safe reason, with no raw error.
 */
reason: DiagnosticUnavailable, };

/**
 * Cached sign-in facts for the default profile, without sign-in help.
 */
export type DiagnosticSignIn = { "state": "present" } | { "state": "missing" } | { "state": "not_required" } | { "state": "unknown", 
/**
 * Existing closed readiness reason, never output or a path.
 */
reason: UnknownReason, };

/**
 * Successful bounded store reads do not imply a full integrity check.
 */
export type DiagnosticStore = { 
/**
 * Schema supported by this binary.
 */
expected_schema: DiagnosticSchemaVersion, 
/**
 * Schema observed on the opened connection.
 */
observed_schema: DiagnosticSchemaVersion, 
/**
 * Durable session aggregates; no session identifiers/content.
 */
sessions: DiagnosticStoredSessions, };

/**
 * Stored session aggregates, from the same `SQLite` transaction.
 */
export type DiagnosticStoredSessions = { 
/**
 * All retained sessions.
 */
total: DiagnosticCount, 
/**
 * Sessions still marked live in durable state.
 */
live: DiagnosticCount, 
/**
 * Ended sessions, including failures/interruption.
 */
ended: DiagnosticCount, };

/**
 * Why one section cannot make a claim.
 */
export type DiagnosticUnavailable = "not_checked" | "stale" | "timed_out" | "read_failed" | "too_many_agents" | "stopping" | "unsupported";

/**
 * Numeric version information; arbitrary prerelease/build words are absent.
 */
export type DiagnosticVersion = { 
/**
 * Major version.
 */
major: number, 
/**
 * Minor version.
 */
minor: number, 
/**
 * Patch version.
 */
patch: number, 
/**
 * A prerelease suffix existed; its words were discarded.
 */
prerelease: boolean, 
/**
 * Build metadata existed; its words were discarded.
 */
build_metadata: boolean, };

/**
 * File diagnostics, or the known workspace diagnostic cache.
 */
export type Diagnostics = { 
/**
 * Authorized server selection.
 */
workspace: Workspace, 
/**
 * One file, or all files the server has reported.
 */
file: FilePath | null, 
/**
 * Sorted result page.
 */
page: PageRequest, };

/**
 * What a file's diff has.
 */
export type DiffContent = { "kind": "hunks", 
/**
 * Some or all of them (`FileDiff.more_from`).
 */
hunks: Array<Hunk>, } | { "kind": "binary" } | { "kind": "submodule", 
/**
 * Before.
 */
from?: CommitId | null, 
/**
 * After.
 */
to?: CommitId | null, } | { "kind": "unchanged" } | { "kind": "too_large", 
/**
 * How large git's output for it was, at least.
 */
bytes: number, } | { "kind": "unknown" };

/**
 * One line of a hunk.
 */
export type DiffLine = { 
/**
 * What it is.
 */
kind: LineKind, 
/**
 * Its text without the end of line (not valid UTF-8 replaced), at most
 * 4 KiB.
 */
text: string, 
/**
 * Whether it was longer and cut.
 */
cut?: boolean, };

/**
 * Which changes.
 */
export type DiffScope = { "kind": "unstaged" } | { "kind": "staged" } | { "kind": "uncommitted" } | { "kind": "base", 
/**
 * The base; a worktree's own, else the project's default branch.
 */
base?: WorktreeBase | null, } | { "kind": "commit", 
/**
 * Which.
 */
id: CommitId, };

/**
 * The changed files of a scope.
 */
export type DiffSummary = { 
/**
 * The commit compared against (the merge base, `HEAD`, a parent);
 * absent before the first commit.
 */
from?: CommitId | null, 
/**
 * By path, at most 5,000.
 */
files: Array<FileChange>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * One entry of a folder.
 */
export type DirEntry = { 
/**
 * Its name.
 */
name: string, 
/**
 * What it is.
 */
kind: EntryKind, 
/**
 * Its size in bytes, for a file.
 */
size?: number | null, 
/**
 * Whether git ignores it.
 */
ignored?: boolean, };

/**
 * A folder's entries.
 */
export type DirListing = { 
/**
 * Folders first, then by name; at most 5,000.
 */
entries: Array<DirEntry>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * `git.discard`: drop local changes.
 */
export type Discard = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which files.
 */
paths: PathSet, 
/**
 * Delete the untracked ones among them too (never ignored files).
 */
untracked: boolean, };

/**
 * What is happening right now in the lane a session already sits in. It
 * changes with turns, permissions, checks and reviews while the lane stays
 * put.
 */
export type DisplayStatus = "working" | "blocked" | "exited" | "no_signal" | "interrupted" | "awaiting_pr" | "idle" | "fixing_ci" | "addressing_comments" | "needs_review" | "review_scheduled" | "reviewing" | "review_failed" | "review_pending" | "draft" | "ci_failing" | "commented" | "changes_requested" | "needs_human_review" | "mergeable" | "approved" | "merged" | "closed" | "terminated";

/**
 * How far a branch is from another.
 */
export type Distance = { 
/**
 * The other branch.
 */
name: RefName, 
/**
 * Commits here that it does not have.
 */
ahead: number, 
/**
 * Commits it has that are not here.
 */
behind: number, };

/**
 * Symbols in one saved file.
 */
export type DocumentSymbols = { 
/**
 * Authorized server selection.
 */
workspace: Workspace, 
/**
 * Relative saved file.
 */
file: FilePath, 
/**
 * Sorted result page.
 */
page: PageRequest, };

/**
 * `worktree.drop_snapshot`: delete a snapshot's ref.
 */
export type DropSnapshot = { 
/**
 * Which project.
 */
project: ProjectId, 
/**
 * The worktree the snapshot was taken of.
 */
worktree: WorktreeId, };

/**
 * Duplicate an entry to the first available sibling copy name.
 */
export type DuplicateFileEntry = { 
/**
 * Authorized workspace selection.
 */
root: WorkspaceRoot, 
/**
 * Source entry; the workspace root itself is refused.
 */
path: RelPath, };

/**
 * The new sibling path; never an assumed local path on the client.
 */
export type DuplicatedFileEntry = { 
/**
 * The exclusively created copy.
 */
path: RelPath, };

/**
 * A duration in whole milliseconds.
 */
export type DurationMs = number;

/**
 * Why a schedule ended.
 */
export type EndReason = "max_runs" | "expired" | "creator_gone" | "target_gone" | "unknown";

/**
 * What an entry of a folder is.
 */
export type EntryKind = "file" | "dir" | "symlink" | "other" | "unknown";

/**
 * Results are bounded and explicitly disclose an incomplete scan.
 */
export type EntrySuggestions = { 
/**
 * Best matches first; deterministic path order breaks score ties.
 */
entries: Array<SuggestedEntry>, 
/**
 * Whether discovery reached its depth/entry budget or results were clipped.
 */
more: boolean, };

/**
 * An environment variable's name.
 */
export type EnvName = string;

/**
 * What kind of failure occurred. Clients branch on this.
 */
export type ErrorCode = "invalid_request" | "unauthorized" | "forbidden" | "protocol_mismatch" | "not_found" | "conflict" | "unsupported" | "harness_not_installed" | "spawn_failed" | "unavailable" | "internal";

/**
 * Further typed context for a refusal. Unknown future reasons stay readable.
 */
export type ErrorDetail = { "kind": "diagnostics", 
/**
 * Resource refusal, never a source error message.
 */
reason: DiagnosticRefusal, } | { "kind": "schedule_preview", 
/**
 * The stable preview reason.
 */
reason: SchedulePreviewError, } | { "kind": "fleet_control", 
/**
 * The stable fleet reason.
 */
reason: FleetControlError, } | { "kind": "device_invite", 
/**
 * The stable pairing setup reason.
 */
reason: DeviceInviteError, } | { "kind": "unknown" };

/**
 * Reply to `session.read`.
 */
export type EventPage = { 
/**
 * Records in order, starting at the requested `from` (or the oldest
 * retained record if older ones were pruned).
 */
records: Array<EventRecord>, 
/**
 * Sequence number to ask for next.
 */
next: EventSeq, 
/**
 * Whether `next` is the end of the log right now.
 */
at_end: boolean, };

/**
 * An event with its position and time; the unit of the session log.
 */
export type EventRecord = { 
/**
 * Position in the session log.
 */
seq: EventSeq, 
/**
 * When the node recorded it.
 */
at: string, 
/**
 * What happened.
 */
event: SessionEvent, };

/**
 * Position of an event in one session's log. Starts at zero and has no
 * gaps, so a client resumes with "everything from seq N".
 */
export type EventSeq = number;

/**
 * Seconds between an `every` schedule's due times: at least a minute
 * (all three references' floor), at most 366 days.
 */
export type EverySeconds = number;

/**
 * `memory.export` (protocol 6): snapshot a profile's memory, one
 * project's or all of it. Needs `admin`: memory is kept per account on
 * purpose (plan decision 4).
 */
export type ExportMemory = { 
/**
 * Whose memory; absent is the harness's default profile.
 */
profile?: ProfileName | null, 
/**
 * One project's, by its working directory on this node; absent is
 * every project.
 */
project?: string | null, };

/**
 * `session.export` (protocol 6): snapshot a structured session's harness
 * files (never its profile's memory) into a transfer. A session mid-turn is
 * refused unless `force`, as for `session.move_profile`. An `operate` call
 * on that session, so an agent exports only what it started. Needs
 * `operate`.
 */
export type ExportSession = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * Snapshot even while a turn runs.
 */
force?: boolean, };

/**
 * Permission for this node to use configured agents' saved sign-ins locally.
 */
export type ExternalLookup = "off" | "on";

/**
 * Whether the provider explicitly enabled additional paid use.
 */
export type ExtraUsage = "enabled" | "disabled";

/**
 * `git.fetch`.
 */
export type Fetch = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * This remote; the branch's, else `origin`, when absent.
 */
remote?: RemoteName | null, };

/**
 * `git.fetch`'s answer.
 */
export type FetchOutcome = { "outcome": "fetched" } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * The key of one field of a question's form, as the agent names it
 * (`question_1`). Echoed back in an answer.
 */
export type FieldName = string & { readonly __brand: "FieldName" };

/**
 * One exact page with a version-bound continuation.
 */
export type FileBytePage = { 
/**
 * Stable identity of the entire source.
 */
revision: FileRevision, 
/**
 * Starting byte, echoed to detect misrouted/incomplete transfers.
 */
offset: FileOffset, 
/**
 * Exact source bytes.
 */
bytes: FileBytes, 
/**
 * Next byte when the source has more; absent at EOF.
 */
next: FileOffset | null, };

/**
 * A bounded opaque page; base64 on the wire, never interpreted as text.
 */
export type FileBytes = string;

/**
 * One changed file in a scope.
 */
export type FileChange = { 
/**
 * Its path.
 */
path: RelPath, 
/**
 * Where it was, for a rename or a copy.
 */
old_path?: RelPath | null, 
/**
 * How it changed.
 */
status: ChangeStatus, 
/**
 * Lines added; absent for a binary file.
 */
additions?: number | null, 
/**
 * Lines removed; absent for a binary file.
 */
deletions?: number | null, 
/**
 * Whether git sees it as binary.
 */
binary: boolean, };

/**
 * What a page of a file holds.
 */
export type FileContent = { "kind": "text", 
/**
 * The page, cut back to a character boundary.
 */
text: string, 
/**
 * Where the rest starts, when there is more.
 */
next?: number | null, } | { "kind": "binary" } | { "kind": "unknown" };

/**
 * One file's diff in a scope.
 */
export type FileDiff = { 
/**
 * Its path.
 */
path: RelPath, 
/**
 * Where it was, for a rename or a copy.
 */
old_path?: RelPath | null, 
/**
 * How it changed.
 */
status: ChangeStatus, 
/**
 * Its mode changed.
 */
mode?: ModeChange | null, 
/**
 * What it has.
 */
content: DiffContent, 
/**
 * The index of the first hunk not sent, when a call's 5,000 lines
 * ran out; ask again with `from_hunk`.
 */
more_from?: number | null, };

/**
 * Filesystem modification time for display/order, never a save guard.
 */
export type FileModifiedMillis = number;

/**
 * A byte position in a file, independent of character positions.
 */
export type FileOffset = number;

/**
 * A file under the authorized workspace, never an absolute path or URI.
 */
export type FilePath = string;

/**
 * A page of a file.
 */
export type FileRead = { 
/**
 * The version read, for a later write.
 */
version: FileVersion, 
/**
 * The page.
 */
content: FileContent, };

/**
 * The exact content identity of a bounded file, also its transfer size.
 */
export type FileRevision = { 
/**
 * The number of source bytes, never a UTF-8 character count.
 */
size: FileSize, 
/**
 * SHA-256 over the complete file, including binary bytes.
 */
hash: ContentHash, };

/**
 * A source byte count, distinct from a position within that source.
 */
export type FileSize = number;

/**
 * What a file is in the index or the working tree, against the other.
 */
export type FileState = "unmodified" | "modified" | "added" | "deleted" | "renamed" | "copied" | "type_changed" | "untracked" | "conflicted" | "unknown";

/**
 * A shell command a person must see before approving a project's terminals.
 */
export type FileTerminal = { 
/**
 * The terminal's title; a blank file name is represented as absent.
 */
name?: SessionTitle | null, 
/**
 * Kept as validated text so approval covers exactly the shell input.
 */
command: ProjectCommand, };

/**
 * A file as it was read or written: what a page shows, and the hash a
 * later write is guarded by ([`WriteExpect::Content`]).
 *
 * [Decision Log]
 * - Purpose: a write must never overwrite a change it did not see, and a
 *   page must be able to say exactly which version it saw.
 * - Constraint: the page is JavaScript, whose numbers hold integers to
 *   2^53 only; protocol 10 had `modified` in nanoseconds (past 2^53 since
 *   1970 plus 104 days) and compared it exactly, so a page's guard almost
 *   never matched and the page wrote with `any` after a check of its
 *   own, which a change landing in between could be lost to.
 * - Alternatives: the time as text (exact, but a guard by the clock is
 *   still wrong on a coarse file system, FAT's 2 s); the whole version
 *   echoed back with `modified` ignored (a field sent and not compared
 *   misleads); the content hash alone (chosen).
 * - Choice (the join of wave 2's first half): the guard is the content's
 *   SHA-256, which a page holds exactly as text; `modified` is shown only,
 *   in milliseconds, which a JavaScript number holds exactly.
 * - Cost: a file over 8 MiB has no hash, so it can be replaced only with
 *   `any` (the editor edits at most 1 MiB anyway); a guarded write hashes
 *   the file once more (at most 8 MiB, on the blocking pool).
 */
export type FileVersion = { 
/**
 * Its size in bytes.
 */
size: number, 
/**
 * When it was last changed, in milliseconds since the Unix epoch, as
 * the file system says (0 when it does not, and for a blob): for
 * showing only, never compared. A JavaScript number holds it exactly.
 */
modified: number, 
/**
 * SHA-256 of its content, for a file of at most 8 MiB: what a later
 * write is guarded by ([`WriteExpect::Content`]).
 */
hash?: ContentHash | null, };

/**
 * `file.find`: files by name.
 */
export type FindFiles = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * What to look for.
 */
query: FindQuery, 
/**
 * The search this one replaces: a newer `file.find` with the same key
 * from the same caller stops an older one still running.
 */
key?: SearchKey | null, 
/**
 * At most this many (at most 200, 50 when absent).
 */
limit?: number | null, };

/**
 * What `file.find` looks for.
 */
export type FindQuery = string;

/**
 * `attachment.finish`: check an upload and keep it with its session.
 * Needs `operate`.
 */
export type FinishUpload = { 
/**
 * Which upload.
 */
upload: UploadId, };

/**
 * A JSON number that is finite: no NaN or infinity, so the types holding
 * it compare as equal or not (`Eq`), and it always encodes as JSON.
 */
export type FiniteNumber = number;

/**
 * Safe for any caller with Read; no file text, credentials or secret names.
 */
export type FleetApplied = { 
/**
 * The last generation applied here, possibly before the latest join.
 * It has no current-home association: never compare its bare number
 * with the current home's generation to claim this node is up to date.
 */
generation: Generation | null, 
/**
 * When that apply completed.
 */
applied_at: string | null, 
/**
 * When the member's home last answered.
 */
contacted_at: string | null, 
/**
 * Bundle-relative paths kept as edited, deleted or not owned by Vessel.
 */
conflicts: Array<BundlePath>, 
/**
 * Issues at the last recorded operation.
 */
problems: Array<FleetSyncProblem>, };

/**
 * Why a fleet control was refused, without invitation or filesystem text.
 */
export type FleetControlError = "busy" | "already_home" | "already_member" | "self_home" | "invalid_invitation" | "cannot_join" | "cannot_store" | "join_uncertain" | "not_configured" | "direct_only" | "cannot_read" | "status_too_large" | "published_too_large" | "unknown";

/**
 * A member as its home reports it.
 */
export type FleetMember = { 
/**
 * The member's device on the home.
 */
device: DeviceId, 
/**
 * Its label.
 */
label: string, 
/**
 * The generation it last applied, if it has reported.
 */
applied?: Generation | null, 
/**
 * Files it kept as edited.
 */
conflicts?: Array<BundlePath>, 
/**
 * When it last reported.
 */
reported_at?: string | null, };

/**
 * A node of a fleet, the home included.
 */
export type FleetNode = { 
/**
 * Its id.
 */
node: NodeId, 
/**
 * Its name.
 */
name: string, 
/**
 * The protocol its home speaks with it (a protocol 5 member gets the
 * bundle but no delivered calls).
 */
protocol: ProtocolVersion, 
/**
 * Whether the home can reach it now.
 */
reachable: boolean, };

/**
 * The answer, by this machine's role.
 */
export type FleetPublished = { "role": "standalone" } | { "role": "home", 
/**
 * What it publishes.
 */
bundle?: PublishedBundle | null, } | { "role": "member", 
/**
 * Where the bundle came from.
 */
home: NodeId, 
/**
 * What it last received.
 */
bundle?: PublishedBundle | null, 
/**
 * The version last applied here: older than `bundle`'s when the
 * last apply did not finish, absent before the first.
 */
applied?: Generation | null, } | { "role": "unknown" };

/**
 * This node's current configured role, independently of connectivity.
 */
export type FleetRole = { "kind": "standalone" } | { "kind": "home" } | { "kind": "member", 
/**
 * The configured home.
 */
home: NodeId, 
/**
 * When the current membership was persisted.
 */
joined_at: string, } | { "kind": "unknown" };

/**
 * Actionable issue classes; arbitrary file errors remain in the local log.
 */
export type FleetSyncProblem = "operation_failed" | "apply_failed" | "incomplete_sources" | "grant_failed" | "unknown";

/**
 * Local state plus the home's per-member reports when the caller is Admin.
 */
export type FleetSyncStatus = { 
/**
 * The configured role.
 */
role: FleetRole, 
/**
 * Last apply here, not a promise that the current sources were applied.
 */
local: FleetApplied, 
/**
 * Present only on a home for an Admin caller; read-only sees none.
 */
members?: Array<FleetMember> | null, };

/**
 * The name of one folder to create (a clone's).
 */
export type FolderName = string;

/**
 * Whether the node can see a session's pull request, and why not.
 */
export type ForgeAccess = { "access": "ready", 
/**
 * Who the node is signed in as, when the forge said.
 */
login?: ForgeLogin | null, } | { "access": "not_placed" } | { "access": "no_branch" } | { "access": "on_base" } | { "access": "no_remote" } | { "access": "not_supported", 
/**
 * The host.
 */
host: ForgeHost, } | { "access": "signed_out", 
/**
 * The host.
 */
host: ForgeHost, 
/**
 * What `gh` said.
 */
gh: GhState, } | { "access": "refused", 
/**
 * The host.
 */
host: ForgeHost, 
/**
 * Why.
 */
reason: RefusedReason, 
/**
 * Who the node is signed in as, when known.
 */
login?: ForgeLogin | null, } | { "access": "rate_limited", 
/**
 * The host.
 */
host: ForgeHost, 
/**
 * When reading starts again.
 */
until: string, } | { "access": "unreachable", 
/**
 * The host.
 */
host: ForgeHost, 
/**
 * When the node tries again.
 */
retry_at: string, } | { "access": "limit" } | { "access": "ended" } | { "access": "unknown" };

/**
 * A forge's host as a remote names it (`github.com`, `git.example.org:8443`).
 */
export type ForgeHost = string;

/**
 * Which kind of forge a repository is on.
 */
export type ForgeKind = "github" | "unknown";

/**
 * A person's or a bot's login on a forge.
 */
export type ForgeLogin = string;

/**
 * `forge.status`'s answer.
 */
export type ForgeStatus = { 
/**
 * Every host the node watched or was asked about, at most
 * [`ForgeStatus::MAX_HOSTS`].
 */
hosts: Array<HostStatus>, 
/**
 * Pull requests watched now.
 */
watched: number, 
/**
 * The most it watches.
 */
limit: number, };

/**
 * A page on a forge: `https` (or `http` on a loopback host, the
 * tests' fake), with no user information.
 */
export type ForgeUrl = string;

/**
 * `fleet.forget` (protocol 6): on the home, forget member node `node`'s
 * binding (its key and name, kept for good otherwise), for a node
 * reinstalled with a new key or to be renamed. Its grants are deleted;
 * the invitations outstanding on the home are withdrawn, and the node's
 * id and name go next only to a member device paired after this, so the
 * node is invited again and joins from there. Needs `admin`; never
 * forwarded.
 */
export type ForgetNode = { 
/**
 * The member node.
 */
node: NodeId, };

/**
 * `session.fork`: a new session that continues `session`'s conversation
 * as it was after the turn whose end is record `after`, or as it is now.
 * Needs `operate`; answers the new session, as `session.create` does.
 */
export type ForkSession = { 
/**
 * The session forked.
 */
session: SessionId, 
/**
 * A `turn_ended` record of it that carries a point; absent for the
 * whole conversation (refused while a turn runs).
 */
after?: EventSeq | null, };

/**
 * `fleet.forward` (protocol 6): run `call` on fleet node `node`. Sent by a
 * client to its own node, which checks `call` against the caller as if it
 * were local, then passes it on with the caller's [`Origin`]. Needs `read`
 * here; the inner call's own scope is checked on every hop.
 */
export type Forward = { 
/**
 * The node that runs the call; this node itself runs it locally.
 */
node: NodeId, 
/**
 * The call, which must be [forwardable](Method::forwardable).
 */
call: Call, };

/**
 * Reply to `fleet.forward`, `fleet.relay` and `fleet.deliver`: the inner
 * call's reply. An inner call that fails fails the envelope with its error.
 */
export type Forwarded = { 
/**
 * What the target answered.
 */
reply: Reply, };

/**
 * A file whose name matched.
 */
export type FoundFile = { 
/**
 * Its path.
 */
path: RelPath, 
/**
 * The characters of `path` (counted in Unicode scalar values, from 0)
 * that matched the query, for highlighting.
 */
positions: Array<number>, };

/**
 * What `file.find` found.
 */
export type FoundFiles = { 
/**
 * Best first.
 */
matches: Array<FoundFile>, 
/**
 * How many files were searched.
 */
indexed: number, 
/**
 * Whether every file of the root was searched (the index has a
 * bound).
 */
complete: boolean, 
/**
 * Whether a newer search from the same connection replaced this one
 * before it finished (then `matches` is empty).
 */
superseded?: boolean, };

/**
 * How saved contents relate to the response.
 */
export type Freshness = "saved" | "cached" | "unknown";

/**
 * A bundle generation. The home increases it by one each time it
 * publishes; members compare it to what they last applied.
 */
export type Generation = number;

/**
 * `agent.history`: a page of `harness`'s own conversations, as its agent
 * lists them. The node starts the agent as `harness.options` does (under
 * `profile`, in a scratch folder), asks it, and stops it; no session is
 * started or loaded. Needs `operate`; refused to agents; at most two at
 * once on a node (`unavailable` beyond).
 */
export type GetAgentHistory = { 
/**
 * Which agent; an added agent needs protocol 11.
 */
harness: HarnessRef, 
/**
 * Under this profile (another account keeps other conversations);
 * `None` is the harness's default.
 */
profile?: ProfileName | null, 
/**
 * Only the conversations held in this folder (absolute, on the
 * node; the node resolves its links); all of them without.
 */
cwd?: string | null, 
/**
 * Where the page starts: a page's `next`, as it was given (at most
 * [`GetAgentHistory::MAX_CURSOR_BYTES`]; anything else is
 * `invalid_request`); the newest without.
 */
cursor?: string | null, };

/**
 * `agent.status`: the facts of every agent and profile, or of one agent
 * (and one of its profiles). Needs `read`; `refresh`, which runs checks,
 * is refused to an agent and needs `agent`.
 */
export type GetAgentStatus = { 
/**
 * Only this agent.
 */
agent?: HarnessRef | null, 
/**
 * Only this profile of it (its default without one).
 */
profile?: ProfileName | null, 
/**
 * Check now: run its status command and its handshake (no prompt).
 */
refresh?: boolean, };

/**
 * `fleet.callback` (protocol 6): what a member with no route of its own
 * needs for its home to call it back: the relays the home registers with,
 * and a ticket that registers this member's own node key there for a while
 * (never the relays' token). The home answers only a member device, for
 * the node and key its record binds that device to, or, before its first
 * grant, for a claim its record would take. Asked again before the ticket
 * expires. Needs `sync`; never forwarded.
 */
export type GetCallback = { 
/**
 * The member's node.
 */
node: NodeId, 
/**
 * Its key, which the ticket names.
 */
key: NodePublicKey, };

/**
 * `agent.catalogue`: the node's copy of the registry. Needs `read`;
 * `refresh`, which fetches it, is refused to an agent.
 */
export type GetCatalogue = { 
/**
 * Fetch it first (unless the copy is younger than a minute).
 */
refresh?: boolean, };

/**
 * `git.changes`: the changed files of a scope.
 */
export type GetChanges = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which changes.
 */
scope: DiffScope, 
/**
 * Leave out changes that only touch white space.
 */
ignore_whitespace: boolean, };

/**
 * `session.conversation`: a session's conversation as it stands. Needs
 * `read`.
 */
export type GetConversation = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * Inspect this authenticated caller only. No id can be selected.
 */
export type GetDeviceSelf = Record<symbol, never>;

/**
 * Request a bounded metadata-only report. Never refreshes providers or agents.
 */
export type GetDiagnostics = Record<symbol, never>;

/**
 * `git.diff`: one file's hunks in a scope.
 */
export type GetFileDiff = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which changes.
 */
scope: DiffScope, 
/**
 * Which file (its new path, for a rename).
 */
path: RelPath, 
/**
 * Leave out changes that only touch white space.
 */
ignore_whitespace: boolean, 
/**
 * From this hunk on.
 */
from_hunk?: number | null, };

/**
 * Read this node's role and last apply, without resolving any secrets.
 */
export type GetFleetStatus = Record<symbol, never>;

/**
 * `forge.status`: how the node is signed in to each forge.
 */
export type GetForgeStatus = Record<symbol, never>;

/**
 * `harness.options` (protocol 9): what a harness's agent offers before
 * any session of it exists, for a new-session form. The node starts the
 * agent as a structured session would (under `profile`, in a scratch
 * directory), reads its options from the handshake, and ends it; no
 * prompt is sent. Needs `read`.
 */
export type GetHarnessOptions = { 
/**
 * Which harness; an added agent needs protocol 11.
 */
harness: HarnessRef, 
/**
 * Under this profile; `None` is the harness's default. A profile can
 * change what an agent offers (another account, another provider).
 */
profile?: ProfileName | null, };

/**
 * Hover at a saved source position.
 */
export type GetHover = AtPosition;

/**
 * Look up one exact current entry, including after it leaves a loaded page.
 * An absent entry is resolved/withdrawn, not a successful duplicate answer.
 */
export type GetInboxEntry = { 
/**
 * Source the client is still showing or awaiting confirmation for.
 */
key: InboxKey, 
/**
 * Opt-in full live form. A single reply is capped at 15 MiB; larger
 * content stays an explicit compact reference, never shortened choices.
 */
fetch?: InboxFetch, };

/**
 * `fleet.manifest` (protocol 5): the home's current bundle. Needs `Sync`.
 */
export type GetManifest = null;

/**
 * `node.info`: describe the node.
 */
export type GetNode = null;

/**
 * `node.settings`: the node's settings in force. Needs `read`.
 */
export type GetNodeSettings = Record<symbol, never>;

/**
 * `project.get`: one project, at once or after its next change.
 */
export type GetProject = { 
/**
 * Which.
 */
project: ProjectId, 
/**
 * Wait (at most 30 s) until its `revision` differs from this one.
 */
changed_from?: number | null, };

/**
 * `project.commands`: what the project's file would run.
 */
export type GetProjectCommands = { 
/**
 * Which project.
 */
project: ProjectId, 
/**
 * The file at this commit or ref; the project's folder as it is
 * otherwise.
 */
base?: WorktreeBase | null, };

/**
 * `fleet.published`: what this machine shares, or last received, through
 * its fleet. Needs `admin`.
 */
export type GetPublished = Record<symbol, never>;

/**
 * `pr.get`: a session's pull request, whole.
 */
export type GetPullRequest = { 
/**
 * The session.
 */
session: SessionId, 
/**
 * Read it from the forge now (at most once per 10 s per session;
 * otherwise the watcher's copy).
 */
refresh: boolean, };

/**
 * Get the node's public VAPID identity, never its private key.
 */
export type GetPushKey = null;

/**
 * Long-poll one complete registry revision, with a finite server wait.
 */
export type GetRegistry = { 
/**
 * None reads immediately; an unchanged revision may wait within caller shares.
 */
changed_from: RegistryRevision | null, };

/**
 * `schedule.get`: one schedule. Needs `read`.
 */
export type GetSchedule = { 
/**
 * Which.
 */
schedule: ScheduleId, };

/**
 * `session.get`: one session.
 */
export type GetSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * `session.options` (protocol 9): a structured session's options as its
 * agent last reported them. Needs `read`.
 */
export type GetSessionOptions = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * Read host-level state; Admin only, never an agent tool.
 */
export type GetSkillsStatus = Record<symbol, never>;

/**
 * `git.status`.
 */
export type GetStatus = { 
/**
 * Where.
 */
root: WorkspaceRoot, };

/**
 * `terminal.get`: one terminal, at once or after its next change.
 */
export type GetTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, 
/**
 * Wait (at most 30 s) until its `revision` differs from this one.
 */
changed_from?: number | null, };

/**
 * usage.limits: read cached facts; operate additionally permits refresh.
 */
export type GetUsageLimits = { 
/**
 * Selected saved login, never a path or endpoint.
 */
scope: LimitsScope, 
/**
 * Whether to request a bounded refresh.
 */
refresh: LimitRefresh, };

/**
 * Direct identity lookup includes archived records for recovery inspection.
 */
export type GetWorkspace = { 
/**
 * Exact host-local workspace identity.
 */
workspace: WorkspaceId, };

/**
 * What looking for the machine's `gh` found.
 */
export type GhState = "missing" | "signed_out" | "failed" | "unknown";

/**
 * A successful read distinguishes absent comparison, bounded refusal and data.
 * Partial coverage stays in the existing summary's `more` field.
 */
export type GitCompared<T> = { "kind": "no_base" } | { "kind": "base_mismatch", 
/**
 * The ref recorded when this worktree was created.
 */
stored: RefName, 
/**
 * The different base selected by the caller.
 */
requested: WorktreeBase, } | { "kind": "too_large" } | { "kind": "ready", 
/**
 * Payload from the existing git parser, not text interpreted by a UI.
 */
data: T, };

/**
 * Which two versions a git document compares.
 */
export type GitComparison = { "kind": "uncommitted" } | { "kind": "committed", 
/**
 * Absent means the registered worktree/project base.
 */
base?: WorktreeBase | null, } | { "kind": "all", 
/**
 * Absent means the registered worktree/project base.
 */
base?: WorktreeBase | null, } | { "kind": "commit", 
/**
 * Already parsed before any git process is started.
 */
id: CommitId, };

/**
 * A continuation supplied by the node; zero requests the first bounded page.
 */
export type GitHunkOffset = number;

/**
 * A git operation that leaves a repository between two states.
 */
export type GitOperation = "merge" | "rebase" | "cherry_pick" | "revert" | "bisect";

/**
 * A forced refresh always carries a typed reason, never an unchecked flag.
 */
export type GitReadReason = "view" | "refresh" | "mutation" | "recovery";

/**
 * Why git refused a change: what a client can act on.
 */
export type GitRefusal = { "kind": "needs_credential" } | { "kind": "timed_out" } | { "kind": "non_fast_forward" } | { "kind": "diverged" } | { "kind": "would_overwrite", 
/**
 * The files, at most 100.
 */
paths: Array<RelPath>, } | { "kind": "conflicts", 
/**
 * The files, at most 100.
 */
paths: Array<RelPath>, } | { "kind": "nothing_to_commit" } | { "kind": "no_identity" } | { "kind": "hook_failed", 
/**
 * Its last lines.
 */
output: string, } | { "kind": "no_upstream" } | { "kind": "detached" } | { "kind": "checked_out_elsewhere", 
/**
 * That folder.
 */
path: string, } | { "kind": "nothing_to_stash" } | { "kind": "stash_moved" } | { "kind": "branch_exists" } | { "kind": "no_such_ref" } | { "kind": "operation", 
/**
 * Which.
 */
operation: GitOperation, } | { "kind": "remote_failed", 
/**
 * git's last lines, credentials removed.
 */
message: string, } | { "kind": "repository_config", 
/**
 * The key, as git lists it; absent when the whole configuration
 * was refused (`too_large`).
 */
key?: ConfigKey | null, 
/**
 * Why.
 */
reason: RepositoryConfigReason, } | { "kind": "unknown" };

/**
 * A root's git status.
 */
export type GitStatus = { 
/**
 * The branch checked out; absent when detached.
 */
branch?: BranchName | null, 
/**
 * The commit checked out; absent before the first commit.
 */
head?: CommitId | null, 
/**
 * Against the branch's upstream, when it has one.
 */
upstream?: Distance | null, 
/**
 * Against the base a worktree was made from, when that is a branch.
 */
base?: Distance | null, 
/**
 * An operation in progress.
 */
operation?: GitOperation | null, 
/**
 * Every changed, staged, untracked or conflicted file (not ignored
 * ones), at most 5,000.
 */
files: Array<StatusEntry>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * Whether the node can run the git calls.
 */
export type GitSupport = { "state": "found", 
/**
 * Its version.
 */
version: GitVersion, } | { "state": "too_old", 
/**
 * Its version.
 */
version: GitVersion, 
/**
 * The oldest that works.
 */
minimum: GitVersion, } | { "state": "missing" } | { "state": "unknown" };

/**
 * A git version, as `git --version` reports it (`2.43.0`, `2.45.1` of
 * `2.45.1.windows.1`).
 */
export type GitVersion = { 
/**
 * The first number.
 */
major: number, 
/**
 * The second.
 */
minor: number, 
/**
 * The third (0 when git printed two).
 */
patch: number, };

/**
 * A typed preference in query/cache identity, not a free-form git option.
 */
export type GitWhitespace = "show" | "ignore";

/**
 * `fleet.grant` (protocol 6): a member gives its home a credential on
 * itself, as a single-use pairing link made with its own `device.invite`
 * and the scopes its owner chose. The home redeems it like any device and
 * keeps the credential; revoking it on the member cuts the home off.
 * Needs `sync`.
 */
export type GrantHome = { 
/**
 * The link, made on the member.
 */
link: PairingLink, };

/**
 * Whether case matters in a search in files.
 */
export type GrepCase = "sensitive" | "insensitive" | "smart";

/**
 * One line that matched.
 */
export type GrepMatch = { 
/**
 * The file.
 */
path: RelPath, 
/**
 * The line, from 1.
 */
line: number, 
/**
 * Where the match starts on it, from 1, in Unicode scalar values.
 */
column: number, 
/**
 * The line, at most 512 characters.
 */
text: string, 
/**
 * Whether it was longer and cut.
 */
cut?: boolean, };

/**
 * How a search in files reads its query.
 */
export type GrepMode = "literal" | "regex";

/**
 * What `file.grep` looks for.
 */
export type GrepQuery = string;

/**
 * What `file.grep` found.
 */
export type GrepResult = { 
/**
 * By file, then line.
 */
matches: Array<GrepMatch>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * A harness found on a node.
 */
export type Harness = { 
/**
 * Which harness: one of the four, or (protocol 11) an agent added on
 * the node, which a node leaves out for an older client.
 */
kind: HarnessRef, 
/**
 * Resolved executable path.
 */
path: string, 
/**
 * First line of `<binary> --version`, if it answered in time.
 */
version: string | null, 
/**
 * Structured-mode availability.
 */
structured: StructuredSupport, 
/**
 * Terminal-mode availability (protocol 11): every one of the four
 * installed runs in a terminal; an added agent does not. Left out
 * when available.
 */
terminal?: TerminalSupport, };

/**
 * An agent harness Vessel knows how to drive.
 *
 * Closed on purpose: everything harness-specific (argv, environment,
 * structured-mode launch) lives in one module per variant in
 * `vessel-harness`, and adding a variant must fail to compile until that
 * module exists.
 */
export type HarnessKind = "claude-code" | "codex" | "gemini" | "open-code";

/**
 * Reply to `harness.options`.
 */
export type HarnessOptions = { 
/**
 * What the agent offered, in its order. Empty for an agent that
 * offers none.
 */
options: Array<SessionOption>, 
/**
 * When the node asked the agent: the node answers from a recent probe
 * rather than start the agent for every caller.
 */
probed_at: string, 
/**
 * What the agent said it can do (protocol 10): whether a new
 * session's composer offers steering and images. Absent from older
 * nodes, and for a harness without structured sessions here.
 */
abilities?: AgentAbilities | null, };

/**
 * Any agent a node can run (protocol 11, ADR 0032): one of the four Vessel
 * knows, or one added on that node by its command.
 *
 * On the wire a known agent is the string it was before protocol 11
 * (`"claude-code"`, `"codex"`, `"gemini"`, `"open-code"`), so every spec
 * and list an older node or client wrote reads the same; an added agent is
 * `{"added": "<name>"}`, an object, so it can never be read as one of the
 * four.
 */
export type HarnessRef = HarnessKind | { added: AgentName, };

/**
 * The harness's own session id (an ACP `sessionId`). Kept so a
 * structured session can be resumed by the harness after a node restart.
 */
export type HarnessSessionId = string & { readonly __brand: "HarnessSessionId" };

/**
 * An HTTP header field name.
 */
export type HeaderName = string;

/**
 * One conversation an agent keeps. Importing it is `session.create` with
 * `resume: harness_session`, its `cwd`, the `profile` it was listed under
 * (the conversation is in that account's files), and its `title` (an
 * imported session is otherwise untitled until its next prompt). The
 * import is refused `conflict` while another live session is on it.
 */
export type HistoryEntry = { 
/**
 * The agent's id for it.
 */
harness_session: HarnessSessionId, 
/**
 * The folder it ran in, absolute.
 */
cwd: string, 
/**
 * Its title as the agent gives it, cut as a prompt's title is.
 */
title?: SessionTitle | null, 
/**
 * When it was last active, as the agent says it.
 */
updated_at?: string | null, 
/**
 * The newest session on this node, live or kept, on this
 * conversation of this agent: a client opens it (`session.resume` for
 * an ended one) rather than import it twice. A rewound session is on
 * its new conversation, so the one it left lists unmarked.
 */
in_vessel?: SessionId | null, };

/**
 * How the node is signed in to one host.
 */
export type HostStatus = { 
/**
 * The host.
 */
host: ForgeHost, 
/**
 * Its kind.
 */
kind: ForgeKind, 
/**
 * What signing in found.
 */
access: ForgeAccess, 
/**
 * Where the token came from, when there is one.
 */
source?: TokenSource | null, };

/**
 * Hover text is untrusted content; clients must not render raw HTML.
 */
export type Hover = { 
/**
 * Plain text or Markdown as indicated by `format`.
 */
text: string, 
/**
 * The text's representation.
 */
format: Markup, 
/**
 * Optional range in the requested file.
 */
range: Range | null, 
/**
 * Whether text was shortened to fit its budget.
 */
truncated: boolean, 
/**
 * Scope and freshness, even when text is empty.
 */
context: Context, };

/**
 * An `http://` or `https://` URL with a host and no credentials in it.
 *
 * A URL is public: it travels in manifests and, for some harnesses, on a
 * command line that every local user can read (`ps`). So `user:pass@` is
 * refused; a credential goes in a header, as a secret reference
 * (`ConfigValue::Secret`). A token in the query string cannot be told from
 * an ordinary parameter, so the same rule is stated here rather than
 * enforced: keep secrets out of URLs.
 */
export type HttpUrl = string;

/**
 * A run of changed lines and their context.
 */
export type Hunk = { 
/**
 * Its first line on the old side (1-based; 0 for an empty side).
 */
old_start: number, 
/**
 * Its lines on the old side.
 */
old_lines: number, 
/**
 * Its first line on the new side.
 */
new_start: number, 
/**
 * Its lines on the new side.
 */
new_lines: number, 
/**
 * The function or section git names after `@@`, when it does.
 */
section?: string | null, 
/**
 * Its lines.
 */
lines: Array<DiffLine>, };

/**
 * Minutes of idleness before a park: 1 to [`IdleMinutes::MAX`] (a week).
 */
export type IdleMinutes = number;

/**
 * What `worktree.remove` does when something would be lost.
 */
export type IfLost = "keep" | "snapshot" | "discard";

/**
 * Git's classification, independent from the request's inclusion policy.
 */
export type IgnoreState = "ignored" | "visible";

/**
 * Whether directory discovery includes git-ignored entries.
 */
export type IgnoredEntries = "include" | "exclude";

/**
 * Implementations at a saved source position.
 */
export type Implementations = AtPosition;

/**
 * `memory.import` (protocol 6): the same pull for memory. A file that
 * differs from one already there is written beside it
 * (`.vessel-incoming`), as the fleet's materialiser does; nothing is
 * overwritten. Needs `admin`.
 */
export type ImportMemory = { 
/**
 * The memory to copy.
 */
from: MemoryRef, 
/**
 * The profile here; absent is the default.
 */
profile?: ProfileName | null, 
/**
 * The project here, when copying one; absent is the source's.
 */
project?: string | null, };

/**
 * `session.import` (protocol 6): the target pulls a session from `from`
 * (exporting it there, through `fleet.forward` when it is another node),
 * places its files in the chosen profile's home, renamed for the working
 * directory here, and with `start` resumes it as a new session. A file
 * already there with other content refuses the import before anything is
 * written. Moving is importing, then killing the source. Needs `operate`.
 */
export type ImportSession = { 
/**
 * The session to copy.
 */
from: SessionRef, 
/**
 * The profile here; absent is the harness's default.
 */
profile?: ProfileName | null, 
/**
 * The working directory here; absent is the source's.
 */
cwd?: string | null, 
/**
 * Start a session resuming the conversation.
 */
start?: boolean, };

/**
 * Import older client selection only if the host has never stored one.
 */
export type ImportSkillsSelection = { 
/**
 * Validated legacy preference, never arbitrary input paths.
 */
selection: SkillSelection, };

/**
 * A count of inbox entries, independent of the currently loaded page.
 */
export type InboxCount = number;

/**
 * Exact counts for the whole node, never inferred from a partial page.
 */
export type InboxCounts = { 
/**
 * Every current request and outcome, including read ones.
 */
total: InboxCount, 
/**
 * Requests waiting for a person's answer, including read ones.
 */
waiting: InboxCount, 
/**
 * Entries whose attention remains unread.
 */
unread: InboxCount, };

/**
 * A stable position in a node's inbox, never reused after removal.
 */
export type InboxCursor = number;

/**
 * Whether a tool's entire detail fits in an inbox page.
 */
export type InboxDetail = "complete" | "in_session";

/**
 * An entry and the session's current identifying metadata.
 */
export type InboxEntry = { 
/**
 * Page position, local to this node.
 */
cursor: InboxCursor, 
/**
 * Exact source record.
 */
key: InboxKey, 
/**
 * Current session title.
 */
title: SessionTitle | null, 
/**
 * Project, when the session was placed in one.
 */
project: ProjectId | null, 
/**
 * Current project name for a person, when the project still exists.
 */
project_name: ProjectName | null, 
/**
 * When attention was first requested.
 */
at: string, 
/**
 * Shared read state.
 */
read: InboxRead, 
/**
 * Request or outcome.
 */
item: InboxItem, };

/**
 * Whether a get is a cheap projection recheck or recovers one live form.
 */
export type InboxFetch = "summary" | "live_request";

/**
 * What needs attention. Unknown future kinds remain readable, not actionable.
 */
export type InboxItem = { "kind": "permission", 
/**
 * Request the session actor still owns.
 */
request: PermissionRequest, 
/**
 * Last announced tool state, when available.
 */
tool: ToolCall | null, 
/**
 * Large tool bodies stay in the session.
 */
detail: InboxDetail, } | { "kind": "question", 
/**
 * Request the session actor still owns.
 */
question: Question, } | { "kind": "in_session", 
/**
 * The request remains pending, and is never silently dropped.
 */
request: InboxRequest, } | { "kind": "finished" } | { "kind": "failed" } | { "kind": "limit" } | { "kind": "unknown" };

/**
 * The exact source record, also used to open the conversation at that point.
 */
export type InboxKey = { 
/**
 * Session owning the entry.
 */
session: SessionId, 
/**
 * Record that first asked for attention.
 */
seq: EventSeq, };

/**
 * At most sixteen entries, with an exclusive cursor for older entries.
 */
export type InboxPage = { 
/**
 * Whole-node counts, from the same read transaction as this page.
 */
counts: InboxCounts, 
/**
 * Entries, newest first.
 */
entries: Array<InboxEntry>, 
/**
 * Fetch the next page when present.
 */
next: InboxCursor | null, };

/**
 * Shared attention state; reading never resolves a request.
 */
export type InboxRead = "read" | "unread";

/**
 * A request whose unusually large form stays in its session.
 */
export type InboxRequest = { "kind": "permission", 
/**
 * The actor's request id.
 */
id: PermissionId, } | { "kind": "question", 
/**
 * The actor's request id.
 */
id: QuestionId, };

/**
 * Calls to the symbol at a saved source position.
 */
export type IncomingCalls = AtPosition;

/**
 * What progress the server actually reported, never inferred from silence.
 */
export type Indexing = "unknown" | "busy" | "observed_idle";

/**
 * `project.init`: a new repository.
 */
export type InitProject = { 
/**
 * The folder, absolute: absent or empty.
 */
path: string, 
/**
 * What to call it; its folder's name otherwise.
 */
name?: ProjectName | null, };

/**
 * Directory inspection carries the metadata needed by all three sort modes.
 */
export type InspectDirectory = { 
/**
 * Authorized root, resolved again by the destination node.
 */
root: WorkspaceRoot, 
/**
 * Directory below that root.
 */
directory: RelPath, 
/**
 * Explicit ignore policy.
 */
ignored: IgnoredEntries, };

/**
 * One bounded directory snapshot.
 */
export type InspectedDirectory = { 
/**
 * At most the ordinary file-list budget, directories first.
 */
entries: Array<InspectedEntry>, 
/**
 * The directory exceeded the response budget.
 */
more: boolean, };

/**
 * A validated entry with optional, honestly unknown metadata.
 */
export type InspectedEntry = { 
/**
 * Full relative path, including its basename.
 */
path: RelPath, 
/**
 * Links remain links, never falsely advertised as regular files.
 */
kind: EntryKind, 
/**
 * Regular-file size when available.
 */
size: FileSize | null, 
/**
 * Display/order timestamp; absence is not the Unix epoch.
 */
modified: FileModifiedMillis | null, 
/**
 * Actual git classification.
 */
ignored: IgnoreState, };

/**
 * `agent.install`: run the vendor's documented command for `agent`'s
 * `part` (the one `agent.list` shows). Needs `admin`; refused to an
 * agent and to a call from another node.
 */
export type InstallAgent = { 
/**
 * Which of the four.
 */
agent: HarnessKind, 
/**
 * It, or its ACP adapter.
 */
part: InstallPart, };

/**
 * An install a node started (`agent.install`, protocol 11, ADR 0032):
 * minted by the node, kept until the next install or a restart.
 */
export type InstallId = string & { readonly __brand: "InstallId" };

/**
 * An install the node started.
 */
export type InstallJob = { 
/**
 * Its id, for `agent.install_output`.
 */
id: InstallId, 
/**
 * Which of the four.
 */
agent: HarnessKind, 
/**
 * It, or its adapter.
 */
part: InstallPart, 
/**
 * The command, as a person would type it.
 */
command: string, 
/**
 * When it started.
 */
started_at: string, };

/**
 * One install a known agent's vendor documents.
 */
export type InstallOffer = { 
/**
 * What it installs.
 */
part: InstallPart, 
/**
 * The whole command, as a person would type it.
 */
command: string, };

/**
 * A page of an install's output, the shape of `worktree.output`.
 */
export type InstallOutput = { 
/**
 * Where this page starts.
 */
from: number, 
/**
 * Standard output and error together, as printed.
 */
bytes: TerminalBytes, 
/**
 * Where the next page starts.
 */
next: number, 
/**
 * Whether the output reached its limit and the rest was not kept.
 */
truncated: boolean, 
/**
 * The install now.
 */
state: InstallState, };

/**
 * What an install puts in place.
 */
export type InstallPart = "harness" | "adapter";

/**
 * An install's state.
 */
export type InstallState = { "state": "running" } | { "state": "succeeded" } | { "state": "failed", 
/**
 * Its exit code.
 */
code?: number | null, } | { "state": "timed_out" };

/**
 * Whether an agent is installed.
 */
export type Installed = { "state": "found", 
/**
 * Where.
 */
path: string, 
/**
 * Its version: the four's `--version`, an added agent's own word
 * in its handshake (never run with `--version`).
 */
version?: AgentVersion | null, 
/**
 * Whether it runs conversations here.
 */
structured: StructuredSupport, } | { "state": "missing", 
/**
 * The program looked for.
 */
program: AgentProgram, } | { "state": "too_old", 
/**
 * Where.
 */
path: string, 
/**
 * Its version, when known.
 */
version?: AgentVersion | null, 
/**
 * The ACP version it speaks.
 */
speaks: number, 
/**
 * The one Vessel speaks.
 */
needs: number, } | { "state": "too_new", 
/**
 * Where.
 */
path: string, 
/**
 * Its version, when known.
 */
version?: AgentVersion | null, 
/**
 * The ACP version it speaks.
 */
speaks: number, 
/**
 * The one Vessel speaks.
 */
needs: number, } | { "state": "unknown", 
/**
 * Why.
 */
reason: UnknownReason, };

/**
 * Reply to `device.invite`.
 */
export type Invitation = { 
/**
 * The link to hand to the new device.
 */
link: PairingLink, 
/**
 * When it expires.
 */
expires: string, };

/**
 * The single-use secret of a pairing invitation: 128 random bits.
 */
export type InvitationSecret = string;

/**
 * `device.invite` (protocol 4): make a pairing link. Needs `Admin`.
 */
export type InviteDevice = { 
/**
 * Label for the device that redeems it (it may name itself instead).
 */
label?: string | null, 
/**
 * How long the invitation is valid, in seconds (the node caps it).
 */
ttl_secs: number, 
/**
 * What the device may do (protocol 5); `None` is
 * [`Scope::DEVICE_DEFAULT`], what every protocol 4 device got. A fleet
 * member is invited with `[sync]`.
 */
scopes?: Array<Scope> | null, };

/**
 * Join this node to the home named by one single-use invitation.
 */
export type JoinFleet = { 
/**
 * Never logged or returned; the node redeems it on the caller's behalf.
 */
link: PairingLink, 
/**
 * What this node grants its home; never inferred from the invitation.
 */
grant: Array<Scope>, };

/**
 * The authenticated home after the membership was persisted.
 */
export type JoinedFleet = { 
/**
 * The home, without credentials or connection routes.
 */
home: NodeInfo, };

/**
 * The lane a session is in: where it sits between first prompt and merge,
 * and which loop is turning it.
 */
export type KanbanColumn = "building" | "validating" | "needs_review" | "ready" | "archive";

/**
 * Why tidying kept a worktree.
 */
export type KeptReason = { "kind": "uncommitted", 
/**
 * How many.
 */
files: number, } | { "kind": "unique_commits", 
/**
 * How many.
 */
commits: number, } | { "kind": "operation", 
/**
 * Which.
 */
operation: GitOperation, } | { "kind": "submodules" } | { "kind": "flagged_files" } | { "kind": "worktree_refs" } | { "kind": "in_use" } | { "kind": "inspection_failed" } | { "kind": "unknown" };

/**
 * `session.kill`: stop a session's process (and its process group).
 */
export type KillSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * `terminal.kill`: end its process (a service is not started again); the
 * terminal is kept, ended, with its output.
 */
export type KillTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, };

/**
 * A supported server family; JavaScript uses the TypeScript server.
 */
export type Language = "rust" | "type_script" | "python" | "go" | "cpp";

/**
 * Leave this node's fleet. Applied files stay; already left is success.
 */
export type LeaveFleet = Record<symbol, never>;

/**
 * One configured login; an omitted profile means the configured default,
 * unlike usage.query's all-history profile selector.
 */
export type LimitAccount = { 
/**
 * Which agent runs here.
 */
harness: HarnessRef, 
/**
 * Selected configured profile; None is the default.
 */
profile: ProfileName | null, };

/**
 * Reported credits, without assigning a currency or estimating a price.
 */
export type LimitCredits = { 
/**
 * Reported balance, withheld without operate permission.
 */
balance: FiniteNumber | null, 
/**
 * Provider's reported eligibility, never inferred from a balance.
 */
has_credits: boolean | null, 
/**
 * Whether the provider reported unlimited credits.
 */
unlimited: boolean | null, };

/**
 * Safe failure categories; raw credential/API errors never cross the wire.
 */
export type LimitFailure = "no_login" | "expired" | "rejected" | "unreachable" | "changed_response" | "unsupported" | "unsafe_login_store" | "different_route" | "unassociated" | "rate_limited" | "busy";

/**
 * Provider's stable quota identity, bounded to 512 bytes. Punctuation
 * remains significant: model ids `fable-pro` and `fable_pro` differ.
 */
export type LimitKey = string;

/**
 * Saved login source, for a precise credential-use disclosure.
 */
export type LimitLoginStore = "claude_code" | "codex" | "open_code";

/**
 * Provider's human label, bounded to 256 bytes; UI text may wrap it.
 */
export type LimitName = string;

/**
 * Facts a session notice can draw; not proof why a turn stopped.
 */
export type LimitNotice = { 
/**
 * Reported full general windows, identified in the successful snapshot.
 */
reported_full: Array<LimitKey>, 
/**
 * Latest reset only when all selected full windows have future resets.
 */
latest_reported_reset: string | null, };

/**
 * Finite vendor-reported percentage; drawing clamps belong to consumers.
 */
export type LimitPercent = FiniteNumber;

/**
 * Only a reported length identifies a period; slot position never does.
 */
export type LimitPeriod = { "kind": "five_hour" } | { "kind": "weekly" } | { "kind": "other", 
/**
 * Vendor-supplied duration.
 */
duration: DurationMs, } | { "kind": "unknown" };

/**
 * Cached read or explicitly requested vendor refresh.
 */
export type LimitRefresh = "cached" | "refresh";

/**
 * Independent operation a quota applies to; scoped windows are not summed.
 */
export type LimitScope = { "kind": "general" } | { "kind": "model", 
/**
 * Verbatim vendor identity when supplied.
 */
id: LimitKey | null, 
/**
 * Bounded presentation name.
 */
name: LimitName, } | { "kind": "surface", 
/**
 * Verbatim vendor identity when supplied.
 */
id: LimitKey | null, 
/**
 * Bounded presentation name.
 */
name: LimitName, } | { "kind": "feature", 
/**
 * Verbatim feature identity.
 */
id: LimitKey, 
/**
 * Bounded presentation name.
 */
name: LimitName, } | { "kind": "code_review" };

/**
 * Position identifies an unknown window without inventing its duration.
 */
export type LimitSlot = "only" | "primary" | "secondary";

/**
 * Last successful sanitized response; at most 128 windows.
 */
export type LimitSnapshot = { 
/**
 * Observation time stays unchanged on a failed refresh.
 */
at: string, 
/**
 * Recognized vendor subscription label.
 */
plan: LimitName | null, 
/**
 * Independent allowances, never a combined percentage.
 */
windows: Array<LimitWindow>, 
/**
 * Optional provider credit facts.
 */
credits: LimitCredits | null, 
/**
 * Optional paid-use switch reported by Claude.
 */
extra_usage: ExtraUsage | null, 
/**
 * Optional explicit provider permission to continue.
 */
allowed: boolean | null, 
/**
 * Optional explicit provider exhaustion flag.
 */
limit_reached: boolean | null, 
/**
 * Some entries were malformed or exceeded the bounded representation.
 */
incomplete: boolean, };

/**
 * One independent reported allowance.
 */
export type LimitWindow = { 
/**
 * Stable identity in this response.
 */
id: LimitKey, 
/**
 * Only vendor-reported duration is classified.
 */
period: LimitPeriod, 
/**
 * Distinguishes slots when length is missing.
 */
slot: LimitSlot, 
/**
 * Which work it meters.
 */
scope: LimitScope, 
/**
 * None is unknown, distinct from a reported zero.
 */
used: LimitPercent | null, 
/**
 * Only the vendor's reset; no inferred reset from a slot.
 */
resets_at: string | null, };

/**
 * A saved account or the saved login of a session's selected profile.
 * This does not prove what credential an already-running process remembers.
 */
export type LimitsScope = { "kind": "account", 
/**
 * Selected configured binding.
 */
account: LimitAccount, } | { "kind": "session", 
/**
 * Session whose selected profile is inspected.
 */
session: SessionId, };

/**
 * What a line of a hunk is.
 */
export type LineKind = "context" | "added" | "removed" | "no_newline";

/**
 * `agent.list`: every agent the node can run, the four then the added ones
 * by name. Starts no process. Needs `read`.
 */
export type ListAgents = Record<symbol, never>;

/**
 * `git.branches`.
 */
export type ListBranches = { 
/**
 * Where.
 */
root: WorkspaceRoot, };

/**
 * `device.list` (protocol 4): every device, revoked ones included.
 */
export type ListDevices = null;

/**
 * `file.list`: one folder.
 */
export type ListDir = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which folder; the root when empty.
 */
dir: RelPath, 
/**
 * List what git ignores too (marked).
 */
ignored: boolean, };

/**
 * `harness.list`: discover installed harnesses.
 */
export type ListHarnesses = null;

/**
 * A bounded descending page. Omitted cursor starts at the newest entry.
 */
export type ListInbox = { 
/**
 * Exclusive upper bound, returned as `next` by the preceding page.
 */
before: InboxCursor | null, };

/**
 * `fleet.members` (protocol 5): the home's members and what each last
 * reported. Needs `Admin`.
 */
export type ListMembers = null;

/**
 * `model.list` (protocol 7): the routes this node offers, for pickers and
 * `vessel models`. Needs `read`.
 */
export type ListModels = null;

/**
 * `fleet.nodes` (protocol 6): what this node knows of its fleet (a member
 * asks its home with `fleet.peers`). Needs `read`.
 */
export type ListNodes = null;

/**
 * `fleet.peers` (protocol 6): the home's view of its fleet, for its
 * members. Needs `sync`.
 */
export type ListPeers = null;

/**
 * `port.list`: the listeners found, at once or after the next change.
 */
export type ListPorts = { 
/**
 * Wait (at most 30 s) until the `revision` differs from this one.
 */
changed_from?: number | null, };

/**
 * `profile.list` (protocol 5): the node's profiles, the implicit default
 * of each harness included. Needs `Read`.
 */
export type ListProfiles = { 
/**
 * Only this harness's profiles; an added agent needs protocol 11.
 */
harness?: HarnessRef | null, };

/**
 * `project.list`: every project.
 */
export type ListProjects = Record<symbol, never>;

/**
 * `prompt.list`: the saved prompts, by name. Needs `read`.
 */
export type ListPrompts = { 
/**
 * Only this project's and the machine's; every one when absent.
 */
project?: ProjectId | null, };

/**
 * `pr.list`: every watched session's pull request.
 */
export type ListPullRequests = Record<symbol, never>;

/**
 * `schedule.runs`: a schedule's newest runs. Needs `read`.
 */
export type ListRuns = { 
/**
 * Which schedule.
 */
schedule: ScheduleId, };

/**
 * `schedule.list`: every schedule on the node (at most 256). Needs
 * `read`.
 */
export type ListSchedules = Record<symbol, never>;

/**
 * `script.list`: a place's scripts, their approval and their runs.
 */
export type ListScripts = { 
/**
 * Whose checkout.
 */
place: ScriptPlace, };

/**
 * `secret.list` (protocol 6): the names of this node's secrets, never
 * their values. Needs `admin`.
 */
export type ListSecrets = null;

/**
 * `session.list`: preserve the legacy full collection unless a caller
 * explicitly supplies the versioned discovery selection.
 */
export type ListSessions = SessionSelection | null;

/**
 * `worktree.snapshots`: a project's snapshots.
 */
export type ListSnapshots = { 
/**
 * Which project.
 */
project: ProjectId, };

/**
 * `git.stashes`.
 */
export type ListStashes = { 
/**
 * Where.
 */
root: WorkspaceRoot, };

/**
 * `terminal.list`: the node's terminals, or one place's.
 */
export type ListTerminals = { 
/**
 * Only this session's or project's.
 */
place?: TerminalPlace | null, };

/**
 * List snapshots; no secondary client database or path-based identity.
 */
export type ListWorkspaces = { 
/**
 * Narrow to one registered project when provided.
 */
project: ProjectId | null, 
/**
 * Default active only.
 */
lifecycle: WorkspaceList, 
/**
 * Default foreground only.
 */
visibility: WorkspaceVisibilityFilter, };

/**
 * `worktree.list`: every worktree, or one project's.
 */
export type ListWorktrees = { 
/**
 * Only this project's.
 */
project?: ProjectId | null, };

/**
 * A TCP socket in the listening state.
 */
export type ListeningPort = { 
/**
 * The port.
 */
port: number, 
/**
 * The local address it listens on (`127.0.0.1`, `::1`, `0.0.0.0`,
 * `::`): loopback only, or every interface. Text on the wire.
 */
address: string, 
/**
 * The listening process.
 */
pid: number, 
/**
 * Its name, where the system said.
 */
process?: ProcessName | null, 
/**
 * Whose tree it is in.
 */
owner: PortOwner, };

/**
 * A discovery read's inclusion policy, never permission to operate on an id.
 */
export type ListingScope = "shown" | "all";

/**
 * A location relative to the authorized workspace.
 */
export type Location = { 
/**
 * Saved source file.
 */
file: FilePath, 
/**
 * Scalar range.
 */
range: Range, };

/**
 * A call edge anchored at its caller's source sites.
 */
export type LspCall = { 
/**
 * Caller for incoming calls, callee for outgoing calls.
 */
symbol: Symbol, 
/**
 * Authorized call sites, bounded along with all other locations.
 */
sites: Array<Location>, };

/**
 * What a home publishes: every file by its path, and the MCP servers with
 * secret references unresolved.
 */
export type Manifest = { 
/**
 * Which publication this is.
 */
generation: Generation, 
/**
 * The files.
 */
files: { [key in BundlePath]: BundleFile }, 
/**
 * The MCP servers.
 */
mcp: { [key in McpServerName]: McpTransport<ConfigValue> }, };

/**
 * Change shared read attention for exactly one current entry.
 */
export type MarkInbox = { 
/**
 * Exact source, so an old mark never consumes newer attention.
 */
key: InboxKey, 
/**
 * New shared state.
 */
read: InboxRead, };

/**
 * `session.mark`: set a mark. Needs `operate`.
 */
export type MarkSession = { 
/**
 * Which session.
 */
session: SessionId, } & ({ "mark": "pin" } | { "mark": "unpin" } | { "mark": "archive" } | { "mark": "unarchive" } | { "mark": "read", 
/**
 * The next record the reader has not seen.
 */
through: EventSeq, } | { "mark": "unread" });

/**
 * How a bounded hover text is represented.
 */
export type Markup = "plain_text" | "markdown";

/**
 * How many runs a schedule makes before it ends: 1 to 1,000,000.
 */
export type MaxRuns = number;

/**
 * An MCP server's name, unique within a fleet's definitions.
 */
export type McpServerName = string;

/**
 * How a harness reaches an MCP server. `V` is `ConfigValue` in definitions
 * and `ResolvedValue` at launch; only environment variables and headers
 * hold one, so a secret can never become an argument.
 */
export type McpTransport<V> = { "type": "stdio", 
/**
 * The program, found on the launching node's `PATH` if relative.
 */
command: string, 
/**
 * Its arguments, literal only.
 */
args?: Array<string>, 
/**
 * Its environment.
 */
env?: { [key in EnvName]: V }, } | { "type": "http", 
/**
 * Its endpoint.
 */
url: HttpUrl, 
/**
 * Request headers (credentials usually go here).
 */
headers?: { [key in HeaderName]: V }, };

/**
 * Reply to `memory.import`.
 */
export type MemoryImported = { 
/**
 * Files written.
 */
written: number, 
/**
 * Files already there with the same content.
 */
same: number, 
/**
 * Files that differed, written beside the ones there.
 */
beside?: Array<BundlePath>, };

/**
 * A profile's memory on a node of the fleet.
 */
export type MemoryRef = { 
/**
 * The node.
 */
node: NodeId, 
/**
 * Whose memory there; absent is the default profile.
 */
profile?: ProfileName | null, 
/**
 * One project's, by its working directory there; absent is all.
 */
project?: string | null, };

/**
 * How a pull request is merged.
 */
export type MergeMethod = "merge" | "squash" | "rebase";

/**
 * `pr.merge`'s answer.
 */
export type MergeOutcome = { "outcome": "merged", 
/**
 * The commit the merge made on the base.
 */
commit: CommitId, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: MergeRefusal, } | { "outcome": "unknown" };

/**
 * `pr.merge`: merge the session's pull request, as the caller saw it.
 */
export type MergePullRequest = { 
/**
 * The session.
 */
session: SessionId, 
/**
 * The pull request the caller looked at.
 */
number: PullNumber, 
/**
 * The method.
 */
method: MergeMethod, 
/**
 * The head the caller looked at, in full (40 or 64 digits): a push
 * since refuses the merge.
 */
head: CommitId, };

/**
 * Why `pr.merge` merged nothing.
 */
export type MergeRefusal = { "kind": "no_pull_request" } | { "kind": "other_pull_request", 
/**
 * Its number.
 */
number: PullNumber, } | { "kind": "not_open", 
/**
 * What it is.
 */
state: PullState, } | { "kind": "draft" } | { "kind": "head_moved", 
/**
 * The head now.
 */
head: CommitId, } | { "kind": "not_mergeable", 
/**
 * The forge's words, when it gave some.
 */
message?: string | null, } | { "kind": "method_not_allowed" } | { "kind": "merge_queue" } | { "kind": "cannot_pin" } | { "kind": "access", 
/**
 * Why.
 */
access: ForgeAccess, } | { "kind": "rejected", 
/**
 * Its message, at most 2 KiB.
 */
message: string, } | { "kind": "unknown" };

/**
 * Whether it can be merged now.
 */
export type Mergeable = "mergeable" | "conflicting" | "blocked" | "unstable" | "behind" | "unknown";

/**
 * Who said a message.
 */
export type MessageRole = "prompt" | "reply";

/**
 * Every RPC method.
 */
export type Method = "node.info" | "harness.list" | "session.list" | "session.get" | "session.create" | "session.side_open" | "session.side_prompt" | "session.side_reset" | "session.send" | "session.kill" | "session.read" | "session.subscribe" | "session.unsubscribe" | "node.stop" | "device.invite" | "device.self" | "device.list" | "device.revoke" | "profile.list" | "profile.reload" | "session.move_profile" | "fleet.manifest" | "fleet.blob" | "fleet.watch" | "fleet.report" | "fleet.join" | "fleet.leave" | "fleet.sync" | "fleet.status" | "fleet.published" | "fleet.members" | "fleet.forward" | "fleet.relay" | "fleet.deliver" | "fleet.grant" | "fleet.callback" | "fleet.peers" | "fleet.nodes" | "fleet.forget" | "session.export" | "session.import" | "memory.export" | "memory.import" | "transfer.read" | "transfer.close" | "secret.set" | "secret.list" | "secret.delete" | "secret.copy" | "lsp.workspace_symbols" | "lsp.document_symbols" | "lsp.definition" | "lsp.references" | "lsp.hover" | "lsp.implementations" | "lsp.type_definition" | "lsp.incoming_calls" | "lsp.outgoing_calls" | "lsp.diagnostics" | "model.list" | "harness.options" | "session.options" | "session.set_option" | "session.rename" | "session.prompt" | "session.queue" | "session.conversation" | "session.resume" | "session.mark" | "session.delete" | "attachment.begin" | "attachment.write" | "attachment.finish" | "attachment.read" | "project.list" | "project.get" | "project.add" | "project.clone" | "project.init" | "project.remove" | "project.commands" | "project.approve" | "worktree.list" | "worktree.create" | "worktree.output" | "worktree.setup" | "worktree.remove" | "worktree.snapshots" | "worktree.drop_snapshot" | "git.status" | "git.changes" | "git.diff" | "git.show" | "git.branches" | "git.log" | "git.stashes" | "git.stage" | "git.unstage" | "git.discard" | "git.commit" | "git.fetch" | "git.push" | "git.pull" | "git.switch" | "git.rename_branch" | "git.stash" | "file.list" | "file.read" | "file.write" | "file.create_dir" | "file.rename" | "file.delete" | "file.find" | "file.grep" | "forge.status" | "pr.list" | "pr.get" | "pr.open" | "pr.merge" | "pr.feedback" | "terminal.create" | "terminal.list" | "terminal.get" | "terminal.read" | "terminal.subscribe" | "terminal.send" | "terminal.rename" | "terminal.kill" | "terminal.close" | "script.list" | "script.run" | "script.restart" | "port.list" | "agent.list" | "agent.add" | "agent.remove" | "agent.status" | "agent.catalogue" | "agent.install" | "agent.install_output" | "usage.query" | "push.key" | "push.subscribe" | "push.unsubscribe" | "push.presence" | "inbox.list" | "inbox.get" | "inbox.mark" | "usage.limits" | "usage.lookup" | "schedule.preview" | "schedule.list" | "schedule.get" | "schedule.create" | "schedule.update" | "schedule.pause" | "schedule.resume" | "schedule.run" | "schedule.delete" | "schedule.runs" | "prompt.list" | "prompt.create" | "prompt.update" | "prompt.delete" | "session.search" | "session.park" | "session.wake" | "node.settings" | "node.set_settings" | "session.fork" | "session.rewind" | "agent.history" | "session.detach" | "workspace.registry" | "workspace.list" | "workspace.get" | "workspace.rename" | "workspace.pin" | "workspace.release_automation" | "git.compare" | "git.compare_file" | "node.diagnostics" | "file.inspect" | "file.read_bytes" | "file.duplicate" | "file.observe" | "file.suggest" | "skills.status" | "skills.reconcile" | "skills.uninstall" | "skills.save_selection" | "skills.import_selection";

/**
 * A media type without parameters, `type/subtype`, lower case
 * (`image/png`, `application/pdf`).
 */
export type MimeType = string;

/**
 * A file mode change (`100644` to `100755`, say), as git prints modes.
 */
export type ModeChange = { 
/**
 * Before, in octal digits read as a number.
 */
old: number, 
/**
 * After.
 */
new: number, };

/**
 * An API a provider or a harness speaks.
 */
export type ModelApi = "anthropic_messages" | "openai_responses" | "openai_chat_completions";

/**
 * One call a session made to a model through the node's proxy: metadata
 * only, never prompt or completion content, tool arguments or keys.
 */
export type ModelCall = { 
/**
 * The route the session runs on.
 */
route: ModelRouteName, 
/**
 * The provider's model that was called.
 */
model: ModelId, 
/**
 * The API the provider speaks.
 */
api: ModelApi, 
/**
 * The tokens it used (zero where the provider said nothing).
 */
usage: TokenUsage, 
/**
 * How it ended.
 */
stop: ModelStop, 
/**
 * Why it failed, when it did.
 */
failure?: ModelFailure | null, 
/**
 * From the proxy's first byte out to the answer's end.
 */
duration_ms: DurationMs, };

/**
 * Why a model call failed: its class, never the provider's message.
 */
export type ModelFailure = "rate_limited" | "overloaded" | "upstream_error" | "refused_request" | "context_too_long" | "timeout" | "truncated" | "unauthorized";

/**
 * A model name as the harness understands it (`opus`, `gpt-5`, ...).
 * The set is open and harness-specific, so it is not an enum.
 */
export type ModelId = string;

/**
 * A route a node offers (`model.list`, protocol 7).
 */
export type ModelRoute = { 
/**
 * Its name, which profiles use.
 */
name: ModelRouteName, 
/**
 * The provider it calls.
 */
provider: ProviderName, 
/**
 * The API that provider speaks.
 */
api: ModelApi, 
/**
 * The provider's model.
 */
model: ModelId, 
/**
 * The provider's model for a harness's small, fast calls.
 */
small_model?: ModelId | null, };

/**
 * A model route's name in a node's `models.toml`, which profiles name
 * with `model = "..."` (protocol 7).
 */
export type ModelRouteName = string;

/**
 * How a model call ended.
 */
export type ModelStop = "end_turn" | "tool_use" | "max_tokens" | "stop_sequence" | "refusal" | "pause" | "error";

/**
 * `session.move_profile` (protocol 5, ADR 0011): restart a structured
 * session's harness under another profile, resuming the same conversation.
 * Refused mid-turn unless `force`, and refused when the target profile is
 * not logged in. Answers with the session as it is now. Needs `Operate`.
 */
export type MoveProfile = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * The profile to move to; `None` is the harness's default.
 */
profile?: ProfileName | null, 
/**
 * Move even in the middle of a turn, cancelling it.
 */
force?: boolean, };

/**
 * What `fleet.forget` did.
 */
export type NodeForgotten = { 
/**
 * The key the node was bound to.
 */
key: NodePublicKey, 
/**
 * The name it was known by, now released.
 */
name?: string | null, 
/**
 * The member devices that granted for it; their grants are deleted.
 * One that should not sync any more is revoked separately.
 */
devices: Array<DeviceId>, 
/**
 * Invitations that were outstanding on the home and are withdrawn.
 */
withdrawn: number, };

/**
 * Identity of a Vessel node; one per machine, stable across restarts.
 */
export type NodeId = string & { readonly __brand: "NodeId" };

/**
 * Facts about a node.
 */
export type NodeInfo = { 
/**
 * Stable id for this node.
 */
id: NodeId, 
/**
 * Human-readable name (defaults to the hostname).
 */
name: string, 
/**
 * OS hostname.
 */
hostname: string, 
/**
 * `std::env::consts::OS`.
 */
os: string, 
/**
 * `std::env::consts::ARCH`.
 */
arch: string, 
/**
 * Version of the node binary.
 */
version: string, 
/**
 * Protocols the node speaks.
 */
protocol: ProtocolRange, 
/**
 * When the node process started.
 */
started_at: string, 
/**
 * Whether it takes new sessions (protocol 3; absent means serving).
 */
state: NodeState, 
/**
 * The node's static key for secure channels (protocol 4; absent from
 * older nodes and from nodes without a key yet).
 */
public_key?: NodePublicKey | null, 
/**
 * System IANA time zone, detected at start (protocol 13). Absent on
 * older nodes or when the system cannot supply an IANA name.
 */
timezone?: TimeZoneName | null, };

/**
 * A node's static X25519 public key: the identity a client pins at pairing
 * and checks on every secure handshake.
 */
export type NodePublicKey = string & { readonly __brand: "NodePublicKey" };

/**
 * The node's settings in force.
 */
export type NodeSettings = { 
/**
 * How long a structured session may stay idle before its agent is
 * parked.
 */
park_after: ParkAfter, 
/**
 * Which protocol drives each agent's new sessions (protocol 18);
 * absent when every agent runs over ACP.
 */
drivers?: AgentDrivers, 
/**
 * Startup policy, hidden until its protocol is assigned (ADR 0059).
 * This value-only field does not enable an engine or register input.
 */
restart_recovery?: RestartRecovery, };

/**
 * Whether a node is taking new work.
 */
export type NodeState = "serving" | "draining";

/**
 * Observe until a change notification or a finite heartbeat deadline.
 */
export type ObserveFile = { 
/**
 * Authorized workspace selection.
 */
root: WorkspaceRoot, 
/**
 * File path, never the workspace root.
 */
path: RelPath, 
/**
 * The last state the client received; absent for an initial snapshot.
 */
after: ObservedFile | null, };

/**
 * What an observed saved file currently contains.
 */
export type ObservedFile = { "kind": "ready", 
/**
 * The source revision.
 */
revision: FileRevision, } | { "kind": "missing" };

/**
 * The pairing-link format version. Only version 1 exists; a link from a
 * newer format is refused with a clear error rather than half-understood.
 */
export type OfferVersion = 1;

/**
 * A best-effort offset into sorted results; edits may shift later pages.
 */
export type Offset = number;

/**
 * `pr.open`'s answer.
 */
export type OpenOutcome = { "outcome": "opened", 
/**
 * It.
 */
pull_request: PullRequest, } | { "outcome": "exists", 
/**
 * It.
 */
pull_request: PullRequest, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: OpenRefusal, } | { "outcome": "unknown" };

/**
 * `pr.open`: open a pull request from the branch a session's folder is
 * on.
 */
export type OpenPullRequest = { 
/**
 * The session.
 */
session: SessionId, 
/**
 * Its title.
 */
title: PullTitle, 
/**
 * Its description.
 */
body?: PullBody | null, 
/**
 * The branch it merges into; the project's default branch when
 * absent.
 */
base?: BranchName | null, 
/**
 * Open it as a draft.
 */
draft: boolean, };

/**
 * Why `pr.open` made nothing.
 */
export type OpenRefusal = { "kind": "not_pushed" } | { "kind": "no_commits" } | { "kind": "push", 
/**
 * Why.
 */
refusal: GitRefusal, } | { "kind": "access", 
/**
 * Why.
 */
access: ForgeAccess, } | { "kind": "rejected", 
/**
 * Its message, at most 2 KiB.
 */
message: string, } | { "kind": "unknown" };

/**
 * Open or find the newest unarchived side conversation. Needs read and
 * operate; only people, including delegated people, may call it.
 */
export type OpenSideQuestion = { 
/**
 * The original structured session, never another side.
 */
session: SessionId, };

/**
 * What an option is about, as the agent labels it. For placing a control
 * (the composer's model, effort and permissions), never for deciding what
 * a value means: the values are the agent's.
 */
export type OptionCategory = "mode" | "model" | "model_config" | "thought_level" | "other";

/**
 * A client's choice for one option: at creation
 * (`SessionMode::Structured.options`) or with `session.set_option`. A
 * choice the agent does not offer (an unknown option, a value not in the
 * list, a toggle for a select) fails, at creation the whole session, as an
 * unknown model does.
 */
export type OptionChoice = { 
/**
 * Which option.
 */
id: OptionId, } & ({ "kind": "select", 
/**
 * The value.
 */
value: OptionValueId, } | { "kind": "toggle", 
/**
 * Whether to turn it on.
 */
on: boolean, });

/**
 * A session config option's id as the agent names it (ACP's
 * `configId`, for example `model` or `mode`; protocol 9). Echoed back
 * verbatim when a client changes the option.
 */
export type OptionId = string & { readonly __brand: "OptionId" };

/**
 * One value a select option offers.
 */
export type OptionValue = { 
/**
 * The value's id, sent back in an [`OptionChoice`].
 */
id: OptionValueId, 
/**
 * Its label, in the agent's words.
 */
name: string, 
/**
 * A longer explanation, when the agent gives one.
 */
description?: string | null, 
/**
 * The name of the group the agent files it under, when it groups its
 * values. Values of one group are adjacent.
 */
group?: string | null, };

/**
 * One value of a select option, as the agent names it (ACP's `value`;
 * protocol 9). Echoed back verbatim when a client picks it.
 */
export type OptionValueId = string & { readonly __brand: "OptionValueId" };

/**
 * Coverage of optional fields, independent of whether a turn supplied
 * its mandatory input/output counters. Zero remains a reported value.
 */
export type OptionalUsageReports = { 
/**
 * Turns reporting reasoning tokens.
 */
thought: ReportCount, 
/**
 * Turns reporting cache reads.
 */
cached_read: ReportCount, 
/**
 * Turns reporting cache writes.
 */
cached_write: ReportCount, };

/**
 * Who a forwarded call speaks for, as the node it started on vouches.
 */
export type Origin = { 
/**
 * The node the call started on.
 */
node: NodeId, 
/**
 * The principal there.
 */
by: OriginBy, 
/**
 * What that principal holds on its own node. The target never grants
 * more than these, whatever its own grant to the delivering device.
 */
scopes: Array<Scope>, };

/**
 * The principal a call started with, on its own node.
 */
export type OriginBy = { "kind": "owner" } | { "kind": "device", 
/**
 * Its id on that node.
 */
id: DeviceId, } | { "kind": "agent", 
/**
 * The agent's session.
 */
session: SessionId, 
/**
 * How many agents deep that session is (the owner's own sessions
 * are 0), so the target can refuse a session past the depth limit.
 */
depth: number, };

/**
 * Calls from the symbol at a saved source position.
 */
export type OutgoingCalls = AtPosition;

/**
 * A bounded page, deterministically sorted before offset selection.
 */
export type Page<T> = { 
/**
 * At most the requested number of items, capped by the node.
 */
items: Array<T>, 
/**
 * Next offset within the collected result, if more remain.
 */
next: Offset | null, 
/**
 * True if item/byte budgets cut off some server data.
 */
truncated: boolean, 
/**
 * Coverage, freshness and observed indexing progress.
 */
context: Context, };

/**
 * The requested page. Limits are capped at 200 by the node.
 */
export type PageRequest = { 
/**
 * Zero-based result offset.
 */
offset: Offset, 
/**
 * Items requested (default 50, capped at 200).
 */
limit: number, };

/**
 * A pairing link: `vessel://pair#` followed by the base64url JSON of a
 * [`PairingOffer`]. The offer sits after `#` so that, if a link is ever
 * opened through a web page, the fragment never reaches a server.
 */
export type PairingLink = string;

/**
 * Everything a client needs to pair with a node, carried by a pairing link.
 *
 * Unknown fields are refused (ADR 0010): an offer is security-relevant, so a
 * field this version does not understand must not be silently ignored;
 * format changes bump `v` instead.
 */
export type PairingOffer = { 
/**
 * Format version (always 1).
 */
v: OfferVersion, 
/**
 * The node's id (pinned like an SSH node's).
 */
node: NodeId, 
/**
 * The node's static key (pinned; the handshake checks it).
 */
key: NodePublicKey, 
/**
 * Relays the node is registered with, in preference order.
 */
relays: Array<RouteUrl>, 
/**
 * Direct addresses the node was told to advertise (`--public`).
 */
direct: Array<RouteUrl>, 
/**
 * The single-use invitation.
 */
invite: InvitationSecret, 
/**
 * When the invitation expires.
 */
expires: string, };

/**
 * How long a structured session may stay idle before its agent is parked:
 * a node setting (protocol 12, `node.settings`, ADR 0038).
 */
export type ParkAfter = { "kind": "never" } | { "kind": "idle", 
/**
 * Minutes, 1 to [`IdleMinutes::MAX`].
 */
minutes: IdleMinutes, } | { "kind": "unknown" };

/**
 * Why a session's agent is parked.
 */
export type ParkReason = { "reason": "idle" } | { "reason": "asked" } | { "reason": "wake_failed", 
/**
 * Why, as the agent or the node said it (at most
 * [`ParkReason::MESSAGE_BYTES`]).
 */
message: string, } | { "reason": "unknown" };

/**
 * `session.park`: stop an idle session's agent now, keeping the session
 * and its conversation. Needs `operate`.
 */
export type ParkSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * A parked session's agent: since when, and why.
 */
export type Parked = { 
/**
 * When it was parked (or its last wake failed).
 */
since: string, 
/**
 * Why.
 */
reason: ParkReason, };

/**
 * Which files a call acts on.
 */
export type PathSet = { "kind": "all" } | { "kind": "paths", 
/**
 * The paths.
 */
paths: Array<RelPath>, };

/**
 * `schedule.pause`: stop a schedule's runs. Needs `operate`.
 */
export type PauseSchedule = { 
/**
 * Which.
 */
schedule: ScheduleId, };

/**
 * A pending permission request inside a structured session. Minted by the
 * node (not the harness) so many clients can answer the same request and
 * the first answer wins.
 */
export type PermissionId = string & { readonly __brand: "PermissionId" };

/**
 * One choice offered by a permission request.
 */
export type PermissionOption = { 
/**
 * Harness-assigned option id, echoed back when chosen.
 */
id: PermissionOptionId, 
/**
 * Label to show.
 */
name: string, 
/**
 * Meaning.
 */
kind: PermissionOptionKind, };

/**
 * A permission option id offered by the harness (for example
 * `allow-once`). Echoed back verbatim when a client picks it.
 */
export type PermissionOptionId = string & { readonly __brand: "PermissionOptionId" };

/**
 * What choosing an option means.
 */
export type PermissionOptionKind = "allow_once" | "allow_always" | "reject_once" | "reject_always";

/**
 * How a permission request was settled.
 */
export type PermissionOutcome = { "outcome": "selected", 
/**
 * The chosen option.
 */
option: PermissionOptionId, } | { "outcome": "cancelled" };

/**
 * A request for human approval, pending until a client answers.
 */
export type PermissionRequest = { 
/**
 * Node-assigned id clients answer with.
 */
id: PermissionId, 
/**
 * The tool call needing approval.
 */
tool_call: ToolCallId, 
/**
 * Choices, in the harness's order.
 */
options: Array<PermissionOption>, };

/**
 * Explicit pin mutation cannot accidentally target all workspaces sharing a cwd.
 */
export type PinWorkspace = { 
/**
 * Exact host-local identity.
 */
workspace: WorkspaceId, 
/**
 * Explicit desired state, including unpin.
 */
pin: WorkspacePin, };

/**
 * One step of the agent's plan.
 */
export type PlanEntry = { 
/**
 * What the step is.
 */
content: string, 
/**
 * Importance.
 */
priority: PlanPriority, 
/**
 * Progress.
 */
status: PlanStatus, };

/**
 * Priority of a plan entry.
 */
export type PlanPriority = "high" | "medium" | "low";

/**
 * Progress of a plan entry.
 */
export type PlanStatus = "pending" | "in_progress" | "completed";

/**
 * `port.list`'s answer.
 */
export type PortList = { 
/**
 * The listeners the caller may see, by owner, then port.
 */
ports: Array<ListeningPort>, 
/**
 * How the last scan went.
 */
scan: PortScan, 
/**
 * Changes when the list or the scan's state does.
 */
revision: number, 
/**
 * Whether there were more than 256.
 */
more: boolean, };

/**
 * Whose process tree a listener is in.
 */
export type PortOwner = { "kind": "session", 
/**
 * Which.
 */
session: SessionId, } | { "kind": "terminal", 
/**
 * Which.
 */
terminal: TerminalId, };

/**
 * An inclusive service port range whose order cannot be bypassed by serde.
 */
export type PortRange = string;

/**
 * How the last scan went.
 */
export type PortScan = { "state": "pending" } | { "state": "scanned", 
/**
 * When.
 */
at: string, } | { "state": "unavailable", 
/**
 * Why.
 */
reason: string, } | { "state": "unknown" };

/**
 * A literal program selection whose path shape is validated before approval.
 */
export type PortScript = string;

/**
 * A 1-based Unicode scalar position; the line's exclusive end is valid.
 */
export type Position = { 
/**
 * One-based line.
 */
line: number, 
/**
 * One-based Unicode scalar column, not a byte or UTF-16 offset.
 */
column: number, };

/**
 * How many next runs to compute: 1 to 10, default 5.
 */
export type PreviewCount = number;

/**
 * `schedule.preview`: compute without saving or starting anything.
 */
export type PreviewSchedule = { 
/**
 * When it runs. An interval without an anchor is anchored at `after`.
 */
cadence: Cadence, 
/**
 * Strictly after this instant; absent uses one node clock reading.
 */
after?: string | null, 
/**
 * At most ten runs; absent means five.
 */
count: PreviewCount, };

/**
 * What the system calls a process (`comm` on Linux, `lsof`'s command on
 * macOS, the image name on Windows): at most 64 characters, no control
 * character. Content a person reads, kept plain.
 */
export type ProcessName = string;

/**
 * A profile as a node reports it. Names of the variables it sets are
 * listed; their values never leave the node (they may be secrets or paths
 * to credentials).
 */
export type ProfileInfo = { 
/**
 * The profile's name.
 */
name: ProfileName, 
/**
 * A label for people (for example the account it logs in as).
 */
label?: string | null, 
/**
 * The harness it applies to: one of the four, or (protocol 11) an
 * agent added on the node.
 */
harness: HarnessRef, 
/**
 * Names of the environment variables it sets.
 */
sets: Array<EnvName>, 
/**
 * The model route it names (`model = "..."` in `profiles.toml`,
 * protocol 7): its sessions' harness reaches that route's model
 * through the node's proxy. Absent for a profile on the harness's own
 * provider, and from older nodes.
 */
model?: ModelRouteName | null, };

/**
 * A profile's name: what `--profile` and `profiles.toml` use.
 */
export type ProfileName = string;

/**
 * One shell command from a project's file (`vessel.toml`).
 */
export type ProjectCommand = string;

/**
 * The commands a project's file (`vessel.toml`) names, and whether a
 * person approved that set (ADR 0027).
 */
export type ProjectCommands = { "state": "none" } | { "state": "listed", 
/**
 * Run when a worktree is created.
 */
setup: Array<ProjectCommand>, 
/**
 * Run before a worktree is removed.
 */
teardown: Array<ProjectCommand>, 
/**
 * What approving this set approves.
 */
digest: CommandsDigest, 
/**
 * Whether a person approved it for this project.
 */
approved: boolean, 
/**
 * Its scripts and services, in name order (protocol 11, ADR
 * 0031); the digest covers them. Left out when there are none.
 */
scripts?: Array<ScriptEntry>, 
/**
 * Terminal commands the client must show before approval (ADR 0066).
 */
terminals?: Array<FileTerminal>, 
/**
 * Allocation choices that must be displayed or refused before approval.
 */
service_ports?: ServicePorts | null, 
/**
 * The set last approved for this project, to show what changed
 * (protocol 11): only while this set is not approved, and only
 * when the node kept what that approval covered (approvals given
 * from protocol 11 on). Bounded as this set is.
 */
last_approved?: ApprovedCommands | null, } | { "state": "invalid", 
/**
 * Why.
 */
message: string, };

/**
 * A project: a folder the node works in (protocol 10, ADR 0027).
 */
export type ProjectId = string & { readonly __brand: "ProjectId" };

/**
 * A folder the node works in.
 */
export type ProjectInfo = { 
/**
 * Its id.
 */
id: ProjectId, 
/**
 * What it is called (its folder's name unless given one).
 */
name: ProjectName, 
/**
 * The folder on the node, absolute: a repository's top level when the
 * folder added was inside one.
 */
root: string, 
/**
 * Whether it is a git repository. A folder that is not has files but
 * no git and no worktrees.
 */
git: boolean, 
/**
 * Where new worktrees start: the remote-tracking branch `origin/HEAD`
 * names (`origin/main`), else a local `main`, else `master`, else the
 * branch the folder has checked out.
 */
default_branch?: RefName | null, 
/**
 * Ready, or being cloned.
 */
state: ProjectState, 
/**
 * When it was added.
 */
created_at: string, 
/**
 * Grows by one at every change (a clone's progress, its end), so a
 * client can wait for the next (`project.get` with `after`).
 */
revision: number, };

/**
 * The projects, and what git the node found.
 */
export type ProjectList = { 
/**
 * Whether the git calls can work on this node.
 */
git: GitSupport, 
/**
 * Every project, oldest first, at most 1,000.
 */
projects: Array<ProjectInfo>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * What a project is called on the page.
 */
export type ProjectName = string;

/**
 * Whether a project can be used yet.
 */
export type ProjectState = { "state": "ready" } | { "state": "cloning", 
/**
 * What git is doing.
 */
stage: CloneStage, 
/**
 * How far along that stage is, when git says.
 */
percent?: number | null, } | { "state": "clone_failed", 
/**
 * Why, for a client to act on.
 */
reason: CloneFailure, 
/**
 * git's last lines, credentials removed, for a person.
 */
message: string, } | { "state": "missing" } | { "state": "unknown" };

/**
 * Reply to `prompt.list`.
 */
export type PromptList = { 
/**
 * The prompts, by name.
 */
prompts: Array<SavedPrompt>, };

/**
 * What a saved prompt is called; unique within its project (or the
 * machine's own).
 */
export type PromptName = string;

/**
 * A session's queue.
 */
export type PromptQueue = { 
/**
 * The prompts waiting, next first.
 */
entries: Array<QueuedPrompt>, 
/**
 * Why it does not move on its own; absent while it does.
 */
held?: QueueHold | null, };

/**
 * `session.prompt`: send a prompt, delivered as asked. Needs `operate`.
 */
export type PromptSession = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * The message.
 */
content: Array<ContentBlock>, 
/**
 * How it goes.
 */
delivery: Delivery, 
/**
 * Tell the caller's own session, through its queue, when the turn
 * this prompt starts waits for an answer and when it finishes, or
 * the session ends (protocol 11, ADR 0033: the starter told). Only a
 * session's agent asks, on its own node. Left out when false.
 */
notify?: boolean, };

/**
 * Ask in a side conversation with a fresh bounded parent snapshot.
 * General prompt and queue ingress refuses sides to preserve this rule.
 */
export type PromptSideQuestion = { 
/**
 * The side's id, not its parent's.
 */
session: SessionId, 
/**
 * A nonblank question of at most 16 KiB.
 */
question: PromptText, };

/**
 * The text a schedule, a saved prompt or a hook sends.
 */
export type PromptText = string;

/**
 * An inclusive range of protocol versions a peer can speak.
 *
 * [Decision Log]
 * - Purpose: fleet nodes, the CLI and future editor plugins will run
 *   different versions at the same time.
 * - Alternatives: exact-match versions (Zed's collab server answers 426 on
 *   any mismatch, workable only because Zed ships client and server
 *   together); no envelope version at all (Paseo, which now carries hundreds
 *   of per-field compatibility shims).
 * - Choice: both sides state a range in the handshake and use the highest
 *   common version, or refuse with `ErrorCode::ProtocolMismatch`.
 */
export type ProtocolRange = { 
/**
 * Oldest supported version.
 */
min: ProtocolVersion, 
/**
 * Newest supported version.
 */
max: ProtocolVersion, };

/**
 * A wire protocol version.
 */
export type ProtocolVersion = number;

/**
 * A model provider's name in a node's `models.toml` (protocol 7).
 */
export type ProviderName = string;

/**
 * A bundle by name: no file contents, no secret values, no commands.
 */
export type PublishedBundle = { 
/**
 * Its version.
 */
generation: Generation, 
/**
 * Every file, by path in the bundle's order.
 */
files: Array<SharedFile>, 
/**
 * The tool (MCP) servers, by name.
 */
servers: Array<McpServerName>, 
/**
 * The secrets those servers name, each once, sorted: names only; each
 * machine resolves them from its own store.
 */
secrets: Array<SecretName>, };

/**
 * `git.pull`: fast-forward from the upstream.
 */
export type Pull = { 
/**
 * Where.
 */
root: WorkspaceRoot, };

/**
 * A pull request's description.
 */
export type PullBody = string;

/**
 * A pull request's number in its repository (GitLab's merge request
 * `iid`): 1 to 2^53 − 1, what a page's JavaScript holds exactly.
 */
export type PullNumber = number;

/**
 * `git.pull`'s answer.
 */
export type PullOutcome = { "outcome": "pulled", 
/**
 * The commit before.
 */
from: CommitId, 
/**
 * The commit now.
 */
to: CommitId, } | { "outcome": "up_to_date" } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * A pull request, whole: what `pr.get` answers.
 */
export type PullRequest = { 
/**
 * What a card shows.
 */
summary: PullSummary, 
/**
 * Who opened it, when the forge named a login.
 */
author?: ForgeLogin | null, 
/**
 * Its head branch.
 */
head_branch: BranchName, 
/**
 * The branch it merges into.
 */
base: BranchName, 
/**
 * Its head commit's checks.
 */
checks: Checks, 
/**
 * People's reviews.
 */
reviews: Reviews, 
/**
 * When it was opened.
 */
created_at: string, 
/**
 * When it was merged.
 */
merged_at?: string | null, 
/**
 * When it was closed (merged or not).
 */
closed_at?: string | null, 
/**
 * The methods its repository allows.
 */
merge_methods: Array<MergeMethod>, 
/**
 * When this node last read it.
 */
read_at: string, };

/**
 * `pr.list`'s answer.
 */
export type PullRequestList = { 
/**
 * Every watched (live) session's, at most the watch limit.
 */
entries: Array<SessionPullEntry>, };

/**
 * `pr.get`'s answer: the pull request whole.
 */
export type PullRequestView = { 
/**
 * Whether the node can see it.
 */
access: ForgeAccess, 
/**
 * The last one read.
 */
pull_request?: PullRequest | null, 
/**
 * Whether its failing checks and reviews are sent to the session.
 */
feedback: boolean, };

/**
 * Where a pull request is in its life.
 */
export type PullState = "open" | "merged" | "closed" | "unknown";

/**
 * What a card needs of a pull request; carried in `SessionInfo`.
 */
export type PullSummary = { 
/**
 * Its repository.
 */
repo: RepoRef, 
/**
 * Its number there.
 */
number: PullNumber, 
/**
 * Its page.
 */
url: ForgeUrl, 
/**
 * Its title.
 */
title: PullTitle, 
/**
 * Open, merged or closed.
 */
state: PullState, 
/**
 * A draft.
 */
draft: boolean, 
/**
 * The commit its head is at.
 */
head_commit: CommitId, 
/**
 * Its checks, together.
 */
checks: ChecksState, 
/**
 * The forge's review verdict.
 */
review: ReviewDecision, 
/**
 * Whether it can merge now.
 */
mergeable: Mergeable, 
/**
 * When it last changed on the forge.
 */
updated_at: string, };

/**
 * A pull request's title: one line.
 */
export type PullTitle = string;

/**
 * `git.push`.
 */
export type Push = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * To this remote (`origin` when absent) instead of the upstream.
 */
remote?: RemoteName | null, 
/**
 * Make the pushed branch the upstream.
 */
set_upstream: boolean, };

/**
 * Whether this report corresponds to a person's input.
 */
export type PushActivity = "active" | "unchanged";

/**
 * Web Push authentication material; Debug cannot print its secret bytes.
 */
export type PushAuth = string;

/**
 * A browser-issued HTTPS capability. Never log or navigate to it.
 */
export type PushEndpoint = string;

/**
 * What the person is looking at, without disclosing any content.
 */
export type PushFocus = { "focus": "session", 
/**
 * Session this device is viewing.
 */
session: SessionId, } | { "focus": "elsewhere" } | { "focus": "background" };

/**
 * The public identity passed to the browser's pushManager.subscribe.
 */
export type PushKey = { 
/**
 * Uncompressed P-256 point, base64url without padding.
 */
key: PushPublicKey, };

/**
 * Confirmation of the node's lease. Renew by subscribing again.
 */
export type PushLease = { 
/**
 * Subscription stops receiving after this time, even across restart.
 */
expires_at: string, };

/**
 * `git.push`'s answer.
 */
export type PushOutcome = { "outcome": "pushed", 
/**
 * Where to.
 */
remote: RemoteName, 
/**
 * Which branch.
 */
branch: BranchName, 
/**
 * Whether the push set the branch's upstream.
 */
upstream_set: boolean, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * The device's encryption point, or the node's public VAPID identity.
 */
export type PushPublicKey = string;

/**
 * A browser's subscription, owned by the authenticated paired device.
 */
export type PushSubscription = { 
/**
 * HTTPS service capability, never a URL to navigate to or log.
 */
endpoint: PushEndpoint, 
/**
 * Device's uncompressed P-256 encryption public key.
 */
p256dh: PushPublicKey, 
/**
 * Device's authentication secret for payload key derivation.
 */
auth: PushAuth, };

/**
 * `usage.query` (protocol 21). Totals cover the selected range; `after`
 * pages only the individual session samples, never the total.
 */
export type QueryUsage = { 
/**
 * What is counted.
 */
scope: UsageScope, 
/**
 * Selected rolling interval.
 */
period: UsagePeriod, 
/**
 * Exclusive sequence cursor; only valid for a session scope.
 */
after?: EventSeq | null, };

/**
 * A question waiting for an answer.
 */
export type Question = { 
/**
 * Node-assigned id clients answer with.
 */
id: QuestionId, 
/**
 * What the agent asks (cut at [`Question::MAX_MESSAGE`] bytes).
 */
message: string, 
/**
 * The tool call it belongs to, when the agent says.
 */
tool_call?: ToolCallId | null, 
/**
 * The form, in the agent's order.
 */
fields: Array<QuestionField>, };

/**
 * An answer to a question (`session.send`'s `answer_question`).
 */
export type QuestionAnswer = { "answer": "accept", 
/**
 * Each answered field's value.
 */
values: { [key in FieldName]: AnswerValue }, } | { "answer": "decline" };

/**
 * One field of a question's form.
 */
export type QuestionField = { 
/**
 * Its key, echoed back in the answer.
 */
name: FieldName, 
/**
 * Its label.
 */
title?: string | null, 
/**
 * A longer explanation.
 */
description?: string | null, 
/**
 * Whether an accepting answer must give it.
 */
required: boolean, } & ({ "kind": "text", 
/**
 * Fewest characters.
 */
min_length?: number | null, 
/**
 * Most characters.
 */
max_length?: number | null, 
/**
 * A shape the text must have.
 */
format?: TextFormat | null, 
/**
 * A regular expression the text must match, as the agent wrote it.
 * Passed on for a client to check; the node does not (it would
 * need a regular-expression engine for one field kind), so the
 * agent may still refuse an answer that misses it.
 */
pattern?: string | null, 
/**
 * What to start with.
 */
default?: string | null, } | { "kind": "number", 
/**
 * Smallest allowed.
 */
minimum?: FiniteNumber | null, 
/**
 * Largest allowed.
 */
maximum?: FiniteNumber | null, 
/**
 * What to start with.
 */
default?: FiniteNumber | null, } | { "kind": "integer", 
/**
 * Smallest allowed.
 */
minimum?: number | null, 
/**
 * Largest allowed.
 */
maximum?: number | null, 
/**
 * What to start with.
 */
default?: number | null, } | { "kind": "boolean", 
/**
 * What to start with.
 */
default?: boolean | null, } | { "kind": "choice", 
/**
 * The options, in the agent's order.
 */
options: Array<ChoiceOption>, 
/**
 * Several may be chosen.
 */
multiple: boolean, 
/**
 * Fewest to choose (several).
 */
min_items?: number | null, 
/**
 * Most to choose (several).
 */
max_items?: number | null, 
/**
 * What is chosen to start with: one value for a choice of one,
 * any number for several.
 */
default?: Array<string>, } | { "kind": "unknown" });

/**
 * A question an agent asks (protocol 10). Minted by the node, as a
 * permission request's id is, so many clients can answer and the first
 * answer wins.
 */
export type QuestionId = string & { readonly __brand: "QuestionId" };

/**
 * How a question was settled.
 */
export type QuestionOutcome = { "outcome": "answered", 
/**
 * Each answered field's value.
 */
values: { [key in FieldName]: AnswerValue }, } | { "outcome": "declined" } | { "outcome": "cancelled" };

/**
 * Reply to `session.queue`.
 */
export type QueueChanged = { 
/**
 * The queue afterwards.
 */
queue: PromptQueue, 
/**
 * For `send_now` and `resume`: what became of the entry sent, if one
 * was.
 */
delivered?: Delivered | null, };

/**
 * A prompt waiting in a session's queue (protocol 10), minted by the
 * node when it is queued.
 */
export type QueueEntryId = string & { readonly __brand: "QueueEntryId" };

/**
 * Why the queue does not move on its own.
 */
export type QueueHold = "cancelled" | "failed" | "ended" | "asked";

/**
 * A prompt waiting for its turn.
 */
export type QueuedPrompt = { 
/**
 * The entry's id.
 */
id: QueueEntryId, 
/**
 * What will be sent.
 */
content: Array<ContentBlock>, 
/**
 * When it was queued (or last edited).
 */
queued_at: string, 
/**
 * Raised by each edit; an edit names the revision it was made from,
 * so two people editing one entry cannot overwrite each other unseen
 * (agent-orchestrator checks the same).
 */
revision: number, 
/**
 * Who queued it.
 */
by: Creator, };

/**
 * An exclusive-end source range.
 */
export type Range = { 
/**
 * Inclusive start.
 */
start: Position, 
/**
 * Exclusive end.
 */
end: Position, };

/**
 * A JSON value carried as text and never inspected.
 *
 * In TypeScript it is `unknown` (ADR 0017): the one type of the wire with
 * no shape to declare. A page narrows it before use, as Vessel would have
 * to define a typed struct.
 */
export type RawJson = unknown;

/**
 * `attachment.read`: a piece of a session's attachment, from `offset`
 * (at most `BlobChunk::MAX_LEN` bytes). Needs `read`.
 */
export type ReadAttachment = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * Which attachment.
 */
attachment: AttachmentId, 
/**
 * Where to start.
 */
offset: number, };

/**
 * `fleet.blob` (protocol 5): part of a blob, from `offset`. A member asks
 * only for blobs it lacks, chunk by chunk. Needs `Sync`.
 */
export type ReadBlob = { 
/**
 * Which blob.
 */
hash: BlobHash, 
/**
 * Where to start, in bytes.
 */
offset: number, };

/**
 * `session.read`: a page of a session's log. The authoritative way to fill
 * gaps; live subscriptions are an optimisation on top of it.
 */
export type ReadEvents = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * First sequence number wanted.
 */
from: EventSeq, 
/**
 * Maximum number of records; the node may return fewer and caps it.
 */
limit: number, 
/**
 * With nothing at `from` yet, wait this many seconds for something to
 * arrive before answering (an empty page when nothing did), at most
 * [`ReadEvents::MAX_WAIT`] (protocol 6). A long poll, so waiting on
 * another node's session needs no subscription across hops.
 */
wait?: number | null, };

/**
 * `file.read`: a page of a file.
 */
export type ReadFile = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which file.
 */
path: RelPath, 
/**
 * From this byte.
 */
offset: number, 
/**
 * At most this many bytes (at most 1 MiB, 256 KiB when absent).
 */
limit?: number | null, };

/**
 * Read one page; later pages must name the first page's revision.
 */
export type ReadFileBytes = { 
/**
 * Authorized workspace selection.
 */
root: WorkspaceRoot, 
/**
 * Validated path below that root.
 */
path: RelPath, 
/**
 * First byte requested; zero starts a transfer.
 */
offset: FileOffset, 
/**
 * Required on continuation, optional only at offset zero.
 */
expected: FileRevision | null, };

/**
 * `agent.install_output`: a page of an install's output. Needs `admin`.
 */
export type ReadInstall = { 
/**
 * Which install.
 */
job: InstallId, 
/**
 * From this byte of its output.
 */
from: number, 
/**
 * Wait this many seconds (at most 30) for more output or its end.
 */
wait?: number | null, };

/**
 * `git.log`: a page of history.
 */
export type ReadLog = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * From this commit or ref; `HEAD` when absent.
 */
from?: WorktreeBase | null, 
/**
 * Commits to skip from there.
 */
skip: number, 
/**
 * At most this many (at most 200, 50 when absent).
 */
limit?: number | null, };

/**
 * `worktree.output`: a page of setup and teardown output.
 */
export type ReadSetupOutput = { 
/**
 * Which worktree.
 */
worktree: WorktreeId, 
/**
 * From this byte of the output.
 */
from: number, 
/**
 * Wait this many seconds (at most 30) for more output or setup's end.
 */
wait?: number | null, };

/**
 * `terminal.read`: a page of a terminal's log. Answered with an
 * `EventPage`, as `session.read` is.
 */
export type ReadTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, 
/**
 * The first record wanted; a page starts at the oldest kept one when
 * this is older.
 */
from: EventSeq, 
/**
 * The most records; the node caps it at [`ReadTerminal::MAX_LIMIT`].
 */
limit: number, 
/**
 * With nothing at `from` yet, wait this many seconds for something
 * (at most 30).
 */
wait?: number | null, };

/**
 * `transfer.read` (protocol 6): part of one file of an open transfer, from
 * `offset`, as `fleet.blob` reads the bundle. Only the caller that opened
 * it may read it. Needs `read`. Answers a `BlobChunk`.
 */
export type ReadTransfer = { 
/**
 * Which transfer.
 */
transfer: TransferId, 
/**
 * Which file, by content.
 */
hash: BlobHash, 
/**
 * Where to start, in bytes.
 */
offset: number, };

/**
 *Bounded recent failed local RPC completions, not raw log lines.
 */
export type RecentFailures = Array<DiagnosticFailure>;

/**
 * Explicit non-destructive install/update of selected bundled skills.
 */
export type ReconcileSkills = Record<symbol, never>;

/**
 * A name git resolves: a branch, a remote-tracking branch such as
 * `origin/main`, or a tag.
 */
export type RefName = string;

/**
 * References at a source position.
 */
export type References = { 
/**
 * Whether declaration locations should also be included.
 */
include_declaration: boolean, 
/**
 * Authorized server selection.
 */
workspace: Workspace, 
/**
 * Relative saved file.
 */
file: FilePath, 
/**
 * One-based Unicode scalar position.
 */
position: Position, 
/**
 * Sorted result page (unused by hover).
 */
page: PageRequest, };

/**
 * Why a forge refused the node's token.
 */
export type RefusedReason = { "kind": "unauthorized" } | { "kind": "no_access" } | { "kind": "sso", 
/**
 * Where to authorise it, when the forge said.
 */
url?: ForgeUrl | null, } | { "kind": "forbidden" } | { "kind": "unknown" };

/**
 * An agent's id in the ACP registry (`goose`).
 */
export type RegistryId = string;

/**
 * Monotonic registry version; checked advancement refuses exhausted revisions.
 */
export type RegistryRevision = number;

/**
 * Complete person-visible registry state, replaced atomically by clients.
 * No field is added to existing session replies while the feature is pending.
 */
export type RegistrySnapshot = { 
/**
 * Cryptographic node identity survives a node-book alias rename.
 */
node: NodeId, 
/**
 * Global monotonic version for ordering complete snapshots.
 */
revision: RegistryRevision, 
/**
 * Includes empty workspaces; separate from the session list.
 */
workspaces: Array<WorkspaceDescriptor>, 
/**
 * Explicit session/terminal ownership in the same authoritative snapshot.
 */
memberships: Array<WorkspaceMembership>, };

/**
 * A version in the ACP registry: the registry's own, or an entry's.
 */
export type RegistryVersion = string;

/**
 * A relative path inside a root, parts separated by `/`. The empty path is
 * the root itself.
 */
export type RelPath = string;

/**
 * `fleet.relay` (protocol 6): a member asks its home to pass a call on.
 * The home refuses an `origin.node` other than the member's own (a member
 * speaks only for its own principals). Needs `relay`, which a member gets
 * only when the home's owner asks for it.
 */
export type RelayCall = { 
/**
 * The node that runs the call; the home runs it itself when it is
 * the target.
 */
node: NodeId, 
/**
 * The call.
 */
call: Call, 
/**
 * Who it speaks for.
 */
origin: Origin, };

/**
 * A registration ticket (protocol 6, `fleet.callback`): lets the one node
 * whose key it names register with a relay until it expires, without the
 * relay's token. A fleet's home mints one for a member from the token it
 * registers with, and the relay checks it with the same token, so the
 * relay keeps no state for it and rotating the token voids every ticket.
 */
export type RelayTicket = { 
/**
 * The node it registers: the relay takes it only with this key, and
 * only once the node proved it holds the key.
 */
key: NodePublicKey, 
/**
 * When it stops registering the node, in seconds since the Unix epoch;
 * a relay also ends a registration made with it then.
 */
expires_at: number, 
/**
 * The MAC over the key and `expires_at`.
 */
mac: TicketMac, };

/**
 * Compare source and descriptor revision before releasing this workspace's automation gate.
 */
export type ReleaseWorkspaceAutomation = { 
/**
 * Exact lifecycle owner on this node.
 */
workspace: WorkspaceId, 
/**
 * A changed source or metadata revision requires reinspection and another explicit choice.
 */
expected: RegistryRevision, 
/**
 * Must equal the retained blocked source, not a generic approve-all flag.
 */
source: BlockedSource, };

/**
 * `profile.reload` (protocol 5): read `profiles.toml` again. If it does
 * not parse, the node keeps the profiles it had and the error says where.
 * Answers with the profiles now in effect. Needs `Admin`.
 */
export type ReloadProfiles = null;

/**
 * The owner or a device of another node, for [`SessionOrigin::Remote`].
 */
export type RemoteCreator = { "kind": "owner" } | { "kind": "device", 
/**
 * Its id on that node.
 */
id: DeviceId, };

/**
 * A remote's name (`origin`).
 */
export type RemoteName = string;

/**
 * `agent.remove`: forget an added agent. Running sessions keep running;
 * an ended one cannot be started again. Refused while a profile extends
 * it. Needs `admin`, and is refused to an agent and to a call from
 * another node.
 */
export type RemoveAgent = { 
/**
 * Which agent.
 */
name: AgentName, };

/**
 * `project.remove`: forget a project (its files stay).
 */
export type RemoveProject = { 
/**
 * Which.
 */
project: ProjectId, };

/**
 * `worktree.remove`.
 */
export type RemoveWorktree = { 
/**
 * Which worktree.
 */
worktree: WorktreeId, 
/**
 * What to do when something would be lost.
 */
if_lost: IfLost, };

/**
 * `worktree.remove`'s answer.
 */
export type RemoveWorktreeOutcome = { "outcome": "removed", 
/**
 * The snapshot kept first, when one was.
 */
snapshot?: Snapshot | null, } | { "outcome": "kept", 
/**
 * Every reason that applies.
 */
reasons: Array<KeptReason>, };

/**
 * `git.rename_branch`.
 */
export type RenameBranch = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * This branch; the one checked out when absent.
 */
from?: BranchName | null, 
/**
 * Its new name.
 */
to: BranchName, };

/**
 * `git.rename_branch`'s answer.
 */
export type RenameOutcome = { "outcome": "renamed", 
/**
 * Its name now.
 */
branch: BranchName, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * `file.rename`: move a file or folder within its root.
 */
export type RenamePath = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * What.
 */
from: RelPath, 
/**
 * To where (nothing may be there).
 */
to: RelPath, };

/**
 * `session.rename` (protocol 9): give a session a title of its own. Every
 * client sees the change as a `renamed` event and in `SessionInfo.title`;
 * the reply is the session as it is now. Needs `operate`.
 */
export type RenameSession = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * The title.
 */
title: SessionTitle, };

/**
 * `terminal.rename`: give a terminal a title.
 */
export type RenameTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, 
/**
 * The title.
 */
title: SessionTitle, };

/**
 * Manual rename is independent from filesystem/branch names.
 */
export type RenameWorkspace = { 
/**
 * Existing identity.
 */
workspace: WorkspaceId, 
/**
 * None resets the displayed override.
 */
title: SessionTitle | null, };

/**
 * A successful response of any method, as sent on the wire.
 */
export type Reply = { "method": "node.info", "result": NodeInfo } | { "method": "harness.list", "result": Array<Harness> } | { "method": "session.list", "result": Array<SessionInfo> } | { "method": "session.get", "result": SessionInfo } | { "method": "session.create", "result": SessionInfo } | { "method": "session.side_open", "result": SessionInfo } | { "method": "session.side_prompt", "result": Delivered } | { "method": "session.side_reset", "result": SessionInfo } | { "method": "session.send", "result": Ack } | { "method": "session.kill", "result": Ack } | { "method": "session.read", "result": EventPage } | { "method": "session.subscribe", "result": Subscribed } | { "method": "session.unsubscribe", "result": Ack } | { "method": "node.stop", "result": Ack } | { "method": "device.invite", "result": Invitation } | { "method": "device.self", "result": DeviceSelf } | { "method": "device.list", "result": Array<DeviceInfo> } | { "method": "device.revoke", "result": Ack } | { "method": "profile.list", "result": Array<ProfileInfo> } | { "method": "profile.reload", "result": Array<ProfileInfo> } | { "method": "session.move_profile", "result": SessionInfo } | { "method": "fleet.manifest", "result": Manifest } | { "method": "fleet.blob", "result": BlobChunk } | { "method": "fleet.watch", "result": Generation } | { "method": "fleet.report", "result": Ack } | { "method": "fleet.join", "result": JoinedFleet } | { "method": "fleet.leave", "result": Ack } | { "method": "fleet.sync", "result": Ack } | { "method": "fleet.status", "result": FleetSyncStatus } | { "method": "fleet.published", "result": FleetPublished } | { "method": "fleet.members", "result": Array<FleetMember> } | { "method": "fleet.forward", "result": Forwarded } | { "method": "fleet.relay", "result": Forwarded } | { "method": "fleet.deliver", "result": Forwarded } | { "method": "fleet.grant", "result": Ack } | { "method": "fleet.callback", "result": Callback } | { "method": "fleet.peers", "result": Array<FleetNode> } | { "method": "fleet.nodes", "result": Array<FleetNode> } | { "method": "fleet.forget", "result": NodeForgotten } | { "method": "session.export", "result": TransferManifest } | { "method": "session.import", "result": SessionImported } | { "method": "memory.export", "result": TransferManifest } | { "method": "memory.import", "result": MemoryImported } | { "method": "transfer.read", "result": BlobChunk } | { "method": "transfer.close", "result": Ack } | { "method": "secret.set", "result": Ack } | { "method": "secret.list", "result": Array<SecretName> } | { "method": "secret.delete", "result": Ack } | { "method": "secret.copy", "result": Ack } | { "method": "lsp.workspace_symbols", "result": Page<Symbol> } | { "method": "lsp.document_symbols", "result": Page<Symbol> } | { "method": "lsp.definition", "result": Page<Location> } | { "method": "lsp.references", "result": Page<Location> } | { "method": "lsp.hover", "result": Hover } | { "method": "lsp.implementations", "result": Page<Location> } | { "method": "lsp.type_definition", "result": Page<Location> } | { "method": "lsp.incoming_calls", "result": Page<LspCall> } | { "method": "lsp.outgoing_calls", "result": Page<LspCall> } | { "method": "lsp.diagnostics", "result": Page<Diagnostic> } | { "method": "model.list", "result": Array<ModelRoute> } | { "method": "harness.options", "result": HarnessOptions } | { "method": "session.options", "result": Array<SessionOption> } | { "method": "session.set_option", "result": Array<SessionOption> } | { "method": "session.rename", "result": SessionInfo } | { "method": "session.prompt", "result": Delivered } | { "method": "session.queue", "result": QueueChanged } | { "method": "session.conversation", "result": Conversation } | { "method": "session.resume", "result": SessionInfo } | { "method": "session.mark", "result": SessionInfo } | { "method": "session.delete", "result": SessionDeleted } | { "method": "attachment.begin", "result": Upload } | { "method": "attachment.write", "result": Written } | { "method": "attachment.finish", "result": AttachmentRef } | { "method": "attachment.read", "result": BlobChunk } | { "method": "project.list", "result": ProjectList } | { "method": "project.get", "result": ProjectInfo } | { "method": "project.add", "result": ProjectInfo } | { "method": "project.clone", "result": ProjectInfo } | { "method": "project.init", "result": ProjectInfo } | { "method": "project.remove", "result": Ack } | { "method": "project.commands", "result": ProjectCommands } | { "method": "project.approve", "result": ProjectCommands } | { "method": "worktree.list", "result": WorktreeList } | { "method": "worktree.create", "result": CreateWorktreeOutcome } | { "method": "worktree.output", "result": SetupOutput } | { "method": "worktree.setup", "result": WorktreeInfo } | { "method": "worktree.remove", "result": RemoveWorktreeOutcome } | { "method": "worktree.snapshots", "result": SnapshotList } | { "method": "worktree.drop_snapshot", "result": Ack } | { "method": "git.status", "result": GitStatus } | { "method": "git.changes", "result": DiffSummary } | { "method": "git.diff", "result": FileDiff } | { "method": "git.show", "result": FileRead } | { "method": "git.branches", "result": BranchList } | { "method": "git.log", "result": CommitPage } | { "method": "git.stashes", "result": StashList } | { "method": "git.stage", "result": GitStatus } | { "method": "git.unstage", "result": GitStatus } | { "method": "git.discard", "result": GitStatus } | { "method": "git.commit", "result": CommitOutcome } | { "method": "git.fetch", "result": FetchOutcome } | { "method": "git.push", "result": PushOutcome } | { "method": "git.pull", "result": PullOutcome } | { "method": "git.switch", "result": SwitchOutcome } | { "method": "git.rename_branch", "result": RenameOutcome } | { "method": "git.stash", "result": StashOutcome } | { "method": "file.list", "result": DirListing } | { "method": "file.read", "result": FileRead } | { "method": "file.write", "result": FileVersion } | { "method": "file.create_dir", "result": Ack } | { "method": "file.rename", "result": Ack } | { "method": "file.delete", "result": Ack } | { "method": "file.find", "result": FoundFiles } | { "method": "file.grep", "result": GrepResult } | { "method": "forge.status", "result": ForgeStatus } | { "method": "pr.list", "result": PullRequestList } | { "method": "pr.get", "result": PullRequestView } | { "method": "pr.open", "result": OpenOutcome } | { "method": "pr.merge", "result": MergeOutcome } | { "method": "pr.feedback", "result": SessionPullRequest } | { "method": "terminal.create", "result": TerminalInfo } | { "method": "terminal.list", "result": TerminalList } | { "method": "terminal.get", "result": TerminalInfo } | { "method": "terminal.read", "result": EventPage } | { "method": "terminal.subscribe", "result": TerminalSubscribed } | { "method": "terminal.send", "result": Ack } | { "method": "terminal.rename", "result": TerminalInfo } | { "method": "terminal.kill", "result": Ack } | { "method": "terminal.close", "result": Ack } | { "method": "script.list", "result": ScriptList } | { "method": "script.run", "result": ScriptRunOutcome } | { "method": "script.restart", "result": ScriptRunOutcome } | { "method": "port.list", "result": PortList } | { "method": "agent.list", "result": Array<AgentInfo> } | { "method": "agent.add", "result": AgentInfo } | { "method": "agent.remove", "result": Ack } | { "method": "agent.status", "result": Array<AgentReadiness> } | { "method": "agent.catalogue", "result": Catalogue } | { "method": "agent.install", "result": InstallJob } | { "method": "agent.install_output", "result": InstallOutput } | { "method": "usage.query", "result": UsageReport } | { "method": "push.key", "result": PushKey } | { "method": "push.subscribe", "result": PushLease } | { "method": "push.unsubscribe", "result": Ack } | { "method": "push.presence", "result": Ack } | { "method": "inbox.list", "result": InboxPage } | { "method": "inbox.get", "result": InboxEntry | null } | { "method": "inbox.mark", "result": Ack } | { "method": "usage.limits", "result": UsageLimits } | { "method": "usage.lookup", "result": ExternalLookup } | { "method": "schedule.preview", "result": SchedulePreview } | { "method": "schedule.list", "result": ScheduleList } | { "method": "schedule.get", "result": ScheduleInfo } | { "method": "schedule.create", "result": ScheduleInfo } | { "method": "schedule.update", "result": ScheduleInfo } | { "method": "schedule.pause", "result": ScheduleInfo } | { "method": "schedule.resume", "result": ScheduleInfo } | { "method": "schedule.run", "result": ScheduleRun } | { "method": "schedule.delete", "result": Ack } | { "method": "schedule.runs", "result": RunList } | { "method": "prompt.list", "result": PromptList } | { "method": "prompt.create", "result": SavedPrompt } | { "method": "prompt.update", "result": SavedPrompt } | { "method": "prompt.delete", "result": Ack } | { "method": "session.search", "result": SearchPage } | { "method": "session.park", "result": SessionInfo } | { "method": "session.wake", "result": SessionInfo } | { "method": "node.settings", "result": NodeSettings } | { "method": "node.set_settings", "result": NodeSettings } | { "method": "session.fork", "result": SessionInfo } | { "method": "session.rewind", "result": SessionInfo } | { "method": "agent.history", "result": AgentHistoryPage } | { "method": "session.detach", "result": DetachedSession } | { "method": "workspace.registry", "result": RegistrySnapshot } | { "method": "workspace.list", "result": Array<WorkspaceDescriptor> } | { "method": "workspace.get", "result": WorkspaceDescriptor } | { "method": "workspace.rename", "result": WorkspaceDescriptor } | { "method": "workspace.pin", "result": WorkspaceDescriptor } | { "method": "workspace.release_automation", "result": WorkspaceDescriptor } | { "method": "git.compare", "result": GitCompared<DiffSummary> } | { "method": "git.compare_file", "result": GitCompared<FileDiff> } | { "method": "node.diagnostics", "result": DiagnosticReport } | { "method": "file.inspect", "result": InspectedDirectory } | { "method": "file.read_bytes", "result": FileBytePage } | { "method": "file.duplicate", "result": DuplicatedFileEntry } | { "method": "file.observe", "result": ObservedFile } | { "method": "file.suggest", "result": EntrySuggestions } | { "method": "skills.status", "result": SkillsSnapshot } | { "method": "skills.reconcile", "result": SkillsSnapshot } | { "method": "skills.uninstall", "result": SkillsSnapshot } | { "method": "skills.save_selection", "result": SkillsSaved } | { "method": "skills.import_selection", "result": SkillsImported };

/**
 * A repository's path on its forge: `owner/name`, or GitLab's
 * `group/subgroup/name`.
 */
export type RepoPath = string;

/**
 * A repository on a forge.
 */
export type RepoRef = { 
/**
 * Which kind of forge.
 */
kind: ForgeKind, 
/**
 * Its host, as the remote names it.
 */
host: ForgeHost, 
/**
 * `owner/name` (or GitLab's nested groups).
 */
path: RepoPath, };

/**
 * `fleet.report` (protocol 5): a member says what it applied and which
 * files it left alone because they were edited locally. Needs `Sync`.
 */
export type ReportApplied = { 
/**
 * The generation applied.
 */
generation: Generation, 
/**
 * Files kept as edited, with the incoming version written beside them.
 */
conflicts?: Array<BundlePath>, };

/**
 * A count of reports, bounded by the retained ledger.
 */
export type ReportCount = number;

/**
 * Presence for this connection: a socket alone never suppresses push.
 */
export type ReportPushPresence = { 
/**
 * Current visible target.
 */
focus: PushFocus, 
/**
 * A real input event, or only a focus change.
 */
activity: PushActivity, };

/**
 * Why a git run was refused for its repository's own configuration
 * (protocol 11, ADR 0030).
 */
export type RepositoryConfigReason = "program" | "layout" | "credential" | "unnamable" | "too_many" | "too_large" | "unknown";

/**
 * Replace the expected side under the same creation gate. Deletion and
 * creation are serialized, not a transaction with rollback.
 */
export type ResetSideQuestion = { 
/**
 * The original session.
 */
session: SessionId, 
/**
 * The side the person saw. A stale id must not delete a newer side.
 */
side: SessionId, };

/**
 * Node-level startup continuation policy (ADR 0059).
 */
export type RestartRecovery = { 
/**
 * Whether interrupted, eligible turns are continued at startup.
 */
enabled: boolean, 
/**
 * Exact text sent after restoring the provider's conversation.
 */
prompt: PromptText, };

/**
 * `script.restart`: run a script's run again (in place while it runs, in
 * a new terminal once it ended).
 */
export type RestartScript = { 
/**
 * The run.
 */
terminal: TerminalId, };

/**
 * `schedule.resume`: let a paused schedule run again, from now. Needs
 * `operate`.
 */
export type ResumeSchedule = { 
/**
 * Which.
 */
schedule: ScheduleId, };

/**
 * `session.resume`: start an ended structured session again with its
 * conversation. Needs `operate`.
 */
export type ResumeSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * The forge's review verdict.
 */
export type ReviewDecision = "approved" | "changes_requested" | "review_required" | "none" | "unknown";

/**
 * People's reviews on the current head, the latest per person, bots
 * left out (agent-orchestrator's `ExternalReviewFacts`).
 */
export type Reviews = { 
/**
 * A person's approval stands.
 */
approved: boolean, 
/**
 * A person requested changes.
 */
changes_requested: boolean, 
/**
 * An unresolved thread holds a person's comment.
 */
commented: boolean, 
/**
 * Unresolved review threads.
 */
unresolved: number, };

/**
 * `device.revoke` (protocol 4): revoke a device and close its connections.
 */
export type RevokeDevice = { 
/**
 * Which device.
 */
device: DeviceId, };

/**
 * `session.rewind`: the session goes back to the end of the turn whose
 * end is record `after`; its agent continues from there. Between turns
 * only. Needs `operate`; answers the session.
 */
export type RewindSession = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * A `turn_ended` record of it that carries a point.
 */
after: EventSeq, };

/**
 * A route to a node: a `ws://` or `wss://` URL, for a relay or a direct
 * address. Only the scheme is checked here; reachability is the client's.
 */
export type RouteUrl = string;

/**
 * How a run's new session's first turn ended.
 */
export type RunEnd = { "end": "turn", 
/**
 * Why.
 */
reason: StopReason, } | { "end": "failed", 
/**
 * The agent's reason, cut at [`RunOutcome::MESSAGE_MAX`] bytes.
 */
message: string, } | { "end": "session", 
/**
 * Its final status.
 */
status: SessionStatus, } | { "end": "unknown" };

/**
 * A run's refused call: the code it answered and its message.
 *
 * Not a [`WireError`]: its [`ErrorCode`] has no `unknown`, so one code a
 * newer node adds would make a whole list of runs unreadable to an older
 * client (the contract's review, item 16).
 */
export type RunError = { 
/**
 * What kind of refusal.
 */
code: RunErrorCode, 
/**
 * Why, cut at [`RunOutcome::MESSAGE_MAX`] bytes.
 */
message: string, };

/**
 * [`ErrorCode`], and `unknown` for a code a newer node has.
 */
export type RunErrorCode = "invalid_request" | "unauthorized" | "forbidden" | "protocol_mismatch" | "not_found" | "conflict" | "unsupported" | "harness_not_installed" | "spawn_failed" | "unavailable" | "internal" | "unknown";

/**
 * Reply to `schedule.runs`: at most [`RunList::MAX`], newest first.
 */
export type RunList = { 
/**
 * The runs.
 */
runs: Array<ScheduleRun>, };

/**
 * A schedule's run, counted from 1.
 */
export type RunNumber = number;

/**
 * What a run did.
 */
export type RunOutcome = { "outcome": "making" } | { "outcome": "started", 
/**
 * The session.
 */
session: SessionId, } | { "outcome": "waiting", 
/**
 * The session.
 */
session: SessionId, } | { "outcome": "finished", 
/**
 * The session.
 */
session: SessionId, 
/**
 * How.
 */
end: RunEnd, } | { "outcome": "prompted", 
/**
 * The session.
 */
session: SessionId, 
/**
 * Whether it started a turn or waits.
 */
delivered: Delivered, } | { "outcome": "skipped", 
/**
 * Why.
 */
reason: SkipReason, } | { "outcome": "failed", 
/**
 * The refusal.
 */
error: RunError, 
/**
 * The session it had started, if it got that far.
 */
session?: SessionId | null, } | { "outcome": "interrupted" } | { "outcome": "unknown" };

/**
 * `schedule.run`: run a schedule once now, outside its cadence. Needs
 * `operate`.
 */
export type RunSchedule = { 
/**
 * Which.
 */
schedule: ScheduleId, };

/**
 * `script.run`: run a script or start a service.
 */
export type RunScript = { 
/**
 * Whose checkout.
 */
place: ScriptPlace, 
/**
 * Which.
 */
script: ScriptName, };

/**
 * `worktree.setup`: run setup now.
 */
export type RunSetup = { 
/**
 * Which worktree.
 */
worktree: WorktreeId, };

/**
 * What made a run.
 */
export type RunTrigger = { "trigger": "due" } | { "trigger": "caught_up", 
/**
 * How many, counted up to 1,000.
 */
missed: number, } | { "trigger": "asked", 
/**
 * Who.
 */
by: Creator, } | { "trigger": "unknown" };

/**
 * Save only after the fresh deletion plan has been explicitly confirmed.
 */
export type SaveSkillsSelection = { 
/**
 * New desired set; all picks up future bundled additions.
 */
selection: SkillSelection, 
/**
 * Empty on first attempt; retry with the returned exact names.
 */
confirmed_removals: SkillNames, };

/**
 * A saved prompt.
 */
export type SavedPrompt = { 
/**
 * Its id.
 */
id: SavedPromptId, 
/**
 * Its name, unique within its project (or the machine's own).
 */
name: PromptName, 
/**
 * The text.
 */
text: PromptText, 
/**
 * The project it belongs to; absent: the machine's.
 */
project?: ProjectId | null, 
/**
 * What a session started from it runs with (structured, without
 * `resume`; its place, if any, in the prompt's project).
 */
session?: SessionSpec | null, 
/**
 * Who last wrote it.
 */
defined_by: Creator, 
/**
 * When.
 */
created_at: string, 
/**
 * When it last changed.
 */
updated_at: string, 
/**
 * Changes at every change; `prompt.update` names the one it was made
 * from.
 */
revision: number, };

/**
 * A saved prompt (protocol 11, ADR 0033).
 */
export type SavedPromptId = string & { readonly __brand: "SavedPromptId" };

/**
 * A schedule as a person writes it: `schedule.create` takes it,
 * `schedule.update` replaces it whole.
 */
export type ScheduleDefinition = { 
/**
 * What it is called.
 */
name?: ScheduleName | null, 
/**
 * When it is due.
 */
cadence: Cadence, 
/**
 * Where its prompt goes.
 */
target: ScheduleTarget, 
/**
 * What it sends.
 */
prompt: PromptText, 
/**
 * What it does about missed due times.
 */
catch_up: CatchUp, 
/**
 * It ends after this many runs that did something.
 */
max_runs?: MaxRuns | null, 
/**
 * It ends when its next due time would be after this.
 */
ends_at?: string | null, };

/**
 * A schedule (protocol 11, ADR 0033).
 */
export type ScheduleId = string & { readonly __brand: "ScheduleId" };

/**
 * A schedule as the node keeps it.
 */
export type ScheduleInfo = { 
/**
 * Its id.
 */
id: ScheduleId, 
/**
 * What it does.
 */
definition: ScheduleDefinition, 
/**
 * Whether it runs.
 */
state: ScheduleState, 
/**
 * The runs that sent their prompt: what `max_runs` bounds.
 */
runs: number, 
/**
 * The due time its last run was for.
 */
last_due?: string | null, 
/**
 * Its newest run.
 */
last_run?: ScheduleRun | null, 
/**
 * Who last wrote its definition; a run acts as them (ADR 0033).
 */
defined_by: Creator, 
/**
 * When it was made.
 */
created_at: string, 
/**
 * When it last changed.
 */
updated_at: string, 
/**
 * Changes when its definition changes or a person pauses or resumes
 * it, not at a run; `schedule.update` names the one it was made from.
 */
revision: number, };

/**
 * Reply to `schedule.list`, by creation, newest first.
 */
export type ScheduleList = { 
/**
 * The schedules.
 */
schedules: Array<ScheduleInfo>, };

/**
 * What a schedule or a hook is called on a page.
 */
export type ScheduleName = string;

/**
 * A preview's instants and the exact cadence to reuse when saving it.
 */
export type SchedulePreview = { 
/**
 * The resolved starting instant.
 */
after: string, 
/**
 * Includes the resolved interval anchor; reuse in create/update.
 */
cadence: Cadence, 
/**
 * Strictly increasing UTC instants, exactly the requested count.
 */
runs: Array<string>, };

/**
 * Why a preview could not be computed, carried in `WireError.detail`.
 */
export type SchedulePreviewError = "unknown_zone" | "unknown_cadence" | "no_future_run" | "time_out_of_range" | "unknown";

/**
 * One run of a schedule.
 */
export type ScheduleRun = { 
/**
 * Which run.
 */
number: RunNumber, 
/**
 * The due time it ran for: the latest, after a catch-up; the moment
 * asked, for a run now.
 */
due_at: string, 
/**
 * When it ran.
 */
at: string, 
/**
 * What made it run.
 */
trigger: RunTrigger, 
/**
 * What it did.
 */
outcome: RunOutcome, };

/**
 * Whether a schedule runs.
 */
export type ScheduleState = { "state": "active", 
/**
 * Its next due time.
 */
next_due: string, } | { "state": "paused", 
/**
 * Since when.
 */
since: string, } | { "state": "ended", 
/**
 * When.
 */
at: string, 
/**
 * Why.
 */
reason: EndReason, } | { "state": "unknown" };

/**
 * Where a schedule's prompt goes.
 */
export type ScheduleTarget = { "kind": "new_session", 
/**
 * What to start.
 */
spec: SessionSpec, 
/**
 * What becomes of the session after its first turn.
 */
after: AfterRun, } | { "kind": "session", 
/**
 * Which session.
 */
session: SessionId, } | { "kind": "unknown" };

/**
 * What a caller may do. Every RPC method declares its requirement
 * (`Method::scope`); `None` requires authentication without an extra scope.
 * The node token and the local socket hold every scope.
 */
export type Scope = "read" | "operate" | "admin" | "sync" | "relay";

/**
 * One script or service, with construction enforcing its declaration's meaning.
 */
export type ScriptEntry = { 
/**
 * The file's parsed name, also used to find the command by name.
 */
name: ScriptName, 
/**
 * Keep the role inseparable from its declared port.
 */
kind: ScriptKind, 
/**
 * The validated text the approved digest covers.
 */
command: ProjectCommand, 
/**
 * A folder inside the checkout; absence selects its root.
 */
dir?: RelPath | null, 
/**
 * An approved override; absence leaves allocation to the runtime.
 */
port?: ServicePort | null, };

/**
 * Run once, or kept running.
 */
export type ScriptKind = "script" | "service";

/**
 * `script.list`'s answer.
 */
export type ScriptList = { 
/**
 * The folder `vessel.toml` was read in.
 */
root: string, 
/**
 * What it names, as it is now, and whether that set is approved.
 */
commands: ProjectCommands, 
/**
 * The runs in this root, live and kept.
 */
runs: Array<TerminalInfo>, };

/**
 * A script's or a service's name in `vessel.toml`: 1 to 64 bytes of ASCII
 * letters, digits, `-`, `_` and `.`, starting with a letter or a digit.
 * A person types it (`vessel script run dev`), so it is kept plain.
 */
export type ScriptName = string;

/**
 * Whose checkout a script runs in.
 */
export type ScriptPlace = { "kind": "session", 
/**
 * Which.
 */
session: SessionId, } | { "kind": "project", 
/**
 * Which.
 */
project: ProjectId, };

/**
 * Why a script did not run.
 */
export type ScriptRefusal = { "kind": "no_project" } | { "kind": "no_such_script" } | { "kind": "no_such_dir" } | { "kind": "invalid_file", 
/**
 * Why.
 */
message: string, } | { "kind": "already_running", 
/**
 * That run.
 */
terminal: TerminalId, } | { "kind": "unknown" };

/**
 * What `script.run` and `script.restart` did.
 */
export type ScriptRunOutcome = { "outcome": "started", 
/**
 * The run.
 */
terminal: TerminalInfo, } | { "outcome": "needs_approval", 
/**
 * The set to approve (`project.approve`).
 */
commands: ProjectCommands, } | { "outcome": "refused", 
/**
 * Why.
 */
reason: ScriptRefusal, };

/**
 * Where the next page of a search starts: the node's handle, handed back
 * as it came (a decimal number in a string, so a page's JavaScript never
 * rounds it).
 */
export type SearchCursor = string & { readonly __brand: "SearchCursor" };

/**
 * `file.grep`: search inside files.
 */
export type SearchFiles = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * What to look for.
 */
query: GrepQuery, 
/**
 * How to read it.
 */
mode: GrepMode, 
/**
 * Whether case matters.
 */
case: GrepCase, 
/**
 * Only under this folder.
 */
dir?: RelPath | null, 
/**
 * At most this many matches (at most 500, 100 when absent).
 */
limit?: number | null, };

/**
 * A message that matched.
 */
export type SearchHit = { 
/**
 * The session it is in.
 */
session: SessionId, 
/**
 * The record where the message (or the piece of a long one) starts.
 */
seq: EventSeq, 
/**
 * When that record was written.
 */
at: string, 
/**
 * Who said it.
 */
role: MessageRole, 
/**
 * Where the words are in it.
 */
snippet: Snippet, };

/**
 * A client's number for a search, so a newer one with the same number
 * replaces it (`file.find`).
 */
export type SearchKey = number & { readonly __brand: "SearchKey" };

/**
 * How many hits a page may hold: 1 to [`SearchLimit::MAX`].
 */
export type SearchLimit = number;

/**
 * `session.search`: every session's messages (or one session's), newest
 * first. Needs `read` (protocol 12).
 */
export type SearchMessages = { 
/**
 * The words.
 */
query: SearchQuery, 
/**
 * Only this session's messages.
 */
session?: SessionId | null, 
/**
 * Continue from an earlier page's `next`.
 */
before?: SearchCursor | null, 
/**
 * At most this many hits ([`SearchLimit::DEFAULT`] when absent).
 */
limit?: SearchLimit | null, };

/**
 * One page of a search.
 */
export type SearchPage = { 
/**
 * The hits, newest first.
 */
hits: Array<SearchHit>, 
/**
 * Present while older messages may still match: hand it back as
 * `before`. A page can hold fewer hits than asked and still have it,
 * when the work a page may do ran out first.
 */
next?: SearchCursor | null, 
/**
 * With `next`: the time of the oldest message this page read, so every
 * match newer than it is in this page or an earlier one. A client
 * that merges several machines' pages by time shows a machine's hits
 * only down to its `reached` until it asks for the next page.
 */
reached?: string | null, };

/**
 * What a person searches for: 1 to [`SearchQuery::MAX_CHARS`] characters
 * once trimmed, at most [`SearchQuery::MAX_WORDS`] words. Every word must
 * be in a message for it to match.
 */
export type SearchQuery = string;

/**
 * A variable an added agent gets from one of the node's secrets
 * (`secrets.toml`, ADR 0012).
 */
export type SecretEnv = { 
/**
 * The variable.
 */
variable: EnvName, 
/**
 * The secret it is set from.
 */
secret: SecretName, };

/**
 * A secret's name in a node's `secrets.toml`; definitions refer to
 * secrets by it, so the value never leaves the node it is on.
 */
export type SecretName = string;

/**
 * A secret's value, on its way into a node's store.
 */
export type SecretValue = string;

/**
 * `session.send`: send input to a live session.
 */
export type SendInput = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * What to send.
 */
input: SessionInput, };

/**
 * `terminal.send`: keystrokes or a resize, for a shell.
 */
export type SendTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, 
/**
 * What.
 */
input: TerminalInput, };

/**
 * Why a service is not started again.
 */
export type ServiceFailure = { "kind": "gave_up", 
/**
 * Restarts tried.
 */
restarts: number, } | { "kind": "not_approved" } | { "kind": "spawn" } | { "kind": "unknown" };

/**
 * A service's configured or allocated TCP port, never the ephemeral sentinel zero.
 */
export type ServicePort = number;

/**
 * A project overrides allocation only by providing at least one valid choice.
 */
export type ServicePorts = { range?: PortRange, port_script?: PortScript, };

/**
 * What a service is doing (protocol 11). Also a record of its log
 * (`SessionEvent::Service`), at every change.
 */
export type ServiceState = { "state": "starting", 
/**
 * Which start, from 1.
 */
attempt: number, } | { "state": "running", 
/**
 * Which start, from 1.
 */
attempt: number, 
/**
 * Since when.
 */
since: string, } | { "state": "backing_off", 
/**
 * The start that failed.
 */
attempt: number, 
/**
 * Its exit code (none for a signal, or a start that failed).
 */
code?: number | null, 
/**
 * When the next start is.
 */
restart_at: string, } | { "state": "exited", 
/**
 * The exit code (0).
 */
code: number, } | { "state": "failed", 
/**
 * Why.
 */
reason: ServiceFailure, } | { "state": "stopped" } | { "state": "unknown" };

/**
 * A session's place on the board, derived by the node (protocol 8).
 *
 * `#[non_exhaustive]`: pull-request evidence joins it when a node can see
 * pull requests; build one with [`SessionBoard::new`].
 */
export type SessionBoard = { 
/**
 * The lane.
 */
column: KanbanColumn, 
/**
 * The phrase for its card, derived inside `column`.
 */
display_status: DisplayStatus, 
/**
 * What the agent was last seen doing.
 */
activity: Activity, 
/**
 * When the node last saw the agent do something: a turn start or end,
 * a permission asked or answered, terminal output.
 */
last_activity_at: string, };

/**
 * `session.delete`'s answer: the session is gone; what became of the
 * worktree made for it.
 *
 * [Decision Log] (milestone 11, where tracks A and B meet)
 * - Purpose: deleting a session tidies the worktree made for it (ADR
 *   0027's rule), and that removal may keep the worktree: uncommitted
 *   work, commits no other branch has, another session in it. A delete
 *   must never lose that work, nor keep it without saying so.
 * - Alternatives: refuse the delete while the worktree would lose work
 *   (two calls for a person, and retention, which deletes on its own,
 *   needs the keeping rule anyway); snapshot it unasked (a decision about
 *   a person's work made for them); answer `Ack` and leave the kept
 *   worktree to be noticed in `worktree.list` (silent).
 * - Choice: the session is deleted; the worktree goes only by the rule a
 *   person's `worktree.remove { if_lost: keep }` follows, and this answer
 *   says which it was, so a client can offer `snapshot` or `discard`.
 */
export type SessionDeleted = { 
/**
 * The worktree made for the session and what tidying did with it;
 * absent when the session had none of its own.
 */
worktree?: TidiedWorktree | null, };

/**
 * Something that happened in a session.
 */
export type SessionEvent = { "type": "status", 
/**
 * New state.
 */
status: SessionStatus, } | { "type": "terminal" } & TerminalOutput | { "type": "agent", 
/**
 * The entry.
 */
event: AgentEvent, } | { "type": "profile_changed", 
/**
 * The profile before; `None` is the harness's default.
 */
from?: ProfileName | null, 
/**
 * The profile now.
 */
to?: ProfileName | null, } | { "type": "model_call" } & ModelCall | { "type": "options_changed", 
/**
 * Every option, as the agent reports them.
 */
options: Array<SessionOption>, } | { "type": "renamed", 
/**
 * The title now.
 */
title: SessionTitle, 
/**
 * Who produced it (ADR 0062). Absent decodes as unknown. The
 * event stays protocol 9: a peer that ignores an unknown field
 * still reads `title`. A typed older hop may omit the origin;
 * the next reader then keeps the title with unknown origin.
 */
origin?: TitleOrigin, } | { "type": "queue_changed", 
/**
 * The queue now.
 */
queue: PromptQueue, } | { "type": "commands_changed", 
/**
 * Every command, in the agent's order.
 */
commands: Array<AgentCommand>, } | { "type": "usage_changed", 
/**
 * The report.
 */
usage: ContextUsage, } | { "type": "parked", 
/**
 * Why.
 */
reason: ParkReason, } | { "type": "woken" } | { "type": "forked", 
/**
 * The session forked.
 */
from: SessionId, 
/**
 * The record of `from` the fork starts after; absent for the
 * whole conversation.
 */
after?: EventSeq | null, } | { "type": "rewound", 
/**
 * The `turn_ended` record it went back to.
 */
after: EventSeq, 
/**
 * The agent's conversation from now on (a fork of the old one,
 * which the agent keeps).
 */
harness_session: HarnessSessionId, 
/**
 * How many tool calls left behind edited, deleted or moved files:
 * those changes were not put back.
 */
files_changed: number, };

/**
 * A session on a node. Unique across the fleet, so it can be used
 * without the node id once peers exist.
 */
export type SessionId = string & { readonly __brand: "SessionId" };

/**
 * Reply to `session.import`: where the conversation is now.
 */
export type SessionImported = { 
/**
 * The harness's id for it, to resume it by later.
 */
harness_session: HarnessSessionId, 
/**
 * The profile it was placed in; absent is the default.
 */
profile?: ProfileName | null, 
/**
 * The working directory it belongs to here.
 */
cwd: string, 
/**
 * Files written.
 */
written: number, 
/**
 * The new session, when `start` asked for one.
 */
session?: SessionInfo | null, };

/**
 * A session as reported by the node.
 *
 * `#[non_exhaustive]` for the reason given on [`SessionSpec`]; build one
 * with [`SessionInfo::new`].
 */
export type SessionInfo = { 
/**
 * Session id.
 */
id: SessionId, 
/**
 * What was asked for.
 */
spec: SessionSpec, 
/**
 * The profile the harness runs with now: `spec.profile` until
 * `session.move_profile` changes it (protocol 5).
 */
profile?: ProfileName | null, 
/**
 * Who created it (protocol 5, ADR 0013). Absent from older nodes, which
 * only ever had the owner.
 */
created_by: Creator, 
/**
 * Durable current-parent boundary (ADR 0090), separate from creation audit.
 * Absent on older peers and before a human explicitly detaches the child.
 */
detach_receipt?: DetachReceipt | null, 
/**
 * Current process state.
 */
status: SessionStatus, 
/**
 * When the session was created.
 */
created_at: string, 
/**
 * Latest acknowledged local durable mutation, not a label-map revision
 * or an atomic snapshot of live actor state. None preserves Unknown on
 * legacy rows and peers that cannot carry this feature (ADR 0063).
 */
updated_at?: SessionUpdatedAt | null, 
/**
 * When the process ended, if it has.
 */
ended_at?: string | null, 
/**
 * Sequence number the next event will get (= number of events so far).
 */
next_seq: EventSeq, 
/**
 * The harness's own session id, once a structured session has one.
 */
harness_session?: HarnessSessionId | null, 
/**
 * The home a call creating it came through, when another node asked
 * for it (protocol 6).
 */
via?: NodeId | null, 
/**
 * Where it came from, when not from this node (protocol 6).
 */
origin?: SessionOrigin | null, 
/**
 * The model route it was launched with, and the provider's model its
 * harness is pinned to (protocol 7). Absent for a session on its
 * harness's own provider, and from older nodes.
 */
model?: SessionModel | null, 
/**
 * Its place on the board: the lane and the phrase the node derived
 * from what its agent is doing (protocol 8, ADR 0019). Absent from
 * older nodes; a client then places it by `status`.
 */
board?: SessionBoard | null, 
/**
 * What it is called now (protocol 9), the one truth for its current
 * title: `spec.title` when one was asked for; otherwise, for a
 * structured session, the first line of its first prompt with text
 * (`SessionTitle::from_prompt`); and whatever `session.rename` gave it
 * last. Every change is also a `renamed` event in its log, and the
 * node's store keeps it across restarts. Absent until it has one, and
 * from older nodes; a client then names it by its folder.
 */
title?: SessionTitle | null, 
/**
 * The labels it carries now (ADR 0063): `spec.labels` at creation,
 * then whatever `session.labels` set. Left out when there are none,
 * and stripped for a peer that predates them.
 */
labels?: SessionLabels, 
/**
 * Immutable discovery classification, independent of process state and
 * archival. Older or unassigned peers receive no listing metadata.
 */
listing?: SessionListing, 
/**
 * Who produced [`title`](Self::title) (ADR 0062). Absent means
 * [`TitleOrigin::Unknown`](crate::title::TitleOrigin::Unknown): this
 * build does not speak the field yet, or the row was stored before
 * it. A `renamed` event carries the same origin.
 */
title_origin?: TitleOrigin, 
/**
 * What a person marked on it: pinned, archived, unread (protocol 10,
 * ADR 0026). Kept by the node, so every client sees the same; left
 * out when nothing is marked, and absent from older nodes.
 */
marks?: SessionMarks, 
/**
 * What its agent said it can do in its last handshake (protocol 10):
 * steer, start again, take images or audio. Kept by the store, so an
 * ended session says whether `session.resume` can work. Absent for a
 * terminal session, before the handshake, and from older nodes.
 */
abilities?: AgentAbilities | null, 
/**
 * Its project, worktree and branch (protocol 10), when its spec named
 * a place. Absent from older nodes. Boxed, as `SessionSpec.place` is.
 */
workspace?: SessionWorkspace | null, 
/**
 * Its pull request, for a live session with a project or a worktree
 * (protocol 11, ADR 0029): whether the node can see it, the last one
 * read, and whether feedback goes to the session. Set by the node;
 * cleared when the session ends.
 */
pull_request?: SessionPullRequest | null, 
/**
 * Its agent's process is parked (protocol 12, ADR 0038): stopped
 * while the session stays `running`, started again by the next
 * prompt. Absent while it runs, for an ended session, and from older
 * nodes.
 */
parked?: Parked | null, };

/**
 * Something a client sends into a live session. Body of `session.send`.
 *
 * Sending a variant that does not match the session's mode fails with
 * `ErrorCode::Unsupported`; the node never guesses a conversion.
 */
export type SessionInput = { "type": "terminal", 
/**
 * The input.
 */
input: TerminalInput, } | { "type": "prompt", 
/**
 * The user's message.
 */
content: Array<ContentBlock>, } | { "type": "cancel_turn", 
/**
 * The turn meant: its `turn_started` record's `seq` (protocol 10).
 * When another turn runs by the time the cancel lands (the queue
 * started the next one as the first ended), that turn goes on
 * and only the queue is held. Absent: whatever turn runs, as
 * before protocol 10.
 */
turn?: EventSeq | null, } | { "type": "answer_permission", 
/**
 * Which request.
 */
id: PermissionId, 
/**
 * The choice.
 */
outcome: PermissionOutcome, } | { "type": "answer_question", 
/**
 * Which question.
 */
id: QuestionId, 
/**
 * The answer.
 */
answer: QuestionAnswer, };

/**
 * The labels a session carries now, or the ones a creation asked for.
 *
 * Keys are ordered, so a summary and a stored row do not depend on who
 * sent the map. An empty map is left off the wire.
 */
export type SessionLabels = Record<string, string>;

/**
 * A creation-only discovery choice, separate from the session's run state.
 */
export type SessionListing = "shown" | "background";

/**
 * A session's marks, as `SessionInfo.marks`.
 */
export type SessionMarks = { 
/**
 * Kept at the top of lists.
 */
pinned?: boolean, 
/**
 * When it was put away (the board's archive lane). An archived
 * session has ended.
 */
archived_at?: string | null, 
/**
 * Something happened that nobody has marked seen.
 */
unread?: boolean, };

/**
 * The route a session was launched with (protocol 7).
 */
export type SessionModel = { 
/**
 * The route.
 */
route: ModelRouteName, 
/**
 * The provider's model the harness is pinned to.
 */
model: ModelId, };

/**
 * One of a session's config options, as the agent reports it.
 */
export type SessionOption = { 
/**
 * The option's id, sent back in an [`OptionChoice`].
 */
id: OptionId, 
/**
 * Its label, in the agent's words.
 */
name: string, 
/**
 * A longer explanation, when the agent gives one.
 */
description?: string | null, 
/**
 * What it is about.
 */
category: OptionCategory, } & ({ "kind": "select", 
/**
 * The value in effect.
 */
current: OptionValueId, 
/**
 * Every value offered, in the agent's order.
 */
values: Array<OptionValue>, } | { "kind": "toggle", 
/**
 * Whether it is on.
 */
on: boolean, } | { "kind": "unknown" });

/**
 * Where a session came from, when another node of the fleet had a hand in
 * it (protocol 6). `created_by` says who created it on this node; an
 * agent's own node is in `created_by` already, so `remote` names only the
 * owner or a device of the other node.
 */
export type SessionOrigin = { "kind": "remote", 
/**
 * The node the call started on.
 */
node: NodeId, 
/**
 * Who started it there.
 */
by: RemoteCreator, } | { "kind": "imported", 
/**
 * The node it was copied from.
 */
node: NodeId, 
/**
 * The session there.
 */
session: SessionId, };

/**
 * Where a session runs (`SessionSpec.place`, protocol 10).
 */
export type SessionPlace = { "kind": "project", 
/**
 * Which project.
 */
project: ProjectId, 
/**
 * A folder inside it; the project's root when absent.
 */
dir?: RelPath | null, } | { "kind": "worktree", 
/**
 * Which.
 */
worktree: WorktreeId, };

/**
 * One watched session's pull request (`pr.list`).
 */
export type SessionPullEntry = { 
/**
 * The session.
 */
session: SessionId, 
/**
 * Its pull request.
 */
pull_request: SessionPullRequest, };

/**
 * What a session carries of its pull request (`SessionInfo.pull_request`).
 */
export type SessionPullRequest = { 
/**
 * Whether the node can see it.
 */
access: ForgeAccess, 
/**
 * The last one read, kept while access fails later.
 */
summary?: PullSummary | null, 
/**
 * Whether its failing checks and reviews are sent to the session.
 */
feedback: boolean, };

/**
 * A session on a node of the fleet.
 */
export type SessionRef = { 
/**
 * The node.
 */
node: NodeId, 
/**
 * The session there.
 */
session: SessionId, };

/**
 * Scope and an AND predicate on current labels. Scope is required on the
 * wire so an empty object cannot silently choose a discovery policy.
 */
export type SessionSelection = { 
/**
 * Background inclusion is separate from permission to access an id.
 */
scope: ListingScope, 
/**
 * Every requested key and value must match; empty adds no predicate.
 */
labels: SessionLabels, };

/**
 * What to run. Body of `session.create`.
 *
 * `#[non_exhaustive]` with a constructor (protocol 5): every field added
 * since protocol 2 broke each struct literal in the workspace. Build one
 * with [`SessionSpec::new`] and the `with_*` methods; fields stay public
 * for reading and for changing after construction.
 */
export type SessionSpec = { 
/**
 * Harness to launch: one of the four, or (protocol 11) an agent added
 * on the node.
 */
harness: HarnessRef, 
/**
 * Working directory. Defaults to the node's home directory.
 */
cwd?: string | null, 
/**
 * The profile to launch with (protocol 5, ADR 0011); `None` is the
 * harness's default, which is how every earlier session ran.
 */
profile?: ProfileName | null, 
/**
 * What to call it, as asked at creation (protocol 9). The session's
 * title now is `SessionInfo.title`, which starts as this and changes
 * on its own; this stays what was asked.
 */
title?: SessionTitle | null, 
/**
 * Labels asked for at creation (ADR 0063). The map now is
 * [`SessionInfo::labels`], which starts as this and changes on its
 * own; this stays what was asked. Left out when there are none.
 */
labels?: SessionLabels, 
/**
 * Creation-only discovery override (ADR 0063). Omission can inherit
 * the creating agent; reusable templates retain that omission.
 */
listing?: SessionListing | null, 
/**
 * Where it runs (protocol 10, ADR 0027): a project's folder or a
 * worktree. With a place, `cwd` is left out by the client and filled
 * by the node with the folder the place resolves to, so a client that
 * predates places still sees where the session runs. Boxed: a spec is
 * held by every session's info and copied into its futures, and most
 * sessions have no place.
 */
place?: SessionPlace | null, 
/**
 * A separate side conversation of this node-local parent (protocol 19).
 * Never a shared native conversation id. Defaults absent for older peers.
 */
side_question_of?: SessionId | null, } & ({ "mode": "terminal", 
/**
 * Extra arguments passed verbatim to the harness binary.
 */
args: Array<string>, 
/**
 * Initial window size.
 */
size: TerminalSize, } | { "mode": "structured", 
/**
 * Model to request, if not the harness default.
 */
model?: ModelId | null, 
/**
 * Resume this harness session instead of starting a new one.
 */
resume?: HarnessSessionId | null, 
/**
 * The agent's config options to set once the session exists, in
 * order, after `model` (protocol 9, ADR 0022): a permissions mode,
 * an effort, anything the agent offers. A choice the agent does
 * not offer fails the session, as a model it does not offer does.
 */
options?: Array<OptionChoice>, });

/**
 * Lifecycle of the harness process behind a session.
 *
 * This is process state only. Whether a structured session is mid-turn is
 * derived from `TurnStarted`/`TurnEnded` events, so there is one source of
 * truth for each fact.
 */
export type SessionStatus = { "state": "starting" } | { "state": "running" } | { "state": "exited", 
/**
 * Exit code, if the OS reported one (none when killed by a signal).
 */
code: number | null, } | { "state": "failed", 
/**
 * Human-readable reason.
 */
message: string, 
/**
 * A fork that never held a conversation (protocol 14, ADR 0052):
 * its agent refused the fork or did not start. Nothing can be
 * resumed; a client offers to remove it (`session.delete`).
 * Absent (false) for every other failure and from older nodes.
 */
empty_fork?: boolean, } | { "state": "interrupted" };

/**
 * A session's title: one line of text a person reads.
 */
export type SessionTitle = string;

/**
 * A local writer's monotonic, millisecond mutation timestamp. It is data,
 * never a label-map revision, execution fence or caller attestation.
 */
export type SessionUpdatedAt = string;

/**
 * Where a session runs, as the node resolved it (`SessionInfo.workspace`).
 */
export type SessionWorkspace = { 
/**
 * Its project.
 */
project: ProjectId, 
/**
 * Its worktree, when it runs in one.
 */
worktree?: WorktreeId | null, 
/**
 * The worktree's branch when the session started.
 */
branch?: BranchName | null, };

/**
 * A change to [`AgentDrivers`]: the agents it names; the others stay.
 * Inside `settings.json` too, so it never refuses an unknown field (an
 * agent a later build names), as [`SetNodeSettings`] says.
 */
export type SetAgentDrivers = { 
/**
 * Claude Code's.
 */
claude_code?: AgentDriver | null, 
/**
 * Codex's.
 */
codex?: AgentDriver | null, 
/**
 * `OpenCode`'s.
 */
open_code?: AgentDriver | null, };

/**
 * `pr.feedback`: send the pull request's failing checks and reviews to
 * the session, or stop.
 */
export type SetFeedback = { 
/**
 * The session.
 */
session: SessionId, 
/**
 * On or off.
 */
enabled: boolean, };

/**
 * `node.set_settings`: change the settings it names; the others stay.
 * Needs `admin`. Answers the settings in force.
 *
 * Also the type of `settings.json` (vessel-store's `settings_file`), so it
 * and every type inside it never refuse a field they do not know (no
 * `deny_unknown_fields`, a later choice read as `unknown`): a file a later
 * build wrote must still parse after a downgrade, keeping what this build
 * knows. vessel-store's `a_file_from_a_later_build_keeps_what_this_build_knows`
 * holds the rule.
 */
export type SetNodeSettings = { 
/**
 * How long a structured session may stay idle before its agent is
 * parked; `unknown` is refused.
 */
park_after?: ParkAfter | null, 
/**
 * Which protocol drives the agents it names (protocol 18); `unknown`
 * is refused, and so is `direct` for an agent whose driver this node
 * does not offer yet.
 */
drivers?: SetAgentDrivers | null, };

/**
 * `secret.set` (protocol 6): store a value in this node's `secrets.toml`,
 * replacing one of the same name. Needs `admin`.
 */
export type SetSecret = { 
/**
 * Its name.
 */
name: SecretName, 
/**
 * Its value.
 */
value: SecretValue, };

/**
 * `session.set_option` (protocol 9): change one option of a running
 * structured session. The reply is every option as the agent reports them
 * after the change (one change can move others); the agent's refusal is
 * the call's error. Needs `operate`.
 */
export type SetSessionOption = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * The change.
 */
choice: OptionChoice, };

/**
 * usage.lookup: administrators alone change the machine credential-use policy.
 */
export type SetUsageLookup = { 
/**
 * Explicit policy, persisted before acknowledgement.
 */
policy: ExternalLookup, };

/**
 * Whether a new worktree runs the project's setup commands.
 */
export type SetupChoice = "run" | "skip";

/**
 * How setup failed.
 */
export type SetupFailure = { "kind": "exit", 
/**
 * Its index in the setup list.
 */
command: number, 
/**
 * Its exit code; absent when a signal ended it.
 */
code?: number | null, } | { "kind": "timed_out" } | { "kind": "spawn" } | { "kind": "unknown" };

/**
 * A page of a worktree's setup and teardown output.
 */
export type SetupOutput = { 
/**
 * Where this page starts in the output.
 */
from: number, 
/**
 * Standard output and error together, as printed.
 */
bytes: TerminalBytes, 
/**
 * Where the next page starts.
 */
next: number, 
/**
 * Whether the log reached its limit and stopped keeping output.
 */
truncated: boolean, 
/**
 * The setup's state now.
 */
setup: SetupState, };

/**
 * A worktree's setup.
 */
export type SetupState = { "state": "none" } | { "state": "skipped" } | { "state": "running", 
/**
 * Since when.
 */
started_at: string, } | { "state": "succeeded", 
/**
 * When.
 */
finished_at: string, } | { "state": "failed", 
/**
 * How.
 */
failure: SetupFailure, 
/**
 * When.
 */
finished_at: string, } | { "state": "unknown" };

/**
 * Diagnostic severity; absence is distinct from an error.
 */
export type Severity = "error" | "warning" | "information" | "hint" | "unknown";

/**
 * A SHA-256 digest, as 64 hexadecimal digits (lower case).
 */
export type Sha256Hex = string;

/**
 * One shared file.
 */
export type SharedFile = { 
/**
 * Its place in the bundle.
 */
path: BundlePath, 
/**
 * What it is, from its place.
 */
kind: SharedFileKind, 
/**
 * Its size in bytes.
 */
size: number, 
/**
 * Whether it is written executable (a skill's script).
 */
executable: boolean, };

/**
 * What a shared file is.
 */
export type SharedFileKind = "instructions" | "skill" | "other";

/**
 * `git.show`: a page of a file at a commit or ref (a diff's old side).
 */
export type ShowFile = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * At which commit or ref.
 */
at: WorktreeBase, 
/**
 * Which file.
 */
path: RelPath, 
/**
 * From this byte.
 */
offset: number, 
/**
 * At most this many bytes (at most 1 MiB, 256 KiB when absent).
 */
limit?: number | null, };

/**
 * How a person signs in to an agent. The node only says how (ADR 0032):
 * a login acts as the account, and runs where its account is.
 */
export type SignInHelp = { 
/**
 * The agent's own command, to run on that machine as the profile's
 * account; absent when it signs in inside its own interface.
 */
command?: string | null, 
/**
 * The variable that chooses the account (`CLAUDE_CONFIG_DIR`), by
 * name: its value is the profile's, in `profiles.toml`.
 */
home?: EnvName | null, 
/**
 * The sign-in methods its last handshake offered (ACP `authMethods`).
 */
methods?: Array<AuthMethod>, };

/**
 * Whether an agent is signed in under a profile.
 */
export type SignedIn = { "state": "yes" } | { "state": "no", 
/**
 * How a person signs in.
 */
help: SignInHelp, } | { "state": "not_required" } | { "state": "unknown", 
/**
 * Why.
 */
reason: UnknownReason, };

/**
 * Import acknowledgement without an ambiguous boolean result.
 */
export type SkillImportOutcome = "imported" | "already_set";

/**
 * A skill's portable directory name, excluding harness-private dot directories.
 */
export type SkillName = string;

/**
 * A normalized set used for selections, installed names and confirmations.
 */
export type SkillNames = Array<SkillName>;

/**
 * One user-visible operation against a bundled skill name.
 */
export type SkillOperation = { "kind": "add", 
/**
 * The bundled directory.
 */
name: SkillName, } | { "kind": "update", 
/**
 * The bundled directory.
 */
name: SkillName, } | { "kind": "delete", 
/**
 * The managed directory, requiring explicit confirmation.
 */
name: SkillName, };

/**
 * All follows future bundle additions; custom remains the exact chosen set.
 */
export type SkillSelection = { "mode": "all" } | { "mode": "custom", 
/**
 * Unknown available names are refused by the service before writes.
 */
skills: SkillNames, };

/**
 * Import never overwrites an explicit current choice.
 */
export type SkillsImported = { 
/**
 * The durable choice after import or refusal to replace.
 */
selection: SkillSelection, 
/**
 * Whether this call established the initial selection.
 */
outcome: SkillImportOutcome, };

/**
 * A selection save either commits or names deletions without changing state.
 */
export type SkillsSaved = { 
/**
 * Fresh view after saving, or the unchanged current view on confirmation.
 */
snapshot: SkillsSnapshot, 
/**
 * None means committed; otherwise confirm these exact fresh-plan removals.
 */
confirmation_required: SkillNames | null, };

/**
 * One complete view for host settings and onboarding.
 */
export type SkillsSnapshot = { 
/**
 * Actual installed state, independent from the selection.
 */
state: SkillsState, 
/**
 * Complete typed operations; no silently truncated list.
 */
ops: Array<SkillOperation>, 
/**
 * Names this release ships.
 */
available: SkillNames, 
/**
 * Names with an existing directory in at least one target.
 */
installed: SkillNames, 
/**
 * Durable user preference.
 */
selection: SkillSelection, };

/**
 * A complete status without conflating selected and actually installed skills.
 */
export type SkillsState = "not-installed" | "up-to-date" | "drift";

/**
 * Why a run sent nothing.
 */
export type SkipReason = { "reason": "previous_running" } | { "reason": "previous_waiting" } | { "reason": "stopped" } | { "reason": "missed", 
/**
 * How many, counted up to 1,000.
 */
count: number, } | { "reason": "unknown" };

/**
 * A worktree's working state kept as a commit before it was removed.
 */
export type Snapshot = { 
/**
 * The worktree it was.
 */
worktree: WorktreeId, 
/**
 * The branch it had checked out.
 */
branch?: BranchName | null, 
/**
 * The snapshot commit: the working tree, its parent `head`.
 */
commit: CommitId, 
/**
 * What `HEAD` was.
 */
head: CommitId, 
/**
 * When it was made.
 */
created_at: string, };

/**
 * A project's snapshots.
 */
export type SnapshotList = { 
/**
 * Newest first, at most 1,000.
 */
snapshots: Array<Snapshot>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * A window of a message around its first match: at most
 * [`Snippet::MAX_CHARS`] characters, white space runs as one space.
 */
export type Snippet = { 
/**
 * The window, in order.
 */
parts: Array<SnippetPart>, 
/**
 * The message goes on before the window.
 */
cut_before: boolean, 
/**
 * The message goes on after the window.
 */
cut_after: boolean, };

/**
 * One piece of a snippet: text, and whether it is a match.
 */
export type SnippetPart = { 
/**
 * The text.
 */
text: string, 
/**
 * Whether the text is one of the query's words.
 */
matched: boolean, };

/**
 * `git.stage`.
 */
export type Stage = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which files.
 */
paths: PathSet, };

/**
 * What to do with the stash.
 */
export type StashAction = { "kind": "save", 
/**
 * Its message.
 */
message?: CommitMessage | null, 
/**
 * Untracked files too.
 */
untracked: boolean, } | { "kind": "apply", 
/**
 * Which (`stash@{index}`).
 */
index: number, 
/**
 * The entry's commit, as listed: refused when the index moved.
 */
commit: CommitId, } | { "kind": "pop", 
/**
 * Which.
 */
index: number, 
/**
 * The entry's commit.
 */
commit: CommitId, } | { "kind": "drop", 
/**
 * Which.
 */
index: number, 
/**
 * The entry's commit.
 */
commit: CommitId, };

/**
 * `git.stash`.
 */
export type StashChanges = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * What to do.
 */
action: StashAction, };

/**
 * A stash entry.
 */
export type StashEntry = { 
/**
 * Its position: 0 is the newest (`stash@{0}`).
 */
index: number, 
/**
 * Its commit, which names it when indexes move.
 */
commit: CommitId, 
/**
 * Its message.
 */
message: string, 
/**
 * When it was made.
 */
time: string, };

/**
 * A repository's stash.
 */
export type StashList = { 
/**
 * Newest first, at most 100.
 */
entries: Array<StashEntry>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * `git.stash`'s answer.
 */
export type StashOutcome = { "outcome": "done", 
/**
 * The stash now.
 */
stashes: StashList, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * A file with a change, as `git status` reports it.
 */
export type StatusEntry = { 
/**
 * Its path.
 */
path: RelPath, 
/**
 * Where it was, for a rename or a copy.
 */
old_path?: RelPath | null, 
/**
 * Staged: the index against `HEAD`.
 */
index: FileState, 
/**
 * Not staged: the working tree against the index.
 */
worktree: FileState, };

/**
 * `node.stop` (protocol 3): stop the node, now or once idle. Used by
 * remote update; a client that negotiated protocol 2 never sends it.
 */
export type StopNode = { 
/**
 * When to stop.
 */
when: StopWhen, };

/**
 * Why a turn stopped.
 */
export type StopReason = "end_turn" | "max_tokens" | "max_turn_requests" | "refusal" | "cancelled";

/**
 * When `node.stop` takes effect.
 */
export type StopWhen = "now" | "when_idle";

/**
 * Whether a harness can run as a structured (ACP) session on this node.
 */
export type StructuredSupport = { "state": "available" } | { "state": "unavailable", 
/**
 * Human-readable reason.
 */
reason: string, };

/**
 * `session.subscribe`: stream a session's events from `from` onwards.
 */
export type Subscribe = { 
/**
 * Which session.
 */
session: SessionId, 
/**
 * First sequence number wanted (zero for the whole history).
 */
from: EventSeq, };

/**
 * Replace or renew this paired device's single subscription.
 */
export type SubscribePush = { 
/**
 * Browser subscription, with no caller-supplied device identity.
 */
subscription: PushSubscription, };

/**
 * `terminal.subscribe`: a terminal's records from `from`, then live ones,
 * as `ServerFrame::Event` frames; ended with `session.unsubscribe`.
 */
export type SubscribeTerminal = { 
/**
 * Which.
 */
terminal: TerminalId, 
/**
 * The first record wanted (the oldest kept one when this is older).
 */
from: EventSeq, };

/**
 * Reply to `session.subscribe`. Events follow as `ServerFrame::Event`.
 */
export type Subscribed = { 
/**
 * Id that tags the events of this subscription.
 */
subscription: SubscriptionId, 
/**
 * The session at the moment of subscribing.
 */
info: SessionInfo, };

/**
 * Why a subscription stopped delivering events.
 */
export type SubscriptionEnd = { "reason": "session_ended" } | { "reason": "unsubscribed" } | { "reason": "lagged", 
/**
 * First sequence number not delivered.
 */
resume_from: EventSeq, };

/**
 * Node-assigned id of one event subscription; unique per connection.
 */
export type SubscriptionId = number & { readonly __brand: "SubscriptionId" };

/**
 * A picker query within a registered workspace, never an arbitrary host root.
 */
export type SuggestEntries = { 
/**
 * Root authorization uses the same boundary as file reads.
 */
root: WorkspaceRoot, 
/**
 * Directory below that root to discover.
 */
directory: RelPath, 
/**
 * Bounded query; blank lists children without walking descendants.
 */
query: SuggestionQuery, 
/**
 * Entry kinds the caller can use.
 */
kinds: SuggestionKinds, 
/**
 * How paths match the query.
 */
matching: SuggestionMatch, 
/**
 * Requested result budget, validated by the node against the shared maximum.
 */
limit: SuggestionLimit | null, };

/**
 * A usable regular file or directory, with a validated in-root path.
 */
export type SuggestedEntry = { 
/**
 * Relative to the authorized workspace, never a client-local path.
 */
path: RelPath, 
/**
 * Files/directories only; symlinks and special entries are excluded.
 */
kind: SuggestedEntryKind, };

/**
 * The two entry kinds a picker can actually consume.
 */
export type SuggestedEntryKind = "file" | "directory";

/**
 * Which entries a picker accepts; avoids independent boolean combinations.
 */
export type SuggestionKinds = "files" | "directories" | "both";

/**
 * A validated result count, decoded before root resolution or disk work.
 */
export type SuggestionLimit = number;

/**
 * Reference matching modes, distinct from exact-path retrieval.
 */
export type SuggestionMatch = "fuzzy" | "suffix";

/**
 * A bounded, possibly empty query; empty lists immediate children.
 */
export type SuggestionQuery = string;

/**
 * `git.switch`.
 */
export type Switch = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * The branch.
 */
branch: BranchName, 
/**
 * Create it from here; it must exist otherwise.
 */
create?: WorktreeBase | null, };

/**
 * `git.switch`'s answer.
 */
export type SwitchOutcome = { "outcome": "switched", 
/**
 * Which.
 */
branch: BranchName, } | { "outcome": "refused", 
/**
 * Why.
 */
refusal: GitRefusal, };

/**
 * A symbol in saved source. Nested document symbols are flattened.
 */
export type Symbol = { 
/**
 * Bounded display name.
 */
name: string, 
/**
 * Its semantic kind.
 */
kind: SymbolKind, 
/**
 * Name/selection location.
 */
location: Location, 
/**
 * Optional enclosing symbol name.
 */
container: string | null, };

/**
 * LSP symbol kinds, including a typed unknown for future protocol values.
 */
export type SymbolKind = "file" | "module" | "namespace" | "package" | "class" | "method" | "property" | "field" | "constructor" | "enum" | "interface" | "function" | "variable" | "constant" | "string" | "number" | "boolean" | "array" | "object" | "key" | "null" | "enum_member" | "struct" | "event" | "operator" | "type_parameter" | "unknown";

/**
 * Ask this node to sync now. Ack means accepted, not already applied.
 * Concurrent requests coalesce behind the node's current apply.
 */
export type SyncFleet = Record<symbol, never>;

/**
 * Raw bytes to or from a PTY.
 *
 * [Decision Log]
 * - Purpose: PTY output is arbitrary bytes. A read can end in the middle of
 *   a UTF-8 sequence or an escape sequence, so it cannot be a `String`.
 * - Alternatives: a JSON array of numbers (4x size); lossy UTF-8 (corrupts
 *   split characters); separate binary websocket frames (a second framing
 *   scheme next to the typed JSON frames, and harder to relay and to log).
 * - Choice: `Bytes` in memory (cheap to fan out), base64 text on the wire
 *   and in the session log.
 * - Cost: about 33% size overhead on terminal traffic. Revisit with a binary
 *   codec if terminal bandwidth ever matters; the type does not change.
 */
export type TerminalBytes = string;

/**
 * A terminal beside a session: a shell, or a run of a project's script
 * (protocol 11, ADR 0031).
 */
export type TerminalId = string & { readonly __brand: "TerminalId" };

/**
 * A terminal as the node reports it.
 *
 * `#[non_exhaustive]` with a constructor, for the reason `SessionInfo`
 * gives: a field added later would break every struct literal.
 */
export type TerminalInfo = { 
/**
 * Its id.
 */
id: TerminalId, 
/**
 * Where it was placed.
 */
place: TerminalPlace, 
/**
 * The absolute folder it started in.
 */
folder: string, 
/**
 * What it runs.
 */
kind: TerminalKind, 
/**
 * What a person called it.
 */
title?: SessionTitle | null, 
/**
 * Who opened it (ADR 0013).
 */
created_by: Creator, 
/**
 * When it was opened.
 */
created_at: string, 
/**
 * When its process ended for good, if it has.
 */
ended_at?: string | null, 
/**
 * Its process's state: `running`, then `exited`, `failed` or
 * `interrupted` (a service is `running` while it is kept running,
 * restarts included).
 */
status: SessionStatus, 
/**
 * A service's state.
 */
service?: ServiceState | null, 
/**
 * A shell's window.
 */
size?: TerminalSize | null, 
/**
 * The oldest record still kept: older output was dropped (the
 * scrollback is bounded).
 */
oldest_seq: EventSeq, 
/**
 * The number the next record will get.
 */
next_seq: EventSeq, 
/**
 * Changes with every field but the two sequence numbers, for
 * `terminal.get`'s `changed_from`.
 */
revision: number, };

/**
 * Input to a terminal-mode session.
 */
export type TerminalInput = { "kind": "data", 
/**
 * The bytes.
 */
data: TerminalBytes, } | { "kind": "resize", 
/**
 * New size.
 */
size: TerminalSize, };

/**
 * What a terminal runs.
 */
export type TerminalKind = { "kind": "shell", 
/**
 * The program started.
 */
program: string, } | { "kind": "script", 
/**
 * Its name in `vessel.toml`.
 */
script: ScriptName, 
/**
 * What runs.
 */
command: ProjectCommand, } | { "kind": "service", 
/**
 * Its name in `vessel.toml`.
 */
script: ScriptName, 
/**
 * What runs.
 */
command: ProjectCommand, } | { "kind": "unknown" };

/**
 * `terminal.list`'s answer: live terminals first, then kept ones, each
 * newest first.
 */
export type TerminalList = { 
/**
 * At most 96 (64 live, 32 ended).
 */
terminals: Array<TerminalInfo>, };

/**
 * A chunk of PTY output.
 */
export type TerminalOutput = { 
/**
 * The bytes, as read from the PTY.
 */
data: TerminalBytes, };

/**
 * Where a terminal runs.
 */
export type TerminalPlace = { "kind": "session", 
/**
 * Which.
 */
session: SessionId, } | { "kind": "project", 
/**
 * Which.
 */
project: ProjectId, 
/**
 * A folder inside the project's root; the root when absent.
 */
dir?: RelPath | null, };

/**
 * Terminal window size in character cells.
 */
export type TerminalSize = { 
/**
 * Columns.
 */
cols: number, 
/**
 * Rows.
 */
rows: number, };

/**
 * `terminal.subscribe`'s answer.
 */
export type TerminalSubscribed = { 
/**
 * Tags the subscription's events.
 */
subscription: SubscriptionId, 
/**
 * The terminal when the subscription opened.
 */
terminal: TerminalInfo, };

/**
 * Whether a harness runs as a terminal session on a node (protocol 11).
 * Absent from older nodes, whose every harness runs in one.
 */
export type TerminalSupport = { "state": "available" } | { "state": "unavailable", 
/**
 * Human-readable reason.
 */
reason: string, };

/**
 * A shape free text must have.
 */
export type TextFormat = "email" | "uri" | "date" | "date_time";

/**
 * A ticket's HMAC-SHA256 under the relay's token (`vessel_relay`
 * computes and checks it). Not printed: it is no use without the
 * ticket's node key, but nothing needs to see it.
 */
export type TicketMac = string;

/**
 * A session's worktree, tidied with it.
 */
export type TidiedWorktree = { 
/**
 * Which worktree.
 */
worktree: WorktreeId, } & ({ "outcome": "removed", 
/**
 * The snapshot kept first, when one was.
 */
snapshot?: Snapshot | null, } | { "outcome": "kept", 
/**
 * Every reason that applies.
 */
reasons: Array<KeptReason>, });

/**
 * An IANA time zone's name, as a person chose it (`Asia/Seoul`).
 */
export type TimeZoneName = string;

/**
 * Who produced the session's current title (ADR 0062). Absent on the wire
 * decodes as [`Unknown`](Self::Unknown): a record from before generated
 * names, or an origin this build does not know. Generation replaces only
 * [`Given`](Self::Given).
 */
export type TitleOrigin = "typed" | "given" | "generated" | "unknown";

/**
 * Saturation is explicit; a capped counter is never presented as exact.
 */
export type TokenAccounting = "reported" | "overflow";

/**
 * A number of tokens.
 */
export type TokenCount = number;

/**
 * Where a node's token for a host came from (never the token).
 */
export type TokenSource = "environment" | "gh" | "unknown";

/**
 * The tokens one call used.
 */
export type TokenUsage = { 
/**
 * Input tokens neither read from nor written to a prompt cache.
 */
input: TokenCount, 
/**
 * Input tokens read from a prompt cache.
 */
cache_read: TokenCount, 
/**
 * Input tokens written to a prompt cache.
 */
cache_write: TokenCount, 
/**
 * Output tokens, reasoning included.
 */
output: TokenCount, 
/**
 * Of the output, the tokens spent reasoning, where the provider says.
 */
reasoning: TokenCount, };

/**
 * A tool call as first announced.
 */
export type ToolCall = { 
/**
 * Harness-assigned id.
 */
id: ToolCallId, 
/**
 * Short human-readable description.
 */
title: string, 
/**
 * Category.
 */
kind: ToolKind, 
/**
 * Initial status.
 */
status: ToolCallStatus, 
/**
 * Content produced so far.
 */
content: Array<ToolCallContent>, 
/**
 * Files involved.
 */
locations: Array<ToolLocation>, 
/**
 * The tool's raw input arguments, forwarded and never interpreted.
 */
raw_input?: RawJson | null, };

/**
 * Output attached to a tool call.
 */
export type ToolCallContent = { "type": "content", 
/**
 * The content.
 */
content: ContentBlock, } | { "type": "diff", 
/**
 * File being changed.
 */
path: string, 
/**
 * Previous text (`None` for a new file).
 */
old_text?: string | null, 
/**
 * New text.
 */
new_text: string, };

/**
 * A tool call id assigned by the harness.
 */
export type ToolCallId = string & { readonly __brand: "ToolCallId" };

/**
 * Progress of a tool call.
 */
export type ToolCallStatus = "pending" | "in_progress" | "completed" | "failed";

/**
 * A change to an announced tool call. `None` fields are unchanged; `Some`
 * collections replace the previous value.
 */
export type ToolCallUpdate = { 
/**
 * Which call.
 */
id: ToolCallId, 
/**
 * New title.
 */
title?: string | null, 
/**
 * New category.
 */
kind?: ToolKind | null, 
/**
 * New status.
 */
status?: ToolCallStatus | null, 
/**
 * Replacement content.
 */
content?: Array<ToolCallContent> | null, 
/**
 * Replacement locations.
 */
locations?: Array<ToolLocation> | null, 
/**
 * The tool's raw input arguments, when the harness sends them only with
 * an update (Claude's adapter often does). Forwarded, never interpreted.
 */
raw_input?: RawJson | null, 
/**
 * The tool's raw output, forwarded and never interpreted.
 */
raw_output?: RawJson | null, };

/**
 * What kind of work a tool call does. Lets clients pick an icon and a
 * default rendering without knowing harness tool names.
 */
export type ToolKind = "read" | "edit" | "delete" | "move" | "search" | "execute" | "think" | "fetch" | "switch_mode" | "other";

/**
 * A file location a tool call touches.
 */
export type ToolLocation = { 
/**
 * Path on the node.
 */
path: string, 
/**
 * 1-based line, if known.
 */
line?: number | null, };

/**
 * What a transfer holds.
 */
export type TransferDetail = { "kind": "session", 
/**
 * The harness's id for the conversation, to resume it by.
 */
harness_session: HarnessSessionId, 
/**
 * The directory it ran in (the files are named after it).
 */
cwd: string, 
/**
 * The session as its node reports it: harness, profile, creator,
 * title.
 */
info: SessionInfo, 
/**
 * The agent's options as the session runs with them now
 * (protocol 9): every select's current value and every toggle's
 * state from its last `options_changed`, or before any, its
 * spec's own choices. The importer applies them after the load,
 * so the session continues in the mode it was in (a permissions
 * mode above all) rather than the agent's default.
 */
options?: Array<OptionChoice>, } | { "kind": "memory", 
/**
 * Which harness.
 */
harness: HarnessKind, 
/**
 * Whose memory; absent is the default profile.
 */
profile?: ProfileName | null, 
/**
 * The project it is for, if only one.
 */
project?: string | null, };

/**
 * An open transfer: a snapshot of files a node holds for whoever opened
 * it (protocol 6). Random, but not a secret: the node also checks that
 * the caller reading or closing it is the one that opened it.
 */
export type TransferId = string & { readonly __brand: "TransferId" };

/**
 * Reply to `session.export` and `memory.export`.
 */
export type TransferManifest = { 
/**
 * The open transfer.
 */
transfer: TransferId, 
/**
 * Every file, by its path under the harness home.
 */
files: { [key in BundlePath]: BundleFile }, 
/**
 * What the files are.
 */
detail: TransferDetail, };

/**
 * Where a fork or a rewind can start (protocol 14, ADR 0052): the end of a
 * turn, named as the agent names it.
 *
 * [Decision Log]
 * - Purpose: ACP's `session/fork` forks the whole conversation; both
 *   installed adapters also read an anchor (`_meta.jetbrains.air.fork`),
 *   an agent message's id, and fork up to it (Claude) or after the turn
 *   that holds it (Codex).
 * - Choice: only a turn's end is a point (a cut inside a turn would leave
 *   a tool call without its result), and only when the turn's last agent
 *   message carried an id and nothing but the turn's end followed it (a
 *   thought, a plan or a tool call after it would be cut off). The point
 *   carries the conversation it belongs to, so a fork from a point a
 *   rewind left behind takes the conversation that still holds it.
 */
export type TurnPoint = { 
/**
 * The turn's last agent message.
 */
message: AgentMessageId, 
/**
 * The agent's conversation it is in.
 */
conversation: HarnessSessionId, };

/**
 * One turn's tokens, as the agent reports them with the turn's end (the
 * prompt's answer `usage`).
 *
 * Not [`TokenUsage`](crate::TokenUsage): that is the node proxy's own
 * count, with defined cache rules; agents disagree on whether `input`
 * counts cached tokens, so this keeps their words.
 */
export type TurnUsage = { 
/**
 * Every token of the turn.
 */
total: TokenCount, 
/**
 * Input tokens.
 */
input: TokenCount, 
/**
 * Output tokens.
 */
output: TokenCount, 
/**
 * Reasoning tokens, when counted apart.
 */
thought?: TokenCount | null, 
/**
 * Tokens read from a cache.
 */
cached_read?: TokenCount | null, 
/**
 * Tokens written to a cache.
 */
cached_write?: TokenCount | null, };

/**
 * Type definitions at a saved source position.
 */
export type TypeDefinition = AtPosition;

/**
 * Explicit whole-directory uninstall, confirmed by the human-facing client.
 */
export type UninstallSkills = Record<symbol, never>;

/**
 * Why a fact is not known.
 */
export type UnknownReason = { "reason": "not_installed" } | { "reason": "not_checked" } | { "reason": "in_use" } | { "reason": "not_said" } | { "reason": "no_home" } | { "reason": "unread" } | { "reason": "timed_out", 
/**
 * The limit.
 */
after: DurationMs, } | { "reason": "failed", 
/**
 * Where.
 */
step: CheckStep, };

/**
 * `git.unstage`.
 */
export type Unstage = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which files.
 */
paths: PathSet, };

/**
 * `session.unsubscribe`: stop a subscription on this connection.
 */
export type Unsubscribe = { 
/**
 * Which subscription.
 */
subscription: SubscriptionId, };

/**
 * Stop push to this paired device alone.
 */
export type UnsubscribePush = null;

/**
 * `prompt.update`: change a saved prompt (its project stays). Needs
 * `operate`; agents are refused.
 */
export type UpdatePrompt = { 
/**
 * Which.
 */
prompt: SavedPromptId, 
/**
 * The revision the change was made from; another is `conflict`.
 */
revision: number, 
/**
 * Its name.
 */
name: PromptName, 
/**
 * The text.
 */
text: PromptText, 
/**
 * What a session started from it runs with; absent clears it.
 */
session?: SessionSpec | null, };

/**
 * `schedule.update`: replace a schedule's definition. Needs `operate`.
 */
export type UpdateSchedule = { 
/**
 * Which.
 */
schedule: ScheduleId, 
/**
 * The revision the change was made from; another is `conflict`.
 */
revision: number, 
/**
 * The new definition.
 */
definition: ScheduleDefinition, };

/**
 * Reply to `attachment.begin`.
 */
export type Upload = { 
/**
 * The upload, for `attachment.write` and `attachment.finish`.
 */
upload: UploadId, 
/**
 * Largest piece a write takes, in bytes.
 */
chunk: number, };

/**
 * An upload in progress (protocol 10). Random, but not a secret: only
 * the caller that began it may write to it or finish it.
 */
export type UploadId = string & { readonly __brand: "UploadId" };

/**
 * One saved account's bounded facts and explicit credential-use policy.
 */
export type UsageLimits = { 
/**
 * Administrator's machine-wide choice.
 */
policy: ExternalLookup, 
/**
 * The selected configured binding.
 */
account: LimitAccount, 
/**
 * Intended saved-login store for disclosure, never a credential path.
 */
source: LimitLoginStore | null, 
/**
 * Latest safe state; never a claim about a running process's in-memory login.
 */
state: AccountLimitState, 
/**
 * Visibility of monetary/credit balances.
 */
cost_visibility: CostVisibility, 
/**
 * Optional factual full-window notice for a matching selected route.
 */
notice: LimitNotice | null, };

/**
 * The runtime's observation boundary, independent of the vendor's numbers.
 */
export type UsageObservation = "current" | "replayed";

/**
 * Rolling periods; a day is 24 hours, a week seven days.
 */
export type UsagePeriod = "day" | "week" | "session";

/**
 * A bounded, retained usage report. No claim of billing completeness.
 */
export type UsageReport = { 
/**
 * Query time.
 */
at: string, 
/**
 * When this node first enabled accounting; no historical backfill.
 */
accounting_started: string, 
/**
 * Earliest time retained by the age policy.
 */
retained_from: string, 
/**
 * The node pruned samples in this interval, or omitted agent groups.
 * Coverage of the selected scope may be partial; totals still count
 * every retained selected sample.
 */
incomplete: boolean, 
/**
 * Whether cost was withheld by the destination scope check.
 */
cost_visibility: CostVisibility, 
/**
 * For a session query, its latest cumulative agent reports. These
 * are observations, not money assigned to the selected time period.
 */
current_costs: Array<Cost>, 
/**
 * Selected totals.
 */
totals: UsageTotals, 
/**
 * At most 256 groups, in stable agent/profile order.
 */
agents: Array<AgentUsage>, 
/**
 * At most 100 session samples, oldest first after the cursor.
 */
samples: Array<UsageSample>, 
/**
 * Cursor for the next session page, only when more remain.
 */
next: EventSeq | null, };

/**
 * One committed turn or cost increment. The event sequence is its stable
 * identity. Cost updates are not assumed to describe a whole turn.
 */
export type UsageSample = { 
/**
 * Event that supplied the report.
 */
seq: EventSeq, 
/**
 * When the node recorded it.
 */
at: string, 
/**
 * The mutually exclusive report kind.
 */
kind: UsageSampleKind, };

/**
 * A turn end and a running cost update are different sources.
 */
export type UsageSampleKind = { "kind": "turn", 
/**
 * Exactly what the agent reported.
 */
usage: TurnUsage | null, } | { "kind": "cost_unavailable" } | { "kind": "cost_baseline", 
/**
 * What the agent said, without inventing its timing.
 */
cost: Cost, } | { "kind": "cost", 
/**
 * Beginning of the observation interval. Queries that cut this
 * interval do not assign its whole cost to the shorter period.
 */
since: string, 
/**
 * Never mixed with another unit.
 */
cost: Cost, };

/**
 * Which usage the caller asks for.
 */
export type UsageScope = { "kind": "machine" } | { "kind": "agent", 
/**
 * Agent implementation.
 */
harness: HarnessRef, 
/**
 * None includes every profile. Some selects that recorded name.
 */
profile: ProfileName | null, } | { "kind": "session", 
/**
 * Session identity.
 */
session: SessionId, };

/**
 * Sums of values actually reported, never estimates. Optional cache and
 * reasoning counts sum only the turns reporting those fields.
 */
export type UsageTotals = { 
/**
 * None when no turn supplied token counts; reported zero stays Some.
 */
tokens: TurnUsage | null, 
/**
 * Whether a sum exceeded the wire counter range.
 */
token_accounting: TokenAccounting, 
/**
 * Separate totals per unit; empty when absent or not permitted.
 */
costs: Array<Cost>, 
/**
 * Turns that supplied tokens.
 */
reported: ReportCount, 
/**
 * Ended turns without a token report.
 */
unreported: ReportCount, 
/**
 * Cost samples omitted for unknown timing or numerical precision.
 * An unrepresentable unit sum counts every contributing cost sample.
 */
unaccounted_costs: ReportCount, 
/**
 * How many turns supplied each optional token field. Compare each
 * count with reported + unreported before calling it a complete sum.
 */
optional_reports: OptionalUsageReports, };

/**
 * `session.wake`: start a parked session's agent again with its
 * conversation, answered once it is ready. Needs `operate`.
 */
export type WakeSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * `fleet.watch` (protocol 5): wait for a generation newer than `after`.
 * A long poll, not a subscription, so it needs no new frame type: the home
 * answers as soon as one is published, or with its current generation
 * (possibly `after` itself) when its wait limit passes, and the member asks
 * again. Needs `Sync`.
 */
export type WatchFleet = { 
/**
 * The generation the member has.
 */
after: Generation, };

/**
 * An error returned in place of a response.
 */
export type WireError = { 
/**
 * Machine-readable kind.
 */
code: ErrorCode, 
/**
 * Human-readable explanation. Never contains secrets.
 */
message: string, 
/**
 * Optional machine-readable reason (protocol 13); older errors omit it.
 */
detail?: ErrorDetail | null, };

/**
 * The server identity a query selects. This is not a filesystem grant.
 */
export type Workspace = { 
/**
 * Absolute root on the destination node; the node checks authority.
 */
root: string, 
/**
 * One server family per canonical workspace root.
 */
language: Language, };

/**
 * Applies to automatic setup, named services and metadata work, not explicit user shells.
 */
export type WorkspaceAutomation = { "kind": "allowed" } | { "kind": "blocked", 
/**
 * Render the actual source; never derive it from cwd, branch or approval digest.
 */
source: BlockedSource, };

/**
 * Existing backing identity is separate from the logical workspace identity.
 */
export type WorkspaceBacking = { "kind": "local" } | { "kind": "managed", 
/**
 * Exact backing identity, not a branch or cwd heuristic.
 */
worktree: WorktreeId, };

/**
 * Every client receives placement from the node; duplicate cwd means distinct owners.
 */
export type WorkspaceDescriptor = { 
/**
 * Stable independent identity.
 */
id: WorkspaceId, 
/**
 * Registered project on this node.
 */
project: ProjectId, 
/**
 * Authoritative execution folder on this node.
 */
path: string, 
/**
 * User title or absence; automatic names never replace a manual rename.
 */
title: SessionTitle | null, 
/**
 * Automatic work from external sources needs an explicit retained-source release.
 */
automation: WorkspaceAutomation, 
/**
 * Pin survives clients and reconnects; no second sidebar preference store.
 */
pin: WorkspacePin, 
/**
 * Backing ownership used by archive/recovery, never guessed from path shape.
 */
backing: WorkspaceBacking, 
/**
 * Active or archived as persisted by the node.
 */
lifecycle: WorkspaceLifecycle, 
/**
 * Durable visibility independent of route/tab state.
 */
visibility: WorkspaceVisibility, 
/**
 * Creation time for stable list ordering.
 */
created_at: string, 
/**
 * Revision permits stale response rejection without comparing timestamps.
 */
revision: RegistryRevision, };

/**
 * Independent workspace identity; shared cwd never implies shared ownership.
 */
export type WorkspaceId = string & { readonly __brand: "WorkspaceId" };

/**
 * Archived records remain inspectable and can be restored through reinspection.
 */
export type WorkspaceLifecycle = { "kind": "active" } | { "kind": "archiving", 
/**
 * Kept durably so interrupted cleanup can be inspected and resumed.
 */
at: string, } | { "kind": "restoring", 
/**
 * Retained recovery intent survives a node restart without duplicate creation.
 */
at: string, } | { "kind": "archived", 
/**
 * Host timestamp used for history ordering.
 */
at: string, };

/**
 * Explicit filter keeps archived records out of the default sidebar.
 */
export type WorkspaceList = "active" | "archived" | "all";

/**
 * Membership travels with the same snapshot as placement to avoid split hydration.
 */
export type WorkspaceMembership = { 
/**
 * The independent lifecycle container.
 */
workspace: WorkspaceId, 
/**
 * A resource belongs to at most one workspace on this node.
 */
resource: WorkspaceResource, };

/**
 * Pin is durable registry presentation, not a device-local sidebar ordering key.
 */
export type WorkspacePin = "unpinned" | "pinned";

/**
 * Ownership is explicit even when several workspaces share the same directory.
 */
export type WorkspaceResource = { "kind": "session", 
/**
 * Stable session identity, independent of provider-specific session ids.
 */
session: SessionId, } | { "kind": "terminal", 
/**
 * Terminal identity does not imply ownership by its cwd.
 */
terminal: TerminalId, };

/**
 * The folder a git or file call works in.
 */
export type WorkspaceRoot = { "kind": "project", 
/**
 * Which.
 */
project: ProjectId, } | { "kind": "worktree", 
/**
 * Which.
 */
worktree: WorktreeId, };

/**
 * Workspace symbol search.
 */
export type WorkspaceSymbols = { 
/**
 * Authorized server selection.
 */
workspace: Workspace, 
/**
 * Search text; bounded by the node.
 */
query: string, 
/**
 * Sorted result page.
 */
page: PageRequest, };

/**
 * A background workspace remains durable but is hidden by default lists.
 */
export type WorkspaceVisibility = "foreground" | "background";

/**
 * Lists do not infer background inclusion from an unrelated client preference.
 */
export type WorkspaceVisibilityFilter = "foreground" | "all";

/**
 * Where a new branch starts.
 */
export type WorktreeBase = { "kind": "ref", 
/**
 * Its name.
 */
name: RefName, } | { "kind": "commit", 
/**
 * Its id.
 */
id: CommitId, };

/**
 * Which branch a new worktree has checked out.
 */
export type WorktreeBranch = { "kind": "new", 
/**
 * Its name; `vessel/<8 hex>` when absent.
 */
name?: BranchName | null, 
/**
 * Where it starts; the project's default branch when absent.
 */
base?: WorktreeBase | null, } | { "kind": "existing", 
/**
 * Its name.
 */
name: BranchName, } | { "kind": "snapshot", 
/**
 * The worktree the snapshot was taken of.
 */
worktree: WorktreeId, 
/**
 * The new branch's name; `vessel/<8 hex>` when absent.
 */
name?: BranchName | null, };

/**
 * A worktree the node made for a project (protocol 10, ADR 0027).
 */
export type WorktreeId = string & { readonly __brand: "WorktreeId" };

/**
 * A worktree the node made.
 */
export type WorktreeInfo = { 
/**
 * Its id.
 */
id: WorktreeId, 
/**
 * Its project.
 */
project: ProjectId, 
/**
 * Its folder on the node.
 */
path: string, 
/**
 * The branch checked out now; absent when detached.
 */
branch?: BranchName | null, 
/**
 * The branch the node created for it, when it did: the only branch
 * ever deleted with it, and the one tidying checks.
 */
created_branch?: BranchName | null, 
/**
 * Where it started.
 */
base?: WorktreeStart | null, 
/**
 * When it was made.
 */
created_at: string, 
/**
 * Who asked for it (ADR 0013).
 */
created_by: Creator, 
/**
 * The session it was made for, when one was (the first session
 * started in it).
 */
created_for?: SessionId | null, 
/**
 * The live sessions whose folder is in it.
 */
sessions: Array<SessionId>, 
/**
 * Its setup.
 */
setup: SetupState, 
/**
 * Why tidying left it when its last session ended.
 */
kept?: KeptReason | null, };

/**
 * The worktrees.
 */
export type WorktreeList = { 
/**
 * Oldest first, at most 1,000.
 */
worktrees: Array<WorktreeInfo>, 
/**
 * Whether there were more.
 */
more: boolean, };

/**
 * Why `worktree.create` made nothing.
 */
export type WorktreeRefusal = { "kind": "not_git" } | { "kind": "not_ready" } | { "kind": "no_commit" } | { "kind": "branch_exists" } | { "kind": "checked_out_elsewhere", 
/**
 * That folder.
 */
path: string, } | { "kind": "no_such_base" } | { "kind": "no_such_branch" } | { "kind": "no_such_snapshot" } | { "kind": "limit" } | { "kind": "unknown" };

/**
 * Where a worktree started.
 */
export type WorktreeStart = { 
/**
 * The commit.
 */
commit: CommitId, 
/**
 * The ref it was asked by, when it was asked by one.
 */
name?: RefName | null, };

/**
 * What a write expects to find; a write that finds anything else
 * changes nothing and is `conflict`.
 */
export type WriteExpect = { "kind": "absent" } | { "kind": "content", 
/**
 * The SHA-256 of the content expected.
 */
hash: ContentHash, } | { "kind": "any" };

/**
 * `file.write`: a whole text file.
 */
export type WriteFile = { 
/**
 * Where.
 */
root: WorkspaceRoot, 
/**
 * Which file.
 */
path: RelPath, 
/**
 * Its new content, at most 1 MiB.
 */
text: string, 
/**
 * What must be there now.
 */
expect: WriteExpect, };

/**
 * `attachment.write`: the next piece of an upload. Only the caller that
 * began it may write. Needs `operate`.
 */
export type WriteUpload = { 
/**
 * Which upload.
 */
upload: UploadId, 
/**
 * Where the piece starts: the bytes received so far.
 */
offset: number, 
/**
 * The piece, base64 on the wire.
 */
data: string, };

/**
 * Reply to `attachment.write`: how much of the file the node has. A
 * piece repeated after a lost reply (it ends at or before this) is
 * answered again without being written twice.
 */
export type Written = { 
/**
 * Bytes received so far; the next piece starts here.
 */
received: number, };
