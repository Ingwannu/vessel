# Project configuration, as Paseo's

Status: slice 1 decided, not implemented past the decision. ADR 0066.

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

## What Vessel already has

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
   digest, provisional protocol 21). No code.
2. **Next.** Parse the missing keys, put them in the version 3 digest,
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

Out of this plan: `metadataGeneration`, `.worktreeinclude`, a node-wide
port range, a service proxy, and a streaming setup panel. Each is named
in ADR 0066 as not a key this track adds.

## Resume point

- Done: ADR 0066 and this plan. No code. Digest versions 1 and 2 are
  unchanged. `paseo.json` is not read.
- Half-done: nothing. The contract is written and not parsed.
- Next: slice 2, in `project_file` and the node that runs setup and
  starts a service. Do not start slice 3 or 4 in that change. Do not
  bump a protocol number until the fields exist; 21 is provisional.
- The slice report names the commit the gate was run on. This file does
  not copy that log.
