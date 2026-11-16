# ADR 0031: Terminals beside sessions, a project's scripts and services, and the ports they open

Status: accepted, 2026-11-12 (milestone 11, wave 2, track C,
`docs/plans/backend-m11.md`, "Track C: contract" and its review; the
wire is protocol 11, `crates/vessel-proto/src/{terminals,scripts,ports}.rs`;
the runtime `crates/vessel-session/src/terminals/`; the node's part
`crates/vessel-node/src/terminals/`). Amends ADR 0027's digest.

## Context

agent-orchestrator, GOAT and Paseo each open shell terminals beside an
agent's session that outlive the page, run a project's named scripts and
long-running services from their interface, and find the ports those
open; Paseo also lets agents create, read and type into terminals. Until
protocol 11 Vessel had one terminal only: the agent's own, a session in
terminal mode (ADR 0003), carried by `vessel-pty`, the session runtime's
ring and its subscription frames, with bounded memory and resumption by
sequence number.

Questions:

1. Is a shell terminal a kind of session, or a thing of its own?
2. What runs, where, with which environment, and what bounds it?
3. How does a repository's script come to run, and what does an
   approval given before scripts existed cover?
4. How is a service kept running, and stopped?
5. How are ports found, how often, and what is done with them?
6. What may an agent do with a terminal, and what stops it using one to
   leave what it was given?

## Options

**What a terminal is**

1. A session whose spec runs a shell instead of a harness.
2. A thing of its own with a transport of its own (agent-orchestrator's
   `/mux`, GOAT's and Paseo's terminal streams).
3. A thing of its own carried by the sessions' terminal transport: the
   same records, frames, resumption and pages, under its own id.

**Scripts' approval**

1. A digest of its own per script, approved one by one.
2. The project's one digest grown to cover scripts, every earlier
   approval invalidated (a new version line for every file).
3. The one digest grown only for a file that names a script: a file
   without one keeps its version 1 digest.

**Services**

1. Run once, as agent-orchestrator's preview and Paseo's services
   (neither restarts).
2. Restarted on failure, backing off, with a ceiling.

**Ports**

1. Assign each service a port (Paseo's `PASEO_PORT`) and report that.
2. Find the listening sockets of the processes Vessel started, as GOAT's
   scanner does for every listener.

## Decision

**1. A terminal is a thing of its own, carried by the sessions'
terminal transport (option 3).** As a session (option 1) a shell would
be a card on the board with no turns; in every session list, where a
protocol 10 client cannot decode a spec without a harness, so its whole
`session.list` would fail unless the node hid shells from it; counted in
an agent's eight sessions; offered export, import, archive, resume and
marks; given an agent credential and `vessel mcp`; and written to the
store, so every byte a person's shell printed (`env`, `cat .env`) would
be kept on disk under the sessions' retention. A transport of its own
(option 2) would be the second terminal path the existing one makes
unnecessary. So a terminal (`TerminalId`) is listed and limited apart
from sessions, kept in memory only, and carried exactly as a terminal
session is: `vessel-pty` runs it; the session runtime's ring keeps its
scrollback and broadcasts it; its log is `EventRecord`s numbered from
zero (`terminal` output, `status`, `vessel:` notices); `terminal.subscribe`
answers with the same `ServerFrame::Event` and `SubscriptionEnded`
frames, `lagged { resume_from }` included, and `terminal.read` with the
same `EventPage`; input is the same `TerminalInput`. The page's terminal
view draws one as it draws a terminal session, and the bridge follows
one as it follows a session. A terminal adds no event: a service's state
is typed on its `TerminalInfo` and read by its revision, and its log gets
a `vessel:` line for a person (a new `SessionEvent` variant could never
reach an older client, which cannot ask for a terminal's log, and would
add an arm to every exhaustive match over the session's events).

**2. What runs.** The user's shell, found once by the binary (library
code never reads the process's environment): `$SHELL` when it names an
executable file, else the account's shell, else `/bin/sh`, with `-l` on
macOS only (as its Terminal starts shells); on Windows `pwsh.exe
-NoLogo`, `powershell.exe -NoLogo`, `%ComSpec%`, `cmd.exe`. It starts in
its place's folder (a session's, or a project's folder or a folder in
it), with the node's environment less every credential of Vessel's
(`VESSEL_*TOKEN`, `VESSEL_*SECRET`, `VESSEL_AGENT_SESSION`,
`VESSEL_NODE_SOCKET`: present only when the node was started inside
another session's process tree, and then also a routed harness's
`ANTHROPIC_BASE_URL` and `ANTHROPIC_AUTH_TOKEN`), plus `TERM` and
`COLORTERM`. A
session's agent credential and model token go to its harness only. 64
live terminals a node, 8 a session, 16 a project folder, 4 an agent; 32
ended ones kept; 1 MiB of output and 4,096 records of scrollback each,
the oldest dropped first (`oldest_seq` moves; a reader from before it
starts there), and a live follower at most 64 records behind before it
is told it lagged. On Linux a shell's session is swept when it ends, so
the jobs an interactive shell puts in groups of their own end with it;
elsewhere they are told `SIGHUP`, which bash and zsh pass on.

**3. Scripts (option 3).** `vessel.toml` gains `[scripts.<name>]` and
`[services.<name>]` (`command`, optional `dir`). A file that names
neither keeps its version 1 digest, so an approval given in protocol 10
still covers exactly what it showed; a file that names one is digested
as version 2 (setup, teardown, then every script's name, kind, command
and folder), which no earlier approval matches, so a first script, and
every later change to one, asks again. One approval per set, as ADR 0027
has it (option 1 would add a list of approvals per project for no gain:
a person reads the file as a whole). A protocol 10 client is shown such
a set as `invalid`, so it never holds the digest of something it cannot
display; the protocol 11 page's approval view must show the scripts, or
refuse such a set, before it is joined (a condition on track B-page). `script.run` reads the file in the run's root as it is, runs the
named command only from the parse whose digest is approved, through the
platform's shell as setup does, piped, standard input closed, as one
process group (a job object on Windows), so grandchildren end with it.

**4. Services (option 2)**, because the owner asked for it and a dev
server that dies on a syntax error should come back when the file is
saved: a restart after 1, 2, 4, 8, then 16 s; a run of 60 s resets
the count; the sixth failure without one gives up. An exit with 0 is not
restarted. The approval is checked again before every restart. Each
change is on the run's `TerminalInfo` (typed) and in its log (a
`vessel:` line). A person's restart of a live run runs it again in
place; an ended run's log is closed, so it runs again in a new terminal.
Stopped by a person, its session's deletion or removal by retention, or
the node's stop; not by its session's end, which `session.resume` can
undo in the same folder. A live terminal in a worktree or a project keeps
it from being removed, as a live session does.

**5. Ports (option 2).** The node walks the process trees of its live
sessions and terminals and reports their TCP listeners, with the address
(so loopback is told from every interface), the process and the owner:
`/proc` on Linux, `ps` and `lsof` on macOS, `netstat` and one PowerShell
query on Windows, none of them privileged. At most one scan every 3 s,
only within 60 s of someone asking, on the blocking pool, bounded in
processes, descriptors, time and output. Nothing is opened, forwarded or
proxied: how a page reaches a port is the preview track's decision, and
goes through the bridge, on an origin other than the page's and with no
credential of the bridge (a previewed app is repository code). Assigning
ports (option 1) would make every service's command depend on a variable
Vessel invents; it is left for the preview track if a preview needs a
known port before the service binds.

**6. Agents.** An agent's own shell tool may be sandboxed (Codex's
sandbox, Claude Code's sandbox and allowlists); a terminal the node
starts is not. So an agent may open a terminal only where a person
marked `agent_terminals = true` (off for every default profile) on both
its own session's profile and the profile of the session it opens it
beside: its own, or one it may operate (ADR 0013's ownership, across
nodes too; its own node checks its own profile before passing the call
on). Both, because a person allows terminals for one harness's sandbox:
an agent without the switch must not get a shell by starting a session
with a profile that has it. At most 4 at once; it uses only terminals
it made, and they end when its own session ends or the session they are
beside does (watched from the start, before the call answers, so a
caller that stops waiting leaves nothing running); `port.list` shows it only
its own; scripts are refused to it. What stops it leaving what it was
given through a terminal: a person turned it on knowing the agent's own
sandbox; it starts in the session's folder, as the same user; it holds no credential, so it cannot act
through the node as the agent or the node; the agent never reaches a
person's terminal (an `ssh` login, a `sudo` timestamp, another folder)
or another agent's; each write is a tool call of its harness, held to
the session's permission mode as its own shell tool is (Vessel does not
pre-approve its MCP tools, and marks the write destructive and
open-world); its terminals are counted, attributed, and end with its
session. A process of the node's user can still
read the node's files and token: attribution and least privilege, not
containment (ADR 0013).

## Consequences

- Protocol 11: thirteen calls (`terminal.*`, `script.*`, `port.list`),
  a `scripts` field on `ProjectCommands::Listed` and a stand-in for older
  clients; no event. `profiles.toml` gains `agent_terminals`.
- Terminals end with the node and are not kept on disk; the store's
  schema is not raised (schema 11 was set aside for this track and is not
  used).
- On Windows a shell's own detached children can outlive it (the pseudo
  console gap of ADR 0003's terminal sessions), and on macOS a background
  job that ignores `SIGHUP`; scripts and services are in job objects on
  Windows and cannot. On Unix a process that left its group on purpose
  (`setsid`) outlives a script, and if the node crashes its groups run on
  (a service keeps its port).
- Scripts run piped, not in a PTY (Paseo and GOAT use one): no colour
  from programs that check for a terminal, late output from those that
  buffer when piped. No readiness or health probe: `running` means
  started.
- A script runs whatever the repository's own scripts say on that branch
  (`npm run dev` runs `package.json`'s script): ADR 0027's limit holds.

## Amended when wave 2's first half was joined (2026-11-12)

Tracks B2 and C were built apart; where they meet was decided in the
node's join (plan, "Integration 2 (node)"):

- **A start holds its worktree.** A terminal or a script's run whose
  folder is in a worktree takes that worktree's "starting" count (ADR
  0030's, `Workspaces::hold_folder`) before its reservation and its
  spawn, and gives it back once registered; a removal claims the worktree
  first and then reads what runs where (live sessions, live terminals,
  terminals starting). So the removal sees the start (`kept { in_use }`),
  or the start finds the claim and is refused (`conflict`) before
  anything runs; teardown never runs under it. This replaces the check
  after the start, which left a window (a reading before the start's
  reservation, a claim after its last check). The node's e2e test
  `meeting` races the two.
- **Archiving ends what runs beside a session.** `session.mark {
  archive }` ends its terminals and runs (shells, scripts, services) and
  keeps them, with their output, as `terminal.kill` does; deletion and
  retention still end and forget them; a session's end alone still ends
  none but agents'. An archived session is put away: a service beside it
  would run on unseen, holding its port and its worktree.
- **No forge sign-in in a repository's processes.** Besides Vessel's
  credentials, every terminal, script and service, and setup and
  teardown (ADR 0027), runs without the variables the node's forge token
  is read from (`GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`,
  `GITHUB_ENTERPRISE_TOKEN`, ADR 0029): a shell an agent may type into
  and a repository's script are not the node's sign-in. A person's `gh`
  in a terminal still finds its own stored login. Not closed: an agent's
  own harness process inherits the node's environment as before (ADR
  0013), and `gh`'s stored login is readable by any process of the user.
- **A session's terminals are not on `SessionInfo`.** A page lists
  `terminal.list {}` once (at most 96) and groups by `place`: a
  terminal's changes move its own `revision`, not the session's, so a
  count on the session would be stale between the page's reads of the
  sessions.
- **The session's place is the registry's.** A script beside a session,
  and a terminal's count against a project, take the session's project
  and worktree from the workspace registry (`workspace_of`); the info a
  session's runtime holds carries none (the service fills it per reply),
  so `script.run { place: session }` answered `no_project` for every
  session.
- **Archived, nothing new starts beside it.** `terminal.create` and
  `script.run` beside an archived session answer `conflict` until it is
  brought back, and a start that raced the archiving ends itself (its
  check after registering sees the mark; the archiving ends what was
  registered before the mark).
- **What "both profiles" does not stop, said plainly** (the join's
  review). An agent whose profile lacks `agent_terminals` may still start
  a session under a profile that has it (ADR 0013 does not limit which
  profile an agent starts a session with) and prompt that agent, which
  then uses a terminal of its own: what it runs there is held to that
  profile's agent and permission mode, as its own shell tool is, so the
  switch gives exactly what a person allowed that profile. Which profiles
  an agent may start sessions under is ADR 0013's question, left open.
  An agent of another node opening a terminal beside a session here is
  tied to that session's end only: this node does not see the agent's own
  session end, so its terminal runs until the session beside it ends or a
  person closes it (the agent's credential, gone with its session, no
  longer reaches it). A terminal already open keeps running when a person
  turns `agent_terminals` off, as a running service does when its
  approval is withdrawn: the switch is asked at each start.

## Amended by ADR 0066 (2026-11-14)

Decision 5 left assigning a port to a later track. ADR 0066 specifies
the project-file keys for terminals opened after setup, a service's
port, and a TCP health probe, and the digest version that covers the
commands those keys name. Its parser and approval contract may land before
execution, while the protocol assignment refuses the new sets to clients.
It does not add a proxy. The scanner in
decision 5 stays.

## Deterministic terminal admission (W17 prerequisite, source only)

[Decision Log]
- Purpose: receipt-driven creation must reserve its exact TerminalId before any
  process starts, through the same registry used by ordinary shells and scripts.
- Existing assets: one mutex already counts live entries and starts; a reservation
  holds the startup folder until registration or drop; the node's TaskTracker and
  actor kill/end path already own asynchronous cleanup.
- Alternatives: a second id registry races the real map. An unchecked map insert
  replaces an incumbent. A wrapping reservation number can alias an old guard.
- Choice: refuse existing and starting duplicate IDs under the existing lock.
  A private allocation token and the registry state's opaque Arc identity bind
  each held Reservation, without a serialized id or wrapping counter. Synchronous
  registration validates that token, expected TerminalId and registry incarnation
  before a vacant-only insert. Existing callers build actor info from the reserved
  id. Successful registration/watch ordering remains unchanged.
- Rejection: before returning the refusal, move the handle, reservation and plan
  into the existing tracked task owner. Stop a fresh rejected actor; a handle whose
  id is already an incumbent is retained without killing that incumbent. Observe
  the existing CLOSE_WAIT budget; a timeout drops only its JoinHandle observer,
  never its tracked task or retained startup reservation. Caller cancellation is
  likewise unable to drop that owner. A remaining incumbent releases the retained
  reservation when its actor ends, including node shutdown.
- Limits: one rejected cleanup retains one already-counted start reservation;
  there is no extra unbounded start/worker registry. This observes the base actor
  end and does not certify reaping of every descendant, complete auxiliary I/O,
  physical roots, forgotten debt or previous-boot recovery. Those proof producers
  are separate; production checkpoint restore remains Busy.

Implementation is staged on announceded4cd137. Authored tests cover starting and
existing duplicates, simultaneous attempts, wrong-id and foreign-incarnation
binding, rejected-handle cleanup, cancelled observation and incumbent survival.
No test execution, V3 acceptance, merge or consumer-ready hash is claimed here.

A missing actor-end watch must not hang node shutdown. While the node is live,
its uncertain rejected start stays retained. Once the existing runner shutdown
token fires, the same CLOSE_WAIT bounds the final end observation. An unobserved
end produces the private ShutdownUnconfirmed diagnostic and an Unavailable wire
refusal if its observer remains; it is logged before node-lifetime ownership is
released. It never becomes successful cleanup or persisted debt reconciliation.
This bounded shutdown branch preserves the separate future proof requirement.
