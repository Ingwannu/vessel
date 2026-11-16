# Architecture

The backend as built through milestone 4 (2026-11-06). What each crate
holds now is in "State" below; each milestone's plan is in `docs/plans/`,
and the reasons are in `docs/adr/`.

## Pieces

```
  vessel CLI      agents (`vessel mcp`)      `vessel ui` (bridge)      (clients)
        \                 |                        /    \
        |                 |                       /   browser (web/, loopback)
   protocol 2..=7: typed frames over one connection, which is one of
   a websocket with the token, the local socket, SSH stdio (`vesseld
   attach`), or a Noise channel to the node's key, direct or relayed
                                  |
                       vesseld (one per machine)
     +-------------+--------------+--------------+---------------+
     |             |              |              |               |
  rpc server    session        store          fleet          secure gateway
  (principals,  runtime        (SQLite log,   (home or       (paired devices,
   scopes, the  (one actor     identity,      member;        relay links)
   envelope)    per session)   devices)       transfers,
                /        \                    secrets)
     terminal mode      structured mode
     (PTY, raw TUI)     (ACP client)
                \        /
     claude / codex / gemini / opencode, each under a profile

  vesseld relay: splices end-to-end encrypted connections, keeps nothing
```

A fleet is nodes of this shape: one is the home, the others are members
that follow it, and calls between nodes go through the home (ADRs 0012
and 0013).

## Crates

| Crate | Job | Depends on |
| --- | --- | --- |
| `vessel-proto` | Wire contract: ids, sessions, events, agent timeline, errors, typed RPC table; keys, devices, pairing links, relay control (milestone 3); validated names, profiles, creators, agent credentials, fleet manifests and MCP server definitions (milestone 4); protocol 6's envelope (`forward`: `Origin`, `fleet.forward`/`relay`/`deliver`), transfers (`transfer`, with the size limits) and secrets on the wire (`secret`: `SecretValue`, redacted); a session's place on the board (`board`, protocol 8); what a session runs with and its title (`options`, `title`: the agent's config options, choices, `SessionTitle`, protocol 9; ADR 0062 adds typed title origin with a merge-assigned protocol threshold); projects, worktrees, git and files (`workspace`: `RelPath`, `CloneUrl`, bounded names, typed outcomes and refusals, `SessionPlace`, `SessionWorkspace`, protocol 10); agents beyond the four (`agents`: `HarnessRef`, `AgentName`, protocol 11); schedules, runs, saved prompts and hooks (`automation`: `CronExpr` parsed into bit sets, bounded names and prompt text, typed run outcomes, protocol 11). No I/O. | - |
| `vessel-rpc` | Moves `ClientFrame`/`ServerFrame` over any frame transport; handshake, correlation, concurrency limits, generic over `Handler`. From protocol 5 it holds both sides to the negotiated version: a call newer than the connection (`Call::since`) is refused by the client before sending and by the server before dispatch, and outgoing frames are adapted to an older client where they are encoded (`ServerFrame::for_protocol`). | proto |
| `vessel-plugin` | Trusted node-owned standalone helper supervision (ADR 0079): bounded private frames, atomic declarations, cancellation-retained admission, private diagnostics and checked owned cleanup. No installer, source acquisition, caller authorization, broker or product Node/App dependency in this slice. | proto, proc |
| `vessel-proc` | Command specs with opt-in diagnostic privacy, spawning in a process group (a kill-on-close job object on Windows), killing the group; since milestone 11 (ADR 0031) the TCP ports a process tree listens on (`listening`: `/proc` on Linux, `lsof` on macOS, `netstat` and CIM on Windows, bounded), a terminal session's leftovers (`kill_session`, Linux) and the account's login shell. Generic selected-file kernel observations (`lock`, ADR 0001), without node layout or signal authority from a PID snapshot. | - |
| `vessel-harness` | Everything harness-specific, one module per harness: discovery, version probe, terminal and ACP launch commands; `HarnessEnv` (the account harnesses run as, given by the binaries, never read from the process by a library); since milestone 4 harness profiles and `profiles.toml`, each harness's home and login check, a session's files and linking them between homes, a profile's memory files and renaming a path for another working directory (transfers, protocol 6), MCP servers for terminal sessions and for Claude Code's direct driver (`claude_mcp_file`; every write checks its directory is the writer's own, owner-only and not a link), and what `vessel init` offers (install and login commands, other accounts' homes); since milestone 5 a profile's `model` route and pointing its harness at the node's model proxy (`routing`: Claude Code by its gateway variables, Codex by a `vessel` provider in `-c` overrides); since protocol 18 (`direct`, ADR 0057) each agent's own protocol: its arguments, the versions its driver was checked against and whether it is offered, `launch_direct` and the `--version` command. | proto, proc |
| `vessel-lsp` | Saved-file language context (M6, ADR 0016): bounded Content-Length transport, typed LSP edge, process actors keyed by workspace/language and shared fairly between allowed roots, conservative launch policy with a once-resolved Rust toolchain and checked Cargo projects, native watches and saved snapshots, progress/diagnostics, query conversion and pagination. No authentication or arbitrary LSP method passthrough. | proto, proc |
| `vessel-workspace` | Projects' work (milestone 11, ADRs 0023, 0027 and 0030): the `git` program run hardened (argument vectors, prompts, hooks and configured programs off, literal pathspecs, time and output limits, three bounded pools (local, network, clones), the version checked once; before every run in a repository its own configuration listed, the filters and merge drivers it defines turned off on the command line, and a run refused that would start a program or send a credential the repository's configuration names, `git/run/own_config.rs`); status, diffs with typed hunks, log, branches, stashes, commit, fetch, push, pull, switch, rename; worktrees, the "nothing would be lost" inspection, snapshots under `refs/vessel/snapshots`; clones into a temporary folder; every path resolved and checked against its root; file list, read, write, create, rename, delete; the name index and `git grep`; `vessel.toml` and its digest, with bounded metadata styles decoded independently of commands; setup and teardown commands with a bounded log (run without Vessel's credentials and the forge's sign-in, protocol 11); what a pull request needs of a folder (the checkout in one run, a branch's remote and its address, kept out of `Debug`, and its remote-tracking commit; protocol 11). No principal, store or session. | proto, proc |
| `vessel-pty` | Runs a command in a PTY; byte I/O and resize; on Windows, answers the pseudo console's startup cursor query. Honors the process spec's opt-in diagnostic privacy without changing execution. | proc |
| `vessel-agent` | ACP client: drives a harness or adapter over stdio and maps ACP updates to `AgentEvent`s; gives MCP servers in `session/new` and `session/load` (milestone 4); the agent's config options (`options`: reported once before ready and on every change, choices applied in the handshake, changes on a running session, ACP's older modes as a mode option) and `probe`, what an agent offers without a session (protocol 9); `history`, a page of the agent's own conversations over `session/list` without a session, behind a cursor of Vessel's (protocol 17, ADR 0053). Since protocol 18 (ADR 0057) `Driver` picks the protocol a session, a probe or a listing speaks (`spawn_with`, `probe_with`, `history_with`): ACP (`driver`) or a direct driver of its own module (`codex`, `claude`, `opencode`), behind the same seam; a driver not built yet is refused before its agent starts. `codex` (ADR 0057, slices 1 and 2): Codex's app-server over JSON-RPC lines (`wire`), the protocol subset as types checked against Codex 0.166.0's schema (`protocol`, `contract`, `schema/app-server.json`), its messages as events (`mapper`), the session's mode option with codex-acp's values (`mode`), its approvals and questions held for a person under node-minted ids (`asks`), and the session loop (`session`). A launch a direct driver cannot honour (a choice it does not offer, a fork it cannot make, its private file) is refused before anything is spawned (`LaunchRefusal`). The private launch file goes through `vessel-harness` (`claude_mcp_file`, `McpFile`), the one owner of that format and of the file's checks, hence the dependency. | proto, harness, proc |
| `vessel-session` | Session actors: owns the process, assigns `EventSeq`, keeps a ring cache, broadcasts, hands every record to the node for persistence; each session's place on the board (`board`, the reducer ported from agent-orchestrator, fed by `activity`, protocol 8, and by the pull request the node gives a live session through a watch, protocol 11); the agent's options logged as `options_changed`, titles (`renamed`, from the first prompt or a rename), the choices a session carries on, and probing an agent (protocol 9); terminals beside sessions (`terminals`, protocol 11, ADR 0031): a shell in a PTY or a script's run piped in its own process group, each an actor with a ring bounded by records and bytes, and a service's restarts with backoff; which turn a prompt started or joined, for whoever asked (`delivery/tickets`, protocol 11); parking an idle agent and waking it (`parking`, protocol 14, ADR 0038): the route a handle sends through (`Live`, `Moving`, `Parked`), each actor's own idle timer, and a park or a wake reusing a move's stop and start with the session's current choices and harness session id; rewinding (`forking`, protocol 14, ADR 0052): a move's stop and start with a fork of the conversation at a turn's end, the old conversation started again on any failure, and a fork's start (`ForkStart`: `session/fork`, its `forked` record, its check's replay dropped); one agent per conversation (`runtime/claims`, protocol 17, ADR 0053): a start that resumes a conversation or a move claims it under every profile its files are linked into, refused while another live session holds it there; which protocol drives a session (`runtime/drivers`, protocol 18, ADR 0057): the node's setting, the drivers this build offers and the installed agent's version, chosen at its first launch and kept for its life. | proto, harness, pty, agent, proc |
| `vessel-store` | Data directory (`node.id`, `token`, `node.key`, `vessel.db`, `profiles.toml`, `settings.json`: the node settings a person changed, protocol 12, `mcp/`, `uploads/`) and its locks, SQLite event log (schema 2: paired devices; schema 3: sessions' current profile and creator; schema 4: where a session came from, `origin` and `via`, protocol 6; schema 5: a session's model route; schema 6: a session's current title and the choices of its last options report, protocol 9; schema 7: the conversation's projections, the marks and attachments; schema 8: retention's order; schema 9: projects, worktrees and approved command sets, protocol 10; schema 10: pull request feedback, protocol 11; schema 11: the set an approval covered; schema 12: schedules and their runs, saved prompts, hooks with their deliveries and keys, protocol 11, a run about to be made written in the commit that takes its due time; schema 13: the search index of every session's messages, protocol 12), the `workspace/` folder of worktrees, setup logs and git's empty hooks folder, the `probe/` directory agents are probed in, `attachments/` (kept and read back without following a link, protocol 10), retention, restart recovery. | proto, proc |
| `vessel-node` | `vesseld`: config, auth, HTTP/websocket server, `Handler` implementation, persistence and subscription plumbing; devices, scopes and the secure gateway, and `vesseld relay` (milestone 3); harness profiles (`profiles`: `profiles.toml`, `profile.list`/`reload`, what `session.create` launches with, what `session.move_profile` moves to), agent principals (`agents`: a credential per session, grants, limits, ownership) and fleet sync (`fleet`: the home publishing `fleet.toml`'s sources and serving `fleet.*`, members joining, pulling, applying into every profile's home and reporting; this node's secrets) (milestone 4); transfers (`transfers`: `session.export`/`memory.export` snapshots held for whoever opened them, `transfer.read`/`close`, expiry, and `session.import`/`memory.import` pulling through a `Source`: this node, or another through the envelope as the caller) (protocol 6, F4); `envelope` and `fleet/links` (protocol 6, Track X): `fleet.forward`/`relay`/`deliver` with the delegated principal (the delivering device's scopes ∩ the origin's, ownership and agent limits applied to the origin), the mutual grant (`fleet.grant`, `fleet/granted.json` on a member, `fleet/grants/` and the binding record `fleet/bindings.json` on the home, `fleet.forget`, which withdraws the home's outstanding invitations, and `fleet.callback`, the home's relays and a relay ticket for a member with no route of its own, which `relays` links to: `fleet/links/callback.rs`), `fleet.peers`/`nodes`, and `session.read` waiting with its shares (`waits`). Model routing (`models`: `models.toml`, the loopback proxy, model tokens, limits, `ModelCall` records, `model.list`) (milestone 5, ADR 0015). Protocol 9 (ADR 0022): `service/options` (`harness.options`, `session.options`, `session.set_option`, `session.rename`) and `probes` (what each agent offers, probed once for concurrent callers and kept ten minutes). Protocol 10 (ADRs 0023, 0027): `workspace` (who may call: agents refused all but orchestration in `guard`; roots refused; the registry of projects, worktrees and approvals; clones and setups as tasks; tidying a session's worktree; a session's `place` resolved into `cwd` and notices) and `service/{projects,worktrees,git,files}`. Protocol 11 (ADR 0029): `forge` (one watcher task: sessions placed in a project or worktree read for their branch and remote, watches keyed by repository and branch, at most 64, a person's first; conditional looks with backoff; each live session's view handed to its actor; feedback through the queue; `pr.open` and `pr.merge` run in the caller's call) and `service/forge`; Protocol 11 (ADR 0031): `terminals` (where one may be opened and by whom, the limits, the environment without Vessel's credentials, the registry, a script's plan under its approval, the port scanner) and `service/{terminals,scripts}`. Protocol 11 (ADR 0033): `automation` (the scheduler: one task owning every schedule, an index of due times it sleeps on, the room it hands out in due order (four runs at once, eight scheduled sessions in their first turn), what a run came to; `time`: a cron's next wall-clock minute in a zone from `jiff`'s bundled database, daylight-saving gaps and folds, missed due times; who may change what; how a session's records read to a follower; `prompts`: saved prompts' names, bound and revisions under one lock; `notices`: the starter told, its bounds and its words) and `service/{schedules,scheduled,followed,prompts,notices}` (the calls, a run made as its definer through the service's own checks and its session followed and put away, a saved prompt's spec checked as `session.create` would check it, and a told turn followed until its starter has heard). Protocol 12: `service/search` (ADR 0037: a named session must exist, two searches at once) and `service/parking` (ADR 0038: `session.park`/`wake` through the session's handle; a drain for an update not waiting for parked sessions); `settings` and `service/settings` (`node.settings`/`set_settings`: `--park-after` with what a person saved in `settings.json` laid over it, written then applied to the runtime's watch under one lock; protocol 18's drivers, `direct` refused while this build does not offer it); `service/forking` (ADR 0052: a turn's point read from its `turn_ended` record, a fork created through `session.create`'s own path, a rewind handed to the session's actor with the count of file changes it leaves); `service/history` (ADR 0053: `agent.history` through the probe's launch, two at once, `in_vessel` from the sessions).  What a probe's handshake said of an agent reaches the node as `vessel-agent`'s typed outcomes (`readiness`, `probes`: `ProbeError`, `Disconnect`, `Refusal`; protocol 11's `agent.status`, ADR 0032), and the calls that start an agent's work through the session runtime name its inputs (`service/history`: `HistoryFrom`; `service/forking`: `ForkFrom`; the side questions: `PromptContext`); the node decides nothing about ACP itself. | proto, rpc, harness, session, store, secure, relay, fleet, client, lsp, llm, workspace, forge, proc, agent |
| `vessel-client` | Typed async client and gap-free session streams (and terminals', protocol 11: one stream generic over the log it follows); the node book; pairing and secure connections over direct and relay routes (milestone 3); the local socket with the token or an agent's credential (milestone 4); fetching a file chunk by chunk into a caller's `ChunkSink` (`fetch_chunks`, protocol 6); ADR 0083's pending workspace file SDK with finite deadlines and validated byte-page adaptation into that same sink loop; protocol 10's conversation calls, protocol 11's schedules and saved prompts, and `compose` (a question's answer from text, a queued prompt's new text) for the CLI and the agent tools (milestone 11); `comparison`, one nonclone client worker composing ordinary project/worktree/session/attachment calls with bounded input, immutable common files and passive recovery snapshots (ADR 0046). | proto, rpc, secure, relay |
| `vessel-remote` | Remote nodes over SSH (milestone 2): the `ssh` runner, platform detection, installing `vesseld` by upload, `cleanup`; the SSH connector. | proto, rpc, client |
| `vessel-secure` | End-to-end secure channel (milestone 3, ADR 0009): Noise NK through `snow`, the node key pair, routing ids and fingerprints, the record layer, `SecureChannel` (a frame transport over any binary-message carrier), the relay's possession proof. | proto |
| `vessel-relay` | The relay (milestone 3, ADR 0009): `serve` (its own accept loop; registration with token or ticket and possession proof, splicing, limits counted per IPv4 address and per IPv6 `/64`, `/56` and `/48` (`address`, public: the node's direct route counts the same way, and reads a trusted proxy's header with the same `ClientAddress`), a share of sockets kept for those that authenticated, the client-address header believed only from trusted proxies, close codes), registration tickets (`ticket`: HMAC-SHA256 under the token, minted by a fleet's home for a member's key, protocol 6), which addresses a link may dial (`reach`: `Reach::Public` resolves the relay's host at dial time and connects only to public addresses, for relays a member's home named), the node's `NodeLink` (with the token, or tickets it renews), the client's `dial`/`connect`, websocket carriers (`WsCarrier`, `AxumCarrier`) and the node's `respond` (closing with `4401`), TLS for `wss://` (rustls with `ring`). | proto, secure |
| `vessel-fleet` | Fleet bundles (milestone 4, ADR 0012): the home's sources into a manifest and a content-addressed blob store; plan and apply into harness homes with recorded hashes (`managed.json`), local edits kept with the incoming version beside them, symlinks never followed, targets linked to the sources left alone; skills named by `SKILL.md`; MCP secret references resolved at launch; transfers as file sets (`snapshot` into a staging store within protocol 6's limits, `place` into a home never overwriting: a session whole or not at all, memory beside what differs). | proto, harness |
| `vessel-mcp` | `vessel mcp` (milestone 4, ADR 0013): the MCP server (stdio, on `rmcp`) a node attaches to every session as `vessel`; each tool is one call through the session's agent principal; `list_projects`, `list_worktrees`, `create_worktree` (following setup) and `create_session`'s `project`/`worktree` (protocol 10); `get_pull_request` and `open_pull_request` (protocol 11: an agent's open never pushes; merging and feedback are not offered); `create_terminal`, `list_terminals`, `read_terminal`, `write_terminal` (marked destructive and open-world), `close_terminal` and `list_ports` (protocol 11, ADR 0031: only where both profiles set `agent_terminals`, only its own; scripts are not offered); the schedule tools, `list_saved_prompts` and `notify` on a prompt, on the agent's own node (protocol 11, ADR 0033). | proto, client |
| `vessel-llm` | Model APIs for the node's routing proxy (milestone 5, ADR 0015): Server-Sent Events decoded from any chunking and encoded (`sse`); the Anthropic Messages, `OpenAI` Responses and Chat Completions wire types, open to kinds Vessel does not know yet (`wire`); a neutral request and stream, the translators between them, a same-API passthrough and the upstream client (`client`: rustls with the platform's roots, no redirects or proxies, timeouts to connect, to the first byte and between bytes, bounded bodies, retries only before the first byte, errors classified with the key stripped). Bounded tool-free complete replies retain explicit finality and safe diagnostics. Nothing about sessions, nodes or harnesses. | proto |
| `vessel-forge` | Pull requests on a forge (milestone 11, track B2, ADR 0029): one object-safe trait (`Forge`), GitHub behind it (REST for conditional reads and writes, one GraphQL read, a merge pinned to its head), a remote's URL read into its repository, signing in with a token the binary found or the machine's `gh` (a token only in a sensitive header, never logged, kept five minutes), one bounded HTTP client (four slots, one kept for a person; same-origin redirects; 4 MiB bodies; per-host token bucket, quota floor and cooldowns), and an in-memory forge for tests (feature `fake`). ADR 0060 adds `fake/seed`: bounded, versioned files seed that same provider, with per-repository failures and refused merges. The separate `vessel-forge-fixture` node executable requires `vessel-node/forge-fixture`, explicit scratch paths and loopback listeners; product `vesseld` has no fake-provider selector. No new crate dependency edge. Nothing about sessions, the store or principals. | proto, proc |
| `vessel-web` | `vessel ui`'s server (milestone 7, ADR 0017): loopback only, one-use bootstrap and mutual session proofs, bounded pre-auth transport, `Host`/`Origin` checks and security headers, the single-page app embedded at build time (`web/dist`, or a placeholder page; each text file with the brotli and gzip forms the web build made, one chosen per request by `Accept-Encoding`, milestone 11), the browser's websocket, and the bridge to the book's nodes: one shared connection per node with its state pushed to every tab, calls forwarded as typed `Call`s, session streams (and terminals', `follow_terminal`, protocol 11) that resume, and the bridge's own ops (the book's nodes, pairing, QR codes, and the book's edits: forget, rename, default, milestone 10; the page's preferences and drafts, kept in files and pushed to every tab, and the index's first paint, milestone 11, ADR 0028). The CLI gives it the nodes, and makes the book's edits with `BookFile`, through `NodeSource`. | proto, client |
| `vessel-cli` | `vessel` binary: commands, interactive attach, `vessel mcp`, `vessel init` (first-run setup, milestone 4); `vessel models`, each profile's route in `vessel profiles` and model calls in `vessel log` (milestone 5); `vessel ui` (milestone 7); `vessel project`, `vessel worktree` and `vessel run --project/--worktree/--new-worktree` (milestone 11); `vessel pr` (list, show, open, merge, feedback, status), `vessel shell`, `vessel script`, `vessel ports`, `vessel schedule` and `vessel prompt` (protocol 11); `vessel search`, `vessel session park/wake/fork/rewind`, `vessel node settings` (`--driver`, protocol 18) and `vessel history` (protocol 12); `vessel session compare` (ADR 0046), a bounded manifest and one common input with complete partial-result reporting. | proto, client, node, remote, mcp, harness, web |
| `vessel-testkit` | Fake harnesses (`fake-tui`, `fake-acp`, which also make one model call through a routed session's proxy on `model <text>`, milestone 5), `fake-ssh` for tests and `fake-provider` for the model routing smoke; with the `relay` feature, a relay that misbehaves on command; with the `upstream` feature, a model provider on loopback answering with scripted streams, errors, drops, silences and hangs (milestone 5). The fakes report the environment, arguments, MCP servers, loaded session id and working directory they got (milestone 4). With the `authority-fixtures` feature, typed caller-authority fixtures built from `vessel-proto`'s types (protocol 20); the fake binaries keep no wire dependency. | proto |

The CLI also provides graceful `vessel node stop` (`node_ctl`, ADR 0064),
using the existing Admin-scoped RPC, authenticated local identity binding
(`node_ctl/observe`) and an OS-lock completion event (`lock_wait`) under a
bounded command deadline.
Service-file observations (`node_ctl/service_probe`) reuse the installer's
bounded template decoder (`init/service/matching`); they supply warnings,
never manager execution or authority inferred from editable file text.

The dependency direction is fixed by ADR 0001; adding an edge means updating
that ADR and this table, in the same commit. `scripts/check-boundaries.mjs`
(in the gate) compares the last column with every crate's normal and build
dependencies and fails on a difference either way. Test-only edges: `vessel-node`'s end-to-end tests
drive a node through `vessel-client` (a dev-dependency; the client does not
depend on the node, so there is no cycle), and `vessel-remote`'s and
`vessel-cli`'s SSH tests read `vessel-store`'s node lock to stop any node
they started.

### External crates with one owner

A format or a protocol has one crate that speaks it; the others ask that
crate. `scripts/check-boundaries.mjs` fails when a crate outside the second
column depends on one of these (normal or build dependencies; a test may
set up a state through another crate's format, as `vessel-node`'s e2e
`callback` does with SQLite).

| Crate | Who may depend on it | Why |
| --- | --- | --- |
| `agent-client-protocol` | only `vessel-agent`, `vessel-testkit` | ACP types never leave the client (ADR 0003); the fake agent speaks it. |
| `rusqlite` | only `vessel-store` | The event log's schema and its migrations have one owner. |
| `snow` | only `vessel-secure` | The Noise handshake and the record layer (ADR 0009). |
| `portable-pty` | only `vessel-pty` | Terminals behind one seam, with its Windows answers. |
| `process-wrap` | only `vessel-proc` | Process groups and kill-on-close jobs in one place. |
| `rmcp` | only `vessel-mcp` | The MCP server (ADR 0013). |
| `clap` | only `vessel-cli`, `vessel-node` | Arguments are parsed by the binaries (`vessel`, `vesseld`); libraries take typed configuration. |
| `tracing-subscriber` | only `vessel-cli`, `vessel-node` | A library emits events; a binary decides where they go. |
| `ts-rs` | only `vessel-proto`, `vessel-web` | The page's TypeScript is generated from the wire and the bridge's frames. |
| `tokio` | never `vessel-proto` | The wire contract has no runtime: the native app's `view` uses it and compiles to WebAssembly. |

### Names only some files may use

| Name | Crate | May appear | Why |
| --- | --- | --- | --- |
| `vessel_proc` | `vessel-store` | only `src/lock/` | Delegate the generic lock query only; no spawn, signal, group or session cleanup. |

## Data flow

1. A client opens a connection and presents who it is: the bearer token on
   `/rpc` or as the local socket's first frame (the owner, ADR 0005), a
   paired device's credential inside the Noise channel (ADR 0010), or a
   session's agent credential on the local socket (ADR 0013). It sends
   `Hello` with its protocol range and gets `Welcome` with the negotiated
   version and `NodeInfo`. Every call is then checked against its
   method's scope and, for an agent, against ownership.
2. `session.create` resolves the profile, asks `vessel-harness` for a
   launch command, and `vessel-session` starts an actor in terminal mode
   (`vessel-pty`) or structured mode (`vessel-agent`). The command carries
   the profile's environment and, for a terminal session, its MCP servers
   in the harness's launch-time form; a structured session gets them in
   `session/new` or `session/load` (ADRs 0011, 0012). The fleet's servers
   come with this node's secrets, and the session's own `vessel` server
   with its agent credential.
3. The actor turns process output into `SessionEvent`s, numbers them
   (`EventSeq`, gap-free from zero), caches them in a ring, pushes each one
   into the bounded durable channel (the node writes it to the store), and
   broadcasts it to live followers.
4. `session.subscribe{from}` sends cached and stored records from `from`, then
   live ones, never before the `Subscribed` reply (ADR 0006). A subscriber
   that falls behind, on the node or in its own client, gets
   `SubscriptionEnded{Lagged{resume_from}}` and continues with `session.read`
   plus a new subscription (`vessel-client`'s `SessionStream` does this).
   Nothing is skipped silently.
5. `session.send` routes `SessionInput` to the actor: terminal bytes and
   resizes, or prompts, cancels, permission answers and (protocol 10)
   answers to questions. The first answer to a permission request or a
   question wins; the resolution is an event, so every client sees it.
   Questions are ACP form elicitations, advertised only to the agent of a
   session the owner or a device created (an agent's runs unwatched);
   `vessel-agent` checks an answer against the question's form before the
   agent sees it.
6. `session.move_profile` (ADR 0011) moves a structured session to another
   profile: the node resolves the target, finds both homes and runs the
   target's login check (`profiles::moving`); the session's actor holds
   input, refuses or cancels a running turn, hard-links the session's
   files into the target home, stops the agent and starts it again under
   the new profile with `session/load`, then publishes `ProfileChanged`.
   The session keeps its id, log and subscribers.
7. Agents (ADR 0013): `session.create` gives the new session's agent a
   place in `agents` (its creator's scopes, at most `read` and `operate`;
   its depth) and, when the node runs `vessel mcp`, a credential, which
   goes only into the session's `vessel` MCP server definition. That
   server connects to the node's local socket with the credential as its
   first frame; its calls run as `Principal::Agent`, and `operate` calls on
   a session need the session to descend from the agent's. The credential
   and the agent's connections end with the session.
8. Fleet sync (ADR 0012): the home bundles its sources (instructions,
   skills, MCP definitions) into a manifest and a content-addressed blob
   store whenever they change. A member, paired with the home as a `sync`
   device, long-polls `fleet.watch`, fetches the manifest and the blobs it
   lacks, applies them into every profile's home (keeping local edits as
   conflicts), and reports what it did.
9. Calls across nodes (protocol 6): a client or agent asks its own node
   with `fleet.forward { node, call }`. A member passes it to the home
   (`fleet.relay`, needs `relay`), and the home delivers it to the target
   (`fleet.deliver`) as the device the target granted it at join. The target
   runs it as `Principal::Delegated`: the scopes it granted the home,
   intersected with the origin's, with ownership and agent limits applied
   to the origin.
10. Transfers (protocol 6): `session.export` or `memory.export` snapshots
    files on the source for the caller that asked; `session.import` or
    `memory.import` on the target pulls them chunk by chunk (from this
    node, or from another through the envelope as the same caller), checks
    each file's hash, renames paths for the new working directory and
    places them without overwriting anything.
11. On shutdown the node kills each session's process group, waits for final
    statuses, and flushes the store. On the next start, sessions that were live
    are marked `Interrupted`. Their logs stay readable, and structured ones can
    be resumed through their `HarnessSessionId`.
12. Schedules (protocol 11, ADR 0033): `schedule.create` checks the
    target as a run would meet it and hands the definition to the
    scheduler, one task that owns every schedule and sleeps until the
    earliest due time (at most 10 minutes at a time). When one comes due it
    decides (due, caught up, skipped), writes the run as `making` with the
    next due time in one commit; the run waits for room the scheduler
    hands out itself, oldest due first (four runs at once, eight
    scheduled sessions in their first turn), and a run task makes it
    through the service as the schedule's definer: a
    `session.create` and its prompt, or a prompt through a session's
    queue. A new session's first turn is followed to its end, and the
    session archived by the service's one archive path (its terminals,
    services and pull request watch end with it); each change of
    outcome goes back to the scheduler,
    which keeps it. At start a run left `making` is marked `interrupted`;
    an agent's schedules go when its session ends.

## Speed

Measured, not assumed: each crate's `benches/` (criterion) for the hot
paths, `vessel-cli`'s `load` test (the real node and `vessel ui` under
many sessions), and `web/e2e/perf.ts` for the page. `docs/PERFORMANCE.md`
has how to run each, the figures before and after every change made for
speed, and what did not help.

## Design rules

- The daemon is the product; clients are thin.
- Harnesses are wrapped, never re-implemented (ADR 0003).
- Wire types exist once, in `vessel-proto`.
- No panics, no unbounded queues, no untyped JSON (see `AGENTS.md`; each
  strict rule, its check and where the code stands: `docs/QUALITY.md`).

## State (2026-11-06)

### Milestones

- **Milestone 1** (one node; `docs/plans/backend-m1.md`): merged on
  `backend/m1`; CI first all green in run 42141788299. On `main` since
  2026-11-07, merged with milestones 2 to 4 from `backend/m4`.
  Checked by hand on Linux (2026-11-05) with real binaries: `vesseld`
  with the CLI ran `codex --version` in terminal mode; a structured Claude
  Code session (`claude-agent-acp` 0.100.0) answered; a `SIGTERM` stop
  left no adapter processes behind, and after a restart the session showed
  `interrupted` with its full log; `--resume` (ACP `session/load`)
  continued the conversation; `/rpc` without a token returned 401.
- **Milestone 2** (SSH; `docs/plans/backend-m2.md`, ADRs 0007 and 0008):
  merged on `backend/m2`. Checked by hand over real OpenSSH: install by
  upload, auto-start with `--bind none`, `run` over SSH, `update` refused
  while a session was live, a clean stop.
- **Milestone 3** (relay and pairing; `docs/plans/backend-m3.md`, ADRs 0009
  and 0010): built on `backend/m3`. Checked by hand on one machine
  through a local relay (pairing, every command through it, revocation).
  By hand, 2026-11-07: the same over `wss://` with the relay on a public
  host behind an HTTPS proxy (milestone 3's plan, "Smoke on one machine").
- **Milestone 4** (the fleet; `docs/plans/backend-m4.md`, ADRs 0011 to
  0013, protocols 5 and 6): Tracks H, F and X are merged on `backend/m4`,
  green on Linux, macOS and Windows. An independent security review of
  the fleet, the envelope, relay tickets and the relay's and node's
  admission limits ran eleven rounds and closed on 2026-11-06; its
  findings, fixes and accepted residuals are recorded in the plan. Both
  manual smokes passed on 2026-11-07 (three real accounts; a fresh machine
  joining), recorded under the plan's done criteria.
- **Milestone 7** (the web UI; `docs/plans/backend-m7.md`, ADR 0017,
  design under `docs/design/web-ui/`): built on `backend/m7-web`.
  `vessel ui` (crate `vessel-web`) serves the page embedded from `web/`
  on loopback and bridges it to the book's nodes; the page is React with
  TypeScript types generated from the wire (`web/src/wire/`). Checked in
  CI by Playwright against real nodes with the fake harnesses, which also
  captures every main screen.
- **Milestone 8** (the web UI's design; `docs/plans/backend-m8.md`,
  ADR 0018): built on `backend/m8-board`, in `web/` and the docs only.
  White and black, one token file and one stylesheet
  (`web/src/ui/tokens.css`, `app.css`), and Sessions as a board whose
  lanes are the protocol's session statuses (`web/src/data/board.ts`).
  No crate and no wire type changed.
- **Milestone 9** (agent-orchestrator's board; `docs/plans/backend-m9.md`,
  ADRs 0019 and 0020, protocol 8): the node places each session on the
  board (`vessel-session`'s `board` and `activity`), and the web UI took
  agent-orchestrator's shell and board. On `backend/m9`.
- **Milestone 10** (controls, a composer, settings, and what a session
  runs with; `docs/plans/backend-m10.md`, ADRs 0021 and 0022, protocol 9):
  in progress on `backend/m10`. Track C: the agent's options passed on
  (probe, choices at creation, changes, reports), and session titles.
- **Milestone 11** (the references' features; `docs/plans/backend-m11.md`,
  protocol 10): track A, the conversation (ADRs 0024 to 0026, store
  schemas 7 and 8), and track B, projects, worktrees, git and files
  (ADRs 0023 and 0027, crate `vessel-workspace`, store schema 9), built
  on `backend/m11-conversation` and `backend/m11-workspace` and merged
  for the node on `backend/m11-int-node` (the plan's "Integration
  (node)").

### The web UI (`web/`)

```
web/src/wire/     generated from the Rust types (proto.ts, bridge.ts) and typed call helpers
web/src/bridge/   the websocket to `vessel ui`: mutual proof, tab session key, typed calls, resumed
                  follows; frames narrowed by predicates (frames.ts), replies by their method; the
                  socket's lifecycle (socket.ts) apart from the frames (client.ts); ids.ts, the one
                  place a plain string or number becomes a branded handle
web/src/data/     stores (useSyncExternalStore), the timeline reducer (tool calls and permissions
                  found through its index by id), the timeline's window arithmetic and where
                  going to a row puts it (window.ts), which entries are its rows once tool
                  calls fold (rows.ts: only keys cross from the reducer's indices to the
                  window's) and an entry as spoken text, the cards' shared clock, formatting, the terminal
                  adapter, the theme, the palette's entries and the forms' choices as pure functions;
                  every call a screen makes as a typed function giving one `Outcome` (outcome.ts:
                  calls.ts for sessions, book.ts for the book's edits); what /new chooses
                  (new-session, agent-choices, options, picks) and what Settings decides
                  (machines, pairing, agent-status, devices, sync, welcome), each decision once;
                  milestone 11's track W: the preferences the bridge keeps (prefs, prefs-change,
                  theme) and the drafts (drafts), the shortcuts' registry, matcher and edits
                  (shortcuts/), find and the outline (find, outline), recall, notices and alerts,
                  the sidebar's view (sidebar), a machine's remembered start (defaults), and
                  records keyed by a handle walked with their keys' type (records);
                  workspace/ (milestone 11's track B-page): one module per subject of
                  protocol 10's projects, worktrees, git reads, git writes and files (find.ts,
                  the one wrapper of `file.find`), each machine's projects (project-store,
                  following a clone), and the pure parts: where a session runs (place,
                  branch-name), the commands to approve compared with the folder's (command-
                  changes), git's refusals and a worktree's states in words (git-words,
                  worktree-words), setup's output as lines (setup-log), a diff as numbered rows
                  (diff-rows) and the Changes list (changes), the file tree (file-tree), the
                  git button's action (git-quick), a selection as a draft's reference
                  (reference), and the lexer for syntax colour (syntax, syntax-languages)
web/src/features/ an area's components and hooks, each stylesheet beside its component: the shell
                  (frame, sidebar), compose (/new, the composer), session (the timeline drawn
                  through a window: only the rows near the view exist; find and the outline go
                  to a row by key through it, drawn or not, and window-pin.ts holds it in place
                  while heights are measured), board, settings/ (the
                  frame and its sections, General, Appearance, Shortcuts with its recorder,
                  Notifications), machines/ (Machines, Agents, the add-machine dialog),
                  projects/ (Settings › Projects, the sidebar's projects, adding one, the
                  approval of its commands, removing a worktree that would lose work),
                  workspace/ (the code panel, a chunk loaded when Changes or Files opens: the
                  diff in the timeline's window, git's bar, menu and dialogs, the tree, the
                  file viewer and editor, "Add to chat"; and the branch's chip and a
                  worktree's setup card, which the entry and /new use), compose/'s tray and
                  the worktree's start (where-tray, where-place, use-starter,
                  start-worktree), the session's panel as tabs (inspector, panel-state),
                  welcome/, shortcuts/ (the one keydown listener and each action's handler),
                  notifications/ (notices, the tone), the command palette (the shell's, with its
                  shortcut; the sidebar's row only opens it), the theme switch, the kit's
                  gallery at /kit
web/src/routes/   the screens, each composing features: home (welcome or board), the board, a
                  session, new session, settings (/settings/<section>), the old addresses'
                  redirects; a session, new session and Settings (and /kit) are chunks of their
                  own, loaded when they open (milestone 11)
web/src/ui/kit/   the controls (ADR 0021): one component per file on Base UI, its CSS beside it
web/src/ui/       the design's tokens, the fonts, the toasts, and the older areas' CSS in styles/
                  (one file per area, imported in order from styles/index.css)
web/src/testing/  tests' helper that names a fixture's handle without a guard (tests and fixtures only)
web/scripts/      the build's integrity pass (tags and the import map), its compression pass
                  (every text file but the index, which is painted per request), and lint's
                  checks (fonts, controls, file size)
web/e2e/          Playwright: a fleet of real `vesseld` nodes with the fake harnesses, the tests
                  (timeline.spec.ts: find and the outline over the window, 20,000 entries),
                  `screens` (the main screens, the session's states, Settings and the
                  welcome, track W's screens, and /kit, in both themes) and `perf` (the page's
                  speed and find's, on demand, docs/PERFORMANCE.md)
```

The layers import one way: `wire` <- `bridge` <- `data` <- `features` <-
`routes`, and `ui/kit` and `ui/styles` import none of them; only the kit
imports `@base-ui/react`. ESLint's `no-restricted-imports` holds the
directions, and `npm run lint` also runs the strict type rules (no type
assertion outside xterm's adapter, no inline suppression, no implicit
truthiness), `check-controls.mjs` (no native select, input, textarea or
`title` outside the kit), `check-fonts.mjs` and `check-size.mjs` (300
lines of TypeScript, 400 of CSS, except a list of files still to divide
that may only shrink; the list is empty since milestone 11). Milestone
10's plan has the rules. `src/testing/` may be imported only by tests
and `src/data/fixtures/`, which lint also holds.

**Branded handles.** The generated declarations brand every handle a
node, the bridge, a harness or the page's bridge client mints:
`SessionId` is `string & { readonly __brand: "SessionId" }`, so a
session's id cannot be passed where a machine's key is wanted, as in
Rust. The brand exists only in the type; the wire is unchanged. What a
person types or names (a book name, a profile, a path, a title, a URL,
a key binding), quantities, secrets and content stay plain.
`vessel-proto`'s `ts/aliases.rs` classifies every primitive alias in one
table, with the rule; the generator fails its test on an alias in no
class and on an entry no file declares any more, so a new alias is
classified in the change that adds it (PROTOCOL.md, "TypeScript
declarations"). On the page a frame's handles come typed with the frame
(`isServerFrame`); anything else becomes a handle only through a guard
of its shape in `bridge/ids.ts` (a session's UUID, a book name or
`_here`, 32 bytes of hex, a positive counter): the address bar's
segments (`routes/session.tsx`, `data/where.ts`), the tab's stored
session, the client's counters. Features may not import `bridge/`, so
no component mints one.

**What the bridge keeps for the page** (milestone 11, ADR 0028). The
page's origin is new on every launch, so its own storage starts empty;
the bridge keeps its preferences and drafts instead. `NodeSource::
preferences` says where (`PageFiles`): the CLI puts
`ui-preferences.json` beside the node book (`--config-dir` moves both)
and `ui-drafts.json` in the data directory; an error finding the place
is not folded into "nowhere", and the page says the choices are kept
for this launch only. `keep` runs one task per file, owning its state:
a tab's `preferences`, `prefer`, `drafts`, `draft` and `save_draft` ops
go to that task's bounded queue from the tab's read loop, in order; a
change reads the file right before it and replaces it whole (0600 in a
0700 directory, a temporary file renamed over it, never through a link
at the name), keeping unknown keys and values it cannot read; a file
from a newer Vessel is left alone. Every state of the preferences is
published with a rising revision on a `watch` and pushed to every tab
as a `preferences` frame (the latest only), which is also how two tabs
agree: the later change wins, and the page replaces its guess with what
the bridge answers. `paint` writes the kept theme, sizes and width on
the served index's `<html>` from that memory, so the first paint is
right with no inline script (the CSP allows none). Drafts are saved at
most once a second, merged meanwhile; the page saves one half a second
after typing stops and when it is hidden. `chrono` (already the
protocol's) times a draft's last save.

The page talks only to `vessel ui`'s `/api/ws`: `call` frames carry the
protocol's own `Call` to a node, `follow` frames stream a session's
records, `op` frames ask the bridge itself, and `node` frames push each
node's connection state (`vessel_web::wire`).

Each launch uses a fresh random `.localhost` origin so persisted HTML and
service workers from an earlier port owner cannot intercept it. The UI needs
a browser supporting these names (Chromium is exercised in CI).
The launch's private Unix handoff/printed URL carries a one-use bootstrap,
not the tab's session key. `auth` and `socket` verify role-separated HMAC
proofs and fresh nonces before starting `bridge`; `transport` counts sockets
from TCP accept through upgrades and enforces HTTP deadlines. The browser
keeps the issued session key in `sessionStorage` for reloads. A changed
launch or bad proof stops the tab. `files` serves content ETags, and
content-hashed `assets/` files as immutable for a year (milestone 10;
`index.html` is `no-store`), each text file in the encoding `encoding`
picks from the request's `Accept-Encoding` among the forms the web build
compressed once (milestone 11; an entity tag per encoding, `Vary:
Accept-Encoding`), but for the index, which `paint` writes into per
request and so is sent as built, uncompressed, with no `Vary` (a form
made at build could not carry the paint; `files::index`); the web
build's `scripts/integrity.mjs` closes the
script/stylesheet SRI graph, which is what keeps a poisoned cache from
running: tags carry `integrity`, and the chunks loaded later are pinned
by the index's import map, which `guard`'s policy admits by its hash
(ADR 0017's amendment of milestone 11). `registry` validates the current book before
allocating entries and cancels removed nodes. Pairing deliberately dials
the supplied link before adding its entry. The page's `forget`, `rename`
and `default` ops are `vessel nodes remove|rename|default` through
`NodeSource`: the book is opened right before each edit, then read again
by the registry, which closes a removed node's connection and ends its
streams as `removed`. Node names are parsed when a frame is decoded
(`wire::NodeKey`, `wire::BookName`, the book's `NodeName` inside), so
this machine's node, which is not a book entry, cannot be forgotten or
renamed, and a frame that does not decode is still answered by its id.
`bridge` keeps the tab (its socket, the frames it reads and writes,
node states); a call's forwarding is `calls`, its followed sessions
`streams`, its ops `ops`. A pairing link on the wire is a `SecretText`,
never printed by `Debug`. See ADR 0017 for transcripts,
bounds and platform tradeoffs.


### By crate

In the crate table's order.

- `vessel-proto`: the wire contract, versions 2 to 11 (`docs/PROTOCOL.md`).
  Protocol 2, milestone 1: typed RPC, the event log, the ACP-shaped agent
  timeline. 3: `node.stop` and `NodeInfo.state`. 4: devices, scopes on
  every method, the node's public key. 5: profiles, `session.move_profile`,
  creators, agent credentials, fleet manifests, `fleet.*` and MCP server
  definitions. 6: the `relay` scope, the envelope (`forward`: `Origin`,
  `fleet.forward`/`relay`/`deliver`), `fleet.grant`/`forget`/`peers`/`nodes`,
  `session.read` with `wait`, transfers (`transfer`, with the size
  limits) and secrets (`secret`: `SecretValue`, redacted in `Debug`).
  7: the LSP rows and model routing. 8: `SessionInfo.board` (`board`:
  `KanbanColumn`, `DisplayStatus`, `Activity`; from 10 an ended session
  is `SessionBoard::exited` until archived, `SessionBoard::archived`
  after, one definition for the runtime and the store). 9: the agent's
  config options (`options`:
  `SessionOption` with a closed category and kind, `unknown` for a later
  kind; `OptionChoice`), `SessionTitle` (parsed: one line, at most 120
  characters; from a prompt, cut at 80), `options_changed` and `renamed`
  with their stand-ins, `harness.options`, `session.options`,
  `session.set_option`, `session.rename`. 10: the conversation
  (`conversation`: delivery and the queue, the agent's commands and
  context, questions, attachments, marks; `SessionInfo.marks` and
  `abilities`), and projects, worktrees, git and files (`workspace`),
  `SessionSpec.place` and `SessionInfo.workspace`. 11 (track G, ADR
  0032): agents beyond the four (`agents`: `HarnessRef`, the four or an
  added agent by `AgentName`, in `SessionSpec`, `Harness`, profiles and
  `harness.options`; `Harness.terminal`; what an older peer is handed,
  `agents/older.rs`: lists without added agents, `Reply::withheld` for a
  reply about one, applied where frames are written; `agent.list`,
  `agent.add`, `agent.remove` and their types, `agents/added.rs`, with
  the bounded texts of `agents/text.rs`).
  Every method has a first version (`since`), and a call that uses a
  later field needs that version (`Call::since`, `rpc/since.rs`; the
  table's macro is `rpc/methods.rs`, the table itself stays in `rpc.rs`).
  No I/O.
- `vessel-rpc`: handshake, typed calls with cancellation and an in-flight
  bound (calls beyond it are refused `Unavailable`; the connection keeps
  being read so `Cancel` gets through), per-connection outbound queue
  with bounded flush batching, subscription ordering, release and lag on
  both ends (ADR 0006), in-memory transport for tests, and `Framed`
  (length-delimited frames over any byte stream: SSH stdio and the local
  socket). The `Welcome`'s `NodeInfo` is read from a watch, so a draining
  node says so. A connection the server closes still answers the calls
  already running (within the close timeout) and writes every frame it
  took off its queue, so `node.stop{now}` gets its `ack`. Both sides hold
  to the negotiated version: a call newer than the connection is refused
  by the client before sending and by the server before dispatch, and
  outgoing frames are adapted to an older client (`ServerFrame::for_protocol`).
- `vessel-proc`: `CommandSpec`, `spawn` as a process-group leader,
  `ProcessGroup` (Unix). Stopping signals the whole group, and the group
  is swept once its leader exits (`signal_with_leader`: an `EPERM` from a
  leader that has begun exiting counts as gone only once the group is).
  On Windows each child starts suspended in its own kill-on-close job
  object (`process-wrap`), so stopping ends the whole tree: an npm `.cmd`
  shim's `cmd.exe`, the adapter it started, and their children.
  Milestone 11 (ADR 0031): `listening::scan` (the TCP listeners of given
  process trees, without privilege and within `ScanLimits`),
  `kill_session` (Linux: what a terminal's shell left in its session) and
  `account_shell`.
- `vessel-harness`: one descriptor module per harness, `Lookup` (explicit
  search path), concurrent `discover` with a version-probe timeout, and
  `launch` (the working directory absolute and existing, the home
  directory when none is given). `HarnessEnv` is the account harnesses
  run as (the user's home and the harness home variables), given by the
  binaries and never read from the process by library code: every
  in-process node a test starts gets a temporary one. Milestone 4:
  `profile` (`Profile`, the implicit default profile per harness,
  `parse_profiles`, typed errors that name the profile and key and never
  quote values); `home` and `login_check`; `layout` (where shared
  instructions and skills live in a home); `sessions` (`session_files`,
  `link_session`: hard links, never `memory/`, never through a symbolic
  link, conflicts found before anything is linked); `memory_files` and
  `rebase` (a path renamed for another working directory: Claude Code's
  project directory; Codex's dated rollouts stay); `mcp` (`McpFile`, the
  owner-only per-session file). No MCP value is ever an argument: Claude
  Code reads an owner-only `--mcp-config=` file, Codex gets `-c
  mcp_servers.<name>` overrides that name variables in its environment.
  Protocol 11 (ADR 0032): `added` (`agents.toml` parsed by
  `parse_agents`, with the file's bounds; `add_agent`/`remove_agent`
  change its text in place with `toml_edit`; `added/command.rs` builds an
  added agent's command from an empty environment, `PASSED_ON` being the
  system variables it gets), `discover_added` (found by path, never run
  for a version), and a `Profile` whose base is one of the four or an
  added agent (`parse_profiles_with`; the profile module is
  `profile/{parse,tests}.rs` under `profile.rs`); `login_status` reads
  Claude Code's and Codex's status commands' output into a typed answer
  (`binary` names the program `agent.status` looks for).
  `setup` holds what `vessel init` offers (install and login commands,
  other accounts' homes). The `fakes` feature installs testkit binaries
  under real harness names. Model selection is not here: it goes over ACP
  in `vessel-agent`.
- `vessel-workspace` (milestone 11, track B): `git` (`Git::probe` finds
  the program and checks its version once; `run` holds the hardened
  invocation of ADR 0023 and the two pools; one module per area: `status`,
  `diff` and `patch` (scopes resolved to git's sides, hunks parsed and
  bounded), `history`, `write`, `clone` (temporary folder, progress,
  cancel), `worktree` (add, the inspection, snapshots, removal), `search`
  (`ls-files`, `check-ignore`, `grep`), `refusal` and `redact` (git's
  messages typed, user information stripped, then cut), `gitfile`
  (whether a worktree's `.git` and its repository's record name each
  other); a call's wait for a slot within its time); `paths` (`Root`:
  every path joined part by part, resolved with links followed, refused
  outside the root or under `.git`, one answer, "not there", for
  outside, not followable and missing); `regular` (a file opened
  without waiting and without following a last link, then judged a
  regular file on the handle: reads, versions, `vessel.toml`, a
  gitfile); `files` (list, paged read, atomic write against an expected
  version, create, rename, delete; a blob paged as a file); `find` (the
  name index and its cache: 500,000 paths, 8 roots, idle ones dropped,
  rebuilt in the background when stale, cancellable searches);
  `project_file` (`vessel.toml`, read only as a bounded regular file
  inside its folder, its digest); `commands` (setup and teardown through
  the platform's shell, one process group, a bounded log, the wait for
  its end within its deadline and cancel). Tests run against temporary repositories with the real git and a
  configuration of their own.
- `vessel-pty`: `PtyProcess` over `portable-pty`, with dedicated reader,
  control (input and resize, in order) and waiter threads behind bounded
  channels. On Windows it answers the pseudo console's startup cursor
  query itself (`startup`), and a session terminates only its direct
  child; the rest end when the pseudo console closes.
- `vessel-agent`: ACP v1 client on `agent-client-protocol` 5.0.
  `AgentProcess` (own process group), `AgentInput` (prompt, cancel,
  permission answers with typed refusals, `turn_running`), `AgentOutputs`
  (bounded). A single driver loop owns the session; handlers forward into
  one ordered channel. The model is chosen through the `model` config
  option. The handshake has a deadline (60 s by default, `vesseld
  --agent-handshake-timeout`); a stuck adapter fails the session. MCP
  servers go in `session/new` and `session/load` (HTTP servers only when
  the agent advertises them). The `mapper` is pure and tested against
  fixtures in `crates/vessel-agent/tests/fixtures/acp/`: session updates
  and content in `mapper.rs`, the prompt side in `mapper/prompt.rs`, tool
  calls and permissions in `mapper/tool.rs`, questions and reports in
  their own. Protocol 9
  (`options`, ADR 0022): the driver keeps the agent's options as ACP
  reported them, applies a session's choices in the handshake (in order,
  after the model; one not offered or refused fails it), reports them as
  `AgentOutput::Options` right before `Ready` and whenever they change
  (`AgentInput::set_option`, run as a task like a prompt, or the agent's
  `config_option_update` / `current_mode_update`), and makes a mode
  option from ACP's older `modes`. `probe` starts an agent, takes the
  options from its handshake and stops it, bounded by the handshake
  timeout. Protocol 11 (`introduction`): what `initialize` said of the
  agent (`agentInfo.version`, its `agent` sign-in methods, bounded) is
  reported once (`AgentOutput::Introduced`, which a session ignores and a
  probe keeps), and a failed handshake carries a typed `Refused` beside
  its message (another ACP version, `auth_required`, a step's failure, its
  time limit).
- `vessel-session`: `SessionRuntime`, `SessionHandle`, `Follow`, records
  shared as `Arc<EventRecord>`; terminal and structured actors share the
  `lifecycle` module (stop-responsive publishing, two-step stop).
  Structured status is `Starting` until the agent's `Ready`, then
  `Running`; one that never became ready ends `Failed`, unless it was
  killed while starting. An ended session stays (ring included) until the
  node removes it once the store has the whole log. Milestone 4:
  `create_with(spec, LaunchOptions)` (profile, MCP servers and directory,
  creator, origin, the id chosen by the node so an agent credential is
  bound before the agent starts), and `move_profile` (`moving`): input is
  held for the move, a running turn is refused unless forced (then
  cancelled, waited for up to 10 s), the files are linked while the old
  agent runs, the agent is stopped and started again with
  `session/load`; if the new one does not start, the old command starts
  again on the same conversation and the move fails without
  `ProfileChanged`. Milestone 9 (ADR 0019): every committed record also
  passes the session's activity tracker (`activity`), and `board::derive`
  (agent-orchestrator's reducer, ported with its tests) places the
  session; the info watch wakes on a new lane, phrase or activity only.
  Protocol 9 (ADR 0022): the agent's option reports are logged as
  `options_changed` (also during a move), an untitled structured session
  takes its title from its first prompt with text, `SessionHandle::rename`
  publishes `renamed` through the actor (terminal sessions too),
  `SessionHandle::choices` is what a move or an export carries on (the
  last report's values, or the spec's), and `SessionRuntime::probe` asks
  an agent what it offers. Protocol 10 (ADR 0026, `again`):
  `SessionRuntime::start_again` runs an ended structured session's agent
  again with `session/load` (a new session's launch, the move's replay
  dropping), its ring and publisher starting at the stored `next_seq`; it
  publishes nothing until the node opens its `Gate`. Attachments (ADR
  0025, `attachments`): the actor checks a prompt's attachments (the
  per-prompt bounds, a kind its agent said it does not take) before it
  queues or sends it and, right before it sends it, asks the node's
  loader (`AttachmentLoader`, a bounded channel given with
  `SessionRuntime::with_attachments`) for their bodies through `pump`, so
  it never blocks on file I/O; the agent's driver checks the kind again
  against its own handshake and puts each body where its reference is
  (`vessel_agent::Attachments`: an image or audio inline, a file as a
  `resource_link`), while the log keeps the reference.
- `vessel-store`: an owner-only data directory (Unix: 0700, an existing
  one tightened or refused if another user's; secrets 0600), the SQLite
  event log with a batching writer thread, paged reads, restart recovery
  to `Interrupted`, retention, `user_version` migrations. `node.id`,
  `token`, `node.key` (32 bytes, created on first start and never
  replaced, wiped from memory when dropped); `NodeLock` (one node per data
  directory, an OS lock with the holder in `vesseld.owner`) and the
  `StartLock`; `profiles.toml` (`DataDir::write_profiles`, owner-only and
  atomic). Schemas: 2 adds `devices` (only the SHA-256 of each
  credential, compared in constant time; a revoked device stays as a
  tombstone), 3 each session's current profile and creator, 4 where a
  session came from (`origin`, `via`), 5 a routed session's model route
  (`model`, milestone 5), 6 a session's current title (`title`, following
  its `renamed` records, read back as `SessionInfo.title`) and the choices
  of its last options report (`choices`, `Store::choices`), protocol 9;
  7 the conversation's projections and the marks (protocol 10, ADR 0026:
  `Store::mark`, `unread` derived from the last record asking for
  attention and the last `read`, `KeptMarks::marks`), and
  `Store::delete_session`; 8 retention's order (archived ended sessions
  pruned first, pinned last) as an index the pruning walks; 9 projects,
  worktrees and approved command sets (`workspace`: each a row holding
  its wire type, written whole; protocol 10, ADR 0027). `Store::pruned`
  counts the sessions retention removed since the store opened, raised
  after each commit that removed some, so the node can tidy what they
  left outside the store (their worktrees).
  Schema 13 (protocol 12, ADR 0037) is the search index (`search`): the
  writer turns each append's prompts and runs of reply chunks into plain
  rows of `search_messages` inside the append's savepoint (`index`; the
  run still open kept in memory and mirrored by the session row's
  cursor), and, only while its queue is empty, runs bounded idle steps
  (`idle`): taking rows into the FTS5 trigram table, deleting what removed
  sessions and pruned records left (`search_gone`, filled by triggers),
  and catching up sessions whose index is behind their log (those stored
  before schema 13). `Store::search` answers a page (`query`, newest
  first by the pieces' ids, which are their times; bounded in what it
  reads); `Store::indexed` waits until no idle work is left.
  `Store::ended_session` and `Store::reopen` start an ended session again
  (protocol 10): the row is live again, without an end and out of the
  archive, before its next run writes, so retention never takes it. A deleted or
  pruned session's folder under `DataDir::attachments_dir` goes once its
  deletion is committed. Attachments (protocol 10, ADR 0025,
  `attachments`): `Store::keep_attachment` moves a finished upload to
  `attachments/<session>/<id>/<name>` without following a link (Unix:
  folders opened relative to each other with `O_NOFOLLOW`, through
  `rustix`), checks its size and SHA-256, then records the row for a live
  session only (a refused row takes the file back);
  `Store::read_attachment` reads one back only as the regular file of the
  recorded size and SHA-256, hashing it again only when its identity
  (device, inode, size, times) changed since the caller last verified it;
  at open, every folder no row names is removed. `DataDir::probe_dir` is where the node probes
  agents. `agents.toml` (protocol 11, ADR 0032: commands the node runs)
  is refused when another user owns it or may write it or the data
  directory (`DataDir::read_agents`), and written atomically, 0600. `models.toml` is read with whether other users
  can read it (`DataDir::read_models`), which the node refuses.
- `vessel-node` (`vesseld`):
  - The service (`service`): one inherent method per call, routing to the
    module that owns the work and mapping its error to the wire once; its
    calls in a module per area (`sessions`, `profiles` with the agents'
    ownership rule, `fleet` with this node's secrets, `transfers`,
    `options`, `conversation`, `again`, `projects`, `worktrees`, `git`,
    `files`, `agents` for protocol 11's); `service.rs` keeps the
    service's parts and a connection's state (its protocol among them).
  - Added agents (protocol 11, ADR 0032): `added` changes `agents.toml`
    one change at a time (`NodeProfiles::editing`), parsed with
    `profiles.toml` before it is written, and refuses an agent principal
    or a delivered call; `NodeProfiles` reads both files together. The
    launcher (`profiles/launch.rs`) gives an added agent's session its
    default profile, its secrets from `secrets.toml` (`McpSource::secret`)
    and only the fleet's MCP servers it names.
  - `agent.status` (`readiness`, `service/status.rs`): installed facts on
    every call; sign-in facts only on a person's `refresh`, from the
    agent's status command (Claude Code, Codex; never beside a running
    session of that profile) or a probe's handshake (the others), kept
    per agent and profile for five and ten minutes. `probes` keeps each
    probe's options and what its handshake said together (`Answer`), so
    `harness.options` and `agent.status` share one probe.
  - `agent.install` (`install`): the last install as a job of the node's
    (`vessel_proc` process group, 15 minutes, the first 1 MiB of output
    kept with a `watch` for waiting reads), started from
    `vessel_harness::installer` (the vendor's command in the clean
    environment plus `npm_config_*`); while it runs, the service refuses
    to start that agent (`refuse_installing`).
  - `agent.catalogue` (`catalogue`): the node's copy of the ACP registry,
    fetched only on a person's `refresh` (`catalogue/fetch.rs`: `reqwest`,
    https or loopback http, bounded) and read entry by entry
    (`catalogue/registry.rs`: exact pins only, the four told by their npm
    packages); `agent.add` from the registry takes the entry from that
    copy.
  - Sessions: axum `/health` and token-checked `/rpc`, the `Handler`
    routing to `sessions` (live runtime sessions plus stored ones; a
    session whose log is complete is served from the store, and a live
    one serves from its ring the records retention already pruned),
    `persist` (one task per session drains its durable channel into the
    store), `subscribe` (ring, then a durable-gated store gap fill, then
    live; `Lagged` when a subscriber falls behind), and an ordered
    shutdown in which subscribers still receive the final `Interrupted`.
    Marks (protocol 10, `sessions::marks`): applied in the store first,
    the store's result handed to a session the runtime still has
    (`SessionHandle::set_marks`, which `SessionHandle::info` adds), under
    one lock so the two never disagree; `archive` stops a live session
    and answers once it has ended. `session.delete` takes only a session
    the store serves. Starting again (`sessions::again`,
    `service::again`): one call per session, the session started gated,
    made visible and its row reopened under the marks' lock, then
    published and persisted; the call answers once its agent is ready. A
    subscription the store serves ends `Lagged` when its session started
    again meanwhile. A session's agent credential and model token are
    retired in its hand-over to the store.
  - Projects, worktrees, git and files (protocol 10, ADRs 0023 and 0027,
    `workspace`): the registry (projects, worktrees, approvals; read back
    at start, a clone or setup the stop interrupted told as failed),
    behind one lock never held across an `.await`; roots refused
    (`policy::check_root`: a file system's root, any folder that is or
    holds the home folder, any folder holding the data folder or inside
    it; `project.init` checks the folder it would make before making
    it); its folders absolute, so the empty hooks folder git is pointed
    at is never read relative to a repository; every git or file call's
    root resolved again (`roots`: still the folder recorded, and a
    worktree's `.git` still naming its repository's record of it,
    `git::gitfile`); clones and setups as tasks of the node's; `place`
    (a session's `SessionSpec.place` resolved into `cwd` and its log's
    first notices; an agent only in a worktree it made or one made for a
    session it may operate, and never with a `cwd` of its own in the data
    folder); `worktree.create`'s limits counting the creations under way;
    `retire`, the removal rule tidying follows (a worktree git could not
    inspect is kept and left for a person). The service only routes
    (`service/{projects,worktrees,git,files}`); agents get only
    orchestration's calls, decided for every method by one exhaustive
    `match` (`policy::refuse_agents`, run by `guard` after the scope).
  - Tidying (`tidying`, where the two halves of protocol 10 meet):
    `session.delete` deletes the session, then removes the worktree made
    for it by `retire`'s rule and answers what became of it
    (`SessionDeleted`); a sweep removes the worktrees of sessions that are
    gone (neither the runtime nor the store has them), at start and after
    each prune `Store::pruned` reports, and, for those kept because a
    session was in them, after each deletion. One tidying at a time under
    a tokio mutex held across the deletion and the removal. Ending or
    archiving a session tidies nothing; `session.resume` refuses a session
    whose folder is gone, or whose worktree a setup, teardown or removal
    holds (`conflict`).
  - Attachments (protocol 10, ADR 0025, `attachments`): uploads
    (`uploads`, `staging`: one owner-only staging file each under
    `uploads/`, appended and hashed piece by piece, held by whoever began
    it, 16 on a node and 4 a caller, a declared size counted against its
    session's 100 MiB and the node's 4 GiB until kept, dropped after 10
    idle minutes and at every start), `attachment.finish` (all bytes, an
    image's first bytes, the session still live, then the store keeps
    it), `attachment.read` (the store's read-back, a piece at a time,
    `Verified` remembering what was hashed), a prompt's references made
    the store's records where a prompt enters the node (`load::resolve`:
    `session.prompt`, a queue `edit`, `session.send`), and the loader
    sessions ask for a prompt's bodies (`load::serve`: base64 for images
    and audio, a `file:` URI for a file, each read back on the blocking
    pool).
  - Transports: the local socket (Unix socket 0600, or a named pipe on
    Windows) whose first frame is the token or an agent's credential;
    `--bind none`; `vesseld attach`, the byte-pumping proxy SSH runs,
    which starts the node through a login shell when none answers;
    `vesseld version` and `cleanup`; `node.stop` (`now`, or drain).
  - Devices (milestone 3): every connection has a principal (`Owner`,
    `Device`, `Agent`, `Delegated`), and every call is checked against its
    method's scope by `guard::Guarded`, generated from the protocol's one
    method table (`vessel_proto::method_table!`, which also generates the
    envelope's `run_here` and the client's unary marker, Quality B);
    `NodeService`'s calls are inherent methods, so only `Guarded` can be
    served. `device.invite` (single-use, in memory by hash, at most a day,
    at most 64 waiting), `device.list`, `device.revoke` (closes the
    device's connections, each of which has its own RPC server).
    `SecureGateway::accept` answers the Noise handshake with the node's key
    pair through `vessel_relay::respond` (closing with `4401` when it
    fails), then the device preamble; at most 64 direct channels (7 per
    address, counted by `vessel_relay::address` as a relay counts, by a
    trusted reverse proxy's header with `--trusted-proxy`) and,
    apart, 40 through each relay link (`vessel_relay::RELAYED_UNADMITTED`,
    a budget made with the link) may be in their handshake or preamble at
    once, a relayed one with 5 s for both (`RELAYED_ADMISSION`), with
    frames limited to 64 KiB until admitted. Carriers come from `/secure` (pinged every 10 s, dropped
    after 30 s of silence) and from one `NodeLink` per relay (`relays`):
    each `--relay`, with the relays' token, and on a fleet member with no
    route of its own its current home's relays, added while the node runs
    with a ticket for its own key that it renews (`fleet.callback`), named
    only in its grant's link and unlinked when it follows another home or
    none; `--public` adds direct routes. Device labels lose
    control and invisible formatting characters.
  - Profiles (H1, H2): `profiles` reads `profiles.toml` at start (a broken
    file stops the node) and on `profile.reload` (a broken one leaves the
    set in effect); `NodeProfiles` owns the `HarnessEnv` and is the one
    place node code resolves a home; `profiles::launch` turns a spec and
    the caller's principal into `LaunchOptions` (missing secrets become
    `vessel:` lines in the session's log). `profiles::moving::plan`
    resolves the target, computes both homes, and runs the target's login
    check (30 s); terminal sessions are refused first.
  - Agents (H4, ADR 0013): the `agents` registry (credential hashes in
    memory, scopes, depth, the agent's connections; at most 8 live
    sessions an agent created and depth 3, checked under one lock), the
    agent preamble on the local socket, the ownership rule in the guard,
    and the `vessel` MCP server in every session from `Config::vessel_mcp`.
  - Fleet (F2, F3, ADR 0012): state under `<data_dir>/fleet/`, with
    `<data_dir>/fleet.toml` making a node the home. One loop looks every
    5 s: the home bundles when its sources changed (at least every 10
    minutes), publishes, and applies to its own profiles (its sources
    protected); a member pulls on (re)connecting, on a `fleet.watch`
    answer and every 10 minutes, fetches missing blobs in 1 MiB chunks,
    applies into every profile's home and reports. The home serves
    `fleet.manifest`, `fleet.blob`, `fleet.watch` (a 50 s long poll),
    `fleet.report` and `fleet.members`. `<data_dir>/secrets.toml`
    (owner-only, errors without values) and `FleetMcp`, which resolves
    the fleet's MCP servers with this node's secrets at each launch;
    `secret.set`/`list`/`delete`, and `secret.copy`, which asks the target
    `secret.list` first and sends the value as `secret.set` only if that
    passes.
  - Across nodes (Track X, protocol 6): `envelope` (`fleet.forward`,
    `relay`, `deliver`, with `Principal::Delegated`: the delivering
    device's scopes intersected with the origin's, ownership and the
    agent limits applied to the origin); `fleet/links` (the mutual grant:
    a member grants its home the scopes chosen at join, once per join,
    `fleet/granted.json`; the home keeps the credentials under
    `fleet/grants/`); `fleet.peers` and `fleet.nodes`; `session.read` with
    `wait` and its shares (`waits`: 8 per origin, 16 per other node, 16 per
    relaying member device, 32 per connection).
  - Fleet identity, after the security review: the home binds each
    member's node id, key and name for good (`fleet/bindings.json`); a
    name stays reserved while its node's device is revoked, and the home
    refuses a member named like itself or another member (the fix is the
    node's `--name`). Rebinding a reinstalled or renamed node takes the
    owner's `fleet.forget { node }` on the home (admin, not forwardable):
    it withdraws every outstanding invitation and records the devices that
    exist then, and the forgotten id or name goes only to a device paired
    afterwards, the first to claim it keeping it. Refusals about revoked
    or forgotten nodes carry no ids or times.
  - Transfers (F4, protocol 6): `transfers` keeps `session.export` and
    `memory.export` snapshots under `<data_dir>/transfers/<id>/` for the
    caller that opened them (another caller gets `not_found`), at most 16
    at once, for 10 idle minutes, and empties the directory at start; a
    live session mid-turn and terminal sessions are refused.
    `session.import` and `memory.import` pull through a `Source` (this
    node, or another through the envelope as the importing caller),
    fetch with hashes checked (`fleet::chunks`, shared with the member's
    pull), rename and place, and with `start` create the session with
    `origin: imported`.
  - Models (milestone 5, ADR 0015): `models` reads `models.toml`
    (providers with their API, base URL and key's secret name; routes
    naming a provider and its model and small model), refused if other
    users can read it, reloaded with `profile.reload` (a file that cannot
    be used leaves the routes in effect); `model.list` gives route names,
    models and APIs only. The proxy listens on `127.0.0.1` on a port the
    system picks and is stopped with the node. A request is admitted in
    order, on its headers before any body is read: no `Origin` (403), no
    compressed body (415), a live session's token (`vslm-`, 256 random
    bits, kept as SHA-256 and compared against every entry in constant
    time, refused once the session's end is visible, removed when it is
    retired; else 401 with nothing of the request logged), its own
    address as `Host` (`127.0.0.1:<port>` or `localhost:<port>`, else
    403), the route still defined, room in the limits (4 calls per
    session, 16 per node; else 429 with `Retry-After`); then the body is
    read under 32 MiB and 30 s. Its own accept loop (`models::listen`)
    bounds connections (256) and header reads (10 s, also the idle
    keep-alive timeout; a head cut off by it gets 408); a refusal closes
    its connection, and when every place is held the oldest connection
    that has not shown a token (or is past its session's 32) is closed
    to make room. A provider key under 20 bytes or of fewer than ten
    different ones refuses the call (403) before anything goes up; a
    local server is given none. The model must
    be the route's model or small model (else 400 naming both). Same API
    on both sides: the request's bytes go up (less what
    `vessel_llm::passthrough` must strip) with only `anthropic-version`
    and `anthropic-beta` of the harness's headers, and the answer's bytes
    come back event by event, decoded and written again, read alongside
    for usage; a stream that ends before its answer did gets an error
    event. Different APIs: translated through the neutral form, the
    upstream always streamed, keepalives every 15 s of silence, failures
    redacted before they are written. Either way an upstream event
    holding the call's key (in its name or data, the data decoded as JSON
    with member names) is not written: the call ends in a classified
    failure.
    Every answer is bounded (32 MiB, as read) and every write toward the
    harness has the client's idle timeout. The provider's key is read
    from `secrets.toml` for each call. Each admitted call ends in one `ModelCall` record in
    its session's log (`SessionHandle::model_call`) and one log line,
    metadata only. `session.create` under a routed profile mints the
    session's token by id before the launch (`Models::launch`), records
    the route in `SessionInfo.model`, and hands the harness a
    `vessel_harness::ModelProxy`; the token opens calls once the session
    has started. A move keeps the session's route (another is
    `unsupported`) and gives the restarted agent a second token, the old
    one kept until the move ends (`begin_move`/`end_move`, as agent
    credentials).
- `vessel-client`: the websocket connection with the bearer token (`wss://`
  refused for token connections: ADR 0005's amendment), one typed method
  per call, the gap-free `SessionStream`. `Connector` (`WsConnector`; the
  SSH one is in `vessel-remote`), `follow_resuming`/`ResumingStream`
  (redials with 0.5-30 s jittered backoff and follows again from its
  cursor; never repeats a call), `Pinned` (refuses a node whose id is not
  the pinned one). The node `book` (`<config_dir>/vessel/nodes.toml`, a
  project's `.vessel/config.toml` that may only pick one of the user's
  nodes, `resolve` for `--node`; edited with `toml_edit`, keeping
  comments). Milestone 3: paired nodes in the book (`key`, `relay`,
  `direct`, `credential = { file }`), `DeviceCredential` files (0600, in a
  0700 directory, bound to their node's id and key, so a book entry naming
  another key cannot draw the secret out), `pairing::pair` and the device
  preamble, and `secure::SecureConnector`: direct routes first (3 s), then
  relays (10 s), each followed by the handshake to the pinned key, the
  preamble and `hello`; a refusal after the handshake ends the search.
  Milestone 4: `NodeClient::connect_local` (the local socket, with the
  token or an agent's credential), `call_on` (`fleet.forward`), the
  fleet, secret and transfer calls, and `fetch_chunks`, the one per-file
  loop that streams a file into a caller's `ChunkSink` (the fleet's blob
  writer: hashed as written, kept only if right).
- `vessel-remote`: `ssh` (the system client with fixed options,
  `ControlMaster` sharing on Unix, commands from fixed words and
  single-quoted values), `platform` (`uname -sm`, four release targets),
  `install` (upload through stdin, SHA-256 check, run its `version`, then
  rename onto `vesseld-<version>` or `vesseld-<version>-dev-<hash>`;
  then `cleanup`), `release` (the archive and `SHA256SUMS` fetched with
  `gh release download` and checked), `provision` (`local`, `release` or
  `auto`), and `SshConnector` (`ssh … <data_dir>/bin/<binary> attach`,
  `ProxyExit` codes as typed errors, exit 127 installs and retries once).
  Tested against a temporary home through `ssh` and `gh` stand-ins, and
  with the real `vesseld` through `fake-ssh` (Unix only).
- `vessel-secure`: Noise `NK_25519_ChaChaPoly_SHA256` (prologue
  `vessel/secure/1`, one 10 s deadline) over any `Carrier`.
  `SecureChannel` is a `FrameTransport`: frames are chunked canonically
  into records of at most 65,535 bytes, sealed only when the carrier can
  take them, and every size and chunk rule is checked before anything is
  buffered; any tampering ends the channel. `NodeKeypair`, `routing_id`,
  `fingerprint`, the relay's possession proof, `set_max_frame` (the node
  admits under 64 KiB and raises it after the preamble). Tested with the
  published NK vectors, tamper cases, a seeded fuzz of the parser, and 16
  MiB frames.
- `vessel-relay`: `serve` on plain HTTP behind a reverse proxy
  (`/v1/node`, `/v1/accept`, `/v1/connect`, `/health`), state in memory
  only. A node registers with the relay token and the possession proof. A
  client is not read until the node's data connection arrives (no
  buffering), then both directions are forwarded a message at a time; a
  sender's close frame goes through unchanged. Limits: one record per
  message, 64 waiting clients per node, 10 s to accept, a per-address
  token bucket, and resource caps (ADR 0009's amendment): registered nodes
  (refused `full`), nodes still registering and relayed clients per node
  and in all (closed `4503`), and a byte rate per direction of a relayed
  connection (`server::throttle`); refusals logged at info, throttled,
  never with a token. Its own accept loop (`server::accept`, hyper's
  HTTP/1 with a 10 s header deadline) counts every socket from `accept`,
  in all and per address (`503` past either), and drops what is left 5 s
  into a stop; registering places and clients are also counted per
  address, a registering node and a spliced client have 2 s for their
  first message, and some registering places are kept for nodes that
  registered before (ADR 0009's amendments). Registration takes the token or a ticket for the
  registering key (`ticket`, ADR 0009's amendment); a ticket's
  registration counts against the node cap and ends when the ticket
  expires. `NodeLink` keeps a node registered (jittered backoff,
  `LinkState` on a watch), with the token or tickets it is handed, again
  at once with each newer one. One `Keepalive` (a ping every 10 s, a drop after
  30 s of silence) covers relayed and direct connections, and `Liveness`
  keeps it from inside a socket's adapter: the node's `/secure` carrier
  and, since 2026-11-14, its token `/rpc` transport (ADR 0038's
  amendment). `respond` runs
  the node's handshake on a websocket and closes with `4401` when it
  fails. `vesseld relay` runs it (README, "Running a relay").
- `vessel-fleet`: `bundle` turns the home's sources into a `Manifest` and
  a content-addressed `BlobStore` (owner-only, hash-checked writers; a
  damaged blob is removed so it is fetched again). Skills are named by
  their `SKILL.md`'s `name`; dot entries are never bundled; names that
  collide on macOS or Windows are reported. `plan` decides per place in
  each harness home (write, adopt identical content, keep a local edit
  with the incoming version beside it, leave a deleted file deleted until
  restored, remove what left the bundle if unchanged), skipping places in
  or linked to the sources; `apply` reaches every place through
  directories held open from the root (no symlink followed), decides
  again right before acting, writes owner-only and atomically, and
  records what it wrote in `managed.json`. `resolve` fills secret
  references and names every missing secret. Transfers as file sets:
  `snapshot` into a staging store (256 MiB per file, 2 GiB in all, checked
  as read) and `place` (never overwriting: a session whole or not at all,
  memory beside what differs).
- `vessel-mcp`: `vessel mcp`, the MCP server on `rmcp` (stdio) a node
  attaches to every session as `vessel`. Tools: `list_nodes`,
  `list_sessions`, `create_session`, `send_input`, `answer_permission`,
  `read_events`, `wait_turn` (a long poll of `session.read` with `wait`),
  `kill_session`; milestone 11 (protocol 10, `tools/conversation.rs`):
  `send_input`'s `delivery` (`session.prompt`), `read_conversation`,
  `change_queue`, `answer_question` (values read by the question's field
  kinds, `vessel_client::compose`), `resume_session`, `archive_session`;
  no read/unread marks, pins, deletes or attachments for agents; and
  (`tools/workspace.rs`) `list_projects`, `list_worktrees`,
  `create_worktree` (following setup to its end), `create_session`'s
  `project`, `dir` and `worktree`. Each but `list_nodes` takes `node` and
  goes through `fleet.forward` for another node. One module per area:
  `sessions` (the session tools and their schemas), `waiting` (the long
  polls), `args` (reading a tool's arguments), `conversation`,
  `workspace`, `lsp`.
- `vessel-cli` (`vessel`): `node start`, `status`, `harnesses`,
  `sessions`, `run`, `attach`, `send`, `log`, `kill`; terminal attach in
  raw mode with `Ctrl-]` to detach (also under the kitty keyboard
  protocol and `modifyOtherKeys`; the terminal is reset on every way out),
  structured attach as a conversation with numbered permission answers.
  The local token file is sent only to a loopback `--node`, and a relative
  `--cwd` is resolved only for one. Milestone 2: `--node` through the book, `vessel nodes`, ids pinned on
  first contact, attach through a `ResumingStream` (`[vessel] connection
  lost; reconnecting…`), `vessel node install|status|update`. Milestone 3:
  `vessel node pair` (the link on stdout, a QR code on a terminal),
  `vessel node devices [revoke]`, `vessel nodes add NAME --pair` (the link
  from standard input only), every command to a paired node through
  `Pinned<SecureConnector<WsDialer>>`. Milestone 4: `profiles`, `run
  --profile`, `session move-profile|copy|move`, `fleet
  init|invite|join|leave|status|members|forget|restore` (`forget` against
  the home over `--node`), `fleet secret
  set|list|delete|copy` (values from a prompt without echo or stdin,
  never argv), `fleet memory copy`, `init` (idempotent steps: node
  identity, harnesses, install offers, profiles for other accounts' homes,
  login checks, `--service` as a systemd user unit or launchd agent,
  `--join`), and `mcp`. Milestone 11 (protocol 10, `conversation/`):
  `send --queue|--steer|--interrupt|--attach FILE` (without them the old
  `session.send`; files uploaded a piece at a time, typed by extension),
  `session queue [edit|move|remove|send-now|hold|resume|clear]`, `session
  answer` (a question's `field=value`s or a permission's option),
  `session resume`, `session pin|unpin|archive|unarchive|read|unread`,
  `session delete` (asks unless `--yes`; says whether the session's
  worktree went or why it stayed), and each session's marks in
  `sessions`; entries and requests named by an id prefix
  (`selector::IdPrefix`); and (`workspace.rs`) `project
  list|add|clone|init|remove|commands|approve`, `worktree
  list|new|output|setup|remove|snapshots` (a kept worktree's reasons in
  words, `render::worktree`), `run --project [--dir] | --project
  --new-worktree | --worktree`. Arguments in `args/`, one module per
  command (`conversation`, `workspace`, `node`, `fleet`, `nodes`). The CLI reads `VESSEL_NODE`, `VESSEL_TOKEN`,
  `VESSEL_DATA_DIR`, `VESSEL_CONFIG_DIR`, `VESSEL_SSH` and `VESSEL_GH`;
  tests set them so they never touch the user's book, `ssh` or GitHub.
  Tested through the binary (`tests/cli/`), including a test that plants
  secret values and searches every log, file and command line a fleet run
  produced (`canary.rs`).
- `vessel-testkit`: `fake-tui`, `fake-acp`, `fake-ssh` (the OpenSSH
  command line, running the "remote" command locally under a fake home,
  with an injectable connection drop), the `Fake::path` locator (a nested
  `cargo build` once per test process; the first one of a run recompiles
  about 30 crates), and `within`, the test safety net whose clock starts
  after that build. The fakes report the environment, arguments, MCP
  servers, loaded session id and working directory they got (`report
  <path>` writes them to a file). With the `relay` feature, a relay that
  drops, replays, reorders, corrupts or delays records on command.
- Release workflow (`.github/workflows/release.yml`, milestone 2): on a
  `v<version>` tag matching the workspace version, it builds `vesseld` for
  four targets (musl through `cargo-zigbuild`), packaged with
  `SHA256SUMS`, and attaches them to the tag's release. Not run yet: no
  tag has been pushed. Signing waits for the owner's decision on public
  releases.

## Later

Model routing (Phase 3), the LSP bridge (Phase 4) and the clients (Phase
5) each get a plan and ADRs when they start. See `ROADMAP.md`.

## Saved-file IDE context (milestone 6)

`vessel-node::lsp` first checks Read scope through the common guard and then
resolves the caller's roots: a local session's cwd or operator-configured
roots; a delegated call has only destination-configured roots. The node
canonicalizes roots, and `vessel-lsp` checks files before starting a server.
`lsp_config` reads only an explicitly named operator JSON file. Server results
are confined again before any result-selected source is opened.

`vessel-lsp::manager` owns a bounded map of server actors keyed by the
workspace they analyse and accounted to the caller's allowed root, so slots
are shared fairly between roots (a busier root yields its least recently
used server). `process` owns the
child group and two pipe tasks; `protocol` contains the typed LSP subset;
`sync` owns bounded saved snapshots and coalesced native watches; `query`
converts into protocol types. A server handles one query at a time (eight
queued), while notifications and server requests are serviced during waits.
Different servers run independently. Idle/crash/shutdown drop or reap the
whole process group. The node's task tracker waits for LSP shutdown.

`registry` resolves trusted executables from an explicit PATH and creates a
minimal environment with a node-owned working and temporary directory.
`toolchain` resolves the operator's Rust toolchain once to absolute
binaries, so no rustup proxy chooses a compiler from a project directory.
`policy` refuses executable or unsupported repository configuration in
conservative mode and returns the Cargo projects rust-analyzer may link. This is not an OS sandbox; ADR 0016
records each server's execution risks and the full-analysis opt-in.

## Reported usage (milestone 12, ADR 0042)

`vessel-store::usage` projects ACP turn counts and positive running cost
increments into `usage_samples` in the event append's transaction (schema
15, migration named `USAGE_LEDGER`, after side context's schema 14). The existing writer queue owns
atomic writes. Readers reuse an idle connection cache; it is not a
concurrency bound. Usage scans have two shared try-acquired slots whose
permits stay inside queued/running blocking closures, including after
caller cancellation. Excess scans receive a typed busy refusal with retry
advice. A same-native-session ready record preserves the
cost baseline; repeated cumulative reports add nothing. The session runtime
marks context observations during replay, even when its Ready event is
consumed during a move, wake or rewind. The store treats these reports as
baselines, so an increase observed before the new native/profile marker
cannot spend old account history. Loaded or reset
counters and increments spanning the queried period carry explicit gaps. No context size,
proxy report or transcript is added to ACP tokens. Profile assignment is
captured at each sample, including a move in the same batch.

The ledger is capped at 90 days and 100,000 samples, independent of the
event byte cap; session deletion removes its samples. The metadata records
accounting start and the newest report lost to the row cap. A bounded
indexed read returns the totals, per-agent/profile groups and one session
page. `service::usage` applies principal restrictions and strips every
cost projection without operate scope. `vessel-client::usage` and
`vessel usage machine|agent|session` share that destination-side contract.
`usage::money` validates exact Decimal inputs (`rust_decimal`, MIT),
accumulates bounded integer multiples of 1e-28 (`num-bigint`, MIT OR
Apache-2.0), and checks the final finite-number decimal spelling. A unit
whose sum cannot be represented is omitted with all its contributing
reports counted as unaccounted. Intermediate sums never return to binary
floats. No provider access is involved in reported accounting.


## Inbox and disconnected attention (protocol 22, ADRs 0040–0041)

`vessel-proto::{inbox,push}` owns typed pages, source keys, shared read state,
validated subscription capabilities and connection presence. `vessel-store`
projects attention in the log append transaction (`inbox`, bounded `tools`),
keeps paired-device leases (`push`) and the independent owner-only VAPID
identity (`push_key`, `push-vapid.key`). The migration backfills retained
ended-session outcomes once; startup recovery withdraws interrupted requests.
Each attention insert reads the stored session marks in that same transaction:
archive blocks delayed attention, and delayed records inherit the kept read
watermark and explicit unread mark. Backfill uses the identical rule. Neither
projection nor read marking creates another answer validator.

`vessel-node::service::{inbox,push}` checks principals; existing session calls
still enforce Operate and first answer wins. The node enriches a page with
current project/session labels. Its `push` worker wakes on committed attention,
coalesces a burst, rechecks current authority/read state/presence and encrypts
only a fixed generic payload. `crypto`/`vapid` use web-push-native and RustCrypto
p256; `delivery` pins public DNS addresses and sends over existing reqwest /
rustls, with redirects/proxies off. The relay sees only the existing encrypted
transport. A late HTTP 404/410 removes only the attempted device, capability
and exact lease expiry; renewing the same capability survives that response.
Explicit unsubscribe remains unconditional. No phone UI or hosted push account
is introduced.

`vessel-client::{inbox,push}` exposes those calls, and `vessel inbox` lists,
shows one selected live form, answers and marks selected-node entries. CLI answers reuse
`conversation::answer`, including the form's own validation. The next native slice uses typed bounded `link` calls and a UI-independent
`view::inbox` fold, with partial availability and ACK distinct from projection
confirmation. Its inbox area draws rows; shell navigation/counts, board lane
and OS delivery consume the same model. ADRs record lifecycle, exact bounds and notification
privacy. Dependency direction is unchanged; the node adds the account-free
push builder, while the protocol uses the workspace's existing URL parser.

Inbox live-request reads recover one exact pending form from the session
actor independently of event retention, capped at 15 MiB per reply. Default
summary reads retain the bounded durable projection for background refresh.

The web client's `data/usage` owns the typed accounting call, a single
in-flight/latest-selection reader, and pure locale-aware facts. A single
names-only catalogue comes from the last all-agent report on the selected
machine; it replaces rather than accumulates groups and holds no token or
cost values. Agent/profile scopes read node totals directly. Historical
unnamed reports stay in All profiles; they are not a current-default account.
The Settings Usage feature uses existing row/select controls. Changing
machine/period/agent/profile hides the previous identity before the effect
starts, and unmounting clears names and discards queued or late observations. No browser
provider access or price arithmetic is introduced. Missing optional
counts retain field coverage; unsafe browser integers are labelled rather
than presented as exact. Account limits remain a separate merged-contract
dependency (ADR 0075).


## Native app localization (milestone 12, staged)

### Plugin value boundary (ADR 0079, staged)

`vessel-proto::plugins` owns validated plugin/contribution/method identities,
captured node/runtime-generation addresses and the small semantic theme
palette. Its constructors and serde boundary refuse malformed identifiers,
resource paths masquerading as ids and executable paint content. These
values do no I/O and add no dependency edge. `PLUGINS` remains unassigned;
no plugin method is registered and no helper is started by these types.
Later native registry and supervised node-runtime layers consume this one
contract rather than defining their own plugin identities.

The separate `spikes/native` workspace keeps UI and I/O out of
`vessel-app-view`. Its localization build script reads the Fluent source
catalog, validates translations and generates typed accessors; only embedded
resources reach the target library. `localization` prepares language bundles
and retains plain messages before rendering. NUMBER display rules and DATETIME whole patterns come from each language
folder; typed Gregorian/local-clock parts cross that boundary. ICU4X was
rejected after the release-size experiment, recorded in ADR 0039.
Pairing is the first whole screen on that API.
About's licence and bundled-software vocabulary also uses generated plain
getters; its closed inventory roles map to them in `view::about`, without
formatting a message while drawing each row. The rest of About's shared
sections retain their owners' tables during the staged migration.
Updater labels and whole value messages use the same catalogs. The pure
`view::updates` projection prepares shared text on state/language changes;
the host groups its updater runtime, authoritative state and prepared words
in `UpdateUi`. Both desktop and browser shells use it without introducing
the native updater's I/O dependencies into view.
Rewind and fork vocabulary uses the generated catalogs beside the remaining
conversation tables. Rewind recoveries project typed node codes; fork source
captions are retained in `view::fork::Headers` across unchanged fleet events.
The host refreshes captions at fleet/act/language events and invalidates rows
only when captions change. Origin and frame notices retain shared text for
language refresh without another request. Typed fork/origin-read recoveries keep
diagnostics in host logs/session state; unknown empty-fork reasons borrow a
complete recovery message rather than copying a diagnostic while drawing.
The host reads the typed
language preference before creating a shell; System resolves against the
host tag, while VESSEL_APP_LANG is the checking override. General's language
row uses the generated registry and native names, never the pseudo locale.
A preference change prepares the catalog outside rendering, then refreshes
both Fluent and legacy labels and retained field placeholders without
clearing input or restarting. Typed pairing errors and success notices keep
the data needed to reword an existing outcome. Legacy area tables stay until
their owners migrate them. The crate map,
execution instructions and gate are in `spikes/native/README.md`; ADR 0039
records the decision and `docs/plans/backend-m12-i18n.md` its measurements.

## Native settings' machine book (milestone 12)

The independent workspace under `spikes/native` keeps screen rules in `view`,
node/file operations in `link`, shared controls in `kit`, and drawing in `app`
(its README owns the full crate map). `view::machine_book` validates names and
models edits and safe failure reasons. `link::machines` applies those edits
through the existing CLI `BookFile` on the blocking pool, then publishes a fresh
snapshot. Selection follows a successful write only; raw book errors stay at
the file boundary because malformed content may contain credentials. No new
wire method or store is introduced. Startup, tick, reload, retry and edit reads
share a fixed book path and the same typed boundary. Syntax failures reuse the
shared safe `FileFault` projection; other file failures use closed reasons.
A failed edit does not refresh or mutate the existing clients or session follow.
## Side questions (milestone 12, protocol 19)

ADR 0045 keeps side questions in a separate native session. The optional
parent id lives in the stored SessionSpec. `vessel-node::service::side_questions`
serializes discovery, creation and expected-id reset; a creation lease
moves into the session creation task so request cancellation cannot start
an unseen duplicate. Ordinary creation naming a side link follows that
same path. `side_questions` owns the person/read policy and bounded
snapshot formatting; `sessions::side_questions` waits for persistence and
provides the dedicated Start-only ingress. Generic prompt/tracked/queue
paths refuse sides centrally, including automation.

`vessel-store::side_context` projects bounded latest plans with their log
commit (schema 14), and reads a bounded newest-first transcript tail on
the existing blocking reader pool. Retention can prune the plan's event
without erasing its projection. Current status/attention is read separately;
no goal is fabricated. The snapshot and complete prompt have UTF-8 byte
limits, including metadata and truncation markers. Nothing is sent back
into the parent session. This isolates conversation context, not project
tools or filesystem rights. A parent rewind clears the plan projection
in that same log append; side-tail reads stop at the latest rewind rather
than presenting discarded future messages. Earlier valid turns are also
omitted by this bounded partial-context policy. Generic forks refuse a side
source so that only serialized side creation can set its parent link.

`vessel-client::side_questions` holds typed calls; the CLI's `session side`
opens/reuses or asks, printing the side id. Existing read, subscription,
approval, cancellation and explicit-resume paths serve that id. An older
client may read a side, but ordinary prompts are refused; an older node
refuses the new calls. Side logs survive a missing parent, while questions
and resume require it. Imports clear the local parent link. The track plan
records the reference acceptance list and remaining verification.


Provider context is one typed actor control with its bounded question.
`vessel-agent::PromptContext` bounds UTF-8 bytes and redacts Debug. Only
ACP receives the prefix; the driver emits TurnStarted with the question
alone and checks the actual serialized ACP request before sending. The
ordinary prompt/attachment path is unchanged. No sticky context survives
failure, cancellation or a busy refusal. Existing restart/profile-move replay suppression continues to cover
UserMessageChunk until Ready. A new native import transfers and may replay
provider history, including generated context; it has no local side link.
## Protocol 16: what a machine shares, for a person (ADR 0056)

`fleet/published` reads `fleet.published` from the machine's own files on a
blocking task under the fleet read permits: the role (`projection::role`),
`manifest.json` (what a home publishes or a member last pulled) and, for a
member, `state.json`'s applied version. It keeps paths, kinds (from the
bundle layout: `instructions.md`, `skills/…`), sizes, executable flags,
tool server names and the secret names they use, and drops everything
else of the manifest (blob hashes, commands, arguments, addresses,
headers, literal values). It shares `projection`'s byte-bounded file read
and reply budget, and refuses the whole answer past a bound. `admin`, not
forwardable, refused to agents; `vessel fleet published` prints it.

## Native agents' option lists (milestone 12, track P)

What each agent on each machine offers (`harness.options`: models, efforts,
permissions modes, on/off options; and the `/` commands a session of it last
reported) is one store, `view::offers`, held by the
app's composer area and read by every control that draws those lists. It
decides what is shown (the list there is, however old; loading or the error
only when there is none), when to ask again (on open, at connect, when a
machine's agents change or a person retries; one request per agent in
flight, none within 30 s of a good answer on open) and how an answer lands in
an open list (new rows at the edge away from its anchor, the shown rows in
place). `link::offers` keeps the last answers on the
device between runs in one owner-only, versioned, bounded file in the app's
state folder, keyed by the node's id (not a book's name), written only when an
answer differs; a machine that connects gets its kept lists
(`Event::OptionsKept`) before the fleet shows it online. No new wire method.
The rules, bounds and what was followed from Paseo's provider snapshot are in
`docs/plans/backend-m12-composer.md`.

What was typed in a composer and not sent is a draft per session and one
for the new chat (`view::composer::drafts`, held by the composer area): put
back when its composer comes back, written a second after the last edit by
`link::drafts` to `drafts.json` beside the agents' lists (owner-only,
versioned, bounded as the web bridge bounds the page's; a file that does
not read is set aside, never overwritten). Up and Down in an empty session
composer walk the messages sent in it (`view::composer::recall`, Paseo's
`ComposerRecall`); a recalled message is not a draft until it is edited.

## Protocol 13: schedule preview (N3)

`NodeInfo.timezone` carries the startup-detected system IANA name; unknown
stays absent. The RPC handshake and `node.info` omit it below protocol 13.
`service/schedule_preview` computes `schedule.preview` through the same
`automation::time::Due` as the scheduler. Its four-slot semaphore is acquired
before `spawn_blocking`; the worker owns the permit through cancellation.
The result includes the effective interval anchor for create/update reuse.
The request count is parsed as 1–10; every search has the existing eight-year
bound, and failure returns no partial list. Typed reasons use optional
`WireError.detail`, leaving existing error encodings unchanged when absent.
The protocol crate uses the workspace's `thiserror` for this typed error;
no runtime or I/O is added there. The CLI and client expose the same call. The web registry boxes the status
inside a queued connection change so this larger metadata does not inflate
its small removal notification; the serialized page frame is unchanged.
Testkit carries Paseo's cron cases; the fake fleet checks machine timezone,
ten runs, target-zone defaults and impossible-expression refusal.

The member loop treats cancellation after a grant or state write as shutdown,
clears its cached home connection, and checks again before each new pull.
This prevents timer/watch readiness from admitting fresh disk work after
shutdown; an apply already started retains its normal completion semantics.

N3's fleet controls reuse the offline join/leave implementation in
`fleet/joining`; `fleet/mutation` takes a separate non-waiting OS membership
lock shared with init. Only blocking preflight and commit hold that OS lock;
network pairing holds the node-wide Tokio mutation permit. Commit rechecks
the role and never replaces a racing CLI's committed membership; failures
after redemption are explicitly uncertain, without automatic retry. The
permit remains owned through blocking disk work after RPC cancellation. Pairing never holds the apply
lock and no single-use invitation is automatically retried. The authenticated
service refuses agents and forwarding for Admin join/leave/sync. Accepted
sync requests coalesce in `Home`'s notifications; supervisor and member keep
their serial apply engines. Leaving wakes the supervisor and reconciles the
previous home's device even after a partial leave before returning its error.
The supervisor cancels and joins a removed/replaced member loop before
reconciliation waits for its grant lock, and before a successor starts. This
prevents leave waiting for the grant it has not cancelled, or an old loop's
apply/cache cleanup finishing after the successor. The silent-route leave
regression retains the existing five-second cancellation budget.
Membership writes rotate a cancellation token held by `Home`: a grant is
cancelled even if the supervisor is already waiting on its lock. A tick
snapshots that token before reading disk, so a loop created from an old
snapshot is cancelled too. All such tokens remain children of node shutdown.
Restart can retain an unused owner-only credential, removed by a later leave.
Existing admitted apply can
finish; leave does not roll it back or remove materialized files.

`fleet/projection` reads role and local apply history without resolving
secrets, reading source text or sending arbitrary filesystem error messages.
Four read permits bound concurrent disk work. File-byte and item bounds
precede decoding/cloning: state 1 MiB, membership 64 KiB, local conflicts 256,
total conflicts 512, members from an indexed 128-device window. Its `more`
flag causes a typed whole-result refusal, never a truncated complete list.
The status path inspects only those reports under the same lock before
copying. A nonallocating serialization sink caps payload below 1 MiB with
space for the RPC envelope; the permit survives that blocking work too.
The bounded store read retains its caller's generic admission guard inside
the blocking closure, returning it with the window; cancellation cannot free
the node's permit while database work is still queued or running. Storage
does not depend on node types. A one-thread blocking-pool regression checks
that lifetime without sleeps.
Read-only and delegated clients receive
relative conflict paths and typed issue classes. Only the home's own owner
or paired Admin also receives existing member-device reports. Status is last
apply history with no current-home association: bare generations must not be
compared with the current home to claim up to date. CLI `--node fleet
join/leave/status` uses those same calls; the unselected offline form stays
available, and `fleet sync` always addresses the selected node (ADR 0044).
The CLI's existing test-only `serde_json` dependency is now a normal one:
encoding the already-confirmed invitation uses the protocol's formatter and
retains its concrete error source rather than inventing a string error.

## Caller authority query (N3 row 3, protocol 20)

`device.self` projects the guard's admitted principal: a paired caller's own
id and canonical effective scopes, or no device id for owner, agent and
delegated calls. It performs no I/O and sends at most five scope values.
`Method::scope` explicitly returns None for authentication without an extra
scope; the generated guard remains the single admission check, with every
legacy method retaining its scope. This is not anonymous access. Ownership,
agent limits and profile policy remain separate checks (ADR 0044).

Typed envelopes require their inner call's wire version as every hop decodes
and re-encodes it. Clients and servers refuse old direct or routed hops
before execution. No scope fallback is inferred for an unavailable query.
`NodeClient::device_self` and `vessel node devices self` expose the same
projection, with the CLI branching before Admin-only listing.

`vessel-testkit` optionally depends on proto for `authority-fixtures`, enabled
only by node tests, to share concrete `DeviceSelf` fixtures. Ordinary fake
binaries keep that feature off. No new external crate or schema is needed.

### Invitations require advertised routes (row 4, ADR 0044)

The existing device authority validates an invitation's effective Routes
before creating randomness, encoding a link or adding it to the pending map.
Public invitations use startup --public/--relay configuration; a tracked
fleet callback uses its explicit override. An explicit empty override refuses
rather than falling back to defaults. Bind addresses are never inferred.
Missing routes map once to Conflict plus DeviceInviteError::NoRoutes, with
setup guidance. Configured routes are not a reachability guarantee and no
probe or automatic relay activation is performed. Existing Admin guards run
first. The CLI prints no link or QR on refusal; older clients keep the human
message while typed clients can localize the stable error reason. No new
RPC, external dependency, persistent state or ownership rule is introduced.

## Opt-in saved-sign-in limits (milestone 12, protocol 23, ADR 0042)

`vessel-harness::limits` owns the supported agents' saved-login formats,
fixed vendor origins, bounded response decoding and per-node request context.
`LimitBinding` captures only launch configuration and a canonical profile
label. Session actors retain that binding through park/wake and rewind;
successful profile moves replace it with their actual target's binding, and
failed moves preserve it. A profile reload cannot silently retarget a running
session's quota lookup. After a node restart a stored session has no live
binding: it reports unassociated rather than choosing today's profile.
Bindings do not prove which sign-in a running agent has kept in memory.

`vessel-node::limits` owns a 32-command mailbox, at most two concurrent probes,
and at most 256 RAM-only account/binding observations. Its independent owner
saves and applies policy even after an RPC is cancelled. Turning lookup off
cancels the generation and clears observations; old jobs cannot publish into
a later opt-in generation. Blocking credential/response/certificate work
retains a shared two-slot admission permit until the closure actually exits,
including when its awaiting task was cancelled. Shutdown drains this work.
A bounded vendor/token cooldown is shared across aliases and Codex/OpenCode;
policy toggles do not bypass it. There is no background polling.

`vessel-store::usage_lookup` persists only the off/on decision in an atomic,
owner-only file. It is separate from SQLite usage history and sync; malformed
policy fails off. Library configuration disables all local probes. Binaries
capture only nonsecret routing/path facts and select the local backend; the
administrator's persisted policy still defaults off. Fake-enabled builds
refuse that backend. Tests use only synthetic temporary sign-ins and recorded
vendor response shapes, never the operator's files or vendor network.

`service::limits` enforces destination read for cached facts, operate for an
explicit refresh, and admin for the machine policy. Agents may read only their
own local session. Read-only viewers cannot see credit balances. Remote
forwarding uses the existing origin/destination scope intersection; no login,
private path or endpoint is accepted from or returned to a client. It forwards
sanitized facts or the admin policy choice, never credentials.

`vessel-client::usage` and the CLI expose the same typed calls. A full-window
notice is a fact about the observed saved login, with independent window ids
and an optional latest known future reset. Paid-use permission suppresses that
notice. It neither diagnoses why a turn stopped nor implements automatic
continuation. Native screens and persistent wait/switch recovery remain outside
this node slice.

For a multi-provider agent, `LimitBinding::check_session_model` keeps native
provider-format knowledge in the harness. The node asks the existing cached
options channel before a session probe and rechecks after it. An unknown or
changed provider cannot attribute the saved OpenAI account to that session;
the explicitly selected account scope remains available independently.

## Dictation contract (ADR 0067, staged)

`vessel-proto::dictation` owns bounded audio values and their decoders, and
provider metadata (`settings_values`, `local_model`). It does not perform
I/O. The metadata retains a parsed endpoint URL, using the existing
workspace `url` dependency, and a typed catalogue with future IDs retained
on read. It loads or downloads no model. PCM and transcript diagnostics redact their contents.
The stream actor and endpoint are subsequent implementation slices; no
method, settings field or provider is available from these values alone.
No crate edge changes: the node will reuse its existing HTTP and secret
storage dependencies when the service lands.

The unregistered settings values (`settings`, `patch`, `secret_ref`) distinguish
keep, set and reset, validate provider options against the merged choice, and
redact reference diagnostics. Their read view reports only key presence. They
resolve no credential and provide no file codec or callable settings field.

The next staged node helpers own provider PCM conversion, bounded segments,
in-memory WAV encoding and opaque connection ownership. Their tests are compiled
under a test-only module until the RPC wiring removes staging; helper tests do
not establish a live service or the transport-close path.

The node's `http_tls` module builds the operating-system root, explicit
ring and ALPN configuration once per client constructor. `catalogue::fetch`
uses that shared configuration; it retains its own address validation,
HTTP policy and deadlines. This extraction introduces no destination,
credential, request or new dependency. Later asynchronous callers must run
the synchronous root loading on a blocking worker.

`FleetFiles::resolved_secret` reuses the node's secret parser and lookup,
returning the existing non-serializable `ResolvedValue` for internal header
consumers. The `secret_value` wire-copy wrapper delegates to it; no new RPC
can read a secret and storage/refusal behaviour is unchanged.

## Session creation labels (ADR 0063)

`vessel-proto::labels` validates the bounded label map before creation.
`SessionSpec.labels` retains what creation asked; `SessionInfo.labels`
starts with it, while the store keeps a distinct current-label column.
Import copies that current map. While the protocol is unassigned, every peer
receives infos without labels; after assignment, older peers do,
including nested transfer replies and saved/scheduled session specs. One
optional version constant is assigned at integration; no dependency edge changes.
Filtering and patch/event ingress follow in the next slices.

## Staged project-file terminal contract (ADR 0066)

`vessel-workspace::project_file::terminals` owns TOML terminal decoding and
ordered digest fields; it starts no process. `vessel-proto::workspace`
carries the validated commands into the existing approval snapshot.
The node describes that snapshot through the existing project-command path;
reply adaptation refuses it to peers without the assigned field version.
The CLI's `file_terminals` formatter makes those commands visible. Runtime
terminal creation, service-port assignment and health are later changes.
No dependency edge, external dependency or store schema changes.

## Project pull-request values (ADR 0061, transport pending)

`vessel-proto::forge` owns an opaque bounded provider cursor, a cursor's
source-repository association and a validated page of open PR summaries.
The page decoder caps retained entries before constructing domain data;
duplicate numbers, another repository, non-open entries and empty
continuations fail the whole page. Cursor diagnostics omit provider text.
These values register no call and add no fields to existing replies or
events. Their feature version remains unassigned; `pr.list` continues to
mean watched sessions. Project resolution, provider listing and client/CLI
transport are a following stage, not exposed by these value definitions.

### Startup recovery values (ADR 0059)

`vessel-proto::restart` defines the disabled/reference-prompt policy and partial
patch values using the existing bounded `PromptText`. The feature protocol is
unassigned. Both settings replies pass through peer projection, including the
existing recursive envelope walker; an unassigned field stays hidden from every
peer. The node's existing response builder supplies only the disabled default.
There is no recovery settings engine, request registration or startup service
in this value-only slice; those remain for the staged and wiring slices.

## Ended conversations in the page (ADR 0036 amendment)

The bridge's `wire::SidebarView::default` and the page's `prefs-change`
start with ended sessions visible. `data/sidebar` reuses that page default
and still applies archive exclusion separately. The existing `PrefsDoc`
field reader fills an absent choice from the wire default, preserving an
explicit false and unknown file fields without a migration or rewrite.

## Timeline following during layout

The page's `data/follow-end` decides end intent from consecutive scroll
and layout geometry. `features/session/window-follow` owns the DOM snapshot;
`window-watch` applies it to scroll, container resize and row measurement,
while `window-pin` still owns a requested row. A layout-queued scroll must
not turn an end reader into an upward reader before resize delivery.
An actual upward move stays authoritative, so resize does not pull that
reader down. Listener/observer cleanup remains local to the timeline.

## Node-control process boundary

The approved amendment in ADR 0001 moves the generic selected-file Linux
lock observation from store into proc. Store retains `NodeLock`, its data
layout, record handling and caller-facing API; proc owns only OS mechanisms.
The move adds `vessel-store -> vessel-proc` in its implementation commit.
The later real force/restart caller adds `vessel-cli -> vessel-proc` in its
own commit. The documents-only contract left the map unchanged. The first
edge is now implemented; the later CLI edge remains proposed. The boundary
check still compares the active table and current implementation exactly.

A kernel PID is never signal authority on its own. Descriptor possession,
process generation and namespace verification precede the separate CLI
caller; authenticated directory binding and session consent remain in the
CLI. ADR 0001 records rejected alternatives and the tests owed by each step;
ADR 0064 retains the product contract and platform refusals.

The reviewed contract also requires name-level restrictions in the same
implementation commits as the edges. The store-to-proc move restricts the
name `vessel_proc` in store to `src/lock/`, which delegates the lock query
only. The real CLI caller restricts proc's signal entry point to the force
module. These rows belong in `Names only some files may use`; a graph edge
alone would otherwise expose the owner's unrelated process operations.


The first process-mechanism edge of ADR 0001 is active: store delegates its
selected-file kernel query to proc through `lock/query`. Its public
`NodeLock` API and snapshot re-export remain unchanged. The name rule keeps
this dependency inside that lock boundary. Proc still has no internal
normal dependency; its snapshot alone grants no signal authority. The later
CLI signal edge remains proposed and is not added by this move.

The staged node `speech_audio` module is shared by dictation and spoken
conversation (ADR 0067 / ADR 0080). It owns a borrowed aligned PCM16 view,
fixed-provider-rate conversion and checked in-memory WAV framing. Recording
budgets, silence, segmentation, prompts and multipart fields stay with callers.
It adds no crate edge or dependency; consumers remove its test gate when wiring
real product callers. Payload-bearing Debug is redacted and no audio file is
created by the codec.

The node `connection_life` primitive owns an opaque Arc identity and a close
observer. It contains no quota, presence id, transport or worker. Every
connection's context (`service::Connection`) makes one when the connection
opens, over the transport's own closing token (`vessel_rpc::Outbound::closing_token`),
and acting-as contexts clone it. The observer fires when the transport is closing
or when the last context goes away, whichever comes first, so a call still
running in the close grace does not keep it open. Resources reach it through
`Connection::life()` and retain only `owner()` and `closed()`. Workspace preview,
dictation, spoken conversation and later resources use this accessor rather than
creating another ownership key. Until the first of them reads it, the non-test
build expects the accessor to be unused; that consumer removes the expectation.

### Process observation boundary

ADR 0066 defines a runner-issued `ProcessLifetime`: Proc and PTY own its
revocation; AgentProcess and session/terminal handles pass read-only
observations to node consumers. A checked `ProcessId` is only an OS value.
Neither PID equality nor the observation alone proves socket ownership or
authorizes Preview. Fresh complete inspection and workspace/session policy
remain at the node boundary. This carrier introduces no dependency
edge or wire type.

## Transient system-context launch boundary

`vessel-agent::setup` owns AgentSetup separately from the ACP handshake state
machine and re-exports preserve the existing public path. Its optional
SystemPromptOverride is a bounded non-serializable instruction value.
`vessel-harness::system_prompt` owns the transport capability beside launch
policy; no transport currently advertises support. AgentProcess refuses an
unsupported override before process spawn, with a typed Unsupported mapping.
The field does not enter SessionSpec, user messages, argv or environment.
Reload orchestration and actual native override/restore transports are separate
work; see docs/plans/provider-transient-context.md. No dependency edge changes.


## Relay registration child lifetime (ADR0096)

One `vessel-relay::NodeLink::run` owns a bounded `link/pending::DataOpens`
JoinSet until every pending dial/handshake reaches accept or discard. Normal
stop cancels/joins these children before Stopped/run completion; an already
handed-off carrier belongs independently to the existing gateway. The node's
Relays accept closure also checks its registration cancellation token. No
second runner, transport, dependency or wire family is introduced. Dropping
a run aborts its pending children and is not a normal stop acknowledgement.

`vessel-proto::BookRouteId` is a client-minted, entry-scoped nonzero u32
for the shared client-book model (ADR 0097). It is local data, not an RPC
connection id or wire field; its lower-crate home lets the client and
native view share it without a dependency between those layers. Checked
allocation exposes exhaustion. This additive value changes no method,
protocol version, book format or current consumer.

The coordinator-approved local client-book appearance values live in
vessel-proto::book_values (Label/EmptyLabel, Color, BadgePreference). They
are copied from the exact source-reviewed pure move ccedd4860, without
algorithm changes, tokens or locality inference. This lower boundary
avoids a client/view dependency edge. It adds no RPC/method/version/TS,
file format or storage limit behavior. E1 retains the native compatibility
re-exports and consumer tests on its consuming branch; base6 itself never
contained that unreleased hosts module.

ManagedLocalAlias is the one pure reserved-name syntax in book_values.
Client book destinations and native local-name wrappers use it without
depending on each other. Only new mutation destinations are reserved;
parsing this syntax grants no provenance or effect authority and changes
no existing legacy book behavior. No writer/admission is added here.

### Checked owned group cleanup

Proc's additive Child::wait_checked distinguishes leader exit from the first
owned group/job cleanup result. The immutable receipt or typed retained cause
never retries a delayed numeric group id. Legacy wait and ProcessLifetime revocation remain compatible; neither an observer nor a stored
PID supplies this receipt. Its accepted cleanup request does not assert every
descendant's observed OS death (docs/plans/checked-process-cleanup.md).

### Saved-settings diagnostic boundary

The store's settings-file decoder owns safe parse diagnostics: only the file
path, category and bounded path shape leave it; the raw serde cause is private.
The node's startup recovery logger uses that diagnostic while retaining the
existing set-aside behavior. It reuses serde_path_to_error already in the
workspace; no workspace-crate edge changes (ADR 0038).

### Independent workspace registry (ADR 0082, metadata at protocol 25)

`vessel-proto::registry` carries independent workspace identity, exact backing,
lifecycle and atomic node/workspace/resource snapshots. Workspace and create
operation ids are distinct UUID newtypes. No identity is inferred from cwd or a
node-book alias. Existing project/worktree IO remains `workspace::WorkspaceRoot`.

`vessel-store::registry` uses the existing writer savepoints and read pool. A
single append migration retains descriptors, backing reservations, resource
ownership/start leases and the global snapshot revision. Capacity reserves
worst-case retained bytes before effects and never evicts ownership. Writer
commit publishes a coalesced watch; readers hydrate descriptors and memberships
in one transaction. `registry::write::{require_active,bump_revision}` is the
in-transaction seam for catalog assignment extensions, not a second registry.

`vessel-node::service::registry` handles registry reads, bounded long-poll,
rename, pin and the blocked-source release through the single method table:
protocol 25's six metadata methods. Agent principals, direct or delegated, are
refused before lookup; a forwarded person-origin call holds the intersected
scopes and reads without the long poll. The separate
create and lifecycle packages own actor launch and archive/recovery orchestration.
Pre-spawn leases fence archive against late actor registration. Restoring is an
exclusive persisted claim before backing recovery. Filesystem provisioning remains on the separate continuation until a real
create driver wires it; no unwired node service ships in this dependency. Project config,
icons, editor integration and automatic archive remain separate incomplete work.
Those have no method row; protocol 25 covers the six metadata methods only.

### Settings writer lifetime

The node settings owner retains an owned serialization guard in a tracked task
through atomic file replacement and runtime application. RPC cancellation drops
only the waiter. After connections finish, teardown closes settings mutation
admission and joins admitted writers before the store closes (ADR 0064). This
adds no crate edge, setting, format or wire method.

## Bounded node diagnostics (ADR 0074)

`vessel-proto::diagnostics` owns the validated report, closed section states,
process values and named collection/output bounds. The canonical RPC table
registers `node.diagnostics` as a Read-only unary call of protocol 24.
A request to an older peer stops at the shared compatibility admission
boundary before client transmission or server collection. Fleet forwarding is refused; direct
authenticated agents retain their effective Read scope.

The node composes facts from their existing owners: Store aggregates, runtime
counts, immutable profile selection and cached readiness, relay watches, and
`vessel-proc::self_process` for fixed bounded Linux metadata. It starts no
agent, refreshes no provider and reads no arbitrary process, log or path.
The node captures its monotonic startup clock before startup I/O and publishes
listener state after successful binding. Sources have separate observation
times; this is not an atomic snapshot of the whole machine.

Collection capacity and shutdown tracking belong to retained leases. Each
queued/running blocking job retains its lease after the response deadline or
caller cancellation, so a timed-out request cannot bypass the resource bound
or make shutdown overlook remaining work. Missing and timed-out sections are
explicit. A bounded journal accepts only completed local call error codes and
timestamps; canonical method shape excludes envelope duplication.

`vessel-client` uses the existing connection, a named request deadline and the
admitted node identity. The CLI renders typed fields into bounded text or JSON
and prints safe error categories without private connection/source chains.
Native rendering remains a separate consumer. Provider diagnostics belongs to
its provider owner, while desktop logs/watchdogs belong to their lifecycle
owners. This report deliberately excludes free log text, environment, endpoint
addresses, keys, pairing links, private names/paths and version suffix text.

The source assembly and authority/lifetime rules have real node and synthetic
process regression fixtures. Package verification is recorded separately in
its plan; a successful assigned-protocol request is not claimed before
integration assigns the feature.

## Retained project metadata admission

The node's workspace projects own an opaque asynchronous admission hold shared
by registration/removal and backing creation. Its semaphore bounds holders and
waiters; the provisioner retains a clone through descriptor finalization.
The existing worktree creator accepts the caller's reserved identity only through
the admitted Skip path, with in-flight duplicate exclusion. This adds no crate
edge, wire call or second registry. It does not attest config approval, physical
root continuity or process lifetime; see `docs/plans/project-admission.md`.

Standalone tool body values live in vessel-proto::timeline: a closed eleven-kind
ToolCallDetail vocabulary, validated display identities and measurement units.
The portable body kernel has no event/RPC attachment or execution path; native
renderers borrow it without creating a competing provider model. No dependency
edge changes. See docs/plans/timeline-body-values.md for exact source provenance
and pending consumer-adoption checks.

The private finish-notice vocabulary is also a standalone proto value producer:
timeline/notice owns the closed NoticeKey/checkpoint, the existing UUID macro owns
FinishIntentId, and timeline/admission owns logical retained-state ceilings.
No public method accepts these keys. A decoded key does not supply execution,
parentage or grant authority. The current watcher/store/actor are unchanged;
their real atomic admission fence and persistence remain separate work.

### Connection-scoped workspace memberships

The registry's whole-snapshot hydration and workspace attention mutations share
`registry::workspace_memberships` in `vessel-store`. Passing a `WorkspaceId`
selects only that workspace on the caller's existing SQLite connection; the
complete snapshot passes `None` inside its held read transaction. One typed
decoder handles both. The helper starts no second transaction and never filters
pending or historical resources into a misleading live-member list. Mutation
callers retain `require_active`, current role/attention checks and the write in
one existing writer transaction; this reader supplies no authorization or exit
proof. No new protocol, schema, cache or dependency accompanies the seam.

Execution incarnation/invalidation has one owner in the existing Agents registry.
Each live entry owns a UUID and latest-change watch; natural retirement invalidates
then closes it. Transition owners use invalidate_execution before their effects.
The proto stamp/ceiling are correlation values only. This first producer implements
no runtime capture, parentage lease, actor queue policy or private notice admission;
see docs/plans/timeline-execution-invalidation.md. No dependency or wire edge changes.

## Page feedback and modal ownership

The existing Base UI dialog, command palette and sheet own their key and
focus behavior. `web/src/ui/page-focus` lets page-originated focus and
keyboard capture yield to a currently open kit dialog or menu, using its public
role/open attributes. This closes the gap before its initial focus frame:
a late-mounted draft cannot take a menu's focus, and a page shortcut recorder cannot
consume Escape before the dialog receives it. No second modal registry or
Escape listener competes with Base UI's nested controls and IME handling.
`compose/use-draft-presence` keeps the mounted draft's asynchronous completion
guard separate from the optional initial focus; closing a modal does not
re-run that mount effect. A paused shortcut recording remains pending.

Toasts retain the existing host and lifetime rules. The viewport sits below
the header and centres existing cards, with responsive width and a bounded
scrolling stack. It no longer occupies the lower composer/dock region.
The layer's tokens, status roles, colors and dismiss actions are unchanged.

The web socket's authentication close path distinguishes a missed hello
budget from invalid proof. Budget expiry remains a retryable transport close;
only a refusal discards the tab's credential. Admission, mutual proof and
single-use redemption remain owned by the same server and browser modules.
ADR 0017 records why an absent proof at a resource deadline is insufficient
to refuse identity; `docs/plans/backend-m12-web-identity.md` records verification.

### Creation-only session discovery metadata (ADR 0063, wiring in progress)

`SessionSpec.listing` keeps an omitted override distinct from explicit Shown.
`SessionInfo.listing` is the immutable effective classification, independent of
process state and archival. The store appends a default-Shown column (schema
21, `vessel-store::schema::listing`); it does
not rewrite original specs or change the migration runner. Protocol projection
hides both fields while the lifecycle assignment is pending, including reusable
templates and transfers. The existing creation and discovery service paths
consume these types for legacy/unbound sessions; explicit inputs remain refused
while assignment is pending. Canonical bound workspace mirroring and the
separately integrated CLI/MCP consumers remain outside this node package.

### Export metadata readiness (ADR 0063)

The node's `metadata_export` policy shares protocol visibility with reply
projection. The exporter validates its one owned info snapshot before reading
provider history, so the checked classification is the one in the manifest.
Local pulls have no wire projection and retain their existing import guards.
Forward and Relay also validate returned export manifests against their next
peer before projection; a new source/home cannot silently turn an old importer
into a Shown session. This adds no crate edge, authority or wire shape.

Rejected export manifests retain their original envelope route for CloseTransfer.
The existing forwarding TaskTracker owns cleanup across caller cancellation;
Links keeps direct/callback selection and the source guard keeps holder/scope
checks. Cleanup waits no longer than the existing transfer idle lifetime, while
normal shutdown grace and source expiry/restart remain unchanged.


## RPC caller-drop admission (ADR0080 prerequisite)

The client writer reuses each call's existing oneshot closure instead of a
second cancellation queue. It skips closed callers at dequeue and observes
closure while waiting for reply-registration capacity or sink readiness.
The private send_call helper installs the live waiter immediately before the
synchronous start_send admission; the reader's registration-before-answer
ordering is preserved without canceled unsent registrations. Public client
APIs and frame/protocol shapes stay unchanged. Already admitted transport
bytes and remote side effects remain outside retraction; post-admission
Cancel is still best-effort. This adds no crate dependency or voice activation.

The optional composer command-kind values live in vessel-proto::composer_commands. AgentCommand gains an optional classification; the existing event and reply projection removes it while the feature has no assigned protocol. This carrier adds no RPC, method row, runtime/driver behavior or crate edge. Existing constructors supply no classification; the full discovery producer and native intent/receipt path remain separate. See docs/plans/composer-kind-values-carrier.md.

Private PTY spawn diagnostics use the existing tracing-core subscriber API to
forward the caller's span context while suppressing command-bearing upstream
portable-pty records in a synchronous thread-local scope. The daemon separately
caps portable_pty at warn after its user filter to suppress inherited Windows
environment trace records. See `plans/wp-command-diagnostics.md` for why both
boundaries are required and their execution limits. The test-only log and
tracing-subscriber edges exercise the actual bridge without a second logger.

## Voice settings values carrier

vessel-proto::voice contains only the canonical VoiceSettings/SetVoice metadata
and bounded independent recognition/synthesis options. It reuses dictation's
existing omission/reset visitor, validated metadata and secret references;
readers receive key presence only. The pending None constant grants no runtime
or protocol capability. This carrier adds no node settings field, persistence,
RPC, handler, model, provider or audio engine. Host Voice UI may consume it only
through a typed fake until the real node field/adapter lands. See
`docs/plans/wp-voice-values.md` and the broader ADR0080 voice package.

## Physical-root borrow producer (pending validation)

The root lease leaf adds only a BlobHash-backed PhysicalRoot value and a Linux
opened-FD mechanism using existing dependencies. One workspace::exclusion owner
issues opaque leases; StartGuard pins cwd plus the nearest registered project or
worktree ancestor and borrows its exact RootLease. The retained metadata pool and ordinary file.write
consumer preserve admission through physical completion and caller loss.
The same registry snapshot supplies both root kinds. Opened parent/child scopes
conflict in either order, including nested registrations added during a lease;
sharing only a parent does not serialize siblings. Relative input refuses before
ambient-cwd filesystem lookup.
This is source awaiting exact-leaf validation and V3, not a restore, terminal
quiescence or lifecycle capability. No store schema, protocol, checkpoint RPC or
event is introduced. Other writer/cleanup consumers remain pending on the same
producer. See docs/plans/root-lease-seam.md for scope and validation obligations.

`vessel-session::LaunchAdmission` is a read-only clone of the runtime's existing
shutdown token and Node's original early-stop token. Its one predicate is shared
by final structured/terminal spawn and external preparation observers; it grants
no permission or current-run authority. The Node factory lives beside setup so
this wiring adds no size debt. Settings/SQL/recovery consumer changes are outside
this standalone prerequisite (ADR 0064 and launch-admission plan).
