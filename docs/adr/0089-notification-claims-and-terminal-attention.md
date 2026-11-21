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

The reference is Paseo 0.15.4's sources (~/refs/paseo-latest/packages; `server/src/terminal/agent-hooks/`
unless a path says otherwise):

- **`vessel hooks <agent> <event>`**, after `cli/src/commands/hooks.ts`.
  - Paseo reads `PASEO_TERMINAL_ID`, `PASEO_ACTIVITY_TOKEN` and `PASEO_TERMINAL_ACTIVITY_URL` (lines 55-57)
    and exits silently when any is missing.
  - It reads stdin with a 100 ms idle timeout (lines 63-76, the timeout at 68) and posts with a 500 ms
    abort (line 98). Every failure is silent.
  - Vessel reads `VESSEL_TERMINAL_ACTIVITY_ID`, `_SECRET` and `_SOCKET` and connects with the terminal's
    capability (`LocalCredential::TerminalActivity`), sending `terminal.activity` with no revision. It
    keeps the same timeouts and the same silence.
- **Event maps**, copied exactly:
  - `claude/claude.ts:4-9,28-48`: UserPromptSubmit → Working; Stop, StopFailure, SessionEnd → Idle;
    Notification → NeedsInput only when the stdin JSON's `notification_type` is `idle_prompt`.
  - `codex/codex.ts:4-10`: UserPromptSubmit, PreToolUse, PostToolUse → Working; PermissionRequest →
    NeedsInput; Stop → Idle.
  - `opencode/opencode.ts:4-10`: `session.status.busy` and `.retry` → Working; `.idle` → Idle;
    `permission.asked` → NeedsInput; `permission.replied` → Working.
  - An unknown agent or event reports nothing.
- **Installer**, after `agent-hook-installer.ts`, `claude/claude-settings.ts`, `codex/codex-settings.ts`
  and `opencode/opencode-plugin.ts`.
  - Claude: `$CLAUDE_CONFIG_DIR` or `~/.claude`, file `settings.json`. Each event gets one entry, which
    `claude-settings.ts:33-53` writes as `{matcher: "", hooks: [{type: "command", command, timeout: 10}]}`.
  - Codex: `$CODEX_HOME` or `~/.codex`, file `hooks.json`, with the same entry plus `commandWindows`.
  - OpenCode: `$OPENCODE_CONFIG_DIR` or `$XDG_CONFIG_HOME/opencode` (`~/.config/opencode`), file
    `plugins/<name>.js`, the plugin source that runs `<cli> hooks opencode <event>`.
  - The command (`agent-hook-installer.ts:136-150`) is
    `if [ -n "$VESSEL_TERMINAL_ACTIVITY_ID" ]; then "${VESSEL_HOOK_CLI:-vessel}" hooks "<agent>" "<event>" 2>/dev/null || true; fi`,
    with a cmd.exe form. Paseo's marker is `hooks <agent>`, unquoted (`claude/claude.ts:25`,
    `codex/codex.ts:26`). Paseo stops with an error on a file that is not JSON
    (`claude-settings.ts:23-24`) and makes a non-object `{}`.
- **Setting.** "Terminal agent hooks" is off by default, as is `enableTerminalAgentHooks`
  (`server/src/server/config.ts:681`). Following `terminal-agent-hook-setting.ts`, the node
  installs the hooks when the setting is on at start and when it is turned on, and removes them when it
  is turned off. The settings field is pending under this feature's constant, in the same packet, so one
  protocol number carries the whole feature.

Vessel's rules beyond Paseo (coordinator, 2026-11-20):

- The installer writes a person's real agent configs. Before its first write to an existing file it keeps
  a one-time backup beside it (`<file>.vessel-backup`).
- It changes only the entries that carry its marker. An invalid file is left untouched, with one logged
  line; it is never rewritten. Paseo parses an invalid object as `{}` and would rewrite it.
- Turning the setting off removes exactly the marked entries.
- The operator's machines run Paseo and Vessel side by side, and Paseo's entries sit in the same files.
  Vessel's install, reinstall and off leave every Paseo entry byte-identical and in place. Vessel's marker
  is its exact command form, not a substring. A test proves this on Paseo 0.15.4's own installer output,
  not on a copy of a real file.
- No test, capture or gate writes a real `~/.claude*`, `~/.codex` or OpenCode config. Tests use a scratch
  home and config directories, and a guard refuses to run when any of them resolves to the real home.

As built (`vessel-node/src/agent_hooks.rs` and its modules, `vessel-cli/src/hooks.rs`):

- **Where.** Each agent's home is taken as the node starts it (`HarnessEnv::home`): `$CLAUDE_CONFIG_DIR`
  or `~/.claude`, `$CODEX_HOME` or `~/.codex`, and `~/.config/opencode`. Paseo also honours
  `OPENCODE_CONFIG_DIR` and `XDG_CONFIG_HOME` for OpenCode; Vessel sets neither for the OpenCode it runs.
- **Marker.** An entry is Vessel's when a command is exactly one Vessel writes for one of the agent's
  events, shell or cmd.exe form. A person's command that only resembles it is theirs. If a form changes,
  the old one must stay recognised, or off would leave old entries behind.
- **Beside Paseo.** The commands quote the agent and the event. Paseo's install and uninstall drop every
  command containing `hooks claude` or `hooks codex` (`claude/claude-settings.ts:98-113`,
  `codex/codex-settings.ts:105-139`). Unquoted, Vessel's commands contain it, and each Paseo restart
  would take Vessel's hooks out. `crates/vessel-node/tests/fixtures/agent_hooks/generate.mjs` checks this.
  It runs Paseo 0.15.4's installer from `@getpaseo/server`, under `env -i`, with `configDir` set to a
  scratch folder:
  - it installs Paseo over a person's file;
  - it adds Vessel's entries;
  - it installs and uninstalls Paseo again.

  Vessel's entries survive both steps. The unquoted form fails at Paseo's install. The fixtures it writes
  are what the test
  `paseo_entries_stay_byte_identical_and_in_place_through_install_reinstall_and_off` reads.
- **What changes in a file.** The file is read keeping its keys in order and each value's text as written.
  On writing, every value Vessel did not change is put back as its text. Only `hooks` and the lists of
  Vessel's own events that changed are laid out again, two-space as Paseo and the agents write. So in a
  file Paseo wrote:
  - every other entry keeps its bytes and its place;
  - Vessel's entry goes last in each of its events;
  - off after install gives the file back byte for byte.

  While on, an entry already exactly Vessel's stays where it is. Paseo reinstalling its own entries after
  Vessel's therefore moves nothing back and forth. An entry shared with a person's command keeps that
  command. Paseo rewrites the whole file, and a file that is not an object becomes `{}`.
- **What is left alone.** These files are left untouched, with one warning:
  - a file that is not JSON;
  - a file with `hooks` or one of Vessel's events twice;
  - a file whose `hooks` or an event list has another shape.

  Turning the setting off never creates a file. Install and off touch OpenCode's plugin only when its text
  is exactly Vessel's.
- **Writes.** Writes are owner-only and atomic (`vessel_fleet::write_private`), after the one-time
  `<file>.vessel-backup`.
  - A link is followed, as Paseo's `writePrivateFileAtomicSync` does (`server/src/server/private-files.ts:27-55`):
    the target is written, so a `settings.json` linked into a person's dotfiles stays a link. The backup
    holds the target's bytes, beside the link (node review R2).
  - The backup is created with `create_new`, so no second writer can overwrite it (R3).
  - Removing OpenCode's plugin keeps no copy: only a file exactly Vessel's is removed, and a copy would be
    a stray file in OpenCode's `plugins` folder.
  - In test builds every write path refuses the real home's agent configuration.
- **The setting.** `terminal_agent_hooks` is in `NodeSettings` and `SetNodeSettings`. It is hidden below,
  and refused before, `ATTENTION_NOTIFICATIONS_PROTOCOL`. It is applied when changed, and at start when on
  once the constant is assigned (never while it is unassigned: nothing could report, node review note 7).
  One worker applies it, always the latest state wanted, on the settings writers' tracker
  (`agent_hooks/applier.rs`). A quick on-then-off cannot interleave two read-modify-writes (R3).
- **Beside Paseo, R1.** No command Vessel writes contains `hooks claude` or `hooks codex`, tested with
  Paseo's own `includes()` (`paseos_markers_match_no_command_vessel_writes`).
- **A hook never fails the agent (R4).** In every terminal with a capability the node sets
  `VESSEL_HOOK_CLI` to the `vessel` installed beside `vesseld` (`terminals::hook_cli`). Paseo sets
  `PASEO_HOOK_CLI` to its CLI's absolute path the same way (`terminal/terminal.ts:460-467,481-492`).
  The command also ends `2>/dev/null || true` (cmd.exe: `2>NUL & exit /b 0`). A missing `vessel`
  (127), or an older one without `hooks` (clap's 2, which Claude Code takes as a blocking hook result),
  still returns 0 silently. Paseo's command has neither: its CLI ships inside its app. Paseo also puts
  its CLI's folder first on the terminal's `PATH`; Vessel leaves the person's `PATH` as it is.
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

## The clients' session claims (2026-11-21, the claims follow-up's second row)

Both clients claim a session notice before showing it, behind this feature's constant:
- the app: `view/src/notices/claims.rs`, `link/src/claims.rs`, `app/src/areas/notify/claims.rs`;
- the page: `web/src/data/claims.ts`, wired in `features/notifications/use-notices.ts`.

While the constant is unassigned, no machine admits claims, and a notice takes ADR 0114's path unchanged.

- **Source.** A claim names the exact inbox entry by key and cursor. A client learns them only from
  `inbox.list`, which pages newest first, sixteen a page, with no session filter.
  - Once per machine per change, a client reads that machine's first page.
  - For each notice it claims the newest unread entry of that session whose item matches the notice's
    kind: a request for "needs you"; Finished or Limit for "finished".
- **Limit.** An entry not on the first page is not claimed. This happens when sixteen newer entries came
  first. The notice then falls back to ADR 0114's local path and shows (the coordinator's rule,
  2026-11-20). Two windows may both show it in that case. The alternative was a session-scoped claim form
  before assignment, which is a wire change on the node line.
- **What shows.**
  - `Admitted` and the fallback show.
  - A first page that cannot be read takes the fallback for every notice it would have asked about
    (2026-11-21, at assignment): it names no source, as an entry past the page does, and a transient
    failure must not silence them.
  - Every other outcome, a failed claim, and a lost reply show nothing. The inbox still holds the
    entry.
  - A claim is never retried.
  - Failed sessions never reach a claim: the diff drops them (the "Local error attention" section).

## The page's terminal notices (2026-11-21, the claims follow-up's third row, page part)

The page does what the app does (ADR 0114 "Terminal notices"), behind this feature's constant.

- **The constant on the page.** `ts.rs` PROTOCOLS now exports `ATTENTION_NOTIFICATIONS_PROTOCOL` into
  `wire/proto.ts` (`null` while unassigned). `data/claims.ts` imports it, typed `number | null`, so neither
  value makes a guard look constant to strict lint. Assigning the number on the node line turns the page's
  claims on with no page edit (node review note on row 2).
- **Notices** (`data/terminal-notices.ts`, `features/notifications/use-terminal-notices.ts`):
  - each connected machine that admits claims has its whole terminal list followed (`terminal.list` with no
    place, waiting on `changed_from`);
  - a connection's first list tells nothing; a Finished or Needs-input attention at a revision not told before
    is a notice; a failed terminal never is;
  - Settings' alerts decide first (alerts off claims nothing); the claim names the exact revision; only
    `admitted` shows, titled "Terminal finished" or "Terminal needs input" with the terminal's title;
  - a click opens a session whose workspace holds the terminal's place (`holds`) and asks that workspace for
    the terminal's tab (`data/terminal-tab-request.ts`);
  - when a list answers without its revision moving, the follower waits a quiet second before asking again,
    so a node that cannot wait does not make it spin (app review note on row 3).
- **Presence** (`data/presence.ts`, `data/shown-terminal.ts`): the workspace's active terminal tab is reported
  as `terminal` focus to a machine that admits claims, and as `elsewhere` to any other. This changes push
  today: before, the page reported the route's session, so that session's push was held back while a terminal
  tab had the focus. Paseo sets the focused agent only for an agent tab (`workspace-screen.tsx:1436-1456`).
- **Viewing clears** (`features/workspace-tabs/use-terminal-attention.ts`): while the page is visible and the
  active terminal asks for the person, `terminal.attention_clear` goes with the exact revision its list shows,
  once per revision, on a machine that admits claims.
- **Tests:** `data/terminal-notices.test.ts` (the first list, a new revision once, the kinds, a failed terminal,
  the exact clear, `holds` for each place kind, the tab request taken once) and `data/presence.test.ts` (a
  shown terminal is `terminal` focus only to a machine that admits claims).

## The page's inbox read (2026-11-21, the claims follow-up's fourth row, page part)

On a machine that admits this feature's constant, the page marks the session shown read with `inbox.read`
(`{session, through}`) instead of `session.mark read` (`features/session/use-read-marks.ts`, `data/claims.ts`'s
`readInbox` and `admitsClaims`). When a read is due is unchanged (ADR 0026).

- `inbox.read` takes the `Read` scope, so a device given only `read` marks too.
- It answers an Ack, and the session's list shows the mark.
- Below the constant, and always while it is unassigned, the read mark stays as it is. The person's own "Mark as
  read" stays `session.mark`.
- The app does the same (ADR 0114 "Inbox read").

## The page's badge and favicon (2026-11-21, the claims follow-up's fifth row, page part)

Not behind this feature's constant: this changes the page today. Paseo shows how many workspaces want the person
on the desktop's dock and swaps its favicon among none, running and attention (`use-favicon-status.ts`,
`desktop-badge-state.ts:5-14`). The page now does both.

- **Paseo's rule, ported** (`web/src/data/badge.ts`):
  - each agent falls in one bucket, by priority needs input, failed, running, attention, done
    (`protocol/src/agent-state-bucket.ts:14-36`);
  - a workspace takes the highest-priority bucket of its agents;
  - the badge counts workspaces in needs input, failed or attention;
  - the favicon looks at the agents themselves: running when any runs, else attention when any wants
    attention, else none. A failed agent turns no favicon.
- **Vessel's buckets:** needs input is the node's `blocked` phrase, failed a failed session, running a live
  one, attention an unread one (`marks.unread`, Vessel's "requires attention"), done the rest. Archived
  sessions count for nothing. A workspace is the sidebar's: the session's project or worktree root, or the
  session alone.
- **The badge:** `navigator.setAppBadge(n)`, or `clearAppBadge()` at none, where the browser offers it (an
  installed page). Elsewhere nothing is set. `features/notifications/use-badge.ts`, mounted by the shell.
- **The favicon:** the build writes `favicon-running.svg` and `favicon-attention.svg` beside `favicon.svg`
  (`web/vite-mark.ts`): the same tile with Paseo's dot at its ratio (cx = cy = 570, r = 130 of a 700 side) and
  colours (#3b82f6 running, #22c55e attention). The hook points `link[rel=icon]` at the one in force.
- **Native:** the row's app part follows on the app line. An OS dock or taskbar badge needs a GPUI patch,
  which stays a stated difference (TRACKER 831).
- **Tests:** `data/badge.test.ts` (each bucket; a workspace's most pressing session and the count; the
  favicon's three states); `e2e/brand.spec.ts` (the icon link is one of the three; both variants are served
  with their dot).

## Turned on (2026-11-21, impl U; the number is given at the node-line merge)

`ATTENTION_NOTIFICATIONS_PROTOCOL` is assigned: drafted 47 in this packet (after T2's P5a 45 and G's P6a 46), and
the integrator sets the final number at merge, in merge order (`PROTOCOL_VERSION` with it, docs/PROTOCOL.md "Version
47"). Everything below that was
pending on it goes live together, and every test that read "while unassigned" now runs its assigned branch.

What this packet adds for it, each with the coordinator's approval (2026-11-21):

1. **Paseo's Ctrl-C/Esc fallback** (`terminal/terminal.ts:180-182,1164-1166`). A terminal input that is exactly
   `\x03` or `\x1b` applies `TerminalActivitySignal::Interrupt` to that terminal
   (`vessel-session/src/terminals/attention.rs` `interrupts`, `handle.rs` `send`). Working becomes unknown, and any
   other state stays. Claude Code runs no `Stop` hook on an interrupt, so without this the terminal would stay
   working. An Esc that starts a longer sequence (an arrow key) is not one.
2. **`vessel hooks` end to end** (`vessel-cli/tests/cli/terminals/hooks.rs`), on a real node:
   - in a Vessel terminal, `vessel hooks claude UserPromptSubmit` then `Stop` give Working, then Finished, on the
     terminal;
   - a standalone Esc after another `UserPromptSubmit` gives unknown.
   The command reaches the node through its local endpoint with the terminal's capability, so this is also the
   IPC route's wire test owed below (`ipc::admit` → `admit_producer` → a report).
3. **The worker's per-burst pushed set, integrated** (`push/tests/node_delivery/terminal_delivery.rs`). After a
   terminal's Finished is pushed, a second finish at once falls in the terminal's gap and is not pushed.
4. **A failed first inbox page takes ADR 0114's fallback** for every notice ("The clients' session claims").
5. **Only a notifying window is elected.**
   - Paseo elects only among clients subscribed with `notifications: true` (`websocket-server.ts:2884-2894,
     2987-2989`; `session.ts:2336-2345`), and its app always subscribes so (`session-context.tsx:492`).
   - Vessel's windows have their own alerts. A window that only sounds, or tells nothing, would otherwise be
     elected as the latest active, and the window that shows notifications would hear `not_recipient`.
   - `ReportPushPresence.notifies` says whether the window shows the system's notifications. It is versioned to
     this constant, and absent reads true.
   - `push/policy.rs` `plan_for` elects only among notifying windows. Every present window still counts for
     suppression, and for push: a person present only at silent windows is not pushed, and no window is admitted.
   - A non-notifying window never claims, and plays its own sound (ADR 0114). The page:
     `features/notifications/notifies.ts`, `use-notices.ts`, `use-terminal-notices.ts`, `use-presence.ts`. The
     app's half is on the app line.

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
   At assignment (done, "Turned on" item 2): the IPC route (`ipc::admit` to
   `admit_producer` to a report) with the assigned protocol, as the wire test
   the "Activity-only local producer capability" section already requires.
6. The terminal list wake. Done: T2's `Shown` holds the attention once the
   feature is assigned (left out while unassigned, when every peer has it cut).

At assignment (the node review of the owed packet, 2026-11-20, PASS):
- the worker's per-burst pushed set gets its integration test beside the IPC
  route's (done, "Turned on" item 3);
- the spacing reads the wall clock, so a clock step moves a gap; a monotonic
  `Instant` would ignore steps;
- `pushed` takes the time after the burst's deliveries, which only lengthens a
  gap by their duration;
- a burst still gives each device one source, as before.

At assignment (the node review of the agent hooks, 2026-11-20):
- Paseo clears a terminal's working activity on a standalone Ctrl-C or Esc in its input
  (`terminal/terminal.ts:180-182,1164-1166`), because Claude Code runs no `Stop` hook on an interrupt.
  Vessel had no such input-side fallback, so after Esc during a Claude turn the terminal would stay
  "working". Done with the assigned wire ("Turned on" item 1).

Recorded, not owed (they fail closed or are allowed by the design):
- one malformed parentage row fails a whole `pending_push` page through
  `vessel_has_current_parent`;
- a data directory made by a staging-line build (claims at schema 22) fails to
  migrate with "table already exists";
- the push worker marks terminal revisions seen before its store reads, so a
  store error loses those pushes silently ("losing an opportunity");
- `CredentialHash` serves device, agent and terminal secrets without domain
  separation.
