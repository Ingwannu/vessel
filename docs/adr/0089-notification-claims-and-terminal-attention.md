# ADR 0089: One connected notification recipient and explicit terminal attention

Status: implemented/staged, pending checks and assignment; not package acceptance.

## Purpose and intent

Complete the connected-client part of ADR 0041's notification policy: exactly
one recently active eligible connection may display an attention notification,
while a visible client focused on that target suppresses delivery. Add explicit
terminal activity and attention without guessing from arbitrary PTY output.
The durable session inbox remains ADR 0040's authority for requests and reads.

## Existing implementation and constraints

The node's push policy already computes Suppressed, Local(recipient) or Push
from connection presence, using the reference's 180-second presence threshold.
Only the push worker consumes that decision. No wire response tells a desktop
that it was selected, so several connected clients can independently notify.
The existing presence call targets a session, elsewhere or background; it has
no terminal focus target. TerminalInfo reports process state but has no
explicit activity/attention revision. A plain shell's output cannot establish
that the person needs to answer anything.

Paseo reference (0.15.4): server `agent-attention-policy.ts:3-79`,
`websocket-server.ts:2838-3010`, and
`packages/server/src/terminal/activity/terminal-activity-tracker.ts:12-80`; client
`hooks/client-activity-tracker.ts:1-117`,
`notification-open/model.ts:16-84`. Read source before implementing the
corresponding service; this decision does not assert runtime parity yet.

## Alternatives considered

1. Keep per-window local watermark delivery: retains duplicate desktop alerts.
2. Add unsolicited delivery frames to every stream: requires another ordered
   fan-out and reconnect contract, duplicating the durable inbox projection.
3. A typed exact-source claim by the current eligible recipient, reusing node
   presence and store rechecks; explicit terminal attention revisions.
4. Infer terminal prompts or work from output text: incorrect for plain shells,
   arbitrary programs, localization and malicious escape sequences.

## Choice

Choose exact-source claims. A claim carries only an existing attention identity
and the reporting node's cursor/revision. The node checks current unread/live
attention, current effective Read authority, the connection's current presence,
the recipient decision and prior admission before returning permission to
notify. A reply has explicit outcomes for admitted, not recipient, focused
suppression, withdrawn/read, and already admitted. Unknown future outcomes
never authorize delivery. Claims do not resolve or mark attention read.

All identities are typed and scoped to the authenticated reporting connection;
a caller cannot nominate a different recipient. An uncertain claim is never
retried into a second delivery. Session admission uses one persistent table keyed by inbox cursor with an
ON DELETE CASCADE foreign key to the inbox row. Its population is bounded by
the existing inbox retention and live request limits. Atomic insert rechecks
current unread/eligible source and a paired device's current Read grant;
owner-origin claims have their existing owner authority. A post-insert presence
or disconnect recheck may suppress best-effort delivery but never reopen that
source for duplicate admission. Losing a notification opportunity leaves the
inbox unchanged. Terminal admission is runtime-only, keyed by terminal revision;
terminals themselves do not survive node restart.

A known plain shell has explicit Untracked activity: neutral only in workspace
aggregation, with no terminal completion claim. Unknown is reserved for missing,
lost or future producer evidence. Known activity is Idle or Working; attention
is Finished or NeedsInput with an exact
non-reused revision. Only structured lifecycle/hook input owned by that
terminal may change activity. Working to idle yields finished; an explicit
needs-input signal yields needs-input; visible focus clears only that exact
revision. Raw PTY output is never a semantic input. Session and terminal
notification targets stay separate typed variants; no fabricated session id
stands in for an unattached terminal.

The terminal integration producer and the existing terminal actor are joined
through their typed boundary; a UI must not invent activity for a terminal
whose producer cannot report it yet. The native terminal owner receives the
contract before its controls are wired.

## Privacy, authority and older peers

ADR 0041's privacy floor remains: OS/push text is generic. Session titles,
paths, prompts, commands, questions and source identifiers never enter the OS
payload. The desktop keeps a bounded process-local mapping from an unrelated
random notification tag to the source. A stale tag after restart/eviction opens
the inbox without choosing another source. The existing UUID implementation
supplies the unrelated tag at the app boundary, not in the pure view model.

This extends ADR 0041's stated per-window recipient limitation. Focus handling
follows the reference's focused-target suppression rather than inventing
activity merely because a socket or window remains open. Web Push remains the
accepted account-free mobile delivery difference; no Expo/APNs/FCM credentials
or platform service are presumed to exist.

Agent and forwarded/delegated origins cannot claim a person's OS delivery.
Read-only devices can read/mark and receive notifications but cannot answer
permissions/questions. Revoke and disconnect invalidate connection presence
and pending admission. A typed node error contains no raw source content.

New calls/fields use one unassigned Option<ProtocolVersion> feature constant
and the shared required_protocol accessor. No provisional numeric version is
chosen here. Fields are hidden in every direct, list, nested and forwarded
older-peer reply; old clients receive no unsolicited new frame. Both directions
of unassigned/older-peer refusal are tested before Quality B registration.
No schema number is chosen by this document; the node integrator assigns it
once admission persistence is specified.

## Benefits, costs and effects

The store remains the lifecycle authority and one node owns recipient choice.
Clients cannot turn unread-count changes into repeated notifications. Explicit
terminal revisions preserve first-clear behavior and honest unknown state.
Costs are a bounded claim operation, admission retention, and platform-specific
terminal producers; those gaps remain visible until exercised against fakes.
No new network listener, shell helper or provider credential is introduced by
the claim boundary. Pure state/routing is cross-platform; platform delivery
and terminal producer support must be reported separately on Windows/macOS.

## Verification and resume

Required tests: two concurrent clients and equal-activity tie; focused target
suppression; absent/stale presence; read/resolve/revoke/disconnect before claim;
duplicate/uncertain claim; restart and bounded retention; session/terminal
identity isolation; explicit terminal transitions and stale revision clears;
all unassigned and older-peer paths; no source contents in OS payloads.

Five Quality B rows are registered as pending under the one unassigned feature
constant: notification.claim, terminal.attention_clear, terminal.activity,
inbox.attention and inbox.read. Store admission, semantic reducer, optional-field
older projections and capability routing are staged. The normal-client report
carries the exact observed revision; stale reports cannot complete a newer run.
No protocol assignment, whole package gate, completed hook/native consumer or
V3/V4 acceptance is inferred from registration. The plan records actual narrow
failures, corrections and remaining assigned-wire/platform obligations.

## Activity-only local producer capability

Paseo CLI hooks.ts uses per-terminal activity attribution and silently ignores
missing setup or failed reports. Vessel uses the existing local IPC endpoint,
not another HTTP listener. A TerminalActivityAuth frame authenticates a random
256-bit per-terminal secret; only its constant-time-verified hash is retained.
The capability is injected into a shell only after the protocol is assigned,
with an exact terminal id and socket/pipe path. Its credential dies at terminal
end, explicit removal or node shutdown. Inherited hook target/secret/socket are
scrubbed before another terminal receives its own capability. CommandSpec Debug
already hides environment values; the new auth/secret Debug is redacted too.

[Decision Log]
- Purpose: a CLI/provider hook may report its own terminal without acquiring
  owner, agent, session, inbox, filesystem or push authority.
- Existing constraints: normal terminal.activity requires Operate and the exact
  observed revision, but a hook must work without a general node credential.
- Alternatives: fabricate an owner/agent principal; add a public HTTP hook port;
  add a terminal role to every principal match and hope other methods refuse it.
- Choice: a separate ActivityProducer Handler generated from the same one method
  table. It holds only TerminalHandle, accepts only ReportTerminalActivity for
  that id, and rejects every other current or future method by construction.
- Why: no general principal is fabricated and no second method list or listener
  is introduced. Local capability authentication precedes the usual handshake.
- Impact: normal clients still require a revision; this activity-only handler may
  advance its own current revision when absent. Explicit supplied stale revisions
  still conflict. This never writes session turn state. CLI maps provider events
  to the semantic signal and retains its bounded, silent failure behavior.

The producer contract, environment injection, closure/denial tests and actual
CLI consumer must pass before the feature's pending protocol is assigned. A
wire test with the final assigned protocol remains required at integration.

### The CLI consumer: agent hooks (2026-11-20)

The reference is Paseo 0.15.4 as installed (`@getpaseo/cli` and its `@getpaseo/server`, paths under each
package's `dist/`):

- **`vessel hooks <agent> <event>`**, after `cli/dist/commands/hooks.js`.
  - Paseo reads `PASEO_TERMINAL_ID`, `PASEO_ACTIVITY_TOKEN` and `PASEO_TERMINAL_ACTIVITY_URL` (lines 31-33)
    and exits silently when any is missing.
  - It reads stdin with a 100 ms idle timeout (line 42) and posts with a 500 ms abort (line 69). Every
    failure is silent.
  - Vessel reads `VESSEL_TERMINAL_ACTIVITY_ID`, `_SECRET` and `_SOCKET` and connects with the terminal's
    capability (`LocalCredential::TerminalActivity`), sending `terminal.activity` with no revision. It
    keeps the same timeouts and the same silence.
- **Event maps**, copied exactly from `server/dist/server/terminal/agent-hooks/`:
  - `claude/claude.js:2-7,39-42`: UserPromptSubmit → Working; Stop, StopFailure, SessionEnd → Idle;
    Notification → NeedsInput only when the stdin JSON's `notification_type` is `idle_prompt`.
  - `codex/codex.js:2-8`: UserPromptSubmit, PreToolUse, PostToolUse → Working; PermissionRequest →
    NeedsInput; Stop → Idle.
  - `opencode/opencode.js:2-8`: `session.status.busy` and `.retry` → Working; `.idle` → Idle;
    `permission.asked` → NeedsInput; `permission.replied` → Working.
  - An unknown agent or event reports nothing.
- **Installer**, after `agent-hook-installer.js`, `claude/claude-settings.js`, `codex/codex-settings.js` and
  `opencode/opencode-plugin.js`.
  - Claude: `$CLAUDE_CONFIG_DIR` or `~/.claude`, file `settings.json`. Each event gets one entry, which
    `claude-settings.js:20-33` writes as `{matcher: "", hooks: [{type: "command", command, timeout: 10}]}`.
  - Codex: `$CODEX_HOME` or `~/.codex`, file `hooks.json`, with the same entry plus `commandWindows`.
  - OpenCode: `$OPENCODE_CONFIG_DIR` or `$XDG_CONFIG_HOME/opencode` (`~/.config/opencode`), file
    `plugins/<name>.js`, the plugin source that runs `<cli> hooks opencode <event>`.
  - The command (`agent-hook-installer.js:43-50`) is
    `if [ -n "$VESSEL_TERMINAL_ACTIVITY_ID" ]; then "${VESSEL_HOOK_CLI:-vessel}" hooks <agent> <event>; fi`,
    with a cmd.exe form. Paseo's marker is `hooks <agent>`.
- **Setting.** "Terminal agent hooks" is off by default, as is `enableTerminalAgentHooks`
  (`server/dist/server/server/config.js:429`). Following `terminal-agent-hook-setting.js`, the node
  installs the hooks when the setting is on at start and when it is turned on, and removes them when it
  is turned off. The settings field is pending under this feature's constant, in the same packet, so one
  protocol number carries the whole feature.

Vessel's rules beyond Paseo (coordinator, 2026-11-20):

- The installer writes a person's real agent configs. Before its first write to an existing file it keeps
  a one-time backup beside it (`<file>.vessel-backup`).
- It changes only the entries that carry its marker. An invalid file is left untouched, with one logged
  line; it is never rewritten. Paseo parses an invalid object as `{}` and would rewrite it.
- Turning the setting off removes exactly the marked entries.
- No test, capture or gate writes a real `~/.claude*`, `~/.codex` or OpenCode config. Tests use a scratch
  home and config directories, and a guard refuses to run when any of them resolves to the real home.

As built (`vessel-node/src/agent_hooks.rs` and its modules, `vessel-cli/src/hooks.rs`):

- **Where.** Each agent's home is taken as the node starts it (`HarnessEnv::home`): `$CLAUDE_CONFIG_DIR`
  or `~/.claude`, `$CODEX_HOME` or `~/.codex`, and `~/.config/opencode`. Paseo also honours
  `OPENCODE_CONFIG_DIR` and `XDG_CONFIG_HOME` for OpenCode; Vessel sets neither for the OpenCode it runs.
- **Marker.** The marker is Vessel's own: `VESSEL_HOOK_CLI` together with `hooks <agent> `. Paseo's
  marker, `hooks claude`, would also match Paseo's own entries in the same file.
- **What changes in a file.** The file is read keeping its keys in order and each value's text as written.
  Only `hooks` and the lists of Vessel's own events are rebuilt, and only marked commands leave them; an
  entry shared with a person's command keeps that command. Paseo rewrites the whole file, and a file that
  is not an object becomes `{}`.
- **What is left alone.** A file that is not JSON, or whose `hooks` or an event list has another shape, is
  left untouched with one warning. Turning the setting off never creates a file, and removes OpenCode's
  plugin only when it carries the marker.
- **Writes.** Writes are owner-only and atomic (`vessel_fleet::write_private`), after the one-time
  `<file>.vessel-backup`. In test builds every write path refuses the real home's agent configuration.
- **The setting.** `terminal_agent_hooks` is in `NodeSettings` and `SetNodeSettings`. It is hidden below,
  and refused before, `ATTENTION_NOTIFICATIONS_PROTOCOL`. It is applied when changed, and at start when on,
  on the settings writers' tracker.
- **The OpenCode plugin** runs `$VESSEL_HOOK_CLI`, or `vessel` when unset, as the other agents' commands
  do. Paseo's plugin runs `paseo`.

A hook without an explicit revision is scoped to its terminal and follows the
reference producer's event ordering. It does not attest which provider command
generation produced an event. Revision-supplied reports and all attention-clear
callbacks retain exact revision checks; no stronger causal ordering is claimed.

## Desktop idle evidence and lease renewal

An optional push.presence idle_age is a nonnegative elapsed millisecond newtype,
not an absolute client timestamp. It uses the same pending feature constant as
terminal focus; older peers never receive it. The server computes its own now
minus the elapsed age, and never moves activity behind newer actual input.
Native samples retain their monotonic observation time so queueing and repeated
heartbeats age the evidence instead of keeping an idle window present forever.
Unknown platform evidence is absent; ordinary in-app input remains independent.

Paired heartbeat renews only its caller's existing active Web Push lease, after
half of the 48h lifetime or exactly at that boundary (Paseo 0.15.4
`packages/server/src/server/push/token-store.ts:34-42`). The committed update rechecks current Read/revocation
and never resurrects an expired or missing lease. Node-owner and agent/delegated
origins cannot borrow another device's subscription identity. A renewed expiry
also prevents a late old 410 from deleting that newer lease.

The renewal runs beside the presence reply, never in it (node review of TRACKER
815, R1). `push.presence` is live since protocol 22 and answered from memory, so
a report neither waits for the store's writer nor fails when a write fails
(closed while the node stops, a disk error); the failure is logged. A device
tries at most once an hour, and not before a new subscription's half-life
(`vessel-node/src/push/renewal.rs`). Paseo renews at the first heartbeat past
the half-life; here it can be up to an hour later, which still leaves the lease
a day. The renewal is the node's own and not on the wire, so it is not gated by
the feature constant.

## Terminal Web Push joins the one worker

Terminal metadata wakes the existing coalescing push worker. Its independent
revision watermark does not consume an Inbox cursor; one generic encrypted
notification per exact lease/burst may represent either canonical source enum
variant. No fabricated SessionId, new payload, principal, listener or transport
is introduced. The observer retains the canonical registry owner and metadata revisions, never
an individual terminal handle or a second PTY/follow task. Pruning releases the
registry entry; closed change watches and node cancellation end the worker. This path stays off
while the feature constant is unassigned. One terminal's pushes are spaced apart, a minute after
a push and up to an hour for a terminal that keeps asking (item 1 of the owed list below).

One factored lease predicate checks exact device, serialized subscription and
expiry, current Read, revocation and expiry time. Inbox admission keeps that
predicate and source eligibility in ONE SQL snapshot. Terminal admission awaits
the same lease predicate, then synchronously rechecks exact runtime revision and
presence immediately before sending; no preparatory await follows that check.
Gone responses use the existing exact attempted-lease expiry operation.

Tests are drafted for lease-only admission, independent/non-regressing terminal
watermarks, unknown/cleared/failed/foreign sources and metadata watcher teardown.
The real-node/fake-terminal encrypted HTTP fixture tests unassigned refusal now
and activates its positive branch after integrator assignment. A pending-branch
pass is not positive terminal delivery evidence; the assigned integration run
and execution of the drafted preparation-time terminal cancellation cases
remain required.

## Explicit untracked versus future activity

The pending activity DTO separates Untracked (a known plain shell without a
semantic producer) from Unknown (missing/lost producer or serde(other) future
state). Reusing Unknown as a neutral projection would silently classify a new
wire state as uninstrumented. TerminalInfo's value constructor initializes only
known Shell kind as Untracked; managed/future kinds remain Unknown. A semantic
Clear/disconnect still yields Unknown and cannot manufacture a neutral fact.
Both mean no terminal dot, but only explicit Untracked is neutral in workspace
aggregation. Old peers still receive no attention field under the same pending
feature; no new method, number, scope or migration is added. The native adapter
waits for reviewed canonical values rather than duplicating this wire contract.

## Local error attention is independent of Web Push eligibility

Paseo websocket-server.ts:2899-2904 passes every attention reason to the local
recipient election, while agent-attention-policy.ts:77-79 excludes error ONLY
from push eligibility. Local admission therefore accepts an exact unread failed
Inbox source with the same device/source/read/once-only checks. Web Push
candidates and final push admission still exclude failed outcomes. This server
recipient/admission rule is not the client OS-delivery rule: Paseo
`packages/app/src/contexts/session-context.tsx:267-269` returns immediately for
error. The native opportunity and final projection gates therefore exclude
Failed while retaining the source in Inbox and failed badges. Generic OS text
and focus suppression do not change. A real-store regression checks the server
local once-only admission, shared read suppression and actual Web Push query;
separate native tests check error suppression. No new wire or scope is added.


## Local final-check snapshot consistency

The pooled-reader local notification recheck must observe current person
authority and the exact source in one read transaction. Two independent
auto-commit SELECTs could accept old device Read authority while the source is
Seen, then accept Unread source state after revocation; those facts need not
have been eligible together. The same borrowed SQLite connection now encloses
the existing canonical admissible function in a read transaction and commits
the read before returning. No async wait spans that transaction, no policy is
duplicated, and no new authority is granted. A later revoke cannot recall an
already admitted notification; this is one consistent snapshot, not transport
linearizability or retroactive recall. Actual two-order regressions and changed
source checks remain owed before claiming the correction is verified.


## Passive read admission and current-parent consumption

Passive Read ingress acquires the existing transferable marking lease in the
caller before retaining a node task. Abandoning a mutex wait therefore creates
no retained work. Once admitted, the same owned guard travels through the
existing live/durable pending veto, actual Store commit and live publication,
so caller loss cannot split those last two states. TaskTracker accounts for
accepted work; it is not a queue bound. No second lock or lane is introduced,
and inherited general mark_live behavior is outside this correction.

Notification eligibility uses the canonical configured
vessel_has_current_parent scalar inside the shared SESSION SQL predicate. It
uses current global parentage, distinct from immutable Creator audit and from
workspace-relative display roots. Valid detach can restore eligibility without
rewriting history; attached or malformed parentage cannot acquire it. Candidate
filtering stays before LIMIT, and source/lease joins retain their SQL snapshot.
Fresh final consumer reads follow asynchronous preparation; an old held WAL
snapshot is not a freshness guarantee across revocation. The feature remains
unassigned, and direct Store/lease tests do not imply assigned wire or delivery
proof. Changed-source checks and the actual drafted regressions are pending.

## Owed before the feature constant is assigned

The node review of TRACKER 815 (2026-11-20, the staging line merged onto the
node line with the constant still None) accepted the source and listed what must
be done before a number is assigned. The packet after it (branch
backend/m12-impl-usage-815-owed) answers each:

1. Push flood. An agent may report its own terminal's activity, and a hook can
   loop Working/Idle; each Idle made a new Finished revision pushed once per
   250 ms burst. Done: a push opens a per-terminal gap of a minute, doubled up
   to an hour by a push at its end and reset after twice the gap of quiet; the
   worker records the terminals a burst pushed once per burst
   (`vessel-node/src/push/terminal/spacing.rs`). Paseo has no such limit; this
   is a deliberate difference.
2. Workspace script rows. Done: `WorkspaceScriptRow::for_protocol` passes its
   terminal through `TerminalInfo::for_protocol`, and `ServicePage` and
   `WorkspaceScriptOutcome` (Accepted, Already) use it.
3. The environment scrub. Done: the terminal activity variables are removed
   like credentials but do not count as a session's process tree, so a node
   started from a Vessel terminal keeps the user's model route.
4. `create_terminal`. Done: `register` takes the capability's secret and the
   entry holds its producer from the start; `install_producer` is gone.
5. Tests weaker than their names. Done:
   - while unassigned, `terminal_delivery` drives the node's own path (the
     terminal's handle goes Working then Finished) and sees no Web Push;
   - a capability admits only its terminal with its own secret and closes on
     forget and on end (`admit_assigned` holds `admit_producer`'s checks
     behind the gate; a test-only `give_producer` stands in for the
     capability the node gives once assigned);
   - a renewal for a device revoked with its subscription row still there is
     refused by the renewal's own check;
   - a current cursor paired with another session's key claims nothing;
   - `inbox.attention` reads two sessions from one snapshot.
   Still owed at assignment: the IPC route (`ipc::admit` to `admit_producer`
   to a report) with the assigned protocol, as the wire test the
   "Activity-only local producer capability" section already requires.
6. The terminal list wake. Done: T2's `Shown` holds the attention once the
   feature is assigned (left out while unassigned, when every peer has it cut).

At assignment (the node review of the owed packet, 2026-11-20, PASS):
- the worker's per-burst pushed set gets its integration test beside the IPC
  route's;
- the spacing reads the wall clock, so a clock step moves a gap; a monotonic
  `Instant` would ignore steps;
- `pushed` takes the time after the burst's deliveries, which only lengthens a
  gap by their duration;
- a burst still gives each device one source, as before.

Recorded, not owed (they fail closed or are allowed by the design):
- one malformed parentage row fails a whole `pending_push` page through
  `vessel_has_current_parent`;
- a data directory made by a staging-line build (claims at schema 22) fails to
  migrate with "table already exists";
- the push worker marks terminal revisions seen before its store reads, so a
  store error loses those pushes silently ("losing an opportunity");
- `CredentialHash` serves device, agent and terminal secrets without domain
  separation.
