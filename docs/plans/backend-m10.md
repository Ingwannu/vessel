# Milestone 10: controls, a composer, settings, and what a session runs with

Status: in progress, 2026-11-11. Built on `backend/m10`, stacked on
`backend/m9` (milestones 8 and 9 are not merged yet).

Read first: `AGENTS.md`, `docs/plans/backend-m9.md`, ADRs 0017 to 0020,
`docs/design/web-ui/DESIGN.md`. The references, all public:

- [agent-orchestrator](https://github.com/OrchestratorInc/agent-orchestrator)
  (Apache-2.0): the shell, the board, the "Create a new task" dialog, the
  session view, its `DESIGN.md` and `frontend/src/styles/tokens.css`.
- [T3 Code](https://github.com/pingdotgg/t3code) (MIT): the composer and
  its row of controls (model, effort, access), the controls themselves.
- [GOAT](https://github.com/goat-si/goat) (MIT, a fork of T3 Code): adding
  a computer, the settings pages, the provider status.
- [Paseo](https://github.com/getpaseo/paseo): the welcome screen, adding a
  host, the settings' structure, model / thinking / mode on a running
  agent.

## Goal

The owner looked at milestone 9's page and said what is wrong with it:

1. The dropdowns and text fields are the browser's own. Every control is
   to be built for this design.
2. There is no dark mode to be found.
3. It looks poor, and the font does not look applied.
4. It is not intuitive: nothing to choose a model, an effort or the
   permissions with.
5. Take more from agent-orchestrator; look at T3 Code, at Paseo and at
   GOAT for the composer, the settings and connecting a computer.
6. Then, shown two captures (a machine card with a heavy dark edge; the
   search pill with the input's own focus box inside it): take
   agent-orchestrator's and GOAT's UI and design system and make it as
   similar as it can be, and use nothing the browser draws by itself.

So the sources are, in order: agent-orchestrator, then GOAT, then T3 Code
where GOAT changed nothing. Where the first two differ on the same thing,
agent-orchestrator's stands (its shell and board are already here).

A capture of the page agrees: the new-session dialog's native selects
overflow the dialog, it speaks of harnesses, profiles and ACP, and it has
no way to pick a model. Milestone 9 took the reference's shell and board;
this milestone takes its controls and flows, and gives the node what they
need.

## Out of scope

- Plugins (a later milestone; ADR 0020's page-in-shells design is what
  makes one plugin reach every surface).
- The desktop and phone shells (ADR 0020, still proposed).
- Pull-request facts for the board's three empty lanes.
- A file browser on a node. The folder field suggests folders sessions
  already ran in; it does not list a node's disk.

## Before writing code

1. **What exists.** The ACP handshake already receives the agent's
   session config options and uses the `model` one
   (`vessel-agent/src/model.rs`); the mapper drops the two updates that
   say they changed. The bridge already inspects a pairing link, pairs and
   draws a QR code (`vessel-web`, ops `inspect`, `pair`, `qr`). The page
   has one dialog, a board, a tree, tokens, and plain CSS in one file.
2. **Reuse or new.** The options are the agent's own, passed on: no list
   of models or modes is written into Vessel. The controls are new, on a
   headless library. The pairing flow is the existing ops behind a guided
   dialog.
3. **Outside.** The four references above for how it looks and flows.
   For the controls' behaviour (focus, typeahead, placement, dismissal),
   [Base UI](https://base-ui.com) (`@base-ui/react`, MIT, unstyled): the
   part of a custom dropdown that hand-written ones get wrong. ADR 0021.
4. **Decision: modify, and add one dependency.**

## Words on the page

The CLI and the protocol keep their words. The page says what a person
would:

| Protocol | Page |
|---|---|
| node | machine |
| harness | agent |
| session config option, category `mode` | permissions |
| session config option, category `thought_level` | effort |
| profile | profile (shown only when a machine has any) |

"Structured" and "terminal" become one switch, "Open as terminal", off by
default. "Fleet", "node book", "ACP" and "PTY" do not appear outside
Settings' detail rows.

## Contract (protocol 9)

Track C adds it to `vessel-proto` in its first commit, as below, and the
generated TypeScript with it. A track that needs it changed asks the
coordinator.

`crates/vessel-proto/src/options.rs`:

- `OptionId`, `OptionValueId`: opaque ids, the agent's own, echoed back.
- `OptionCategory`: `mode`, `model`, `model_config`, `thought_level`,
  `other`. ACP's categories; an unknown one is `other`.
- `OptionValue { id, name, description?, group? }`.
- `OptionKind` (tagged `kind`): `select { current, values }`,
  `toggle { on }`.
- `SessionOption { id, name, description?, category, <kind> }`.
- `OptionChoice { id, <value> }` where the value is `select { value }` or
  `toggle { on }`: what a client asks for.

On existing types:

- `SessionMode::Structured` gains `options: Vec<OptionChoice>`, applied
  after the session is created, in order, after `model`. A choice the
  agent does not offer fails the session, as an unknown model does today.
- `SessionSpec.title: Option<String>` and `SessionInfo.title:
  Option<String>`. A session without one takes the first line of its
  first prompt, cut at 80 characters, when that prompt arrives.
- `SessionEvent::OptionsChanged { options: Vec<SessionOption> }`: logged
  whenever the agent's options change (after the handshake, after a set,
  and when the agent itself reports a change). Older clients receive a
  harness log line instead, as `ProfileChanged` does.
- `SessionEvent::Renamed { title }`.

Calls:

| Method | Scope | Request | Reply |
|---|---|---|---|
| `harness.options` | read | `GetHarnessOptions { harness, profile? }` | `HarnessOptions { options, probed_at }` |
| `session.options` | read | `GetSessionOptions { session }` | `Vec<SessionOption>` |
| `session.set_option` | operate | `SetSessionOption { session, choice }` | `Vec<SessionOption>` |
| `session.rename` | operate | `RenameSession { session, title }` | `SessionInfo` |

`PROTOCOL_VERSION` is 9.

## Track C: what a session runs with (`backend/m10-options`)

Owns: `vessel-proto` (the contract above), `vessel-agent`,
`vessel-session`, `vessel-node`, `vessel-store`, the handlers and fakes a
new call needs in `vessel-rpc`, `vessel-client` and `vessel-web`'s
allowlist, `docs/PROTOCOL.md`, ADR 0022. Does not touch `web/src` except
the generated `web/src/wire/proto.ts`.

1. **The contract**, with handlers that answer `unsupported` until the
   steps below fill them, the TypeScript regenerated, the gate green. This
   is its own commit and the track reports when it exists: the web tracks
   build on it.
2. **Probing an agent.** `harness.options` starts the agent as a
   structured session would (same profile, a scratch directory under the
   node's data), reads the options from the handshake and ends it. Results
   are cached per harness and profile and refreshed when older than ten
   minutes or when the harness's version changes; two callers share one
   probe. An agent that offers ACP's older `modes` and no `mode` option
   gets one synthesized from them (set with `session/set_mode`).
3. **Choices at creation**, in the handshake, generalizing
   `model::choose`.
4. **Choices on a running session**: `session.set_option` while idle or
   mid-turn as the agent allows; the agent's refusal is the call's error.
   `ConfigOptionUpdate` and `CurrentModeUpdate` from the agent are no
   longer skipped: both end as `OptionsChanged`.
5. **Titles**: from the spec, from the first prompt, from
   `session.rename`; kept in the store.
6. **Truth over neatness.** What an agent does not offer is not offered:
   no effort control for an agent without a `thought_level` option. Record
   in the track's report what each installed agent actually returns.
7. **Docs and gate** as in milestone 9.

## Track E: the book from the page (`backend/m10-book`)

Owns: `vessel-web` (the bridge's ops), `vessel-cli` where the book is
written, the generated `web/src/wire/bridge.ts`.

- `forget { node }`: remove a machine from the book and close its
  connection. Not the machine this page was opened from.
- `rename { node, name }` and `default { node }`.
- Every op is refused when the page's session is not the bootstrap one,
  as `pair` is. Tests beside the existing bridge tests.

## Track W0: the controls (`backend/m10`, first)

Owns `web/`. Everything later stands on it.

1. **A kit** in `web/src/ui/kit/`, on Base UI, styled only from tokens:
   Button and IconButton, TextField, TextArea (grows with its text),
   Select, Menu, Combobox (a field with suggestions), Popover, Dialog,
   Tooltip, Switch, Checkbox, Segmented, Tabs, Badge, Kbd, Spinner,
   EmptyState. Sizes, radii, borders, focus rings, the popup's surface and
   its motion are agent-orchestrator's; where it has no such control,
   GOAT's. A field is one shell: the shell carries the border, the fill
   and the focus ring, the input inside it draws nothing. A `Row` for
   lists of machines, devices, agents and settings; selection is a wash,
   never an edge.
2. **No native control left.** Every `<select>`, and every `<input>` and
   `<textarea>` outside the kit, is replaced; `npm run lint` fails on a
   new one (a check beside `check-fonts.mjs`).
3. **Theme**: system, light or dark, system by default as the reference
   has it, applied before first paint, following the system while set to
   it. A switch in the sidebar's foot, in Settings and in the command
   palette.
4. **Type**: agent-orchestrator's faces, Geist and Geist Mono (SIL Open
   Font License, bundled as it bundles them), then Pretendard for the
   scripts Geist lacks, in every stack (the owner's rule keeps Pretendard
   ahead of any generic family). The font file is preloaded and hashed
   assets are cached as immutable, so a remote page does not show the
   fallback face on each visit.
5. **CSS in parts**: `app.css` (2,600 lines) becomes one file per area, so
   two tracks can work at once.
6. **A gallery** at `/kit` in development builds: every control, every
   state, both themes. The screens script captures it.

## Track W1: starting and running a session (`backend/m10-compose`)

- **New session** is agent-orchestrator's `/new` page: the heading "What
  do you want to work on?", a raised prompt box, and in its foot one
  control for agent, model and effort (a menu with a row for each), one
  for permissions, then Send. Under the box, where it runs: the machine
  and the folder. Each is filled from the machine (`harness.list`,
  `harness.options`); what an agent does not offer is not shown. Behind
  the box's "+": a profile, "Open as terminal" and its arguments. Send
  creates the session, sends the prompt and opens it.
- **The session**: its title in the bar; the composer at the foot as
  GOAT's (a raised box, the prompt, a row with permissions, model, effort
  and Send or Stop) reading and setting the running session's options; one
  permission prompt, not two; the inspector as agent-orchestrator's
  summary.
- **The sidebar and the board**: New session at the top; Board; machines
  with their sessions; Settings and the theme switch at the foot. Cards
  and rows show the session's title.

## Track W2: machines and settings (`backend/m10-settings`)

- **Settings** as a page with its own section list: General, Appearance,
  Machines, Agents, Models, Devices, Sync, About. Fleet, Sync and Devices
  leave the sidebar and live here.
- **Add a machine**: a guided dialog over the existing ops. Run
  `vessel node pair` there (copy), paste the link here, compare the name
  and the key's fingerprint, connect, then see which agents it has.
  Reached from the sidebar, Settings, the command palette and the welcome
  screen. Forget, rename and default from a machine's row (track E).
- **Agents**: per machine, each agent's version, whether it can run as a
  session, and what it offers (models, efforts, permissions).
- **Welcome**: with no machine connected, the page says so and offers the
  dialog, as Paseo's first screen does.

## Structure (every web track)

Milestones 7 to 9 left the page with one 2,600-line stylesheet, route
files of 450 to 700 lines that fetch, decide and draw in one function, and
calls to the bridge written inside components. This milestone adds screens
on top of that, so the shape is fixed first and checked by `npm run lint`,
not by review:

- **Layers, one direction.** `wire/` (generated) <- `bridge/` (the socket)
  <- `data/` (state, derived values, and every call to a node as a typed
  function) <- `features/<area>/` (an area's components and its own
  hooks) <- `routes/` (a route composes features and nothing else).
  `ui/kit/` and `ui/styles/` import none of them. ESLint's
  `no-restricted-imports` holds the directions.
- **A component does not call a node.** It calls a function in `data/`
  that does, and gets a typed result or a typed failure back.
- **One component, one file, one stylesheet beside it** in the kit and in
  features. A file over 300 lines of TypeScript, or 400 of CSS, fails the
  check unless it is on the list of files still to be divided; that list
  only shrinks.
- **Decisions are data.** What a lane, a phrase or an option means is a
  pure function with a unit test (`data/board.ts` is the pattern); a
  component only draws its result.
- **No cast at a call site.** A value from the DOM or the wire is narrowed
  by a function that checks it; the casts left are at the two boundaries
  that already have them (the socket's frames, xterm's internals).

### Types

The owner's rule: no `any`, and types held as strictly as they can be, in
TypeScript as in Rust. The compiler options and the ESLint presets were
already at their strictest; what they still allowed is closed here, in
`web/eslint.config.js`, so it fails `npm run lint` instead of waiting for
a review:

- No type assertion (`consistent-type-assertions: never`,
  `no-unsafe-type-assertion`). A value from the socket or the DOM is
  narrowed by a predicate that checks it. One file keeps an assertion
  (xterm's private core), by a config override that names it.
- No inline suppression (`noInlineConfig`); `@ts-expect-error` only with a
  description.
- No implicit truthiness (`strict-boolean-expressions`).
- Explicit parameter and return types on every exported function of a
  logic module.
- In the kit: a generic control keeps its option's type (`Select<T>`
  gives back `T`, not `string`); props that only make sense together are
  one discriminated union.
- The check scripts are type-checked too.

Still open, for after the kit: the generated wire types alias every id to
`string`, so a node's key can be passed where a session's id is wanted.
Branding them in the generator (`vessel-proto/src/ts.rs`) and parsing at
the two places a bare string enters (a route's parameters, a frame) is
its own change across all of `web/src`.

In Rust the title added by protocol 9 becomes `SessionTitle`, parsed on
the way in (trimmed, not empty, bounded, no control characters), instead
of the bare `String` the contract above first said.

The Rust side keeps what `AGENTS.md` already enforces (the crate map and
its one direction of dependency, `unwrap`, `expect` and `panic` denied by
the workspace's lints, closed enums matched exhaustively). Three files
there are over 900 lines (`vessel-node`'s model proxy, fleet links and
service); a track that adds to one of them adds a module beside it
instead.

## Gate

Rust tracks: `cargo fmt --all`, `cargo clippy --workspace --all-targets --
-D warnings`, `cargo test --workspace`. Web tracks: `npm run lint`,
`typecheck`, `test`, `build`, `e2e`, `screens`, and the captures looked
at, in both themes, beside the references' own.

## Left open

- Whether the page's words (machine, agent) should reach the CLI.
- Plugins and the shells.
