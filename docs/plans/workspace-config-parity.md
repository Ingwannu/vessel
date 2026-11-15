# Project configuration, as Paseo's

Status: slice 1 reviewed; slice 2 is split into independently reviewed changes. ADR 0066.

## What Paseo does

Read in Paseo 0.15.4. Paths are the package paths in that tree.

`paseo.json` at the repository root (`packages/protocol/src/paseo-config-schema.ts`,
`packages/server/src/utils/paseo-config-file.ts`):

- At most 256 KiB, a regular file. Unknown keys are kept.
- `worktree.setup` and `worktree.teardown`: a string or a list. Blank
  entries are dropped (`normalizeLifecycleCommands`). Each kept string
  is one shell command, in order, in the worktree. The first failure
  stops the list.
- Setup reads the file on disk in the worktree
  (`packages/server/src/utils/worktree.ts`, `runWorktreeSetupCommands`).
  The public worktrees page says the committed file of the base branch;
  the code that runs the commands reads the worktree copy. A different
  function only compares setup with `HEAD` and reports whether it
  differs (`worktree-setup-commit-status.ts`).
- `worktree.terminals`: a list of `{ name?, command }`. Bad entries are
  skipped. After setup has succeeded, each becomes an interactive
  terminal; the command is typed in, plus a carriage return, after the
  first output or 1.5 s
  (`worktree-session.ts` then `worktree-bootstrap.ts`,
  `runWorktreeAutoTerminals`). A failure is logged and does not undo the
  worktree.
- `scripts.<name>`: `command` (a string; anything else drops the
  script), `type: "service"` or not, optional numeric `port` kept only
  for a service (`getScriptConfigs`). A service is supervised, given a
  port, and probed. A plain script runs once.
- `worktree.servicePorts`: `range` or `portScript` or both; at least
  one. `portScript` wins over `range`. An explicit service `port` wins
  over the block. The project block replaces the daemon-wide
  `worktrees.servicePorts`; it does not merge with it
  (`workspace-service-port-allocator.ts`). The allocator requests direct
  execution: four arguments, 10 s, 1024 bytes, one TCP port. On Windows,
  the plugin's `process.ts:36–44` overrides `shell: false` for `.cmd` and
  `.bat` and inserts `cmd.exe`. Vessel deliberately refuses that automatic
  shell path with `PortScriptError::WindowsCommandScript` to keep a chosen
  branch name out of interpreter reparsing (ADR 0066). Its runtime test
  `windows_command_scripts_are_refused_before_spawn` is still owed.
  A reserved port
  is refused. A port the script returns is not probed first.
- Health (`script-health-monitor.ts`, `local-service-connection.ts`):
  TCP connect to loopback, not HTTP. `pending` for 5 s, then every 3 s
  with a 500 ms probe. Two failures become `unhealthy`. A success
  becomes `healthy`.
- `metadataGeneration.{title,branchName,commitMessage,pullRequest}.instructions`
  replace the default wording for that kind of generated text
  (`public-docs/metadata-generation.md`). Not commands.

Not in `paseo.json`:

- Auto-archive after a merged pull request is `daemon.autoArchiveAfterMerge`,
  default false (`packages/server/src/server/config.ts`). The safety
  conditions are a later slice; they were not re-read for this one.
- Editor targets are a closed list under
  `packages/desktop/src/features/editor-targets/` (VS Code, Insiders,
  VSCodium, Cursor, Zed, Xcode, Android Studio, the JetBrains ids, Trae,
  Kiro, Antigravity, and the file manager). Detection and the open call
  are a later slice.
- `worktrees.root` is the daemon file.

## Vessel before this track

`crates/vessel-workspace/src/project_file.rs` and `project_file/scripts.rs`:
`vessel.toml`, setup, teardown, scripts, services, `dir`, digest versions
1 and 2. Unknown keys are ignored and are not digested. A bad file runs
nothing.

`crates/vessel-proto` `CommandSet`, `ScriptEntry`, `ServiceState`.
Services restart (1, 2, 4, 8, 16 s; a 60 s run resets; the sixth failure
stops). The node finds listening ports; it does not assign one
(ADR 0031). Approval is one digest per project, refused to agents
(ADR 0027). Protocol version is 20.

No `terminals` key, no `port`, no `service_ports`, no health enum, no
auto-archive, no editor target. Searched as those names in the node
crates while writing ADR 0066.

## Reuse

| Need | Existing | Decision |
| --- | --- | --- |
| Project file and digest | `project_file` | Modify. A new file would split the approval. |
| Command, path, script name, title | `ProjectCommand`, `RelPath`, `ScriptName`, `SessionTitle` | Modify. No new stringly types. |
| Service run, terminal caps | session terminals, ADR 0031 | Modify. Terminals to open are shells that already exist. |
| Port discovery | the existing scanner | Leave it. Assignment is a different fact (ADR 0031, option 1, which that ADR deferred). |
| `paseo.json` parser | none, and ADR 0027 refused that filename | Do not add one. ADR 0066. |
| Approval text and script comparison | CLI workspace formatter and web command-changes | Modify. Keep their behavior in focused modules and name every entry field before adding port data. |
| Crate edge or dependency | none needed | No new edge, no new crate, no new dependency. |

## Slices

1. **Done as a decision.** ADR 0066: the key table, the refusal to read
   `paseo.json`, and the contract for the missing keys (version 3
   digest). The wire number is given at merge. The code, when it lands,
   gates the fields on one constant and does not change
   `PROTOCOL_VERSION` until that merge. No code yet.
2. **In progress.** Parse the missing keys, put them in the version 3 digest,
   open the terminals after a successful setup, allocate a service port,
   report health. Tests beside those rules. A file with none of the new
   keys keeps digest version 1 or 2 byte for byte. Wire fields are
   additive; a peer older than the assigned protocol sees a version 3
   set as invalid and cannot approve it.
3. **Not started.** Auto-archive after a merged pull request, with
   Paseo's safety conditions, default off. Reads the forge's merge
   state; does not change the forge client or the git implementation
   beyond that read. Its own section of ADR 0066 when it is designed,
   not before.
4. **Not started.** Editor targets: detect on the node's machine, open a
   path in one. A validated target enum, never a shell string. Its own
   section of ADR 0066 when it is designed.

Out of this plan: `.worktreeinclude`, a node-wide port range, and a
service proxy. `metadataGeneration` is ADR 0062's `[metadata.title]`,
`[metadata.branch]`, `[metadata.commit]` and `[metadata.pull]`, not this
track's parser, and not part of the command digest. A streaming setup
panel is not a slice: Paseo pushes `workspace_setup_progress`, and Vessel
already has `WorktreeInfo.setup` plus `worktree.output` (ADR 0066).
Adding a second push is not one of this track's slices.

## Slice 2a: terminal approval contract

Reference read again before implementation:

- `packages/server/src/utils/worktree.ts:285-316`: file order, commands
  trimmed, blank commands skipped, names trimmed and blank names absent.
- `packages/server/src/server/worktree-bootstrap.ts:271-343`: readiness
  on first output or the 1.5 s deadline, carriage return, concurrent opens.
- `packages/server/src/server/worktree-session.ts:549-597`: terminals
  follow completed setup; setup failure bypasses them.
- `packages/protocol/src/paseo-config-schema.ts:74-97`: tolerant decoder.

Decision: modify the existing `ProjectFile`, approved `CommandSet`, reply
adaptation, and CLI rendering. Reuse `ProjectCommand`, `SessionTitle`, the
existing SHA-256 encoding and serde; no external dependency or crate edge.
One terminal decoder owns the new list; a second project file would split
approval. It uses JavaScript whitespace trimming before existing validators:
BOM is trimmed and NEL is kept, unlike plain Rust `str::trim`. Differences remain ADR 0066's: TOML only, entire-file refusal,
32 terminals and bounded titles, and sequential opens when execution lands.

This change parses only `[[worktree.terminals]]`, includes it in the
approval snapshot and displays it in the CLI formatter. It does **not**
open terminals. Port keys, allocation, health, and automatic opening follow
in separate changes. Version 3 already writes the empty service-port fields
and the empty port slot per script, so completing those keys will not
change a terminal-only file's digest. Fixed byte-encoding vectors pin
versions 1, 2 and 3, independently of the implementation.

`PROJECT_FILE_KEYS: Option<ProtocolVersion>` is `None` until integration
assigns the number and clients can display the complete set. No sentinel
and no `PROTOCOL_VERSION` bump. All negotiated peers currently receive
`invalid` for a set with terminals (or a previous approval with terminals),
including script list/run/restart approval replies and worktree creation.
Tests also exercise the assigned-threshold branch with a synthetic version.
An older node's absent terminal list decodes as empty. Existing files without
terminals retain their old bytes and normal approval path. The web dialog
and comparison each project every field by its generated type (including
optional ones). Until terminal display/comparison lands, terminal-bearing
current or previous sets return a typed refusal; no approval button is
shown and a partial comparison cannot claim the commands are unchanged.

## Resume point

- Done: slice 1's reviewed ADR and follow-up amendments; slice 2a's
  reviewed terminal approval contract and exhaustive web refusal.
- Integration: node baseline `a365d295` is merged with both architecture
  texts preserved. Its root and web gates passed with no waiver or retry;
  the earlier protocol-test waiver no longer applies. `PROJECT_FILE_KEYS` stays
  unassigned.
- Integration update: node baseline `f453ec2e` is merged after the
  declaration-boundary gate. Its protocol/store constants, method tables,
  shared tests and documents remain intact. The shared `required_protocol`
  accessor is available; `PROJECT_FILE_KEYS` stays unassigned. The combined
  gate now covers this baseline and the TOML port parser. Native view/card
  adaptation remains an integration prerequisite.
- Implemented: slice 2a's terminal decoder, approval fields, digest vectors,
  compatibility and CLI rendering. The slice's validation results belong in
  its review report; no runtime behaviour is implied by parsing these keys.
- Half-done: automatic terminal execution and port allocation are absent;
  declared service ports and allocation choices are parsed only into approval
  data. No helper is invoked. `paseo.json` is not read.
- Next: keep `PROJECT_FILE_KEYS` unassigned until the web approval dialog
  displays terminals, web `command-changes` compares them, and the native
  approval card displays them, including declared ports and every allocation
  choice. The current web refusal is a deliberate stopgap; display/comparison lands together with the native card's work.
  The native lead owns its card and exhaustive adapter. This track owns
  the web refusal now and its later terminal display/comparison.
- Implemented port values: `ServicePort` and `PortRange` keep checks and
  refusal text on shared constants. `ServicePort` now occurs in the nested
  declaration field below; `PortRange` enters the approved allocation block
  with its literal validated program. The existing serde conversion pattern
  is reused;
  observed listeners remain a separate fact from a declared or allocated
  service port.
- Implemented value contract: `ExecPath` keeps private literal text and
  its Unix, Windows or shared grammar. Its decoder and serde check raw
  components without filesystem access or native normalization; five
  portable fixture tests cover prefix forms, components, round trips and
  the shared byte bound. Rust's Windows prefix source also recognizes a
  forward slash in the verbatim UNC marker; the remaining verbatim
  components still use backslashes. It is carried through the block's
  `PortScript`, without a separate wire field or direct project-file key.
  Native invocation and the Windows command-script refusal remain later typed checks, with their tests still owed.
- Approval display: the absolute path now shares `RelPath`'s control
  character refusal. LF, CR, ESC, another C0, DEL and C1 are tested through
  both text and serde for Unix, drive and UNC paths; NUL keeps its own
  diagnostic. The absolute per-component limit remains a native execution
  check, with its reason in ADR 0066.
- Implemented refactor: one `trim_project_text` input rule now serves
  port ranges and terminal entries. Their existing tests and digest vectors
  are unchanged; a shared-rule test pins BOM, NEL and Unicode line/paragraph
  separators. No accepted input, approval bytes or execution path changes.
- Implemented value layer: `PortScript` privately owns a non-root relative
  path or an `ExecPath`, reusing their validation after shared edge trimming.
  `ServicePorts` privately owns the optional range and program and refuses
  an empty block through both its constructor and serde. Unknown keys and
  explicit null choices are refused. Unit tests cover the typed paths,
  source chains, trim behavior, nonempty block and strict serde; a TypeScript
  feature test pins optional fields without nullable values. These types
  now reach the parser and wire only through that approved block.
- Prepared display boundary: the CLI command formatter and web script
  comparison now have focused modules with complete entry projections.
  Existing output and comparison expectations stay unchanged; no field
  or approval behavior is added.
- Implemented declaration boundary: `ScriptEntry` privately owns kind/port;
  constructor and serde refuse a plain script with a port. The CLI names
  every borrowed view field, the web names every nested field and refuses
  current, previous or folder snapshots with declarations. The port is
  covered by the reserved version-three digest slot; old vectors stay fixed.
  The TOML decoder now reads a service's integer port through `ServicePort`;
  invalid numbers and a plain script's port refuse the whole file. Boundary,
  source-chain and digest tests cover this input edge. Native constructor,
  getter and card adaptation remain the native lead's integration condition.
  No allocator or health execution path is present.
- Before port fields: ADR 0066 now defines the `ScriptEntry` construction
  invariant and complete borrowed display view. The native lead owns its
  constructor/getter and exhaustive-card adaptation; web coverage and
  comparison name nested entry fields too. The declaration boundary above
  implements this part; the native adaptation remains owed.
- Implemented allocation boundary: the optional strict nonempty block is
  retained by the file and current/prior approval snapshots even without
  services. Its two reserved digest slots cover both choices. CLI names the
  complete borrowed owner view and prints both; web names every nested key
  while retaining its optional value type, and refuses current/prior/folder
  blocks, including an empty peer object. File, digest, persisted approval,
  six-reply, CLI and web tests cover these boundaries; execution is absent.
- Then: settle the runtime contract before helper, allocator or health code.
  A relative bare
  program is checkout-rooted, using a worktree's own copy in its own folder,
  unlike Paseo's PATH lookup; the execution test proving that selection
  and relative-link containment remain owed. Complete display in all three
  approval clients before assigning the protocol. Runtime follows with approval and lifecycle
  tests. Returned review findings take priority.
- Allocation projection contract: reuse the private `ServicePorts` owner
  with a complete borrowed view before adding its optional block to the
  file and approval snapshots. Keep block presence and both digest slots
  even without services. CLI and the native lead's card must name every
  view field; web names every nested field by type and refuses current,
  prior and folder snapshots until full display exists. The contract and
  required tests are in ADR 0066; the approval field is implemented above,
  and no allocator is connected.
- Still unverified: Windows, actual shells, and negotiated support after
  the integrator assigns the protocol number.

## Runtime package preparation

A14 groups the remaining port runtime into one package with small commits
and narrow checks; one whole gate follows its finished wiring. Security
changes retain per-commit review. The direct-helper contract in ADR 0066
reuses `vessel-proc`, `Root`, the shared text trim and credential removal
inventory; no shell pipeline, scanner truncation or new dependency. Its
staged primitive must not be mistaken for a registered product caller.
The earlier every-start helper and process-spawn health descriptions were
corrected against the actual cache and first-observation callers. Cache
lifetime/refresh and health observation must be settled before their wiring.
A new approval does not implicitly withdraw an earlier digest.

Resume: allocation approval block is committed and its exact-commit root
gate passed; native full-view integration remains owed. Next implement and
narrow-check one direct invocation with the path/cleanup/decoder cases in
the ADR, then the plan owner and health. Auto terminals, archive and editor
targets remain unimplemented. No new protocol number is assigned.

## Independent process lifetime carrier

A standalone change on the assigned node head adds Proc's process-id and
lifetime modules, runner-owned revocation and read-only handle transport.
This does not include the staged allocator or a browser consumer. The ADR
records the reuse choice, accepted platform limits and event-based tests.
The node scanner and Preview must continue refusing authority from cached
PID/port data until their own fresh ownership integration is implemented.

### Carrier implementation checkpoint

The separate process carrier now has runner-issued observations in Proc and
PTY, delegated through AgentProcess and current session/terminal handles.
Revocation precedes signal attempts, follows observed exit/wait uncertainty,
and runs on owner drop. PTY's blocking waiter independently revokes its run.
Session park, wake, profile replacement and service restart replace or clear
the current observation; old observations cannot become live again. No wire,
Preview admission or new dependency is introduced. Source is awaiting its
narrow checks, security review and standalone package gate. Windows execution
and fresh socket/workspace ownership remain unverified.

The direct terminal-session test also checks the SessionHandle's PTY observer,
separately from a standalone shell handle. The issuer guard type itself is
must-use because dropping it revokes authority; its Self-returning methods
rely on that one annotation. Narrow evidence for the earlier snapshot is
kept separately from this pending revision.

The first carrier narrow build found the new explicit wait-error branch
returning std::io::Error rather than the existing WaitError wrapper. The
revision keeps revocation first and restores the existing source-preserving
From conversion. No test or deadline changed; the failed build log is kept,
and no tests ran in that build.

The release-capable child integration has a separate obligation: reject a
bound release without revocation, but revoke before successful ownership
release. Its detached direct-child wait remains the actual exit observation.
This carrier's assigned base has no release operation; these joint tests
belong to the later combined lifecycle integration and are not claimed here.

The next narrow check reached the PTY integration tests and rejected an
unquoted Rust item in their module documentation. The revision fixes only
that documentation and explains why watch revocation is retained even when
no observer has subscribed yet. The source error and documentation-lint
failures remain distinct recorded checks; neither ran tests.

The Proc child stores its private guard as observation, keeping that name
separate from a release-capability lifetime policy in the lifecycle work.
The public lifetime() accessor and all revocation points are unchanged;
this private rename changes no test expectation or runtime behavior.

## Process carrier qualification checkpoint

The four changed crates passed all-target Clippy and their full tests at
3d982ce9: 334 passed, none failed, two ignored across 27 result groups.
All fourteen new PID/watch/child/PTY/session/terminal cases ran. The first
compile-error and later test-documentation lint checks remain recorded as
failures rather than test passes. No deadline or assertion was weakened.

The composed carrier now includes the designated shared base 6c782cb6 and
the reviewed receipt fix c5da5305. The incoming native, proto, store, node,
workspace, testkit and web trees are unchanged. The only later product edit
is the private observation-field rename, with the same public API and
revocation behavior. Qualification of this composition requires the whole
root gate and security review before integration. It introduces no D2
helper/allocator caller, protocol field, number or dependency.

Release-capable child integration, durable process identity and the actual
service/Preview consumers remain separate work. This carrier cannot prove
OS death, persistent ownership or socket/workspace authority. Windows and
native execution are outside this checkpoint's evidence.

## Runtime preparation retained after the shared carrier landing

Direct-helper primitive: staged under the node terminal module's test-only
declaration. One owner resolves the approved program, supplies literal
argument/environment pairs, and decodes bounded stdout only after successful
exit. Both pipes are independently bounded. Unit and real fake-process tests
cover the preparation/decoder and child-owner cancellation boundary; narrow
results are recorded in the review report. No allocator caller or product
execution is enabled, and the whole runtime package gate remains owed.
The Windows classifier is portable-tested; actual Windows path resolution,
symlink target checks and execution are not established on a Unix runner.
The forthcoming workspace registry may replace the current project/worktree
place-id mapping; that integration waits for its reviewed API.

## Service runtime package scope

The package now owns the node allocator, service environment, service
proxy/routes and health/status projection together. Its app consumer is the
workspace scripts area; browser Preview consumes a fresh server-side
ownership observation, rather than treating a cached detected port as an
authority. The shared workspace identity/registry is ADR 0082's contract;
no second workspace identity is introduced here. Before production wiring,
complete the cache/restart/lifecycle, health observation and route security
contracts in ADR 0066 against their actual Paseo callers.

Reference inventory: `server/workspace-service-env.ts`,
`workspace-service-port-registry.ts`, `workspace-service-port-allocator.ts`,
`service-proxy.ts`, `script-health-monitor.ts` and
`script-status-projection.ts`; script entry/exit is `worktree-bootstrap.ts`.
The existing listener scanner is reusable, but its cached PID/port result
has no owner-generation proof and drops the scan's truncation into a debug
message. It cannot authorize Preview. Fresh complete ownership, process
lifetime/generation and immediate revocation need an explicit shared
contract before browser integration; scheme needs its own bounded probe.
No route URL, browser capability or health API is promised by this plan.

Current state: the approved data/parser layers are reviewed. The helper
primitive is committed under the test-only module and is undergoing narrow
Clippy/test checks. There is no product allocator/proxy/health caller.
Whole-package gate and integration review remain owed.

Service environment-name preparation: modify the existing `ScriptName`
asset rather than parsing names again in the allocator. Its ASCII grammar
allows the standard library's uppercase/collapse rule to reproduce Paseo's
`workspace-service-env.ts:25–31`; no Unicode dependency or new wire field is
needed. A derived private-storage `ServiceEnvName` is shared by file
collision checks and future injection. The decoder refuses two services
with the same derived name before returning any usable file, as ADR 0066
already requires; plain-script names do not participate. Runtime injection
still follows in the package, without a product caller in this step.

Windows execution guard preparation also exercises real Unix symlink
resolution under the explicit Windows naming policy: an approved `.exe`
alias resolving to a mixed-case batch file is refused before any spawn.
This portable filesystem model supplements the Windows-compiled refusal
test and does not certify Windows native path resolution or execution.

Allocator selection preparation reuses the approved `ScriptEntry` owner,
`ServicePorts`, helper primitive and Tokio's existing socket API. Explicit
ports precede the block; helper precedes range; absent choices bind a kernel
ephemeral IPv4 loopback port and release the probe socket. Range traversal
uses a caller-supplied seed for a deterministic starting offset, skips
reserved/busy candidates, and visits each endpoint exactly once. The plan
owner remains responsible for global dynamic reservations and bounded
retries; this primitive does not claim to reserve a socket or a plan.
Existing explicit declarations are checked only against the same place,
matching Paseo; dynamic choices also exclude other dynamic owners. No
new dependency, wire field, protocol number or product caller is added.

Quality B integration: merge the assigned node head `29eebb8b` after the
primitive checks. The exported method table now owns pending protocol rows
and method shape; no parallel method list or feature number is added here.
The node head's existing protocol, schema, method entries and tests are
preserved. No conflict resolution was required. The per-package whole gate
will cover this join after runtime wiring; narrow joined checks come first.

## Runtime preparation on the designated node base

The declaration/parser/allocation approval data are now in the shared node
head. The preparation branch merges ba45a6e3 without changing its protocol,
store schema, method rows, boot setup or native sources. Helper and port
selection modules remain test-only: no service runtime caller, proxy or
health implementation is claimed by this merge. New method rows, when the
runtime is ready, use the shared table and its unassigned feature gate.
The composed preparation still needs its focused checks before new behavior.

The designated node head now includes the qualified carrier and the common
application/node base. Its complete tree is retained; the earlier preparation
merge exclusions do not apply to this head. The runtime preparation above
remains test-only and does not qualify allocator, proxy or health execution.
Archive and editor targets belong to the registry package (ADR 0082); this
track has handed over no implementation for either. Focused composition checks
remain owed before further runtime wiring.


## Health clock preparation

Reference reopened: `server/script-health-monitor.ts:33–52,89–125,144–160,
209–226`. The existing actor, scanner and observer do not own TCP health.
Add the focused test-only observation clock defined in ADR 0066, using the
reference first-observation grace and consecutive-failure transitions. No
networking or route authority is introduced. The future route owner must
cancel pending probes and replace this clock on actual run replacement;
Paseo's retained-hostname behavior is deliberately stricter here as stated
in the ADR. Scheduler, real probe, eligibility, projection and consumer
integration remain owed. Resume: contract committed before clock code;
focused checks and the runtime package gate are still pending.


Clock source preparation now exists under the outer test-only module. Its
lazy callback cannot run before the observation grace; failed probes saturate
at the shared threshold, success clears the streak and unchanged health
returns no publication request. Synthetic-time cases cover the exact grace
boundary, recovery/reset, prolonged failure, dropped pending probes and a
replacement clock. These tests have been authored but not yet executed.
The callback carries no endpoint or authority. Resume: narrow node checks
are owed; product route ownership, probe cancellation/revalidation and the
full runtime package remain unimplemented.


## Service proxy publication contract

ADR 0066 now separates the default loopback proxy from per-service Admin
opt-in to anonymous public sharing. The typed `services` node setting will
reuse settings persistence and needs a clear per-service warning before
mutation; wildcard node listen and declared/running ports grant nothing.
Paseo's anonymous registered routes and the deliberate security difference
are stated. The browser Preview consumer is the exact attested backend
origin, not this proxy or node control. No setting/listener/URL API is wired
by this contract. Checked canonical registry incarnation/start/restart and
fresh process/socket/scheme proof are prerequisites; the current registry
source has no incarnation guarantee across physical restoration, so no
local generation or cwd heuristic substitutes for it. Resume: source
contract recorded before front-door code; focused checks/review are owed.


## Canonical helper workspace input

The designated head now exports the reviewed WorkspaceId value. The staged
helper and allocator will require that identifier explicitly, with
VESSEL_WORKSPACE_ID matching the literal workspace argument. The former
project/worktree fallback was staging-only and is withdrawn before any
caller. This does not adopt the unreviewed registry or establish permission:
its checked start/restart/root boundary remains a caller prerequisite.
Resume: contract first, then input plumbing and same-checkout/different-id
regressions; narrow checks owed.


The staged invocation and allocation request now require the explicit
WorkspaceId value. Literal argv and VESSEL_WORKSPACE_ID match for local and
worktree placements; an additional same-checkout/different-workspace test
would catch the removed legacy fallback. No registry or process admission
is inferred from this value. Helper and allocator remain test-only. Resume:
source committed, joined focused checks still owed; earlier remote queued
attempts were cancelled by build-machine maintenance before any check ran.


## Current shared node base

The preparation branch now merges the designated node head `ede20014`.
Its composer correction, plugin values, AgentEvent protocol accessor and
node service-file observations are retained unchanged. Protocol and store
schema stay the head's; no new method, feature assignment or dependency is
introduced here. The staged helper, allocator and health source is byte
identical to the previously narrow-tested preparation. The parser command
bound is unchanged while its shared lower-layer successor is reviewed.
This merge has no conflict resolution. The final service-runtime package
still owes its canonical ownership producers, actual callers and whole gate.

### Preview matcher/context implementation

The A10 preview module now carries exact literal-loopback origin matchers
and canonical connection observers without the guard. This does not grant
network access; no lease, dial or positive proof source is wired. The next
commit adds the closed unavailable producer and owned connection lifecycle.

### Preview lease and connection preparation

The staged node service now reserves node/per-lease/physical-work capacity
before dialing, registers attempts under the same gate as close/drain,
and retains a connected socket through blocking inspection and late-result
refusal. The owned I/O wrapper has no raw-socket extraction and checks
revocation/freshness on every payload poll. Tests model canonical authority
and accepted-peer/scheme evidence; real mode refuses without dialing.
Budgets live beside the API in preview/budget.rs. The absent canonical
producers and final runtime/package gate remain explicit prerequisites.

### Safe staged worker diagnostics

Preview inspection and helper resolution now share a typed worker failure
that privately retains Tokio's original cause but excludes panic payloads
from every public formatter and source. Raw task failures in other areas
are outside this change. This changes no helper input, process ownership,
approval, cancellation or network admission policy; both callers stay A10.

### Next package: workspace service consumer contract

The Preview privacy correction is source accepted, with changed-head narrow
execution still owed. The next proposed contract is workspace-services-api.md:
typed workspace snapshots/control, cached generated environment, health,
proxy metadata and dedicated Admin publication/settings. No DTO/method/SDK
source lands in this documents commit. Read never plans or runs a helper.
The private SavedNodeSettings envelope belongs to the settings owner; consume
its reviewed API, never implement another file model/writer. Canonical
incarnation is still missing, so public activation remains Unavailable.
After security review, values/staged rows/client wrappers precede handlers
and native consumer handover. Keep existing helper/Preview check receipts
separate from this package and run its enclosing gate before real wiring.


Consumer contract review correction: all eight proposed rows are explicitly
nonforwardable. The private Admin read/write boundary admits only direct
Owner/Admin Device, never Agent/Delegated even with forged/all scopes; the
workspace Read/Operate rows use the same direct rule with their exact scope.
The contract now owes no-history snapshot/cap/expiry tests and shared plus
node-wide waiting-read admission before runtime wiring. DTO/source remains
unimplemented until this security correction is accepted.

### Current dependency and values boundaries

The announced registry metadata head is merged without changing its assigned
protocol or schema. Its connection lifetime accessor and registry/start-lease
primitives are available, but canonical physical incarnation, seeded run and
accepted-peer/scheme proofs remain separate prerequisites. No real Preview
consumer is wired, so both shared accessor dead-code expectations remain.

The consumer contract correction is accepted. DTOs, staged methods and SDK
wrappers are authored in a separate values tree; its source checks and wire
generation do not qualify the helper/Preview stack here. The parser now uses
the incoming shared command and configuration byte-budget owners. Existing
immutable narrow checks keep their original commits and evidence scopes;
the merged runtime preparation still owes its own enclosing package gate.

The later announced lifecycle/file/Skills head is also merged. Its session
selection, migration and method contracts remain unchanged, and no local
runtime module is activated. The corrected deadline fixture is still checked
on its earlier immutable commit; that receipt will not qualify this new
composition. Service value/SDK work remains in the separate values tree.
