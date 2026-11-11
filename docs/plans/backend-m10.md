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
- Every op sits where `pair` sits: behind the socket's mutual proof and
  within a tab's limit of calls in flight, with no further gate. A launch
  has one session key and every authenticated tab holds it, so there is
  no "bootstrap session" to tell apart (this plan first said there was;
  ADR 0017's amendment records the decision). Tests beside the existing
  bridge tests.

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

Owns `features/compose/`, `features/session/`, `features/shell/` (the
frame and the sidebar, moved out of `ui/shell.tsx`), the old
`routes/new-session.tsx`, `session.tsx`, `structured.tsx`, `terminal.tsx`
and `sessions.tsx` (divided as they move), and the calls for options and
titles in `data/`.

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

Owns `features/settings/`, `features/machines/`, `features/welcome/`, the
old `routes/settings.tsx`, `fleet.tsx`, `sync.tsx` and `devices.tsx`
(divided as they move), and the settings routes. It does not edit the
sidebar: track W1 draws the section list there from
`data/settings.ts`.

- **Settings** as a page with its own section list, at
  `/settings/<section>`: Appearance; Agents, Models; Machines, Devices,
  Sync; About (`web/src/data/settings.ts` is the one list). While it is
  open the sidebar shows the sections in place of the machines, with a
  way back, as agent-orchestrator and GOAT both do. Fleet, Sync and
  Devices leave the sidebar and live here; their old addresses redirect.
  No "General" section: nothing true would be in it yet.
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

## Track E: done (2026-11-11)

Built, merged into `backend/m10`:

- The bridge's ops `forget`, `rename` and `default`, each answering with
  the book as it stands; `vessel nodes rename`, on the book's one writer
  (`BookFile::rename`). The machine this page was opened from is not a
  book entry and is neither forgotten nor renamed.
- Whatever names a node on the bridge's wire is parsed where the frame is
  decoded (`wire/node.rs`: `NodeKey`, `BookName`); a frame that does not
  decode is still answered by its id, with a typed reason.
- A pairing link and the text of a QR code are `SecretText`: `Debug`
  prints a placeholder, and the content is read in three places only.
- Content-hashed assets are served as immutable (ADR 0017's amendment
  says why the poisoned-cache defence does not depend on revalidating
  them).
- `bridge.rs` divided: the tab's socket (292 lines), `streams.rs`,
  `calls.rs`, `ops.rs`.

Checked by the track: the three Rust gates (1,015 tests), and the web
side's typecheck, lint, unit and end-to-end tests against the regenerated
`bridge.ts`. An independent review's four findings in this area are
fixed.

Left as it is: two concurrent `vessel` processes writing the book at the
same instant can still lose one edit (the book has no lock; it had none
before); a forgotten paired machine's credential file stays on disk, as
`vessel nodes remove` leaves it; `wire.rs` (653 lines) and `registry.rs`
(494) each have a seam and were not divided.

## Track C: done (2026-11-11)

Built, merged into `backend/m10` (ADR 0022, `docs/PROTOCOL.md` version 9):

- **Probing an agent** (`harness.options`): the agent is started as a
  session would start it, its options read from the handshake, and it is
  ended; cached per harness and profile, one probe shared by concurrent
  callers, bounded by the handshake timeout. An agent that is not
  installed or cannot start answers a typed error with its last lines; a
  harness with no structured mode answers an empty list.
- **Choices** at creation, on a running session (`session.set_option`),
  and reported by the agent itself: each ends as one `OptionsChanged`
  carrying the whole set. Every structured session logs one just before
  it is ready, an empty one when its agent offers nothing, so a client
  tells "nothing offered" from "not known yet".
- **Titles**: `SessionTitle`, parsed on the way in (trimmed, not empty,
  no control characters, 120 characters typed or 80 taken from the first
  prompt that has text). `SessionInfo.title` is the current title;
  `spec.title` only what creation asked for. A rename survives a restart
  (the store's schema 6).
- **A moved session keeps both**: export carries the title and the
  choices as they stand, import and a move between profiles apply them
  after the load, and fail loudly when the destination's agent does not
  offer one.
- `OptionKind` takes a kind it does not know (`unknown`); clients skip
  such an option.

What the installed agents returned from a handshake (no prompt sent):

| Agent | Options |
| --- | --- |
| Claude Code (through its ACP adapter) | permissions mode (five), model, effort (six levels), two model toggles, and its own sub-agent list |
| OpenCode | model, mode (two) |
| Codex | nothing here: its ACP adapter is not installed on the machine that was checked |
| Gemini | not installed there |

Checked by the track: the three Rust gates (1,035 tests), with the fake
harnesses offering options. An independent review's eight findings in
this area are fixed.

Not checked: Vessel's own probe against a real agent (the table above
came from a separate handshake script, and the mapping is tested on its
recorded answers); setting an option on a real agent.

## Track W0: progress (2026-11-11)

Built on `backend/m10` as it stood before track E's merge, in `web/` and
the docs this track owns (`docs/design/web-ui/DESIGN.md`, ADR 0021, the
web UI section of `docs/ARCHITECTURE.md`). No crate and no wire type
changed.

What was built:

- **Dependencies** (ADR 0021): `@base-ui/react` 1.9.0,
  `@fontsource-variable/geist` and `@fontsource-variable/geist-mono`
  5.3.0, pinned exactly; the Geist licences served beside Pretendard's.
- **The stylesheet in parts.** `app.css` is `web/src/ui/styles/*.css`,
  one file per area, imported in a fixed order from `styles/index.css`.
  Shown to be the same stylesheet before anything changed: the built CSS
  held the same 420 rules in the same cascade, and the screens differed
  only in text that changes per run (ports, ids, times).
- **The kit**, `web/src/ui/kit/`: Button and IconButton, TextField,
  TextArea (grows with its text), Select, Menu (checkbox and radio rows,
  submenus), Combobox, Popover, Dialog, Tooltip, Switch, Checkbox,
  Segmented, Tabs, Badge, Kbd, Spinner, EmptyState, Field and FieldGroup,
  Row and RowList, the command palette, and Icon, Banner, Skeleton and
  StatusDot moved in from the screens. One component per file, its CSS
  beside it; the popup surface and option row, the field shell and the
  check label written once. The values are DESIGN.md §6.13 and §6.14.
- **No native control left.** Every select, input and textarea outside
  the kit is the kit's (the new-session and move dialogs, Settings,
  Devices, the archive's search, the composer), every `title` tooltip is
  a kit Tooltip, `ui/dialog.tsx` is gone (every dialog and the palette are
  on the kit's Dialog), the sidebar's search is one shell, and Fleet's
  machines are rows.
- **The theme.** Every colour token is `light-dark()`; `data/theme.ts`
  keeps the choice and pins it on `<html>` before the first render. The
  switch is in Settings, at the sidebar's foot and in the palette. The
  terminal takes resolved colours again when the scheme changes.
- **Type.** Geist and Geist Mono, then Pretendard, in every stack and in
  xterm's; the Latin Geist file preloaded from `index.html` (the build
  gives the preload the stylesheet's own hashed name, so `integrity.mjs`
  is untouched).
- **Icons** at the reference's stroke and sizes, with check, chevrons,
  search, close, sun, moon, monitor and plus.
- **Structure and types.** The layer rules, the file budget and the
  strict type rules below. The bridge's frames are narrowed by
  predicates (`bridge/frames.ts`), a call is typed by its method and its
  reply narrowed by that method; the e2e fixture is checked by the
  socket's own predicate. What the screens decided in place moved to
  `data/` with tests (`commands`, `choices`, `archive`, `nodes`,
  `format`), so no listed screen grew.
- **The gallery** at `/kit` (`features/kit-gallery/`), captured by
  `e2e/kit-screens.ts` light and dark, at 1440 and 390px wide, with a
  select at the window's right edge holding a long Korean option, a menu
  and its submenu, a tooltip with the focus ring, a popover, a
  combobox, a dialog and the palette, each opened from the keyboard.

The checks `npm run lint` runs:

- ESLint: `strictTypeChecked` and `stylisticTypeChecked`, React hooks,
  the rules below, and `no-restricted-imports` for the layers (`wire` <-
  `bridge` <- `data` <- `features` <- `routes`; the kit imports none of
  them; only the kit imports `@base-ui/react`).
- Prettier.
- `scripts/check-fonts.mjs`, extended: a stack that names Geist names
  Pretendard after it; `@font-face` families are checked against the
  forbidden faces only.
- `scripts/check-controls.mjs` (new): a `<select>`, `<input>` or
  `<textarea>`, or a `title` on an element or a `Link`, outside
  `src/ui/kit/`.
- `scripts/check-size.mjs` (new): 300 lines of TypeScript, 400 of CSS;
  tests and the generated wire types not counted.

`npm run typecheck` also checks `web/scripts/` (`checkJs`, each script
`// @ts-check`), and `tsconfig.json` has `noUncheckedSideEffectImports`.

The rules this track turned on, and what each found before it was fixed:

| Rule | Findings |
| --- | --- |
| `consistent-type-assertions` (`never`) | 11 |
| `no-unsafe-type-assertion` | 9 |
| `strict-boolean-expressions` (booleans and nullable objects only) | 4 |
| `noInlineConfig` (the one inline disable, in `context.ts`) | 1, and the `react-hooks/exhaustive-deps` finding it had hidden: 1 |
| `reportUnusedDisableDirectives` | 0 |
| `explicit-module-boundary-types` (`*.ts`) | 0 |
| `no-non-null-assertion`, `prefer-readonly`, `no-unnecessary-condition`, `no-unnecessary-type-parameters`, `method-signature-style` | 0 each |
| `ban-ts-comment` (`@ts-expect-error` with a description only) | 0 |

The one assertion left is xterm's private core in
`data/terminal-input.ts`, by an override that names the file.

Files still to divide (`check-size.mjs`; the ceiling, then the size now):

| File | Ceiling | Now |
| --- | --- | --- |
| `src/routes/structured.tsx` | 700 | 697 |
| `src/bridge/client.ts` | 467 | 467 |
| `src/routes/sessions.tsx` | 451 | 450 |
| `src/data/timeline.ts` | 406 | 382 (406 with track E) |
| `src/data/board.ts` | 353 | 353 |
| `src/routes/devices.tsx` | 349 | 346 |
| `src/routes/session.tsx` | 328 | 325 |
| `src/routes/fleet.tsx` | 316 | 314 |

Gate, run in `web/` on the code as committed (the docs commit after it
changes no code):

- `npm ci`: added 281 packages, audited 282. `npm audit` reports 10
  moderate, all `minimatch` under ESLint and typescript-eslint, which this
  track did not change.
- `npm run lint`: ESLint and Prettier clean; `fonts: 48 stacks follow the
  rule`, `controls: nothing native outside the kit`, `size: within
  budget, 8 files still to divide`.
- `npm run typecheck`: no errors, the page and then `scripts/`.
- `npm run test`: 15 files, 63 tests passed.
- `npm run build`: built; the script 1,124.59 kB (333.41 kB gzipped), the
  stylesheet 70.43 kB.
- `npm run e2e`: 16 passed.
- `npm run screens`: 2 passed (every main screen; the kit), 31 captures,
  each looked at in both themes.

Merged with `backend/m10` as it now stands (track E) in a trial merge
that was not kept: ESLint, Prettier, the checks, `tsc` and the unit tests
pass.

Where this differs from the plan or the reference, and why:

- The gallery ships in every build, not in development builds only: the
  screens capture the built page `vessel ui` serves, and a second entry
  or a lazy chunk would be a second graph for `integrity.mjs` to pin. It
  holds no data, needs no token, and nothing links to it.
- Caching hashed assets as immutable is `vessel-web`'s, under `crates/`,
  outside this track; track E did it on `backend/m10`.
- The plan named T3 Code for the controls agent-orchestrator lacks; the
  coordinator made GOAT the second source. GOAT's are the segmented
  control's thumb, the popover's widths, the key chip, the machine row
  and the 24px toolbar sizes.
- The deliberate differences from the references are DESIGN.md §12.
- A radio group is a `FieldGroup`, not a `Field`: Base UI's field gives
  its label to every control inside it, so each radio was named "Mode".
- `data/timeline.ts` is listed at 406 lines, its size once track E's
  title events are merged, so the merge of the two tracks passes.

Left open:

- Only Chromium was exercised (the one browser Playwright has here);
  the page was not looked at in Firefox or Safari.
- The Combobox is used only by the gallery; track W1's folder field is
  its first screen.
- The generated wire types still alias every id to `string` (Structure,
  Types).
- This section and track E's both end this file, so merging the two
  joins them by hand.

## Track W1: progress (2026-11-11)

Built on `backend/m10-compose`, from `backend/m10` as it stood after
tracks C, E and W0. No crate and no wire type changed: the fake agents
already offer a permissions mode (with a value they refuse), a model, an
effort in two groups and an on/off setting, which is what the screens
and their tests needed.

What was built:

- **Calls and decisions as data.** `data/calls.ts`: every call the
  session screens make, each answering a typed result or the bridge's
  typed failure (`harness.options`, `session.create`, prompt, permission
  answer, cancel, kill, move, `session.set_option`, `session.rename`, the
  stream). Pure modules with tests: `options.ts` (an agent's options laid
  out by category only, in its words; what the controls show while
  loading, failed or for a terminal; an older node), `picks.ts` (choices
  made before a session exists, sent only when they change something, in
  the agent's order), `titles.ts`, `folders.ts`, `approval.ts` (Approve,
  Decline and "More" from the agent's option kinds; the timeline's line),
  `agents.ts` (agents by name, profiles with their routes),
  `new-session.ts` (the starting machine and agent, the spec Send
  creates, a terminal's arguments split as a shell would), `sidebar.ts`.
- **New session** at `/new` (`features/compose/`), agent-orchestrator's
  new-task page: the heading, the raised box, its foot row ("+" with a
  profile and "Open as terminal" with its arguments; one control for
  agent, model and effort; the permissions control; Send), the tray with
  the machine and a folder field suggesting where sessions ran. The
  options load in the trigger ("Loading models…") without blocking the
  page; a failure is said in the menu and under the box, and the session
  starts on the agent's defaults. The old dialog is gone; every New (the
  title bar, the sidebar, an empty Building lane, the palette, `n`) leads
  here with the machine in view.
- **The session** (`features/session/`): its title in the frame's bar,
  renamed in place; the composer (the same box and controls, reading
  `Timeline.options`, setting with `session.set_option`, Send or Stop);
  one permission prompt docked over the composer (GOAT's approval row)
  and one line in the timeline that becomes the outcome; the summary as
  agent-orchestrator's inspector, collapsible sections with the
  identifiers behind Details; the log and composer in a 768px column; a
  menu of the session's actions. The terminal keeps its frame under the
  same title bar.
- **The shell** (`features/shell/`): the frame and the sidebar out of
  `ui/shell.tsx` (removed), the sidebar agent-orchestrator's (brand,
  Search, New session, Board, Machines with their sessions by title,
  Settings and the theme at the foot; Settings' sections and Back while
  under `/settings`, from `data/settings.ts`, which this track did not
  edit). A page can draw into the title bar (`TitleBar`).
- **The board** (`features/board/`): cards titled by the session's
  title, the agent by name, the machine in the page's words.
- **The kit**: Avatar, Collapsible, a 256px menu, grouped radio values,
  switch rows and notes in menus, button shapes (10px quiet triggers, a
  round Send and Stop as a primary icon button), a quiet Combobox,
  TextField's `onBlur`, eight icons; all in the gallery.
- **Divided and off the list**: `routes/structured.tsx`, `session.tsx`,
  `sessions.tsx` (and `terminal.tsx`, which was not listed);
  `data/board.ts` (its phrases are `data/phrase.ts`) and
  `data/timeline.ts` (its shapes are `data/timeline-model.ts`). Three
  files are left on the list: `bridge/client.ts`, `routes/devices.tsx`
  and `routes/fleet.tsx`. The stylesheets of the session, the timeline,
  the permission prompt, the terminal, the board, its cards and its
  archive, and the shell moved beside their components;
  `ui/styles/motion.css` is gone (its last rule is the card's).

Checked, in `web/`, on the code as committed:

- `npm run lint`: ESLint and Prettier clean; `fonts: 44 stacks follow the
  rule`, `controls: nothing native outside the kit`, `size: within
  budget, 3 files still to divide`.
- `npm run typecheck`: no errors.
- `npm run test`: 22 files, 106 tests passed.
- `npm run build`: the script 1,151.13 kB (341.23 kB gzipped), the
  stylesheet 78.98 kB.
- `npm run e2e`: 20 passed. Rewritten for `/new`: every test that drove
  the old dialog, the routed-model tests (the profile under "+"), the
  sidebar's name. New: every New leads to `/new` on the machine in view;
  a session starts with permissions chosen before it (`Plan`) and runs
  with them; an option changed on a running session shows once the agent
  reports it, and one the agent refuses is said and changes nothing; a
  title renamed in place reaches the sidebar.
- `npm run screens`: 3 passed, 56 captures; `e2e/session-screens.ts`
  adds the new-session page and each of its menus, the session waiting on
  a permission, its Details, actions and rename, the board and the
  Settings sidebar, light and dark, and at 390px. Each was looked at
  beside agent-orchestrator's and GOAT's.

Not built:

- The "Back" row returns to the board, not to the page Settings was
  opened from.
- The summary does not toggle (`⌘I`); it leaves below 1280px of window.
- Files cannot be attached to a prompt (the reference's "+" attaches
  files; Vessel's prompts carry text only).

Differences from the references that remain, and why (DESIGN.md §12,
items 6 to 12): the permissions rows share one shield and the agent's
own wording (no per-agent knowledge, ADR 0022); no prompt chips under the
heading; no search or favourites in the agent menu; "+" is a popover
holding a text field; terminal or timeline is chosen once; the summary
has one view and is 340px wide; the session is named in the title bar
rather than a strip of tabs.

Not verified: only Chromium was exercised; a real agent's options were
not read through these screens (the fakes stood in, at no cost).

## Track W2: progress (2026-11-11)

Built on `backend/m10` (tracks C, E and W0 merged), in `web/` and the
docs this track owns. No crate and no wire type changed.

What was built:

- **Settings as routed pages.** `/settings/<section>` for every section
  `data/settings.ts` lists (the contract with track W1, not edited), an
  unknown or missing one going to the default; `/fleet`, `/fleet/<machine>`,
  `/sync` and `/devices` redirect to their sections with the machine they
  named (`?machine=`). Each section is agent-orchestrator's settings page
  at GOAT's readable width: a centred 896px column, one lead sentence,
  sections 40px apart, a 14px heading over a card of the kit's rows.
  Literal values (addresses, ids, paths) only in detail rows.
- **Machines**: GOAT's environment rows (a server icon, the name, one
  line of platform, reach, state and live sessions; "This machine" and
  "Default" badges; a "…" menu of Rename…, Make default, Forget…). Forget
  and Rename ask first, in dialogs that say what they do and do not do;
  this machine has neither. A chosen machine's detail (what Fleet's
  inspector was) opens under the list: state, reach, platform, version
  and protocol, id, its agents (each opening its card), its live
  sessions.
- **Add a machine**: one dialog over track E's ops, reached from
  Machines, the welcome and the command palette (a new palette action).
  Paseo's order (the command, the link, "Connect to this machine?" with
  the key's fingerprint, routes, id and expiry, then a name checked as
  typed) and GOAT's checklist after Connect (Paired, Connected, Agents
  found, each pending, passed, failed or skipped, a failure saying what
  to do with a command to copy). The link lives in the dialog's state
  only.
- **Agents**: per machine, every agent Vessel drives as GOAT's provider
  rows (Available, Needs attention with the machine's reason, Not found);
  an agent's card asks `harness.options` only when opened and lists what
  it offers in the plan's words (Models, Effort, Permissions, then the
  agent's own).
- **Models, Devices, Sync, Appearance, About** on rows and the kit, their
  actions kept (invite with a hidden-until-asked link and QR code,
  revoke; the sync state), their words plain. About lists each machine's
  version and protocol and every bundled package's licence.
- **Welcome**: the index is the board once a machine is connected and
  Paseo's first screen until then, with `vessel node start` to copy when
  this machine's own node is the one not running.
- **The bridge's op failures are typed.** `Bridge.op` rejected with a
  bare `Error`; it rejects with `OpFailed { reason }` now (`unreadable`,
  `busy`, `failed`, or `disconnected` for a lost socket), and
  `data/outcome.ts`'s `attempt` turns every call and op into an
  `Outcome`, so what a screen says next is a switch on the reason.
- **Kit**: `CommandLine` (GOAT's setup command, copying and saying so on
  itself) and four icons (server, and a checklist's passed, failed and
  skipped), in the gallery.
- **Structure**: `routes/fleet.tsx`, `devices.tsx` and `sync.tsx` are gone
  and off the list of files still to divide (6 left); `routes/settings.tsx`
  only composes. Every decision is a pure function with a test
  (`data/machines`, `pairing`, `agents`, `devices`, `sync`, `welcome`,
  `settings-place`, `model-rows`, `about`, `outcome`); every call is a
  typed function in `data/` (`book`, `agents`, `devices`, `sync`). The
  styles only the old pages used (tables, the summary strip, panels,
  pairing) are removed, as is `nodeFacts`.

Checked:

- End to end: Settings' machines (from `/fleet`), a device invited and
  revoked (from `/devices?node=`), the Models section, and a new
  `machines.spec.ts`: a machine added through the dialog with a real link
  from `vessel node pair` (garbage refused first, the fingerprint matched
  against `vessel node status`, the name rules), then renamed, made
  default and forgotten, each checked against `vessel nodes list`; an
  agent's card listing what the fake agent offers.
- Captures (`e2e/settings-screens.ts`): every section, each stage of
  adding (and a refused link), the menu, rename and forget, light and
  dark, the busiest at 390px, and the welcome from a UI with an empty
  book; each looked at.

Gate, run in `web/` on the code as committed (the docs commit after it
changes no code):

- `npm run lint`: ESLint and Prettier clean; `fonts: 47 stacks follow the
  rule`, `controls: nothing native outside the kit`, `size: within
  budget, 6 files still to divide`.
- `npm run typecheck`: no errors, the page and then `scripts/`.
- `npm run test`: 26 files, 104 tests passed.
- `npm run build`: built; the script 1,149.21 kB (340.58 kB gzipped), the
  stylesheet 76.25 kB.
- `npm run e2e`: 18 passed.
- `npm run screens`: 3 passed (every main screen; the kit; Settings and
  the welcome), 68 captures, 42 of them this track's, each looked at.

What the wire does not give a screen (no Rust changed; for the
coordinator):

- A pairing link names the machine by its id and key only, so "Connect
  to this machine?" cannot show its name (Paseo's shows the host's);
  `PairingOffer` would need the node's name.
- `vessel node pair` does not print the key's fingerprint; the dialog
  points at `vessel node status`'s `key` line instead. Printing it next
  to the link would make the comparison one step.
- A device paired from the page is labelled with the UI machine's host
  name (the `pair` op takes no device label).
- The bridge stops dialling this machine's node after a hard failure (no
  token before the node has run); the welcome retries it on focus and on
  "Check again" rather than claim it connects by itself.

For the merge with track W1:

- "Start a session there" calls `openNewSession(machine)`; when W1's
  `/new` replaces the dialog, that function is the one place to change.
- The sidebar here still has Fleet, Sync and Devices (they redirect) and
  no section list; no in-page section list was added. The palette's
  places likewise still name Fleet, Sync and Devices.
- `e2e/open.ts` waits for the sidebar's "Fleet" navigation; `screens.ts`
  lost its Fleet, Sync, Devices and Settings captures (they are in
  `settings-screens.ts`) and imports the sync setup from `fleet-home.ts`.

Differences from the references that remain: DESIGN.md §12, items 6 to
9 (an 896px column; an agent's card as one list, not tabs; no on/off
switch on a machine's row; the confirmation names the machine by its id).

Not verified: browsers other than Chromium; an agent's card against a
real agent (only the fake); pairing through a relay (the test fleet's
links name a direct route only).
