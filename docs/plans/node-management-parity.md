# Plan: node management parity (ADR 0064)

How Vessel stops, restarts and reconfigures a node, against Paseo's daemon
commands. The decision is ADR 0064. This file is the evidence and the
slice order. The docs commit adds both and no command.

## Resume point

- Done: slices 1, 2a (launchd) and 2b (graceful stop). Slice 2c adds
  authenticated local-directory binding and OS-lock completion; focused
  checks precede its commit and the whole gate runs on that commit.
- Half-done: force, matching-service finalization and restart remain
  separate slices. Connection closure alone is still the observation for
  an endpoint whose token file belongs to another local directory.
- Next: gate and report 2c, then force, service finalization and restart,
  followed by slices 3 and 4. No real agent or service manager is verified.

## Existing asset check

| Asset | Where | Decision |
| --- | --- | --- |
| `node.stop` (`now`, `when_idle`), `Admin`, protocol 3 | `crates/vessel-proto/src/rpc.rs`, `vessel-node/src/lifecycle.rs`, `shutdown.rs` | Modify the CLI only. Do not add a stop method. |
| Live-session rule of `vessel node update` | `crates/vessel-cli/src/remote.rs` | Reuse for stop and restart. |
| `node.settings` / `node.set_settings`, `settings.json` | `vessel-proto/src/settings.rs`, `vessel-node/src/settings.rs`, `vessel-store/src/settings_file.rs` | Extend with unset and reload. Do not add `config.json`. |
| `vesseld.lock`, `vesseld.owner` | `vessel-store/src/lock.rs` | Do not add an argument list and do not execute the record. On Linux, read the `flock` holder from `/proc/locks` and its executable from `/proc/<pid>/exe`. |
| `vessel init --service` | `vessel-cli/src/init/service.rs` | Modify the launchd `KeepAlive`. Do not rewrite a unit whose text differs. |
| `vesseld attach` start | `vessel-node/src/attach/start.rs` | Adopt its detach for a foreground restart. Do not guess `--bind none` when the arguments were not recorded. |
| Device pairing, local token | ADR 0005, ADR 0010, `vessel node pair` | Adopt as the password's replacement. No password hash. |
| Paseo supervisor, dotted config paths | the daemon package | Do not adopt. Reasons in the ADR. |

No new crate, no new dependency, no new edge in `docs/ARCHITECTURE.md`.
External programs (`systemctl`, `launchctl`) stay programs on `PATH`,
found the way the service installer finds them, and tests put stand-ins
there.

## What Paseo does

Read from Paseo 0.15.4. Paths are inside that repository.

Commands are registered in `packages/cli/src/commands/daemon/index.ts`
(lines 17 to 31): `start`, `run`, `status`, `stop`, `restart`, `reload`,
`pair`, `config`, then `set-password`.

- `start` (`start.ts` 5 to 25): local home, readiness 600s, reports
  `started` or `already_running`, the pid, the listen address and the log
  path.
- `run` (`start.ts` 28 to 44): foreground, same launch, exit code of the
  daemon.
- `status` (`status.ts` 49 to 135): local fields from the instance file and
  the saved config (`home`, `pid`, `listen`, `localDaemon`, log path);
  live fields from `getDaemonStatus` (version, worker pid, providers,
  relay). A remote miss is an error; a local miss is `unreachable` with a
  note.
- `stop` (`stop.ts` 7 to 101): `--timeout` 15s, `--kill-timeout` 3s,
  `--force` local only, `--keep-terminals` incompatible with `--force`.
  Shutdown is an RPC (`shutdownServer`). `--force` may then kill that
  local process. A remote stop does not check the exit.
- `restart` (`restart.ts` 20 to 21, 25 to 26, 51 to 58, 101 to 118): the
  command restarts the worker, "retaining its supervisor launch". The
  supervisor is checked before the restart and again when the worker pid
  differs; a replaced supervisor is `DAEMON_REPLACED`. Deadline default
  600s. `--kill-terminals` closes them instead of keeping them. The human
  line says the supervisor launch was retained.
- `reload` (`reload.ts` 4 to 66): RPC `reloadDaemonConfig`. Human output
  lists restart-required paths and override-controlled paths, and the
  restart command.
- `config` (`config.ts` 8 to 114): local file, home required. `get`
  redacts names that look like a password, key, token, secret or
  authorization. `set` JSON-parses unless `--string`. `unset` deletes the
  path. Then reload if a daemon is listening; a saved-but-not-applied
  result names the next command. A failed reload after a successful save
  is `CONFIG_SAVED_RELOAD_FAILED`, not a silent success.
- `set-password` (`set-password.ts` 64 to 116): two terminal prompts, empty
  refused, mismatch refused, bcrypt hash written to
  `daemon.auth.password`, restart required. No terminal means set
  `PASEO_PASSWORD` instead.
- `--home` (`packages/cli/src/utils/command-options.ts` 10 to 28): default
  `~/.paseo`. `start`, `run`, `config` and `set-password` require a local
  home. `stop`, `restart`, `reload`, `status` and `pair` also take a
  remote host.

Reload result (`packages/server/src/server/daemon-config-store.ts` 55 to
59): `appliedPaths`, `restartRequiredPaths`, `overrideControlledPaths`.
Reloadable paths are the list at lines 181 to 206. The apply loop is 509
to 551. `$schema` and `version` are ignored. Speech paths
(`features.dictation`, `features.voiceMode`) are applied by the speech
owner, not by that list. Plugin records are not swapped in; the switch
`pluginsEnabled` is.

The file schema is `packages/server/src/server/persisted-config.ts` 236 to
371. A new file gets version 1, listen `127.0.0.1:6767`, CORS
`https://app.paseo.sh`, relay disabled, app base URL `https://app.paseo.sh`
(lines 382 to 396). Further defaults when a key is absent:
`packages/server/src/server/config.ts` lines 36 to 39 (port 6767, relay
`relay.paseo.sh:443`, app base URL, trusted proxies `loopback`), 548 to
566 (MCP on, inject off, usage on unless false, browser tools off,
auto-archive off, empty system prompt), 681 to 689 (terminal hooks off,
plugins off, idle cleanup on, catalog timeout absent), 842 to 843 (resume
off, prompt `Resume working.`). Git limits 64 per second and 8 at once:
`packages/server/src/utils/git-process-scheduler.ts` 9 to 12.

Launch overrides that a reload will not apply are collected in
`config.ts` from line 730 (`PASEO_LISTEN`, `PASEO_TRUSTED_PROXIES`,
`PASEO_PASSWORD`, `PASEO_RELAY_*`, `PASEO_LOG*`, and the others in that
function). A relay whose enabled flag was set at launch cannot be changed
until that override is gone (`daemon-config-store.ts`, the throw in
`applySupportedPatch`).

Removed CLI flags (`local-daemon.ts` 80 to 107) point at the config key.

Edge cases the slices have to keep a typed answer for:

- stop of a daemon that is not running;
- `--force` against a remote;
- `--force` with `--keep-terminals`;
- config edit without a home;
- save succeeded, reload failed;
- reload of a path pinned by the environment;
- a password that is empty, mismatched, or typed with no terminal.

## Starting point before implementation

Recorded in ADR 0064's "Existing implementation". At the start, the gaps
were the missing commands `stop`, `restart`, `config` and `reload`,
and a launchd unit that would undo a clean stop. The gaps it does not
close are Paseo's features this decision does not add (plugins, voice, a
daemon password, terminals that survive a restart, resume on startup).
Metadata, browser tools, MCP policy and auto-archive are ADR 0062, ADR
0069, ADR 0065 and ADR 0066, added at merge, not gaps this plan closes
or refuses.

## Slices

### Slice 1 (this commit)

ADR 0064 and this plan, including the corrections after the first
return. No code. The gate for this documents-only slice is the fast
checks: `cargo fmt --all --check`, `node scripts/check-rust-size.mjs`
and `node scripts/check-boundaries.mjs`. Clippy and `cargo test` are
not run. Nothing under `web/` changes.

### Slice 2: `vessel node stop` and `vessel node restart`

Split into 2a (launchd clean-exit policy), 2b (graceful stop), 2c (local
identity and lock completion), then forced cleanup, service finalization
and restart. Each is independently tested and gated.
2a reuses `init/service.rs` and `write_once`: no dependency or crate edge
is added. Standard-library rendering and the existing service manager
are sufficient. Unlike Paseo's retained supervisor launch
(`restart.ts` 20–21, 118), Vessel runs the node directly as a user service;
clean exit must therefore stay stopped. This is the service difference
already decided in ADR 0064. Linux's template and Windows's existing
service refusal are unchanged. Runtime launchd behaviour requires macOS.


As the ADR's Stop and Restart sections. Tests beside the command, with
the testkit's node, never a real agent and never the network:

- no live session: stop ends the node; the lock drops;
- live sessions and no flag: listed, node still serving, exit non-zero;
- `--when-idle`: state `draining`, a new session is `unavailable`, the
  node exits after the live one ends;
- `--now` without a terminal and without `--yes`: refused, nothing stopped;
- `--force` signals only the pid `NodeLock::holder` reports, and does not
  signal a stale owner file;
- a matching unit is stopped or restarted through the stand-in
  `systemctl` / `launchctl` on `PATH`; a unit whose text differs is not;
- foreground restart on Linux spawns `/proc/<pid>/exe` of the `flock`
  holder, with that process's arguments, and waits until a new pid holds
  the lock;
- the spawned process's environment is the invoker's: a marker set only
  on the stopped process is absent, a marker set only on the invoker is
  present, neither marker is in the owner file, and the test does not
  print the environment. Stdout is the node log. On Unix the child's
  process group id is its own pid;
- a record whose executable path was rewritten while the lock is held is
  refused, and the planted path is not executed;
- a program replaced at its path while it runs (`readlink` ends with
  ` (deleted)`) is restarted as the regular file now at that path when
  the record still names it; a record that names another path is not
  run; a missing path refuses with `the running program was replaced on
  disk; start the node with vessel node start` and spawns nothing;
- a process whose arguments were not read is a refusal that names
  `vessel node start`;
- restart when nothing is running is a refusal;
- a remote target's restart is the refusal the ADR names; its stop still
  calls `node.stop`;
- the Windows sentences for `--force` and `restart`, and the macOS
  sentence when the executable cannot be confirmed, are the ADR's words.
  The stand-in executable exists only in the test, not in a product build.

The launchd template change is in this slice, with the service unit test
updated for `KeepAlive` only on a failed exit. No `sleep` as the wait.

### Slice 3: `config` and `reload`

Two methods, as the ADR. `NODE_CONFIG_PROTOCOL` is
`Option<ProtocolVersion> = None`; the number is given at merge, and only
the integrator writes `Some(n)`. Before registering either method, merge
the node head with the shared `required_protocol()` accessor from the
titles track. Until that merge, ship value types only, with no registered
call or parallel gating mechanism. `PROTOCOL_VERSION` stays at its base.
The client refuses `None` before sending and the node refuses before
dispatch with typed `Unsupported` / `UnassignedProtocol`.

`docs/PROTOCOL.md` is updated when the methods are registered. Exhaustively
destructure `SetNodeSettings`: a new field must receive a key and its
ADR's reload class. `config set` names the whole field. `mcp.inject` and
`mcp.policy` apply to the next session; `mcp.serve` applies at once
(ADR 0065, added at merge). This track invents no dotted syntax.

Tests after the shared accessor arrives: round trip; Admin to mutate,
Read to get; unassigned calls refused without sending or dispatch; lower
negotiated versions refused after assignment; unset restores the launch
value; invalid settings reload changes nothing; unknown keys refuse
before a call; empty restart/override lists for this track's existing
keys; secret-free output. Any later addition to existing replies/events
must test every direct/list/page/nested/forwarded per-peer path while its
feature is unassigned and below its assigned version. Generated TS and
web gates accompany registration.

### Slice 4: sessions across a restart

No new behaviour. Tests, on a test node:

- `now`: every live session `Interrupted`, log committed, structured
  `harness_session` still set, terminal not resumable;
- `when_idle`: the live session finishes, a parked session does not hold
  the drain, and it is `Interrupted` once the node exits;
- after restart the session is not running and no prompt was sent to an
  agent.

## Stated differences (index)

The ADR has the full table. The ones a review will look for:

- no `set-password`: pair, revoke, or delete `token` and start again;
- no `--home`: `--data-dir` or a book entry;
- no dotted paths: a key is a variant of the settings enum; a name with
  no variant is refused; `metadata`, `browser_tools` and `mcp` are ADR
  0062, ADR 0069 and ADR 0065, added at merge;
- a foreground restart runs the lock holder's executable and the
  invoker's environment, not the owner file and not the stopped
  process's environment. A file replaced in place is the new file at
  the kernel's path, not the ` (deleted)` text;
- Windows has no service unit; `--force` and `restart` refuse with the
  ADR's sentences. macOS foreground restart refuses the same way when
  the executable cannot be confirmed without `unsafe`;
- stop asks before interrupting live sessions (ADR 0008), unlike Paseo;
- no terminal keep, no resume-on-startup;
- remote restart is refused; remote stop works;
- `restart_required` and `override_controlled` exist and are empty;
- saved `park_after` keeps beating the flag.

## Slice 2b implementation evidence

- Read Paseo `packages/cli/src/commands/daemon/stop.ts` 13–21
  (15-second graceful and 3-second forced defaults), 44–71 (remaining
  deadline, RPC, local-only force), 95–98 (remote exit unverified);
  `local-daemon.ts` 74–79 (positive finite seconds, rounded up to ms).
- Reused `NodeClient::closed` and the existing Admin-scoped `stop_node`;
  the lock is re-exported through `vessel-node` for local lifecycle checks,
  without adding a dependency edge or duplicating persistence rules.
  The preflight lock check applies only when connection uses the implicit
  local token: a book endpoint's explicit token file may name a different
  local data directory and must still be usable over the RPC.
- `[modify]`: extend CLI dispatch and arguments; reuse the update's live
  listing and confirmation. Those existing functions now receive the
  resolved target and live-session slice; confirmation returns a typed
  reply so the new call crosses no raw name/count/boolean boundary.
  The update command retains the same output and approval rule. A supervisor, external API or new dependency
  is unnecessary for graceful shutdown. The Rust standard library parses
  deadlines; unrepresentable monotonic deadlines are typed refusals.
- Differences already decided in ADR 0064: explicit live-session consent,
  drain choice and no terminal preservation. New numerical refusal:
  values too large for the monotonic clock are refused rather than
  becoming an overflowing deadline. This is a checked-arithmetic safety floor.
- Current slice limit: graceful stop only. `--force`, `--kill-timeout`,
  service-manager finalization and `restart` follow separately. Windows
  uses the same graceful RPC and event; actual Windows and macOS execution
  still require those operating systems.
- No wire changes or generated TypeScript changes.

## Slice 2c existing assets and verification

`[modify]`: reuse `DataDir::read_node_id`, the authenticated greeting,
`NodeClient::closed` and the standard-library lock already used by
`NodeLock`. The new observer separates connection-only and directory-bound
postconditions. The small waiter exists because neither the client event
nor `NodeLock::holder` provides a cancellable release event; it uses no new
dependency. ADR 0064 records the alternatives and startup race.

Paseo `packages/server/src/server/daemon-instance.ts` 200–214 was read:
its local exit loop checks the pid and instance replacement, then delays
100 ms. Vessel waits on the existing directory lock rather than polling
that pid. Defaults and the remote request-only result are unchanged.
The preflight and waiter briefly acquiring a lock can cause a concurrent
start's typed refusal; no atomic stop/start guarantee is claimed.

The focused checks cover the stop CLI plus the three waiter tests.
The process-exit test holds the lock in its parent and proves a cancelled
waiter does not keep the child command alive. The two-node CLI test proves
an explicit endpoint does not wait on another still-running local node.
The whole gate runs once after committing and prints HEAD/status first.
No wire, TypeScript, dependency or crate-edge change.

A7: the stop default/minimum, waiter test budget and handshake test budget
are each named once beside the relevant type or fixture. Checks and
expiry/error output use those values, and boundary tests derive from the
constants. The default is rendered by clap rather than duplicated in a
help comment. No fixed budget is copied into an expectation string.
