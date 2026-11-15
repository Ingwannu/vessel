# D2: supervised scripts, service runtime and setup progress

Status: implementation; this replaces the old B1 package scope. The existing
B1 execution/card skeleton is retained, not a claim of full D2 parity.

## Scope and shared ownership

The detailed Paseo 0.15.4 survey covers scripts configuration, supervision,
service environment/port plans, local/public proxy routes, TCP health, status
projection and workspace menu, plus setup progress/explicit trust release.
Project configuration editing is C1's responsibility; this package consumes
its approved definitions. The workspace node track owns missing runtime
contracts; native code must not invent wire calls or infer health from Running.
The browser pane belongs to its own package. Until that host exists, an explicit
preview opens the platform browser, after a validated destination and socket check.

[Decision Log]
- Purpose and intent: one end-to-end surface for a workspace's supervised work.
- Existing implementation and constraints: protocol-11 script.list/run/restart,
  terminal.kill and port.list exist; native project approval and the worktree
  start/setup log already exist. The newer declared port and service_ports fields
  are separately reviewed on the node track; allocator/env/proxy/health are absent.
- Alternatives considered: run local shell commands; fabricate service URLs or
  probe health from the desktop; reuse the node supervisor and typed observations.
- Choice: extend existing view/link/workspace areas, leave node-authoritative
  features pending their reviewed contract and keep explicit unsupported states.
- Why: the client cannot know a remote process's bind interface, port allocation,
  trust state or credentials. A running terminal is not a health probe.
- Advantages, costs and effects: early native execution is useful against existing
  nodes; D2 remains incomplete until missing runtime contracts are implemented.

## Reference mapping and implementation boundaries

Paseo sources already opened for B1 are in [wp-b1.md](wp-b1.md). Additional survey
requirements and reference paths to verify directly before implementation:

| Capability | Paseo reference | Vessel reuse / remaining work |
|---|---|---|
| Configuration | protocol/src/paseo-config-schema.ts; server/utils/paseo-config-file.ts | C1 edits definitions; this package shows every approved field or refuses approval |
| Run/stop/restart | server/worktree-bootstrap.ts:518; app/screens/workspace/scripts/use-workspace-script-actions.ts | Existing node calls and native skeleton; node-side restart deliberately owns the operation |
| Service env/ports | server/workspace-service-env.ts; workspace-service-port-registry.ts; workspace-service-port-allocator.ts | Missing reviewed runtime contract; no guessed environment or allocation |
| Proxy/route aliases | server/service-proxy.ts; script-route-branch-handler.ts | Missing reviewed contract and ownership reconciliation with preview ADR; never construct fake proxy routes |
| Health/projection | server/script-health-monitor.ts; script-status-projection.ts | Missing reviewed wire contract; extend pure state with supported/pending/healthy/unhealthy when supplied |
| Menu/routes/copy | app/screens/workspace/scripts/*; utils/workspace-script-links.ts; workspace-service-routes/store.ts | Existing execution card to gain a workspace action/menu and typed route preference; current explicit/direct observed preview remains useful |
| Setup progress | app/panels/setup-panel.tsx; stores/workspace-setup-store.ts; server/worktree-session.ts:507 | Reuse native workspace/start_card, start_header, setup_log and worktree.output; preserve full logs and explicit approval |
| Helper ledger | server/managed-processes/managed-processes.ts | Provider-runtime concern, not script/terminal ownership; audit with node owner, no second client ledger |

Paseo's TCP monitor uses a three-second single-flight poll, a five-second initial
grace, a 500 ms connect budget and two failures before unhealthy; these are node
behaviour, not permission to probe remote service ports from this desktop.
Its port allocation helper is bounded by ten seconds and 1024 stdout bytes, with
at most ten dynamic uniqueness attempts; the node implementation must own them.

## Resume point

Execution/status/observed-port/card skeleton is committed. Changed-crate clippy
and tests are in progress. The central Step 0/package bases have been merged;
no independent node-line merge is used for the app. Next: make the current vertical slice compile,
add workspace-scoped entry point and terminal reveal using the terminal package's
existing API, audit setup observation against the current start card, and consume
only reviewed node runtime contracts. One whole native gate and one package review
follow completion. Missing env/allocator/proxy/health remain explicitly unverified.

## Existing setup audit

The existing start card already has making, exact-command approval, setup running,
failed actions and streamed combined output. The link follows `worktree.output` and
retains the created worktree on cancellation. The old code treated `SetupState::Unknown`
as successful completion; this package refuses automatic session creation for that
state and shows a complete translated explanation. A regression enumerates all known
setup states. No new wire call is involved.

The current protocol has a combined setup log and one overall state, rather than
Paseo's per-command duration/result rows and persisted blocked-source observation.
Those facts cannot be reconstructed reliably from terminal text. Workspace registry
identity is the shared registry contract; setup's passive pane will use the layout
package's `Target::Setup`, and scripts its `Target::Scripts`. Terminal reveal uses
`Target::Terminal(TerminalId)`. No competing local workspace identifier is introduced.

Missing runtime/registry contracts are dependencies, not accepted parity differences.
The tracker must not mark service allocation, env, proxy, health, route preference or
workspace status delivery passed on the strength of the existing script/port card.


Pure `scripts::services` now defines the survey's pending/healthy/unhealthy states,
per-row and aggregate tones, validated public/proxy/direct targets and preference
fallback/deduplication policy. These are view rules tested with fixed data, not a
new wire API. No current node is reported healthy and no proxy URL is fabricated.
The native adapter will accept them only from the workspace track's reviewed report.

## Package-base merge

Merged the announced package base `8d5513e4` under A8. The only textual conflict
was the native size allow-list: the head's removal of the now-split link root wins;
this package's already-reduced project/detail records remain reduced. No node-head
merge or fake harness conflict was resolved independently. The legacy workspace
registration moved into this package's actor module, so it remains outside the
split shared link root. The root crates changed only by consuming the announced base.

Setup observation/cancellation is this package's node responsibility (ADR 0093),
on the announced node head with the shared method table and pending protocol rows.
Allocator/environment/proxy/health remain the separate workspace runtime package.

## Declared-port adapter after package-base merge

The actual package base already includes private ScriptEntry kind/port and
its complete five-field view. The native execution card now projects all
five fields explicitly, uses ScriptEntry::new in fixtures and labels the
approved port as a declaration. It never turns that value into a listener,
health claim or preview authority. Existing approval rows and changes already
consume the complete view on the package base. The later optional global
service_ports block is not present on this base and remains pending its join.

The first remote narrow submission was interrupted before registration by
the shared submit-lock incident (tool-owner inventory: no run for c350918f).
It produced no compile or test result. A local narrow check covers the updated
adapter; a fresh committed snapshot will be used for the next remote request.

## Cancellation while the last legacy output page is in flight

The legacy start walk now rechecks its answers channel after an awaited
setup-output reply and immediately before creating the session. Otherwise
cancelling while the last successful output page was in flight could still
create a session. The existing button now says Cancel session creation; its
channel closure does not signal the node's setup runner. Actual runner
cancellation remains the separately correlated ADR0093 operation.

## Shared package base 2 and provider-ledger boundary

Consumed announced app base 6036eeb8 after the local app build completed. Its
Machine.node is the welcome identity, and its kit owns TextField Editability
and event listening. The size conflict retains both this package's existing
reductions and the head's smaller field module; no record was raised.

Coordinator ruling moves managed-process helper ledger tracker508 to A9/W22.
The survey says it belongs to provider helpers, not scripts/terminals; W8
adds no parallel persistent process reaper. D2 still owns the native surface
and ADR0093 setup runner/observation/automatic terminal bootstrap.

## Initial isolated runtime smoke (before package base 2 rebuild)

The dbfc9a5d binary connected to a private protocol-15 fake fleet. The app
approved its exact fixture set, ran a plain command to exit 0 and another to
exit 7, started its HTTP service, and displayed its observed loopback listener.
Explicit host input enabled the external-preview control. The service remained
alive after the app exited; its process ancestry was verified against that
private fleet. This is legacy-call evidence only, not the final capture/gate.

Browser delivery is unverified: the initial desktop MIME handler selected a
different installed application. The current QA environment uses private data
and MIME directories and a fake opener; it does not launch a real browser. No
operator MIME/profile setting is changed. Restart/stop are also not yet proved by this
smoke; state-only debug messages distinguish a stale/view-refused click from
a dispatched intention without logging a command, host or credential.

## QA opener restriction

The coordinator prohibits real desktop URL launchers in tests/probes after the
initial system MIME handler launched another desktop application. GUI checks
now place a fixture-only fake xdg-open first on that app process's PATH. It
records the exact allowed localhost URL and always exits successfully so the
desktop cannot fall back to another real launcher; a separate assertion checks
its receipt and refusal files. No operator PATH or MIME settings are modified.

The earlier unsandboxed browser preflight is excluded from final acceptance
evidence. Real browser checks, if needed, may use only the operator-provided
/opt/vessel-cft Chromium with its sandbox enabled; no production browser or
host sandbox policy is changed.

## Complete automatic terminal display (approval still refused)

The native card now retains every automatic terminal's position, optional title
and exact shell command. Positional comparison shows name-only edits, swaps and
complete removals against the last approved set or approved-folder fallback.
Both current and prior values are drawn in full. Duplicate titles never merge
resources. A new FileTerminal field breaks the exhaustive display adapter.
Comparison prose was extracted from the oversized language modules; no size
record was raised. New labels and the additional-change summary are Fluent EN/KO.
The existing approval-refusal cases stay unchanged: display support does not
assign PROJECT_FILE_KEYS or enable automatic execution. The allocation block
still awaits B0's app-base join, while its reviewed data exists in node ba45a6e3.

A post-write snapshot is now retained for drawing only, so reconciliation does
not collapse the card and jump the scroll position. Ready remains the sole
permission source. A failed refresh or new owner cannot use this retained content
for execution or preview; tests exercise these transitions and owner replacement.

The second local native all-targets build reached the 600-second short-lane
limit while compiling GPUI dependencies, with no compiler error emitted. This is
an incomplete build, not a test pass or source failure. Remaining builds use the
ordinary lane; narrow pure view/link checks remain separately scoped.

## Legacy preview callback fencing

The existing node restarts a live service in the same TerminalId. The external
browser callback now retains the terminal publication revision as well as the
observed socket, and refuses a changed revision, starting, backoff or unknown
state. A regression keeps the same terminal id, PID and socket while advancing
its revision, then proves the old callback is refused. This is a UI event fence;
it does not turn the legacy cached port scan into a service lifetime attestation
or authorize the isolated in-app Preview profile. The new node runtime's exact
workspace/lifetime/scheme contract remains a separate dependency.

## Allocation display and current public-exposure boundary

Every present allocation block now renders its inclusive range and exact approved
port-program path, with absent choices explicit. Current, previous, removed and
approved-folder fallback values are retained even with zero services. The UI
compares both choices independently of runtime precedence. Approval remains a typed
refusal pending the complete feature's negotiated support; showing configuration
does not allocate a port or establish a listener. No value is passed to an opener.

Coordinator ruling relayed by the workspace runtime owner: service proxy defaults
to loopback. External publication requires explicit per-service Admin opt-in and
must warn, with the exact workspace/script, that anyone with the URL can access it
without Vessel authentication. A wildcard node listen must never turn publication
on automatically. The new node field/API is not ready, so the app adds no guessed
exposure RPC, preference or optimistic public state. The private in-app Preview
uses attested backend origin and remains separate from this proxy.

App compilation follows the shared B0 line. The temporary own node runtime merge
was reverted; the complete node implementation remains in its separate tree. A
values-only carrier441f71013 is under V3 review and cannot be consumed before source
pass and integrator adoption. The app's crate tree equals B0's approved root.

## Shared connection admission for scripts

[Decision Log]
- Purpose: captured Run/Restart/Stop and system-browser actions must retain the
  exact authenticated node admission even if a saved alias reconnects.
- Existing assets: base6 supplies Machine.connection_context and the actor-owned
  SelectedConnection before-poll/after-result fence. A raw NodeClient selected by
  alias cannot prove that it is the admission the user saw.
- Alternatives: per-area epochs or another node-id cache would duplicate G4's
  authority. Neither is introduced.
- Choice: Target carries the shared immutable ConnectionContext; settings keeps
  a separate navigation-only project selection. The actor admits the exact
  alias/context and each typed RPC uses SelectedConnection. Controls capture
  their Target, recheck the current fleet admission at dispatch/open, and stale
  receipts cannot hydrate a replacement admission.
- Effects: background reads may rebind only after the new shared snapshot;
  mutations never replay. A disconnected/legacy snapshot has no operation
  authority. Script discovery itself supplies fresh project commands after a
  write, avoiding a duplicate unscoped ProjectAsk read. Peer failures retain
  structured codes without forwarding raw peer diagnostic strings.

Regressions use the existing in-memory framed peer and actor admission factory.
They exercise retirement after capture before first poll (zero script frames),
all existing lifecycle IDs/refusals, and same-node reconnect rejecting an old
read receipt. These checks do not establish canonical WorkspaceId service
authority, which remains the separate node runtime dependency.

## Scripts panel boundary

The existing W12 registry hosts the same script rows in a passive Scripts tab.
A launcher button opens it with the captured LayoutKey, and exact TerminalId
reveal uses W12's canonical open boundary. C2 owns the terminal renderer; its
pending resident renderer is not supplied by this package.

Owner state is keyed by (LayoutKey, TabId). An app-only monotonic presentation
instance additionally correlates replies when a forgotten layout reuses a tab
number; it grants no workspace admission or process authority. Closed tabs drop
only display state. Polls require the canonical panel-visibility query and an
active OS window. Hidden receipts settle state without starting another read.
Legacy Session keys map only to their exact ScriptPlace::Session; root-wide
legacy results are filtered to TerminalPlace::Session with that exact id before
display/action admission. Canonical Workspace keys show unavailable until W11
and D2 provide the reviewed runtime contract. No selected-session/cwd fallback
is used for them.

The renderer shares lifecycle/status/listener/host controls with project settings.
Drawn mutations capture both original admission/owner and the successful read
identity. A refreshed approved command with the same name cannot inherit an old
Run button. Preview still checks the current publication/socket at opening and
does not become in-app Preview authority. Regressions cover repeated tab/read
numbers across panel recreation, canonical-key refusal, explicit-session mapping,
shared-root foreign-run exclusion and stale drawn actions after refresh.

Observed-listener rows also copy the validated URL through the kit's existing
clipboard boundary. Copy and Open share the final captured owner, publication
revision and current socket/host recheck; neither uses a stale rendered URL
string. The current route is the legacy direct-host route. Proxy/public route
selection and persistent preference remain unavailable until their typed
runtime producer exists.

The settings-card timer also uses the captured window and stops reads while
the OS window is inactive. Mutation receipts only settle state; the visible
window timer performs reconciliation, so a late reply cannot restart polling
in the background. Final URL/lifecycle callback dispatch checks OS activity.

A changed shared connection admission clears an entered service-host override
before the first new read. Host text chosen for an earlier physical peer must
not silently migrate to its replacement, even under the same saved alias.

G4 confirmed that SelectedConnection has no reviewed safe direct-route
projection. The earlier saved-book host suggestion is removed: rereading an
alias after admission could describe its replacement binding. Current reads
therefore supply no suggested host; a person explicitly chooses the host their
system browser can reach. A future suggestion requires E2's coordinated
authority-binding projection. No connection internals or extra route resolver
are added by this feature.

The legacy node still reserves scripts by (root path, ScriptName), as
service/scripts.rs154-174 and terminals/registry.rs define. Two explicit legacy
sessions sharing that root may therefore receive AlreadyRunning rather than two
independent processes. The session card filters foreign TerminalPlace owners and
never adopts that refused TerminalId or reinterprets root equality as canonical
workspace ownership. Exact WorkspaceId service lifetimes remain unsupported
until the new runtime contract replaces that legacy reservation boundary.

The staged health-tone policy now follows script-status.ts's exact running-only
condition. Starting/backoff/ended/unknown states cannot inherit green or red
from a previous child report. This remains a pure presentation regression, not
a live health consumer or a service lifetime grant.

Retained processes render TerminalInfo.folder as their observed directory. A
session-placed terminal or an omitted TerminalPlace.dir does not prove that an
old command ran at the project root. Current configured rows still render the
exact approved relative dir; observed cwd is a separate presentation field.

Existing service/scripts.rs216-242 restarts the stored run plan and approved
digest, not the newly displayed file command. Configured rows now show both the
current declaration and recorded TerminalKind command/kind plus actual folder;
the explicit Restart note states which command/directory are repeated. Retained
rows already show that recorded plan. A changed file therefore cannot silently
relabel the command a Restart action will execute.

The first changed-source strict native probe reported documentation/copy-argument
lints, an oversized legacy approval Step and one unconsumed ready reference.
These are corrected without assertion/lint exceptions. The Step boxes its
comparison payload to keep additive declaration fields from growing every
setup event. The old raw-client scripts Hub branch now refuses explicitly;
only actor SelectedConnection preflight may dispatch script RPCs.

## Command reads and approval admission

[Decision Log]
- Purpose: a project confirmation must retain the connection and complete digest
  originally read, even if its saved alias is rebound before dispatch.
- Existing assets: G4 ConnectionContext/SelectedConnection, Reading/RequestKeys,
  the existing protocol-10 GetProjectCommands/ApproveCommands and full approval
  projection. No new wire method or version is needed.
- Alternatives: take the later Settings selection or a logical ProjectReadKey as
  authority; make approval depend on newer script.list support. Both lose a
  required boundary or older-node compatibility.
- Choice: a dedicated area-lived command read/confirmation model shares the same
  actor admission guard and bounded typed call helper as Scripts. Tickets retain
  original context/project/digest/snapshot. Explicit reads can choose a new
  admission; confirmations never retarget or replay.
- Effects: commands remain readable on protocol-10 nodes without script.list.
  Review refreshes the complete set before offering confirmation. Read/loading/
  failure/duplicate/stale callbacks cannot approve. Withdrawal removes only the
  inspected grant. Unsupported executable fields still refuse new approval.

Raw ProjectAsk::Commands/Approve routes now return unsent without polling a
client; uncorrelated legacy command receipts cannot hydrate the detail. Scoped
command replies alone update the new model. Script discovery no longer writes
that independent approval cache. Unknown mutation results require explicit read.

Detail's command renderer moved into detail_commands.rs, shrinking its inherited
size record. Detail::new(ProjectId) is unchanged. load_detail now takes mutable
Shell, Window and Context so every settings entry starts both independently scoped
command and script reads. Worktree/snapshot/remove methods remain unchanged.
Tests use the canonical test actor admission factory and in-memory framed peer,
covering exact digest/owner receipts, retired-before-poll zero frames, stale read/
confirmation refusal, unknown-write reconciliation and raw peer message redaction.

Mutation/preview/approval widget ids derive from their existing receipt ticket
(and panel incarnation), so a pressed old widget cannot become a refreshed
command or confirmation under the same button id. Listener controls additionally
include exact TerminalId to avoid equal-port collisions across runs. These are
presentation keys, not another epoch allocator or approval authority.

The extracted command card uses the existing kit spacing scale rather than
transferring a legacy pixel literal to a new file. Its original detail leaf's
remaining literal allowance shrinks with the extraction; no token budget grows.
