// Generated from the Rust wire types (ADR 0017). Do not edit: running
// `VESSEL_WRITE_TS=1 cargo test --features ts typescript` in the crate that
// generates this file rewrites it, and CI fails while it differs.

/**
 * Empty success.
 */
export type Ack = null;

/**
 * The latest state of a session's agent, as the node observed it.
 */
export type Activity = "active" | "idle" | "waiting_input" | "exited";

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
audio: boolean, };

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
hint?: string | null, };

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
usage?: TurnUsage | null, } | { "event": "turn_failed", 
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
 * A request of any method, as sent on the wire.
 */
export type Call = { "method": "node.info", "params": GetNode } | { "method": "harness.list", "params": ListHarnesses } | { "method": "session.list", "params": ListSessions } | { "method": "session.get", "params": GetSession } | { "method": "session.create", "params": CreateSession } | { "method": "session.send", "params": SendInput } | { "method": "session.kill", "params": KillSession } | { "method": "session.read", "params": ReadEvents } | { "method": "session.subscribe", "params": Subscribe } | { "method": "session.unsubscribe", "params": Unsubscribe } | { "method": "node.stop", "params": StopNode } | { "method": "device.invite", "params": InviteDevice } | { "method": "device.list", "params": ListDevices } | { "method": "device.revoke", "params": RevokeDevice } | { "method": "profile.list", "params": ListProfiles } | { "method": "profile.reload", "params": ReloadProfiles } | { "method": "session.move_profile", "params": MoveProfile } | { "method": "fleet.manifest", "params": GetManifest } | { "method": "fleet.blob", "params": ReadBlob } | { "method": "fleet.watch", "params": WatchFleet } | { "method": "fleet.report", "params": ReportApplied } | { "method": "fleet.members", "params": ListMembers } | { "method": "fleet.forward", "params": Forward } | { "method": "fleet.relay", "params": RelayCall } | { "method": "fleet.deliver", "params": Deliver } | { "method": "fleet.grant", "params": GrantHome } | { "method": "fleet.callback", "params": GetCallback } | { "method": "fleet.peers", "params": ListPeers } | { "method": "fleet.nodes", "params": ListNodes } | { "method": "fleet.forget", "params": ForgetNode } | { "method": "session.export", "params": ExportSession } | { "method": "session.import", "params": ImportSession } | { "method": "memory.export", "params": ExportMemory } | { "method": "memory.import", "params": ImportMemory } | { "method": "transfer.read", "params": ReadTransfer } | { "method": "transfer.close", "params": CloseTransfer } | { "method": "secret.set", "params": SetSecret } | { "method": "secret.list", "params": ListSecrets } | { "method": "secret.delete", "params": DeleteSecret } | { "method": "secret.copy", "params": CopySecret } | { "method": "lsp.workspace_symbols", "params": WorkspaceSymbols } | { "method": "lsp.document_symbols", "params": DocumentSymbols } | { "method": "lsp.definition", "params": Definition } | { "method": "lsp.references", "params": References } | { "method": "lsp.hover", "params": GetHover } | { "method": "lsp.implementations", "params": Implementations } | { "method": "lsp.type_definition", "params": TypeDefinition } | { "method": "lsp.incoming_calls", "params": IncomingCalls } | { "method": "lsp.outgoing_calls", "params": OutgoingCalls } | { "method": "lsp.diagnostics", "params": Diagnostics } | { "method": "model.list", "params": ListModels } | { "method": "harness.options", "params": GetHarnessOptions } | { "method": "session.options", "params": GetSessionOptions } | { "method": "session.set_option", "params": SetSessionOption } | { "method": "session.rename", "params": RenameSession } | { "method": "session.prompt", "params": PromptSession } | { "method": "session.queue", "params": ChangeQueue } | { "method": "session.conversation", "params": GetConversation } | { "method": "session.resume", "params": ResumeSession } | { "method": "session.mark", "params": MarkSession } | { "method": "session.delete", "params": DeleteSession } | { "method": "attachment.begin", "params": BeginUpload } | { "method": "attachment.write", "params": WriteUpload } | { "method": "attachment.finish", "params": FinishUpload } | { "method": "attachment.read", "params": ReadAttachment } | { "method": "project.list", "params": ListProjects } | { "method": "project.get", "params": GetProject } | { "method": "project.add", "params": AddProject } | { "method": "project.clone", "params": CloneProject } | { "method": "project.init", "params": InitProject } | { "method": "project.remove", "params": RemoveProject } | { "method": "project.commands", "params": GetProjectCommands } | { "method": "project.approve", "params": ApproveCommands } | { "method": "worktree.list", "params": ListWorktrees } | { "method": "worktree.create", "params": CreateWorktree } | { "method": "worktree.output", "params": ReadSetupOutput } | { "method": "worktree.setup", "params": RunSetup } | { "method": "worktree.remove", "params": RemoveWorktree } | { "method": "worktree.snapshots", "params": ListSnapshots } | { "method": "worktree.drop_snapshot", "params": DropSnapshot } | { "method": "git.status", "params": GetStatus } | { "method": "git.changes", "params": GetChanges } | { "method": "git.diff", "params": GetFileDiff } | { "method": "git.show", "params": ShowFile } | { "method": "git.branches", "params": ListBranches } | { "method": "git.log", "params": ReadLog } | { "method": "git.stashes", "params": ListStashes } | { "method": "git.stage", "params": Stage } | { "method": "git.unstage", "params": Unstage } | { "method": "git.discard", "params": Discard } | { "method": "git.commit", "params": Commit } | { "method": "git.fetch", "params": Fetch } | { "method": "git.push", "params": Push } | { "method": "git.pull", "params": Pull } | { "method": "git.switch", "params": Switch } | { "method": "git.rename_branch", "params": RenameBranch } | { "method": "git.stash", "params": StashChanges } | { "method": "file.list", "params": ListDir } | { "method": "file.read", "params": ReadFile } | { "method": "file.write", "params": WriteFile } | { "method": "file.create_dir", "params": CreateDir } | { "method": "file.rename", "params": RenamePath } | { "method": "file.delete", "params": DeletePath } | { "method": "file.find", "params": FindFiles } | { "method": "file.grep", "params": SearchFiles };

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
 * `transfer.close` (protocol 6): drop the snapshot. Only the caller that
 * opened it may close it. Needs `read`.
 */
export type CloseTransfer = { 
/**
 * Which transfer.
 */
transfer: TransferId, };

/**
 * A command the agent offers, without its `/` (`compact`, `review`).
 * Sent back as the start of a prompt, `/name`, so it has no white
 * space: what follows the first space is the command's input.
 */
export type CommandName = string;

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
 * `session.create`: start a harness.
 */
export type CreateSession = { 
/**
 * What to run.
 */
spec: SessionSpec, };

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
 * A duration in whole milliseconds.
 */
export type DurationMs = number;

/**
 * What an entry of a folder is.
 */
export type EntryKind = "file" | "dir" | "symlink" | "other" | "unknown";

/**
 * An environment variable's name.
 */
export type EnvName = string;

/**
 * What kind of failure occurred. Clients branch on this.
 */
export type ErrorCode = "invalid_request" | "unauthorized" | "forbidden" | "protocol_mismatch" | "not_found" | "conflict" | "unsupported" | "harness_not_installed" | "spawn_failed" | "unavailable" | "internal";

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
 * What a file is in the index or the working tree, against the other.
 */
export type FileState = "unmodified" | "modified" | "added" | "deleted" | "renamed" | "copied" | "type_changed" | "untracked" | "conflicted" | "unknown";

/**
 * What a write expects to find, so it never overwrites a change it did
 * not see.
 */
export type FileVersion = { 
/**
 * Its size in bytes.
 */
size: number, 
/**
 * When it was last changed, in nanoseconds since the Unix epoch.
 */
modified: number, 
/**
 * SHA-256 of its content, for a file of at most 8 MiB: a change
 * within one tick of a coarse clock keeps the size and the time.
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
 * The name of one folder to create (a clone's).
 */
export type FolderName = string;

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
 * `fleet.manifest` (protocol 5): the home's current bundle. Needs `Sync`.
 */
export type GetManifest = null;

/**
 * `node.info`: describe the node.
 */
export type GetNode = null;

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
 * `git.status`.
 */
export type GetStatus = { 
/**
 * Where.
 */
root: WorkspaceRoot, };

/**
 * A git operation that leaves a repository between two states.
 */
export type GitOperation = "merge" | "rebase" | "cherry_pick" | "revert" | "bisect";

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
message: string, } | { "kind": "unknown" };

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
 * What `worktree.remove` does when something would be lost.
 */
export type IfLost = "keep" | "snapshot" | "discard";

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
 * A supported server family; JavaScript uses the TypeScript server.
 */
export type Language = "rust" | "type_script" | "python" | "go" | "cpp";

/**
 * What a line of a hunk is.
 */
export type LineKind = "context" | "added" | "removed" | "no_newline";

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
 * `secret.list` (protocol 6): the names of this node's secrets, never
 * their values. Needs `admin`.
 */
export type ListSecrets = null;

/**
 * `session.list`: every session the node knows, newest first, including
 * ended sessions restored from disk.
 */
export type ListSessions = null;

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
 * `worktree.list`: every worktree, or one project's.
 */
export type ListWorktrees = { 
/**
 * Only this project's.
 */
project?: ProjectId | null, };

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
 * Every RPC method.
 */
export type Method = "node.info" | "harness.list" | "session.list" | "session.get" | "session.create" | "session.send" | "session.kill" | "session.read" | "session.subscribe" | "session.unsubscribe" | "node.stop" | "device.invite" | "device.list" | "device.revoke" | "profile.list" | "profile.reload" | "session.move_profile" | "fleet.manifest" | "fleet.blob" | "fleet.watch" | "fleet.report" | "fleet.members" | "fleet.forward" | "fleet.relay" | "fleet.deliver" | "fleet.grant" | "fleet.callback" | "fleet.peers" | "fleet.nodes" | "fleet.forget" | "session.export" | "session.import" | "memory.export" | "memory.import" | "transfer.read" | "transfer.close" | "secret.set" | "secret.list" | "secret.delete" | "secret.copy" | "lsp.workspace_symbols" | "lsp.document_symbols" | "lsp.definition" | "lsp.references" | "lsp.hover" | "lsp.implementations" | "lsp.type_definition" | "lsp.incoming_calls" | "lsp.outgoing_calls" | "lsp.diagnostics" | "model.list" | "harness.options" | "session.options" | "session.set_option" | "session.rename" | "session.prompt" | "session.queue" | "session.conversation" | "session.resume" | "session.mark" | "session.delete" | "attachment.begin" | "attachment.write" | "attachment.finish" | "attachment.read" | "project.list" | "project.get" | "project.add" | "project.clone" | "project.init" | "project.remove" | "project.commands" | "project.approve" | "worktree.list" | "worktree.create" | "worktree.output" | "worktree.setup" | "worktree.remove" | "worktree.snapshots" | "worktree.drop_snapshot" | "git.status" | "git.changes" | "git.diff" | "git.show" | "git.branches" | "git.log" | "git.stashes" | "git.stage" | "git.unstage" | "git.discard" | "git.commit" | "git.fetch" | "git.push" | "git.pull" | "git.switch" | "git.rename_branch" | "git.stash" | "file.list" | "file.read" | "file.write" | "file.create_dir" | "file.rename" | "file.delete" | "file.find" | "file.grep";

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
public_key?: NodePublicKey | null, };

/**
 * A node's static X25519 public key: the identity a client pins at pairing
 * and checks on every secure handshake.
 */
export type NodePublicKey = string & { readonly __brand: "NodePublicKey" };

/**
 * Whether a node is taking new work.
 */
export type NodeState = "serving" | "draining";

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
 * Which files a call acts on.
 */
export type PathSet = { "kind": "all" } | { "kind": "paths", 
/**
 * The paths.
 */
paths: Array<RelPath>, };

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
approved: boolean, } | { "state": "invalid", 
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
delivery: Delivery, };

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
 * `git.pull`: fast-forward from the upstream.
 */
export type Pull = { 
/**
 * Where.
 */
root: WorkspaceRoot, };

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
 * A successful response of any method, as sent on the wire.
 */
export type Reply = { "method": "node.info", "result": NodeInfo } | { "method": "harness.list", "result": Array<Harness> } | { "method": "session.list", "result": Array<SessionInfo> } | { "method": "session.get", "result": SessionInfo } | { "method": "session.create", "result": SessionInfo } | { "method": "session.send", "result": Ack } | { "method": "session.kill", "result": Ack } | { "method": "session.read", "result": EventPage } | { "method": "session.subscribe", "result": Subscribed } | { "method": "session.unsubscribe", "result": Ack } | { "method": "node.stop", "result": Ack } | { "method": "device.invite", "result": Invitation } | { "method": "device.list", "result": Array<DeviceInfo> } | { "method": "device.revoke", "result": Ack } | { "method": "profile.list", "result": Array<ProfileInfo> } | { "method": "profile.reload", "result": Array<ProfileInfo> } | { "method": "session.move_profile", "result": SessionInfo } | { "method": "fleet.manifest", "result": Manifest } | { "method": "fleet.blob", "result": BlobChunk } | { "method": "fleet.watch", "result": Generation } | { "method": "fleet.report", "result": Ack } | { "method": "fleet.members", "result": Array<FleetMember> } | { "method": "fleet.forward", "result": Forwarded } | { "method": "fleet.relay", "result": Forwarded } | { "method": "fleet.deliver", "result": Forwarded } | { "method": "fleet.grant", "result": Ack } | { "method": "fleet.callback", "result": Callback } | { "method": "fleet.peers", "result": Array<FleetNode> } | { "method": "fleet.nodes", "result": Array<FleetNode> } | { "method": "fleet.forget", "result": NodeForgotten } | { "method": "session.export", "result": TransferManifest } | { "method": "session.import", "result": SessionImported } | { "method": "memory.export", "result": TransferManifest } | { "method": "memory.import", "result": MemoryImported } | { "method": "transfer.read", "result": BlobChunk } | { "method": "transfer.close", "result": Ack } | { "method": "secret.set", "result": Ack } | { "method": "secret.list", "result": Array<SecretName> } | { "method": "secret.delete", "result": Ack } | { "method": "secret.copy", "result": Ack } | { "method": "lsp.workspace_symbols", "result": Page<Symbol> } | { "method": "lsp.document_symbols", "result": Page<Symbol> } | { "method": "lsp.definition", "result": Page<Location> } | { "method": "lsp.references", "result": Page<Location> } | { "method": "lsp.hover", "result": Hover } | { "method": "lsp.implementations", "result": Page<Location> } | { "method": "lsp.type_definition", "result": Page<Location> } | { "method": "lsp.incoming_calls", "result": Page<LspCall> } | { "method": "lsp.outgoing_calls", "result": Page<LspCall> } | { "method": "lsp.diagnostics", "result": Page<Diagnostic> } | { "method": "model.list", "result": Array<ModelRoute> } | { "method": "harness.options", "result": HarnessOptions } | { "method": "session.options", "result": Array<SessionOption> } | { "method": "session.set_option", "result": Array<SessionOption> } | { "method": "session.rename", "result": SessionInfo } | { "method": "session.prompt", "result": Delivered } | { "method": "session.queue", "result": QueueChanged } | { "method": "session.conversation", "result": Conversation } | { "method": "session.resume", "result": SessionInfo } | { "method": "session.mark", "result": SessionInfo } | { "method": "session.delete", "result": SessionDeleted } | { "method": "attachment.begin", "result": Upload } | { "method": "attachment.write", "result": Written } | { "method": "attachment.finish", "result": AttachmentRef } | { "method": "attachment.read", "result": BlobChunk } | { "method": "project.list", "result": ProjectList } | { "method": "project.get", "result": ProjectInfo } | { "method": "project.add", "result": ProjectInfo } | { "method": "project.clone", "result": ProjectInfo } | { "method": "project.init", "result": ProjectInfo } | { "method": "project.remove", "result": Ack } | { "method": "project.commands", "result": ProjectCommands } | { "method": "project.approve", "result": ProjectCommands } | { "method": "worktree.list", "result": WorktreeList } | { "method": "worktree.create", "result": CreateWorktreeOutcome } | { "method": "worktree.output", "result": SetupOutput } | { "method": "worktree.setup", "result": WorktreeInfo } | { "method": "worktree.remove", "result": RemoveWorktreeOutcome } | { "method": "worktree.snapshots", "result": SnapshotList } | { "method": "worktree.drop_snapshot", "result": Ack } | { "method": "git.status", "result": GitStatus } | { "method": "git.changes", "result": DiffSummary } | { "method": "git.diff", "result": FileDiff } | { "method": "git.show", "result": FileRead } | { "method": "git.branches", "result": BranchList } | { "method": "git.log", "result": CommitPage } | { "method": "git.stashes", "result": StashList } | { "method": "git.stage", "result": GitStatus } | { "method": "git.unstage", "result": GitStatus } | { "method": "git.discard", "result": GitStatus } | { "method": "git.commit", "result": CommitOutcome } | { "method": "git.fetch", "result": FetchOutcome } | { "method": "git.push", "result": PushOutcome } | { "method": "git.pull", "result": PullOutcome } | { "method": "git.switch", "result": SwitchOutcome } | { "method": "git.rename_branch", "result": RenameOutcome } | { "method": "git.stash", "result": StashOutcome } | { "method": "file.list", "result": DirListing } | { "method": "file.read", "result": FileRead } | { "method": "file.write", "result": FileVersion } | { "method": "file.create_dir", "result": Ack } | { "method": "file.rename", "result": Ack } | { "method": "file.delete", "result": Ack } | { "method": "file.find", "result": FoundFiles } | { "method": "file.grep", "result": GrepResult };

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
 * `session.resume`: start an ended structured session again with its
 * conversation. Needs `operate`.
 */
export type ResumeSession = { 
/**
 * Which session.
 */
session: SessionId, };

/**
 * `device.revoke` (protocol 4): revoke a device and close its connections.
 */
export type RevokeDevice = { 
/**
 * Which device.
 */
device: DeviceId, };

/**
 * A route to a node: a `ws://` or `wss://` URL, for a relay or a direct
 * address. Only the scheme is checked here; reachability is the client's.
 */
export type RouteUrl = string;

/**
 * `worktree.setup`: run setup now.
 */
export type RunSetup = { 
/**
 * Which worktree.
 */
worktree: WorktreeId, };

/**
 * What a caller may do. Every RPC method declares the one it needs
 * (`Method::scope`); the node checks it for calls from a paired device.
 * The node token and the local socket hold every scope.
 */
export type Scope = "read" | "operate" | "admin" | "sync" | "relay";

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
 * A client's number for a search, so a newer one with the same number
 * replaces it (`file.find`).
 */
export type SearchKey = number & { readonly __brand: "SearchKey" };

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
title: SessionTitle, } | { "type": "queue_changed", 
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
usage: ContextUsage, };

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
 * Current process state.
 */
status: SessionStatus, 
/**
 * When the session was created.
 */
created_at: string, 
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
workspace?: SessionWorkspace | null, };

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
 * Where it runs (protocol 10, ADR 0027): a project's folder or a
 * worktree. With a place, `cwd` is left out by the client and filled
 * by the node with the folder the place resolves to, so a client that
 * predates places still sees where the session runs. Boxed: a spec is
 * held by every session's info and copied into its futures, and most
 * sessions have no place.
 */
place?: SessionPlace | null, } & ({ "mode": "terminal", 
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
message: string, } | { "state": "interrupted" };

/**
 * A session's title: one line of text a person reads.
 */
export type SessionTitle = string;

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
 * A chunk of PTY output.
 */
export type TerminalOutput = { 
/**
 * The bytes, as read from the PTY.
 */
data: TerminalBytes, };

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
 * A number of tokens.
 */
export type TokenCount = number;

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
message: string, };

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
 * What a write expects.
 */
export type WriteExpect = { "kind": "absent" } | { "kind": "version", 
/**
 * The version read.
 */
version: FileVersion, } | { "kind": "any" };

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
