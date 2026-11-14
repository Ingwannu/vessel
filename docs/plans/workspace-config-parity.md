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
  (`workspace-service-port-allocator.ts`). The script is executed, not a
  shell: four arguments, 10 s, 1024 bytes, one TCP port. A reserved port
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

- Done: slice 1's reviewed ADR and follow-up amendments.
- Implemented: slice 2a's terminal decoder, approval fields, digest vectors,
  compatibility and CLI rendering. The slice's validation results belong in
  its review report; no runtime behaviour is implied by parsing these keys.
- Half-done: automatic terminal execution is absent; port keys remain ignored
  as before. `paseo.json` is not read.
- Next: keep `PROJECT_FILE_KEYS` unassigned until the web approval dialog
  displays terminals, web `command-changes` compares them, and the native
  approval card displays them. The current web refusal is a deliberate
  stopgap; display/comparison lands together with the native card's work.
  The native lead owns its card and exhaustive adapter. This track owns
  the web refusal now and its later terminal display/comparison.
- Then: add the port contract in 2b, followed by runtime with approval and
  lifecycle tests. Returned review findings take priority.
- Still unverified: Windows, actual shells, and negotiated support after
  the integrator assigns the protocol number.
