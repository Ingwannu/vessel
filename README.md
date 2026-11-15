# Vessel

An **Agentic Development Environment**: one lightweight node per machine that
runs your agent harnesses (Claude Code, Codex, Gemini, OpenCode), and clients
that reach every node as if it were local.

Vessel is not an editor. It is the layer underneath: the daemon, the wire
protocol, the relay, and the sync. Editors (Zed, VS Code, a future native
client) plug into it.

## Why

Today each machine is an island. The harness on your desktop cannot hand work
to the Linux box with the GPU, your skills and MCP servers are configured
separately everywhere, and remote access means SSH plus luck.

Vessel aims for:

1. **Lightweight.** Native Rust, single binary, no runtime to install.
2. **Harness-agnostic.** Wraps the CLIs you already use instead of replacing
   them. Remote feels local.
3. **A fleet, not a host.** First-run setup in minutes. Skills, MCP servers,
   memory and conversation history sync across machines. Every agent on every
   node can see and delegate to the others.
4. **Any model, any harness.** Route a harness to whatever model you like.
5. **IDE-grade context.** Symbols, references, diagnostics and call graphs,
   fed to the agent through LSP rather than locked inside one editor.

See [docs/VISION.md](docs/VISION.md) for the long version and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the pieces fit.

## Status

Milestones 1 to 7 are done, on `main`, and green on Linux, macOS and
Windows ([docs/ROADMAP.md](docs/ROADMAP.md) has the details and what is
next):

1. **One node, done properly.** Terminal sessions in a PTY and structured
   sessions over ACP, a durable log every client can follow and resume,
   a token on every connection, the `vessel` CLI.
2. **Any machine over SSH.** `vessel` installs and updates `vesseld` there
   itself; an attach survives a lost connection.
3. **Any machine behind NAT.** A relay that carries only end-to-end
   encrypted traffic, and devices paired with a node, each revocable.
4. **One person's machines as one fleet.** Harness profiles, instructions,
   skills and MCP servers shared from one home node, conversations and
   memory moved on request, first-run setup, and agents that start and
   drive sessions on any node of the fleet (below).
5. **Every harness on every model.** A profile names a model route, and
   the node's own proxy on loopback lets Claude Code or Codex run on
   another provider's model (Anthropic, OpenAI, or any OpenAI-compatible
   server), recording each call's tokens in the session's log (below).
6. **IDE context.** The node runs language servers on saved files and gives
   agents symbols, references, hover, call hierarchy and diagnostics as MCP
   tools, without running repository code by default (below).
7. **A web UI.** `vessel ui` serves the fleet in a browser on this
   machine: a board of every machine's sessions, new sessions with the
   agent's own model, effort and permissions, a session's timeline or
   terminal with its model calls, and Settings, where machines are added,
   renamed and forgotten and each machine's agents, models, devices and
   sync are shown ("Web UI" below).

## Try it

```bash
cargo build --release
./target/release/vessel init                # node, harnesses, profiles, logins; asks before changing anything
./target/release/vessel init --service      # and run the node as a per-user service (Linux, macOS)
./target/release/vessel init --join --yes < link.txt   # and join a fleet with the link `vessel fleet invite` printed
./target/release/vessel node start          # or run it in the foreground (or ./target/release/vesseld)
# in another terminal, on the same machine (the token is read from ~/.vessel):
./target/release/vessel status
./target/release/vessel run claude          # Claude Code's own TUI; Ctrl-] detaches
./target/release/vessel run claude --structured   # needs claude-agent-acp on PATH
./target/release/vessel sessions
./target/release/vessel attach 3f2a         # any unique prefix of a session id
./target/release/vessel log 3f2a > session.log
```

The per-user service starts at login and restarts a failed process. A clean
shutdown stays stopped. Existing service files are preserved; an older
macOS agent with unconditional `KeepAlive` still starts again after a
clean exit until you remove and recreate it with `vessel init --service`.
New service files store an absolute data-directory path, so they work from
the service manager's working directory. Existing relative-path units stay
unchanged; remove and recreate one to adopt that behavior. Paths and PATH
must be Unicode without control characters to be stored in a unit.

`vessel node stop` requests graceful shutdown through the node's Admin
permission. With live sessions it lists them and refuses; `--when-idle`
returns after starting a drain, and `--now --yes` interrupts them. Without
`--yes`, interruption requires a terminal confirmation. `--timeout` bounds
the command in positive seconds; `--help` shows the default. When the local directory's
identity matches the connected node, success
also observes its lock being released. Another directory's loopback
endpoint only proves connection closure; remote success confirms only the
request.
Before a local shutdown or drain, the CLI compares the user service file
with the current installer template for the authenticated directory. A
personal or older file is left alone and produces a warning that the node
may start again. Unreadable, malformed or oversized service files are
refused before shutdown. No service manager is invoked and a matching disk
file does not prove which configuration it loaded.
Forced cleanup, service-manager finalization and restart are pending.

Sessions survive client disconnects; stopping the node marks live ones
`Interrupted`, and their logs stay readable after a restart.

`vessel log ID --tail N` prints only the last N records after `--from`.
Omit `--tail` to read the whole log; `--tail 0` prints nothing. A count must
be a complete non-negative integer, validated before connecting. The tail
window grows only as records arrive, without reserving the requested count.
stdout keeps session output and stderr keeps status and retention notices.

Add `--filter tools|text|errors|permissions` for one event kind or
`--since 2026-01-01T00:00:00Z` for an inclusive RFC 3339 start time.
Selection applies sequence, then time, then kind, then tail. Unknown filters
and incomplete timestamps fail before connecting. Terminal bytes have none
of the four structured kinds.

`-f` / `--follow` uses its default history window when tail is omitted and
then matching live records through the current run's end or Ctrl-C. Its
help shows the default count. Interrupting the command keeps the session
running; a later resumed run can be followed with another invocation.

A session title uses the first nonempty prompt line unless a title was
supplied, keeping [ADR 0022's title limits](docs/adr/0022-session-options-and-titles.md).
The node records who supplied a title so later generation can preserve a
person's choice; older titles have unknown origin (ADR 0062). Automatic
generation is not enabled yet in this build.

### A conversation: the queue, files, questions, marks

A structured session's next message can go in while its agent still
works, and the node keeps what waits, so every client sees the same queue
(ADR 0024):

```bash
vessel send 3f2a "run the tests next" --queue   # waits its turn: prints `queued <entry>`
vessel send 3f2a "use the staging config" --steer   # into the running turn (agents that take it)
vessel send 3f2a "stop, do this instead" --interrupt   # cancels the turn, then starts this
vessel send 3f2a "what is in these?" --attach shot.png --attach notes.md
vessel session queue 3f2a                    # what waits, next first; held or not
vessel session queue 3f2a edit 9c "run only the unit tests"   # its files stay
vessel session queue 3f2a move 9c --before 7d   # or last without --before
vessel session queue 3f2a send-now 9c --steer   # remove, hold, resume, clear too
vessel session answer 3f2a 51c way=left sure=yes   # a question the agent asks, by field
vessel session answer 3f2a 8e allow-once     # a permission request, by option
vessel session resume 3f2a                   # an ended session's agent picks the conversation up again
vessel session park 3f2a                     # stop its idle agent now; wake, or its next prompt, starts it again
vessel session fork 3f2a --after 120         # a new session continuing the conversation from that turn's end
vessel session rewind 3f2a --after 120       # take this session back to that turn's end
vessel history claude --cwd ~/app            # the agent's own conversations in that folder, to import one
vessel run claude --structured --resume 4c1e… --cwd ~/app --title "the parser"   # import it: its history is the log
vessel session pin 3f2a                      # unpin, archive, unarchive, read, unread
vessel session delete 3f2a                   # an ended one, for good (asks; --yes to skip), its worktree too if nothing would be lost
```

- Without `--queue`, `--steer`, `--interrupt` or `--attach`, `send` starts
  a turn as it always did, refused while one runs. Queued prompts start
  one per turn; a person cancelling a turn, a failed turn, or the session
  ending holds the queue until `vessel session queue ID resume`.
- `--attach` uploads the file a piece at a time and keeps it with the
  session (an image or audio up to 10 MiB, which the agent gets inline if
  it takes them; any other file up to 50 MiB, which it reads by a link).
- Entries, questions and requests are named by their id or a unique
  prefix of it, as sessions are; `vessel attach` and `vessel log` show
  them, and `queue` shows each entry's revision (`edit --revision`
  refuses an edit made from an older one).
- `vessel sessions` shows each session's marks. `archive` stops a live
  session first; `resume` brings an archived one back; `delete` refuses
  a live one.
- Forking and rewinding (protocol 14) start at a turn's end: `vessel log`
  shows the number on its `turn ended` line. Only agents that can fork
  their conversation (Claude Code and Codex through their ACP adapters
  do); Vessel checks that the fork ends where it asked and refuses it
  otherwise. A rewind keeps what came after in the log, and does not put
  files back (it says how many changes it left).
- An idle agent is parked (protocol 12): after an hour without activity
  (`vessel node settings --park-after MINUTES|never`, kept across
  restarts and applied at once; `vesseld --park-after` is the value of a
  node where nobody changed it) its process is stopped while
  the session stays running, unless a turn runs, something waits on an
  answer or its queue moves. Someone following it (a page that has it
  open, `vessel attach`) does not keep it awake, as in Paseo: the
  follower sees it parked and its next prompt wakes it. The next prompt,
  option change or `vessel
  session wake` starts it again on the same conversation; `vessel
  sessions` says `(parked)`. Only an agent whose adapter can load a
  conversation again (ACP `loadSession`) is parked; what it left running
  in the background stops with it.
- Agents over their own protocols (protocol 18, ADR 0057): `vessel node
  settings --driver codex=direct` chooses Codex's app-server instead of
  its ACP adapter for new sessions (likewise `claude` and `opencode`;
  `acp` goes back). Codex's is offered: it asks for permissions and
  questions with Codex's own choices, and its `mode` option is codex-acp's
  (`read-only`, `workspace-write`, `agent`, `agent-full-access`). It runs
  for a `codex` of at least 0.166.0, otherwise over ACP with a line in
  the session's log. The node refuses `direct` for Claude Code and
  OpenCode until their drivers can ask for permissions as ACP does.

Search what was said in every session (protocol 12): prompts and the
agents' replies, live, ended or archived, newest first.

```bash
vessel search parser 오류                    # every word, inside other words, without case
vessel search zebrafish --session 3f2a       # one session's
vessel search parser --before 1763031235000000000   # the next page, from the last line's `more:`
```

- Each line is a session, the record where the message starts (`vessel
  log ID --from` that number shows it in place), the time, who said it,
  and the words in `[` `]`. Tool calls, their output and thoughts are not
  searched; a reply is found once it has ended.
- Agents have the same as the `search_sessions` tool.

### A side question beside the running turn

```bash
vessel session side 3f2a                    # open/reuse a separate side; prints its id
vessel session side 3f2a "What is the agent doing?"
vessel session side 91ab "Explain that plan" # ask again using the side id
vessel attach 91ab                          # read its replies and answer approvals
vessel session resume 91ab                  # explicitly resume an ended side
```

A side gets the original conversation's latest status, recorded plan and
limited recent messages. Its questions and answers never enter the
original conversation or stop its running turn. It shares normal project
tools/permissions, as another session in that folder does. It starts with
recent context rather than a full provider-history fork. Read and operate
access are both required; agents cannot open sides. Use `session side`
for every question: ordinary `send` and queue operations refuse sides so
that their context cannot become stale. Existing side logs remain readable
if their parent disappears, but new questions and resume then fail.
Vessel records only the visible question as user input. The provider may
retain the supplied parent context in its own conversation files, which
can be included in a native session export.

### One prompt to several models

`session compare` makes a separate Git worktree and structured session for
one to eight selections. Write a JSON array to `targets.json`, for example:

```json
[{"harness":"claude-code","model":"MODEL_A"},{"harness":"codex","model":"MODEL_B"}]
```

Use model ids offered by your agent; optional `profile` and `options`
are the normal structured-session choices. Then run:

```bash
vessel session compare --project PROJECT_ID --base main --targets targets.json "Check this change" --attach notes.md --json
```

The base resolves once, and every model receives the same text and stored
file bytes. Setup must be approved through the normal project commands;
comparison never approves, skips setup, queues or steers. Partial failures
keep successful sessions. The final report lists every known branch,
worktree, session and upload/offset, including effects whose reply was lost.
A partial run exits nonzero; inspect those resources before an explicit new
attempt. Reports cannot resume a job across CLI processes. Continue or
remove the ordinary sessions/worktrees with their existing commands.
`--on NODE_ID` selects a fleet destination; the project id belongs to it.
No provider is prompted merely to discover its options. Agent credentials
cannot use this helper: the existing git.log agent guard refuses its base
preflight. No new agent tool or authority is added.

### Another machine, over SSH

Any machine you can `ssh` to with a key works (Linux or macOS,
`x86_64` or `aarch64`); nothing needs to be installed or opened there
first.

```bash
./target/release/vessel nodes add gpu-box me@gpu-box --default
./target/release/vessel run claude          # installs vesseld, starts the node, attaches
./target/release/vessel --node gpu-box sessions
./target/release/vessel node status         # version, serving/draining, live sessions
./target/release/vessel node update --when-idle   # new vesseld once its sessions end
```

- The first contact uploads `vesseld` to `~/.vessel/bin/` on that machine:
  the build next to `vessel` (a development checkout, named by content
  hash), or this version's release (fetched with `gh`). `vessel node
  install --from local|release` does it explicitly.
- The node's id is recorded in `~/.config/vessel/nodes.toml`, and a
  different machine answering at that name later is refused.
- A project can pick one of your nodes with `.vessel/config.toml`
  (`default_node = "gpu-box"`), and may do nothing else.
- `vessel nodes list|show|remove|rename|default` edit the book in place,
  keeping its comments; the web UI's Machines page makes the same edits.
  A rename moves the whole entry and the default with it, but not a
  project file that names the old name.
- A dropped connection during `attach` reconnects by itself and continues
  where it was.

### Running a relay

A relay lets clients reach a node that has no public address. It only
splices connections: everything through it is end-to-end encrypted to the
node's key (ADR 0009), and it keeps nothing on disk.

```bash
head -c 32 /dev/urandom | base64 > relay.token && chmod 600 relay.token
./target/release/vesseld relay --bind 127.0.0.1:7078 --token-file relay.token
```

- Nodes that use the relay need the same token; clients need none.
- The relay speaks plain HTTP. For `wss://`, put it behind the HTTPS
  reverse proxy that already serves the machine, for example with nginx
  (the relay URL is then `wss://relay.example.com/vessel`):

  ```nginx
  # In the http block: connections per client address.
  limit_conn_zone $binary_remote_addr zone=vessel:10m;

  location /vessel/ {
      limit_conn vessel 16;
      proxy_pass http://127.0.0.1:7078/;
      proxy_http_version 1.1;
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection "upgrade";
      proxy_set_header X-Real-IP $remote_addr;
  }
  ```

  and run the relay with `--client-address-header X-Real-IP
  --trusted-proxy 127.0.0.1`.
- Behind a proxy every connection comes from the proxy, so the
  per-client limits need the client's address from a header
  (`--client-address-header X-Real-IP`, or `CF-Connecting-IP` behind
  Cloudflare). The header is believed only from the peers named with
  `--trusted-proxy` (an address or a network in CIDR form, repeatable),
  and the relay refuses to start with a header and no trusted proxy;
  anyone else is counted by their own address, whatever they send. The
  proxy must overwrite the header with one bare address, IPv4
  (`192.0.2.7`) or IPv6 (`2001:db8::7`): a list (as `X-Forwarded-For`
  grows), a port, brackets or the header twice are answered `400`, and
  a request from the proxy without the header counts as the proxy
  itself, rate included. A trusted proxy's sockets are not
  counted per address at accept (they would all be the proxy's): the
  relay counts per client from the header once a request arrives, so
  the proxy should limit connections per client itself, as `limit_conn`
  does above.
- The relay pings every connection every 10 s, so a proxy's idle timeout
  of 60 s (nginx's default) is enough. A connection silent for 30 s, its
  pings unanswered, is dropped.
- `GET /health` answers `ok`.
- Limits, each a flag with a `VESSEL_RELAY_*` variable of the same name
  (table below). The defaults leave a fleet of tens of nodes plenty of
  room:

  | Flag | Limits | Default |
  | --- | --- | --- |
  | `--max-nodes` | nodes registered at once | 128 |
  | `--max-registering` | nodes still registering at once | 64 |
  | `--max-clients` | relayed connections at once, in all | 384 |
  | `--max-clients-per-node` | relayed connections to one node at once | 128 |
  | `--max-bytes-per-second` | bytes a second each way through one connection, after a one-second burst (0: no limit; at least 64 KiB) | 16 MiB |
  | `--max-connections` | sockets open at once, of every kind, counted from accept | 1000 |
  | `--max-unauthenticated` | of `--max-connections`, sockets that have not authenticated yet; the rest is kept for registered nodes and paired clients. Less than half of `--max-connections` | 400 |
  | `--max-connections-per-address` | sockets not authenticated yet from one address, counted from accept; not applied to a trusted proxy's sockets. 4 × it under half of `--max-unauthenticated` | 16 |
  | `--max-registering-per-address` | nodes still registering at once from one address. 4 × it under half of `--max-registering` | 7 |
  | `--reserved-registering` | of `--max-registering`, places kept for nodes that registered here before (at most half) | 16 |
  | `--max-clients-per-address` | relayed connections at once from one address. 4 × it under half of `--max-clients` | 16 |
  | `--max-node-clients-per-address` | relayed connections to one node at once from one address while the node admits them (their first 7 s). 4 × it under half of the 40 a node admits at once through one relay | 4 |
  | `--trusted-proxy` | a proxy whose `--client-address-header` is believed (above); repeatable | none |

  Per address means an IPv4 address, or an IPv6 `/64`; a `/56` may hold
  twice each per-address limit and a `/48` four times, so walking the
  prefixes of one allocation gains little. So that one `/48` never holds
  half of anything, the relay refuses to start unless four times each
  per-address limit is less than half of its pool, and
  `--max-unauthenticated` less than half of `--max-connections` (the
  rules in the table); the refusal names both flags, the largest value
  that would pass and the smallest pool that would. New connections from one
  address are rate-limited from accept (30 at once, then one a second;
  scaled the same way); a socket that authenticates gives its share of
  the rate back, so only sockets that never do spend it. A socket has
  authenticated once its node registered (token or ticket, and proof of
  its key), once the node's data connection was claimed, or, for a
  client, once its node has admitted it: the relay cannot see that
  (everything is encrypted to the node), but a node drops a relayed
  channel it has not admitted within 5 s, so a client still connected
  7 s after it was paired was admitted.

  A node past `--max-nodes` is told the relay is full and tries again
  every 30 s; a client or a registering node past a connection limit is
  closed with code `4503`, and a socket past an accept limit (or the
  rate) is answered `503`, both free to try again. The relay logs each
  kind of refusal at info, at most once every 10 s with the count in
  between.
- Deadlines, not configurable: a request's headers within 10 s; a
  registering node's first message, and a client's first record once its
  node answered, within 2 s; a stopping relay drops what is still open
  5 s after it was asked to stop; and a closed connection is read for at
  most 1 s more, until the other side closes too, so its close code is not
  lost to a reset (it keeps its place, and counts against the limits,
  meanwhile). Registering nodes take the open places
  first; the `--reserved-registering` places are taken only when no open
  place is left, by a connection whose first message names a node
  registered here before (since the relay started). Until it speaks such
  a connection holds no place; anyone else is then refused as full. So a
  known node gets back in while strangers hold every open place.
- What the limits hold against: one host, or one IPv6 `/48`, cannot deny
  the relay to anyone else, and nodes registered here before can register
  again while it tries. Many hosts at once (a botnet) are out of scope:
  they can fill the unauthenticated share and the open registering
  places, so new connections are refused until they stop, though nodes
  and clients already through keep working. Filtering at the network
  edge, or a DDoS service in front, is the answer to that. A node behind
  relays keeps, for each relay it is linked to, a budget of its own for
  channels waiting to be admitted through it (40), apart from the other
  relays' and its direct route's (64, of which one address holds 7, a
  `/48` 28), so a crowd through one route never takes another's room.
- A fleet's home that registers with the relay gives its members, those
  started without `--relay` of their own, tickets for their own keys
  (`fleet.callback`), never the token: a ticket registers that one node,
  for a day while the member renews it, and holds a place among
  `--max-nodes` like any node until it leaves or its ticket expires.
- Each relayed connection holds two sockets on the relay, each node one:
  keep `max-nodes + 2 × max-clients` under the relay's open-file limit
  (`ulimit -n`, or `LimitNOFILE` in a systemd unit), which the defaults
  do for the common 1024, and raise both together; `--max-connections`
  (1000) caps every socket, so the relay refuses before it runs out.

### A node behind NAT: pairing a device

A node with no public address keeps a connection to a relay (above), and
devices pair with it once. Every connection, direct or relayed, is
end-to-end encrypted to the node's key, which the device pins at pairing
(ADRs 0009 and 0010).

```bash
# on the node's machine
vesseld --relay wss://relay.example.com/vessel --relay-token-file relay.token
vessel node pair --label laptop       # prints a single-use link (and a QR code)
# on the laptop: paste the link at the prompt, never on the command line
vessel nodes add gpu-box --pair
vessel --node gpu-box run claude
# later, on the node (or from any admin device)
vessel node devices
vessel node devices self                  # this connection's own grant; no Read/Admin needed
vessel node devices revoke 3fa9       # its open connections close
```

Pairing requires an advertised direct address (`--public`) or relay (`--relay`).
Without one, the node refuses before issuing a link and says what to configure.
An advertised route must still be reachable from the other device.

- `vessel nodes add --pair` shows the node's key fingerprint; compare it
  with `vessel node status` on the node before confirming. Piped in, the
  link needs `--yes`.
- A node with a reachable address can advertise it (`vesseld --public
  203.0.113.5:7077`): devices try it first, for 3 s, then the relays. The
  node pings direct connections every 10 s, as a relay does, so a dropped
  network is noticed within 30 s and an attach resumes; behind a proxy, an
  idle timeout of 60 s is enough.
- The direct route holds at most 64 connections that have not finished
  their handshake and device preamble, 7 of them from one address (an
  IPv4 address or an IPv6 `/64`; a `/56` 14, a `/48` 28), so one host
  cannot keep devices out. Behind a reverse proxy every client would be
  the proxy's one address: run the node as a relay is run behind one,
  with `--client-address-header X-Real-IP --trusted-proxy 127.0.0.1`
  ("Running a relay": the header in one form, believed only from the
  proxies named, `400` otherwise; a request without it counts as the
  proxy). The node refuses to start with the header and no trusted
  proxy.
- The device's credential is a file in `~/.config/vessel/credentials/`
  (0600); the book points to it and never holds the secret. Moving a
  device means pairing it again.

### Projects and worktrees

A node keeps the folders it works in (protocol 10, ADRs 0023 and 0027).
A session can start in a project's folder, or in a worktree of its own: a
folder and a branch of the project's repository, prepared by the
project's setup commands.

```bash
vessel project add ~/code/app               # inside a repository: its top level
vessel project clone https://github.com/owner/repo.git ~/code
vessel project list
vessel project commands app                 # what vessel.toml would run, its digest, what was approved last
vessel project approve app <digest>         # that exact set runs from now on
vessel worktree new app                     # a new branch from the default one; follows setup
vessel run claude --project app --new-worktree   # the same, then a session in it
vessel run claude --worktree 3f2a           # a session in an existing worktree
vessel worktree remove 3f2a                 # kept if anything would be lost
vessel worktree remove 3f2a --snapshot      # or keep its work as a snapshot first
```

The project's file, `vessel.toml` at the repository's root:

```toml
[worktree]
setup = ["npm ci", "cp \"$VESSEL_PROJECT_ROOT/.env\" ."]
teardown = "docker compose down"
```

- Its commands run only once a person approved that exact set for that
  project; any change to them is asked again. Agents and read-only
  devices cannot approve.
- A worktree is removed only when nothing would be lost (no change,
  operation, submodule, flagged file or unshipped commit); otherwise it is
  kept and says why. The worktree made for a session goes by that rule
  when the session is deleted (`vessel session delete` says whether it
  went or why it stayed) or when retention drops the session; ending or
  archiving a session leaves it, so `vessel session resume` starts the
  session again in it. A session whose folder is gone, or whose worktree
  a setup or a removal holds, does not start again.
- Every call looks at its folder again: one replaced since it was added
  (by a link to another, say), or a worktree that no longer belongs to
  its repository, is refused, and git is not run in it. A project's
  folder cannot be one holding your home folder or the node's data, nor
  git's own folder (a `.git` folder or a bare repository).
- git is the machine's own `git` (2.25 or newer), run without prompts,
  without the repository's hooks (except on commit and push) and without
  the programs its configuration names (ADR 0030): a filter or merge
  driver a repository's own `.git/config` defines is turned off, and a
  fetch, pull, push or commit is refused, naming the key, when that file
  sets an `ssh` command, a credential helper, a proxy, a TLS setting or a
  signing program. Your global git configuration is used as it is; move a
  per-repository setting there (under `includeIf "gitdir:…"`) for Vessel
  to use it.

Project-file terminal support is staged (ADR 0066):
`[[worktree.terminals]]` accepts a `command` and optional `name`, at most
32 entries. Invalid entries refuse the whole file; names use the existing
120-character title limit. These entries enter the approval digest, but
this build does not open them automatically. Its unassigned wire version
means negotiated clients receive an explicit refusal for such a set until
integration enables clients that can show it. Existing files without the
new key retain their approvals.

### Pull requests

A session placed in a project or a worktree, on a branch other than the
project's default one, has its pull request watched by its node: its
checks, reviews and whether it can merge place the session on the board
(validating, in review, ready) and show on its card.

```sh
gh auth login                               # once, on the node's machine
vessel pr list                              # every watched session's
vessel pr show 3f2a --refresh               # one session's, read now
vessel pr open 3f2a --title "Fix the parser" --body-file notes.md
vessel pr merge 3f2a --number 12 --head <full commit id> --method squash
vessel pr feedback 3f2a on                  # send it its failing checks and reviews
vessel pr status                            # the forges, and who the node is signed in as
```

- GitHub (and GitHub Enterprise) for now. The node signs in as the
  machine already is: `GH_TOKEN` or `GITHUB_TOKEN` in its environment for
  github.com, `GH_ENTERPRISE_TOKEN` for the host `GH_HOST` names, else
  `gh auth token`. A token is kept in memory only and never logged; a
  host with none gets no request.
- A forge that is slow, down or limiting the node never holds a session:
  its last facts stay on the card with the reason, and the node looks
  again later (a minute while it moves, five once it is quiet).
- Opening pushes the branch first (never forced); a merge names the pull
  request and its head exactly and is refused if either moved. Nothing
  merges on its own.
- Feedback is off until you turn it on per session. It then sends, once
  each, failing checks, requested changes, unresolved comments and
  conflicts as one prompt through the session's queue, never into a
  running turn; a review's or comment's words are quoted only when their
  author can write to the repository.
- A session's agent can read its pull request and open one (`vessel`
  tools `get_pull_request`, `open_pull_request`), after pushing the
  branch with its own tools; it cannot merge or turn feedback on.

### Shells, scripts and services beside a session

A node opens shells beside a session or in a project (protocol 11, ADR
0031). They outlive the page and the attach, and end with the node; their
last 1 MiB of output is kept in memory, never on disk.

```bash
vessel shell --session 3f2a               # a shell in the session's folder; Ctrl-] detaches
vessel shell --project app --dir web      # or in a project's folder
vessel shell list                         # live first, then the ended ones kept
vessel shell attach 9c1e                  # back to it, its screen replayed
vessel shell close 9c1e                   # end it and forget it
vessel script list --project app          # the project's scripts and services, and their runs
vessel script run test --project app      # run one, following its output
vessel script run web --session 3f2a      # a service in the session's worktree
vessel script restart 7b2d                # in place while it runs
vessel script stop 7b2d
vessel ports                              # what the sessions' and terminals' processes listen on
```

Scripts and services are named in the project's `vessel.toml`, beside
`[worktree]`, and run under the same approval:

```toml
[scripts.test]
command = "cargo test"

[services.web]
command = "npm run dev"
dir = "web"                                # a folder inside the checkout
```

- They run only once a person approved that exact set (`vessel project
  approve`); a file that names scripts is digested anew, so an approval
  given before it named any does not cover them. Agents cannot run them.
- A service that fails starts again after 1, 2, 4, 8 and 16 s, and is
  given up after five restarts without a minute's run; one that exits
  cleanly stays ended. A script or service ends with everything it
  started; a shell's own jobs end with it on Linux (on macOS and Windows a
  job a shell detached may outlive it).
- A terminal's environment is the node's without Vessel's own tokens and
  credentials and without the forge's sign-in (`GH_TOKEN`,
  `GITHUB_TOKEN` and the enterprise pair; `gh` in a shell still finds its
  own login), as setup's and teardown's are; a script's adds
  `VESSEL_PROJECT_ROOT`, `VESSEL_WORKTREE`, `VESSEL_BRANCH` and
  `VESSEL_SCRIPT`.
- Archiving a session ends the shells, scripts and services beside it,
  keeping them with their output; deleting it ends and forgets them
  before its worktree is tidied. A live one, or one starting, keeps its
  worktree from being removed.
- Ports are found without privilege (`/proc` on Linux, `lsof` on macOS,
  `netstat` on Windows), every few seconds while someone asks; nothing is
  opened or forwarded.

### Schedules and saved prompts

A node runs prompts on its own (protocol 11, ADR 0033): a schedule sends
its prompt to a new session each time, or into an existing session's
queue, on a cron expression in a time zone or every so often. Saved
prompts are a person's texts, named, for a project or for the machine.

```bash
vessel schedule create --name nightly --cron "0 9 * * 1-5" --tz Asia/Seoul \
  --harness claude --project app --prompt "Look at last night's failures"
vessel schedule create --every 30m --session 3f2a --prompt-file check.md
vessel schedule list
vessel schedule preview --cron "0 9 * * 1-5" --tz Asia/Seoul --count 5
vessel schedule runs nightly                # what each run came to
vessel schedule run nightly                 # once now
vessel schedule edit nightly --cron "0 8 * * 1-5"
vessel schedule edit nightly --keep         # the rest of its session as it was
vessel schedule pause nightly
vessel prompt save review --project app --text "Review the open pull requests"
vessel prompt run review --session 3f2a     # into its queue
```

- A run acts as whoever last made or changed the schedule, as they are
  when it runs: a revoked device's schedules end, an agent's end with its
  session. A new session's run is archived after its first turn unless
  `--keep` (its terminals and services end with it, as when a person
  archives); nobody is asked questions in it, and a permission it asks
  makes the run wait (the next due times are skipped meanwhile).
- `--harness` names one of the four or an agent added on the node
  (`vessel agents`). A schedule or a saved prompt is named by its whole
  id, or by its name or the start of its id when only one matches.
- At most four runs are made at once and eight scheduled sessions are in
  their first turn; the rest wait, the oldest due first. `schedule run`
  answers `unavailable` after 30 s without room.
- A time the clocks skip runs once at the jump; a time they repeat runs
  once. Due times missed while the node was stopped give one run (the
  latest, if less than a day old; `--catch-up skip` for none).
- Prompts never go into a running turn: they wait in the queue, and a
  schedule never puts a second one there, nor wakes a session a person
  stopped.
- Saved prompts are kept by the node, never read from a repository's
  files; agents may read them, not write them.

## Profiles, the fleet and agent tools

Milestone 4 (ADRs 0011 to 0013). Each part works on its own: profiles on
one node; a fleet, where every node follows one home; conversations and
memory copied on request; and agents that use all of it through the
`vessel` MCP tools.

### Profiles: one harness, several accounts

A profile is a harness with its own environment, typically another
account's configuration directory. Profiles belong to the machine that runs
them: `profiles.toml` in the node's data directory (owner-only, like the
token).

```toml
[profiles.claude-work]
harness = "claude"
label = "Claude (work)"
env = { CLAUDE_CONFIG_DIR = "~/.claude-work" }
unset = ["ANTHROPIC_API_KEY"]   # removed from the node's own environment

[profiles.codex-work]
harness = "codex"
env = { CODEX_HOME = "~/.codex-work" }
```

```bash
vessel profiles                              # the default per harness, and these
vessel profiles --reload                     # after an edit; a broken file changes nothing
vessel run claude --profile claude-work
vessel session move-profile 3f2a claude-work # at a usage limit: same conversation, other account
```

- Every harness has a default profile with its own name (`claude`,
  `codex`, `gemini`, `opencode`): the node's own environment.
- `vessel init` writes profiles for the other accounts' homes it finds and
  offers each one's login.
- A move keeps the session's id, log and followers. It works for
  structured Claude Code and Codex sessions, between turns (`--force`
  cancels a running one), and only to a profile that is logged in.
- `agent_terminals = true` lets the agents of a profile's sessions open
  terminals beside them, or beside sessions they started whose profile
  sets it too (ADR 0031). It is off for every profile unless
  written: a terminal the node starts is outside the harness's own
  sandbox, so turn it on knowing what that sandbox would have stopped.

### Any agent that speaks ACP

Besides the four, a node runs any agent that speaks the Agent Client
Protocol, added by the command that starts it (ADR 0032). It is kept in
`agents.toml` beside `profiles.toml`, offered wherever the four are, and
runs conversations only (no terminal).

```bash
vessel fleet secret set GOOSE_KEY            # read from standard input
vessel agents add goose --label Goose --env GOOSE_MODE=auto \
  --secret ANTHROPIC_API_KEY=GOOSE_KEY -- goose acp
vessel agents                                # the four, then the added ones
vessel run goose
vessel agents remove goose

vessel agents status                         # installed? signed in? (nothing runs)
vessel agents status codex --profile codex-work --refresh
vessel agents catalogue --refresh            # the ACP registry, fetched when you ask
vessel agents add example --from-registry example-agent   # shows the command, asks
vessel agents install codex                  # the vendor's npm command, shown, asked, run on the node
vessel agents install claude --adapter       # Claude Code's ACP adapter
```

- It starts with an empty environment: the search path, your home, the
  system's own variables (locale, terminal, time zone, temporary folder,
  proxies without a password), then what you give: `--inherit NAME` from
  the node's environment, `--env NAME=VALUE`, `--secret NAME=SECRET`
  from the node's secrets. Keys never go in its arguments: every client
  sees them, and so does the process table.
- Of the fleet's MCP servers it gets only those named with `--mcp`;
  Vessel's own tools always.
- Only a person adds or removes one (the `admin` scope; an agent never).
  An agent's session cannot move to another profile or be copied to
  another node.
- `vessel agents status --refresh` asks without a turn: Claude Code's and
  Codex's own status commands (never beside a running session of that
  profile), and for the others their ACP handshake. It says how to sign
  in; it does not sign in for you.
- An install runs the vendor's own command on the node (never beside a
  running session of that agent), with its output printed as it runs; it
  sees none of the node's credentials. Signing in afterwards is yours:
  `vessel agents status` says how.
- The catalogue lists what each registry entry would run; adding one
  writes exactly that (an `npx`/`uvx` command pinned to the version
  shown). An entry that ships only an archive is shown, not downloaded.
- On the page (`vessel ui`) an added agent is chosen on the new-session
  page after the four, by its label, and named so on its sessions;
  Settings › Agents lists it, found or not. It runs as a conversation
  only, so no terminal is offered for it; while a secret it names is not
  set on its machine, the page says so and does not start it. Adding,
  removing and installing are done with `vessel agents`.

### A fleet: one home, many members

One node keeps your instructions and skills: the home. Every other node is
a member, and gets copies in each of its profiles (ADR 0012).

```bash
# on the home: name the sources (here, a directory you keep them in)
vessel fleet init --instructions ~/agents/AGENTS.md --skills ~/agents/skills --skip 'draft-*'
vessel fleet invite --label laptop            # a single-use link, valid 10 minutes
# on the laptop: a fresh machine sets itself up and joins in one step,
# reading the link from standard input (never the command line)
vessel init --service --join
vessel fleet status                           # what it applied, and the files it left alone
# on the home
vessel fleet members
vessel fleet published                        # what it shares, by name (needs admin)
```

- Edit the sources on the home; members follow within seconds.
- A file you edited on a member is kept, with the home's version beside it
  as `<name>.vessel-incoming`, and listed as a conflict by `vessel fleet
  status`. A fleet file you deleted stays deleted until `vessel fleet
  restore <path>`. Files Vessel did not write are never touched.
- `vessel fleet join` joins a node that is already set up; `vessel fleet
  leave` stops following, and the files stay.
- Names pick nodes (`--to`, the agents' `node`), so the home refuses a
  member named like itself or another member. A node takes its machine's
  name unless told otherwise, and machines cloned from one image share
  one: name each node when you join it (`vessel node start --name <name>`,
  `VESSEL_NAME`, or `--name` in the service unit `vessel init --service`
  wrote). A refused member says why in `vessel fleet status` and asks
  again after a restart under its new name.
- The home binds each member's node id to its key and name for good, and
  keeps a name reserved while its node's device is revoked. A member
  reinstalled with a new key, or renamed, is refused until you run `vessel
  fleet forget <node id> --node <home>` (it needs `admin` there) and then
  invite it again: forgetting withdraws every invitation still outstanding
  on the home, and the id and name then go to the first member device
  paired afterwards that claims them, so invite only that node next and
  others once it has joined. Revoke its old device there too if it should
  not sync any more.

### MCP servers and secrets

MCP servers defined on the home reach every session on every node, handed
to the harness at launch (never written into its configuration files).
A value they need is a secret, kept per node:

```toml
# in the home's fleet.toml
[mcp.servers.docs]
type = "stdio"
command = "docs-mcp"
env = { DOCS_TOKEN = { secret = "DOCS_TOKEN" } }
```

```bash
vessel fleet secret set DOCS_TOKEN                 # typed (not shown) or piped, never an argument
vessel fleet secret list                           # names only
vessel fleet secret copy DOCS_TOKEN --to NODE_ID   # to another node of the fleet
```

- A session on a node without the secret runs without that server, and
  its log says which secret is missing.
- A value is never shown, logged or passed as an argument. Copying one
  needs the target to have granted the home `admin` (`vessel fleet join
  --grant read,operate,admin`); it is asked before the value is sent.

### Conversations and memory, on request

A structured session's conversation, or a profile's memory, is copied
between profiles of one node or to another node of the fleet (`--to`):

```bash
vessel session copy 3f2a --profile claude-work --cwd /srv/app --start   # resume a copy there
vessel session move 3f2a --to gpu-box --profile claude                  # copy, resume there, stop here
vessel fleet memory copy --from-profile claude --from-project /srv/app --profile claude-work
```

- Between turns only. Nothing is overwritten: a session lands whole or not
  at all, and memory that differs goes beside the existing file as
  `<name>.vessel-incoming`. Memory is never synced continuously; each
  account keeps its own.
- The copy is renamed for the working directory it goes to, and the new
  session records where it came from.

### Calls across nodes, and what they may do

Calls from one node to another go through the home (protocol 6). A member
grants the home scopes on itself when it joins (`--grant`, by default
`read,operate`; `none` grants nothing), and a call runs on its target with
no more than that grant and the caller's own scopes. A member's calls
reach the other members only if it was invited with `vessel fleet invite
--relay`.

The home calls a member back through a route the member names. A member
with none of its own (started with `vessel node start` or the service
`vessel init --service` installs, no `--relay` or `--public`) registers
with the home's relays, which its home tells it, so run the home with a
relay (`vesseld --relay URL --relay-token-file FILE`, see "A node behind
NAT") and every member is reachable without flags. A home with no relay
has none to share: its members still sync, but the home cannot run calls
on them until one of the two gets a route (`vessel init --join` and
`vessel fleet status` say so). The relay's token stays with the home: a
member gets a ticket that registers its own key only, for a day, renewed
while it reaches its home (a member with no route has to join, and grant,
within an hour of being invited), and it dials only `wss://` relays at
public addresses, checked when it connects, or relays on the host it
reaches its home by.

### Agent tools

Every session's agent gets the `vessel` MCP server (`vessel mcp`, started
by the node) with a credential of its own, ending with the session. Its
tools: `list_nodes`, `list_sessions`, `create_session` (with a first
prompt, and a `project` or `worktree` to run in), `send_input` (with a
`delivery`: `start`, `queue`, `steer` or `interrupt`),
`answer_permission`, `read_events`, `wait_turn`, `kill_session`,
`read_conversation` (the queue, the agent's commands and context, what
waits on an answer), `change_queue`, `answer_question`,
`resume_session` and `archive_session`, and for projects
`list_projects`, `list_worktrees` and `create_worktree` (which follows
setup to its end; at most 8 worktrees of its own, a session in one only
if it made it), and for terminals `create_terminal`, `list_terminals`,
`read_terminal`, `write_terminal` (marked destructive: typing runs
programs), `close_terminal` and `list_ports`; each but `list_nodes` and
the schedule tools takes `node`, so an agent can start a session on
another machine and read its answer. On its own node: `create_schedule`,
`create_heartbeat` (a prompt to itself, through its own queue),
`list_schedules`, `schedule_runs`, `pause_schedule`, `resume_schedule`,
`delete_schedule` and `list_saved_prompts`; at most 8 schedules per
session, gone when it ends, never run now.

- An agent has its creator's scopes, at most `read` and `operate`, and
  operates only on sessions it started (and theirs).
- At most 8 live sessions of its own per node, and agents start agents at
  most 3 levels deep.
- `send_input` and `create_session`'s first prompt take `notify` (this
  node only): the agent's own session is told, as a message queued into
  it, never typed into a running turn, when that prompt's turn waits for
  an answer and when it finishes, or the session ends; once per event, at
  most 32 turns followed per session and 60 notices an hour into it.
- Sessions record who started them. This is attribution under one OS
  user, not a sandbox (ADR 0013).
- Terminals only where a person allowed them (`agent_terminals = true` in
  its own session's profile and in that of the session it opens one
  beside: its own or one it started), at most 4 at once; it sees and uses
  only its own, and they end with its session or the one they are beside.
  It never runs a project's scripts.
- Agents do not mark sessions read or unread (the node refuses: an agent
  reading is not its owner seeing), pin them, delete them or attach
  files. The node asks questions only in sessions a person or a device
  started, and an agent operates only on its own, so `answer_question`
  has nothing to answer until a profile can grant an agent more (ADR
  0013).

## Models: every harness on every model

A node can run Claude Code or Codex on another provider's model through a
proxy of its own on loopback (milestone 5, ADR 0015). The harness keeps
speaking its own API; the node translates to the provider's (Anthropic
Messages, OpenAI Responses, or any OpenAI-compatible Chat Completions
server such as OpenRouter, vLLM or Ollama), or passes the bytes through
when both speak the same one.

Routes live in `models.toml` in the node's data directory, which must be
readable by you alone (on Unix the node refuses it otherwise; on Windows
it relies on the data directory's permissions, as for its other files).
The provider's key is a secret of the node's, named there, never written
in the file:

```toml
[providers.openrouter]
api = "openai-chat-completions"
base_url = "https://openrouter.ai/api/v1"
key = { secret = "OPENROUTER_API_KEY" }

[models.qwen-coder]
provider = "openrouter"
model = "qwen/qwen3-coder"
small_model = "qwen/qwen3-coder-flash"   # optional: the harness's fast calls
```

```toml
# profiles.toml: a profile on the route
[profiles.claude-qwen]
harness = "claude"
model = "qwen-coder"
```

```bash
vessel fleet secret set OPENROUTER_API_KEY   # typed at a prompt, or piped in
vessel profiles --reload                     # reads models.toml again too
vessel models                                # the routes: provider, API, models
vessel run claude --profile claude-qwen
vessel log 3f2a                              # model calls: route, model, tokens, time
```

- A routed session's harness gets a token of the session's own, never the
  provider's key, and is pinned to the route's models: a request for
  another model is refused with both names. The key is read for each
  call and goes only into the request to the provider.
- Give a provider no key, or a long random one. A local server (vLLM,
  LM Studio, Ollama) that needs none gets no `key` line at all; a key
  under 20 characters, or one as plain as `EMPTY` or `sk-1234`, is
  refused, since the proxy searches every answer for it and a model may
  write such a word.
- Claude Code and Codex can be routed, in terminal and structured mode;
  Gemini CLI and OpenCode cannot yet. A session keeps its route: moving
  it to a profile of another route is refused. A routed profile or
  session cannot pick another model or provider with its own arguments
  (Codex's `-m`, `-p`, `--oss`, `-c model...`; Claude Code's
  `--settings`); settings in the account's own home still apply.
- The proxy answers only on `127.0.0.1`, to a live session's token, with
  its own address as `Host`. The provider's key does not reach the
  harness in what the provider sends back: it is removed from an error,
  and a streamed answer that quotes it ends in an error there.
- Each call is a `model_call` record in the session's log, with its
  tokens and how it ended; never its content.
- At most 4 calls at once per session and 16 per node; a call past them
  is answered as a rate limit.
- `docs/smoke/model-route.sh` checks a real provider end to end
  (`docs/smoke/README.md`).

## Web UI

`vessel ui` serves Vessel's web UI on this machine and opens it in the
browser on Unix (on Windows, open the printed address).

- **New session** (the first screen once a machine is connected, the
  sidebar, the title bar's New, the command palette, or `n`) asks what
  you want to work on, in the project you worked in last on that machine
  (from a session, its project). Under the box choose the agent
  and, from what that agent itself offers on that machine, its model, its
  effort, its on/off settings and its permissions; under the "+" a profile
  and "Open as terminal"; under the box the machine, then the project
  and where in it the session runs: its folder as it is ("Local"), a new
  worktree on a new branch from a base, or a worktree on an existing
  branch (with no project, a folder, suggesting the folders sessions
  already ran in there). Send starts the session with those choices and
  opens it at once, with the prompt in it while the agent is still
  starting (a message typed meanwhile waits behind the first). For a new
  worktree it first makes the worktree: if the project's `vessel.toml`
  names commands that were not
  approved, it shows every one of them (setup, teardown, and its scripts
  and services, which the approval covers too) and what changed since
  the set approved last, for you to approve or to start without setup;
  setup's output shows under the box as it runs, and a failed setup
  waits for you (run it again, start anyway, or stop).
  Reading what an agent offers takes the machine a few seconds the first
  time (it starts the agent to ask); until then, or if the agent cannot
  say, the session starts on the agent's own defaults.
- **A session** is named by its title (the first line of its first prompt
  until you rename it, in place, in the title bar), shows its timeline
  and a composer that changes the same choices while it runs, and asks
  each permission once, over the composer. Its summary holds the plan,
  the files it touched, its model calls and, under Details, its
  identifiers. Beside Summary, **Changes** and **Files** show the code
  of its worktree or project (the branch's chip in the title bar opens
  Changes, in a sheet on a phone): every changed file's diff, by
  comparison (everything since the base, uncommitted, staged, unstaged,
  a commit), with stage, unstage and discard per file and git's actions
  (commit, push, pull, fetch, switch, create and rename a branch,
  stash), each refusal said in words; the file tree, a file by name,
  lines by content, a viewer and an editor that never overwrites a
  change made on disk since it read the file. Lines selected in a diff
  or a file go into the composer's draft as a reference ("Add to
  chat"). A session opened as a terminal is the agent's own TUI.
  What you type is the session's draft until you send it: it survives
  leaving the session, a reload and a new launch. In an empty composer
  Up and Down bring back the prompts you sent. Find in the session
  (`Ctrl+F`, `⌘F` on a Mac) searches its prompts and messages and counts
  the matches; the outline at the conversation's left edge, one mark per
  prompt, shows each prompt and the reply to it and jumps there.
- **While a turn runs** (a node on protocol 10), a message you send waits
  in the machine's queue, shown over the composer: edit, reorder, delete
  or steer each one; Stop holds the queue until you resume it. Settings ›
  General's "Default send" makes Send queue, steer into the running turn
  (agents that take it) or interrupt it; `Ctrl+Enter` or Ctrl with the
  click sends the other way. Files are pasted, dropped or picked with the
  paperclip, checked against what the agent takes and the machine's
  limits, and uploaded when you send; a sent prompt shows its files, an
  image fetched only when it comes into view. `/` at the start lists the
  agent's commands, `@` finds the session's files. A question the agent
  asks is answered over the composer, one step at a time. The context
  meter beside the model shows how full the agent's context is, and each
  turn its tokens. From the title bar's menu: pin, mark unread, start an
  ended session again, archive, restore and delete it.
- **The board** shows every machine's sessions by title, one lane per
  stage of delivery, a session an agent started named on its card; an
  ended one stays in its lane as exited until it is archived (its card's
  Archive; archiving also ends the terminals and services beside it,
  their output kept), and the archived ones are in the archive under it,
  to restore or delete (from a node before protocol 10, every ended one
  is); a session in a worktree shows its branch. The sidebar lists each
  machine's projects (each opens to its live sessions, and its "+"
  starts a session there) and each machine's live sessions under it,
  pinned ones first, marked while one works or waits on you and when one
  is unread; its
  Display menu groups them by status instead, sorts them by last
  activity and shows ended ones. `Ctrl+B` hides it; below 768px of window
  it opens from the title bar's menu button.
- **Settings** (at the sidebar's foot; inside it the sidebar lists its
  sections): General (whether Enter or `Ctrl+Enter` sends, and "Default
  send": whether a message sent while the agent works is queued, steered
  into its turn or interrupts it), Appearance
  (theme, interface and code font sizes, the conversation's width, and
  whether tool calls are collapsed), Shortcuts (every key, searchable,
  each rebound by pressing the new one; a key another command holds is
  offered to move), Notifications, Machines (add, rename, make default,
  forget), Projects (add a folder, clone a repository with its progress,
  start a new one; approve or withdraw a project's commands; its
  worktrees, removed only after saying what would be lost and offering a
  snapshot; its snapshots), Agents (what each agent on a machine
  offers: model, effort, permissions), Models, Devices (pairing a phone with a QR code), Sync
  and About. A machine in the sidebar opens it there. The page follows
  the system's light or dark theme until you choose one.
- **What you choose is kept by `vessel ui`, not the browser** (ADR
  0028): each launch opens at a new address, which would start a
  browser's own storage empty. The preferences are
  `ui-preferences.json` beside the node book (`--config-dir` moves both),
  the drafts `ui-drafts.json` in the data directory (`~/.vessel`, or
  `--data-dir`), both written by this user only (0600, replaced whole).
  Every open tab sees a change at once; when two tabs change one thing,
  the later change wins. Settings › General says where they are kept,
  and says so when they are kept for this launch only (no place to keep
  them, a file it cannot read, or one written by a newer Vessel, which
  is left as it is). A new session on a machine starts with the agent,
  profile and options its last one had there.
- **Notifications** (Settings › Notifications, off until chosen): a
  browser notification, a sound, or both, when a session starts to need
  you or finishes its work while the page is open but not in front, and
  a count in the tab's title. The browser grants notifications per
  address, and each launch has a new one: allow them again after each
  start of `vessel ui` (the section has the button and says what the
  browser answered).
- With no machine connected, the page opens on a welcome that offers to
  add one; once one is connected, on the board.

```bash
vessel ui            # serves on 127.0.0.1:7079 (any free port if taken), opens the browser
vessel ui --no-open  # prints the address instead
```

- The page reaches the nodes of your node book (and this machine's node)
  through `vessel ui`, which connects to them as `vessel` does: tokens,
  SSH, paired credentials and pinned ids stay in this process. The browser
  holds none of them and never speaks to a node itself.
- It listens on loopback only. Each printed address uses a fresh random
  `vessel-<id>.localhost` hostname so an earlier process on that port cannot
  seed this launch's HTML cache or service worker. Use a browser that resolves
  `*.localhost` locally: **Chrome or Edge 127+, or Firefox 138+, is
  required** (those versions check the integrity of the page's screens
  loaded later, ADR 0017). Safari
  does not resolve these hostnames and is not supported for now. Do not
  replace the printed hostname with a fixed address: that loses the
  per-launch browser-origin isolation.
- The address's `#bootstrap=` fragment is a
  single-use code: treat it like a password until opened. The page removes
  it from the address bar and exchanges proof of it for an independent
  session key in this tab's `sessionStorage`. Reloads and network reconnects
  work; reusing the address in a fresh tab does not. Start `vessel ui` again
  for another independent browser context. Every connection verifies a fresh
  server proof and launch id; an old tab stops when the UI is restarted.
  A tab whose connection keeps failing stops redialling and says it is
  disconnected; Retry (or a reload) resumes once `vessel ui` answers again.
- **Adding a machine** (the "+" by Machines in the sidebar, Settings ›
  Machines › Add machine, the welcome, or the command palette): run `vessel node pair` on that machine and paste
  the link it prints. The page reads the link without sending anything and
  shows the machine's key; compare it with the `key` line of
  `vessel node status` there before connecting. Then name it (the book's
  names: letters, digits, `.`, `_`, `-`), and a checklist shows it paired,
  connected and which agents it has. The link works once, for ten minutes;
  the page never puts it in an address, a log or a notification.
- An agent's card in Settings › Agents starts that agent once on its
  machine to ask what it offers (no prompt is sent); the machine keeps
  the answer for ten minutes.
- The old addresses `/fleet`, `/sync` and `/devices` lead to the matching
  Settings section.
- Links in agent output, in the timeline's Markdown or a terminal's OSC 8
  hyperlinks, open only when they are http(s): in a new tab that cannot
  reach the UI's, after a confirmation showing the URL for the terminal.
  The UI's own tab never navigates away.
- The Unix browser handoff is a 0600 file in a directory created as 0700,
  under `XDG_RUNTIME_DIR` when set (otherwise the system temporary directory).
  SIGINT, SIGTERM and SIGHUP remove it. Forced kills/crashes cannot guarantee
  cleanup, but a redeemed code is already dead. On Windows automatic opening
  is disabled until an atomic private ACL implementation is available; the
  printed address works and no handoff file is created.
- Device pairing links and QR codes start hidden. Use **Show pairing link**;
  copying, leaving the page, or 30 seconds passing hides them again.
- A session reaches a model route through a profile naming it: choose
  the profile under the new-session box's "+", where each says its
  route. A routed session shows its route in its summary's Details, and
  each model call (route, model, tokens, how it ended, how long; never
  content) in its timeline, summed in its summary or the terminal's
  frame.
- One connection per node is shared by every tab. A session's stream
  survives the node's connection dropping and resumes where it was.
- Release builds of `vessel` embed the page. A build from source embeds it
  when `web/dist` exists (`npm ci && npm run build` in `web/`), and shows a
  page saying how to build it otherwise.

The page's own development (Node.js 22 or later):

```bash
cd web
npm ci
npm run lint && npm run typecheck && npm test   # ESLint, Prettier, the font rule; tsc; vitest
npm run build && npm run fonts                  # SRI-protected web/dist and built font audit
npm run e2e                                     # Playwright against real nodes with the fake harnesses
npm run screens                                 # every main screen into web/artifacts/screens/
npm run perf                                    # the page's speed, measured, into web/artifacts/perf.json
```

`e2e`, `screens` and `perf` need the Rust binaries first (`cargo build -p
vessel-cli -p vessel-node --bin vesseld` and `cargo build -p
vessel-testkit --bins`). Set `VESSEL_SCREEN_DIR` to write screenshots outside
the checkout. `perf` runs on demand only (minutes, not part of `e2e`): the
board loading cold over a slow and a phone-like link, a timeline of 5,000
and 20,000 entries, find in it and going to a match far from the view, a
terminal under heavy output, a board of 500 cards, a session's code
panel over a large change and a large tree (`docs/PERFORMANCE.md` keeps
the figures). `VESSEL_PERF_OUT` names its
JSON file, `VESSEL_PERF_RUNS` the runs per measurement, and
`VESSEL_CARGO_PROFILE=release` runs the fleet on release binaries (`cargo
build --release ...`) instead of debug ones. CI uploads only the
deliberate screenshots with pairing links hidden; traces, videos and
automatic failure screenshots are disabled.

## Environment

| Variable | Means | Default |
| --- | --- | --- |
| `VESSEL_NODE` | `--node`: a book name, `ssh://[user@]host[:port]`, `user@host`, or a websocket address | the project's or the book's `default_node`, else `127.0.0.1:7077` |
| `VESSEL_TOKEN` | `--token`, for a websocket node | the book entry's token file; for a node on this machine, `~/.vessel/token` |
| `VESSEL_DATA_DIR` | `--data-dir`, this machine's data directory (the node's state; `ssh/` for shared logins) | `~/.vessel` |
| `VESSEL_CONFIG_DIR` | where the node book lives (`<dir>/vessel/nodes.toml`); tests point it at a temporary directory | the platform's config directory (`~/.config` on Linux) |
| `VESSEL_SSH` | the `ssh` program to run (tests use `fake-ssh`) | `ssh` on `PATH` |
| `VESSEL_GH` | the GitHub CLI used to download releases | `gh` on `PATH` |
| `VESSEL_CLIENT_ADDRESS_HEADER` | `vesseld --client-address-header`, behind a reverse proxy (needs a trusted proxy) | unset: the peer's address |
| `VESSEL_TRUSTED_PROXY` | `vesseld --trusted-proxy`, comma-separated | none |
| `VESSEL_RELAY_BIND` | `vesseld relay --bind` | `127.0.0.1:7078` |
| `VESSEL_RELAY_TOKEN_FILE` | `vesseld relay --token-file`, the registration token nodes present | required |
| `VESSEL_RELAY_CLIENT_ADDRESS_HEADER` | `vesseld relay --client-address-header` (needs a trusted proxy) | unset: the peer's address |
| `VESSEL_RELAY_TRUSTED_PROXY` | `vesseld relay --trusted-proxy`, comma-separated | none |
| `VESSEL_RELAY_MAX_NODES`, `VESSEL_RELAY_MAX_REGISTERING`, `VESSEL_RELAY_MAX_CLIENTS`, `VESSEL_RELAY_MAX_CLIENTS_PER_NODE`, `VESSEL_RELAY_MAX_BYTES_PER_SECOND`, `VESSEL_RELAY_MAX_CONNECTIONS`, `VESSEL_RELAY_MAX_UNAUTHENTICATED`, `VESSEL_RELAY_MAX_CONNECTIONS_PER_ADDRESS`, `VESSEL_RELAY_MAX_REGISTERING_PER_ADDRESS`, `VESSEL_RELAY_RESERVED_REGISTERING`, `VESSEL_RELAY_MAX_CLIENTS_PER_ADDRESS`, `VESSEL_RELAY_MAX_NODE_CLIENTS_PER_ADDRESS` | the `vesseld relay` limits of the same names ("Running a relay") | 128, 64, 384, 128, 16 MiB, 1000, 400, 16, 7, 16, 16, 4 |
| `VESSEL_NODE_SOCKET`, `VESSEL_AGENT_SESSION`, `VESSEL_AGENT_SECRET` | `vessel mcp` only: set by the node in each session's `vessel` MCP server | set by the node |

## Layout

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for every crate and how they
depend on each other, [docs/adr/](docs/adr/) for why,
[docs/QUALITY.md](docs/QUALITY.md) for the rules every change is held to and
where the code stands against them, and
[docs/LESSONS.md](docs/LESSONS.md) for what Vessel takes from Zed and Paseo.

## Development

```bash
cargo fmt --all
node scripts/gate.mjs
```

The runner prebuilds the test harnesses and product node before the test
clocks, then checks both the default workspace and the forge fixture feature.
CI runs the same Rust checks on Linux, Windows and macOS, with the fast source
checks on Linux. Lints are strict
(no `unwrap`, no panics, no indexing, no raw arithmetic); see the workspace
`Cargo.toml`. Changes to `web/` also need its checks ("Web UI" above),
which CI runs on Linux.

For account-free pull-request fixtures, build the separate
`vessel-forge-fixture` executable with `--features vessel-node/forge-fixture`.
It loads a bounded, typed JSON seed into the existing in-memory forge;
product `vesseld` has no fake-forge selector. Workspace checks alone skip
required-feature binaries, so the gate and CI explicitly enable `forge-fixture`
for Clippy and its launcher tests. Normal product/release builds never enable
that feature. See [ADR 0060](docs/adr/0060-a-file-seeded-forge-for-test-fleets.md)
and the [test fleet instructions](spikes/native/README.md#trying-it-against-fake-nodes-and-capturing-a-screen).

On Unix, the generated-service plist regression test also uses `python3`
with its standard-library XML/plist parser. It runs in isolated, no-site
mode and reads only synthetic test data from stdin; Python is not used by
the installed service or product binary.

## License

Vessel is **source-available, not open source.** It is licensed under the
[PolyForm Strict License 1.0.0](LICENSE.md):

- You may read the code and use the software for noncommercial purposes
  (personal use, and noncommercial organisations as the license defines
  them).
- You may not distribute it, make changes to it or works based on it, or
  use it for any commercial purpose.
- Anything else, commercial use included, needs a separate license from
  Team-WICKED.

Contributions from outside Team-WICKED are not accepted.

Copyright 2026 Team-WICKED.

## IDE context without an editor

Milestone 6 ([plan](docs/plans/backend-m6.md), [ADR 0016](docs/adr/0016-read-only-language-context.md)):
install the language servers yourself and put trusted binaries on PATH.
Vessel discovers rust-analyzer, typescript-language-server, basedpyright or
pyright, gopls and clangd. It never installs servers or downloads toolchains.

```bash
vessel node start --lsp-root /workspace/project
# equivalent daemon entry point:
vesseld --lsp-config /path/to/operator-lsp.json
```

An operator configuration can pin executable paths and a TypeScript engine:

```json
{
  "roots": [{"path": "/workspace/project", "full_analysis": false}],
  "servers": {"rust": {"program": "/opt/tools/rust-analyzer", "args": []}},
  "typescript_server": "/opt/tools/typescript/lib/tsserver.js",
  "rust_toolchain": "/opt/tools/rust-toolchain",
  "cargo_home": "/opt/tools/cargo-cache",
  "python_interpreter": "/opt/tools/python/bin/python3"
}
```

The optional tool/cache paths are useful with isolated homes; conservative
TypeScript needs a trusted `tsserver.js` (explicit, or discovered through a
global `tsserver` on PATH), never one selected from project `node_modules`.
Conservative Rust uses `rust_toolchain`, else rustup's default toolchain
(resolved once to absolute binaries, never through a rustup proxy that a
project's `rust-toolchain.toml` could redirect), else `rustc`/`cargo` on
PATH; with a custom `RUSTUP_HOME`, pin `rust_toolchain`. A pinned, linked
or path toolchain must lie outside every configured root. It analyses only
the Cargo projects it checked: the nearest `Cargo.toml` at or above the
workspace, else those one directory below it. `python_interpreter` is the
interpreter pyright runs; on Windows conservative Python needs it. Servers
run in `<data dir>/lsp/cwd` with temporary files in `<data dir>/lsp/tmp`.
Config is read once at startup. `--lsp-root` may repeat; each directory grants
Read callers access, including callers from other fleet nodes. A local
session also has its own cwd. Requests cannot change these grants.

Agents receive ten `vessel_lsp_*` MCP tools: `workspace_symbols`,
`document_symbols`, `definition`, `references`, `hover`, `implementations`,
`type_definition`, `incoming_calls`, `outgoing_calls`, `diagnostics`.
For example, `vessel_lsp_hover` takes:

```json
{
  "workspace": {"root": "/workspace/project", "language": "rust"},
  "file": "src/lib.rs",
  "position": {"line": 12, "column": 8}
}
```

Language values: `rust`, `type_script`, `python`, `go`, `cpp`. `node` optionally
selects another fleet node by name/id; that destination checks its own root
grants. Columns are 1-based Unicode scalar values, not bytes or UTF-16 units.
Only saved files are visible. List tools take `page: {"offset": 0, "limit": 50}`
(up to 200 items); sorted pages can shift if files change between calls.

Every result reports covered files, observation time, freshness and observed
indexing state. Cached diagnostics may be older than a query; an empty cache
is unknown, never proof that the workspace is clean. A work-done progress
end does not prove all indexing finished. Unsupported features report that
explicitly; results beyond resource budgets report truncation.

Conservative analysis disables Rust build scripts/proc macros/check-on-save,
uses offline dependency handling, disables TypeScript automatic acquisition,
and refuses executable or unsupported project configuration. This loses
semantic coverage, including unexpanded derives and uncached offline dependencies.
Only the operator can enable full analysis for a root, with
`--lsp-full-analysis-root` or `full_analysis: true`; it may execute repository
build tools. It covers every directory below that root, including session
cwds and fleet callers' workspaces inside it: a full-analysis parent makes
every root under it full. LSP runs as the node account: **this is not an OS sandbox**.
The per-server restrictions and residual trust boundaries are in ADR 0016.

Servers start lazily (at most four, shared between allowed roots: a root
holding fewer servers takes over the busiest root's least recently used one
and waits for it, and a burst from one root cannot take an equal root's
slot), shut down after five idle minutes, and retry after crashes with
backoff. Documents, diagnostics, queues and output
sizes are bounded. To exercise this repository through a real rust-analyzer
and an isolated agent MCP connection, run `python3 docs/smoke/m6-lsp.py`.

### Reported usage

```bash
vessel usage machine --period day       # past 24 hours; --period week is past 7 days
vessel usage agent claude --period week # all profiles; --profile selects one recorded name
vessel usage session 3f2a               # retained totals and up to 100 reports
vessel usage session 3f2a --after 100    # the next page; totals are unchanged
```

Counts and costs are only what the agent reported. Context occupancy is
separate; missing reports are not zero and costs in different units stay
separate. Cost replayed while loading a conversation establishes a baseline
instead of being counted as new spending. Accounting starts with this version, retains at most 90 days
and 100,000 reports, and follows session deletion. New usage queries need
read scope; cost in these queries additionally needs operate scope.

The web client also shows machine accounting in **Settings → Usage**.
Choose a machine and the past 24 hours or seven days, then Refresh for a
new observation. Agent/profile choices use names from the last All agents
report; each choice reads its totals from the node. All profiles includes
reports without a recorded profile name, rather than guessing the current
default login. Reasoning and cache counts keep their reported-turn
coverage, and cost stays separated by unit or explicitly hidden by access.
Offline machines and failed reads show guidance instead of zero totals.
This page does not yet show account limits or individual session reports.

A schedule preview is calculated on the target node without saving or
starting work (`vessel schedule preview --every 30m`, or `--cron` with an
optional `--tz`). Omitted `--tz` uses the CLI machine's IANA zone, as schedule creation does;
if unknown, supply it explicitly. `--after` fixes the UTC cursor for a
reproducible preview; `--count` is 1–10. API callers reuse the returned
cadence when saving so an interval keeps its preview anchor. `vessel status`
also reports the node's time zone. ADR 0033 defines the daylight-saving rules.

The page's sidebar keeps ended conversations visible until archived.
Display → Show ended sessions can hide them; a saved off choice stays off.

### Inbox and push

`vessel inbox list` shows the selected machine's waiting approvals/questions
and finished, failed or limited sessions. `vessel inbox show ENTRY` fetches one
current request, including a large question omitted from the list, without
marking it read or answering it. Use `vessel inbox answer ENTRY
OPTION` for an approval, or `vessel inbox answer ENTRY field=value ...` for a
question; `--decline` uses the existing request's decline path. `vessel inbox
read ENTRY` and `unread ENTRY` share attention with other devices without
answering the request. Entry numbers are stable for their lifetime; a request
answered elsewhere disappears, so list again if a number is no longer current.
Session read-through and archive also apply when a published event reaches
storage later: an already-read source stays read, and archived attention stays
out of the inbox.

Protocol 22 also defines paired-device Web Push registration for a future
phone/browser host. The node owns its VAPID identity, encrypts only a generic
attention payload, expires subscriptions and rechecks device access before
sending. No hosted account is required. This is the node/API side; an installed
phone subscription UI is not included. See ADRs 0040–0041 for privacy, bounds
and the dependency licence inventory.

### Account limits (explicit local opt-in)

```bash
vessel usage limits codex                  # cached facts only
vessel usage lookup on                     # administrator: enable on this machine
vessel usage limits codex --refresh        # operate: read the saved login's limits
vessel usage limits claude --profile work --refresh
vessel usage session-limits 3f2a --refresh # actual live launch's saved-login binding
vessel usage lookup off                    # disable and clear external observations
```

The default is off. Retained `usage machine|agent|session` reports still show
what the agent itself reported. An explicitly requested limit refresh reads
the configured agent's local saved access token (and Codex account identifier,
when present), and sends it only to Anthropic or ChatGPT's fixed limits
endpoint. Supported saved stores are Claude Code, Codex, and OpenCode's OpenAI
OAuth login. Vessel does not sign in, refresh tokens, scrape pages, send these
credentials through a fleet link, or store them in usage history. Read-only
devices may read sanitized cached facts but cannot refresh, change policy or
see credit balances.

Observed saved-login limits can differ from a running agent's remembered
login. The CLI preserves unknown values, distinct windows and paid-use flags;
full subscription usage alone does not prove the session cannot continue.
Reset times are human durations. Snapshots stay only in bounded node memory
and are cleared at restart or opt-out. No automatic wait/continue or account
switch is implemented by these commands. Actual vendor-account and macOS
Keychain behavior require a separate operator check; automated tests use only
offline synthetic fixtures.

Startup continuation is being prepared in separate slices (ADR 0059). Its
policy values and settings reply projection exist, with an unassigned protocol;
there is no new startup setting or automatic resume behavior in this slice.

The shared CLI presentation boundary is described in
[W17 output seam](docs/plans/wp-w17-output.md). Configured `models` preserves its
default listing and accepts explicit JSON/YAML, quiet and headerless output.

`vessel [--node NAME] node diagnostics [--json]` collects a bounded metadata
report over that node's existing authenticated connection. It is served from
protocol 24 on. Against an older peer the command refuses before sending, so no
local report stands in for the selected node.
The command's safe error output omits private connection/source details.
