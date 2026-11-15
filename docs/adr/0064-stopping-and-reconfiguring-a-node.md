# ADR 0064: Stopping, restarting and reconfiguring a node

Status: proposed, 2026-11-14. The contract only. No command and no wire
method is added by this file. The new-call constant is
`pub const NODE_CONFIG_PROTOCOL: Option<ProtocolVersion> = None`.
The integrator assigns `Some(n)` only when every client that shows or
approves the new data is ready; this decision chooses no number.

Before registering `node.reload` or `node.unset_settings`, this track
merges the node head containing the shared `required_protocol()` accessor,
landed once by the titles track. Until then it ships value types only.
With that accessor, an unassigned call returns `None`; the client refuses
before sending and the node refuses before dispatch with typed
`Unsupported` (`CallError::UnassignedProtocol { method }`). This track
introduces no second gate. `PROTOCOL_VERSION` stays at the base value.
`docs/PROTOCOL.md` is updated with registration, not before it.

## Purpose and intent

Paseo manages its daemon with `start`, `run`, `status`, `stop`, `restart`,
`reload`, `config`, `pair` and `set-password`, and with one `config.json`
per home (`--home`). A reloadable key takes effect without a restart; any
other changed key is reported as restart-required; a key pinned by the
launch is reported as override-controlled and not applied.

Before this decision, Vessel starts, installs, updates, inspects, pairs
and tunes a node, and can stop one on the wire. It lacks `stop`, `restart`,
`reload` and `config` commands; the resume plan records their delivery. This decision says which of Paseo's commands
and keys that existing surface is, which keys become `node.settings`
(applied by a reload), which stay on the command that started the process,
and what a person does instead of a daemon password.

## Existing implementation and constraints

### What Paseo does

Commands (`packages/cli/src/commands/daemon/index.ts`, the list at lines
17 to 31, and `set-password` at 26 to 30):

| Command | What it does |
| --- | --- |
| `start` | Local home only. Starts from the saved config, or reports the one already running. Readiness deadline 600s (`start.ts` 5 to 25). |
| `run` | Local home, foreground. The same launch, in the process, for deployment overrides (`start.ts` 28 to 44). |
| `status` | Local instance record when the home is local, plus a live probe when something is listening (`status.ts` 13 to 47 and 108 to 135). |
| `stop` | Asks the daemon to shut down. Deadline 15s, forced-kill deadline 3s. `--force` is local only and cannot be combined with `--keep-terminals`. A remote endpoint is asked, and its exit is not verified (`stop.ts` 7 to 18 and 48 to 70). |
| `restart` | Replaces the worker and keeps the supervisor's launch. Readiness 600s. Confirms a new pid (`restart.ts` 6 to 18 and the pid check that follows). |
| `reload` | `reloadDaemonConfig`. Prints applied paths, restart-required paths and override-controlled paths, and the restart command (`reload.ts` 4 to 66). |
| `config get\|set\|unset` | Edits the local file (a home is required). `get` redacts a key whose name looks like a secret. `set` parses JSON unless `--string`. A running daemon is then reloaded (`config.ts` 8 to 22 and 36 to 114). |
| `pair` | Prints a pairing link. `--home` selects the daemon (`pair.ts` 64 to 73). |
| `set-password` | Reads a password twice on a terminal, stores a bcrypt hash at `daemon.auth.password`, and says to restart (`set-password.ts` 64 to 116). |
| `--home` | Which local daemon. Default `~/.paseo` (`packages/cli/src/utils/command-options.ts` 10 to 28). |

Reload (`packages/server/src/server/daemon-config-store.ts` 55 to 59 and
181 to 206, applied at 484 to 551). The result is three sorted lists:
`appliedPaths`, `restartRequiredPaths`, `overrideControlledPaths`. A path
in `RELOADABLE_PATHS` whose value changed, and is not pinned by a launch
override, is applied. Any other persisted difference since the process
started is restart-required, except `$schema`, `version`, and speech paths
when speech is applied on its own. Plugin source changes are not applied
(the running plugin list is kept); the global plugin switch is reloadable.
A launch override (`packages/server/src/server/config.ts` 730 to 761, and
the relay and log paths that follow) puts that path in
`overrideControlledPaths` and the file does not win.

The file (`packages/server/src/server/persisted-config.ts` 236 to 371, the
written default at 382 to 396) and the defaults applied when a key is
absent (`packages/server/src/server/config.ts` 36 to 39 and 548 to 566,
681 to 689, 842 to 843; git at `packages/server/src/utils/git-process-scheduler.ts`
9 to 12):

| Key | Default when absent | Paseo reload class |
| --- | --- | --- |
| `$schema`, `version` | version 1 in a new file | ignored by reload |
| `daemon.listen` | `127.0.0.1:6767` | restart-required; `PASEO_LISTEN` pins it |
| `daemon.hostnames` (`allowedHosts` is folded into this on read) | built-in loopback names, and an environment list is appended rather than pinned | reloadable |
| `daemon.trustedProxies` | `["loopback"]` | reloadable; `PASEO_TRUSTED_PROXIES` pins it |
| `daemon.mcp.enabled` | true | reloadable; a removed launch flag used to pin it |
| `daemon.mcp.injectIntoAgents` | false | reloadable; same |
| `daemon.usage.enabled` | true (only an explicit false turns it off) | reloadable |
| `daemon.browserTools.enabled` | false | reloadable |
| `daemon.git.maxProcessesPerSecond` | 64 | reloadable; env can pin it |
| `daemon.git.maxProcessConcurrency` | 8 | reloadable; env can pin it |
| `daemon.autoArchiveAfterMerge` | false | reloadable |
| `daemon.enableTerminalAgentHooks` | false | reloadable |
| `daemon.appendSystemPrompt` | empty | reloadable |
| `daemon.terminalProfiles` | absent means the built-in profiles; an empty list means the person removed them | reloadable |
| `daemon.agentProfiles` | absent | reloadable |
| `daemon.cors.allowedOrigins` | `https://app.paseo.sh` in a new file | reloadable |
| `daemon.relay.enabled` | false in a new file | reloadable; `PASEO_RELAY_ENABLED` pins it |
| `daemon.relay.requireProtectedSecurity` | false | reloadable |
| `daemon.relay.endpoint` | `relay.paseo.sh:443` | restart-required; env pins it |
| `daemon.relay.publicEndpoint` | absent | restart-required; env pins it |
| `daemon.relay.useTls` | absent | restart-required; env pins it |
| `daemon.relay.publicUseTls` | absent | restart-required; env pins it |
| `daemon.serviceProxy.enabled` | localhost proxying stays on; false only suppresses the optional listen and public layers | restart-required |
| `daemon.serviceProxy.listen` | absent | restart-required |
| `daemon.serviceProxy.publicBaseUrl` | absent | restart-required |
| `daemon.auth.password` | absent (no password) | restart-required; `PASEO_PASSWORD` pins it. Not in `RELOADABLE_PATHS`. `set-password` says to restart |
| `app.baseUrl` | `https://app.paseo.sh` | reloadable; `PASEO_APP_BASE_URL` pins it |
| `providers.local.modelsDir` | under the home | restart-required; `PASEO_LOCAL_MODELS_DIR` pins it |
| `pluginRegistries` | absent | restart-required (not in `RELOADABLE_PATHS`) |
| `pluginsEnabled` | false | reloadable |
| `plugins` | absent | not applied by reload; a change needs the plugin operation or a restart |
| `worktrees.root` | absent | restart-required |
| `worktrees.servicePorts` | absent | restart-required |
| `agents.providers` | absent | reloadable |
| `agents.idleRuntimeCleanup` | true (one hour, outside this file) | reloadable |
| `agents.resumeOnStartup` | false | reloadable |
| `agents.resumePrompt` | `Resume working.` | reloadable |
| `agents.catalogRefreshTimeoutMs` | absent | reloadable |
| `agents.metadataGeneration` | absent (automatic) | reloadable |
| `agents.skills.selection` | absent | reloadable |
| `features.dictation`, `features.voiceMode` | dictation on, voice mode off, in the speech resolver | applied by the speech owner during reload, not by `RELOADABLE_PATHS` |
| `features.webUi.enabled`, `features.webUi.distDir` | web UI off | restart-required; env can pin them |
| `log.level`, `log.format`, `log.console.*`, `log.file.level`, `log.file.path`, `log.file.rotate.maxSize`, `log.file.rotate.maxFiles` | console info, file info | restart-required; `PASEO_LOG*` pins a path it sets |

Removed launch flags (`local-daemon.ts` 80 to 107) tell the person to set
the config key and start or restart. Foreground is `run`, not a flag.

### What Vessel already does

- `vessel node start` runs the node in this process until Ctrl-C, `SIGTERM`
  or `node.stop` (`crates/vessel-cli/src/commands.rs`, `node_start`).
  `vessel node install` only puts a binary on another machine and does not
  start or restart anything (`crates/vessel-cli/src/remote.rs`, `install`;
  ADR 0008). `vessel node update` installs, then `node.stop`, and the next
  attach starts the new binary. With live sessions and no flag it changes
  nothing.
- `node.stop` is already a wire method, `Admin`, since protocol 3
  (`crates/vessel-proto/src/rpc.rs`). `now` ends live sessions
  `Interrupted`; `when_idle` refuses new sessions and exits when none is
  live (`docs/PROTOCOL.md` version 3; `crates/vessel-node/src/lifecycle.rs`).
  Parked sessions do not delay the drain. Shutdown publishes that final
  status, waits until the log is committed, then closes connections
  (`crates/vessel-node/src/shutdown.rs`).
- One node holds `vesseld.lock` for its whole life. Who it is (pid, binary,
  version, when) is in `vesseld.owner`, and that record is trusted only
  while the lock is held. A pid file alone was rejected because a pid is
  reused (`crates/vessel-store/src/lock.rs`).
- `vessel init --service` writes a user service that runs
  `<this vessel> --data-dir <dir> node start`, and leaves a file whose
  content differs alone (`crates/vessel-cli/src/init/service.rs`). systemd
  is `vessel.service` with `Restart=on-failure`. launchd is `vessel.node`
  with `KeepAlive` true, so a clean exit starts it again.
- `vesseld attach` starts a node when none answers: `--bind none
  --detached --data-dir <dir>`, under a separate start lock
  (`crates/vessel-node/src/attach/start.rs`).
- Settings a person changed are `settings.json`, the wire type
  `SetNodeSettings`: `park_after` (protocol 12) and `drivers` (protocol
  18). `node.settings` needs `read`; `node.set_settings` needs `admin`.
  A saved value wins over `--park-after`. What nobody saved keeps
  following the flag (`crates/vessel-node/src/settings.rs`,
  `crates/vessel-store/src/settings_file.rs`). A bad file is logged at
  start and the flags stand; the next change replaces it.
- Devices are paired (`vessel node pair`, `admin`) and revoked. The local
  token is `<data_dir>/token`, mode 0600. There is no password (ADR 0005,
  ADR 0010).
- Separate homes already exist: `--data-dir` / `VESSEL_DATA_DIR` for a
  node on this machine (`crates/vessel-cli/src/args.rs`), and a book entry
  per remote node.

Constraints that the choice has to keep:

- A saved `park_after` must keep beating the flag (the settings module's
  own decision). A saved bind that beat the flag would make an
  attach-started node take the default TCP port.
- `node.stop` stays the one shutdown. A second stop method would be two
  facts (the rule against a repeated method list).
- Nothing is signalled by process name. The owner record is not a pid to
  kill once the lock is free, and it is not a program a restart executes.
- A library does not read the environment or the service manager. The
  `vessel` binary does.
- No new crate and no new dependency: the service managers are programs
  on `PATH`, as `vessel init --service` already finds them.

## Alternatives considered

1. **A `config.json` of dotted paths, as Paseo.** `set` would accept any
   path and reload would diff JSON. Rejected: a path is a string, and
   behaviour would be chosen by matching it (the rules against a bare
   string and against matching on text). Unknown paths would be easy to
   store and then ignore.
2. **Persist every start flag in `settings.json` and let a saved value
   win**, as `park_after` does. Rejected for bind, relays and the proxy
   allow-list: `vesseld attach` and the service unit put those on the
   command line on purpose. One saved bind would undo `--bind none`.
3. **A supervisor process that restarts a worker**, as Paseo. Rejected:
   the node is the process. A supervisor is a new process, a new protocol
   and a new failure domain, which ADR 0008 already deferred (the session
   host). Restart can re-exec, or ask the service manager that already
   exists.
4. **An RPC that re-execs the node from inside.** Rejected: launchd with
   `KeepAlive` true starts a second process when the first exits. The
   binary cannot see the unit file without reading the home and the
   service manager, which a library must not do.
5. **A daemon password.** Rejected: ADR 0005 and ADR 0010. A shared
   password is one secret for every caller. A device credential is per
   machine and revocable.

## The choice

### Commands

| Paseo | Vessel | Difference |
| --- | --- | --- |
| `daemon start` | `vessel node start` | Already the foreground process. Not a wire call. |
| `daemon run` | `vessel node start` | No second command. Deployment overrides are the flags of that start, not a mode. |
| `daemon status` | `vessel node status` | Already. |
| `daemon stop` | `vessel node stop`, calling the existing `node.stop` | New command, no new method. See below. |
| `daemon restart` | `vessel node restart` | New command, no new method. See below. |
| `daemon reload` | `vessel node reload` | New method. Number given at merge, gated on `NODE_CONFIG_PROTOCOL`. |
| `daemon config get` | `vessel node config get` | `node.settings` (`read`). |
| `daemon config set` | `vessel node config set` | `node.set_settings` (`admin`). |
| `daemon config unset` | `vessel node config unset` | New method. Same constant. |
| `daemon pair` | `vessel node pair` | Already. |
| `daemon set-password` | none | Stated below. |
| `--home` | `--data-dir`, or a book entry | No new flag. |

`vessel node install` is not a service install. The service is
`vessel init --service`.

### Keys

Every field of `SetNodeSettings` is a variant of one key enum, with the
reload class decided in the ADR that adds the field: applied on reload,
start-time (`restart_required`), or pinned by the launch
(`override_controlled`). The enum is not a closed list of every settable
key, and this decision does not keep one. The wire name is the variant.
`unknown` exists so a newer peer's list still decodes; it is never
applied and never selected by matching text.

Slice 3 maps keys by exhaustively destructuring `SetNodeSettings`. A new
field does not compile until the ADR that adds it has given the field a
variant and a class, in that same commit. The fields in this tree today
are `park_after` and `drivers`. A key another ADR adds is that ADR's,
added at merge, and is not classed here as missing or refused.

| Paseo key | Vessel | Class |
| --- | --- | --- |
| `agents.idleRuntimeCleanup` | `park_after` on `node.settings` | **Reloadable setting.** Minutes from 1 to 10080, or never; default 60 minutes. Not a boolean, and not a fixed hour. |
| (no Paseo key) | `drivers` on `node.settings` | **Reloadable setting.** Vessel's own (ADR 0057). New sessions only; a running session keeps the protocol it started with. |
| `daemon.listen` | `--bind` / `VESSEL_BIND`, default `127.0.0.1:7077`, or `none` | **Start-time.** `config set bind` is refused. |
| `daemon.relay.endpoint`, `useTls`, `publicEndpoint`, `publicUseTls` | `--relay`, `--relay-token-file`, `--public` | **Start-time.** |
| `daemon.trustedProxies` | `--trusted-proxy`, `--client-address-header` | **Start-time.** |
| `daemon.hostnames` | the fixed `Host` checks of the web UI and the model proxy | **None.** Not a setting. |
| `daemon.cors.allowedOrigins` | no CORS, by design of the web UI | **None.** |
| `daemon.mcp.enabled`, `daemon.mcp.injectIntoAgents` | ADR 0065 adds one field `mcp`, added at merge | **That ADR's class.** `inject` and `policy` apply at the next session start (a running session keeps the list it started with). `serve` applies at once: the child process starts, or it is stopped (ADR 0065). Not classed as absent here. |
| `daemon.usage.enabled` | none | **None.** |
| `daemon.browserTools.enabled` | ADR 0069 adds `browser_tools`, added at merge | **That ADR's class.** The field was read (off unless set). When a live change applies was not read, so this row assigns no class. |
| `daemon.git.maxProcessesPerSecond`, `maxProcessConcurrency` | none | **None.** |
| `daemon.autoArchiveAfterMerge` | ADR 0066's later slice, added at merge if that slice adds a field | **That ADR's.** 0066 adds no `node.settings` key and defers the switch. Not classed as absent. |
| `daemon.enableTerminalAgentHooks` | none | **None.** |
| `daemon.appendSystemPrompt` | none | **None.** Fleet instructions are a different mechanism (ADR 0012). |
| `daemon.terminalProfiles` | none (`agent_terminals` on a profile is not this list) | **None.** |
| `daemon.agentProfiles` | profiles and per-machine defaults, edited by their own commands | **None** as a config key. |
| `daemon.relay.enabled`, `daemon.relay.requireProtectedSecurity` | a relay is on when `--relay` is present; no protected-security switch | **Start-time** for the list. No separate enabled or protected key. |
| `daemon.serviceProxy.*` | none | **None.** |
| `daemon.auth.password` | device credentials and the local token | **None.** See the password difference. |
| `app.baseUrl` | a pairing link names the node's routes | **None.** |
| `providers.local.modelsDir` | none | **None.** |
| `pluginRegistries`, `pluginsEnabled`, `plugins` | none | **None.** |
| `worktrees.root`, `worktrees.servicePorts` | a worktree is created per session; the root is not a setting | **None.** |
| `agents.providers` | `vessel agents` and `profiles.toml` | **None** as a config key. |
| `agents.resumeOnStartup`, `agents.resumePrompt` | a session live at stop becomes `Interrupted` and waits | **None.** Not added. Slice 4 tests the absence. |
| `agents.catalogRefreshTimeoutMs` | `--agent-handshake-timeout` is a different limit | **Start-time**, and not that Paseo key. |
| `agents.metadataGeneration` | ADR 0062 adds `metadata`, added at merge | **That ADR's class.** Not applied to a running agent; the next generation reads the settings. Until that field lands, a title stays the first prompt line. Not classed as absent. |
| `agents.skills.selection` | none (fleet sync is not this selection) | **None.** |
| `features.dictation`, `features.voiceMode` | none | **None.** |
| `features.webUi.*` | `vessel ui` | **None** as a node setting. |
| `log.*` | `RUST_LOG`, stderr | **Start-time.** Not a config key. |
| `$schema`, `version` | `settings.json` has neither | Not accepted. |

`config set` or `config unset` names variants of the key enum. A name
that is not a variant is a usage error before any call. The error names
the flag when one exists (`bind`, `name`, `relay`, `trusted-proxy`, the
handshake timeout, the LSP flags) and says the next start is what applies
it. It does not write the value. A name with no flag and no field is
refused as an unknown key, not stored. `metadata`, `browser_tools` and
`mcp` are not unknown and are not refused here: ADR 0062, ADR 0069 and
ADR 0065 add them, at merge. This command does not implement those fields.
`config set` names a whole field, not a dotted subpath. The pieces of
`mcp` are ADR 0065's; this command does not invent a path syntax for them.

### Reload

`node.reload` (wire `node.reload`, `Admin`, gated on
`NODE_CONFIG_PROTOCOL`) takes an empty request and returns three lists of
the key enum, in the enum's order. Today that order is `park_after` then
`drivers`. A field added later joins where its ADR puts the variant:

- `applied`: reloadable keys whose saved value changed and was applied;
- `restart_required`: persisted keys that are start-time and differ from
  the running process;
- `override_controlled`: keys a launch pin stops the file from changing.

This track persists no start-time key, and a saved `park_after` wins over
the flag, so for the keys this decision maps the last two lists are empty.
They are still returned, and tested empty, so a later key has a place that
is not a silent omission. A key another ADR adds reports the class that
ADR decided; this reload does not drop it and does not reclassify it.
Reload re-reads `settings.json`. A missing file means nothing is saved:
the flags apply, and a key that thereby changes is `applied`. A file that
does not parse is `invalid_request`, and the running settings stay. That
is harsher than process start, which logs a bad file and continues,
because a person asked to apply the file. An `unknown` key in the file is
left as it is and named in the error; it is not dropped and not applied.

`node.set_settings` remains the change from a client. `config set` is that
call, not a file edit followed by a reload. A reload after a set applies
nothing more.

### Unset

`node.unset_settings` (wire `node.unset_settings`, `Admin`, gated on
`NODE_CONFIG_PROTOCOL`) names one or more keys. Each named saved value is
removed, so
the flag or the default applies again, including to live sessions for
`park_after` and to new sessions for `drivers`. The answer is the settings
then in force. An empty list, a repeated key, or `unknown` is
`invalid_request` and writes nothing. Unset is a method of its own, gated through the shared
`required_protocol()` accessor, so an unassigned feature or an older
connection is refused rather than ignoring a field on `node.set_settings`.

### Who may call

Every call that changes the node needs `Admin`: `node.stop` (already),
`node.set_settings` (already), `node.unset_settings`, `node.reload`.
`node.settings` and therefore `config get` need `Read`. A caller without
the scope gets `forbidden` and nothing is written. The node token and the
local socket keep every scope, as they do today.

### Older and newer peers

- `NODE_CONFIG_PROTOCOL` remains `None` until assignment at merge. After
  the shared accessor reaches this branch, the two registered calls return
  `None` from `required_protocol()` and are refused by both client and
  node as typed `Unsupported`. The client sends nothing. Before that
  merge, this track registers no new call.
- Once the integrator assigns `Some(n)`, those calls are available only
  to peers at least `n`. A client negotiated below it refuses before
  sending; a node receiving the call from a lower-version connection
  refuses before dispatch. Existing `config get`, and `config set` of
  `park_after` (since 12) or `drivers` (since 18), keep their existing gates.
- No new field is added to an existing reply or event by this decision.
  Any such later field must be hidden while its feature constant is
  `None`, and from older peers after assignment, in direct, list, page,
  nested and forwarded replies through the shared per-peer mechanism.
- A reload or unset answer that carries `unknown` is shown as a setting
  this client does not know. It is not omitted from the list.

### Stop

`vessel node stop` calls `node.stop`. It does not add a method.

Live sessions use the rule `vessel node update` already uses. With none,
the stop is `now`. With some, and no flag, they are listed and nothing
stops. `--when-idle` drains. `--now` asks first, unless `--yes`, and
refuses without a terminal when `--yes` is missing. This is not Paseo's
stop, which always requests shutdown. The difference is deliberate:
ADR 0008 already refuses to interrupt sessions unless the person asks, and
a terminal cannot be resumed.

The CLI waits until the data directory's lock is free or the connection
has closed, whichever it can see, with the `--timeout` budget (the default is
`StopTimeout::DEFAULT`). For a data directory whose saved `node.id` equals the authenticated
connection's node id, the CLI observes connection closure and then the OS
lock's release. A loopback endpoint whose local directory has another id
only provides connection closure; it cannot authorize waiting on that
other directory. The deadline bounds the whole command.

`--force` is only for a data directory on this machine. If the call does
not finish, the process that holds the lock is signalled (`SIGTERM`, then
`SIGKILL` after `--kill-timeout`, default 3 seconds). That process is the
one "Whose executable" identifies, and only when it agrees with
`NodeLock::holder`. The signal is that pid, never a process found by
name, and never a pid taken from a record that disagrees. A record whose
lock is not held is stale and is not signalled. `--force` on a remote
node, and `--force` with `--when-idle`, are usage errors. On Windows, and
on macOS when the holder cannot be confirmed, nothing is signalled; the
refusals are below.

A service is used only when the unit `vessel init --service` writes is
present and its command contains this data directory as `--data-dir`. A
unit that names another directory, or whose text is not the text this
version writes, is not touched. The report says so when a stop may
therefore not stay stopped.

On systemd the matching unit has `Restart=on-failure`, so a clean exit
stays down. The CLI still asks `systemctl --user stop` of that unit after
the lock is free, so the unit is inactive and not merely exited. It does
not disable the unit: the next login can start it, and `vessel node start`
is the explicit start. `systemctl stop` is not sent while a drain is in
progress, because that signal would interrupt the sessions the drain is
waiting for.

On launchd the unit this version writes keeps the agent alive only when
the exit failed (`SuccessfulExit` false, the same rule as
`Restart=on-failure`). A plist that still has `KeepAlive` true differs,
so it is not rewritten and not unloaded. `node stop` then uses the RPC
only and reports that the agent is set to start again on exit, with the
undo line the plist already contains. A matching plist stays down after
a clean exit. Unload, without the flag that disables the agent across
logins, runs only after the lock is free.

### Restart

`vessel node restart` is not a wire method. It stops, then starts the same
launch. It is refused when the node is not running: restart is not start.

The live-session rule is the stop's rule. The readiness deadline is 600
seconds (`--timeout`). Readiness is a new holder of the lock (a different
pid) that answers `node.info`. The wait is on the lock and that call.

How it starts again:

- A matching service unit: the service manager restarts that unit
  (`systemctl --user restart`, or the launchd kick that stops and starts
  that label). The unit's command is the launch. The new process gets the
  unit's environment, not the invoker's and not the stopped process's:
  `PATH` as `vessel init --service` recorded it (`Environment=` in the
  systemd unit, `EnvironmentVariables` in the plist) plus what the service
  manager itself adds. For `--when-idle` the manager is not told to
  restart while the process is alive; the CLI starts the unit after the
  lock drops. A unit is used only when its text is the text this version
  writes, so a rewritten unit is not what restart runs.
- No such unit, on Linux: the CLI spawns the process that holds the lock,
  after reading it while the lock is still held, then waiting until the
  lock drops. It does not spawn a program named by `vesseld.owner`. See
  below.
- No such unit, on macOS or Windows: refused, with the words below. The
  owner record is not run instead.
- A node reached over SSH or a relay is not started again by this command.
  This machine has neither its service manager nor its executable. Stop
  still works, through `node.stop`, including from `vessel node update`.
  The next `vesseld attach` starts an SSH node, as ADR 0008. The error for
  `restart` says that.

The process a foreground restart spawns runs as the invoking user and can
do what that binary can do as that user. No token, key or relay secret is
placed on its command line. An environment value is not copied into the
owner file, a log or an error. A mismatched record, a remote node, macOS
without a matching unit, and Windows are refusals, not a fallback.

### Whose executable a restart runs

`vesseld.owner` is a record the node writes. An agent running as the
node's user can rewrite a file in the data directory. Comparing the
record's executable with the record's first argument would compare two
fields of the same file, and the next restart would run whatever that
writer put there. Restart does not do that. `NodeOwner` does not grow an
argument list. The owner file is not the program and not the arguments.

While the lock is held, the program and the arguments are read from the
process that holds the lock. The record's `executable` must be that same
program. If it is not, restart refuses, spawns nothing, and the error is
`the owner record does not match the process holding the lock`. The data
directory's files are owner-only (0600 on Unix), which refuses a different
OS user and does not refuse the node user, so the mode is not this check.

- Linux: the holder is the process id of the `flock` writer on
  `vesseld.lock` in `/proc/locks` (the lock file's device and inode), not
  a pid taken from the record alone. The executable is `readlink` of
  `/proc/<pid>/exe` (the standard library, no `unsafe`), except a result
  that ends in ` (deleted)` and is not itself a file, which the next
  point handles. The arguments are `/proc/<pid>/cmdline`, split on NUL.
  `holder()`'s pid must be that same pid. A disagreement is the refusal
  above: nothing is signalled and nothing is spawned from the file's pid
  or the file's path.
- The arguments must be non-empty, must include `--data-dir` of this
  directory, and must contain no NUL inside a field. Otherwise the refusal
  names `vessel node start` and does not guess `--bind none`.
- A binary replaced while it runs (a rebuild, an install over the same
  path, a package upgrade; not `vessel node update`, which stops and lets
  the next attach start the new one) makes Linux `readlink` of
  `/proc/<pid>/exe` return `<path> (deleted)`. That text is not a path to
  execute, and comparing it with the record would refuse a normal upgrade
  as a tampered record. Restart treats a `readlink` result that names an
  existing file as the program, including one whose real name ends in
  ` (deleted)`. When the result does not exist and ends with the suffix
  ` (deleted)`, the path is the text without that suffix. Restart runs
  the regular file now at that path only when the record's `executable`
  is that same path. The path comes from the kernel; the record only has
  to still name it, which is what the process stored from its own
  `current_exe()` when it took the lock. A record rewritten to any other
  path is the tamper refusal above and is not run. The new file is
  whatever now stands at the path the kernel reported, the same path a
  person would start again. Whoever can replace that file can already
  choose what the next start runs. If that path is missing, or is not a
  regular file, restart refuses with `the running program was replaced
  on disk; start the node with vessel node start` and spawns nothing.
- macOS: there is no `/proc`. `proc_pidpath` would name the executable of
  `holder()`'s pid, and calling it is `unsafe`, which this workspace
  forbids. A matching launchd unit is still restarted by the service
  manager, because the program is the unit this version wrote. With no
  such unit, `vessel node restart` refuses with `restart cannot confirm
  the running executable on this system`, and `--force` refuses with
  `forced stop cannot confirm the process holding the lock on this
  system`, rather than signalling or executing the record.
- Slice 2 plants a different executable path in the record while the lock
  is held. Restart refuses, and that path is not executed. It also
  replaces a running program at its path: the new file there is what
  restart spawns, a record pointing somewhere else is not run, and a
  missing path gets the replacement sentence above.

### Environment, stdio and the process group

Nothing from the stopped process's environment is written down. The
environment can hold secrets and the homes an agent was signed in under
(`PATH`, `HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `VESSEL_*`,
`RUST_LOG`). Recording it would store those in the data directory.

A foreground restart is performed by the CLI, so the new process receives
the invoker's environment, not the stopped process's. Paseo's
`daemon restart` restarts the worker while "retaining its supervisor
launch" (`packages/cli/src/commands/daemon/restart.ts`, line 21), and the
result line says the supervisor launch was retained (line 118). Vessel
has no supervisor. This is a stated difference: after a foreground
restart a harness can be found, or signed in, as the invoker rather than
as the process that was stopped. A person who needs the stopped process's
environment starts again with `vessel node start` from that environment.
A service restart differs again: the new process gets the unit's
environment, above, which is the `PATH` captured at `vessel init
--service` and not the invoker's whole environment.

Stdio of a foreground restart is not the invoker's terminal. The first
process may have been logging there. Stdin is null. Stdout and stderr are
the data directory's node log, the file `vesseld attach` already opens.
On Unix the child is started in a new process group, the detach
`vesseld attach` uses (`process_group(0)` in `attach/start.rs`). It is
not a new session: a second detach would be a second mechanism. The child
is not in the CLI's process group, so the CLI's exit and that group's
signals do not reach it.

Slice 2 tests this with two markers that are not secrets. The stopped
process has one, the invoker has the other. The owner file contains
neither. A stand-in executable, built only for the test, records the
environment it was given: the invoker's marker is present and the stopped
process's is absent. The test does not print the environment. The
stand-in's stdout is the node log. On Unix its process group id is its
own pid.

### Windows

`vessel init --service` has no Windows unit. On that system it already
reports that a service is not available yet and that the node is started
with `vessel node start`. This decision does not add a unit.

`vessel node stop` without `--force` is the RPC, on Windows as on Unix.
`--force` sends `SIGTERM` and then `SIGKILL` only where those signals
exist. Windows has neither, and this tree does not terminate an arbitrary
process there. When the stop has not finished and a signal would have
been sent, the command refuses with `forced stop is not available on
Windows; the node was asked to stop and is still running`. It does not
run `taskkill` and it does not take a dependency to call
`TerminateProcess`.

`vessel node restart` on Windows refuses, whether or not a service was
expected, with `restart is not available on Windows; the running
executable cannot be confirmed here, and the owner record is not run.
Start the node with vessel node start`. There is no unit to restart and
no `/proc` with which to confirm the holder.

### What a restart does to sessions

Unchanged, and tested by slice 4 rather than redesigned:

- `now` ends every live session `Interrupted`. The log is committed. A
  structured session can be resumed through its `harness_session`. A
  terminal's PTY is gone and cannot.
- `when_idle` lets live sessions finish. New sessions get `unavailable`.
  Parked sessions do not keep the node up; when it then exits they are
  `Interrupted` too.
- Nothing starts those sessions again. There is no resume-on-startup and
  no resume prompt. Paseo's default is the same (resume off). Paseo can
  also keep terminals across a restart; Vessel cannot, because there is no
  session host (ADR 0008). There is no `--keep-terminals` and no
  `--kill-terminals`.

### Password

There is no `set-password` and no password key.

A person on another machine pairs (`vessel node pair`, `admin`), with
`--read-only` when the device must not operate or administer, and revokes
that device with `vessel node devices revoke`. A person on this machine
uses the token file. Rotating it is deleting `token` and starting the node
again (ADR 0005). `config get` prints settings only. It never prints the
token, the node key, a device secret or a relay token.

## Why this and not the others

The two settings and `node.stop` already exist and already have versions,
scopes and tests. Extending them keeps one settings document and one
shutdown. A second config file or a second stop would drift from them.

Paseo's path strings and its supervisor are the parts that do not fit a
typed node whose process is the service. The reload's three lists are the
part that does fit, so they are kept even while two of them are empty.

Refusing an unknown key, a bad file and a stale pid is the alternative to
dropping it. Each refusal names what was wrong.

## Advantages, costs and effects

Advantages: on Linux, stop and restart work for a matching service unit
and for a foreground process, without a new protocol for stop and without
signalling by name. A matching launchd unit restarts the same way. Config
changes are the settings the node already applies. An older client and an
older node keep working.

Costs: `restart_required` and `override_controlled` carry no keys of this
decision until a later decision persists a start-time value. A remote node
cannot be restarted from here. launchd agents written before the
`KeepAlive` change are not rewritten, and a stop against one of them may
come back; the command says so. A foreground restart on macOS, and any
restart on Windows, is a typed refusal, because the running executable
cannot be confirmed there without `unsafe`. The foreground process gets
the invoker's environment, not the stopped process's. Two wire methods
wait on `NODE_CONFIG_PROTOCOL`, the number given at merge. `NodeOwner`
does not grow a field.

Effects on later slices: slice 2 is the commands `stop` and `restart`, the
live-executable check, the environment test, the tampered-record test and
the launchd template. Slice 3 is `config`, `reload`, `unset`, the
exhaustive `SetNodeSettings` mapping and the protocol document. Slice 4
tests sessions across a stop and a restart and adds no resume behaviour.
No crate boundary changes, so `docs/ARCHITECTURE.md` does not gain a row.
The CLI's help is the user-facing text, updated with the command that
adds it.

## Local exit observations (slice 2c)

[Decision Log]
- Purpose and intent: distinguish a shutdown request, connection closure
  and verified release of this node's data-directory lock.
- Existing implementation and constraints: slice 2b waits for connection
  closure. `Teardown::run` closes RPC connections before the store and
  `serve` drops `NodeLock` afterward; closure alone cannot establish lock
  release. Paseo's `daemon-instance.ts` 200–214 waits for its owner pid,
  rechecks replacement and delays 100 ms between probes.
- Alternatives considered: poll `NodeLock::holder`; use a Tokio blocking
  worker; add a platform watcher or a supervisor. Polling violates the
  event-wait rule. A blocking-pool acquisition cannot be cancelled and
  runtime shutdown waits for that pool. A watcher or supervisor adds a
  dependency or process boundary without improving the existing lock.
- Selected approach: compare the saved node id with the authenticated
  greeting before the request. Only a match enables OS-lock completion.
  One dedicated standard-library thread opens the existing lock read/write,
  waits for exclusive acquisition, unlocks immediately and sends its I/O
  result through a one-slot completion channel. It never writes file data
  or the owner record. A cancelled borrow preserves that same receiver.
- Why this approach: the OS owns the release event on every supported
  platform. A CLI deadline can end the process while the dedicated thread
  is blocked; Tokio's blocking-pool shutdown cannot. No crate or dependency
  is added. This is an implementation difference from Paseo's pid polling.
- Advantages, costs and effects: local success can name actual lock
  release; explicit token-file endpoints still work with another local
  directory. `try_lock` in the existing preflight and exclusive acquisition
  in the waiter can overlap a concurrent node start. That start may receive
  the existing typed `Locked` refusal; acquisition is followed immediately
  by unlock, but the interval is not measured and stop/start is not atomic.
  No owner data is rewritten. Tests pin cancellation, unchanged owner bytes,
  the original I/O cause, process exit with a blocked waiter, and two live
  local nodes with different identities. Windows/macOS runtime verification
  remains separate; the standard lock API already underlies `NodeLock`.

The stop budget's default and minimum are named beside `StopTimeout`.
Clap renders the default from that type, validation errors render its
minimum, and an expiry error carries the same parsed budget used by the
check. Focused waiter/handshake test budgets are named constants, too.


### Kernel lock observation before escalation

[Decision Log]
- Purpose and intent: keep process authority at the existing `NodeLock`
  boundary, and distinguish a kernel writer from a PID in a writable record.
- Existing implementation and constraints: `NodeLock::holder` probes the OS
  lock and returns an optional owner record. It deliberately has no process
  authority claim. Its behavior and record format stay unchanged. Paseo
  `packages/server/src/server/daemon-instance.ts` 197–214 checks reserved and
  self PIDs, instance replacement and exit in a timed loop.
- Alternatives considered: another CLI parser (duplicate lock knowledge),
  trusting the owner PID (insufficient authority), or interpreting no procfs
  row as a free lock (incorrect for every namespace/filesystem).
- Choice: an additive `NodeLock::kernel_holder` yields `KernelProbe::Found`
  with a typed PID and opaque device/inode snapshot, `NotVisible`, or
  `Unsupported`. Linux reads bounded `/proc/locks` data and accepts only an
  exclusive advisory flock writer for that file. Checks and errors use the
  single `LockTable::MAX_BYTES`; numeric errors keep their source. This
  operation acquires no lock, writes no record and sends no signal. Reads
  use the invoking OS account, load no credential or environment, and a
  permission failure remains a typed I/O refusal.
- Why this choice: the library already owns the lock format and lifetime.
  A positive observation can refuse a damaged held owner without the CLI's
  second acquisition probe. An absent row is never proof of availability:
  Linux filters procfs locks by PID namespace, and network filesystems can
  emulate flock with other lock types. The existing OS availability fallback
  remains. See [proc_locks(5)](https://man7.org/linux/man-pages/man5/proc_locks.5.html)
  and [flock(2)](https://man7.org/linux/man-pages/man2/flock.2.html).
- Benefits, costs and effects: one process-authority decoder for later force
  and restart; no dependency, crate edge, wire field or schema change. A
  snapshot is not a lease: later actions must revalidate it. `NotVisible`
  never authorizes a signal. macOS and Windows return `Unsupported` and keep
  their existing OS availability probe. The fallback and completion waiter
  still have the short acquisition/startup race stated above; this does not
  promise atomic stop/start. An unreadable kernel table is a typed refusal,
  not an assumption that a held directory is free. Only Linux local locks
  are exercised here; namespace, NFS/SMB and other OS runtime are not verified.


## Selected-file kernel decoding (slice 2e)

[Decision Log]
- Purpose and intent: an unrelated lock's PID or unfamiliar record must not
  hide the selected directory's holder.
- Existing implementation and constraints: the query decoded every exclusive
  flock PID before comparing dev/inode. The kernel prints signed PIDs and
  can report negative remote-lock owners; their process identity is outside
  this directory's authority.
- Alternatives considered: accept arbitrary signed PIDs as signal targets;
  fail the whole probe for another file; compare canonical printed strings.
  The first weakens process authority, the second hides valid later rows,
  and the third unnecessarily depends on field padding.
- Selected approach: select the lock kind, decode its file identity once,
  and compare dev/inode before parsing the PID. A nonmatching file is skipped.
  An unrecognizable file identity is logged without its contents and supplies
  no process authority. A matching identity still requires the strict typed
  PID and rejects conflicting writers; its numeric error retains the cause.
- Why this approach: scope strict process decoding to the only file the query
  can authorize. The kernel format and PID translation are recorded in
  [Linux fs/locks.c](https://github.com/torvalds/linux/blob/master/fs/locks.c),
  `locks_translate_pid`, `lock_get_status` and `locks_show`.
- Advantages, costs and effects: no new bound, dependency, wire call, signal
  or file mutation. Foreign negative/invalid PID and unknown-identity rows
  cannot mask a valid row before or after them. A matching invalid PID still
  refuses. Absence remains `NotVisible`, never proof that the OS lock is free.

File identity can also be invisible when visible `stat` identifiers differ
from the locked inode's superblock device or inode. Btrfs subvolumes assign
an anonymous device in `btrfs_getattr`
([source](https://github.com/torvalds/linux/blob/master/fs/btrfs/inode.c));
overlayfs may present identifiers distinct from its backing files
([kernel documentation](https://docs.kernel.org/filesystems/overlayfs.html)).
These are additional limitations beside PID namespaces and NFS/SMB lock
emulation. The query then falls back to the actual OS lock. Those filesystem
layouts have not been exercised here; the source reason does not establish
runtime coverage of btrfs, overlayfs, network filesystems or PID namespaces.


## Service path fidelity (slice 2f)

[Decision Log]
- Purpose and intent: ensure a newly installed unit starts this executable
  in the selected data directory, independent of its manager's cwd.
- Existing implementation and constraints: the installer owns both templates
  and leaves differing files alone. Paseo `restart.ts` 20–21, 118 retains
  a supervisor; Vessel instead launches the node as a direct user service.
- Alternatives considered: canonicalize paths, keep relative arguments, or
  silently replace unrepresentable OS bytes. Canonicalization requires
  filesystem access and changes symlink semantics; the others can select
  another directory or executable.
- Selected approach: reuse the installer and existing `RemoteCommand`
  quoting. Resolve new data and service paths with `std::path::absolute`;
  refuse non-Unicode or control characters before a service-file write.
  Centralize per-platform locations and computed undo guidance beside the
  installer; manual launchd load guidance uses the same POSIX value quoting.
  No new dependency or crate edge is needed.
- Why this approach: it preserves path semantics without resolving symlinks
  ([Rust documentation](https://doc.rust-lang.org/std/path/fn.absolute.html)).
  Systemd's command arguments expand dollars even inside quotes, while its
  Environment assignment treats them literally; only data arguments gain
  dollar escaping ([service rules](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html),
  [environment rules](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html)).
- Advantages, costs and effects: new units retain the selected path and
  saved PATH. Existing relative or expanding units differ and are preserved;
  adoption requires explicit removal and recreation. Unit creation and
  existing helper invocations still use the invoking OS user's credentials
  and search path. No new helper or other account's environment is reached.
  Windows retains the existing service-unavailable report. Real manager
  lifecycle behavior and macOS/Windows execution remain unverified.


The existing `write_once` policy also uses exclusive creation: an initializer
cannot truncate a service file created after an earlier existence check.
File I/O runs off the async worker; the platform reports retain their
existing text and order. The first concurrent creator wins; a differing existing file
stays unchanged. A failed initial write can leave an incomplete file for
explicit repair rather than replacing someone else's later edits. This
uses only the existing standard-library I/O and test fixtures.


Existing service files are compared as byte streams with bounded buffering,
not decoded into an unbounded String. A non-Unicode personal file is a
preserved difference; the comparison can also finish against an endless
reader. Only new validated templates are written. This changes installation
comparison behavior; no independent reporting-only refactor is included.


## Process-mechanism ownership amendment

ADR 0001's approved node-control amendment replaces the earlier no-new-edge
assumption for the following process-control phases only. The existing
store-facing `NodeLock` contract stays unchanged while the generic kernel
query moves to proc. Store-to-proc is added with that behavior-preserving
move; CLI-to-proc is added only with the real caller. No external dependency
or wire change is proposed. The separate descriptor/generation/namespace
verification phase must not grant signal authority from a kernel PID alone.
The documents-only contract added neither an edge nor process-control code;
the query move below implements its first approved edge.


## XML comment delimiter safety

[Decision Log]
- Purpose and intent: a home path must never close a generated plist comment
  or introduce another plist key.
- Existing implementation and constraints: the undo hint is present in the
  plist and CLI output. Non-overlapping pair replacement leaves adjacent
  hyphens when input contains longer runs; XML comments cannot contain such
  a pair or end with a hyphen ([XML rules](https://www.w3.org/TR/xml/#sec-comments)).
- Alternatives considered: omit the hint from every plist, refuse otherwise
  valid paths, or replace pairs repeatedly. Omitting changes every existing
  plist's comparison; refusing paths narrows valid OS paths; repeated passes
  obscure the invariant.
- Selected approach: emit comment characters once, inserting a space before
  a hyphen that would follow another, and after a final hyphen. Every normal
  undo hint remains byte-identical; a raw hint ending in a hyphen gains a
  separator. XML text values still use their separate encoder.
- Why this approach: neither adjacency nor a terminal hyphen can survive
  emission, independently of the length or surrounding input. Existing head
  assertions and normal service files need no expectation or format change.
- Advantages, costs and effects: no new Rust dependency, wire call or edge.
  The Unix regression test uses Python's standard-library `plistlib` in
  isolated, no-site mode to parse generated XML and compare the complete
  expected keys and values. This is a test-only interpreter prerequisite;
  no interpreter or Python code ships in the product. The test fixture uses
  the Python distribution's standard library, not an added package.
  Systemd comments cannot be ended by a hyphen sequence; their line boundary
  is protected by the existing control-character refusal before rendering.


## Local observation handoff extraction

The existing lock waiter's dedicated thread and borrowed oneshot receive
are separated from its file operation before adding another caller. This is
a behavior-preserving extraction: thread name, dropped-receiver diagnostic,
lock open/acquire/unlock order, result and source mapping, cancellation and
all existing tests remain unchanged. The diagnostic stays in its original
module through the completion callback, preserving its tracing target as
well as its message. The extracted private helper has only
the actual lock-release operation; later operation kinds arrive with their
callers. It reads no environment and owns no file-format or node policy.
The subsequent deadline fix is separate from this extraction (P2).


## Deadline-safe identity and preflight reads (slice 2g2)

[Decision Log]
- Purpose and intent: a blocked local read must not hold Tokio's blocking
  pool alive after the stop command's timeout has fired.
- Existing implementation and constraints: OS-lock completion already uses
  a detached worker and a borrowed oneshot receive; the preflight and stored
  identity read still use tasks the runtime joins when it exits.
- Alternatives considered: cancel an already-started blocking-pool task,
  add a second handoff, or check file type before opening. Cancellation does
  not interrupt that read; duplication splits one policy; a type check races
  with a replacement between checking and opening.
- Selected approach: reuse the extracted Pending policy for the two actual
  readers, with typed operation names and cause-preserving receiver errors.
  Each caller retains its own file rule and diagnostic.
- Why this approach: the command can stop waiting without making runtime
  shutdown join the blocked read. The OS-lock handoff, result consumption,
  existing tests and budgets remain unchanged; no replay cache is added.
- Advantages, costs and effects: no dependency, wire field or signal. Linux
  child-CLI tests replace node.id or the selected lock path with a FIFO,
  then check deadline failure and a still-Serving node after the child exits.
  The shared test budget is named once. This is not a wire capture.

The tests cover these two readers only. Existing book/project resolution,
connection credential reads and best-effort book pinning are separate paths;
this slice does not establish deadline-safe process exit for all file inputs.
Real macOS/Windows blocked-read behavior is unverified. Paseo's previously
stated PID-polling difference is unchanged.


### Process-query ownership move (slice 2p1)

The approved ADR 0001 edge is implemented without changing stop behavior:
`NodeLock::kernel_holder` still maps the same query and I/O cause, now from
proc through the store's restricted lock shim. Selected-file decoding,
bounds, errors, duplicate handling, fallback and owner bytes are unchanged.
The snapshot is still no signal authority. No force/restart caller, signal,
wire change or new external dependency is included in this move.


## Descriptor possession before process control (slice 2p2a)

[Decision Log]
- Purpose and intent: a kernel row's PID cannot identify a current holder
  by itself. Require that candidate's descriptor to report the selected
  locked file description before publishing the local snapshot.
- Existing implementation and constraints: the moved decoder already selects
  dev/inode before decoding a PID. The existing opaque snapshot grants no
  signal authority. Linux can retain a dead creator's number with an
  inherited flock, so mere PID existence is insufficient.
- Alternatives considered: trust the owner record, accept any open descriptor
  of the inode, or copy a second parser. None relates the candidate to the
  exclusive lock. Descriptor possession still does not settle generation
  or PID namespace; those checks remain a separate required phase.
- Selected approach: pin a read-only candidate proc directory, resolve its
  children through that retained descriptor and inspect only FD identities
  matching the selected file. Decode only their lock blocks using the same
  parser as the global table. Unrelated FD contents cannot poison the query.
  Byte and entry caps have one named constant each, used in errors and tests.
- Why this approach: inode equality alone does not prove that a descriptor
  owns the lock. FDinfo relates the actual file description to its lock;
  a retained proc directory does not reserve a numeric PID. An inaccessible,
  missing or ambiguous observation refuses without changing a lock/record.
- Advantages, costs and effects: no external dependency, wire field or signal.
  The snapshot retains its proc-directory reference, not a copy of the node
  lock. A live unrelated fake is refused while the test's original lock stays
  held. Generation/namespace proof and pre-signal rechecks remain mandatory
  before the later force entry point; this slice is not signal authority.

Primary source: Linux `fs/proc/fd.c` 21–67 obtains the task's actual descriptor;
`fs/locks.c` 2880–2913 checks its file description/owner before printing the
same lock record. The kernel's proc documentation says old proc descriptors
do not retarget a reused PID and do not reserve its number. The directory
replacement fixture tests ordinary descriptor anchoring, not real PID reuse.
No namespace/platform behavior is inferred from that fixture. Real non-Linux
and alternative filesystem behavior remains unverified.


## Generation and namespace bracketing (slice 2p2b)

[Decision Log]
- Purpose and intent: relate descriptor possession to one candidate generation
  and the invoking process's PID namespace before retaining the observation.
- Existing implementation and constraints: the descriptor query retains a
  read-only proc-directory reference, shares the selected-file lock decoder
  and grants no signal authority. The bounded record decoder can also read
  the generation and namespace hierarchy. An open proc descriptor does not
  reserve a numeric PID.
- Alternatives considered: compare numeric PIDs alone, trust start ticks
  alone, read the namespace of the proc view's init, or add a new dependency.
  Coincident numbers do not identify namespaces; start ticks alone do not
  establish lock possession; a normal account may not read the init's
  namespace link. Existing standard-library handles and metadata suffice.
- Selected approach: decode only the candidate PID and start ticks from a
  bounded anchored stat record, leaving command-name bytes opaque. Require
  the observer's NStgid hierarchy to contain exactly its own identifier,
  without another namespace level. Open and retain the observer's and the
  candidate's active PID namespace files; compare device/inode identities.
  Capture generation/namespace before the descriptor scan and compare again
  after it. Changed, missing, unreadable or malformed evidence refuses.
- Why this approach: the kernel emits NStgid from the proc mount namespace
  level through the task's level, so one level proves the observer's proc
  view agrees with its active PID namespace. This is stronger than numeric
  equality alone. Namespace file references retain their identities, while
  anchored stat reads and repeated capture detect changed observations.
- Advantages, costs and effects: no dependency, wire field, process signal or
  public PID getter. The snapshot retains two namespace references as well
  as the proc-directory reference until its last clone is dropped. A missing
  hierarchy or namespace facility is a refusal, never permission to signal.
  Candidate processes in another PID namespace are conservatively refused.

The real held-lock fixture exercises capture on Linux. The unrelated live
fake is also observed through a retained proc reference; after its owned
child exits, the old reference must refuse with the OS disappearance error.
Tests cover opaque
command bytes, missing and invalid numbers, changed generation, namespace
hierarchy ambiguity and actual namespace-file identity disagreement. These
are not a demonstration of real PID reuse or a child entering a PID namespace;
those scenarios remain unverified. The later force entry point must repeat
its proof immediately before each signal. No signal entry point exists in
this change, and the future final-check-to-numeric-signal race remains open.

Primary sources: Linux [proc status generation](https://github.com/torvalds/linux/blob/master/fs/proc/array.c)
emits the hierarchy relative to the proc view;
[PID namespaces](https://github.com/torvalds/linux/blob/master/kernel/pid_namespace.c)
distinguishes the active namespace from the namespace for future children;
[namespace files](https://github.com/torvalds/linux/blob/master/fs/nsfs.c)
retain namespace references. The existing
[proc descriptor limitation](https://docs.kernel.org/filesystems/proc.html)
still applies. Paseo's owner-PID/replacement checks and polling remain the
reference difference already recorded above; these added checks are Vessel's
explicit conservative floor.

## Service-file observations before shutdown (slice 2h)

[Decision Log]
- Purpose and intent: warn before shutdown when a personal or older service
  file may restart the node, without equating disk text with loaded state.
- Existing implementation and constraints: the installer owns both renderers,
  their locations, path validation and escaping. Paseo `packages/cli/src/
  commands/daemon/stop.ts` 44–57 budgets its shutdown connection and 95–98
  reports remote exit as unverified; `restart.ts` 20–21 and 118 retains a
  supervisor launch. Vessel's direct services have no such supervisor.
- Alternatives considered: execute a unit's command, trust its undo comment,
  parse every manager directive, or infer a loaded service from matching text.
  These either grant authority to editable text or confuse different evidence.
- Selected approach: reuse the current installer renderers and location rule.
  Match every byte against the invoking executable and the authenticated
  directory, decoding only the saved PATH needed to regenerate that template.
  A mismatch produces one fixed warning, without echoing the file or PATH.
  Older unconditional launchd agents get that same conservative warning;
  removal and recreation guidance remains in the README and initializer.
- Why this approach: a file's own executable or comment must not validate
  itself. Another CLI installation can therefore produce a conservative
  mismatch even when it talks to the same node. A full match still grants
  no manager authority: drop-ins, stale loaded configuration and the actual
  running job have not been inspected.
- Advantages, costs and effects: a missing file is harmless; unreadable,
  invalid-Unicode or oversized files refuse before the RPC with typed errors
  retaining safe causes. ServiceFile::MAX_BYTES is the single read cap,
  checked before Unicode decoding. The existing dedicated reader handoff
  bounds process exit when file opening blocks. No new dependency or call.

Only a directory whose stored identity matches the authenticated greeting
may supply this observation, including when draining. Remote connections,
other local directories and platforms without an installer service remain
RPC-only. Reads use the invoking OS account and its HOME/XDG location;
no other account's credentials or process environment are read. No file is
created or rewritten and no service manager is invoked. Matching text does
not authorize future manager control. That needs a separate implementation
and review of loaded-state evidence.

Tests owed with the implementation: current templates for both managers;
wrong executable/directory or edited bytes; older launchd policy; bounded
and malformed byte readers with no file contents in errors; isolated CLI
warnings, invalid-file refusal before shutdown, a draining live session,
and a blocked service FIFO with child exit. Stand-in manager executables
must record zero invocations. All fixtures isolate HOME and XDG before
service lookup. Actual manager behavior and macOS/Windows execution remain
unverified by Linux fixtures.

## Cancellation-safe settings publication

[Decision Log]
- Purpose and intent: cancelling an RPC must not let its still-running atomic
  file replacement overtake a later settings change and leave disk and runtime
  disagreeing. Shutdown must also wait for any write already admitted.
- Existing implementation and constraints: a borrowed serialization guard lived
  in the RPC future while spawn_blocking retained the actual writer. Dropping
  the future released the guard without stopping the filesystem operation.
  The original validation, later-file preservation, owner-only atomic writer
  and saved-value precedence remain the required path.
- Alternatives considered: abort the blocking task, hold a guard only inside
  its filesystem closure, or detach an untracked task. Blocking I/O cannot be
  cancelled once started; the second option releases serialization before
  application; the last lets shutdown report completion before publication.
- Selected approach: acquire an owned settings guard before writing and move it
  into an independently scheduled task through I/O and runtime publication.
  Its tracker token precedes the admission check, closing the race with tracker
  shutdown. A cancelled caller drops only its result waiter. After closing and
  joining RPC connections, node teardown closes writer admission and joins the
  writers before closing the store. Late mutations receive typed Conflict.
- Why this approach: one task owns the complete state transition; no second
  writer, database, setting, wire version or dependency is introduced. Paseo's
  daemon-config-store.ts 655–659 serializes operations in its promise queue;
  Vessel explicitly owns the spawned task and shutdown join required by Rust
  future cancellation. No additional parity claim about Paseo shutdown is made.
- Advantages, costs and effects: an admitted write may complete after its caller
  disconnects, and a blocked filesystem can delay node shutdown. Two one-shot
  regressions pause the real blocking writer before file replacement, cancel
  its caller, and check serialization, final disk/runtime agreement and shutdown
  admission/join. No sleeps or replacement fake writer are used. Actual service
  managers, other operating systems and service-account permissions remain
  separate from these isolated fixtures.

## Proposed queued settings ownership and recovery settlement

[Decision Log]
- Purpose and intent: accepted settings work must remain node-owned before lock
  acquisition as well as during real file I/O, publication and policy cleanup.
  A caller leaving must not let a later On bypass an accepted Off obligation.
- Existing implementation and constraints: the retained writer protects an
  acquired saved guard; the new recovery field also needs bounded queued
  ownership, same-value cleanup retry and one pre-Store boot snapshot. A second
  FIFO/disk model would split order and responsibility.
- Alternatives considered: caller-owned mutex wait, an outer field queue,
  independent compatibility writer or callback trait. These abandon accepted
  work, duplicate capacity/ordering, split disk ownership or add an unnecessary
  open extension point.
- Selected approach: one finite non-waiting admission channel and independent
  consumer within the canonical owner. A concrete RecoveryApply over Policy and
  Store runs inside the same saved guard at before-mutation, before-write,
  failure, saved-publication and actual cleanup/no-op points. The opaque Document
  keeps the typed recovery patch without becoming a wire serialization model.
- Why this approach: one admission linearization, one resource-bound owner and
  one file/publication order serve all settings namespaces. Startup hands the
  same validated snapshot to Store candidate capture and the settings owner.
  Shutdown seals effects/admission, then joins accepted settlement before final
  runtime/Store close; no dropped caller or reply pretends cleanup completed.
- Advantages, costs and effects: overflow is a typed non-admission; failed cleanup
  can block later mutations and shutdown. Off preserves its saved/effective
  state while debt remains, On stays paused until actual apply, and unchanged
  explicit operations retry debt without another file replacement. New lifecycle
  fences/order and recovery policy behavior require their joint tests/gate; this
  proposal approves no activation by itself. The named bound, concrete seam,
  diagnostics and required tests are in shared-settings-operation-owner.md.

## Proposed private saved settings envelope

[Decision Log]
- Purpose and intent: separate the saved disk model from the generic settings
  wire patch before adding private service consent. Sharing a file must not
  grant a generic patch or a Read reply authority over that private state.
- Existing implementation and constraints: Store owns settings.json's atomic,
  owner-only and set-aside rules; Node Settings owns saved state and publication.
  The prerequisite repair retains its mutation guard/tracker through I/O and
  publication despite caller drop. The disk API and saved
  mutex currently hold SetNodeSettings directly.
- Alternatives considered: a second private file/writer, service consent in the
  generic patch, or an untyped placeholder. These duplicate lifetime policy,
  expose the wrong mutation surface, or invent an unreviewed authority schema.
- Selected approach: introduce a private-field SavedNodeSettings envelope for
  the same Store/Node boundary, with a generic projection/replacement that
  preserves today's typed fields. Keep serialization at the disk edge and Read
  replies as NodeSettings. The initial model separation preserves current
  canonical file bytes and unknown/default/set-aside/no-op behavior.
- Why this approach: one settings owner keeps concurrent generic and later
  dedicated mutations serialized without a second writer. Shared typed generic
  fields avoid an independent complete CLI key list. This is Vessel authority
  separation, not an assertion of Paseo's file schema; Paseo's mutation queue
  reference and the required compatibility tests are in the saved settings plan.
- Advantages, costs and effects: disk and wire models can evolve separately,
  at the cost of explicit projection and compatibility fixtures. A later private
  services slot requires its own qualified canonical schema/bounds and dedicated
  Admin setter; generic set/reset/reload cannot become a route to that setter,
  and Read cannot expose it. No slot payload or service activation is introduced
  by this proposal. Implementation builds on the reviewed retained writer and the
  announced base; changed-tree checks and review still gate publication. See docs/plans/saved-node-settings.md for the dependency
  boundary and remaining proof.


A namespace with both public choices and private metadata has one disk owner,
not two flattened fields with the same key. Its generic projection/merge must
preserve the private record; future namespaces are introduced under their own
reviewed ADRs. This separation does not freeze a wire-patch flattening strategy,
an opaque JSON placeholder or a public callback API. The saved settings plan
records why same-key flattening is insufficient before those schemas join.


The initial implementation keeps a non-Copy opaque Store envelope, with no
public serde/TypeScript implementation, and an internal transparent document
that delegates to today's generic codec. A complete generic projection and a
consuming candidate update are the only cross-crate data accessors. Node's same
owned saved guard/task now carries that envelope. The blocking writer returns
the owned candidate after successful persistence, rather than relying on Copy
or cloning the complete document again for the blocking thread. File path,
set-aside behavior, no-op ordering and write-before-apply are unchanged.
Canonical-byte/semantic fixtures and a no-op original-byte fixture are added;
source/compile/runtime evidence remains pending. No private schema or setter is
introduced, and this model separation is not service activation qualification.
