# Agent tools and orchestration skills

Contract: ADR 0065, extending ADR 0013. Status: implementation in progress.
Baseline: announced gate-53 node head `93970acb`, with the shared pending method table. This package selects no protocol or schema number.
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
Vessel baseline tools returned model-visible JSON text. Success responses carry
matching structured content where the negotiated revision permits that shape;
legacy array/scalar text is preserved without an illegal structured field.
`?` means optional.

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
The initial package-level gap report led to this package owning exact-turn
remote receipts, reloadable MCP launch policy/injection and skills status,
reconcile, uninstall, selection save/import and durable selection. Existing provider discovery/launch-bundle additions
belong to the provider package; workspace registry additions belong to the
workspace package. This package wraps only merged typed contracts, retaining
typed unsupported results while a dependency is pending. New methods now use the shared pending-row method table. Protocol
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
a second directory-name rule. Protocol selection/status/request values perform no filesystem I/O. Invalid selections are refused,
where Paseo coerces malformed inputs to all: silently broadening installation
on malformed wire input is not accepted.

Per commit: fmt, size/boundaries and narrow changed-crate checks. One whole
root gate at package end on the exact commit through the remote gate. All
fixtures use temporary synthetic homes and fake harnesses. No native UI
changes or captures are required by this node-only package.

## Current implementation and integration boundary

Gate40 ba45a6e3 and the green gate42 node head d432ef0f are merged. Nine pending rows are at the end of the single
`rpc/table.rs`: five local Admin skills calls and four forwardable prompt
tracking calls. `MCP_PARITY=None`; the integrator assigns the number at the
wiring merge. Skills JSON needs no SQLite schema number. No HTTP listener,
agent grant expansion, browser engine or voice playback is enabled here.

| Area | Current source | Remaining boundary |
| --- | --- | --- |
| Existing-call MCP facade | get_session with current permissions; cancel_turn; rename_session; set_session_option; get/update_schedule; list_harnesses; account list_profiles; harness_options; list_pending_permissions | Provider catalogue, named launch bundles and lifecycle query/update contracts remain with their owners |
| Tool edge | Typed ToolId, immutable exact-provider policy for list/direct call, machine alias, selector refusal, identical structured/text JSON success | Browser/voice producers must add their own admission and readiness |
| Launch | Title, typed bounded options, existing labels; default notify; steer when supported, otherwise interrupt | W3 owns asynchronous creation receipt acknowledgement; no parallel receipt store here |
| Settings | Partial mcp edits, private fresh/upgrade migration, snapshot before credential creation, cancellation-independent bounded persistence | Pending feature assignment, reviewed consumer handover |
| Skills | Shipped bundle, safe filesystem sync/recovery, single node writer, typed client calls, selection persistence/import | Pending feature assignment; human-facing confirmation belongs to the client |
| Remote notices | Origin-side exact-turn tracking and bounded receipt calls | Two-node runtime wiring is unverified while the feature is unassigned |

W3's principal-bound agent creation namespace, N3's named launch bundles,
W22's provider catalogue and W17's permitted agent host-directory view are
not consumer-ready. Person-only receipts and Owner/Device-only host
registrations do not become agent authorization. Account profiles and model
routing are not substitutes for launch bundles or provider discovery. The
lifecycle gate in Call::since is the sole label admission gate; there is no
second MCP-specific protocol check. Label mutation/query facades wait for
reviewed types. No unreviewed foreign runtime is adopted.

## Settings, cancellation and launch decision

Settings validate the complete new policy before admission. A permit and
shutdown-tracked task own each admitted write through blocking persistence
and in-memory/watch apply, even if its requesting RPC is cancelled. There are
at most eight admitted settings changes. Saved fields retain precedence over
flags, and omitted mcp subfields preserve existing policy and future stored
names. Unknown new denylist names are refused; unknown saved names survive.

Migration runs under the node lock before node.id creation, preserving an
explicit switch. Only an explicit inject value is a migration marker: a
policy-only saved object receives the fresh/upgrade switch without losing its
policy. Settings links and nonregular files refuse; unreadable/future files
reuse store's private atomic replacement and protective set-aside rules.
Startup configuration I/O runs on a blocking worker.

Each process freezes one exact-provider policy before credential creation.
Injection off or a disabled provider omits the core server; a later settings
edit cannot mix a new policy with an older launch decision. Credential
Mint/Omit uses the existing grant, parentage and depth. A returned profile
move failure restores prior credential presence or absence. Cancellation of
the older profile-move runtime is not newly proven by these changes.

## Bundled assets and host skill commands

Fleet's existing file/path/name rules and hashing are reused. SkillName is
shared through proto; fleet retains frontmatter decoding. A standard-library
build script derives the catalogue from directories and embeds six independently
written Vessel guides: Vessel, advisor, committee, handoff, help and plugin.
No build script installs files, no developer checkout is read at runtime, and
no second all-selection name list is maintained.

The guides preserve profile discovery/fallback, one analysis-only advisor,
two contrasting analysis-only committee members, notification waiting,
forwarded skills, persistent follow-ups, task-preserving handoff without
agent detach, topology-specific help, and version/trust-aware plugin work.
They use current Vessel tools and do not invent creation/profile/plugin APIs.
Paseo's SDK/CLI text is a behavior reference, not copied implementation text.

A bundle enforces portable names/paths, matching frontmatter, reserved
manifest and case-folding collision checks, 1 MiB per file, 256 files per skill,
16 MiB total and 128 skills. Whole selection files are capped at 256 KiB.
Malformed selections refuse rather than broadening installation to all.

The node captures explicit HarnessEnv homes: shared .agents/skills, configured
Claude home's skills and configured legacy Codex home's skills. Libraries do
not read HOME. One bounded blocking writer serializes startup maintenance,
status, reconcile, uninstall, save and import. At most eight commands are
admitted; cancellation cannot release their writer permit or shutdown task
early. This service is product-wired; the historical private module name
`staged` does not mean it is still behind cfg(test).

Fresh startup never installs absent skills. Reconcile/startup apply only
Add/Update operations in their captured plan and never remove unselected
directories. Import only fills an unset preference. Uninstall preserves the
saved choice for later explicit install. Status distinguishes selected,
available, installed and actual content, including releases that only remove
an owned asset. Modified obsolete and user-added files remain.

## Transaction and filesystem decision

The existing safe directory handles reject symlinks and special entries
below explicit roots; Unix operations use directory-relative no-follow I/O.
Windows retains the existing checked-path implementation and does not claim
Unix's open-handle race guarantees. State and skill roots must be disjoint,
checked lexically and after canonical opening. The node derives state from
the canonical prepared DataDir so supported relative data directories work.

A selection change confirms fresh removal names before staging whole
directories beside their original names. The durable owner/version/id journal
binds canonical roots and previous/next selections. It captures the exact
union of bundled, formerly managed and manifest paths, retaining bytes and
ordinary permissions in private per-transaction backups capped at 32 MiB.
Decoded journals, patches and changes have named count limits. Whole-directory
cleanup preflights depth 32 and 8192 entries before unlinking.

Persistence runs last. A failed apply/save restores only content and modes
still matching this transaction's own applied state, or already-original
state. Recreated live names and observed content/mode conflicts retain live data
and recovery material. External writers are not serialized by this service;
a writer racing after a content check is not covered by a general atomic
multi-process transaction guarantee. Late-writer/platform behavior remains
separate from the synthetic conflict checks. New directories are pruned only if empty. Save makes at
most three convergence attempts and reconfirms each fresh removal plan.
Recovery rolls back equal old/new selections unless the commit marker was
written. After durable selection save, cleanup failure retains a journal but
acknowledges the committed choice. Unknown/future manifests refuse rather
than erasing another writer's ownership information.

## Notifications and exact remote turns

A structured own-session MCP context defaults notify on and hides wait_turn
from the advertised list. Direct legacy waits still pass through ToolPolicy.
Explicit delivery is preserved; absent delivery chooses steer only when the
canonical conversation reports support, otherwise interrupt. Create decodes
prompt, notify, title, options and labels before creating a session. Structured
option IDs are nonempty/unique and bounded to 128; terminals refuse structured
model/options values.

A notifying remote create first negotiates session.check_prompt_tracking
across the full route, refusing unsupported hops or a stopping destination
before creating anything. This is capability negotiation, not an atomic
reservation of all subsequent effects. The existing Delivered value cannot
identify which concurrent queued prompt started a turn. Three typed calls
therefore reuse Sessions::prompt_tracked's exact oneshot:
session.prompt_tracked, session.prompt_receipt and
session.close_prompt_receipt.

Operate ownership applies to the target session. A receipt additionally
binds its authenticated opener, including delegated device and origin. Limits
are 32 per opener, 256 per node, 32 waiting readers, a 30-second read wait and
30-minute retention. Checked platform deadline construction refuses before
allocating a receipt if that finite expiry cannot be represented. IDs are
not authority tokens. Storage is ephemeral, with no restart delivery promise.

The origin derives the notification parent only from its direct Agent
principal, reserves the existing follow budget, and uses normal fleet grant
intersection to follow that exact turn's log. It reuses fenced output,
completion/permission wording, rate bounds and interrupted-parent behavior.
Parent end and node shutdown cancel following. Setup and accepted following
outlive requester cancellation. A read failure reports unavailability rather
than success; lost receipt closure leaves only fixed-expiry bounded state.
Local notify retains protocol 11; forwarded notify additionally needs
MCP_PARITY. No callback may choose an arbitrary parent.

## Browser, voice and display seams

ToolId::Browser wraps the browser producer's one closed catalogue. The proto
subtree is unchanged from reviewed d6898cb5 plus the reviewed RFC-variant
identity correction a833243c. Provenance hashes cover all 16 copied files.
No later human-input changes, proxy, browser crate, engine or RPC is imported.
Browser names are not listed by this server; allowed direct names return
engine-unavailable, and disabled names fail policy first. The producer owns
future schemas/dispatch plus enabled-session/qualified-engine/Operate/lease
checks. Its scoped source/check evidence is not this package's whole gate.

VoiceTools is transient, not deserializable from ordinary environment or
prompts, and independent of core ToolId/denylisting. The explicit constructor
hook grants no playback authority and registers no handler. Voice needs its
producer's protocol/readiness/exact-session active lease checks. Core-off
voice-only launcher attachment still awaits the shared launch/reload context.

The MCP edge returns matching model-visible JSON and structured content.
Canonical semantic timeline action presentation belongs to the provider
edge; native's humanized-name plus Input/Output fallback remains a stated
difference. This package does not make native inspect arbitrary MCP JSON.

The dead track's proposed vessel permit CLI is not retained or advertised.
The original allow/deny argument behavior remains a proposal; no missing-ID
shortcut ships here. Existing answer_permission requires one exact request
and session ownership. The unused HTTP transport also remains deferred.

## Verification ledger

Every source commit has format, size and crate-boundary checks; the current
size record is lowered to 626 for node.rs. No size debt is raised. Builds and
tests use serialized prescribed lanes, fake harnesses and synthetic homes.
No operator-home/service mutation is part of testing. Native settings UI
was assigned after the initial node-only phase and requires its own gate/captures.

| Exact source | Check and outcome | Evidence under /tmp/work/wp-mcp |
| --- | --- | --- |
| a694cda2 | proto 289 passed, 0 failed | value-tests-a694 |
| ee3be73e | store 85 passed, 0 failed | store-tests-ee3be73 |
| b4736cfab | fleet library 32 passed, 0 failed | fleet-tests-b4736 |
| 2055feb36 | node library 250 passed, 0 failed | node-tests-2055 |
| ba5bc191 | node command stopped compiling MCP Option abilities; no node tests executed; fixed 2055 | node-tests-ba5bc |
| ee3be73e | strict failed client import/docs lints; fixed b473 | strict-ee3be73 |
| 2055feb36 | strict failed eight source lints; fixed cb836 | strict-2055 |
| cb836083a | strict failed unused synchronous receiver; fixed c7f0 with shutdown negotiation | strict-cb836 |
| cb836083a | MCP suite stopped compiling a borrowed ID assertion; fixed c7f0; no MCP tests executed | mcp-tests-cb836 |
| 7bfc7f72e | TS failed three unclassified aliases; fixed 146c83 | ts-7bfc7 |
| 9e2be4d25 | TS generators 4 passed; stale file diff keeps step rc1; six artifact hashes verified and exact generated patch applied | ts-9e2be |
| c7f0c5df1 | Strict stopped on a module-doc backtick lint; fixed 3ded; functional suite stopped after 91 passed/1 failed/1 ignored at the new default-notification fixture | strict-c7f0, functional-c7f0 |
| 24fd5a5a5 | TS freshness passed: generator tests 4/0, wire diff 0 | ts-24fd5 |
| 3ded9d636 | Strict stopped on two redundant Ack qualifications in the RPC test mock; fixed without assertion changes | strict-3ded |
| c17b45819 | Interrupted by external signal143; completed node lib263/0, fleet lib32/0 + fleet integration32/0; two MCP fixture failures retained; full suite incomplete | functional-c17b |

The alias registry brands node-minted PromptReceiptId; SkillName and
StoredTool remain plain names validated at the Rust boundary. Generated TS was applied from the verified remote artifact after confirming
its source commit and unchanged proto/web generator input. The local file
matches the artifact byte-for-byte. The subsequent exact-24fd5a5a5 freshness check passes with no diff.
Regeneration with a diff remains distinct from that green freshness result.

New regression coverage awaiting the final successful functional run includes delegated identity separation on
both receipt read/close, cancellation returning long-poll reader budget,
paused-time expiry reclaiming admission, default MCP create/send delivering
notices into the actual parent queue without wait tools, and actual shipped
skill install/confirmed removal/uninstall/restart/import/reinstall through
the node worker/store. Those fixtures do not claim two-node runtime or
process-kill crash proof.

Failed logs are retained. Canceled own local waiters never reached compilation
and are not evidence. V3's earlier security checkpoint of 504161f9 returned
one strict-source finding; subsequent runtime wiring and the changed exact
strict result still need source review. Strict success, final functional checks, one whole root A12 gate and package review remain
required before a numbered consumer handover. Source review, scoped test
success, runtime behavior and integration readiness are reported separately.

The gate42 merge brings ProcessLifetime, RPC check_call and the transient
AgentSetup.system_prompt carrier. This package adds no AgentSetup literal;
the shared base supplies its explicit None fields. Unsupported prompt
overrides stay unsupported and voice is not activated. The only merge
conflict joined independent architecture sections, preserving both. Wire
roots are unchanged, so the previously generated MCP artifact remains exact;
merged-head checks are still owed. No whole gate was running during merge.

The first default-notification fixture timed out. Source inspection found
its arbitrary second prompt enters fake-acp's permission-request path, and
an occupied parent can receive a notice in QueueChanged rather than a new
TurnStarted. The fixture now uses the fake's distinct deterministic cwd
report, an explicit temporary cwd, the existing node-notice test's typed
queued-or-started observation, and named per-stage deadlines. Both exact
completion replies and the absence of a wait-tool call remain asserted.
This correction does not yet prove the product path passes; rerun is owed.

The tracker records built source or skeleton gaps only, never a package
PASS. The coordinator confirmed native Skills settings (tracker row 594) belongs
to W18 in a separate app tree; W15 owns its onboarding consumer only. App
implementation and gates are required before package completion. A node-only
check is not native UI evidence.

The interrupted c17 run also contains actual failures before cancellation:
the notification fixture used the private node data directory as cwd (correct
Forbidden), and its permission fixture observed two legitimate requests after
a default notice made the parent fake request permission. The notification
fixture now uses a separate temporary placement. The permission-lifecycle
fixture explicitly opts out of notices, preserving its exact one-request and
empty-after-answer assertions; the separate default-notification test keeps
coverage of notify defaults. Neither actual failure is dismissed as a signal
error. New admission/cancellation/restart node unit fixtures completed in the
263/0 node target, but incomplete e2e/proto/store targets are not claimed.

The exact 01cc8162e functional batch completed (1,104 passed, two failed,
three ignored). Default MCP notifications and the isolated permission lifecycle
now pass; the remaining two failures are ADR 0038 settings fixtures predating
startup injection migration. The damaged-file test now checks the fresh Off
marker after preserving the unusable bytes. The later-build fixture explicitly
pins injection so its original copy-on-first-user-change assertions remain
isolated, and checks injection survives subsequent partial edits. No backup
assertion was weakened. The exact strict batch returned seven test-helper
lints; helpers are explicitly test-only and the event filter enumerates ignored
variants. Failed logs remain under /tmp/work/wp-mcp/{strict,functional}-01cc.
The node tree also merged the green ede20014 base with no whole gate running;
that union is not covered by the older batch. Native UI compilation is queued
separately, and the pure view/link transition and fake-owner tests passed 9/0.

## Process-only CLI suppression

Both `vesseld --no-mcp` and `vessel node start --no-mcp` accept the visible
`--no-inject-mcp` alias. They omit this process's existing VesselMcp launcher
without writing the saved injection choice or changing provider policies. A
later normal start follows the saved decision. In Paseo, --no-mcp also disables
the HTTP endpoint; Vessel has no such endpoint, so both spellings suppress the
stdio attachment. Parser regressions keep the flags out of non-start commands.
The independent cd25ddd1c binary-parser extraction is on the clean node head
for early integration, separate from these behavior changes. Its strict/help
checks and the flags' merged-head checks are pending VM execution.

## Legacy desktop selection difference

Coordinator ruling for tracker598: Vessel has never written a native desktop
skill-selection file. There is therefore no automatic legacy file migration,
and the app must never read Paseo's actual Electron profile. Inventing a
historical Vessel path would not be a compatibility feature. The typed node
import remains available for an explicitly supplied legacy selection: import
only if unset, return the durable choice and Imported/AlreadySet acknowledgement,
and preserve an existing explicit choice across restart. Those behaviors are
covered in the node manager/store and actual host-command fixtures; their
narrow results and final whole-gate status remain separately reported.

The positive first-import path now also has an actual node-worker regression:
unknown catalogue names refuse without persisting; explicit empty custom imports
acknowledge Imported without installing; a restarted worker acknowledges
AlreadySet and retains that choice when All is imported later. This new fixture
is not covered by the older queued 1b32 batch; its latest-head run is owed.

The exact 1b32 strict batch completed with eight further lints, all in the two
new MCP integration helpers provider_discovery and drive_session_and_check_ownership.
They now carry explicit cfg(test), preserving every shape and ownership assertion.
Earlier seven-helper errors no longer appear. This failed run does not establish
an overall strict PASS; latest union/flags/import checks remain owed.

The exact 1b32 functional replacement reached the short lane's 600-second
limit: completed targets report358/0/1, including node lib263 and every MCP
integration target. Node e2e was only partway through comparison cases; the
settings set-aside regressions and later proto/store targets did not complete
in that run. Therefore the old two settings failures are not yet called fixed
by execution. Remaining tests move to the ordinary lane on the latest union,
which also includes the signal base, startup flags and positive import test.

Reset integration invariant: a future whole-MCP configuration reset must
persist explicit Injection::Off plus an empty provider policy, rather than
remove the inject marker. Missing inject on an existing identity denotes a
legacy upgrade and would enable injection on the next start. An empty patch
means preserve, not reset; ordinary toggle sends only inject and keeps policy.
The typed reset shape and restart reason were sent to the config owner for
planning, without offering an unassigned method or claiming its implementation.


## Expanded catalogue and the CLI canary pipe

The exact merged-base functional run reached the actual CLI canary, then its
unchanged overall safety net expired in AgentTools. This was a timeout, not a
reported canary match. Inspection found that its real `vessel mcp` child ran
with debug logging while the fixture consumed stdout alone until reply 3;
stderr was consumed only afterward by wait_with_output. The pinned MCP service
logs its complete tool response at debug level. An expanded catalogue can fill
that pipe and block the next reply while the parent waits on stdout.

The fixture now drains stderr concurrently with the same request/reply exchange.
Every diagnostic byte is still captured for the same secret scan. The secret
assertions, stdin protocol, wrong-session refusal, child lifecycle and original
safety net are unchanged. No product transport, logging or timeout is changed.
The failed run and its logs remain preserved; exact corrected strict and canary
execution are still owed, so this mechanism is not yet called runtime-proven.


## Runtime security checkpoint corrections (row209)

The first return identified an unbounded final remote receipt close and input
bearing decoder/task diagnostic sources. The close now uses one owner-defined
five-second cleanup budget after follow completion, parent end or node stop.
The exact captured origin, destination, session and receipt are retained. A
lost close is explicitly Uncertain, never confirmed deletion or an automatic
retry; remote receipt expiry remains its existing bounded fallback. Returning
from supervision releases the local Following permit and tracked task.

JSON metadata errors retain their concrete serde cause privately, exposing only
category and line/column through Display/Debug and stopping the public source
chain. I/O failures similarly expose kind/OS code, not arbitrary custom text.
Selection owner failures retain their typed cause privately, covering both the
store-read boundary and persistence/rollback diagnostics. Blocking and tracked
JoinError causes remain private behind Cancelled/Panicked categories. Remote
follow and cleanup warnings contain only closed error codes/categories; parent
failure notices stay fixed. This changes no global RPC policy or panic hook.

Regressions exercise actual malformed owned manifests, transaction journals and
selection files, the actual selection save persistence failure, enclosing full
source chains and an actual blocking worker panic. The paused supervision test
holds real Following admission and real receipt reservations while a close
future never replies; completion, parent-end and stop must release local slots
within the cleanup budget without deleting unrelated receipts or claiming a
lost close succeeded. A captured production warning/parent-notice producer tests
peer-error canaries. These are mechanism tests with synthetic homes/peer futures,
not a claim of a live two-node run, global panic-hook sanitization or process-kill
crash recovery. Exact changed-source strict/tests and V3 closure remain owed.


Follow-up source review extended the lifetime correction to every reachable
owner wait. Failure-notice delivery and receipt close now share the same finite
five-second post-follow budget; a stalled notice never prevents the timer from
starting and yields Uncertain rather than a confirmed close. Initial lookup and
TrackPrompt also run under a single tracked setup owner with a named two-minute
budget plus parent-end/node-stop cancellation. Caller RPC cancellation still
leaves that admitted owner tracked. A withheld TrackPrompt response may already
have effects: timeout/stop report only unconfirmed setup, never absence of an
effect or a fabricated receipt, and no automatic new-key retry occurs.

The common setup mechanism enforces lookup completion before creating the
prompt future. Owned-slot/tracker tests hold each stage separately, abort the
caller, then exercise deadline/stop/parent-end. A held failure-notice regression
also proves release without even polling or claiming the later close. These
new cases correct the initial immediately-ready-after fixture gap; their exact
execution and reviewed-source closure remain owed.

The atomic default delivery integration remains with W23's SubmitPrompt contract
(row226 source passed, still TIMELINE_PARITY=None). Client-side ability discovery
is not evidence of its SteerOrInterrupt actor decision or retained receipt result.
This package awaits the checked assigned producer before replacing that facade.

After this correction, tracker row618 additionally requires finish-subscription
restart parity. Its work starts with an ADR0033 amendment and V3 contract review,
then reuses the existing notices/notify watcher rather than adding a watcher.
No subscription persistence implementation is included in this correction.

The parent-end receiver is subscribed before reading the parent's live snapshot,
so an end between that read and setup's first poll is retained. Already finished
parents refuse before reserving Following capacity or dispatching remote setup.
This uses the existing session-end stream and introduces no watcher or route.


### Exact runtime supervision check correction

Run `wp-mcp-runtime-narrow-20261115T152545Z-b853fdda` failed strict
checking solely because the concrete setup and follow supervision futures
exceed the workspace future-size bound. Both await sites now pin their concrete
future in a box, reducing the enclosing task state without changing the owner,
deadlines, cancellation branches, payload or effect ordering. There is no trait
object, lint exception or additional task. The failed log is retained. Its still
queued unit sibling was superseded only after terminal cancellation; exact
corrected strict and tests remain owed.

Separately, the unchanged CLI canary fixture at `0c0f23107` passed strict in
`wp-mcp-narrow-20261115T144644Z-0c0f2310` and the actual selected secret-canary
test in `wp-mcp-20261115T144645Z-0c0f2310` (one passed, none failed). The
AgentTools exchange took 164 ms with the complete diagnostic-byte scan. This
closes that selected test only; the preceding functional run remains
1317 passed, one timeout failure, four ignored. V3 has source-settled the
diagnostic and bounded-owner corrections through `b853fdda`; this does not
replace the exact changed-source execution or whole-package gate.

## Row 618: durable finish subscription proposal

The proposed ADR 0033 amendment records the restart difference against Paseo
0.15.4 finish-subscriptions, the existing watcher/Store reuse, a bounded private
NoticeKey and exact source identity, captured grant/current parent lifetime,
acknowledged replacement, finite storage and recovery admission. W23 owns the
missing durable actor-ticket binding and internal receipt namespace; public
SubmitPrompt cannot select it and Unobserved cannot license resend. W10's
canonical notification body remains unchanged. No persistence, schema number,
RPC or duplicate watcher is implemented. W23 alignment and V3 contract review
precede implementation; row 618 remains pending those dependencies.


W23 alignment adds explicit same-key fingerprint conflict/Capacity semantics,
immutable accepted notice EventSeq plus QueueEntryId, and no synthetic source
sequence for a pruned pending snapshot. The proposed private intent reserves
its entire original-request envelope and every bounded notice envelope/record
slot before storage or dispatch, through the existing W23 admission accounting.
Acknowledged/uncertain tombstones retain that charge until authorized whole
parent-session removal; there is no new eviction/TTL. The separately approved
ObservePromptReceipt remains W23-owned under its existing None gate.


The saved-original-request floor is explicitly finish-registration-only:
`MAX_FINISH_ORIGINAL_REQUEST_BYTES` checks the full canonical encoded request
against 64 KiB before registration/dispatch. W23 has no generic SubmitPrompt
64 KiB bound; its similarly sized native-command text limit is unrelated.
ObservationUnavailable irrevocably ends the observer, so the single extra
checkpoint slot is terminal OR unavailable. Retention releases accounting only
through committed authorized whole-parent-session removal; pruning, replacement
or revocation never releases it.

## Gate-50 node-base merge and settings writer join

The announced green node head `9c150c6e` is merged, not rebased. It carries the
reviewed binary parser seam, canonical settings retained writer/shutdown, new
workspace/detach/diagnostic contracts and portable timeline body values. Method
rows and exhaustive projections retain both sets; this package's nine pending
rows remain at the end, with MCP_PARITY None and no assigned number.

Settings persistence now uses the landed `settings/writer.rs` owned mutex guard,
blocking writer, retained writer tracker, write/apply publication and shutdown
join. The MCP field makes the saved patch non-Copy: the blocking closure keeps
its own cloned snapshot while the same guard survives through apply; policy
watch publication occurs only after persistence. The existing MCP operation
admission still owns pre-lock cancellation through its bounded tracked task.
That is admission for ALL settings operations, not a separate MCP file writer
or physical-I/O pool. Nodectl's future common admission lane will replace that
operation-admission boundary at its reviewed join; its unreviewed private
settings envelope/new lane are not adopted here. There is one actual persistence
implementation. Existing canonical real-blocking-writer/cancellation/shutdown
assertions remain, with MCP switch/watch and omitted-field preservation checks
added at the same real blocking gate. The startup read remains on its existing
blocking preparation path; the now-unused synchronous config wrapper is not
retained beside it. No file-size allowance is raised.

The generated TypeScript merge keeps the incoming node head's exact generated
file as a temporary snapshot. It is intentionally not hand-merged: the complete
new union must be regenerated remotely and verified before a final gate/handoff.
Its generator's inherited trailing spaces are not manually edited. New merged
source strict/tests, the canonical writer union security check, complete wire
freshness and whole package acceptance remain owed. The successful exact
0c812 runtime strict and queued 0c812 tests remain prior-source receipts, not
this merge's execution proof. No row 618 persistence code is included.


V3 read the exact gate-50 settings join at `2839abccc` without a new security
blocker; this is source context only, with current execution and full audit
still owed. The existing pre-lock cancellation fixture now names its unchanged
30-second safety budget beside that fixture. It does not reuse the distinct
canonical real-writer fixture's ten-second budget or change any assertion.
The skills/remote sealed diagnostic causes do not prove privacy of the inherited
common SettingsError::Lost source edge; that owner's later task-error/private
settings join remains separate.


The gate-50 strict run `wp-mcp-runtime-narrow-20261115T162546Z-2839abcc`
returned two concrete merge issues: the same byte-identical `lsp_config` helper
had been inserted twice automatically, and the inherited writer fixture's fully
qualified CancellationToken became unnecessary after its MCP lifetime adapter
import. One identical helper is retained and the fixture uses the in-scope name.
No check, assertion, timeout, effect ordering or lint rule changes. The queued
Settings test on the known compile-red union was superseded only after terminal
cancellation; the failed strict log is retained. Exact corrected seven-crate
strict and Settings execution remain owed.


The new permit-release assertion joins the actual outer operation tracker before
reading capacity, separately from the canonical writer shutdown. A cancelled
caller may have a ready writer result before its detached operation is polled
to completion; an unguarded capacity snapshot would race that legitimate owner.
The existing whole-fixture deadline covers both joins. No sleep, assertion
relaxation or timeout increase is introduced.


## MCP result shape follows negotiated protocol

Inspection found that the latest SDK structured constructor emits arbitrary JSON
even when the real fixture negotiates the object-only 2025-06-18 revision. The
fixture had wrongly asserted every root shape was legal. One private response
adapter now uses the actual RequestContext protocol: old text remains unchanged,
2025 object-shaped success retains matching structured content, older or
non-object legacy success omits the optional field, and 2026 retains arbitrary
matching JSON. The SDK retains ownership of legacy resultType projection.

The actual pipe helper asserts negotiated version and deserializes its
structured slot as an object; every old array/text/data assertion remains. A
version/JSON-shape matrix includes absent metadata, every supported revision,
arrays, objects, strings, numbers, booleans and explicit null. This changes no
node request, policy, selector, effect or grant. ADR0065 records the official
versioned references and compatibility reason. Exact strict and actual MCP
execution remain owed; existing pre-change greens are not expanded to this fix.


The announced gate52 head `1c762ccf` is merged after the seven-crate
`6e558f138` strict passed (rc0, 60 seconds). Its protocol25 assignment belongs
only to workspace metadata; all MCP/browser feature constants remain None.
The new merge adds that canonical assignment and the already-gated web feedback
package without changing this MCP response adapter. Its own strict and complete
MCP suite, including the stricter actual legacy pipe decoder, are the next
changed-source checks. Current app trees still take only their announced app
base; this node merge is not an app adoption.


The first compatibility strict run compiled the product adapter but failed in
the shared test decoder because vessel-mcp has no direct serde derive dependency.
The decoder now uses the existing serde_json typed Map deserializer for the
actual structured slot, rejecting arrays/scalars/null without a new dependency.
Negotiated-version, unchanged text and structured equality/absence assertions
remain; no validation is relaxed. The known compile-red queued MCP suite was
superseded only after terminal cancellation, and its failed strict log is kept.
Corrected strict and the full actual MCP suite remain owed.


## Gate-53 lifecycle join

The announced green `93970acb` is merged without conflicts. Its typed listing
request replaces the old unit type. Both the ordinary list adapter and the
permission aggregate now use `ListSessions::default()`, preserving the legacy
null/full-list behavior rather than silently selecting a new visibility filter.
Explicit listing/label metadata still uses the shared unassigned-protocol
refusal; no SetLabels call or schema is invented here. Listing schema 21 is
the lifecycle owner's assignment; MCP_PARITY remains None.

The response compatibility strict run on `31c8d7e3` failed only on two missing
semicolons in the new matrix's unit-valued block arms. Those are corrected
without changing the matrix, product adapter or assertions. The actual MCP
suite on that pre-merge source is retained as meaningful compatibility
execution; its result cannot attest this new lifecycle union. New merged-head
strict is required. Failed logs remain retained.

The earlier gate-50 settings correction has actual execution: seven-crate
strict `wp-mcp-runtime-narrow-20261115T165218Z-6e558f13` passed, and the matching
Settings run passed all four tests, including the canonical held writer and
outer operation tracker join. These scoped results do not close the full
package, current lifecycle union, crash or cross-node obligations.


A diff-only bound audit found the new actual notification fixture's stage
safety net still used an inline duration. `NOTICE_STEP_BUDGET` now owns the
same 15-second value beside that fixture. No stage, assertion or product
behavior changes. The already queued prior-source strict and MCP suite are
retained with their exact source scope; the next changed-source check will
include this naming correction rather than submitting another duplicate suite.


## Actual process-interruption fixture for skill transactions

[Decision Log]
- Purpose: the prior drop-and-reopen fixture does not prove recovery when a
  process is killed without destructors. Exercise that missing boundary with
  temporary roots and the real public SkillManager save/status implementation.
- Existing assets: save already accepts a typed persistence callback; the
  fleet atomic private file writer supplies a synthetic preference owner.
- Alternatives: adding product crash hooks, another binary or dependency would
  enlarge production behavior just for tests. Reuse the test executable as an
  isolated child and synchronize at that existing callback instead.
- Choice: four child kills cross update/delete with before/after preference
  persistence, always before the journal commit acknowledgement. A pipe marker
  proves entry into the exact callback; child stdin remains held until kill.
  The parent owns kill/reap on normal and failed paths, with one named readiness
  bound. No sleeps, real homes or services are used.
- Checks: restart reads the actual synthetic durable file, reopens the manager,
  checks exact restored/committed/obsolete/user/unrelated files, rollback mode
  preservation, journal/staging cleanup and repeated status idempotence.
- Limits: this is the fleet filesystem recovery mechanism, not the node's
  actual Store callback, a power-failure/fsync proof, every journal write edge
  or row618 durable queue/receipt recovery. It activates no feature or RPC.

The tests are authored only; changed fleet strict and actual child execution
remain owed. This slice also includes the unchanged notification fixture budget
name. Earlier source/check receipts are retained with their exact limits.
