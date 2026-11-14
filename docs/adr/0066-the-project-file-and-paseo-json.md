# ADR 0066: The project file's keys, and not reading `paseo.json`

Status: accepted, 2026-11-14 (milestone 12, the project-file track).
Amends ADR 0027's file and ADR 0031's digest. No crate edge changes.
Nothing on the wire changes in this decision; the fields it names land
with the code, at provisional protocol 21 until the integrator assigns
the final number.

## Context

ADR 0027 made `vessel.toml` the project's file: `[worktree] setup` and
`teardown`, a string or a list, approved as one digest before anything
runs. ADR 0031 added `[scripts.<name>]` and `[services.<name>]`
(`command`, optional `dir`). A file that names neither keeps digest
version 1; a file that names one is version 2. Every other key is
ignored and is not part of the digest.

Paseo (0.15.4) keeps the same facts in `paseo.json` at the repository
root. The schema
(`packages/protocol/src/paseo-config-schema.ts`) is `worktree`,
`scripts` and `metadataGeneration`, and it keeps every other key. The
running code, not the schema's passthrough, decides what is a command.

A project file is untrusted. Anyone who can commit, including an agent
at work in the tree, can name a program. Vessel runs a command only
after a person approved that exact set (ADR 0027). The question this
decision answers is which keys mean the same thing, which are only
renamed, and which Vessel does not have yet, and whether a `paseo.json`
found beside `vessel.toml` is itself a file Vessel executes.

## Decision

[Decision Log]

- Purpose and intent: one table from each key of `paseo.json` to
  `vessel.toml`, and one rule for a second file. Later slices add the
  missing keys without opening a command the approval did not cover.
- Existing implementation and constraints: `vessel-workspace`'s
  `project_file` parses `vessel.toml` (at most 64 KiB, 32 commands a
  list, 32 scripts and services together). `ProjectCommand` refuses
  empty text, text over 4 KiB and NUL. The digest is SHA-256. Version 1
  is the prefix `vessel.toml worktree commands 1\n`, then `setup` and
  `teardown` as a raw name, a little-endian count, and each command
  length-prefixed. Version 2, only when the file names a script or a
  service, is the prefix `vessel.toml commands 2\n`, the same two lists,
  then the scripts in name order (name, table, command, folder). An
  approval of version 1 still matches a file with no scripts. Unknown
  keys parse and do not change the digest
  (`project_file.rs`, the test `a key that does not run is not
  approved`). `CommandSet` on the wire is setup, teardown and scripts
  (`vessel-proto`'s `workspace/approval.rs`). Services restart on
  failure; the node reports listening ports of processes it started and
  assigns none (ADR 0031). Paseo's file is a different name, a different
  limit (256 KiB, `paseo-config-file.ts`) and a parser that skips bad
  entries instead of refusing the file.
- Alternatives considered:
  1. Execute `paseo.json` as a second project file, under the same
     approval.
  2. Execute it only when `vessel.toml` is absent.
  3. Read both and let `vessel.toml` win on a conflicting key.
  4. Do not read `paseo.json`. Translate its meaning into `vessel.toml`
     keys. A person who wants those commands writes them in
     `vessel.toml`, which is then the set the approval shows.
- The choice: alternative 4. The key table below is the contract for the
  keys Vessel still lacks. A key that can start a process enters the
  digest in the same change that starts running it, as version 3, and
  only a file that names such a key leaves versions 1 and 2.
- Why this and not the others: alternatives 1 to 3 each give an agent a
  second place to name a program. "Only when `vessel.toml` is absent" is
  the common case of a Paseo repository, so it is "always, until someone
  writes the other file". Merging two files means the approval must
  show every command either file can name, including a key Vessel would
  ignore and a key it would reinterpret (`type` versus a table). Paseo
  drops a terminal that is not an object, a terminal whose command is
  not a string, an empty command, and a script whose command is not a
  string (`worktree.ts`, `getWorktreeTerminalSpecs` and
  `getScriptConfigs`). Vessel's rule is the opposite: a file that does
  not parse runs nothing, and a refusal names what was wrong. Importing
  the drop would approve a set the file does not actually run, or run a
  command the page did not show. One file, one digest, one approval.
- Advantages: an approval given today still covers the file it was shown,
  because version 1 and version 2 stay byte for byte until a new
  command-bearing key is present. A repository already carrying
  `paseo.json` does not start running it because Vessel was pointed at
  the folder. The missing behaviour has a named place to land.
- Costs and effects: a person moving a project across copies the
  commands into `vessel.toml` by hand. Vessel will not do that copy.
  Keys listed as missing do nothing until the slice that adds them, and
  that slice invalidates the approval of any file that already contains
  them (they are ignored today, so an old approval must not start
  covering them). `metadataGeneration` is not a command and is not this
  decision's work. Auto-archive and opening an external editor are not
  keys of `paseo.json`; they are not given a `vessel.toml` key here.

### The table

Same meaning: the behaviour already matches, under the differences ADR
0027 and ADR 0031 already state (TOML, the 64 KiB bound, empty commands
refused rather than dropped, `VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE`
and `VESSEL_BRANCH` rather than Paseo's `PASEO_*` names). Renamed: the
fact exists under another name. Missing: not parsed as a command yet.
Ignored: kept by Paseo, not a command, not approved, not implemented.

| `paseo.json` | What Paseo does | `vessel.toml` | Class |
| --- | --- | --- | --- |
| `worktree.setup` | A string or a list of strings. Blank entries are dropped. Each remaining string is one shell command, in order, in the worktree; the first failure stops the rest. Read from the worktree's file on disk (`runWorktreeSetupCommands`). A separate check compares setup with `HEAD` and only warns. | `[worktree] setup`, already | Same meaning. Vessel reads the blob at the commit a worktree is created from, and the worktree's file when setup is run again (ADR 0027), which is stricter than the working tree. |
| `worktree.teardown` | The same shape, run before the worktree directory is removed. | `[worktree] teardown`, already | Same meaning |
| `worktree.terminals` | A list of `{ name?, command }`. Opened only after setup has succeeded (`worktree-session.ts` runs setup, then `runWorktreeAutoTerminals`). Each is an interactive shell; the command is written to it as input plus a carriage return, after the first output or 1.5 s. A bad entry is skipped. One that fails to open is logged and does not undo the worktree. | `[[worktree.terminals]]` with `command` and optional `name`. Not parsed yet. | Missing. Contract below. |
| `worktree.servicePorts.range` | Inclusive `"start-end"`, each end from 1 to 65535, start not above end. Used when the service names no port of its own. A project block replaces the daemon-wide `worktrees.servicePorts`; there is no merge. | `[worktree.service_ports] range`. Not parsed yet. | Missing, and renamed (`service_ports`). |
| `worktree.servicePorts.portScript` | A path of an executable, never a shell line. Run with four arguments (script name, workspace id, branch or empty, worktree path) and the same values in the environment. 10 s, at most 1024 bytes of output, which must be one TCP port. It wins over `range` in the same block. A port it returns may already be in use; a port already reserved for another service of this workspace is refused. | `[worktree.service_ports] port_script`. Not parsed yet. | Missing, and renamed (`port_script`). It stays an executable path, not a shell string. |
| `scripts.<name>.command` | A string. A command that is not a string, or blank, drops that script. | `[scripts.<name>] command` or `[services.<name>] command`, already | Same meaning for a string. A non-string is a syntax error of the whole file (TOML), not a skipped script. |
| `scripts.<name>.type` | The string `service` marks a supervised long-running process. Anything else, including a missing type, is a one-shot script. | The table: `[scripts.<name>]` or `[services.<name>]` (ADR 0031) | Renamed. The kind is where the name is written, not a field. |
| `scripts.<name>.port` | A finite number, kept only when the type is `service`. Otherwise ignored. Wins over the allocation block. | `port` on `[services.<name>]` only. Not parsed yet. | Missing. A `port` written on a script, or outside 1 to 65535, refuses the file. Paseo ignores it; a person who wrote it meant it. |
| `scripts.<name>` other keys | Kept (`passthrough`) and not read by the runner. | Unknown keys inside an entry stay ignored, and stay out of the digest, while they start nothing. | Ignored. `dir` is Vessel's own (ADR 0031); Paseo has no `dir`. |
| `metadataGeneration.title.instructions` | Replaces the default wording of a generated workspace title. | None | Ignored. Not a command. Not this track (`paseo.metadata`). |
| `metadataGeneration.branchName.instructions` | The same, for a generated branch name. | None | Ignored, as above |
| `metadataGeneration.commitMessage.instructions` | The same, for a generated commit message. | None | Ignored, as above |
| `metadataGeneration.pullRequest.instructions` | The same, for a generated pull-request title and body. | None | Ignored, as above |
| `metadataGeneration` other keys, including a legacy `agentTitle` | Kept so old files still parse. The instructions above are the ones the generator reads. | None | Ignored |
| Any other top-level key | Kept. | Ignored, not approved | Ignored |

Not keys of `paseo.json`, so they are not rows and they get no
`vessel.toml` key from this decision:

- `worktrees.root` and `worktrees.servicePorts` live in the daemon's
  `config.json`. Vessel's worktree directory stays the one ADR 0027
  chose. There is no node-wide port range: a service with no `port` and
  no project block asks the operating system for an ephemeral port,
  which is Paseo's behaviour when neither block is set.
- `daemon.autoArchiveAfterMerge` (default false, `config.ts`) is a
  daemon switch. It is a later slice, not a project-file key.
- Opening a path in an external editor is a desktop target list, not a
  project file.
- `.worktreeinclude` is a different file. Not this decision.

### The contract for the missing keys

These keys are specified here and not implemented here. Until the code
lands, a file that contains them still parses, the keys are still
ignored, and the digest is still version 1 or 2. The implementation
that starts honouring them is the change that switches that file to
version 3.

```toml
[worktree]
setup = ["npm ci"]
teardown = "docker compose down"

[[worktree.terminals]]
name = "logs"                 # optional; a session title when present
command = "tail -f dev.log"   # one shell line, as setup's commands are

[worktree.service_ports]
range = "3000-4000"           # optional; canonical start-end
port_script = "tools/portmake" # optional; a path, never a shell line

[scripts.test]
command = "npm test"

[services.web]
command = "npm run dev -- --port $VESSEL_PORT"
port = 3000                   # optional; wins over the block
dir = "web"
```

Terminals. At most 32, the same cap as a command list. `command` is a
`ProjectCommand`. `name`, when present, is a `SessionTitle` (trimmed,
one line, at most 120 characters, no control character); a blank name is
the same as no name. A bad entry, or more than 32, refuses the file, so
nothing in it runs. They open only after setup has succeeded, in file
order, each as a shell terminal (it takes input). The command is sent as
one line of input ending in a carriage return, once the shell has
produced output or 1.5 s have passed. A failure to open one does not
remove the worktree; it is a typed notice on that worktree, not a log
line alone. They count toward the terminal caps ADR 0031 already sets
(64 a node, 16 a project folder); one that would pass a cap is the same
kind of notice, naming which terminal did not open. Services are not
started by creating a worktree. Only the listed terminals are.

Ports. Allocation for one service, in this order: its own `port` if it
has one; otherwise `port_script` if the block has one; otherwise `range`
if the block has one; otherwise an ephemeral port. `port_script` is not
passed to a shell. A relative path is a `RelPath` and is joined to the
root the way `dir` is. An absolute path is a newtype: absolute, no NUL,
at most 4096 bytes, no empty, `.` or `..` component, and that text is
what the approval shows and what is executed (no symlink resolution
before the digest, so the digest does not depend on the machine). The
arguments are fixed: the service's name, the place's id, the branch name
or empty, and the absolute folder. The environment names the same four
as `VESSEL_SCRIPT_NAME`, `VESSEL_PLACE_ID`, `VESSEL_BRANCH` and
`VESSEL_WORKTREE`. Ten seconds, and at most 1024 bytes of standard
output, which must be one integer from 1 to 65535. A port already
reserved for another service of this place is a typed error. A port the
script returns is not checked for being free first (Paseo trusts the
allocator that far, and a probe would race the bind). A `range` is
inclusive. The scan starts at an offset supplied by the caller so a test
does not depend on chance, skips reserved ports, and skips a port that
does not accept a loopback bind. Both fields are digested when the block
is present, even though `port_script` wins at run time: editing the
unused range is still a change to the set a person approved.

The service's environment, when it is started, gains `VESSEL_PORT` for
its own port and `VESSEL_SERVICE_<NAME>_PORT` for each service in the
file, with the name in capitals and every character that is not a letter
or a digit made `_`. It does not gain a proxy URL. ADR 0031 left
forwarding to the preview track; a hostname and a reverse proxy are not
a key of the project file. There is no `VESSEL_WORKTREE_PORT`. Paseo's
per-worktree legacy port is not a key of `paseo.json`.

Health. For a service that was given a port, a TCP connect to loopback
on that port (IPv4 and IPv6, as Paseo's local connect does), not an HTTP
request. The state is an enum, `pending`, `healthy` or `unhealthy`, not
a string. It stays `pending` for 5 s after the process has been spawned.
Then a probe every 3 s, each probe at most 500 ms. Two consecutive
failures make it `unhealthy`; a success makes it `healthy` and clears
the count. The field sits beside `ServiceState` on the terminal's info,
because `ServiceState` is the process (starting, running, backing off)
and health is whether the port accepts a connection. Absent means this
peer did not report health. Absent is not `unhealthy`.

Digest version 3. Used when the file names any terminal, any
`service_ports` block, or any service `port`. Otherwise version 1 or 2
is unchanged, including the prefix bytes. Version 3 is the prefix
`vessel.toml commands 3\n`, then setup and teardown as version 1 writes
them, then the scripts as version 2 writes them plus one more
length-prefixed field for each (the port in decimal with no leading
zeros, or an empty field when there is none), then the terminals in file
order (count, then each name or empty, then its command), then the port
block (the canonical `start-end` or empty, then the `port_script` text
or empty). A file with scripts and none of the new keys stays version 2,
so an approval of it still matches. A file whose only addition is a
terminal is version 3 even when it has no scripts, so an approval given
while that key was ignored does not cover the command it starts.

Wire, when the code lands. Additive fields, with a default of absent:
`CommandSet` gains the terminals and the port block; `ScriptEntry` gains
an optional port; terminal info gains an optional health. No new call.
Provisional protocol 21. A peer older than that version is shown a
version 3 set as invalid, as ADR 0031 shows a script-bearing set to a
protocol 10 peer, so it cannot approve a digest of commands it cannot
display. A newer peer reading an older node treats the missing fields as
absent. An older peer reading a newer node ignores the unknown fields
and still sees setup, teardown and scripts; it must not be the peer that
records the approval of a version 3 digest.

`metadataGeneration` stays out of this digest. It is not a command. A
later track that sends those strings to a model does not inherit this
approval; that send is its own decision.

## Consequences

- `vessel.toml` remains the only project file Vessel executes.
- Versions 1 and 2 of the digest stay valid for every file that does not
  name a terminal, a port block or a service port.
- The slices that add terminals, ports and health follow the contract
  above, in the same change as the tests of the digest and of a refusal.
- No dependency is added. No crate edge changes. `docs/ARCHITECTURE.md`
  is unchanged until an edge does.
- Not done here: parsing the new keys, allocating a port, probing
  health, archiving a workspace after a merge, and opening an editor.
