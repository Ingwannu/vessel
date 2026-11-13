# ADR 0032: Any agent that speaks ACP

Status: accepted, 2026-11-12 (milestone 11, wave 2, track G,
`docs/plans/backend-m11.md`, "Track G: contract" and its review; the
wire is protocol 11, `crates/vessel-proto/src/agents/`).

## Context

Vessel knew four agents (Claude Code, Codex, Gemini CLI, OpenCode), a
closed set on the wire (`HarnessKind`) matched exhaustively across the
workspace and the page. The owner asked for nearly everything the three
references do: agent-orchestrator knows 36 agents and says of each
whether it is installed and signed in, with installing and signing in
from its page; GOAT adds any agent from the ACP registry or a local ACP
command, with setup flows and status; Paseo has a generic ACP adapter, a
catalogue of about 35 agents, providers by command, and diagnostics.

Constraints:

- Peers of protocols 2 to 10 decode an agent as one of four strings; a
  list holding anything else fails to decode whole.
- What Vessel knows of the four (homes, logins, session files, fleet
  sync, routing) does not exist for an agent someone added, and must not
  be guessed.
- A configured command is code run as the node's user.
- The registry is data from the network.
- A login acts as an account; a key typed into a page must not pass
  through anything that logs it.

Questions:

1. How is an agent outside the four named on the wire, and what do older
   peers see?
2. Where is an added agent kept, who may change it, what does it get?
3. What does the registry become in Vessel?
4. What are "installed" and "signed in"?
5. What does the node do to install and sign in, and what does it only
   explain?

## Options

**Naming**

1. A fifth `HarnessKind` variant with a name inside: `Copy` lost, and
   every `match` over the four gains an arm that pretends to know.
2. A free string for every agent: the four lose their exhaustive
   matches.
3. Added names as further strings beside the four's: a stored spec
   changes meaning the day Vessel knows a fifth agent by a name someone
   used.
4. A new type, the four as they are or an added agent by name, the
   latter an object on the wire.

**Older peers**

1. A stand-in: report an added agent's session as one of the four (a
   wrong agent shown, and acted on).
2. Leave it out of lists; withhold a reply about one, and refuse a call
   acting on one before it runs.

**What an added agent is given**

1. The node's environment, as the four get it (Paseo, GOAT): every
   provider key and token of the node reaches whatever the command is.
2. An empty environment with a list of the system's own non-credential
   variables, and what its definition names.

**The registry**

1. A catalogue shipped with Vessel, pinned by version (Paseo).
2. The published registry, fetched on request (GOAT), installing on add.
3. The published registry, fetched on request, as suggestions: an entry
   becomes a definition the node builds from its own copy; nothing is
   downloaded by listing or adding.

**Signing in**

1. The node only reports, and says how.
2. A terminal running the agent's own login (agent-orchestrator).
3. A login run in the background, its URL and code relayed, a key piped
   to it (agent-orchestrator for Claude Code and Codex).
4. ACP's own methods: `authenticate`, a browser URL elicited by the
   agent, terminal methods (GOAT).

## Decision

1. **`HarnessRef`** (option 4): `Known(HarnessKind)` or
   `Added(AgentName)`; on the wire the four stay their strings and an
   added agent is `{"added": "<name>"}`. It replaces `HarnessKind` in
   `SessionSpec.harness`, `Harness.kind`, `ProfileInfo.harness`,
   `ListProfiles.harness` and `GetHarnessOptions.harness`; `HarnessKind`
   stays wherever only the four can be. `AgentName` is the profile name
   rule, refused when it is, ignoring case, any name of the four;
   agents' and profiles' names are one space.
2. **Older peers** (option 2): `harness.list`, `profile.list`,
   `profile.reload` and `session.list` leave added agents out; a reply
   about one becomes `unsupported` (`Reply::withheld`, applied where
   frames are written, inside the fleet's envelope too); the node
   records each connection's protocol and refuses the calls that act on
   such a session, and `session.subscribe`, before they run. A call
   naming an added agent needs protocol 11 (`Call::since`).
3. **`agents.toml`** in the node's data directory: program, arguments,
   environment (text or a secret of `secrets.toml` by name), the
   variables to inherit by name, the fleet's MCP servers it gets, and
   where it came from. Changed only by `admin` principals that are
   neither an agent nor a call from another node. Owner-only; refused
   when others can write it. Read with `profiles.toml`, together. Not
   copied by the fleet: a definition names programs, paths and secrets of
   one machine.
4. **What it gets** (option 2): `PATH`, the user's home, the system's own
   non-credential variables, proxies without credentials, what it names;
   the fleet's MCP servers only by name; the `vessel` server and the
   agent credential as the four get them. Never `--version`; never a
   terminal session; no profile `args` or model route; no move, export,
   import or fleet sync.
5. **The registry** (option 3): fetched only on a person's request
   (never by an agent principal), over https, bounded and timed out,
   kept in memory with its time; an entry Vessel cannot read, or that
   pins no exact version, is skipped and counted; an archive-only entry
   is shown, not downloaded; an entry that is one of the four points to
   installing that agent.
6. **Installed and signed in** are typed facts with reasons
   (`AgentReadiness`): found with its path and version, missing, too old
   or too new in ACP, unknown and why; signed in when the agent's own
   status check says so, signed out when it or the handshake's
   `auth_required` says so, not required on a model route, unknown and
   why (not checked, in use, not said, timed out, failed at a step). The
   status commands run only when a person asks for one agent and profile,
   never while a session of that profile runs; their output is read for
   the verdict and dropped.
7. **Installing**: the node runs the vendor's documented command for one
   of the four or its adapter, on an `admin` person's request, shown
   before, with a clean environment, its output bounded and paged.
   **Signing in** (option 1): the node says how (the command, the
   profile's home variable by name, the agent's ACP methods); a key for
   an added agent goes through `secret.set`, which logs no value.

## Why

- An object for an added agent keeps the four's strings, so nothing an
  older peer or the store wrote changes, and no future name can collide.
- Leaving out is the only honest answer for a peer that cannot decode
  the agent; refusing before the call keeps "withheld" from hiding a
  change.
- A clean environment is what makes running a package from the network
  tolerable: it gets what it is given, not everything the node holds.
  The four keep the node's environment because their homes and logins
  are Vessel's knowledge and their vendors are the ones the person chose
  when installing them; changing that is a separate decision.
- Building a registry entry on the node from its own copy is what makes
  "what runs is what the person saw" a property of the node, not of the
  client.
- A status check that refreshes credentials can sign a person out when
  it races the session using them (it has happened); asking only on
  request, and not while that account works, keeps the check from doing
  harm.
- Signing in from the node needs a launch without MCP servers (Claude
  Code's `--mcp-config` takes every word after it) and a relay of
  one-time codes; neither is safe to rush.

## Consequences

- Protocol 11: seven methods (`agent.list`, `agent.status`, `agent.add`,
  `agent.remove`, `agent.catalogue`, `agent.install`,
  `agent.install_output`), `Harness.terminal`, and `HarnessRef` in five
  places.
- `vessel-node` fetches the registry itself with `reqwest` (already in
  the workspace for `vessel-llm`), with the TLS settings `vessel-llm`
  uses: one GET, no redirect, no proxy; https only, but for plain http to
  a loopback address, which is how tests serve the registry.
- A protocol 10 client acting through the fleet on an added agent's
  session by its id is not refused by the member (a delivered call
  carries no client protocol); the home still withholds every reply that
  would carry the session.
- Next steps, each its own decision: downloading registry archives
  (checksums, extraction that cannot leave its folder); signing in from
  the node (a launch without MCP servers, a relay of codes with hosts
  from an allowlist, keys piped to the login's standard input); the
  page's agents screens.
