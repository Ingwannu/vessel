# Plan: node management parity (ADR 0064)

How Vessel stops, restarts and reconfigures a node, against Paseo's daemon
commands. The decision is ADR 0064. This file is the evidence and the
slice order. The docs commit adds both and no command.

## Resume point

- Done: Paseo's daemon commands, reload result and config keys read; Vessel's
  `node.stop`, settings, service unit, attach start and lock read. ADR 0064
  and this plan written. No code.
- Half-done: nothing.
- Next: slice 2, `vessel node stop` and `vessel node restart`, after this
  commit's gate is green. Do not start it on a red gate. A review that
  returns the ADR is fixed before slice 2.

## Existing asset check

| Asset | Where | Decision |
| --- | --- | --- |
| `node.stop` (`now`, `when_idle`), `Admin`, protocol 3 | `crates/vessel-proto/src/rpc.rs`, `vessel-node/src/lifecycle.rs`, `shutdown.rs` | Modify the CLI only. Do not add a stop method. |
| Live-session rule of `vessel node update` | `crates/vessel-cli/src/remote.rs` | Reuse for stop and restart. |
| `node.settings` / `node.set_settings`, `settings.json` | `vessel-proto/src/settings.rs`, `vessel-node/src/settings.rs`, `vessel-store/src/settings_file.rs` | Extend with unset and reload. Do not add `config.json`. |
| `vesseld.lock`, `vesseld.owner` | `vessel-store/src/lock.rs` | Modify the owner record with the arguments. Trust it only while the lock is held. |
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
- `restart` (`restart.ts` 6 to 18 and the loop that follows): supervisor
  stays, worker pid must change, deadline 600s, `--kill-terminals` closes
  them instead of keeping them. A remote restart has only what the
  replacement reports.
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

## What Vessel does today

Recorded in ADR 0064's "Existing implementation". The gaps this plan
closes are the missing commands `stop`, `restart`, `config` and `reload`,
and a launchd unit that would undo a clean stop. The gaps it does not
close are Paseo's features Vessel does not have (plugins, voice, browser
tools, a daemon password, terminals that survive a restart, resume on
startup). Those stay absent, named in the ADR's key table.

## Slices

### Slice 1 (this commit)

ADR 0064 and this plan. No code. Gate is the repository gate; nothing
under `web/` changes, so the page's checks are not run.

### Slice 2: `vessel node stop` and `vessel node restart`

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
- foreground restart re-execs the recorded arguments and waits until a
  new pid holds the lock;
- a record with no arguments is a refusal;
- restart when nothing is running is a refusal;
- a remote target's restart is the refusal the ADR names; its stop still
  calls `node.stop`.

The launchd template change is in this slice, with the service unit test
updated for `KeepAlive` only on a failed exit. No `sleep` as the wait.

### Slice 3: `config` and `reload`

Provisional protocol 21, two methods, as the ADR. `PROTOCOL_VERSION` and
`docs/PROTOCOL.md` move in this commit. Generated TypeScript is
regenerated in the same commit (`web/src/wire/`), and the page checks run
because the wire changed.

Tests: round trip; `admin` required to set, unset and reload; `read`
enough to get; a node negotiated at 20 answers `unsupported` and the CLI
does not send; unset clears the saved value so the flag applies; a bad
`settings.json` on reload changes nothing; an unknown key is refused
before a call; `restart_required` and `override_controlled` are empty;
secrets are not in the output (the settings types have none, and the test
uses a token file the command must not print).

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
- no dotted paths: two keys, unknown refused;
- stop asks before interrupting live sessions (ADR 0008), unlike Paseo;
- no terminal keep, no resume-on-startup;
- remote restart is refused; remote stop works;
- `restart_required` and `override_controlled` exist and are empty;
- saved `park_after` keeps beating the flag.
