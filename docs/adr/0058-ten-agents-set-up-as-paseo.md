# ADR 0058: Ten agents set up as Paseo sets them up

Status: accepted, 2026-11-14, on the owner's word ("set them up exactly
as Paseo does": Codex, Claude Code, Grok, OpenCode, Pi, Oh My Pi, Antigravity,
Muse Code, Amp and Cursor), with the coordinator's decisions on the open
points (recorded in "Decision"). It widens ADR 0057 (Codex, Claude Code and
OpenCode over their own protocols) to the other seven, and amends ADR 0032
(agents named on the wire) and ADR 0048 (agent marks). The plan is
`docs/plans/ten-agents.md`.

## Purpose and intent

A person who picks one of these ten agents in Vessel gets what Paseo gives
them: the same way of driving it, the same permission modes under the same
names and the same defaults, the same models, effort steps and switches,
the same abilities (steer, interrupt, resume, rewind, fork, commands, MCP
servers, images, questions, plan approval, usage), an install and sign-in
check, and, with the vendor's leave, the agent's own mark. Where an agent
runs without permission prompts, Vessel says so in plain words wherever
that mode is offered or a session starts in it.

## Context: what exists, and the constraints

**What Paseo 0.15.4 does** (`getpaseo/paseo` at tag `v0.15.4`:
`packages/server/src/server/agent/provider-registry.ts`,
`packages/protocol/src/provider-manifest.ts`, the built-in plugins under
`plugins/`, and the app's ACP catalogue
`packages/app/src/data/acp-provider-catalog.ts`):

| Agent | How Paseo drives it | Modes (default) |
| --- | --- | --- |
| Codex | its own client of `codex app-server` | `auto`, `auto-review`, `full-access` (`auto-review`) |
| Claude Code | Anthropic's Agent SDK driving the person's `claude` | `plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions` (`auto`) |
| OpenCode | `@opencode-ai/sdk` against `opencode serve` | the server's agents (`build`, `plan`); an Auto Accept switch |
| Pi | `pi --mode rpc`, JSON lines | none (Pi has no permission system) |
| OMP | `omp --mode rpc-ui`, JSON lines with approvals; off by default | `full`, `write`, `ask` (`full`) |
| Antigravity | built-in plugin: `agy` stream-json with `--dangerously-skip-permissions` | `full-access` only |
| Muse Code | built-in plugin: `muse serve`, JSON-RPC ("MSP"), served with `--disable-sandbox --trust-workspace` | `onRequest`, `promptUnmatched`, `denyUnmatched`, `allowAll` (`onRequest`) |
| Grok | ACP catalogue entry: `grok agent stdio` | what the agent reports |
| Amp | ACP catalogue entry: the third-party wrapper `amp-acp` | what the wrapper reports |
| Cursor | ACP with a Cursor client: `cursor-agent acp`, models from the extension `cursor/list_available_models`, a Fast select | what the agent reports |

Paseo also reads several agents' credential files to show usage
(`plugins/*-usage-source`), checks Pi's sign-in by the presence of its
credentials file, and gives the Agent SDK `allowDangerouslySkipPermissions`
on every session.

**What Vessel has:**

- `HarnessKind` names four agents (Claude Code, Codex, Gemini, OpenCode),
  one module each in `vessel-harness`, matched exhaustively. Any other
  agent that speaks ACP is an added agent (ADR 0032): a definition in
  `agents.toml`, a clean environment, the generic mark (ADR 0048 keeps an
  added agent generic even when it is named like a known one), and no
  agent-specific behaviour, since nothing may match on its name.
- ADR 0057's seam: a driver per protocol in `vessel-agent`, chosen per
  known agent by `vessel_harness::direct(kind)`, with ACP the default and
  the fallback.
- Session options are whatever the agent reports (ADR 0022); an option
  value has an id, a name, a description and a group, nothing that says
  whether it runs without asking.
- The ACP registry, fetched on a person's request (ADR 0032), lists
  first-party ACP entries for Grok, Cursor and Muse Code, a separate ACP
  server for Antigravity, and third-party adapters for Amp and Pi.

**Constraints:**

- The node is Rust; Paseo's TypeScript (its SDK clients and plugins) is
  neither a dependency nor a source of code or artwork.
- Harness-specific knowledge lives only in `vessel-harness`, one module
  per agent; closed sets are enums (`AGENTS.md`, sections 2 and 3).
- ADR 0057's security floor: what ACP would ask about is asked about; no
  permission-skipping flag unless the session's mode chooses it; a driver
  never reads an agent's credential files; a choice a driver cannot honour
  is refused with a typed error, never dropped.
- Older peers decode an agent as one of four strings (ADR 0032's
  withholding protects them).

## Alternatives considered

1. **The seven as added agents** (ADR 0032), suggested by the registry.
   Least code, and Grok and Pi's adapter can be added this way today. But
   an added agent keeps the generic mark, cannot carry Cursor's model list
   or any agent's install and sign-in checks, cannot be driven by a direct
   driver (the seam selects per known agent), and Pi, OMP and Antigravity
   have no ACP of their own to add.
2. **An open plugin mechanism** like Paseo's plugin providers, loaded at
   run time: the first open extension point for agents, with a `dyn`
   boundary and a code-loading story, to serve seven agents whose set is
   known now.
3. **ACP wherever any ACP exists** (`pi-acp`, Google's Antigravity ACP
   server, `muse acp`, `amp-acp`). Least driver code, but not what Paseo
   does: Pi's adapter is a third party's, Antigravity's server is a
   separate download (an archive the node does not fetch), and Pi's and
   OMP's own features would depend on adapters.
4. **Seven known agents, with Paseo's drivers where Paseo has its own**
   (chosen).

## Decision

1. **Seven new known agents.** `HarnessKind` gains `Grok`, `Cursor`,
   `Amp`, `Pi`, `Omp`, `Antigravity` and `Muse`, named `grok`, `cursor`,
   `amp`, `pi`, `omp`, `antigravity`, `muse` on the wire (Paseo's ids;
   `amp-acp`, `agy`, `muse-code` and `oh-my-pi` are accepted when parsing).
   Each has its module in `vessel-harness` (binary, home, install command,
   sign-in command or check, version minimum).
   - **Protocol 21, provisional** (19 is side questions, 20 the device's
     own view; the number is confirmed when the line merges). A peer older
     than 21 is handed these as ADR 0032 hands an added agent: left out of
     lists, a reply about one withheld, calls on one refused before they
     run. `HarnessKind::since` says which protocol reads each kind.
   - **Marks**: they are known kinds so that a mark can be attached later
     without a wire change; until the owner and the vendor say so, each
     shows the generic mark beside its name (decision 8).
2. **How each is driven:**
   - **Grok, Cursor, Amp: ACP.** `grok agent stdio` (with
     `--no-auto-update`, which xAI asks automation to pass);
     `cursor-agent acp`; Amp through `amp-acp`, a third-party adapter,
     which Vessel names as such. The registry's entries for Grok, Cursor,
     Amp and Muse Code point to the known agent.
   - **Cursor's extras** are declared by its harness module and served by
     the ACP driver: the `parameterizedModelPicker` client capability, the
     model list from `cursor/list_available_models` (each model with its
     effort steps; a probe never selects a model, since that saves
     Cursor's preferences), refused with a typed "update Cursor" where the
     extension is missing, and its `fast` option as the Fast switch.
   - **Pi, OMP, Antigravity: their own protocols only.** `vessel_harness`'s
     `Structured` gains `None`: these have no ACP launch, and until their
     direct driver is built and selectable a session is refused
     (`unsupported`, naming the driver it needs). New drivers behind ADR
     0057's seam, ordered with that seam's owners first: `PiRpc` (`pi
     --mode rpc`, documented in Pi's `docs/rpc.md`), `OmpRpc` (`omp --mode
     rpc-ui`, sharing Pi's framing), and `AntigravityStream` (`agy`
     stream-json, documented on Google's headless page). The pi-acp
     adapter and Google's ACP server stay reachable as added agents.
   - **Muse Code: its own ACP first, its own protocol next.** The ACP launch
     is `muse acp` with `MUSE_EXPERIMENTAL_ACP=on` (Meta's registry entry),
     the default, saying in the session's log that Muse calls it
     experimental. The `MuseMsp` driver (`muse serve`) follows ADR 0057's
     rules, with a contract test that regenerates Muse's protocol schema
     (`muse schema generate-ts`) from an installed Muse.
   - Each new driver has its fake in `vessel-testkit`, built from the
     vendor's document or schema where one exists, and `selectable` stays
     false until its permissions and questions slice lands.
3. **Modes: Paseo's ids, labels and defaults; the safer mode beside it;
   the unattended one named.**
   - Every agent's modes are Paseo's, under Paseo's ids and labels, and
     its default is Paseo's default, since the owner asked for "exactly".
     Codex: `auto`, `auto-review` (default), `full-access`, with a fixed map
     from codex-acp's ids when a session changes driver. Claude Code: a
     session that chose no mode is launched with `auto`, except under
     Bedrock or Vertex, where it is launched with `default` and only a
     chosen `auto` is refused (as Paseo). OMP: `full` (default), `write`,
     `ask`. Antigravity: `full-access` (default). Muse: `onRequest`
     (default), `promptUnmatched`, `denyUnmatched`, `allowAll`, served as
     Paseo serves it.
   - Where an agent has a safer mode than Paseo's default, that mode is
     offered beside it: OMP's `write`; Antigravity's `deny` (no skip flag:
     Antigravity then denies what needs review and goes on, per Google's
     headless documentation, and each denial is shown once); Muse with its
     sandbox on and the workspace not trusted. If the owner later prefers
     the safer mode as the default, only the default changes.
   - Option values gain `risk` (`safe`, `moderate`, `dangerous`,
     `planning`: Paseo's tiers) and `unattended` (protocol 21, left out when
     false). A direct driver sets them; for an ACP agent, the harness module
     tags the mode ids it knows (Grok's always-approve, Amp's Bypass). A
     mode that runs without permission prompts is named for what it is,
     "Full access", in every list that offers it and in the line a session
     shows when it starts in it; every client shows it as dangerous.
   - A permission-skipping flag (`--dangerously-skip-permissions`,
     `--approval-mode yolo`, Muse's `allowAll`, `--disable-sandbox`) is
     passed only for a session whose mode chooses it (which a default mode
     may do, as in Paseo).
   - Pi has no permission system: its readiness and every Pi session say
     "Pi asks for nothing: every tool runs as you".
4. **Environment.** The seven get ADR 0032's clean environment, plus their
   vendor's documented variables by name (for example `XAI_API_KEY`,
   `CURSOR_API_KEY`, `AMP_API_KEY`, `META_API_KEY`, Pi's provider keys), not
   the node's whole environment. The four keep theirs.
5. **Install and sign-in.** Installed: the binary, its `--version`, and the
   driver's minimum (OMP 16.3.9, Antigravity 1.1.15, Muse 1.3.0, as Paseo).
   Signed in: the agent's own status command or a call that runs no turn
   (Muse's `account/read`, Pi's and OMP's model list being empty, `agy
   models`, ACP's `auth_required`), only on a person's request (ADR 0032,
   decision 6). A driver never reads an agent's credential files: Paseo's
   check of Pi's `auth.json` is replaced by Pi's own model list. Install
   commands are the vendor's own, shown before they run; archive-only
   installs are printed, not run.
6. **MCP servers for Pi, as Paseo does it.** Before a Pi session with MCP
   servers, the driver asks Pi for its commands (a short `pi --mode rpc`
   run with `get_commands`, no turn): when the person's Pi has the
   `pi-mcp-adapter` package, the servers go in a `--mcp-config` file; when
   it has Pi's own MCP extension, they go in an extension file of Vessel's
   own that registers them (`pi.registerMcpServer`), passed with
   `--extension`; both files are written in the node's owner-only
   directory through `McpFile`, since a server's environment can carry
   credentials. When Pi has neither, Paseo starts the session without the
   servers; Vessel refuses it, typed (`LaunchRefusal::Mcp`), and says which
   Pi package would carry them.
7. **Usage** is what the agent's own protocol reports (Pi's and OMP's
   session statistics, Muse's `usage/read`, Antigravity's step usage, ACP's
   usage updates). Paseo's usage plugins, which read credential files and
   call vendors' account endpoints, are not reproduced.
8. **Enabled.** A node setting turns each agent on or off (Paseo's
   `providers.<id>.enabled`); OMP is off by default, as in Paseo.
9. **Marks** (amends ADR 0048): none of the seven ships a mark until the
   owner has answered and its vendor's source and terms are recorded (with
   the vendor's permission where it requires one). Until then each shows
   the generic mark beside its name. Being known kinds, they take a mark
   later without a wire change.

## Why this and not the others

- Known agents are the only form in which Vessel may carry agent-specific
  behaviour (Cursor's model list, Antigravity's denials), checks and
  marks, and the only form ADR 0057's seam selects a driver for.
- A closed set of eleven is still a closed set: adding one fails to
  compile until its module exists, which an open plugin mechanism would
  give up for no present need.
- Paseo's own protocols and defaults are what the owner asked for and what
  carry Paseo's features. Muse's first-party ACP is used first only
  because it costs a harness entry, and stays as the fallback ADR 0057
  keeps for its three.
- Keeping Paseo's defaults while offering the safer mode beside them, and
  naming every unattended mode where it is chosen, keeps "as Paseo" from
  meaning "without asking unknowingly".

## Advantages, costs and effects

- **Advantages:** the ten appear with their own names, modes, models and
  checks, and later their marks; Grok, Cursor, Amp and Muse work over ACP
  as soon as their harness modules land; nothing above ADR 0057's seam
  changes for the new drivers.
- **Costs:** a protocol number (seven kinds, two option fields, a setting);
  seven harness modules; four drivers. In Paseo they are about 1,100 lines
  (Antigravity), 2,300 (Muse), 5,800 (Pi, with its bundled extensions) and
  8,600 (OMP), and each needs upkeep at its agent's releases. OMP's
  protocol has no vendor document, so its fake rests on observed shapes
  and needs real runs.
- **Effects:** ADR 0032's "an added agent is any agent outside the four"
  becomes "outside the eleven"; an older peer sees fewer agents. ADR 0048's
  closed mark set grows as rights are granted. Seven vendors' marks need a
  request first (xAI, Cursor, Amp, Google for Antigravity, Meta for Muse
  Code, and the Pi and OMP projects, whose sites state no terms for their
  logos).
