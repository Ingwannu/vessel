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
   its motion are agent-orchestrator's; where it has no such control, T3
   Code's.
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
- **The session**: its title in the bar; the composer at the foot as T3
  Code's (a raised box, the prompt, a row with permissions, model, effort
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

## Gate

Rust tracks: `cargo fmt --all`, `cargo clippy --workspace --all-targets --
-D warnings`, `cargo test --workspace`. Web tracks: `npm run lint`,
`typecheck`, `test`, `build`, `e2e`, `screens`, and the captures looked
at, in both themes, beside the references' own.

## Left open

- Whether the page's words (machine, agent) should reach the CLI.
- Plugins and the shells.

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
