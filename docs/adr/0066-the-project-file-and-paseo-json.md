# ADR 0066: The project file's keys, and not reading `paseo.json`

Status: proposed, 2026-11-14 (milestone 12, the project-file track).
Amends ADR 0027's file and ADR 0031's digest. No crate edge changes.
Nothing on the wire changes in this decision. When the fields it names
land, the code gates them on one constant of this track and leaves
`PROTOCOL_VERSION` at the base. The protocol number is given at merge.
This decision does not name one, and it does not use a sibling's.

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
  digest before any execution path honors it, as version 3; the staged
  parser/approval change may precede execution. Only a file that names
  such a key leaves versions 1 and 2.
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

### Implementation stages

The initial change was too large to review with its runtime as one slice.
The parser and approval contract therefore land first, with execution still
absent and the protocol assignment still `None`. This extends the timing
of the original digest decision: a key enters version 3 **before** any
execution path can honor it, rather than requiring parser and execution in
one commit. An earlier approval can become unusable during this staged
period; no newly parsed command runs. The alternative was one large change
covering parsing, approval, process startup, ports and health. Small staged
changes keep each rule reviewable while the unassigned protocol refuses a
set clients cannot yet fully show. The final encoding is unchanged. Runtime contracts are refined below
before their corresponding execution paths are connected.

### The table

Same meaning: the behaviour already matches, under the differences ADR
0027 and ADR 0031 already state (TOML, the 64 KiB bound, empty commands
refused rather than dropped, `VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE`
and `VESSEL_BRANCH` rather than Paseo's `PASEO_*` names). Renamed: the
fact exists under another name. Missing: runtime parity is not implemented. The terminal decoder and approval
contract are staged in slice 2a; service ports are also parsed into approved
snapshots. The allocation block is also parsed into approval data; no new
runtime is connected.
Ignored: kept by Paseo, not a command, not approved, not implemented.

| `paseo.json` | What Paseo does | `vessel.toml` | Class |
| --- | --- | --- | --- |
| `worktree.setup` | A string or a list of strings. Blank entries are dropped. Each remaining string is one shell command, in order, in the worktree; the first failure stops the rest. Read from the worktree's file on disk (`runWorktreeSetupCommands`). A separate check compares setup with `HEAD` and only warns. | `[worktree] setup`, already | Same meaning. Vessel reads the blob at the commit a worktree is created from, and the worktree's file when setup is run again (ADR 0027), which is stricter than the working tree. |
| `worktree.teardown` | The same shape, run before the worktree directory is removed. | `[worktree] teardown`, already | Same meaning |
| `worktree.terminals` | A list of `{ name?, command }`. Opened only after setup has succeeded (`worktree-session.ts` runs setup, then `runWorktreeAutoTerminals`). Each is an interactive shell; the command is written to it as input plus a carriage return, after the first output or 1.5 s. A bad entry is skipped. One that fails to open is logged and does not undo the worktree. | `[[worktree.terminals]]` with `command` and optional `name`. Parsing and approval are staged in slice 2a. | Runtime missing: automatic opening follows in another slice. Contract below. |
| `worktree.servicePorts.range` | Inclusive `"start-end"`, each end from 1 to 65535, start not above end. Used when the service names no port of its own. A project block replaces the daemon-wide `worktrees.servicePorts`; there is no merge. | `[worktree.service_ports] range`. Parsed into the approved allocation block. | Missing, and renamed (`service_ports`). |
| `worktree.servicePorts.portScript` | A path of an executable, never a shell line. Run with four arguments (script name, workspace id, branch or empty, worktree path) and the same values in the environment. 10 s, at most 1024 bytes of output, which must be one TCP port. It wins over `range` in the same block. A port it returns may already be in use; a port already reserved for another service of this workspace is refused. | `[worktree.service_ports] port_script`. Parsed into the approved allocation block. | Missing, and renamed (`port_script`). It stays an executable path, not a shell string. |
| `scripts.<name>.command` | A string. A command that is not a string, or blank, drops that script. | `[scripts.<name>] command` or `[services.<name>] command`, already | Same command role. Paseo trims the command; Vessel retains its existing script text to preserve version 2 approvals. A non-string is a syntax error of the whole file, not a skipped script. |
| `scripts.<name>.type` | The string `service` marks a supervised long-running process. Anything else, including a missing type, is a one-shot script. | The table: `[scripts.<name>]` or `[services.<name>]` (ADR 0031) | Renamed. The kind is where the name is written, not a field. |
| `scripts.<name>.port` | A finite number, kept only when the type is `service`. Otherwise ignored. Wins over the allocation block. | `port` on `[services.<name>]` only. Parsed and covered by the version-three approval; allocation is not connected. | Missing. A `port` written on a script, or outside 1 to 65535, refuses the file. Paseo ignores a plain script's port but keeps any finite service port, including fractions and out-of-range values (`worktree.ts:348–354`; `workspace-service-port-registry.ts:168–190`). Vessel refuses these declarations before work: a person who wrote the port meant it, and an invalid port cannot be executed as written. |
| `scripts.<name>` other keys | Kept (`passthrough`) and not read by the runner. | Unknown keys inside an entry stay ignored, and stay out of the digest, while they start nothing. | Ignored. `dir` is Vessel's own (ADR 0031); Paseo has no `dir`. |
| `metadataGeneration.title.instructions` | Replaces the default wording of a generated workspace title (`paseo-config-schema.ts` 48-70; `public-docs/metadata-generation.md`, the per-project example). | `[metadata.title] instructions` | ADR 0062, added at merge. Not a command, so outside this digest. This track does not parse it. |
| `metadataGeneration.branchName.instructions` | The same, for a generated branch name. | `[metadata.branch] instructions` | ADR 0062, added at merge, as above |
| `metadataGeneration.commitMessage.instructions` | The same, for a generated commit message. | `[metadata.commit] instructions` | ADR 0062, added at merge, as above |
| `metadataGeneration.pullRequest.instructions` | The same, for a generated pull-request title and body. | `[metadata.pull] instructions` | ADR 0062, added at merge, as above |
| `metadataGeneration` other keys, including a legacy `agentTitle` | The schema keeps them (`paseo-config-schema.ts` 48-58, the `agentTitle` comment). | Not named in the part of ADR 0062 read (its settings section) | Not a command, and outside this digest. The rest of ADR 0062 was not read for further keys. |
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

The contract below spans several implementation slices. Slice 2a parses
terminal declarations and covers them in version 3 approval snapshots;
it does not open them. Port keys remain ignored until their parser lands.
A file without a parsed new key keeps its version 1 or 2 digest. Every
slice that starts honouring another key includes it in version 3 before
that key can cause execution. The unassigned protocol gate refuses
terminal-bearing approval snapshots on negotiated connections meanwhile.

```toml
[worktree]
setup = ["npm ci"]
teardown = "docker compose down"

[[worktree.terminals]]
name = "logs"                 # optional; TerminalInfo.title when present
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
the same as no name. Paseo trims a name and has no length bound
(`utils/worktree.ts`): a name that is not a non-empty string is dropped
and the terminal still opens, and a bad command is skipped rather than
refusing the file. Vessel refuses the whole file for a name that is not
a title, for a bad command, or for more than 32, so nothing in it runs.
The bound is a stated difference: a title is one line on a card, and
`SessionTitle` is already that bound. The name does not land on a
session. ADR 0031: an opened shell is a terminal, not a session, and it
has no `SessionInfo`. The name is `TerminalInfo.title`
(`Option<SessionTitle>` on `crates/vessel-proto/src/terminals.rs`).

They open only after setup has succeeded. Paseo opens them concurrently
(`Promise.all` in `runWorktreeAutoTerminals`). Vessel opens them one
after another, in file order. Stated difference: the cap notice has to
name which terminal did not open, and two opens racing the 16-per-folder
cap would make that notice depend on which finished first. Each is a
shell terminal (it takes input). The command is sent as one line of
input ending in a carriage return, once the shell has produced output or
1.5 s have passed. A failure to open one does not remove the worktree;
it is a typed notice on that worktree, not a log line alone. They count
toward the terminal caps ADR 0031 already sets (64 a node, 16 a project
folder); one that would pass a cap is the same kind of notice, naming
which terminal did not open. Services are not started by creating a
worktree. Only the listed terminals are.

The value layer lands before the port keys enter the parser. A declared or
allocated `ServicePort` is a nonzero 16-bit number with private storage;
construction and deserialization enforce the same bound. `PortRange`
keeps two such endpoints private and refuses reverse order through both
its constructor and text decoder. The range decoder follows the schema's
whole-string JavaScript trim and one to five ASCII digits per endpoint;
it never trims an endpoint separately. Its serialized approval text is
canonical decimal `start-end`. Standard-library integer types and the
existing serde conversion pattern suffice; no dependency is added.
These types start no process and leave existing file digests unchanged
until the parser and approval fields use them.

Ports. Allocation for one service, in this order: its own `port` if it
has one; otherwise `port_script` if the block has one; otherwise `range`
if the block has one; otherwise an ephemeral port. `port_script` is not
passed to a shell. A relative path is a `RelPath` and is joined to the
root the way `dir` is. An absolute path is a newtype: absolute, no NUL or
other control character,
at most 4096 bytes, no empty, `.` or `..` component, and that text is
what the approval shows and what is executed (no symlink resolution
before the digest, so the digest does not depend on the machine). The
arguments are fixed: the service's name, the place's id, the branch name
or empty, and the absolute folder. The environment names the same four
as `VESSEL_SCRIPT_NAME`, `VESSEL_PLACE_ID`, `VESSEL_BRANCH` and
`VESSEL_WORKTREE`. Ten seconds, and at most 1024 bytes in each of standard output and
standard error, matching the allocator's `execFile` `maxBuffer`. Standard
output is decoded as UTF-8, trimmed with the shared JavaScript rule, and
parsed through `ServicePort`; standard error is bounded and discarded,
never included in diagnostics. A port already
reserved for another service of this place is a typed error. A port the
script returns is not checked for being free first (Paseo trusts the
allocator that far, and a probe would race the bind). A `range` is
inclusive. The scan starts at an offset supplied by the caller so a test
does not depend on chance, skips reserved ports, and skips a port that
does not accept a loopback bind. Both fields are digested when the block
is present, even though `port_script` wins at run time: editing the
unused range is still a change to the set a person approved.

#### Absolute executable paths across clients

- Purpose and intent: a client must be able to read the approved path of a
  node on another operating system. Decoding a Windows node's approval on
  Unix must not turn a valid path into a transport error.
- Existing constraints: `RelPath` supplies the relative-path rules. An
  absolute path has the same byte bound (`RelPath::MAX_BYTES`), no NUL or
  other control character,
  and no empty, dot or parent component. These are lexical rules; neither
  parsing nor hashing may touch the filesystem.
- Alternatives considered: use the decoder host's `Path::is_absolute`;
  keep unrestricted text; or validate the raw text against both supported
  path grammars. The first is host dependent, and `Path::components`
  normalizes away forbidden empty and dot components. Unrestricted text
  would move validation into every execution path.
- Choice: `ExecPath` owns private text and records whether its lexical
  shape is valid for Unix, Windows, or both. Serialization uses that text
  unchanged. At invocation the node accepts its own grammar or returns a
  typed platform refusal before spawning; accepting a remote path for
  inspection does not make that path executable on the local machine.
- Windows grammar: drive plus root, UNC server/share, and verbatim and
  device namespace roots follow the forms described by Rust's
  [path prefixes](https://doc.rust-lang.org/std/path/enum.Prefix.html).
  A root must be followed by a program component. Ordinary Windows paths
  use either separator; verbatim paths use backslashes. Unix uses `/`;
  a backslash there may be a literal filename character. Each grammar
  checks the original components, before normalization.
- Why this choice: the same decoder can carry a remote node's literal
  approval text without changing its meaning according to the client's
  host. Rust's [path documentation](https://doc.rust-lang.org/std/path/struct.Path.html#method.components)
  explains why a native component iterator cannot enforce this contract.
- Costs and effects: a well-formed foreign path can be inspected but is
  refused at invocation on an incompatible node. Filesystem existence,
  executable format and symlink resolution remain execution checks;
  approval still does not pin the file's bytes. An absolute Windows UNC
  path can reach a network share with the node user's credentials, just
  as an absolute local path can reach an executable outside the project.
  No new dependency or crate edge is needed.

Both path types refuse the same `char::is_control` set, including NUL,
line breaks, terminal escape controls, DEL and C1. NUL retains its own
diagnostic. This is an approval security floor: control text must not hide
or redraw what a person sees while the node later executes the full path.
Absolute paths deliberately do not inherit `RelPath::MAX_PART_BYTES`.
They can name a foreign filesystem whose component limits differ; native
component limits are checked at execution along with existence and format.
The shared overall byte cap still bounds inspection and transport. This
does not add a filesystem probe to either decoding or hashing.

#### Port-script selection and nonempty allocation values

- Purpose and intent: validate the program and the allocation block once,
  before either becomes part of an approval or execution plan.
- Existing constraints: `RelPath` deliberately admits the folder root,
  which is useful for file calls but cannot name a program. `ExecPath`
  carries a foreign absolute path without filesystem access. Paseo's
  `paseo-config-schema.ts:5–22` trims `portScript`, requires at least one
  of `range` and `portScript`, allows both, and refuses unknown block keys.
- Alternatives considered: public relative/absolute enum variants; an
  unchecked command string; or an opaque value with one decoder. Public
  variants let a caller insert `RelPath::root()`; unchecked text repeats
  validation at every consumer. Neither protects the constructor boundary.
- Choice: `PortScript` privately owns a non-root `RelPath` or an
  `ExecPath`. Its decoder applies JavaScript whole-string trimming, then
  validates the remaining literal path; serde uses that same decoder.
  A borrowed target view permits inspection but is not an execution input.
  `ServicePorts` privately owns its optional range and port script. Its
  constructor and deserializer both refuse an empty block; no domain
  default can construct one, and unknown block fields are refused.
- Why this choice: the existing path types and serde conversion pattern
  carry their invariants into the future parser. One project-text rule
  owns JavaScript trimming for the range, terminal entries and port script,
  so BOM and NEL handling cannot drift between these input edges. No
  dependency or crate edge is added.
- Costs and effects: these values now reach file parsing and wire replies
  through the optional approved allocation block. Their presence starts no
  process. Program existence, relative-link containment, native grammar
  compatibility and Windows command-script refusal are execution checks
  still owed; the value slice cannot claim they have run.

#### Service declarations before parser and approval integration

- Purpose and intent: a declared port belongs to a service, and every
  consumer of an approval must handle it. Deserialize-only checks are not
  enough if a caller can later change a service into a plain script while
  retaining its port.
- Existing constraints: `ScriptEntry` publicly exposes its kind, command,
  name and folder. Adding a public optional port would permit exactly
  that invalid mutation. Its existing JSON kind remains a string, and
  earlier approvals and clients must keep that shape.
- Alternatives considered: leave public fields and validate at every use;
  replace the kind with a flattened tagged execution enum; or privately
  own the kind and port and expose a complete borrowed display view. The
  first repeats a safety check and permits invalid domain values. The
  tagged enum is sound, but changes the existing kind's Rust and generated
  TypeScript API beyond this additive field.
- Choice: privately own `ScriptEntry`'s kind and optional `ServicePort`.
  A validating constructor and the wire decoder both refuse a plain
  script with a port as `InvalidScriptEntry::PortOnScript`. No setter can
  change either half independently. The already validated name, command
  and folder need no new representation. JSON keeps its current kind
  string and gains only the optional port.
- Why this choice: validation is at construction, with the smallest change
  to the wire. A `ScriptEntryView` borrows every approved field for display;
  `view()` exhaustively destructures the owner without `..`, so another
  field breaks this projection too. Runtime and digest operations take
  the owner, never a caller-constructed display view.
- Costs and effects: native constructor/getter call sites need an API
  adaptation owned by the native lead. The native approval card must
  exhaustively name every view field, and web coverage/comparison must
  project every `ScriptEntry` field by type, as they already do for the
  listed set and `CommandSet`. They show the port/allocation block in full
  or refuse approval, including a port-only change and its removal from
  `last_approved`. This does not assign `PROJECT_FILE_KEYS`; the web dialog,
  web command-changes and native card must display all approved terminal
  and port data before it is assigned. The declaration boundary now implements
  the constructor, serde, digest slot, reply gates and client refusal before
  the file parser began producing the new field. The parser now reads it;
  allocation and execution still do not honor it. Native
  constructor/getter/card adaptation remains an integration prerequisite.

#### Allocation-block approval projection before file integration

- Purpose and intent: a future allocation field must not disappear behind a
  getter-only display. The approved range and program both matter, even when
  one takes precedence at execution.
- Existing constraints: `ServicePorts` already privately owns its nonempty
  choice. Its getters protect mutation, but adding another approved field
  would not force a display that calls only those getters to handle it.
  Paseo's `paseo-config-schema.ts:5–22` permits either or both choices and
  refuses unknown keys. Its allocator (`workspace-service-port-allocator.ts:25–43`)
  chooses the program before the range; this does not make the range text
  disappear from the approval.
- Alternatives considered: public owner fields (lose the construction
  invariant); a manually maintained display DTO (can omit a new field); or
  a complete borrowed projection, as for `ScriptEntry`. Choose the last:
  `ServicePortsView` borrows `port_script` and carries the optional validated
  `PortRange`. The owner's `view()` names every field without `..`, and
  each native or CLI display names every view field. Execution accepts the
  owner, never a caller-constructed display view. No dependency is needed.
- Choice at integration: add an absent-by-default allocation block to
  `ProjectFile`, `CommandSet` and `ProjectCommands::Listed`. The TOML edge
  reads `[worktree.service_ports]` through the existing strict validated
  decoder; there is still no `paseo.json` reader. Presence is retained even
  with no scripts or terminals, counts as a nonempty approval set, and
  selects version three. Its two already reserved digest slots cover the
  canonical range and the trimmed literal program path; earlier vectors
  without a block do not change. Parsing starts no allocator or helper.
- Why this choice: the CLI shows both choices, with the exact approved
  program path. The web's `Required<ServicePorts>` projection must name
  every nested field, in addition to its existing listed/set/entry guards.
  The implemented projection takes keys from `Required<ServicePorts>` and
  values from `ServicePorts[K]`: every key is required while an omitted
  non-null choice stays `undefined`, without an invented null or sentinel.
  Until full display lands, any current, last-approved or folder-comparison
  block yields `unsupported_ports`, never a partial comparison or an
  approval button. The native lead owns its new listed/set field patterns
  and exhaustive allocation view. Both old-peer and unassigned-peer reply
  gates cover the current and prior block on all approval reply paths.
- Costs and tests required: file tests for either/both, an empty or unknown
  block, trim and a block without services; digest changes for either
  choice and its removal; current/prior/folder web refusal and a new
  optional nested-field type probe; CLI complete display and all six reply
  gates. `PROJECT_FILE_KEYS` remains unassigned until the web dialog, web
  comparison and native card show all terminal, declaration and allocation
  data. The allocation integration implements these approval boundaries
  and tests. No allocator, helper invocation or health behavior is connected
  by carrying the block.

Relative program resolution is a deliberate difference from Paseo. Its
allocator passes the trimmed command to the plugin's `execFile` wrapper
(`workspace-service-port-allocator.ts:58–72`,
`packages/plugin/src/server/process.ts:76–100`); a bare command name uses
Node's [PATH lookup](https://github.com/nodejs/node/blob/v22.x/doc/api/child_process.md).
Vessel treats even a bare name as relative to the checkout root the way
`dir` is, without consulting PATH. For a worktree this is its own folder,
so it runs that worktree's copy of the program while approval covers the
project file's text. The allocator's cwd is this checkout root; `dir`
changes the named script's cwd only. Vessel also keeps `RelPath`'s portable forward-slash
grammar and refusals, including parent, backslash, control and repository
metadata components. These are the existing project containment floor:
approval must not silently select an executable elsewhere through PATH
or a relative path that escapes the root. The runtime test
`bare_port_script_is_resolved_under_the_approved_root` must put a different
fake program on PATH and prove that the checkout's program is selected,
including the worktree's copy when it differs from the project's copy.
Relative symlinks are resolved and checked by `Root::existing` at execution;
the digest continues to cover the approved text, not a machine's resolved
filename. These runtime tests remain unimplemented in the value slice.

The service's environment, when it is started, gains `VESSEL_PORT` for
its own port and `VESSEL_SERVICE_<NAME>_PORT` for every service in the
file, itself included. `<NAME>` is Paseo's `normalizeServiceEnvName`
(`workspace-service-env.ts`): Unicode uppercase as JavaScript's
`toUpperCase` does it (so U+00DF becomes `SS`, not an ASCII-only
fold), then each run of characters outside ASCII `A-Z` and `0-9`
becomes one `_`, then leading and trailing `_` are removed. A `ScriptName`
is already ASCII, so the uppercase step does not admit a character the
name type refuses; the collapse is what makes `web-app`, `web_app` and
`web.app` one variable, and `web-` the same variable as `web`. Two
services that normalise to one name refuse the file, naming both. Nothing
in the file runs. Paseo throws the same collision later, when a service
is started (`assertNoServiceEnvNameCollisions`); refusing the file is
earlier, and a silent overwrite is not allowed. The environment does not
gain a proxy URL. ADR 0031 left forwarding to the preview track; a
hostname and a reverse proxy are not a key of the project file. There is
no `VESSEL_WORKTREE_PORT`. Paseo's per-worktree legacy port is not a key
of `paseo.json`.

What an approval covers for `port_script` is the path, not the bytes at
that path. Paseo does not invoke it at every start. Its registry
(`workspace-service-port-registry.ts:55–69,160–217`) initially plans all
configured services, even services not started yet, and reuses entries by
name. `worktree-bootstrap.ts:412–453` refreshes only the selected service
when its existing runtime entry is stopped. A changed declaration on an
unstarted, already planned name can therefore retain its earlier port.
The helper primitive below owns one invocation only, not the cache or the
choice of when to invoke it. Before allocator wiring, its contract must
state cache identity, refresh and release, including this case. There is
no allocator or helper caller yet.

Whenever a helper is invoked, its bytes are read then. An agent that can
write in the tree can replace the program without changing the approved
path or digest. This is the same trust boundary ADR 0027 states for an
approved setup command invoking a repository script: approval covers the
text, not the script's bytes. A worktree invokes its own checkout's copy;
an absolute program may lie outside the checkout. No content pinning is
claimed. Existing Vessel supervision (ADR 0031) rechecks its captured
digest's approval on restart; approving another digest does not withdraw
the earlier one. Withdrawal is explicit.

#### One direct port-helper invocation

[Decision Log]

- Purpose and intent: execute exactly the approved program with four
  literal arguments, and accept one bounded TCP-port result. This primitive
  does not allocate a plan, select a service or authorize execution.
- Existing assets and constraints: `PortScript` already owns validated
  relative or foreign absolute text. `Root::existing` supplies checkout
  containment; `vessel-proc::spawn` supplies piped processes and group/job
  cleanup. Setup's shell/log pipeline has different limits and semantics;
  the private listener scanner truncates output and is not an input decoder.
- Alternatives considered: reuse either capture path, invoke a shell,
  add a subprocess dependency, or compose the existing process API with a
  small bounded reader. Choose the last: no new dependency or crate edge,
  no shell reparse, and overflow/read failures remain typed refusals.
- Choice: resolve paths on the blocking pool. A relative name, including a
  bare name, goes through `Root::existing` in the checkout supplied by
  `ScriptRoot`. An absolute program must match the host grammar and is
  canonicalized at invocation. Both original and resolved Windows names
  are checked before spawn: `.cmd`/`.bat` ignoring case and Win32 trailing
  dots/spaces are refused, as are alternate-stream names. This conservative
  refusal also applies in verbatim namespaces; it avoids relying on a
  normalization distinction to keep an interpreter out. Symlink races after
  this check remain the same-user race documented by `Root` (ADR 0027).
- Arguments and environment: service name, typed place id (worktree id
  when present, otherwise project id), branch or empty, and absolute
  checkout folder. The four `VESSEL_*` names above mirror those values.
  This place-id mapping is a stated difference: Vessel has no Paseo
  workspace id. Reuse `TerminalsConfig.removed`, without adding terminal
  color variables; the binary owns the credential-name inventory. Other
  inherited user variables and filesystem/network permissions remain those
  of the node user. The helper is not a sandbox.
- Failure policy: one named time budget covers path resolution, spawn,
  both pipe readers and child exit; cancellation before spawn starts no
  child. Each pipe has one named byte cap; read at most cap plus one byte
  to distinguish exact-cap EOF from overflow. Drain both concurrently with
  waiting, close stdin, and require a successful exit before decoding stdout.
  Timeout, cancellation, pipe failure and overflow drop the child owner,
  which kills the process group/job. Never log stdout/stderr or retain them
  in an error. Preserve spawn, join, path, I/O, UTF-8 and numeric causes.
- Costs and effects: canonicalization can reach a Windows network share
  with the node user's credentials. Cancelling a blocking resolution
  cannot abort that OS operation, but its result cannot spawn a child.
  This is stronger cleanup than Paseo's direct `execFile` timeout; no
  descendant is intentionally kept. Staged node code stays `cfg(test)`
  until the approval-checked allocator wiring removes it (A10). Its
  narrow checks prove the primitive only, not product execution.
- Required tests: literal metacharacter arguments and four matching env
  values, root selection with a PATH decoy and different worktree copy,
  relative symlink escape refusal, foreign grammar refusal, Windows
  command-script refusal before spawn (including normalized/resolved
  names), strict UTF-8/JS trim/port decoding, independent pipe caps,
  exact-cap acceptance, nonzero exit, timeout and cancellation cleanup.
  Unix fake scripts use the existing executable writer through the test
  sandbox. Actual Windows execution remains unverified on a Unix host.

What those processes can reach. A terminal opened from the file and a
`port_script` are processes of the node's user. A relative `port_script`
stays inside the project root; an absolute one is any executable path
the approval showed, so it can be anywhere that user can execute. Neither
is a shell line, so the file cannot name a pipeline. The terminal's
command is a shell line, typed into the shell ADR 0031 already starts,
so it can reach what that shell can reach. Neither receives a Vessel
credential or a forge token: the same variables ADR 0031's amendment
strips from setup, teardown, terminals, scripts and services
(`VESSEL_*TOKEN`, `VESSEL_*SECRET`, `VESSEL_AGENT_SESSION`,
`VESSEL_NODE_SOCKET`, `GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`,
`GITHUB_ENTERPRISE_TOKEN`). What refuses them: a file that does not
parse, including a port-script path the rules reject and two services
whose names collide, runs nothing; an unapproved digest runs nothing; a
port script that exceeds ten seconds, prints anything but one port, or
returns a reserved port does not start the service. A test supplies the
range scan's start offset and, when it needs a port script, a program
built the way the workspace's other test programs are. No fake answers
in a release build.

On Windows neither path uses a signal, systemd or launchd. `port_script`
names the executable itself, with no automatically inserted `cmd.exe`,
and receives the same four arguments,
placed in a job object so the timeout ends it and the children it
starts, as ADR 0031 already does for a script. The terminal is that
ADR's Windows shell (`pwsh.exe`, then `powershell.exe`, then
`cmd.exe`). A failure to start is a typed error, not a skipped entry.

This is a deliberate Windows difference from Paseo. Although the allocator
requests `shell: false`, `packages/plugin/src/server/process.ts:36–44`
overrides it: `shouldUseWindowsShell` returns `true` for a `.cmd` or `.bat`
command script, which then runs through `cmd.exe`. Vessel will instead
refuse those port scripts before spawning with
`PortScriptError::WindowsCommandScript`: "a port script on Windows must be
a native executable, not a .cmd or .bat command script". This security
floor avoids introducing a command-line interpreter that reparses the four
arguments, including a branch name chosen by a person or an agent. It
does not claim that an executable the user explicitly approves cannot
itself interpret input. The runtime slice must add
`windows_command_scripts_are_refused_before_spawn`, covering mixed-case
extensions, Win32-normalized names and resolved targets that identify a
command script, with a metacharacter-bearing branch and proof that no
child starts. The lexical path type alone neither performs this check nor
claims that test has run; actual Windows execution remains unverified.

Named-script whitespace is an existing difference retained here. Paseo's
`getScriptConfigs` trims `command` (`utils/worktree.ts:331-337`). Vessel's
existing `ProjectCommand` parser validates but retains those bytes, which
version 2 approvals already cover. Trimming existing scripts now would
invalidate those approvals without any new key. Terminal declarations are
new, so their decoder follows Paseo's trimming before the existing command
and title validators; this includes JavaScript's BOM/NEL distinction.

### Setup progress, and why it is not a slice

Paseo, after a worktree exists, emits `workspace_setup_progress` with
`running`, `completed` or `failed`, a detail snapshot of each command's
output as it arrives, and an error string when it fails
(`worktree-session.ts`). That is the streaming panel.

Vessel already has the outcome and the bytes, not that push.
`WorktreeInfo.setup` is `SetupState` (`none`, `skipped`, `running`,
`succeeded`, `failed`) on the worktree itself
(`crates/vessel-proto/src/workspace/worktree.rs`). The log is
`worktree.output` (protocol 10, `ReadSetupOutput` in
`crates/vessel-proto/src/rpc.rs`): a page of the worktree's log file,
and the call waits on the log's length, which the runner bumps on every
write and once when the run ends (`crates/vessel-node/src/workspace/setup.rs`).
A session's own log gets a `vessel:` notice with the result and, after a
failure, its last lines (ADR 0027). There is no per-command progress
event and no panel payload.

This track does not add one. The task's slices are the file's missing
keys, auto-archive, and editor targets. A push beside a log a client can
already follow would be a second way to read the same run. Stated
difference: no `workspace_setup_progress`. A client reads `setup` on the
worktree and pages `worktree.output`.

Health. For a service that was given a port, a TCP connect to loopback
on that port (IPv4 and IPv6, as Paseo's local connect does), not an HTTP
request. The state is an enum, `pending`, `healthy` or `unhealthy`, not
a string. Paseo's grace starts when the monitor first observes a route, not when
its process spawned (`script-health-monitor.ts:129–163`); one global
three-second tick probes only after the five-second grace. Vessel has no
service proxy route. Before health wiring, its contract must name the
corresponding observation and reset event rather than claim a spawn clock
is Paseo's. Each probe lasts at most 500 ms. Two consecutive
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
No new `node.settings` key: this track does not add one. The protocol
number is given at merge. The code gates the fields on one named
constant and leaves `PROTOCOL_VERSION` at the base. A peer older than
that constant is shown a version 3 set as invalid, as ADR 0031 shows a
script-bearing set to a protocol 10 peer, so it cannot approve a digest
of commands it cannot display. A newer peer reading an older node treats
the missing fields as absent. An older peer reading a newer node ignores
the unknown fields and still sees setup, teardown and scripts; it must
not be the peer that records the approval of a version 3 digest.

The repository's own clients must also cover every approved field. During
staging the web dialog refuses any terminal-bearing current or previous
set and offers no approval button; its comparison returns an explicit
unsupported outcome instead of a partial diff. Both boundaries enumerate
the generated types with all optional fields required, so a later field
addition fails type checking. Assignment remains blocked until the web
dialog displays terminals, web command-changes compares them, and the
native approval card displays them. The native lead owns that card's
exhaustive adapter and its stopgap refusal.

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
- Staged in slice 2a: terminal parsing and its approval snapshot. Not done:
  parsing port keys, opening terminals, allocating a port, probing health,
  archiving a workspace after a merge, and opening an editor.

### Process lifetime observations for service ownership

[Decision Log]

- Purpose and intent: a detected port must remain tied to the actual run
  that owns it. PID reuse and a rapid restart cannot let a stale observation
  authorize a replacement process or retain browser Preview access.
- Existing assets and constraints: Proc owns piped child groups/jobs; PTY
  owns its process and waiter thread. Session and terminal handles currently
  report only an atomic PID. The node scanner joins those numeric roots to
  sockets; neither a cached PID nor a declared port proves a current run.
  Paseo's `service-proxy.ts:421–754` routes by registered workspace/service
  and port; it does not expose this per-run observation to Preview.
- Alternatives: timestamps or a numeric generation; invent authority from
  a PID snapshot; or a read-only observation issued by the real runner.
  PID/time values can repeat and watch snapshots can coalesce. Choose the
  runner-issued observation. This is a security floor for the node-owned
  Preview boundary of ADR 0078, not a claim that Paseo implements it.
- Choice: `vessel-proc::ProcessId` is the one nonzero OS PID value type,
  with checked conversion from an integer. It carries no authority.
  `ProcessLifetime` privately shares the identity of one run and a watch
  of its revocation. Comparing two observations compares that opaque
  identity, never PID equality or an address exposed to a client. There is
  no serialization, wire field, numeric generation, new dependency or
  persistent process registry.
- Issuance and revocation: Proc Child and PtyProcess own the issuer guard.
  Observers can inspect the typed PID, compare runs and await revocation;
  they cannot revoke or mint an observation. The runner revokes before a
  signal attempt, on observed exit or uncertain wait, and on owner drop.
  A failed signal still revokes authority; it does not claim the OS process
  died. PTY's actual waiter also revokes, even if the async caller has not
  polled `wait`. A stopped/replaced run never revives its old observation.
- Transport within the process: AgentProcess delegates its child's
  observer without adding driver policy. SessionHandle and TerminalHandle
  carry observers obtained from those actual children. A move/restart
  replaces the observer; parking/ending clears it. Existing numeric PID
  APIs remain for compatibility and are not used to mint authority.
- Reuse decision: add focused `process_id` and `lifetime` modules to Proc,
  reusing standard nonzero storage, Arc and Tokio's existing watch channel.
  Do not put process facts, lock/FD inspection or session policy there.
  Self-process observations reuse the PID value; those observations
  cannot issue a lifetime. No crate dependency edge changes.
- Costs and effects: the extra shared state is one bounded watch per run,
  retained only while an owner or observer holds it. Revocation is a
  conservative authority event, not an OS liveness probe or proof that all
  descendants have exited. A Proc exit is observed when its waiter is
  polled; callers still need complete fresh socket/process inspection.
  A caller retains finite admission while any blocking resolution or
  unfinished launch exists. Same-user hostile processes remain within the
  trust boundary already stated for process groups and checkout paths.
- Required tests: zero PID refusal; cloned observer equality and distinct
  runs with the same PID; watchers before/after revocation without a lost
  wake; wait/signal/drop revocation on real fake children; PTY exit before
  async wait; session move/park and terminal restart replacing/clearing the
  actual observer. Tests use the existing fakes and event readiness, not
  real agents or sleep-based coordination. Windows execution remains
  unverified by a Unix run.

The lifetime alone grants no network permission. Browser Preview still
requires the exact WorkspaceId and opening SessionId, current complete
socket ownership, a verified canonical origin/scheme and revocation on
lifecycle changes. No wire candidate or client URL can construct this
observation. The service runtime and browser wiring are separate consumers;
this carrier adds no proxy or browser capability by itself.

An explicit cleanup-authority release is also a revocation boundary. When
a release-capable child is integrated, refusing release must retain the
still-owned observation; successful release must revoke it before transferring
the direct OS handle. A detached reaper's separate exit observation reports
the actual direct-child wait result. It must not reinterpret this carrier's
revocation as OS death or mint a replacement live carrier from a retained PID.
The joint release integration must test both cases: refused release keeps
the owned observation live, and successful release revokes while the child
continues until its independently observed exit. The assigned carrier base
has no release operation; those joint tests remain with that integration.
