# Protocol

The types are defined in `crates/vessel-proto`; this file is the changelog
and a reader's summary. Bump `PROTOCOL_VERSION` on any breaking change and add
an entry here. Additive, optional fields do not need a bump.

## TypeScript declarations (`web/src/wire/`)

The page's declarations are generated from these types (ADR 0017);
nothing here changes the wire. Since milestone 11 a primitive alias is
declared by what it is (`vessel-proto/src/ts/aliases.rs`, one table):

- **A handle**, minted by a node, the bridge, a harness or the page's
  bridge client and only handed back by the page, is branded:
  `type SessionId = string & { readonly __brand: "SessionId" }`. A
  string or number is not one until a guard of its shape says so
  (`web/src/bridge/ids.ts`), so two kinds of handle cannot be swapped.
  The numeric ones too (`CallId`, `StreamId`, `SubscriptionId`): the
  first two come from one counter and are looked up in different maps.
- **Plain**: what a person types or names (`BookName`, `ProfileName`,
  `SecretName`, `SessionTitle`, `FilePath`, `EnvName`, `HttpUrl`,
  `KeyBinding`, …), checked where it is typed; quantities (`EventSeq`,
  `Offset`, `DurationMs`, `TokenCount`, `ProtocolVersion`, sizes,
  revisions); secrets (`SecretText`, `SecretValue`, `PairingLink`,
  `InvitationSecret`, `PresentedToken`, `TicketMac`), which the page
  takes from a person and hands on once; and content (`TerminalBytes`,
  `DraftText`).

A new alias is classified in that table, by this rule, in the change
that adds it: the generator's test fails on an alias in no class, and on
an entry neither generated file declares any more.

## Version 11 (2026-11-12, wave 2 of milestone 11)

Nodes speak 2..=11. One number for the whole of wave 2 ("Wave 2: tracks"
in `docs/plans/backend-m11.md`); each track adds its part. No event and no
shared reply gains a field, so a protocol 2-10 client reads a version 11
node as before.

### Schedules (track D)

Nine methods, all `since 11`, forwardable, in `vessel-web`'s list
(`docs/plans/backend-m11.md`, "Track D: contract" and its review; ADR
0033).

- Types (`vessel_proto::automation`):
  - `CronExpr`: five fields (minute, hour, day of the month, month, day of
    the week), lists, ranges, steps on `*` or a range, month and day names,
    `@hourly` to `@yearly`; parsed on decode into bit sets, so a bad one
    does not decode. `TimeZoneName` (an IANA name's shape; the node says
    whether it exists), `ScheduleName`, `PromptText` (16 KiB, no control
    character but tab and line ends), `EverySeconds` (60 to 31,622,400),
    `MaxRuns`, `RunNumber`.
  - `Cadence`: `every { seconds, anchor? }` (elapsed time from the anchor
    the node fills), `cron { expr, zone }`, `unknown`.
  - `ScheduleTarget`: `new_session { spec, after }` (a structured
    `SessionSpec`; `after` is `archive`, the default, or `keep`) or
    `session { session }` (through its queue), `unknown`.
  - `ScheduleDefinition { name?, cadence, target, prompt, catch_up,
    max_runs?, ends_at? }`, `CatchUp` (`once`, `skip`), `ScheduleState`
    (`active { next_due }`, `paused { since }`, `ended { at, reason }`),
    `ScheduleInfo { id, definition, state, runs, last_due?, last_run?,
    defined_by, created_at, updated_at, revision }`.
  - `ScheduleRun { number, due_at, at, trigger, outcome }`; `RunTrigger`
    (`due`, `caught_up { missed }`, `asked { by }`); `RunOutcome`
    (`making`, `started`, `waiting`, `finished { session, end }`,
    `prompted { session, delivered }`, `skipped { reason }`, `failed {
    error: RunError, session? }`, `interrupted`). `RunError`'s code is a
    `RunErrorCode`, `ErrorCode` with `unknown`, so a list of runs stays
    readable to an older client when a newer code appears.
  - Every enum that will grow has `unknown`; one sent to a node is
    `invalid_request`.
- Methods: `schedule.list`, `.get`, `.runs` (read); `schedule.create`,
  `.update` (the whole definition, under its `revision`), `.pause`,
  `.resume`, `.run` (once now, answered with the run's outcome),
  `.delete` (operate).
- A run acts as the schedule's `defined_by` as they are when it runs (a
  revoked device ends it; an agent's schedules go with its session), and
  meets the checks a call of theirs would. Agents: 8 schedules per
  session, their own session as a heartbeat's target, never `run` now;
  delegated callers may only read, pause, resume and delete.
- What happens at a daylight-saving gap or fold, after a stopped node, on
  a clock stepped either way, and to a run under way when the node stops,
  is the contract's "Time" and ADR 0033.
- Store: schema 13 (`schedules`, `schedule_runs`, `saved_prompts`, `hooks`,
  `hook_deliveries`, `hook_keys`; 10 to 12 are empty until tracks B2, C
  and G are joined).

### Saved prompts (track D)

Four methods, all `since 11`, forwardable, in `vessel-web`'s list (ADR
0033).

- `SavedPrompt { id, name, text, project?, session?, defined_by,
  created_at, updated_at, revision }`: `PromptName` (64 characters),
  `PromptText`; `project` absent is the machine's; `session` a structured
  `SessionSpec`, what a session started from it runs with, its place (if
  any) in the prompt's project.
- Methods: `prompt.list { project? }` (read; that project's and the
  machine's, `not_found` for an unknown project; by name, the machine's
  first of one name); `prompt.create`, `prompt.update` (name, text and
  `session` under its `revision`; the project stays), `prompt.delete`
  (operate).
- A name is unique within its project (or the machine's): `conflict`, as
  for an old revision and past 256 on the node. `session` is checked as
  `session.create` would check it (`invalid_request` for a terminal or
  resuming spec, or a place outside the prompt's project).
- A person's text: agents and delegated callers may list, not write
  (`forbidden`). Saved prompts are kept by the node, never read from a
  repository's files (ADR 0033 says why).

## Version 10 (2026-11-11, the two tracks joined 2026-11-12)

Nodes speak 2..=10; version 10 adds fifty methods, six events (with
stand-ins for older clients) and optional fields, in two parts built side
by side in milestone 11 (`docs/plans/backend-m11.md`): the conversation
("Track A: contract" and its review; ADRs 0024 to 0026) and projects,
worktrees, git and files ("Track B: contract", which has every call's
failures and bounds; ADRs 0023 and 0027). Where the two meet is the
plan's "Integration (node)".

### The conversation

Ten of the methods, the six events and most of the optional fields.

- Types (`vessel_proto::conversation`):
  - `AgentAbilities { steer, resume, image, audio }`: what the agent's
    ACP `initialize` answer says (`_meta.steering.supported`,
    `loadSession`, `promptCapabilities`). In `AgentEvent::Ready.abilities?`,
    `SessionInfo.abilities?` (kept by the store) and
    `HarnessOptions.abilities?`.
  - `ContentBlock::Attachment { attachment: AttachmentRef }`, `AttachmentRef
    { id, name, kind, mime, size }`, `kind` `image`, `audio` or `file`:
    only in what a client sends and its echo; the node replaces a
    client's reference with its own record by id.
  - The queue: `PromptQueue { entries, held? }`, `QueuedPrompt { id,
    content, queued_at, revision, by }`, `QueueHold` (`cancelled`,
    `failed`, `ended`, `asked`); `Delivery` (`start`, `queue`, `steer`,
    `interrupt`) and `Delivered` (`started`, `steered`, `queued { entry
    }`); `QueueChange` (`edit { entry, revision, content }`, `move {
    entry, before? }`, `remove`, `send_now { entry, delivery }`, `hold`,
    `resume`, `clear`).
  - `AgentCommand { name, description, hint? }`, `CommandName` (no `/`,
    no white space).
  - Questions: `Question { id, message, tool_call?, fields }`, each field
    `text`, `number`, `integer`, `boolean`, `choice` or `unknown`;
    `QuestionAnswer` (`accept { values }`, `decline`); `QuestionOutcome`
    (`answered`, `declined`, `cancelled`).
  - `ContextUsage { used, size, cost? }`, `TurnUsage { total, input,
    output, thought?, cached_read?, cached_write? }` in
    `AgentEvent::TurnEnded.usage?`.
  - `SessionMarks { pinned, archived_at?, unread }` in `SessionInfo.marks`
    (left out when nothing is marked); `Mark` (`pin`, `unpin`, `archive`,
    `unarchive`, `read { through }`, `unread`).
  - `AttachmentLimits` (in `Conversation`), and `Conversation { queue,
    commands, usage?, permissions, questions, abilities?, limits }`.
- Input: `SessionInput::AnswerQuestion { id, answer }`; `SessionInput::
  CancelTurn { turn? }`, the cancel naming the `turn_started` it means.
- Events, each with the stand-in a version 2-9 client gets at the same
  `seq`:
  - `queue_changed { queue }`: `vessel: queue: N waiting (held: why)`;
  - `commands_changed { commands }`: `vessel: commands: /a, /b and N
    more`;
  - `usage_changed { usage }`: `vessel: context: U of S tokens`;
  - `agent { steered { content } }`: `vessel: steered: <first line>`;
  - `agent { question_asked }`: `vessel: the agent asks: <message>`;
  - `agent { question_resolved }`: `vessel: question answered`
    (`declined`, `cancelled`);
  - a `turn_started` naming an attachment: the same event, each
    attachment a `resource_link` to `attachment:<id>`.
  An agent event's version is read from its content as well as its kind
  (`AgentEvent::since`).
- Methods, all forwardable: `session.prompt` (operate), `session.queue`
  (operate), `session.conversation` (read), `session.resume` (operate),
  `session.mark` (operate), `session.delete` (operate),
  `attachment.begin`, `attachment.write`, `attachment.finish` (operate),
  `attachment.read` (read). The `operate` ones that name a session are
  ownership-checked for agents; `attachment.write` and `attachment.finish`
  belong to the caller that began the upload. Their failures and bounds
  are the plan's contract.
- A version 10 call, an `answer_question`, a prompt naming an attachment,
  or a cancel naming its turn needs protocol 10 (`Call::since`).
- What older peers get:
  - Every reply is rewritten for the connection's version
    (`Reply::for_protocol`), the fleet envelope's nested reply included:
    before this, a `session.read` page passed back through
    `fleet.forward`, `fleet.relay` or `fleet.deliver` reached an older
    home unchanged.
  - Ended sessions: from this version, an ended session that is not
    archived is in `building` as `exited`, and only an archived one is in
    `archive` (ADR 0026). A version 8-9 peer still gets every ended
    session in the archive (`SessionInfo::for_protocol`).
- Logs: a session's final `status` is the last record of a run, no longer
  of the log: `session.resume` continues the same log after it.
- What each call does (the contract's details that the summary above
  leaves out): `session.conversation`, `session.prompt` and `session.queue`,
  `cancel_turn`'s `turn`, and questions: `question_asked`,
  `question_resolved` and `session.send`'s `answer_question`
  (`invalid_request` for an answer that does not fit the form,
  `not_found` for an unknown question, `conflict` once it is settled),
  offered only to the agent of a session the owner or a device created;
  marks and deletion: `SessionInfo.marks` everywhere a session's info is
  read, `session.mark` (`archive` stops a live session and answers once it
  has ended; a `read` past the end of the log is `invalid_request`; an
  agent marking `read` or `unread` is `forbidden`), `session.delete`
  (`conflict` for a live session and for one whose log is still being
  written; its answer, below), the board's placement of ended sessions,
  and retention's order (archived first, pinned last); starting again: `session.resume`
  (the row reopened before the new run writes, the log continued after
  the final status, the title, choices and creator kept, the queue back
  held `ended`, out of the archive, a new agent credential; it answers
  once the agent is ready, `spawn_failed` with the reason the log ends
  with otherwise), and a subscription the store was serving ends
  `lagged` when its session started again meanwhile; attachments:
  `attachment.begin`, `.write`, `.finish` and `.read` (an upload is its
  caller's, `not_found` to anyone else; an image or audio only to an
  agent that said it takes them, `unsupported` otherwise and while it has
  not said; a piece that ends at or before what the node has is answered
  with it, one that starts elsewhere is `invalid_request`; an image that
  does not start as its type does is `invalid_request` and its upload
  dropped; `conflict` once the session ended, and for a kept file that
  was changed on disk; uploads open at a restart are dropped), and every
  prompt naming one (`session.prompt`, a queue `edit`, `session.send`, and
  a queued prompt when it is sent; a queue `edit` with no content is
  `invalid_request`, as an empty prompt is) checked against the session's own
  records, the per-prompt bounds and what its agent takes then
  (`invalid_request`); and every call from the command line (`vessel send
  --queue|--steer|--interrupt|--attach`, `vessel session
  queue|answer|resume|pin|unpin|archive|unarchive|read|unread|delete`)
  and, but marks other than archiving, deletion and attachments, from the
  agent tools (`send_input`'s `delivery`, `read_conversation`,
  `change_queue`, `answer_question`, `resume_session`,
  `archive_session`). A `vessel send` without a delivery or a file still
  makes `session.send`, so it works with an older node.

### Projects, worktrees, git and files

Forty of the methods, two optional fields and no event.

- Types (`vessel_proto::workspace`, one module per area): `ProjectId`,
  `WorktreeId`; `RelPath` (relative, `/`-separated, never absolute,
  climbing, a drive, or any `.git` by git's `verify_path` rules: a bad
  one does not decode); `CloneUrl` (`https`, `http`, `ssh`, `git` or
  `user@host:path`; no `<transport>::`, leading `-` or credentials);
  bounded names (`BranchName`, `RefName`, `RemoteName`, `CommitId`,
  `ProjectName`, `FolderName`, `CommitMessage`, `ProjectCommand`,
  `CommandsDigest`, `ContentHash`, `FindQuery`, `GrepQuery`); every reply
  enum that will grow has `unknown`.
- `SessionSpec.place?`: `project { project, dir? }` or `worktree {
  worktree }`. The node fills `cwd` with the folder it resolves to (so a
  protocol 9 client still sees where the session runs), refuses a place
  together with a `cwd`, and starts the log with `vessel:` notices (the
  folder, the branch and its base, setup's result and, after a failure,
  its last lines). In a worktree whose setup runs: `conflict`. A
  `session.create` with a place needs protocol 10 (`Call::since`). The
  first session started in a worktree made for no session claims it.
  `SessionInfo.workspace? { project, worktree?, branch? }`, filled from
  the place on every answer.
- Methods, all forwardable, in `vessel-web`'s list; agents may call only
  `project.list`, `project.get`, `project.commands`, `worktree.list`,
  `worktree.create` (at most 8 they made), `worktree.output`, and
  `session.create` in a project or a worktree they made (or made for a
  session they may operate; a `cwd` of their own inside the node's data
  folder, where worktrees are made, is `forbidden`: they name the
  `place`); every other one answers them `forbidden`. Creations at once
  count against the 8 together.
  - Projects: `project.list` (read; with the git found: `found {
    version }`, `too_old`, `missing`), `project.get { project,
    changed_from? }` (read; a long poll until the revision differs, at
    most 30 s), `project.add { path, name? }`, `project.clone { url,
    parent, folder?, name? }` (answers `cloning`; progress in `state`),
    `project.init { path, name? }`, `project.remove` (forgets; files
    stay), `project.commands { project, base? }` (read),
    `project.approve { project, digest, approved }` (all operate).
    Roots refused: a file system's root, any folder that is or holds the
    home folder, any folder holding the node's data folder or inside it;
    `project.init` checks the folder it would make before it makes
    anything (`.` and `..` refused). A project's `vessel.toml` is read
    only as a regular file inside its folder, at most 64 KiB; one that
    leads out of it is as if there were none.
  - Worktrees: `worktree.list { project? }`, `worktree.create { project,
    branch: new { name?, base? } | existing { name } | snapshot {
    worktree, name? }, setup: run | skip }` answering `created`,
    `needs_approval { commands }` or `refused { reason }`;
    `worktree.output { worktree, from, wait? }` (a page of at most
    64 KiB of setup's output, a long poll); `worktree.setup`;
    `worktree.remove { worktree, if_lost: keep | snapshot | discard }`
    answering `removed { snapshot? }` or `kept { reasons }`;
    `worktree.snapshots { project }`, `worktree.drop_snapshot`.
  - Git (`root`: `project { project }` or `worktree { worktree }`):
    `git.status`, `git.changes { scope, ignore_whitespace }` (scopes
    `unstaged`, `staged`, `uncommitted`, `base { base? }`, `commit {
    id }`), `git.diff` (one file's typed hunks, paged by hunk),
    `git.show` (a file at a commit, paged), `git.branches`, `git.log`,
    `git.stashes` (read); `git.stage`, `git.unstage`, `git.discard`,
    `git.commit`, `git.fetch`, `git.push`, `git.pull` (fast-forward
    only), `git.switch`, `git.rename_branch`, `git.stash` (operate),
    each with typed refusals (`GitRefusal`). A call's wait for one of
    the node's git slots counts against its time (`timed_out`). Every
    call checks its root again: a folder replaced since it was recorded
    (a link to another), or a worktree whose `.git` no longer names its
    repository's record of it, is `conflict`, and git is not run in it.
  - Files: `file.list`, `file.read`, `file.find { query, key? }` (by
    name; a newer search with the same key replaces a running one),
    `file.grep` (read); `file.write { expect: absent | version | any }`,
    `file.create_dir`, `file.rename`, `file.delete` (operate). A path
    that leads out of the root, one that cannot be followed (a link to
    nowhere, a file where a folder should be, no permission) and one
    that is not there all answer `not_found`; what is read is opened
    first and judged a regular file on the opened file (a FIFO is not
    waited on).
- Store schema 9 (`projects`, `worktrees`, `approvals`).

### Where the two meet

- `session.delete` answers `SessionDeleted { worktree? }` (track A's
  branch answered `Ack`; nothing of protocol 10 was released, so the
  reply changed in place): `worktree` is `TidiedWorktree { worktree,
  outcome }` for the worktree made for the session (`WorktreeInfo.
  created_for`), `outcome` `removed` or `kept { reasons }`. The session
  is deleted either way; the worktree goes only by the rule a person's
  `worktree.remove { if_lost: keep }` follows (ADR 0027), so a delete
  never loses uncommitted work or commits no other branch has, never
  snapshots or discards unasked, and says when it kept something (also
  in `WorktreeInfo.kept`).
- Retention dropping a session tidies its worktree by the same rule, in
  the background after the prune; the node also tidies, when it starts,
  every worktree whose session is gone. Ending or archiving a session
  tidies nothing: `session.resume` starts it again in the same folder.
- `session.resume` of a session whose folder is gone (a worktree a
  person removed, or one tidied with the session it was made for, or a
  folder removed by hand) is `conflict`, naming the folder; so is one
  into a worktree whose setup, teardown or removal runs. Nothing of the
  session changes.
- Tidying keeps a worktree git could not inspect (`kept {
  inspection_failed }`) and does not try it again; a person removes it
  with `snapshot` or `discard`. One kept because a session was in it is
  tried again once that session is gone.
- `SessionInfo.workspace` is filled on every answer that carries a
  session's info: `session.list`, `.get`, `.create` (and an import's),
  `.move_profile`, `.rename`, `.subscribe`, `.resume` and `.mark`.
- An imported session's place is dropped (it named the other machine's
  project); its title comes with it, its marks and queue stay behind.
- Store: schemas 7 and 8 are the conversation's, 9 the workspace's.

## Version 9 (2026-11-11)

Nodes speak 2..=9; version 9 adds four methods, two events (with
stand-ins for older clients) and optional fields. Milestone 10's plan
(`docs/plans/backend-m10.md`, "Contract (protocol 9)", track C) and ADR
0022.

- Types (`vessel_proto::options`): the agent's own session config
  options, passed on as the agent reports them; Vessel writes no list of
  models, efforts or modes of its own.
  - `OptionId`, `OptionValueId`: the agent's ids, opaque, echoed back.
  - `OptionCategory`: `mode`, `model`, `model_config`, `thought_level`,
    `other` (ACP's categories; none, a custom one, and one a newer node
    sends read as `other`).
  - `SessionOption { id, name, description?, category, kind, .. }`,
    `kind` one of `select { current, values }` (each value `{ id, name,
    description?, group? }`, in the agent's order; `group` is the name of
    the group the agent files it under), `toggle { on }`, or `unknown`: a
    kind a newer node sends, which a client skips (without it, one such
    option would fail the decode of the whole list).
  - An agent with only ACP's older `modes` (and no option in the `mode`
    category) gets a `mode` option made from them (`session_mode` if the
    agent uses that id), set with `session/set_mode`.
  - `OptionChoice { id, kind, .. }`: what a client asks for, `select {
    value }` or `toggle { on }`.
- `SessionTitle`: a string on the wire, parsed: trimmed, not empty, one
  line without control characters, at most 120 characters. A bad one does
  not decode. A title taken from a prompt is its first non-empty line, cut
  at 80 characters.
- `SessionMode::Structured.options?: OptionChoice[]`: set in order once
  the session exists (after `session/new` or `session/load`), after
  `model`; one the agent does not offer, or refuses, fails the session
  (`failed`, the reason in its message), as an unknown model does.
- `SessionSpec.title?`: the title asked for at creation.
  `SessionInfo.title?`: the session's title now, the one truth for it:
  the spec's, otherwise (structured sessions) the first line of its first
  prompt with text, then whatever `session.rename` gave it last. The node
  keeps it across restarts. Absent until it has one, and from older nodes.
- A `session.create` choosing options or naming a title needs protocol 9
  (`Call::since`): an older node would start the session in the agent's
  default mode, or unnamed, rather than refuse.
- Events:
  - `options_changed { options }`: the whole list. Every structured
    session logs one right before its `ready`, as the options stand after
    the handshake's choices, empty when the agent offers none (so
    "nothing offered" is told from "not known yet"); then one whenever
    they change (a `session.set_option`, or the agent's own report of a
    change, also from the agent a profile move restarts).
  - `renamed { title }`: from the first prompt or `session.rename`.
  - A version 2-8 client gets a `harness_log` line in their place:
    `vessel: options: Model Haiku, Mode plan, Fast mode off` (each
    option's name and its current value's name, or its id when the agent
    does not list it; options of an unknown kind left out; `none` for no
    options), and `vessel: renamed to <title>`.
- Methods, all forwardable:
  - `harness.options { harness, profile? }` (read) returns `{ options,
    probed_at }`: what a harness's agent offers before any session of it
    exists. The node starts the agent under the profile (no prompt is
    sent), reads the handshake, and stops it; the answer is kept ten
    minutes, or until the harness reports another version, and callers
    asking meanwhile share one probe. A harness that cannot run
    structured sessions on the node answers an empty list. Errors:
    `harness_not_installed`; `spawn_failed` when the agent does not start
    or complete its handshake (not signed in, for example; the message
    has the agent's last lines), within the handshake timeout;
    `invalid_request` for an unknown profile or one of another harness.
  - `session.options { session }` (read) returns `SessionOption[]` as the
    agent last reported them, waiting for the handshake of a starting
    session. `unsupported` for a terminal session, `conflict` for an
    ended one (its log's last `options_changed` is what it ran with).
  - `session.set_option { session, choice }` (operate, ownership-checked
    like `session.send`) returns `SessionOption[]` after the change, while
    idle or mid-turn as the agent allows; a value in effect changes
    nothing. `invalid_request` for a choice the agent does not offer
    (nothing is sent to it), `conflict` for the agent's refusal (its
    message in the error's), `unsupported` for a terminal session.
  - `session.rename { session, title }` (operate, ownership-checked)
    returns the `SessionInfo`; any live session, terminal ones included.
    `conflict` for an ended one.
- Sessions that continue elsewhere keep what they run with:
  `TransferDetail::Session.options?` (`session.export`) carries the
  session's choices as they stand (its last report's values; before any,
  its spec's), and `session.import` applies them after the load, failing
  where the agent there does not offer one; the title travels in the
  exported info and is kept. A profile move applies them to the restarted
  agent likewise.

## Version 8 (2026-11-11)

Nodes speak 2..=8; version 8 only adds one optional field, so there is
no version gate: a version 2-7 client ignores it, and a version 8 client
reading an older node finds it absent. Milestone 9's plan
(`docs/plans/backend-m9.md`, track A) and ADR 0019.

- `SessionInfo.board?`: a session's place on the board, derived by the
  node and never sent by a client: `{ column, display_status, activity,
  last_activity_at }`.
  - `column`: `building`, `validating`, `needs_review`, `ready` or
    `archive`. `archive` is not a lane: clients list those sessions
    apart.
  - `display_status`: the phrase for the card, derived inside `column`:
    `working`, `blocked`, `exited`, `no_signal`, `interrupted`,
    `awaiting_pr`, `idle` (building); `fixing_ci`, `addressing_comments`,
    `needs_review`, `review_scheduled`, `reviewing`, `review_failed`,
    `review_pending`, `draft` (validating); `ci_failing`, `commented`,
    `changes_requested`, `needs_human_review` (in review); `mergeable`,
    `approved`, `merged`, `closed` (ready); `terminated` (archive).
  - `activity`: `active`, `idle`, `waiting_input` or `exited`, what the
    agent was last seen doing.
  - `last_activity_at`: when the node last saw the agent do something:
    the handshake completing, a turn's start or end, a permission asked
    or answered, terminal output, the session's end.
- The model is agent-orchestrator's (`contract/kanban.go`), ported
  (`vessel_session::board::derive`): the column first, from lifecycle
  facts, then the phrase from what that column cares about.
- What a node reports today:
  - A live structured session is `building`: `working` while starting
    and during a turn, `blocked` while a permission request waits,
    `interrupted` after a cancelled turn until the next one starts, and
    otherwise `idle`.
  - A live terminal session is `building`: `working` while it prints,
    `idle` once it has printed nothing for five seconds.
  - Any ended session (`exited`, `failed` or `interrupted`, live in this
    run or read back from the store) is `archive` / `terminated`, activity
    `exited`, its end as its last activity. The client shows the status
    word on an archived card. (Version 10 changes this: an ended session
    is `building` / `exited` until it is archived; a version 8-9 peer
    still gets the placement above.)
  - No node gathers pull requests yet, so `validating`, `needs_review`,
    `ready`, `awaiting_pr` and the phrases of those columns are not sent;
    `no_signal` is not either (no Vessel session is expected to report on
    its own). They are in the contract so the next milestone adds facts,
    not wire types.
- Nothing new is pushed: `board` comes with every `SessionInfo` a node
  already returns (`session.list`, `session.get`, `session.create`,
  `session.move_profile`), current as of the call. Inside the node the
  session's info watch wakes on a change of column, phrase or activity;
  `last_activity_at` alone moves without waking it, as `next_seq` does.
- A client talking to a node older than protocol 8 gets no `board` and
  keeps the placement the session's status implies.

## Version 7 (2026-11-07)

Nodes speak 2..=7; version 7 only adds: the LSP contract (milestone 6)
and model routing (milestone 5), each appending its rows to the RPC
table with `since 7`.

### LSP contract (milestone 6)

The M6 LSP rows append to the RPC table with `since 7`:
`lsp.workspace_symbols`, `lsp.document_symbols`, `lsp.definition`,
`lsp.references`, `lsp.hover`, `lsp.implementations`, `lsp.type_definition`,
`lsp.incoming_calls`, `lsp.outgoing_calls`, `lsp.diagnostics`. All need Read
and are forwardable under the existing scope intersection. The runtime serves the implemented capability subset and returns
`unsupported` when the selected server does not advertise a feature.

Each request selects `workspace: {root, language}`. Languages are `rust`,
`type_script`, `python`, `go`, `cpp`. Files are relative paths without
traversal. Positions use 1-based lines and Unicode scalar columns; ranges
have exclusive ends. Page requests default to offset 0, limit 50; the node
caps limits at 200. Pages are sorted but may shift when files change.

Responses carry context: covered files, optional observation time (the oldest contributing cached report), freshness
(`saved`, `cached`, `unknown`), indexing (`unknown`, `busy`, `observed_idle`)
and explicit workspace completeness. An empty diagnostics cache is unknown,
not clean. Hover is bounded untrusted plaintext/Markdown; clients disable
raw HTML. Server JSON never appears in this contract. ADR 0016 and the M6
plan define confinement and the operator-only full-analysis opt-in.

### Model routing (milestone 5)

`docs/plans/backend-m5.md`, ADR 0015. `model.list` follows the LSP rows
in the RPC table.

- Types: `ModelRouteName` and `ProviderName` (a route's and a provider's
  name in a node's `models.toml`: 1-64 letters, digits, `.`, `_` or `-`);
  `ModelApi`: `anthropic_messages`, `openai_responses` or
  `openai_chat_completions`; `TokenUsage { input, cache_read, cache_write,
  output, reasoning }`, each a count of tokens, every field defaulting to
  0, where `input` excludes cached tokens and `reasoning` is part of
  `output` (each API is converted to this at the node's proxy);
  `ModelStop`: `end_turn`, `tool_use`, `max_tokens`, `stop_sequence`,
  `refusal`, `pause` or `error`; `ModelFailure`: `rate_limited`,
  `overloaded`, `upstream_error`, `refused_request`, `context_too_long`,
  `timeout`, `truncated` or `unauthorized`, the class of a failure and
  never the provider's message.
- `ProfileInfo.model?`: the route a profile names (`model = "<route>"` in
  `profiles.toml`); `SessionInfo.model?`: `{ route, model }`, the route a
  session was launched with and the provider's model its harness is pinned
  to. Both optional and absent from older nodes, so no version gate.
- Event `model_call { route, model, api, usage, stop, failure?,
  duration_ms }`: one per call a session's harness made through the node's
  proxy, metadata only (never prompt or completion content, tool arguments
  or keys). A version 2-6 client gets a `harness_log` line in its place at
  the same position: `vessel: model call to <model>: <in> in, <out> out`,
  `<in>` counting cached input, or `vessel: model call to <model> failed
  (<failure>)`.
- `model.list` (read, forwardable) returns `ModelRoute[]`: `{ name,
  provider, api, model, small_model? }`, the routes the node offers. No base
  URL, header or secret name crosses the wire. There is no usage row:
  usage is in each session's log, and clients sum it.
- `session.create` under a profile naming a route the node's
  `models.toml` does not have: `invalid_request`, the route named.
  `session.move_profile` keeps a session's route: a move to a profile of
  another route, or between a route and none, is `unsupported`.

## Version 6 (2026-11-06)

Nodes speak 2..=6; version 6 only adds. Milestone 4's F3, F4 and Track X
implement it (`docs/plans/backend-m4.md`, "Protocol 6 (accepted)"), all
built: the envelope, `fleet.grant`, `fleet.callback`, `fleet.forget`,
`fleet.peers`, `fleet.nodes`, `session.read` with `wait`, the transfers
and the secret rows. (Until each landed, its methods answered
`unsupported`, "not built on this node yet", after the scope check like
every method.)

- Scope `relay`: passing calls on to other fleet nodes. Independent of
  `sync`, and a member gets it only when the home's owner asks (`vessel
  fleet invite --relay`); `vessel fleet invite` grants `sync` alone, never
  `admin`. `device.invite` naming `relay` needs protocol 6 (`Call::since`),
  and `device.list` leaves `relay` out for version 2-5 clients.
- Envelope (Track X): `fleet.forward { node, call }` (read) asks the
  caller's own node to run `call` on fleet node `node`; `fleet.relay {
  node, call, origin }` (relay) is a member asking its home to pass it on,
  refused when `origin.node` is not the member's own node; `fleet.deliver
  { call, origin }` (read) is the home having a member run it, the last
  hop, accepted only from that member's home. All three return
  `Forwarded { reply }`, the inner call's reply; an inner call that fails
  fails the envelope with its error. `origin` is `{ node, by, scopes }`,
  `by` one of `{"kind":"owner"}`, `{"kind":"device","id"}`,
  `{"kind":"agent","session","depth"}`. The target runs the inner call
  with its delivering device's scopes intersected with `origin.scopes`,
  and applies ownership and the agent limits to the origin. Subscriptions
  and the envelope's own methods cannot be forwarded
  (`Method::forwardable`). A call needs protocol 6 on the connection it is
  sent on; the inner call's version is checked where it is delivered.
- Fleet: `fleet.grant { link }` (sync) hands the home a pairing link made
  on the member, so the home holds a credential there (a member's node
  sends it when it connects, with the scopes chosen at join, `read` and
  `operate` by default; the device the home's redemption creates is the
  only one the member takes `fleet.deliver` from, and leaving revokes it).
  A member grants once per join: if its owner revokes the home's device,
  the home stays cut off until the node joins again.
  The home keeps a record of every node id it bound to a key and a name,
  and of the node every member device granted for, revoked and replaced
  devices included, and answers `conflict` to a grant that would rebind a
  device, bind an id to another key or a key to another id, rename a
  node, name the home's own id, or use a name the home or another member
  holds (also while that member's device is revoked). The same id and key
  through another device is the node joined again, and replaces the
  older grant. An id goes to a new key, or a node to a new name, only
  after `fleet.forget { node }` (admin, not forwardable; `vessel fleet
  forget <node>` against the home) and then only for a member device
  paired after it: the call withdraws the invitations outstanding on the
  home, a pairing that redeemed one before and completes after is
  revoked, and it returns `NodeForgotten { key, name?, devices,
  withdrawn }`. Any member device paired after it may then claim the id
  and name, and the first claim binds them, so the owner invites the node
  again before anything else and it joins from there. A
  refusal over a revoked or forgotten node says only that the name is
  reserved for, or the key bound to, another node, without ids or times;
  one over a live member's name names that member; the home's log has the
  details. `conflict` also while the same device has a grant in
  progress; `invalid_request` to a link with no route or more than 8; and
  `unavailable`, without details, when it cannot pair (within 30 s).
  `fleet.callback { node, key }` (sync; from a member device only, never
  forwarded) returns `Callback { relays, ticket? }`: the relays the home
  registers with and a registration ticket for `key` (`RelayTicket { key,
  expires_at, mac }`, never the relays' token; none when the home has no
  relay). A device the home's record binds to `node` and `key`, holding a
  current grant, gets a ticket for a day. One that has not granted yet gets
  one for ten minutes, only within an hour of its pairing (never reaching
  past that), only for a claim the record would take, and for one node and
  key, which the home records and its grant must then claim (`conflict`
  otherwise); others get `forbidden`. A member whose node has no route of
  its own asks on each connection, and again halfway to the ticket's
  expiry while connected, registers with those of the relays it dials (at
  most 8: any on a host it reaches its home by; otherwise `wss://`, not
  `localhost` nor a non-canonical numeric host, and only at public
  addresses its host resolves to when dialed) and names them, and only
  them, in its grant's link; another home's callback, or leaving, unlinks
  them. With none, it sends no grant and records why
  in its fleet state. `fleet.peers` (sync) and `fleet.nodes` (read)
  return `FleetNode[]` (`node`, `name`, `protocol`, `reachable`), the
  home included.
- Transfers (F4): `session.export { session, force? }` (operate,
  ownership-checked for agents) and `memory.export { profile?, project? }`
  (admin) snapshot files into a transfer and return `TransferManifest {
  transfer, files, detail }`, `files` in the bundle's shape (path to
  `{hash, size, executable?}`), `detail` `{"kind":"session",
  "harness_session", "cwd", "info"}` or `{"kind":"memory", "harness",
  "profile"?, "project"?}`. `transfer.read { transfer, hash, offset }`
  (read) returns a `BlobChunk` as `fleet.blob` does; `transfer.close {
  transfer }` (read) drops it; only the caller that opened a transfer may
  read or close it, and transfers expire after 10 idle minutes and at
  restart. `session.import { from: {node, session}, profile?, cwd?, start?
  }` (operate) pulls a session here and returns `SessionImported {
  harness_session, profile?, cwd, written, session? }`; `memory.import {
  from: {node, profile?, project?}, profile?, project? }` (admin) returns
  `MemoryImported { written, same, beside? }`, conflicting files written
  beside the existing ones. Limits: 256 MiB per file and 2 GiB per
  transfer (`MAX_TRANSFER_FILE`, `MAX_TRANSFER_TOTAL`); a transfer over
  either is `invalid_request`, naming the limit and the size.
  As built (F4): an import from another fleet node sends the source's
  `session.export` or `memory.export`, `transfer.read` and
  `transfer.close` through the envelope as the importing caller, so the
  source's grant to the home must cover them (`admin` for memory) and its
  transfer belongs to that delivery; a node in no fleet answers
  `not_found`. A profile with no harness home on the target yet is
  `conflict`, before anything is fetched. A `TransferId` is a UUIDv4;
  the holder check, not the id, keeps a transfer to its opener (another
  caller gets `not_found`, as for an unknown id). At most 16 transfers
  are open on a node (`unavailable` beyond). `session.export` of a
  terminal session is `unsupported`, of one whose agent is still starting
  `conflict`; `force` copies the transcript as far as it is written,
  cancelling nothing. `memory.export` without `profile` is Claude Code's
  default profile. `session.import` refuses (`conflict`) the profile and
  directory the session already is in on the same node, and a different
  file in the way, before writing anything; an unknown target profile is
  `invalid_request` before the source is asked; `start` needs the
  directory to exist (`invalid_request`). `memory.import` with `project`
  needs `from.project`.
- Secrets (F3), all admin: `secret.set { name, value }`, `secret.list`
  (names only), `secret.delete { name }`, `secret.copy { name, to }`. There
  is no `secret.get`: a value leaves a node only into another node's store.
  `secret.copy` sends `secret.set` to `to` through the envelope, so it
  needs the target's owner to have granted the home `admin` there.
  As built: `secret.copy` of a secret not set here is `not_found`, to this
  node itself `invalid_request`; a target that did not grant the home
  `admin` is `forbidden`, the message saying how to (`vessel fleet join
  --grant read,operate,admin` there), and the home itself, which grants
  its members only `sync` and `relay`, is `forbidden` with a pointer to
  `vessel fleet secret set` there. No error carries a value. The target
  is asked `secret.list` through the envelope first (forwardable for
  this, names only), and `secret.set` carrying the value goes only if that
  passes, so a target that would refuse the value never receives it.
- `session.read` takes `wait?` (seconds, capped at 60, protocol 6): with
  nothing at `from` yet, the node answers as soon as a record lands there
  or the session ends, or when the wait ends (an empty page). An ended or
  stored session answers at once. Waiting reads may hold at most half of a
  connection's calls (32 of 64); one origin at most 8 on a node; all
  origins of one other node at most 16 there; and on the home, the reads
  one relaying member device passes on at most 16, whatever origins they
  name. A read past any of these answers at once, as without `wait` (passed on to
  another node, it goes without its wait). A client that gets an empty
  page before its wait could have ended should pause before asking again.
- `SessionInfo` takes `via?` (the home a creating call came through) and
  `origin?`: `{"kind":"remote","node","by"}` (`by` owner or `{"kind":
  "device","id"}`) for a session created by a forwarded call, `{"kind":
  "imported","node","session"}` for an imported one. Absent for a session
  of this node's own, so older clients see nothing new; `created_by` uses
  only the variants protocol 5 has.

- Relay registration and limits (the relay's own wire, not negotiated;
  2026-11-06):
  - `register` may carry `ticket {key, expires_at, mac}` instead of
    `token` (exactly one of the two, else `rejected {reason: "token"}`):
    HMAC-SHA256 under the relay's token over `"vessel relay ticket
    v1\0"`, the key's 32 bytes and `expires_at` (seconds since the Unix
    epoch, 8 bytes big-endian). A fleet's home mints one for a member's
    key (`fleet.callback`). The relay takes it only for the key that then
    proves possession, unexpired and expiring at most 48 h ahead
    (`rejected {reason: "ticket"}` otherwise), and closes a registration
    made with it when it expires; the node registers again with a newer
    one.
  - A relay holding its most nodes answers a registration `rejected
    {reason: "full"}` (after the token or ticket and the proof); a node
    registered with a ticket holds a place like any other and frees it
    when the ticket expires. It closes a client or a still-registering
    control connection past its connection limits with `4503` (full,
    transient), and answers a socket past its limits at accept `503`
    before closing it, including one past its address's rate of new
    connections; through a trusted reverse proxy, where the client's
    address is known only from the request, that rate is checked at the
    upgrade and refused with `4429`. A connection that authenticates (a
    node registered, a node's data connection claimed, a client still
    spliced 7 s after pairing, which its node has admitted) gives its
    share of the rate back. An IPv6 peer is counted by its `/64`, `/56`
    and `/48` (the wider blocks with two and four times each per-address
    limit). A client of one node from one address past
    `--max-node-clients-per-address` while the node admits them is closed
    with `4503`. Behind a trusted proxy, a client-address header that is
    not one bare address is answered `400`.
  - Deadlines a peer must keep: a request's headers within 10 s; a
    node's first control message within 2 s of the upgrade; a client's
    first record (its handshake message) within 2 s of the relay pairing
    it with the node. When every open registering place is taken, a
    control connection holds none until its first message: a key
    registered at that relay before takes one of the places kept for
    known nodes, if one is free; any other is refused `full` at once.
  - A node gives a channel a relay spliced to it 5 s
    (`vessel_relay::RELAYED_ADMISSION`) for the Noise handshake and the
    device preamble together, and drops it otherwise (a direct channel
    keeps 10 s for each); at most 40 channels wait at once through each
    relay link, apart from 64 direct ones, of which one address (an IPv4
    address or an IPv6 `/64`) holds 7, a `/56` 14 and a `/48` 28; past
    those the carrier is dropped at once. Behind a trusted reverse proxy
    (`vesseld --trusted-proxy`) the address is the client's from the
    proxy's header, in the one form a relay takes (one bare address, else
    `400`; without it, the proxy's own). A client must send its preamble
    as soon as its handshake is done.
  - A node that does not know `full` or `ticket` reads the message as
    malformed and retries; a client that does not know `4503` sees a
    plain close; a relay from before tickets refuses a ticket as
    malformed. The limits themselves are the relay's configuration
    (`vesseld relay --max-*`, README).

## Version 5 (2026-11-06)

Nodes speak 2..=5; version 5 only adds, and older connections keep working
(below). Milestone 4 implements it (`docs/plans/backend-m4.md`); until its
tracks land, nodes answer the new methods with `unsupported`, record every
session as created by the owner, and never emit `profile_changed`.

- Every method has a first version (`since`, the table in `vessel-proto`
  `rpc.rs`): `node.stop` 3, `device.*` 4, the methods below 5, the rest 2.
  A call also needs a later version when it uses a later field
  (`Call::since`): `session.create` with `spec.profile`, `device.invite`
  with `scopes`. The client refuses such a call on an older connection
  without sending it (`CallError::NotInProtocol`, code `unsupported`), and
  the node refuses it (`unsupported`, "needs protocol N"), so neither side
  ever relies on the other ignoring a field it does not know.
- Profiles (ADR 0011): `SessionSpec.profile` (absent is the harness's
  default); `SessionInfo.profile` (the profile it runs with now) and
  `SessionInfo.created_by` (`{"kind":"owner"}`, `{"kind":"device","id"}`,
  `{"kind":"agent","node","session"}`; absent from older nodes, read as
  owner). `profile.list { harness? }` (read) and `profile.reload` (admin)
  return `ProfileInfo[]` (`name`, `label?`, `harness`, `sets`: variable
  names only, never values). `session.move_profile { session, profile?,
  force? }` (operate) returns the session. Event `profile_changed { from?,
  to? }`. Errors: `session.create` naming an unknown profile, or one of
  another harness, is `invalid_request`; `profile.reload` against a file
  that does not parse is `conflict` (the profiles in effect stay, and the
  message says where without quoting a value). `session.move_profile`:
  an unknown profile or one of another harness is `invalid_request`; a
  terminal session, or a harness whose session files Vessel cannot move,
  is `unsupported`; a running turn without `force`, a target profile that
  is not logged in, an ended session, a session that is still starting,
  missing or conflicting session files, and input sent while a move runs
  are `conflict`; an agent that does not start under the new profile is
  `spawn_failed` (the session goes on under its previous profile, with
  no `profile_changed`).
- Fleet (ADR 0012), scope `sync`: `fleet.manifest` returns the `Manifest`
  (`generation`, `files`: bundle path to `{hash, size, executable?}`,
  `mcp`: server name to transport, with `{"secret": "NAME"}` references,
  never values); `fleet.blob { hash, offset }` returns `{data (base64),
  total}`, at most 1 MiB per chunk; `fleet.watch { after }` is a long poll
  that returns the home's generation as soon as it differs from `after`
  (older too, so a member follows a home whose data was reset), or `after`
  itself when the home's wait limit (50 s) passes; `fleet.report { generation, conflicts? }` returns `ack`.
  `fleet.members` (admin) returns `FleetMember[]`.
- Bundle paths: `instructions.md` is the home's instructions file, which
  each member writes under its harness's own name (`CLAUDE.md`,
  `AGENTS.md`, `GEMINI.md`); `skills/<name>/...` is a skill, `<name>` being
  the `name` in its `SKILL.md` (not its directory name). Other top-level
  paths are reserved; a member ignores what it does not know. A path
  segment may not be a Windows device name (`CON`, `COM0`-`COM9`,
  `CONIN$`, ...) or a short-name alias (`NAME~1`). There is no mode beyond
  `executable`: members write fleet files owner-only.
- Scopes: `sync` is new. `device.invite` takes `scopes?`; without it a
  device gets `read`, `operate` and `admin`, as in version 4.
- Agents (ADR 0013): on the local socket, an agent's `vessel mcp` sends
  `{"type":"agent","session","secret"}` as its first frame instead of the
  token (a token is never a JSON object). A credential that is unknown,
  wrong, or of an ended session is answered like a wrong token
  (`rejected`, `unauthorized`); an admitted connection closes when its
  session ends. An agent has its session creator's scopes, at most `read`
  and `operate`; `session.send`, `session.kill` and `session.move_profile`
  on a session it did not start (directly or through sessions it started)
  are `forbidden`, as are a ninth live session of its own and a session
  four levels of agents deep. Sessions it creates record `created_by:
  {"kind":"agent","node","session"}`.
- Older connections: a node adapts what it sends to the negotiated
  version, where it encodes frames. `profile_changed` reaches a version 2-4
  client as `agent { harness_log { line: "vessel: profile changed from A
  to B" } }` at the same `seq` (the log stays gap-free), in subscriptions
  and in `session.read` pages alike, and `device.list` leaves out scopes
  the client's version does not have.
- Vessel's own notices in a session's log are `agent { harness_log {
  line: "vessel: ..." } }` records, in terminal sessions too (for example
  right after `starting`, an MCP server left out because this node lacks
  a secret, naming the secret, never a value). A terminal client that
  shows only terminal output can ignore them; `vessel log` prints them.

## Version 4 (2026-11-06)

Nodes speak 2..=4; version 4 only adds. Milestone 3 (relay, secure channel,
pairing) implements it; until then nodes answer `device.*` with
`unsupported` and send no `public_key`.

- Methods: `device.invite { label?, ttl_secs }` returns
  `{ link, expires }`; `device.list` returns `DeviceInfo[]`;
  `device.revoke { device }` returns `ack` and closes that device's
  connections.
- Scopes: every method declares one (`read`, `operate`, `admin`; the table
  in `vessel-proto` `rpc.rs`). Calls from a paired device without it get
  `forbidden`. The node token and the local socket hold every scope.
- `NodeInfo.public_key`: `"x25519:<base64url>"`, absent when unknown.
- Secure channel (ADR 0009): `Noise_NK_25519_ChaChaPoly_SHA256`, prologue
  `vessel/secure/1`, empty handshake payloads (both messages 48 bytes).
  Each carrier message is one record of at most 65,535 bytes: one sealed
  chunk, `flags` (`FIRST = 1`, `FINAL = 2`, others refused), the frame's
  `u32` big-endian length in a `FIRST` chunk, then data. Every chunk but a
  frame's last is full; frames are UTF-8 and at most 16 MiB. Any failed
  record or broken rule closes the channel. Details in
  `docs/plans/backend-m3.md` ("Seams", `vessel-secure`), implemented by
  `vessel-secure`.
- Relay possession proof: the same NK handshake, relay as initiator to the
  registering key, prologue `vessel/relay-proof/1`.
- Preamble inside the channel, before `hello`: the client sends
  `{"type":"device_auth","device","secret"}` or
  `{"type":"pair_request","invite","device_name"}`; the node answers
  `authenticated`, `paired` (with the new credential and `NodeInfo`), or
  `refused` with `unknown_device` | `revoked` | `invalid_invitation` |
  `unavailable`, then closes.
- Pairing link: `vessel://pair#` + base64url(JSON `PairingOffer`): node id,
  key, relay and direct routes (`ws://` or `wss://`), the single-use
  invitation, and its expiry. The offer sits after `#` so it never reaches a
  web server.
- Relay control connection (`vessel_proto::relay`): node sends `register
  {routing_id, key, token}`, relay sends `prove {message}`, node answers
  `proof {message}`, relay sends `registered` or `rejected {reason}`, then
  `incoming {conn}` per waiting client (tickets and the `full` refusal
  came with version 6: "Relay registration" there). Routing ids and connection ids are
  32 lowercase hex characters. Relay close codes: `4404` no such node,
  `4408` not accepted in time, `4413` too large, `4429` rate limited.
- Node close code on a secure-channel websocket: `4401` the handshake failed
  (most likely the client pinned another key). Relays forward a node's close
  code to the client unchanged. Clients treat `4401` as final: re-pair.
- Closing (2026-11-07): a relay or a node that closes a websocket with a
  code then reads and discards what the peer still sends, until the
  peer's own close or end, for at most 1 s, so the close is not lost to a
  reset (a socket dropped with unread bytes is reset, and on Windows the
  reset throws away a close frame still in flight). A peer should answer
  a close and stop sending; one that keeps sending is dropped after that
  second.

## Version 3 (2026-11-06)

Nodes speak 2..=3. Version 3 only adds, so version 2 peers keep working.

- `node.stop { "when": "now" | "when_idle" }` returns `ack`. `now` is the
  signal shutdown: live sessions end `interrupted`. `when_idle` drains: new
  `session.create` calls get `unavailable`, live sessions continue, and the
  node exits when none is live. A client that negotiated 2 never sends it.
  The `ack` is written before the node closes the connection, `now`
  included. (Builds before milestone 2's node side answered `unsupported`.)
- `NodeInfo.state`: `serving` | `draining`. When absent (a version 2 node),
  read it as `serving`.
- `vesseld attach` exit codes (`ProxyExit`): `0` closed, `80` no node
  answered and starting one failed, `81` data directory or token unusable,
  `82` token refused. `127` comes from the remote shell and means "not
  installed". Codes are never renumbered.
- Byte-stream framing, for SSH stdio and the node's local socket: each frame
  is a big-endian `u32` length, then that many bytes of UTF-8 JSON (the
  same text as a websocket frame). The maximum is 16 MiB, checked before
  allocating. On the local socket only, the first frame carries the token
  (ADR 0007), then the usual `hello`. A wrong or missing token is answered
  with `rejected` (`unauthorized`), and the connection closes.

## Version 2 (2026-11-05)

- Transport: one websocket at `/rpc`, JSON text frames.
  `Authorization: Bearer <token>` is required on the upgrade. `GET /health`
  stays open.
- Handshake: the client sends `{"type":"hello","protocol":{"min":2,"max":2},
  "client":{...}}`. The node replies with `welcome` (negotiated version plus
  `NodeInfo`), or with `rejected` and then closes.
- Calls: `{"type":"request","id":N,"call":{"method":"session.create",
  "params":{...}}}` is answered by `{"type":"response","id":N,"outcome":
  {"ok":{"method":"session.create","result":{...}}}}` or by
  `{"outcome":{"err":{"code":"not_found","message":"..."}}}`.
- Methods: `node.info`, `harness.list`, `session.list`, `session.get`,
  `session.create`, `session.send`, `session.kill`, `session.read`,
  `session.subscribe`, `session.unsubscribe`.
- Streams: `session.subscribe{session, from}` returns a subscription id. Then
  `{"type":"event","subscription":S,"record":{"seq":..,"at":..,"event":..}}`
  frames follow, ending with `subscription_ended`
  (`session_ended` | `unsubscribed` | `lagged{resume_from}`).
- Ordering guarantees (ADR 0006): the node never sends `event{S}` before the
  `response` that carries `Subscribed{S}`, and sends nothing for `S` after
  `subscription_ended{S}`. For `session.unsubscribe`, the client receives
  `subscription_ended{unsubscribed}` before the call's `ack`. Records within a
  subscription arrive in `seq` order with no gaps; a gap is only ever
  announced as `lagged{resume_from}`. If the node stops serving a
  subscription for any other reason after announcing it, it also ends it
  with `lagged{resume_from}` at the first record not written, so the client
  resyncs instead of waiting.
- Limits: each connection has a bound on calls in flight. Calls over the
  bound are answered at once with `unavailable`; the node keeps reading the
  connection, so `cancel` and `session.kill` always get through.
- Session modes: `terminal` (base64 PTY bytes) and `structured` (ACP-derived
  `AgentEvent` timeline).

## Version 1 (Phase 0)

REST routes and one websocket per session stream. Replaced entirely by
version 2.
