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

## Current values-package integration boundary

The staged service values and SDK are now based on the announced node gate-61
head. Incoming method rows, projections and protocol/schema facts take
precedence; the eight service rows follow the incoming usage rows. The
forwarding-policy extraction keeps the head's complete decisions in its child
module and adds only the eight explicit service refusals. No service runtime
or private settings writer is activated.

The previous immutable revision passed four-crate strict Clippy, 30 selected
proto/client tests and three web admission tests. Its five TS generation tests
passed, but freshness failed on the uncommitted generated service declarations.
That old snapshot is retained as evidence and will not overwrite the incoming
wire. The merged composition requires its own official regeneration, followed
by committed generated output, freshness and the enclosing package gate.
