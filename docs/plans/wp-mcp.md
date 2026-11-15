# Agent tools and orchestration skills

Contract: ADR 0065, extending ADR 0013. Status: implementation in progress.
Baseline: protocol 23, with the shared pending method table merged. No protocol or schema number is selected here.
Browser tools belong to the browser package; voice output belongs to voice.

## Reference and reuse decision

Modify the existing `vessel-mcp` server and typed node calls. The existing
`rmcp` transport, `NodeClient::call_on`, session principals, notification
follower, fleet bundle validation and store atomic writes are the assets to
reuse. A second registry, a new RPC transport or a dependency would duplicate
those responsibilities. Standard filesystem APIs and the existing hashing
and serialization dependencies suffice. No Paseo source or skill text is
copied; its behavior is the reference.

Reference paths below are relative to the Paseo 0.15.4 tree. `T` means
`packages/server/src/server/agent/tools/paseo-tools.ts`. These source sections
were read, including the handlers; the survey is an index, not proof.
All Paseo outputs are structured content plus model-visible JSON text.
Vessel baseline tools returned model-visible JSON text; current success responses also carry matching structured content. `?` means optional.

## Tool correspondence before implementation

This table describes the baseline, not a claim that the missing tools work.
`node?` in Vessel selects the local node or a fleet node by ID/unique name.
The package will also accept the explicit `machine?` spelling, refusing two
conflicting selectors. Authorization remains the destination node's.

| Paseo tool (source line) | Paseo parameters → result | Vessel baseline parameters → result / gap |
| --- | --- | --- |
| create_agent (1589) | title, provider/model, initialPrompt, labels?, settings?, workspaceId?, notifyOnFinish?=true → agentId, type, status, cwd, workspaceId?, currentModeId, availableModes, guidance | create_session: node?, harness, profile?, cwd/project/worktree?, model?, prompt?, terminal?, notify?=false → session, from?, delivered?; lacks async launch acknowledgement, title/options/labels input and remote finish callback |
| send_agent_prompt (1944) | agentId, prompt, sessionMode?, activeTurnBehavior?=steer, notifyOnFinish?=true → queued receipt/messageId/status/guidance | send_input: node?, session, text, delivery?, notify?=false → from/delivery; remote notify refused; no quiet-period mailbox or interrupt behavior |
| get_agent_status (2077) | agentId → status, snapshot including permissions | none; session.get exists but does not include pending permissions |
| get_agent_activity (3151) | agentId, limit? → agentId, updateCount, currentModeId, curated content | read_conversation/read_events: node?, session, cursor/from?, limit? → typed timeline/log page; no curated summary |
| list_agents (2127) | cwd?=caller, statuses?, sinceHours?=48, limit?=50, includeArchived?=false, includeBackground?=false → agents | list_sessions: node? → session list; baseline no equivalent filters |
| cancel_agent (2191) | agentId → success | none; session.send CancelTurn exists |
| archive_agent (2215) | agentId → success | archive_session: node?, session → marked session; owned sessions only |
| kill_agent (2244) | agentId → success | kill_session: node?, session → killed id; owned sessions only |
| update_agent (2265) | agentId, name?, labels?, settings? → success | none; session.rename / session.set_option exist; labels ingress is separate node work |
| set_agent_mode (3190) | agentId, modeId → success, newMode | none; session.set_option exists with provider-reported IDs |
| list_pending_permissions (3214) | none → permissions(agentId,status,request) | none initially; existing session.conversation permissions permit a current-state aggregate |
| respond_to_permission (3248) | agentId, requestId, response → success | answer_permission: node?, session, request, option? → answered; selected option/cancel only, ownership enforced |
| create_workspace (1381) | isolation, path?, projectId?, title?, background?, mode?, worktreeSlug?, branchName?, baseBranch?, branch?, prNumber?, forge? → workspace descriptor | create_worktree: node?, project, branch?, existing?, base?, setup?, no_wait?, timeout_secs? → worktree/setup result; no local adoption/background/PR placement |
| list_workspaces (1521) | includeBackground?=false → workspaces | list_projects(node?) / list_worktrees(node?,project?) → typed lists; different registry entities |
| archive_workspace (1545) | workspaceId → workspaceId, archivedAgentIds, removedDirectory | none; agent workspace mutation refused by ADR 0027 |
| rename_workspace (2307) | workspaceId?=caller, title → success, workspaceId,title | none; same authority restriction |
| list_workspace_scripts (2367) | workspaceId → scripts(lifecycle,port,proxy,health,terminal) | none; script.list exists but agent scripts refused by ADR 0031 |
| start_workspace_script (2391) | workspaceId,scriptName → script | none; script.run exists with same restriction |
| stop_workspace_script (2418) | workspaceId,scriptName → script | none; agent scripts remain refused |
| list_terminals (2444) | cwd?=caller,all? → terminals(id,name,cwd) | list_terminals: node? → caller-opened terminals only |
| create_terminal (2492) | cwd?=caller,name? → id,name,cwd | create_terminal: node?,session?=caller,title? → terminal; profile permission and ownership required |
| kill_terminal (2531) | terminalId → success | close_terminal: node?,terminal → closed result |
| capture_terminal (2562) | terminalId,start?,end?,scrollback?,stripAnsi?=true → lines,totalLines | read_terminal: node?,terminal,from?,wait_secs? → bounded text,next,dropped_before; no line-range capture |
| send_terminal_keys (2606) | terminalId,keys,literal? → success | write_terminal: node?,terminal,text?,keys?[] → written result; smaller closed key vocabulary |
| create_schedule (2647) | prompt,cron,timezone?,name?,provider?=caller,cwd?,isolation?,maxRuns?,expiresIn? → summary | create_schedule: prompt,every_secs or cron+tz,harness/spec or session,keep?,max_runs?,ends_at?,catch_up? → ScheduleInfo; local only; no name upsert/caller provider inheritance |
| create_heartbeat (2699) | prompt,cron,timezone?,name?,maxRuns?,expiresIn? → summary | create_heartbeat: same cadence args, own session inferred → ScheduleInfo; local only |
| delete_heartbeat (2750) | id → success (caller heartbeat only) | delete_schedule: schedule → deleted id; ownership checked but not heartbeat target |
| list_schedules (2771) | none → new-agent schedules only | list_schedules: none → visible schedules including heartbeats |
| inspect_schedule (2796) | id → stored schedule including runs | none; schedule.get exists, runs are a separate bounded read |
| pause_schedule (2819) | id → success | pause_schedule: schedule → ScheduleInfo |
| resume_schedule (2845) | id → success | resume_schedule: schedule → ScheduleInfo |
| delete_schedule (2871) | id → success | delete_schedule: schedule → deleted id |
| update_schedule (2897) | id,name?,prompt?,cron?,timezone?,maxRuns?,provider?,model?,mode?,cwd?,expiresIn?,clearExpires? → stored schedule | none; schedule.update exists with typed definition patch |
| schedule_logs (2969) | id → runs | schedule_runs: schedule → newest bounded runs |
| run_schedule_once (2995) | id → stored schedule | none; schedule.run refused to agents by ADR 0033 |
| list_providers (3016) | none → providers | none; harness.list exists |
| list_models (3037) | provider → provider,models | none; model.list is routing, not provider discovery; harness.options provides provider choices |
| list_profiles (3065) | none → named launch bundles and notes | none; profile.list exists but returns account profiles, not launch bundles |
| inspect_provider (3089) | provider[/model],cwd?,settings? → capabilities,modes,selectedModel,features | none; harness.options exists, without draft feature evaluation |
| list_hosts (1367) | none → self and registered hosts/status | list_nodes: none → fleet nodes and reachability; no independent host registry |
| speak (1314) | text → ok (voice feature only) | none here; voice owns it, including policy exemption |

Paseo forwards exactly seven tools (`T:709-717`) using `serverId`:
create_agent, create_workspace, list_workspaces, send_agent_prompt,
get_agent_status, get_agent_activity, list_agents. Its local host registers
remote finish subscriptions before accepting a prompt (`T:667-707`). Vessel
already forwards more tools through its authenticated fleet; no second
credential or caller-ID query is introduced.

## Policy, injection and notifications

Read `agent/paseo-tool-policy.ts:7-29`,
`agent/runtime-mcp-config.ts:7-67`, `T:735-741,1257-1292,1607-1681,1965-2049`.
Policy is a provider-specific denylist, absent means enabled. Filtering must
apply both to listing and direct invocation. Injection decisions are frozen
at launch; changing them does not rewrite a running agent's configuration.
Paseo config resolves fresh injection off (`config.ts:552-554`). Existing
Vessel installations retain injection on through the ADR 0065 migration.
Its HTTP sidecar has no consumer here and stays unimplemented/off; no new
listener is required for stdio parity. This addresses the original review's
consumer/default findings without exposing an unused transport.

Paseo has no agent wait tool. Completion/permission/error notices are the
normal delegation flow. Existing `wait_turn` remains a compatibility tool
until notifications work on all supported routes; it must not be advertised
as the required next action. Cross-node notification needs origin-side
following: the destination cannot prompt a session that lives elsewhere.
The existing local follower and its cancellation/ownership bounds are reused.

## Skills contract and ownership

Read `orchestration-skills/index.ts:13-65`,
`internal/controller.ts:42-211`, `internal/operations.ts:1-240`.
A single writer serializes status/reconcile/uninstall/selection/auto-update.
Selection is all or a normalized custom set. Status is not-installed,
up-to-date or drift; additions/updates/deletes are explicit typed operations.
Selection changes that delete directories require confirmed names. Reconcile
and startup maintenance never delete unselected skills. Startup does not
install into empty homes. Update preserves user-added files and modified
obsolete files; uninstall is a separately confirmed whole-directory action.
Filesystem manifests record ownership; crash recovery binds a transaction
to previous/next durable selection. Inputs name bundled skills, never paths.
Targets are explicit configured agent homes; libraries never read HOME.
No writes to the operator's actual agent configuration are part of testing.

Baseline inventory correction: `session.conversation` already returns current
pending permissions (`conversation/mod.rs:43-66`); an aggregate MCP read can
reuse it rather than adding a second node permission store or read call.
Missing node capabilities (one package-level report): cross-node finish
subscriptions, reloadable MCP launch policy and
inject setting, and skills status/reconcile/uninstall/save-selection/import
plus durable selection. Existing provider discovery/launch-bundle additions
belong to the provider package; workspace registry additions belong to the
workspace package. This package wraps only merged typed contracts, retaining
typed unsupported results while a dependency is pending. New methods remain
unregistered until the shared pending-row method table arrives. Protocol
assignment is requested once at integration; file-backed skills need no
SQLite schema change.

## Structure and checks

`vessel-mcp` owns schemas, input decoding and JSON result serialization.
`vessel-proto` owns bounded policy/skill/request/result values; no I/O.
`vessel-node` owns authority, launch snapshots, the serialized skill service
and notification routing. Store owns durable files; fleet's bundle rules
remain the shared path/skill-name validation. The existing client carries
requests through the same grant intersection locally and across nodes.
No new crate/dependency edge is planned. Fleet's portable skill directory-name
value is moved down to `vessel-proto::skills`; its frontmatter parser stays in
fleet. This reuses exactly the existing bundle-path check instead of creating
a second directory-name rule. Skill selection/status/request values add no
registered call or filesystem side effect. Invalid selections are refused,
where Paseo coerces malformed inputs to all: silently broadening installation
on malformed wire input is not accepted.

Per commit: fmt, size/boundaries and narrow changed-crate checks. One whole
root gate at package end on the exact commit through the remote gate. All
fixtures use temporary synthetic homes and fake harnesses. No native UI
changes or captures are required by this node-only package.

## Resume point

- Implemented in source: the 41-row source correspondence, shared tool IDs,
  existing-call session/schedule/provider wrappers, machine alias, immutable
  listing/direct-call policy, typed launch metadata and current permissions,
  matching JSON text/structured results.
- Implemented in source: private settings migration (fresh injection off,
  existing identity on, partial policy retained), partial MCP edits, exact
  provider launch snapshots and credential Mint/Omit rollback.
- Implemented in source: six embedded skills, private selection persistence,
  safe directory-relative sync, confirmed/frozen-plan deletion and patch
  recovery, serialized cancellation-independent node writer and five pending
  Admin-only local skill methods. This machinery is no longer test-only.
- Implemented in source: default completion notices and exact-turn remote
  receipt/follower calls. Their version, and all new settings/skills calls,
  remain MCP_PARITY=None; no protocol/schema number is chosen here.
- Checked: protocol tests 289/0/0 (a694cda2), store tests 85/0/0 (ee3be73e),
  repeated fmt/size/boundary checks. Strict Clippy and fleet/node/MCP execution
  remain incomplete; failed and queued checks are recorded separately.
- Pending: TS generation, strict and functional checks on the final integrated
  head, full root gate once, V3 review and the numbered consumer handover.
- Dependencies: W3's principal-bound agent creation receipts, N3's named launch
  bundles, W22's provider catalogue, W17's permitted agent host-directory view,
  and browser/voice producer readiness. No staged foreign contract is consumed.
  The coordinator's next node head must be merged when announced; pending
  method rows then belong at the end of its single rpc/table.rs.

## Bundled asset decision

The existing fleet file/path/name machinery supplies validation and hashing.
A small standard-library build script embeds the release's skill directory,
rather than reading a developer checkout at runtime or hardcoding another
list for all-selection. The bundle ships Vessel, advisor, committee, handoff,
help and plugin guidance written for Vessel's contracts. Paseo's text is not
copied. No build script installs files. Asset construction enforces named
file, count and total byte budgets before any destination writes.

## Cancellation and launch decision

Settings choices are validated before spawning. An admitted change owns a
bounded permit and a task tracked by node shutdown through file write and
in-memory apply; cancelling the requesting RPC cannot release either early.
The session launcher reads one exact-provider snapshot before credential
creation. A profile move can create or omit the next credential on the
existing grant, and failure restores its prior credential state, including
no credential. The node's data lock is acquired before migration and identity
creation. Startup configuration I/O runs on the blocking worker.

## Voice and browser consumer seams

Voice remains outside the core tool ID/denylist. `VoiceTools` is transient,
not deserializable from ordinary environment or prompts; the MCP constructor
accepts an explicit context. No voice handler is registered by that hook.
The voice producer must combine Enabled with readiness and an authenticated
exact-session active lease; core injection off must not suppress an explicit
voice-only attachment once that producer is integrated. Browser policy will
use `ToolId::Browser(BrowserTool)` when the browser contract is ready, reusing
its one name table; browser enable/qualified-engine/Operate/lease checks stay
with the browser producer. These are consumer seams, not runtime proof.

Reconciliation now applies only the additions/updates in its frozen status plan.
It does not rescan after a transaction captured paths, and it does not rewrite
selected directories outside that plan. Mutations borrow the service mutably
to express the single-writer requirement before node actor serialization.

Selection deletion staging now writes an owned, bounded journal before moving
confirmed directories beside their original names. A rollback refuses to
replace any recreated live name and retains the original staged directory.
Recovery binds the recorded canonical roots and durable selection; equal old
and new choices roll back unless the commit marker was written. Tree cleanup
preflights a named depth/entry budget and refuses special files. This remains
staged code: selection-save file backups and the node controller are not yet
wired, so it is not a completed transactional API.

The staged selection controller now captures the exact union of bundled,
previously managed and manifest paths before applying its frozen plan. Its
private per-transaction backups retain original bytes and permission bits,
with a total backup budget. Persistence runs last. Failed persistence restores
only paths still matching this transaction's own applied state; conflicting
external content retains both live data and backups. Recreated delete names
also refuse rollback rather than overwriting. Save retries convergence at most
three times and rechecks deletion confirmation on each fresh plan. A cleanup
failure after successful persistence retains the journal and reports the save
as committed, following Paseo's controller semantics. Synthetic-home tests
exercise confirmation, persistence failure and external-edit conflicts; their
execution remains pending.


## Pending node skill administration

Quality B permits the five typed `skills.*` rows. They use the existing
`MCP_PARITY: None` requirement, Admin scope, unary shape, and explicit
non-forwardable/agent-refused classifications. Client convenience methods
call the same typed rows; no protocol or SQLite schema number is assigned.
The node owns one bounded blocking writer for all status and mutation work,
including startup repair. A cancelled RPC retains its admitted permit, writer
and shutdown tracking until the operation finishes. Roots come from the
already captured `HarnessEnv`: shared `.agents/skills`, configured Claude
home's `skills`, and configured legacy Codex home's `skills`. No library reads
ordinary environment, and initial startup never installs absent skills.
Selection import only fills an unset preference. Uninstall keeps that
preference for a later explicit install. Changes made by other processes are
not serialized by this actor; journal rollback detects conflicts and retains
the live content plus recovery material.

Implementation checkpoint: the staged service is now product-wired behind
pending methods. Narrow compilation and tests, TS generation, cross-node
finish notifications and the whole package gate/review remain outstanding.
This supersedes earlier progress bullets describing the service as test-only.


## Completion notifications and exact remote turns

A structured own-session tool context now defaults `notify` on and hides
`wait_turn` from the advertised catalogue. The old direct wait adapter remains
for compatibility; it still passes through the same tool policy. Send defaults
to steer when the current canonical conversation permits it, otherwise
interrupt; explicit delivery keeps its original meaning. Create validates all
prompt/notify options before starting a session. A notifying remote create
first calls pending `session.check_prompt_tracking` across the whole route,
so an older/unsupported hop refuses before the creation side effect.

The existing `Delivered` result does not identify the exact turn started by a
queued/steered prompt. Guessing the next log turn could report a different
concurrent prompt as complete. Three pending typed receipt calls reuse the
existing `Sessions::prompt_tracked` oneshot: `session.prompt_tracked`,
`session.prompt_receipt`, `session.close_prompt_receipt`. Operate ownership
checks still act on the target session. Receipts also bind the authenticated
opener (including delegated device and origin), with 32 per opener, 256 per
node, 32 long-poll readers and a fixed 30-minute expiry while waiting to start.
No receipt is an authority token; no receipt or notification is durable over
node restart. Closing/expiry cannot grant another caller access.

The calling node derives the parent only from its direct Agent principal,
reserves the existing notification budget, and follows that receipt's exact
turn through the normal grant-intersected fleet route. It reuses the local
fenced completion/permission wording, interrupted-parent rule, rate bound and
parent-end/shutdown cancellation. The setup and follower are tracked across
RPC cancellation. Read failure reports notification unavailability rather than
success. Receipt closure after an interrupted route may defer to expiry.
No new callback can choose or prompt an arbitrary parent. Typed pending rows
and nested call admission share MCP_PARITY=None until integration.

Narrow strict feedback from the security checkpoint was corrected without
lint suppression. TS also found an inherent SkillOperation::name collision
with the TS trait, so the shared identity accessor is now `skill()`.
Current checks are still pending; none of these source changes claims a
cross-node runtime or whole-package PASS.


## Existing-call launch/status refinements

`create_session` accepts a validated title, typed structured option choices and
the existing bounded label map. These populate SessionSpec rather than a new
launch API; the shared client admission refuses unassigned lifecycle labels and checks
assigned field requirements before transmission. Options are bounded, nonempty and unique by ID, and
terminal sessions refuse structured model/options inputs before creation.
`get_session` retains its existing metadata fields and adds current structured
`permissions` from GetConversation, with a finite complete-result cap. The two
reads are sequential, not an atomic snapshot. Provider-discovery/launch-bundle
and non-fleet host-directory facades remain dependent on their owners'
reviewed contracts. Account profiles do not stand in for launch bundles, and
an agent never reads an Owner's private host directory by default.


Successful MCP results now reuse rmcp's structured-result constructor, which
carries the same value as JSON text and structuredContent. The shared protocol
owns the SessionDetails serializable output; the MCP adapter adds no serde
dependency or parallel wire shape. Regression helpers compare both result
channels on every successful integration call. Malformed creation tests assert
the adapter's actual invalid-params response and unchanged node session count.


The first pure protocol run executed 289 tests: 286 passed and three failed
on pinned policy expectations. The exact Admin and forwardable expected lists
now add only this package's named methods. Local notification remains protocol
11, while the forwarded-notify test explicitly expects MCP_PARITY rather
than inheriting 11. The failure logs are retained. A new synthetic-home
interruption test recovers applied patches against the unchanged durable
choice; persistence-failure coverage also pins Unix permission restoration.
These changes do not constitute a successful rerun yet.


Migration now treats only an explicit `mcp.inject` value as the migration
marker. A partial provider-policy object does not silently turn tools off on
an existing identity: the missing switch receives the fresh/upgrade default
while the exact stored policy remains. Synthetic fresh/existing cases pin
both policy retention and absence of accidental identity creation.


A settings cancellation fixture now holds the actual settings writer lock,
admits an injection edit, cancels only its requesting task and verifies the
permit/shutdown tracking remain held. After releasing the writer it checks
both persisted choice and launch watch state. It creates no session or agent
process. The selection rollback mode fixture explicitly sets its source mode
before asserting restoration; execution is still pending.


Exact a694cda2 protocol tests pass 289/0/0. Store tests pass 84 and fail
one new migration fixture because it used the human alias `claude` instead
of the canonical settings-wire value `claude-code`. The fixture now uses the
wire value; policy retention/default/identity assertions are unchanged. A
store rerun is required; the earlier failed evidence remains intact.


Status also compares the current owned-file manifest with the compiled asset
set. A release that only removes an asset must still produce an update even
if SKILL.md and every remaining asset are unchanged. The regression preserves
an edited obsolete file while removing the unchanged copy in the other home.
Future manifest versions refuse before declaring an install current.


The private transaction root is derived from the prepared data directory's
canonical path, so a supported relative node data directory is not rejected
as a relative skill root. Manifest unknown fields are refused rather than
being erased by an older writer. Unknown skill selections remain invalid
requests through both direct validation and the transactional save error.


Exact ee3be73e store tests pass 85/0/0. Its strict check stopped in the new
client adapter on a wildcard import and a documentation backtick lint; those
are corrected explicitly, without suppressions. Protocol's earlier 289/0/0
result is retained separately from that strict failure.


The reviewed gate40 landing now makes Call::since the sole lifecycle field
gate, including pending None. The temporary MCP-specific label refusal is
removed; the adapter only decodes/validates the typed value. Label mutation
and query facades still wait for the lifecycle owner's reviewed request types.


## Browser identity extraction

The shared ToolId now wraps BrowserTool and delegates exact name parsing and
formatting to its one closed catalogue. The protocol-only subtree is copied
unchanged from the reviewed browser foundation d6898cb5, with its reviewed
RFC-variant identity correction a833243c. No later human-input additions,
proxy, browser crate, RPC, settings or runtime are imported. The browser
producer's scoped strict/tests evidence does not substitute for this
package's own checks. Browser names remain absent from this server's list;
a direct allowed browser name returns a typed engine-unavailable failure.
A disabled name still fails the shared policy first. The producer will mount
its schemas/dispatch only with its own enable/readiness/lease checks.


The gate40 merge preserves settings task tracking, provider launch watch and
skill actor registration. All nine MCP pending rows are at the end of
rpc/table.rs; the node file's size record decreases to 626. The first node
unit command actually stopped compiling its MCP dependency on optional
Conversation.abilities; default steer selection now handles absence as
unsupported before falling back to interrupt. No failed command is counted
as a node test execution.

The bundled guides now preserve the reference workflows rather than only
naming them: profile discovery/fallback, one analysis-only advisor, two
contrasting analysis-only committee members, notification waiting, forwarded
skills, persistent follow-ups, task-preserving handoff without agent detach,
topology-specific help and version/trust-aware plugin work. Their instructions
use current Vessel tool names and do not invent unavailable creation/profile/
plugin APIs. Text is independently written for Vessel, not a renamed copy of
Paseo's implementation-specific SDK/CLI instructions. Build still embeds and
validates the directory catalogue without installing anything.


Exact 2055feb36 node library tests pass 250/0/0, including cancellation and
receipt authority fixtures. Its strict run found eight source lints after
the gate40 merge: redundant qualifications, borrowed error mapping, two
synchronous RPC adapters and unchecked receipt expiry. The synchronous
adapters now follow the existing ready-future RPC pattern; checked expiry
refuses before allocating a receipt if the finite platform deadline cannot
be represented. No lint suppression or assertion change is used. This is
not yet a successful strict rerun or whole-package gate.
