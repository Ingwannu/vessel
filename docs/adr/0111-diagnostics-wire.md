# ADR 0111: The diagnostics wire: an agent's diagnostic, the rest of the node report, and the node's log

Status: proposed (number reserved by the coordinator on 2026-11-18 for the overnight diagnostics packet:
TRACKER 568, 67, 87, 569 and 567); impl U. The protocol number, 33
(`DIAGNOSTICS_WIRE_PROTOCOL`), was assigned at the node-line merge. Extends ADR 0074 (bounded node
diagnostics); supersedes only its "no request runs `--version`" sentence for the two tool versions below.

## Context

Paseo (source `~/refs/paseo-latest/packages`, read 2026-11-18; S = `server/src/server`, A = `app/src`,
P = `protocol/src`, C = `client/src`, D = `desktop/src`) has three diagnostics a person reaches from
Settings:

- A provider's diagnostic (TRACKER 568, 67). `provider_diagnostic_request {provider, requestId}` answers
  `provider_diagnostic_response {provider, diagnostic, requestId}` (P `messages.ts` 1864, 6402; permission
  `daemon.read`), or `rpc_error` `provider_diagnostic_failed` "Failed to get provider diagnostic: …"
  (S `session/provider/provider-catalog-session.ts` 455–484). `ProviderSnapshotManager.getProviderDiagnostic`
  (S `agent/provider-snapshot-manager.ts` 573) runs two things together: the provider's own rows
  (`getDiagnostic()`, bounded by `max(refreshTimeout, 120 000 ms)`, "Timed out collecting <label>
  diagnostic after <ms>ms") and a forced refresh of that provider's catalogue on the global target, whose
  result is published to every client. The text is the provider's label, then `  <label>: <value>` rows
  (`providers/diagnostic-utils.ts`), then `  Models: <n|—>` and `  Status: Ready|Error: …|Unavailable|
  Loading`. Rows: command source, configured command, the daemon's PATH and shell, PATH matches and
  `which -a`/`type -a` output, binary and resolved path, `--version`, sign-in (`Auth`), and for an ACP
  agent the probe's phases (`ACP spawn`, `ACP initialize`, `ACP session/new`, `ACP cleanup`, 20 s each).
  An unknown provider is text, not an error ("Error: Provider X is not configured").
  The app (A `components/provider-diagnostic-sheet.tsx`, from Settings › Providers, a provider's sheet,
  footer button "Diagnostic" with a FileText icon) asks on open (client timeout 180 000 ms) and shows
  "Running diagnostic..." with a spinner, then the text in a monospace block with Copy ("Copy
  diagnostic", toast "Copied diagnostic" / "Failed to copy diagnostic") and Refresh ("Refresh
  diagnostic" / "Refreshing diagnostic"); "No diagnostic available" when empty; an error's message, else
  "Failed to fetch diagnostic", in the block.
- The catalogue's scope (TRACKER 87): one catalogue per cache key (`getCatalogCacheKey`; Claude and Codex
  answer `"host"`, so every folder shares one), a 120 s refresh deadline (`agents.catalogRefreshTimeoutMs`,
  `PASEO_PROVIDER_REFRESH_TIMEOUT_MS`), states `loading` → `ready` / `error` (with its message) /
  `unavailable`, and the diagnostic's forced refresh above.
- The daemon's report (TRACKER 567): `diagnostics.request` → one text block (S `session/daemon/
  diagnostics.ts`), sections Paseo diagnostics, Session observations, Daemon process, Runtime config,
  System (OS, platform, CPU cores, load average, free memory), Disk (path, free), Agents (total, by
  provider, by lifecycle, pending permissions), Workspaces (projects, workspaces, by kind), Providers
  (total, available, unavailable), WebSocket runtime metrics, Tools (`git --version`, `gh --version`,
  3 s each, 512 bytes) and the daemon log's last 80 lines (64 KiB), redacted by pattern. The app's
  diagnostic sheet (A `components/app-diagnostic-sheet.tsx`, Settings › Diagnostics, "App diagnostic",
  "Collect connection, daemon, provider, desktop, and log details", "Run") shows a row per step (Client,
  Desktop, each host: running / done / failed) and appends each connected host's report, "Status:
  unsupported by this daemon" or "Status: host is not connected" otherwise.
- The daemon's log (TRACKER 569): `daemon.log` in Paseo's home, rotated by the supervisor (10 MB, 3 files).
  Only the desktop app shows it (IPC `desktop_daemon_logs`, the last 100 lines, D `daemon/daemon-manager.ts`
  439, 494): Settings › Host's daemon card has "Log file" (the path, or "Log path unavailable"), "Copy path"
  ("Copied" / "Log path copied.") and "Open logs", a dialog "Daemon logs" (testID
  `managed-daemon-logs-dialog`) with the path and the lines, or "(log file is empty)". A host reached over
  the network has no way to show its log. Process diagnostics (S `process-diagnostics.ts`: pid, parent,
  uptime, memory) feed the worker's own log lines and the runtime metrics.

Vessel has, today:

- `node.diagnostics` (ADR 0074, protocol 24): a typed, content-free report (node, daemon process, store,
  agents, sessions, listeners, relays, recent failed calls), rendered by the CLI as Paseo's text. Process
  diagnostics are in it (pid, parent, resident and virtual memory, uptime). No page or app shows it: the
  native app's diagnostic (`view::diagnostics`) reports what the app holds and asks no machine; the page has
  no diagnostic.
- An agent's facts over `agent.status` and `agent.list` (path, version, structured support, sign-in) and
  its catalogue over `harness.options`, from `probes`: one slot per agent and profile, an answer reused for
  ten minutes while the agent's version is unchanged, a failure never reused, a probe bounded by the
  handshake timeout (60 s). Nothing forces a new probe.
- The node's log: a background node writes `vesseld.log` in its data directory (rotated to `.1` above
  10 MiB when the next one starts); a launchd service writes `logs/vessel-service.log`; a systemd service
  writes to the journal; a node started in a terminal writes to it. Nothing reads the log back.

## Decision

[Decision Log]
- Purpose: Paseo's three diagnostics, on every surface Vessel has: an agent's diagnostic with its forced
  catalogue refresh, the node report's missing sections and its readers (the page and the native app's
  diagnostic), and the node's log in a dialog.
- Existing assets reused: ADR 0074's report, collector, admission, bounds and CLI text; `probes` (one
  slot per agent and profile), `readiness` and harness discovery (`Lookup`) for the agent's facts;
  `DataDir` for the log's place; the native app's diagnostic dialog and the staged diagnostics adapter
  (`link`, row 287); the page's settings rows and dialogs.
- Alternatives considered:
  - Paseo's text from the node (one `string`, redacted by pattern). Rejected for the same reason as in
    ADR 0074: a typed reply cannot carry a field nobody reviewed, and pattern redaction misses what it
    does not foresee. The text is rendered by each client from typed facts, as `node.diagnostics` is.
  - Paseo's rows as they are: the daemon's PATH, its shell, `which -a` and `type -a` output, `opencode
    auth list` output. Rejected: environment values and a shell's output are what ADR 0074 keeps out. The
    search path's matches are kept (paths only, the home folded to `~`), since "which binary runs" is the
    question those rows answer.
  - The log as a section of the report (Paseo's "Daemon log tail"). Rejected: the report is read-only and
    made to be pasted elsewhere; a log line can hold a path, a prompt's words or an agent's error. The log
    is its own call, for the machine's administrators, and is not part of the copied report.
  - The log only where the app runs beside the node (Paseo's desktop IPC). Rejected: Vessel's page and app
    reach machines over the network, and an administrator of a machine can already open a shell on it; a
    bounded call gives every surface the same dialog for every machine.
- Choice: one protocol, three parts, below. Each client renders Paseo's text: one Rust renderer in
  `vessel-proto` (the CLI and the native app) and one in TypeScript (the page), held to the same output by
  shared fixtures (`web/src/wire/fixtures/`: JSON a node sends and the text the Rust renderer makes of it;
  the text files are rewritten by the gate's `ts` step, as `proto.ts` is, so a change of the text is a
  reviewed change of a file).
- Costs: listed at the end.

### 1. An agent's diagnostic (TRACKER 568, 67)

- `agent.diagnostic {agent, profile?}` → `AgentDiagnostic` (`Read`, unary, forwardable as `agent.status`
  and `harness.options` are; an agent caller is refused as for `agent.status`).
- The node, in one call:
  1. reads the agent's facts as `agent.status` does (running its sign-in check when none is recent): where
     it is (`Installed`), its version, whether it runs conversations here (structured support), whether it
     is signed in;
  2. lists every match for the agent's program on the node's search path (at most 8, home folded to `~`);
  3. forces a new probe of the agent under that profile (part 2) and reads its steps from what the probe
     already records: spawn, initialize (the handshake, with the agent's own version), the session the
     probe opens to read the options, each ok, failed (the options' error when there is one, as
     `harness.options` gives it) or not reached, and how long the probe took. An agent that runs no
     conversations here starts nothing: every step is "not reached".
- The reply holds: the agent and profile, its title (Paseo's label; an added agent's "<label> (ACP)"),
  the command's source (built in, a profile's, an added agent's) and program (the arguments of an added
  agent, as `agent.list` already gives them; never a profile's environment), the facts above, the matches,
  the probe's phases, the model count (when the probe succeeded) and the catalogue status: `ready`,
  `error {message}` (the probe's error, as `harness.options` already returns it), or `unavailable` (not
  installed, or no conversations here). No `loading`: the call answers once the probe ends.
- An unknown agent or profile is `not_found` (Paseo answers its text "Provider X is not configured"); the
  dialogs show the error's message where the text would be, as Paseo's sheet shows a failed fetch.
- Bounds: the probe's own handshake timeout (60 s) and the sign-in check's own; clients wait 120 s (Paseo's
  180 s covers its 120 s refresh). Strings bounded (title 128 bytes, each reason 4 KiB, 8 matches).
- Text (Paseo's): the title, then `  Command source`, `  Configured command`, `  Search path matches` (later
  matches on lines of their own, indented four spaces, as Paseo's PATH matches), `  Resolved path`,
  `  Version`, `  Conversations`, `  Auth`, `  ACP spawn`, `  ACP initialize`, `  ACP session/new`,
  `  Probe took`, then `  Models: <n|—>` and `  Status: Ready|Error: …|Unavailable`.
- No CLI command: Paseo has none for it, and the CLI's `vessel agent status` already prints the facts.

### 2. The catalogue's scope and its refresh (TRACKER 87)

- Vessel's catalogue key stays the agent and its profile: a profile is an account, and an account offers
  its own models (Paseo's key is per folder unless the provider names a shared one; Vessel's probes run in
  one probe folder, so every folder already shares one catalogue, as Paseo's `"host"` key does).
- Timeout: a probe is bounded by the handshake timeout (60 s); Paseo's 120 s refresh deadline has no
  separate setting here.
- States as a page shows them: asking (the card says so while it waits), the options, or the probe's error
  with its message; "not installed" and "no conversations here" are the agent's status. Unchanged.
- The diagnostic's refresh (`Probes::refresh`): it waits for the key's slot like any caller, then probes
  and keeps the answer, success or failure, in the slot. A later `harness.options` takes that answer as
  it would any recent one (a failure is never reused, as before). Vessel has no catalogue subscription,
  so "published" means the next caller sees it.
- A floor (`REFRESH_FLOOR`, 5 s; the node reviewer's note on row 334): `agent.diagnostic` is `read`, as
  Paseo's is `daemon.read`, so a paired viewer pressing Refresh could keep the node starting an agent back
  to back. A refresh takes, instead of probing, the answer of a probe that finished while it waited for the
  slot, or of a refresh of the same version that finished under 5 s ago, whatever it found. One agent
  start per key per 5 s at most; the cost is a person who signs in and presses Refresh within 5 s seeing
  the previous answer once, and such a reply's "Probe took" is how long this call waited (near zero).

### 3. The rest of the node report (TRACKER 567)

`DiagnosticReport` gains five sections, each optional on the wire (sent only to peers at the new
protocol; older peers keep today's report, since its fields are closed to unknown ones):

| Section | Facts | Source |
|---|---|---|
| System | CPU count, load average (1/5/15 min, hundredths), memory available and total | `std::thread::available_parallelism`; Linux `/proc/loadavg` and `/proc/meminfo` (16 KiB reads, as `/proc/self/status` is); unsupported elsewhere |
| Disk | available and total bytes of the data directory's file system | `statvfs` through `rustix` (already a store dependency, `DataDir::space`); never the path |
| Workspaces | projects with a workspace; active workspaces, of which worktrees the node made; archived ones | the workspace registry |
| Activity | live sessions per built-in agent, added agents' together, and those waiting on a permission | the runtime (`SessionRuntime::diagnostic_activity`) |
| Tools | `git` and `gh`: found, missing, timed out or failed, and a numeric version | `--version` with a 3 s timeout and a 512-byte read, three numbers kept (ADR 0074's version rule) |

Paseo's websocket metrics are not collected (Vessel keeps no such window); its "Session observations"
are the existing recent-failures window. The tool versions supersede ADR 0074's sentence that no report
runs `--version`: two fixed programs, no arguments from anywhere, numbers only.

The report's readers:

- The CLI keeps `vessel node diagnostics`; its renderer moves to `vessel-proto` (`diagnostics::text`) so
  the native app renders the same text. Its words change from Rust's debug names to the shared rule (a wire
  name with `_` read as a space; a tagged value as its tag, then its fields in parentheses), and its sections
  are apart by a blank line, as Paseo's.
- The page gains Settings › Diagnostics (under About): "App diagnostic", "Collect connection, machine,
  node and agent details", "Run", a dialog "App diagnostic" with Copy and Refresh, a row per step (Page,
  each machine: running / done / failed), then the text: the page's version, browser and machine count, and
  per machine its name, protocol and version and its report; "Status: unsupported by this node" below
  protocol 24, "Status: machine is not connected" when it is not, the call's error otherwise. The page has
  no desktop half, so Paseo's desktop sections have no counterpart.
- The native app's diagnostic now asks each connected machine. Before, it asked none.
  - The calls: `link::diagnostics`, one task per call, answered as an event that carries the machine and the
    page's ticket. Failures cross as codes only, the Devices page's rule.
  - The run's state: `view::diagnostics::Run`.
  - The app diagnostic's dialog: Paseo's progress card while any machine is asked, then only the text. The
    first progress row is "App" for Paseo's "Client" (the coordinator's word). Copy and Refresh are the
    title row's icon actions.
  - The diagnostic texts (the app's and an agent's) keep each line whole and scroll sideways, as Paseo's
    `ScrollableCodeSurface` (`horizontal` by default) and the page do. Node logs wraps, as Paseo's plain
    `Text` does (the coordinator's ruling on row 337).
  - An agent's "Diagnostic" sits beside its details on Settings › Agents (from 33). Its sheet is
    `view::diagnostics::Sheet`. A failure takes the text's place in the app's words for its code.
  - Overview's "Node" section holds the "Log file" row and "Node logs" (from 33).
    `view::diagnostics::log_contents` keeps the same truthfulness rule as the page.

### 4. The node's log (TRACKER 569)

- `node.log {lines?}` → `NodeLog` (`Admin`, unary, not forwardable: administration of a node is done
  there, as for `node.stop`).
- Which file: on Unix, the file this node's standard error is (compared by device and inode with
  `logs/vesseld.log` and `logs/vessel-service.log` in its data directory, through `/dev/fd/2`); elsewhere,
  `logs/vesseld.log` when it exists. An old file a node no longer writes is never taken for its log. A node whose standard error is a terminal answers `terminal`;
  anything else that is not one of those two files (the journal, `/dev/null`, a redirect, a rotated
  `vesseld.log.1`) answers `elsewhere`, and the page says only that the log is not a file the node keeps. The path is shown with the home folded to `~`.
- Bounds: the last `lines` (default 100, Paseo's; at most 1000), from at most the last 64 KiB of the file,
  each line cut at 4 KiB; the reply says when it was cut. A line is sent as text: a terminal's escape
  sequences (colours) and other control characters but a tab are dropped. At the source, the node's own
  logging (`daemon_logging`) and its fixtures write colours only to a terminal, so a background node's
  `vesseld.log` (its standard error, opened by `attach/start.rs`) holds none. An older build's file or a
  wrapper's may still hold them, and the page drops them again (`data/log-lines.ts`).
- Process diagnostics stay in the report (part 3 adds nothing to them): the pid, parent, memory and uptime
  ADR 0074 collects are Paseo's `ProcessDiagnostics`.
- Clients, with Paseo's words: a machine's settings have "Log file" (the path, or where the node writes
  instead, or "Log path unavailable" when the call is refused), "Copy path" (toast "Log path copied.") and
  "Open logs". The "Node logs" dialog is Paseo's logs modal: the path (or "Log path unavailable" when the call
  fails) above the contents, 12 px apart, the contents wrapped, muted, 12 px monospace on 18 px lines.
  "(log file is empty)" stands only for a file with no lines. On a failed call the call's error takes the
  contents' place (Paseo's `rowError` colour). Paseo opens its modal once the logs have arrived, so it never
  shows a failure there; the page opens the dialog first, so it must say what happened. The row asks for one line to learn the path; the dialog asks for the default 100.
- Settings › Diagnostics' mark is Paseo's Stethoscope (lucide, on the set's 16-unit grid). The app
  diagnostic's dialog shows its steps until the text exists, then only the text, as Paseo's sheet does.
- An agent's card (Settings › Agents) gains Paseo's "Diagnostic" button (a file mark) beside its title,
  on a machine at protocol 33: a dialog "Diagnostic" asks when it opens and on Refresh, shows "Running
  diagnostic..." until the first answer, then the text, with Copy ("Copied diagnostic").

### Protocol and order

- `DIAGNOSTICS_WIRE_PROTOCOL` (assigned 33): `agent.diagnostic`, `node.log`, the report's new sections. One
  node row, then the page row, then the native row, each with its frames.
- The page reads the number from the generated wire constants (ADR 0017's `PROTOCOLS`).

## Costs

- A diagnostic starts the agent once (its probe), as opening an agent's card does; asking twice probes
  twice.
- Two short processes per report (`git`, `gh`), bounded.
- The log call reads at most 64 KiB; a log line can still hold anything the node wrote, which is why only
  administrators get it and why it stays out of the copied report.
- Three renderers of one text (Rust for the CLI and the native app, TypeScript for the page), held together
  by fixtures.

## Tests

- `vessel-proto`: the new types' wire forms and bounds; the text of an agent's diagnostic and of a full
  report against fixtures shared with the page's tests; the report's new sections absent below the new
  protocol.
- `vessel-node`: `Probes::refresh` replaces a fresh answer and its result is what `harness.options` reads
  next; refreshes back to back share one probe until the floor passes; e2e `agent.diagnostic` for a ready fake agent, a missing one, an unknown one and one whose probe
  fails; the report's sections on Linux; `node.log` for a background node's file, a missing file, the
  line and byte bounds, and refusal without `Admin`.
- The page: the dialogs' states (running, text, error, empty, copy, refresh) and the Settings rows; e2e on
  the fleet.
- The native app: the same states, captured.
